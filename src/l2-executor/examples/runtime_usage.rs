//! L2 Runtime 使用示例

use anyhow::Result;
use l2_executor::{
    BackendType, FibonacciProgram, L2Runtime, RuntimeConfig,
};

/// 示例 1: 自动选择后端
fn example_auto_select() -> Result<()> {
    // 系统自动选择最佳后端
    // Windows: Trace
    // Linux + risc0-poc: RISC0
    let runtime = L2Runtime::auto_select()?;
    
    println!("Selected backend: {}", runtime.backend_type());
    println!("Configuration: {:?}", runtime.config());
    
    // 使用 Trace VM 执行程序
    let vm = runtime.create_trace_vm();
    let program = FibonacciProgram::new(10);
    let witness = vec![0, 1];
    
    let proof = vm.prove(&program, &witness)?;
    println!("Proof generated: public_outputs = {:?}", proof.public_outputs);
    
    Ok(())
}

/// 示例 2: 手动指定后端
fn example_manual_backend() -> Result<()> {
    // 显式指定使用 Trace backend (跨平台)
    let runtime = L2Runtime::new(BackendType::Trace)?;
    
    println!("Using backend: {}", runtime.backend_type());
    
    let vm = runtime.create_trace_vm();
    let program = FibonacciProgram::new(20);
    let proof = vm.prove(&program, &[0, 1])?;
    
    println!("Fibonacci(20) = {:?}", proof.public_outputs);
    
    Ok(())
}

/// 示例 3: 使用配置文件
fn example_config_file() -> Result<()> {
    // 从 TOML 文件加载配置
    let runtime = L2Runtime::from_config_file("config.toml")?;
    
    println!("Loaded config: {:?}", runtime.config());
    
    let _vm = runtime.create_trace_vm();
    // ... 执行业务逻辑
    
    Ok(())
}

/// 示例 4: 自定义配置
fn example_custom_config() -> Result<()> {
    let config = RuntimeConfig {
        backend: Some(BackendType::Trace),
        enable_logging: false, // 关闭日志
        dev_mode: true,        // 启用开发模式
    };
    
    let runtime = L2Runtime::with_config(BackendType::Trace, config)?;
    
    let _vm = runtime.create_trace_vm();
    // ... 执行快速测试
    
    Ok(())
}

/// 示例 5: 检查后端可用性
fn example_check_availability() {
    println!("Available backends:");
    for backend in L2Runtime::available_backends() {
        println!("  - {}", backend);
    }
    
    println!("\nBackend availability:");
    println!("  Trace: {}", L2Runtime::is_backend_available(BackendType::Trace));
    println!("  RISC0: {}", L2Runtime::is_backend_available(BackendType::Risc0));
    println!("  Halo2: {}", L2Runtime::is_backend_available(BackendType::Halo2));
}

/// 示例 6: 跨平台业务逻辑
fn example_cross_platform_business_logic() -> Result<()> {
    // 自动选择后端,业务逻辑无需关心平台差异
    let runtime = L2Runtime::auto_select()?;
    
    // 统一的 Trace VM 接口 (跨平台)
    let vm = runtime.create_trace_vm();
    
    // 执行多个程序
    let programs = vec![
        FibonacciProgram::new(5),
        FibonacciProgram::new(10),
        FibonacciProgram::new(15),
    ];
    
    for program in programs {
        let proof = vm.prove(&program, &[0, 1])?;
        println!("Program {} result: {:?}", 
                 proof.program_id, 
                 proof.public_outputs);
    }
    
    Ok(())
}

/// 示例 7: 条件编译 - RISC0 专用功能
#[cfg(all(feature = "risc0-poc", not(target_os = "windows")))]
fn example_risc0_specific() -> Result<()> {
    use l2_executor::{Risc0Backend, ZkVmBackend};
    
    let runtime = L2Runtime::new(BackendType::Risc0)?;
    let risc0_vm = runtime.create_risc0_vm();
    
    // 使用 RISC0 ZkVmBackend trait
    let program_id = &l2_executor::L2_EXECUTOR_METHODS_FIBONACCI_ID;
    let private_inputs = b"some_private_data";
    let public_inputs = vec![10u32];
    
    let (proof, outputs) = risc0_vm.prove(program_id, private_inputs, &public_inputs)?;
    println!("RISC0 proof generated: {} bytes", 
             bincode::serialize(&proof)?.len());
    
    let verified = risc0_vm.verify(program_id, &proof, &public_inputs, &outputs)?;
    println!("Verification result: {}", verified);
    
    Ok(())
}

fn main() -> Result<()> {
    // 初始化日志 (可选)
    // 使用 RUST_LOG=info 环境变量启用
    
    println!("=== L2 Runtime Examples ===");
    
    println!("1. Auto Select Backend");
    example_auto_select()?;
    
    println!("\n2. Manual Backend Selection");
    example_manual_backend()?;
    
    println!("\n3. Check Availability");
    example_check_availability();
    
    println!("\n4. Cross-Platform Business Logic");
    example_cross_platform_business_logic()?;
    
    // RISC0 专用示例 (仅 Linux)
    #[cfg(all(feature = "risc0-poc", not(target_os = "windows")))]
    {
        println!("\n5. RISC0 Specific Features");
        example_risc0_specific()?;
    }
    
    Ok(())
}
