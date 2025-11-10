// SPDX-License-Identifier: GPL-3.0-or-later
// Optional Groth16 verifier adapter (feature = "groth16-verifier")

use super::{ZkCircuitId, ZkError, ZkVerifier};
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;

// Arkworks imports are only used when feature is enabled (this file is behind the feature)
use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{PreparedVerifyingKey, Proof};
use ark_serialize::CanonicalDeserialize;

/// Incremental Groth16 verifier adapter
/// Note: Real verification wiring can be added later without breaking callers.
pub struct Groth16Verifier {
    // Registry mapping circuit id -> verifier function
    // The verifier takes (proof_bytes, public_inputs_bytes) and returns whether valid.
    registry: DashMap<String, Arc<dyn Fn(&[u8], &[u8]) -> Result<bool, ZkError> + Send + Sync>>,
}

impl Groth16Verifier {
    /// Create an empty verifier registry. Call `register` to add circuits.
    pub fn new() -> Self {
        Self {
            registry: DashMap::new(),
        }
    }

    /// Register a circuit verifier handler.
    ///
    /// The handler should perform Groth16 verification for the specific circuit
    /// and return Ok(true/false) or a ZkError for malformed inputs.
    pub fn register<F>(&self, circuit: impl Into<String>, handler: F)
    where
        F: Fn(&[u8], &[u8]) -> Result<bool, ZkError> + Send + Sync + 'static,
    {
        self.registry.insert(circuit.into(), Arc::new(handler));
    }

    /// Register the built-in multiply_v1 circuit (a*b=c) with a provided verifying key (PVK).
    ///
    /// Inputs encoding contract:
    /// - proof_bytes: ark_serialize CanonicalSerialize of `ark_groth16::Proof<Bls12_381>`
    /// - public_inputs_bytes: ark_serialize CanonicalSerialize of a single `Fr` (c)
    pub fn register_multiply_v1_with_pvk(&self, pvk: PreparedVerifyingKey<Bls12_381>) {
        let pvk = Arc::new(pvk);
        self.register("multiply_v1", move |proof_bytes, public_inputs_bytes| {
            // Deserialize proof
            let mut pr = std::io::Cursor::new(proof_bytes);
            let proof = Proof::<Bls12_381>::deserialize_uncompressed_unchecked(&mut pr)
                .map_err(|_| ZkError::InvalidProof)?;

            // Deserialize a single Fr public input (c)
            let mut ir = std::io::Cursor::new(public_inputs_bytes);
            let c = Fr::deserialize_uncompressed_unchecked(&mut ir)
                .map_err(|_| ZkError::InvalidPublicInputs)?;

            // Verify
            ark_groth16::Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c])
                .map_err(|e| ZkError::Other(anyhow::anyhow!(e)))
        });
    }

    /// Generic registration: public_inputs are encoded as a length-prefixed vector of Fr
    /// Bytes layout: [u32_le length] [Fr0] [Fr1] ... where each Fr is CanonicalSerialize (uncompressed)
    pub fn register_circuit_with_pvk_fr_vec(
        &self,
        circuit: impl Into<String>,
        pvk: PreparedVerifyingKey<Bls12_381>,
    ) {
        let pvk = Arc::new(pvk);
        let name = circuit.into();
        self.register(name, move |proof_bytes, public_inputs_bytes| {
            // Deserialize proof
            let mut pr = std::io::Cursor::new(proof_bytes);
            let proof = Proof::<Bls12_381>::deserialize_uncompressed_unchecked(&mut pr)
                .map_err(|_| ZkError::InvalidProof)?;

            // Deserialize public inputs vec<Fr>
            if public_inputs_bytes.len() < 4 {
                return Err(ZkError::InvalidPublicInputs);
            }
            let mut ir = std::io::Cursor::new(public_inputs_bytes);
            use std::io::Read as _;
            let mut len_buf = [0u8; 4];
            ir.read_exact(&mut len_buf)
                .map_err(|_| ZkError::InvalidPublicInputs)?;
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut inputs: Vec<Fr> = Vec::with_capacity(len);
            for _ in 0..len {
                let fr = Fr::deserialize_uncompressed_unchecked(&mut ir)
                    .map_err(|_| ZkError::InvalidPublicInputs)?;
                inputs.push(fr);
            }

            ark_groth16::Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &inputs)
                .map_err(|e| ZkError::Other(anyhow::anyhow!(e)))
        });
    }

    /// Register ring_signature_v1 circuit with PVK and Poseidon config (not stored, for doc only).
    ///
    /// Public inputs encoding:
    /// - key_image: a single Fr (CanonicalSerialize uncompressed)
    ///
    /// Note: The circuit internally uses the same Poseidon config that was used during proving.
    /// The verifier only needs PVK; Poseidon config is captured in the circuit constraints.
    pub fn register_ring_signature_v1_with_pvk(&self, pvk: PreparedVerifyingKey<Bls12_381>) {
        let pvk = Arc::new(pvk);
        self.register(
            "ring_signature_v1",
            move |proof_bytes, public_inputs_bytes| {
                // Deserialize proof
                let mut pr = std::io::Cursor::new(proof_bytes);
                let proof = Proof::<Bls12_381>::deserialize_uncompressed_unchecked(&mut pr)
                    .map_err(|_| ZkError::InvalidProof)?;

                // Deserialize key_image (single Fr)
                let mut ir = std::io::Cursor::new(public_inputs_bytes);
                let key_image = Fr::deserialize_uncompressed_unchecked(&mut ir)
                    .map_err(|_| ZkError::InvalidPublicInputs)?;

                // Verify
                ark_groth16::Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[key_image])
                    .map_err(|e| ZkError::Other(anyhow::anyhow!(e)))
            },
        );
    }

    /// Register range_proof_v1 circuit with PVK.
    /// Public inputs encoding: single Fr value `c` representing the committed value.
    pub fn register_range_proof_v1_with_pvk(&self, pvk: PreparedVerifyingKey<Bls12_381>) {
        let pvk = Arc::new(pvk);
        self.register("range_proof_v1", move |proof_bytes, public_inputs_bytes| {
            let mut pr = std::io::Cursor::new(proof_bytes);
            let proof = Proof::<Bls12_381>::deserialize_uncompressed_unchecked(&mut pr)
                .map_err(|_| ZkError::InvalidProof)?;

            let mut ir = std::io::Cursor::new(public_inputs_bytes);
            let c = Fr::deserialize_uncompressed_unchecked(&mut ir)
                .map_err(|_| ZkError::InvalidPublicInputs)?;

            ark_groth16::Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c])
                .map_err(|e| ZkError::Other(anyhow::anyhow!(e)))
        });
    }

    /// Register ringct_v1 (SimpleRingCT) circuit with PVK.
    ///
    /// Public inputs encoding (length-prefixed vec of Fr):
    /// - [u32_le length=5]
    /// - input_commitment_x: Fr
    /// - input_commitment_y: Fr
    /// - output_commitment_x: Fr
    /// - output_commitment_y: Fr
    /// - merkle_root: Fr
    ///
    /// Uses the generic vec encoding path.
    pub fn register_ringct_v1_with_pvk(&self, pvk: PreparedVerifyingKey<Bls12_381>) {
        self.register_circuit_with_pvk_fr_vec("ringct_v1", pvk);
    }
}

impl ZkVerifier for Groth16Verifier {
    fn verify_proof(
        &self,
        circuit: &ZkCircuitId,
        proof: &[u8],
        public_inputs: &[u8],
    ) -> Result<bool, ZkError> {
        if let Some(handler) = self.registry.get(&circuit.0) {
            (handler.value())(proof, public_inputs)
        } else {
            Err(ZkError::UnknownCircuit(circuit.0.clone()))
        }
    }
}
