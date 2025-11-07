# zk_verify_range_proof Example

End-to-end demo for Groth16 verification of range_proof_v1 (64-bit) under the feature flag `groth16-verifier`.

What it does:
- Runs trusted setup for a simple 64-bit range proof circuit
- Proves a value v (e.g., 12345678901234) is in [0, 2^64)
- Serializes VerifyingKey (VK), Proof, and public input c (= v)
- Reloads them from disk, prepares PVK, registers the circuit, and verifies via `Groth16Verifier`
- Also checks a wrong c case (expected to be false)

## Run (Windows PowerShell)

```powershell
# Run with feature enabled
cargo run -p vm-runtime --features groth16-verifier --example zk_verify_range_proof
```

Expected output (abridged):
- Writes files under `target/tmp/zk_range_proof_demo` (vk.bin, proof.bin, c.bin)
- Prints:
  - Verify with correct c => true
  - Verify with wrong c => false

## Public Inputs Protocol

- **c**: single Fr (CanonicalSerialize uncompressed), representing the value proven to be in range.

## Notes
- The circuit parameter `n_bits` is decided at setup time and is captured by the verifying key; no need to encode it in public inputs.
- The runtime's `Groth16Verifier` remains feature-gated and does not affect L1 surfaces by default.
