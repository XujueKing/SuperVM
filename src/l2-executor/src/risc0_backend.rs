#![cfg(feature = "risc0-poc")]

use anyhow::{anyhow, Result};
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};
use serde::{Deserialize, Serialize};

use crate::backend_trait::ZkVmBackend;

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

/// RISC0 证明结果
#[derive(Clone, Serialize, Deserialize)]
pub struct Risc0Proof {
    receipt: Receipt,
    pub public_output: u64,
}

impl Risc0Proof {
    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }
}

/// RISC0 zkVM PoC 后端
pub struct Risc0Backend;

impl Risc0Backend {
    pub fn new() -> Self {
        Self
    }

    pub fn prove_fibonacci(&self, a0: u64, a1: u64, rounds: u32) -> Result<Risc0Proof> {
        let env = ExecutorEnv::builder().write(&(a0, a1, rounds))?.build()?;
        let prove_info = default_prover().prove(env, L2_EXECUTOR_METHODS_FIBONACCI_ELF)?;
        let output: u64 = prove_info
            .receipt
            .journal
            .decode()
            .map_err(|e| anyhow!("journal decode failed: {e}"))?;
        Ok(Risc0Proof {
            receipt: prove_info.receipt,
            public_output: output,
        })
    }

    pub fn verify_fibonacci(&self, proof: &Risc0Proof) -> Result<()> {
        proof.receipt.verify(L2_EXECUTOR_METHODS_FIBONACCI_ID)?;
        Ok(())
    }
}

/// 实现 ZkVmBackend trait
impl ZkVmBackend for Risc0Backend {
    type Proof = Risc0Proof;
    type ProgramId = [u32; 8]; // RISC0 ImageID
    type PublicIO = Vec<u64>;

    fn prove(
        &self,
        program_id: &Self::ProgramId,
        private_inputs: &[u8],
        _public_inputs: &Self::PublicIO,
    ) -> Result<(Self::Proof, Self::PublicIO)> {
        // 目前仅支持 Fibonacci 程序
        if program_id != &L2_EXECUTOR_METHODS_FIBONACCI_ID {
            return Err(anyhow!("Unsupported program ID"));
        }

        // 解析私有输入: (a0, a1, rounds)
        if private_inputs.len() < 20 {
            return Err(anyhow!("Invalid input length"));
        }
        let a0 = u64::from_le_bytes(private_inputs[0..8].try_into()?);
        let a1 = u64::from_le_bytes(private_inputs[8..16].try_into()?);
        let rounds = u32::from_le_bytes(private_inputs[16..20].try_into()?);

        let proof = self.prove_fibonacci(a0, a1, rounds)?;
        let outputs = vec![proof.public_output];
        Ok((proof, outputs))
    }

    fn verify(
        &self,
        program_id: &Self::ProgramId,
        proof: &Self::Proof,
        _public_inputs: &Self::PublicIO,
        public_outputs: &Self::PublicIO,
    ) -> Result<bool> {
        if program_id != &L2_EXECUTOR_METHODS_FIBONACCI_ID {
            return Ok(false);
        }

        // 验证 receipt
        self.verify_fibonacci(proof)?;

        // 验证输出匹配
        if public_outputs.len() != 1 || public_outputs[0] != proof.public_output {
            return Ok(false);
        }

        Ok(true)
    }

    fn backend_name(&self) -> &'static str {
        "risc0"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risc0_fibonacci_roundtrip() {
        let backend = Risc0Backend::new();
        // fibonacci(0, 1, 10) → 10 iterations: 0,1,1,2,3,5,8,13,21,34,55,89
        // After 10 iterations starting from (0,1), result is 89
        let proof = backend.prove_fibonacci(0, 1, 10).expect("prove");
        backend.verify_fibonacci(&proof).expect("verify");
        assert_eq!(proof.public_output, 89);
    }

    #[test]
    fn zkvm_backend_trait_usage() {
        let backend = Risc0Backend::new();
        assert_eq!(backend.backend_name(), "risc0");

        // 准备输入: (a0=0, a1=1, rounds=10)
        let mut private_inputs = Vec::new();
        private_inputs.extend_from_slice(&0u64.to_le_bytes());
        private_inputs.extend_from_slice(&1u64.to_le_bytes());
        private_inputs.extend_from_slice(&10u32.to_le_bytes());

        // 通过 trait 接口证明
        let (proof, outputs) = backend
            .prove(&L2_EXECUTOR_METHODS_FIBONACCI_ID, &private_inputs, &vec![])
            .expect("prove via trait");

        assert_eq!(outputs, vec![89]);

        // 通过 trait 接口验证
        let verified = backend
            .verify(&L2_EXECUTOR_METHODS_FIBONACCI_ID, &proof, &vec![], &outputs)
            .expect("verify via trait");

        assert!(verified);
    }
}
