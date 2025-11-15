use sha2::{Digest, Sha256};

use crate::proof::{AggregatedProof, Proof, TraceDigest};

/// 使用简单 Merkle 树的聚合器骨架
#[derive(Default)]
pub struct MerkleAggregator {
    leaves: Vec<TraceDigest>,
}

impl MerkleAggregator {
    pub fn new() -> Self {
        Self { leaves: Vec::new() }
    }

    pub fn add_proof(&mut self, proof: &Proof) {
        self.leaves.push(proof.trace_commitment);
    }

    pub fn finalize(&self) -> AggregatedProof {
        if self.leaves.is_empty() {
            return AggregatedProof::empty();
        }
        let mut layer = self.leaves.clone();
        while layer.len() > 1 {
            layer = layer
                .chunks(2)
                .map(|pair| {
                    if pair.len() == 1 {
                        hash_pair(pair[0], pair[0])
                    } else {
                        hash_pair(pair[0], pair[1])
                    }
                })
                .collect();
        }
        AggregatedProof { root: layer[0], proof_count: self.leaves.len() }
    }
}

fn hash_pair(left: TraceDigest, right: TraceDigest) -> TraceDigest {
    let mut hasher = Sha256::new();
    hasher.update(&left);
    hasher.update(&right);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use hex::ToHex;

    use super::*;
    use crate::{FibonacciProgram, TraceZkVm};

    #[test]
    fn aggregating_two_proofs_changes_root() {
        let vm = TraceZkVm::default();
        let program = FibonacciProgram::new(5);
        let proof_a = vm.prove(&program, &[1, 1]).unwrap();
        let proof_b = vm.prove(&program, &[2, 5]).unwrap();

        let mut aggregator = MerkleAggregator::new();
        aggregator.add_proof(&proof_a);
        aggregator.add_proof(&proof_b);
        let aggregated = aggregator.finalize();

        assert_eq!(aggregated.proof_count, 2);
        assert_ne!(aggregated.root.encode_hex::<String>(), "00".repeat(32));
    }
}
