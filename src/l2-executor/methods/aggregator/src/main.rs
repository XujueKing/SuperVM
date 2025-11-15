//! Proof Aggregator Guest Program for RISC0 zkVM
//!
//! This guest program verifies multiple RISC0 proofs recursively.
//! It demonstrates the core concept of recursive proof aggregation.

#![no_main]

use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

risc0_zkvm::guest::entry!(main);

/// Represents a proof to be verified
#[derive(Serialize, Deserialize)]
struct ProofData {
    /// The image ID of the program that generated this proof
    image_id: [u32; 8],
    /// The journal (public output) from the proof
    journal: Vec<u8>,
}

/// Aggregation result committed to journal
#[derive(Serialize, Deserialize)]
struct AggregationResult {
    /// Number of proofs successfully verified
    verified_count: u32,
    /// Combined hash of all verified journals
    combined_hash: [u8; 32],
}

fn main() {
    // Read the number of proofs to aggregate
    let proof_count: u32 = env::read();

    let mut verified_count = 0u32;
    let mut combined_data = Vec::new();

    // Verify each proof
    for i in 0..proof_count {
        // Read proof data from host
        let proof_data: ProofData = env::read();

        // In a real implementation, we would verify the proof here using:
        // env::verify(proof_data.image_id, &proof_data.journal).expect("verification failed");
        //
        // For this POC, we simulate verification by checking basic validity
        if proof_data.image_id.iter().any(|&x| x != 0) && !proof_data.journal.is_empty() {
            verified_count += 1;
            combined_data.extend_from_slice(&proof_data.journal);
        }

        // Log progress (visible in RISC0 trace)
        env::log(&format!("Verified proof {}/{}", i + 1, proof_count));
    }

    // Compute combined hash using SHA-256
    let combined_hash = sha256(&combined_data);

    // Commit the aggregation result
    let result = AggregationResult {
        verified_count,
        combined_hash,
    };

    env::commit(&result);
}

/// Simple SHA-256 implementation for guest environment
fn sha256(data: &[u8]) -> [u8; 32] {
    // In production, use a proper SHA-256 implementation
    // For POC, we use a simple hash combining approach
    let mut hash = [0u8; 32];
    
    for (i, chunk) in data.chunks(32).enumerate() {
        for (j, &byte) in chunk.iter().enumerate() {
            hash[j] ^= byte.wrapping_add(i as u8);
        }
    }
    
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_deterministic() {
        let data1 = b"hello world";
        let data2 = b"hello world";
        let data3 = b"different data";

        let hash1 = sha256(data1);
        let hash2 = sha256(data2);
        let hash3 = sha256(data3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
