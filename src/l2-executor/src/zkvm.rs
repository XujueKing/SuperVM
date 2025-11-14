use anyhow::{bail, Result};

use crate::program::{ExecutionTrace, TraceProgram};
use crate::proof::Proof;
use crate::backend_trait::ZkVmBackend;

/// 简化版 zkVM，基于轨迹承诺生成证明 (Mock 实现)
#[derive(Default)]
pub struct TraceZkVm;

impl TraceZkVm {
    pub fn prove<P: TraceProgram>(&self, program: &P, witness: &[u64]) -> Result<Proof> {
        let trace = program.generate_trace(witness);
        if trace.len() < 2 {
            bail!("trace too short for proof generation");
        }
        let commitment = trace.commitment();
        let public_outputs = program.public_outputs(witness);
        Ok(Proof::new(program.id(), public_outputs, trace.len(), commitment))
    }

    pub fn verify<P: TraceProgram>(&self, program: &P, proof: &Proof, witness_hint: &[u64]) -> Result<bool> {
        if proof.program_id != program.id() {
            return Ok(false);
        }
        let recomputed_trace = program.generate_trace(witness_hint);
        let recomputed_commitment = recomputed_trace.commitment();
        let recomputed_outputs = program.public_outputs(witness_hint);
        Ok(proof.trace_commitment == recomputed_commitment && proof.public_outputs == recomputed_outputs)
    }
}

/// 支持可插拔后端的 zkVM Wrapper
///
/// 允许在运行时选择不同的 zkVM 后端 (RISC0, Halo2, SP1 等)
pub struct PluggableZkVm<B: ZkVmBackend> {
    backend: B,
}

impl<B: ZkVmBackend> PluggableZkVm<B> {
    /// 创建使用指定后端的 zkVM
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// 获取后端引用
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// 通过后端生成证明
    pub fn prove_with_backend(
        &self,
        program_id: &B::ProgramId,
        private_inputs: &[u8],
        public_inputs: &B::PublicIO,
    ) -> Result<(B::Proof, B::PublicIO)> {
        self.backend.prove(program_id, private_inputs, public_inputs)
    }

    /// 通过后端验证证明
    pub fn verify_with_backend(
        &self,
        program_id: &B::ProgramId,
        proof: &B::Proof,
        public_inputs: &B::PublicIO,
        public_outputs: &B::PublicIO,
    ) -> Result<bool> {
        self.backend.verify(program_id, proof, public_inputs, public_outputs)
    }

    /// 获取后端名称
    pub fn backend_name(&self) -> &'static str {
        self.backend.backend_name()
    }
}

/// 工具方法：从状态轨迹构造承诺
pub fn commit_trace(trace: &ExecutionTrace) -> [u8; 32] {
    trace.commitment()
}
