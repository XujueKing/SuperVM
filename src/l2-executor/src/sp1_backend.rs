//! SP1 zkVM Backend Implementation
//!
//! This module provides a SP1-based implementation of the ZkVmBackend trait.
//! SP1 is an alternative zero-knowledge virtual machine that offers different
//! performance characteristics compared to RISC0.

use crate::backend::{ZkVmBackend, ZkVmProof, ZkVmError};
use anyhow::Result;

#[cfg(feature = "sp1-poc")]
use sp1_sdk::{ProverClient, SP1Stdin, SP1ProofWithPublicValues};

/// SP1 zkVM Backend
///
/// # Features
/// - Faster proof generation than RISC0 (in some scenarios)
/// - Different circuit design (PLONKish vs STARKs)
/// - Smaller proof sizes
/// - Active development and optimization
pub struct Sp1Backend {
    #[cfg(feature = "sp1-poc")]
    client: ProverClient,
    
    /// Program ELF binary
    program_elf: Vec<u8>,
}

impl Sp1Backend {
    /// Create a new SP1 backend with the given program
    ///
    /// # Arguments
    /// - `program_elf`: The ELF binary of the guest program
    ///
    /// # Example
    /// ```ignore
    /// let backend = Sp1Backend::new(FIBONACCI_ELF.to_vec());
    /// ```
    pub fn new(program_elf: Vec<u8>) -> Self {
        Self {
            #[cfg(feature = "sp1-poc")]
            client: ProverClient::new(),
            program_elf,
        }
    }

    #[cfg(feature = "sp1-poc")]
    fn prove_impl(&self, witness: &[u8]) -> Result<ZkVmProof> {
        // Create stdin and write witness data
        let mut stdin = SP1Stdin::new();
        stdin.write_vec(witness.to_vec());

        // Execute the program and generate proof
        let (public_values, proof) = self.client
            .prove(&self.program_elf, stdin)
            .map_err(|e| ZkVmError::ProvingError(e.to_string()))?;

        // Extract proof bytes
        let proof_bytes = bincode::serialize(&proof)
            .map_err(|e| ZkVmError::SerializationError(e.to_string()))?;

        Ok(ZkVmProof {
            proof_data: proof_bytes,
            public_output: public_values.to_vec(),
        })
    }

    #[cfg(feature = "sp1-poc")]
    fn verify_impl(&self, proof: &ZkVmProof) -> Result<bool> {
        // Deserialize proof
        let sp1_proof: SP1ProofWithPublicValues = bincode::deserialize(&proof.proof_data)
            .map_err(|e| ZkVmError::DeserializationError(e.to_string()))?;

        // Verify the proof
        self.client
            .verify(&sp1_proof, &self.program_elf)
            .map_err(|e| ZkVmError::VerificationError(e.to_string()))?;

        Ok(true)
    }

    #[cfg(not(feature = "sp1-poc"))]
    fn prove_impl(&self, _witness: &[u8]) -> Result<ZkVmProof> {
        Err(ZkVmError::BackendNotAvailable("SP1 backend not enabled. Enable 'sp1-poc' feature.".to_string()).into())
    }

    #[cfg(not(feature = "sp1-poc"))]
    fn verify_impl(&self, _proof: &ZkVmProof) -> Result<bool> {
        Err(ZkVmError::BackendNotAvailable("SP1 backend not enabled. Enable 'sp1-poc' feature.".to_string()).into())
    }
}

impl ZkVmBackend for Sp1Backend {
    fn prove(&self, _program: &[u8], witness: &[u8]) -> Result<ZkVmProof> {
        self.prove_impl(witness)
    }

    fn verify(&self, _program: &[u8], proof: &ZkVmProof) -> Result<bool> {
        self.verify_impl(proof)
    }

    fn backend_name(&self) -> &'static str {
        "SP1"
    }

    fn estimate_proving_time(&self, _program_size: usize) -> std::time::Duration {
        // SP1 is generally faster than RISC0
        // Rough estimate: ~50ms per 1KB program
        std::time::Duration::from_millis(50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sp1_backend_creation() {
        let program = vec![0u8; 100];
        let backend = Sp1Backend::new(program);
        assert_eq!(backend.backend_name(), "SP1");
    }

    #[test]
    fn test_sp1_backend_not_enabled() {
        #[cfg(not(feature = "sp1-poc"))]
        {
            let program = vec![0u8; 100];
            let backend = Sp1Backend::new(program);
            let witness = vec![1, 2, 3];
            
            let result = backend.prove(&[], &witness);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not enabled"));
        }
    }

    #[test]
    #[cfg(feature = "sp1-poc")]
    fn test_sp1_prove_verify() {
        // This test requires SP1 toolchain and guest programs
        // Skip if not in proper environment
        if std::env::var("SP1_DEV_MODE").is_ok() {
            // Placeholder for actual test implementation
            // Would use real SP1 guest program here
        }
    }
}
