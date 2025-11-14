//! SuperVM L2 执行层核心骨架
//!
//! 提供 zkVM 执行、证明生成与聚合的最小实现，用于 Phase 8 原型。

#[cfg(all(feature = "risc0-poc", target_os = "windows"))]
compile_error!("`risc0-poc` feature requires a non-Windows host; please build on Linux or WSL");

pub mod proof;
pub mod program;
pub mod zkvm;
pub mod aggregator;
pub mod backend_trait;
pub mod runtime;
pub mod optimized;
pub mod aggregation;
pub mod metrics; // Session 13: 递归聚合策略

#[cfg(feature = "risc0-poc")]
pub mod risc0_backend;

#[cfg(feature = "sp1-poc")]
pub mod sp1_backend;

pub use proof::{AggregatedProof, Proof, TraceDigest};
pub use program::{ExecutionTrace, FibonacciProgram, Sha256Program, TraceProgram};
pub use zkvm::{TraceZkVm, PluggableZkVm};
pub use aggregator::MerkleAggregator;
pub use backend_trait::{ZkVmBackend, ProofAggregator};
pub use runtime::{L2Runtime, RuntimeConfig, BackendType};
pub use optimized::{BatchProcessor, CachedZkVm, CacheStats};

#[cfg(feature = "risc0-poc")]
pub use risc0_backend::{Risc0Backend, Risc0Proof};

#[cfg(feature = "sp1-poc")]
pub use sp1_backend::Sp1Backend;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fibonacci_proof_roundtrip() {
        let program = FibonacciProgram::new(10);
        let vm = TraceZkVm::default();
        let witness = vec![0, 1];
        let proof = vm.prove(&program, &witness).expect("prove");
        assert!(vm.verify(&program, &proof, &witness).expect("verify"));
        assert_eq!(proof.public_outputs, vec![55]);
    }

    #[test]
    fn aggregator_combines_proofs() {
        let program = FibonacciProgram::new(6);
        let vm = TraceZkVm::default();
        let proof_a = vm.prove(&program, &[0, 1]).unwrap();
        let proof_b = vm.prove(&program, &[2, 3]).unwrap();

        let mut aggregator = MerkleAggregator::new();
        aggregator.add_proof(&proof_a);
        aggregator.add_proof(&proof_b);
        let aggregated = aggregator.finalize();

        assert_eq!(aggregated.proof_count, 2);
        assert_ne!(aggregated.root, [0u8; 32]);
    }

    #[test]
    fn sha256_proof_roundtrip() {
        let program = Sha256Program::default();
        let vm = TraceZkVm::default();
        let witness = vec![0x6162636465666768u64]; // ASCII 'abcdefgh'
        let proof = vm.prove(&program, &witness).expect("prove");
        assert!(vm.verify(&program, &proof, &witness).expect("verify"));
        assert_eq!(proof.program_id, "sha256.v0");
        assert!(proof.steps >= 2);
    }
}
