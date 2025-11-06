// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Integration tests for privacy verifier abstraction

use vm_runtime::privacy::{NoopVerifier, ZkCircuitId, ZkVerifier};

#[test]
fn noop_verifier_is_false_by_default() {
    let v = NoopVerifier::default();
    let ok = v
        .verify_proof(&ZkCircuitId::from("ring_signature_v1"), b"proof", b"inputs")
        .expect("noop should not error");
    assert!(!ok, "NoopVerifier must return false by default");
}

// Feature-gated test for the optional Groth16 adapter
#[cfg(feature = "groth16-verifier")]
mod groth16_feature_tests {
    use super::*;
    use vm_runtime::privacy::{Groth16Verifier, ZkError};

    #[test]
    fn groth16_unknown_circuit_is_error() {
        let v = Groth16Verifier::new();
        let err = v
            .verify_proof(&ZkCircuitId::from("nonexistent_circuit"), b"p", b"i")
            .expect_err("should error for unknown circuit id");

        match err {
            ZkError::UnknownCircuit(id) => assert_eq!(id, "nonexistent_circuit"),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
