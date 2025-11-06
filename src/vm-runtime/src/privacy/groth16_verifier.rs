// SPDX-License-Identifier: GPL-3.0-or-later
// Optional Groth16 verifier adapter (feature = "groth16-verifier")

use super::{ZkVerifier, ZkCircuitId, ZkError};
use anyhow::Result;

/// Incremental Groth16 verifier adapter
/// Note: Real verification wiring can be added later without breaking callers.
pub struct Groth16Verifier;

impl Groth16Verifier {
    pub fn new() -> Self { Self }
}

impl ZkVerifier for Groth16Verifier {
    fn verify_proof(&self, circuit: &ZkCircuitId, _proof: &[u8], _public_inputs: &[u8]) -> Result<bool, ZkError> {
        // TODO: map circuit ids to verifying keys and call arkworks when available
        Err(ZkError::UnknownCircuit(circuit.0.clone()))
    }
}
