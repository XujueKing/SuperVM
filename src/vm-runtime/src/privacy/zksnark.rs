// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! ZK-SNARK Verifier Interface (scaffolding)
//! Phase 2.2.4: Provide a generic verifier abstraction that runtime can call.

use anyhow::Result;

/// Logical identifier of a circuit within the system
/// Example values: "ring_signature_v1", "range64_v1", "ringct_v1"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZkCircuitId(pub String);

impl From<&str> for ZkCircuitId {
    fn from(s: &str) -> Self { Self(s.to_owned()) }
}

/// Generic ZK verification error
#[derive(thiserror::Error, Debug)]
pub enum ZkError {
    #[error("invalid proof bytes")]
    InvalidProof,
    #[error("public inputs mismatch")]
    InvalidPublicInputs,
    #[error("unknown circuit: {0}")]
    UnknownCircuit(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Trait for pluggable ZK verifiers
///
/// Implementations could wrap different libraries/backends (arkworks/Halo2/etc.).
pub trait ZkVerifier: Send + Sync {
    /// Verify a proof for a given circuit with public inputs
    fn verify_proof(&self, circuit: &ZkCircuitId, proof: &[u8], public_inputs: &[u8]) -> Result<bool, ZkError>;
}

/// No-op verifier useful for tests or when ZK is not enabled yet
pub struct NoopVerifier;

impl Default for NoopVerifier { fn default() -> Self { Self } }

impl ZkVerifier for NoopVerifier {
    fn verify_proof(&self, _circuit: &ZkCircuitId, _proof: &[u8], _public_inputs: &[u8]) -> Result<bool, ZkError> {
        // By default, do not accept any proof. This makes calls explicit in tests.
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_verifier_always_false() {
        let v = NoopVerifier::default();
        let ok = v.verify_proof(&ZkCircuitId::from("ring_signature_v1"), b"proof", b"inputs").unwrap();
        assert!(!ok);
    }
}
