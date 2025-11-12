//! GPU backend implementation using wgpu (WebGPU cross-platform API)
//! Provides WgpuExecutor for compute shaders and BufferPool for memory management.

#[cfg(feature = "gpu")]
use wgpu::{
    Adapter, Device, Queue, Buffer, BufferUsages, ShaderModule, ComputePipeline,
    BindGroupLayout, BindGroup, CommandEncoder, ComputePass,
};
#[cfg(feature = "gpu")]
use std::sync::Arc;
#[cfg(feature = "gpu")]
use std::collections::VecDeque;

use crate::{Batch, Task, TaskResult, ExecStats, ExecError, ExecErrorKind, DeviceKind, GpuExecutor};

/// WGSL shader for vector addition (f32)
#[cfg(feature = "gpu")]
const VECTOR_ADD_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> input_a: array<f32>;
@group(0) @binding(1) var<storage, read> input_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx < arrayLength(&input_a)) {
        output[idx] = input_a[idx] + input_b[idx];
    }
}
"#;

/// Buffer pool for reusing GPU buffers across batches.
#[cfg(feature = "gpu")]
pub struct BufferPool {
    device: Arc<Device>,
    free_buffers: VecDeque<(usize, Buffer)>, // (size, buffer)
    max_pool_size: usize,
}

#[cfg(feature = "gpu")]
impl BufferPool {
    pub fn new(device: Arc<Device>, max_pool_size: usize) -> Self {
        Self {
            device,
            free_buffers: VecDeque::new(),
            max_pool_size,
        }
    }

    /// Acquire a buffer of at least `size` bytes; reuse from pool if available.
    pub fn acquire(&mut self, size: usize, usage: BufferUsages) -> Buffer {
        // Try reuse
        if let Some(pos) = self.free_buffers.iter().position(|(s, _)| *s >= size) {
            let (_s, buf) = self.free_buffers.remove(pos).unwrap();
            return buf;
        }
        // Allocate new
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pooled_buffer"),
            size: size as u64,
            usage,
            mapped_at_creation: false,
        })
    }

    /// Return a buffer to the pool for reuse.
    pub fn release(&mut self, size: usize, buffer: Buffer) {
        if self.free_buffers.len() < self.max_pool_size {
            self.free_buffers.push_back((size, buffer));
        }
        // else drop (let it be freed)
    }

    pub fn hit_rate(&self) -> f64 {
        // Placeholder: track hits/misses in production
        0.0
    }
}

/// WgpuExecutor: compute shader-based GPU executor for vector operations.
#[cfg(feature = "gpu")]
pub struct WgpuExecutor {
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
    buffer_pool: BufferPool,
}

#[cfg(feature = "gpu")]
impl WgpuExecutor {
    /// Initialize wgpu device and compile vector add shader.
    pub async fn new() -> Result<Self, ExecError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or_else(|| ExecError::new(ExecErrorKind::BackendUnavailable, "No GPU adapter found"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("gpu-executor"),
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .map_err(|e| ExecError::new(ExecErrorKind::Internal, format!("Device request failed: {}", e)))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Compile shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vector_add_shader"),
            source: wgpu::ShaderSource::Wgsl(VECTOR_ADD_SHADER.into()),
        });

        // Bind group layout: 3 storage buffers (input_a, input_b, output)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vector_add_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vector_add_pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("vector_add_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let buffer_pool = BufferPool::new(device.clone(), 16);

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            buffer_pool,
        })
    }

    /// Execute vector addition: each task payload is (Vec<f32>, Vec<f32>) -> Vec<f32>
    pub fn execute_vector_add(
        &mut self,
        batch: &Batch<(Vec<f32>, Vec<f32>)>,
    ) -> Result<(Vec<TaskResult<Vec<f32>>>, ExecStats), ExecError> {
        let start = std::time::Instant::now();
        let mut results = Vec::with_capacity(batch.tasks.len());

        for task in &batch.tasks {
            let (a, b) = &task.payload;
            if a.len() != b.len() {
                return Err(ExecError::new(ExecErrorKind::InvalidTask, "Vector length mismatch"));
            }
            let n = a.len();
            let byte_size = (n * std::mem::size_of::<f32>()) as u64;

            // Allocate buffers
            let buffer_a = self.buffer_pool.acquire(
                byte_size as usize,
                BufferUsages::STORAGE | BufferUsages::COPY_DST,
            );
            let buffer_b = self.buffer_pool.acquire(
                byte_size as usize,
                BufferUsages::STORAGE | BufferUsages::COPY_DST,
            );
            let buffer_out = self.buffer_pool.acquire(
                byte_size as usize,
                BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            );

            // Write data
            self.queue.write_buffer(&buffer_a, 0, bytemuck::cast_slice(a));
            self.queue.write_buffer(&buffer_b, 0, bytemuck::cast_slice(b));

            // Bind group
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vector_add_bg"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: buffer_a.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: buffer_b.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: buffer_out.as_entire_binding() },
                ],
            });

            // Dispatch
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vector_add_encoder"),
            });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("vector_add_pass"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                let workgroups = (n as u32 + 63) / 64;
                cpass.dispatch_workgroups(workgroups, 1, 1);
            }

            // Read back
            let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: byte_size,
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            encoder.copy_buffer_to_buffer(&buffer_out, 0, &staging, 0, byte_size);
            self.queue.submit(Some(encoder.finish()));

            // Map and read
            let buffer_slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
            self.device.poll(wgpu::Maintain::Wait);
            rx.recv()
                .map_err(|_| ExecError::new(ExecErrorKind::Internal, "Channel recv failed"))?
                .map_err(|e| ExecError::new(ExecErrorKind::Internal, format!("Buffer map failed: {:?}", e)))?;

            let data = buffer_slice.get_mapped_range();
            let output: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
            drop(data);
            staging.unmap();

            results.push(TaskResult { id: task.id, output });

            // Return buffers to pool
            self.buffer_pool.release(byte_size as usize, buffer_a);
            self.buffer_pool.release(byte_size as usize, buffer_b);
            self.buffer_pool.release(byte_size as usize, buffer_out);
        }

        let stats = ExecStats {
            device: DeviceKind::Gpu,
            duration: start.elapsed(),
            tasks: results.len(),
        };

        Ok((results, stats))
    }
}

#[cfg(feature = "gpu")]
impl GpuExecutor<(Vec<f32>, Vec<f32>), Vec<f32>> for WgpuExecutor {
    fn execute(
        &mut self,
        batch: &Batch<(Vec<f32>, Vec<f32>)>,
    ) -> Result<(Vec<TaskResult<Vec<f32>>>, ExecStats), ExecError> {
        self.execute_vector_add(batch)
    }

    fn is_available(&self) -> bool {
        true // already initialized
    }

    fn device_kind(&self) -> DeviceKind {
        DeviceKind::Gpu
    }
}

/// Blocking helper to initialize WgpuExecutor (for non-async contexts).
#[cfg(feature = "gpu")]
pub fn create_wgpu_executor() -> Result<WgpuExecutor, ExecError> {
    pollster::block_on(WgpuExecutor::new())
}
