# zk_verify_multiply Example

A minimal end-to-end demo for Groth16 verification (multiply_v1: a*b=c) under the feature flag `groth16-verifier`.

What it does:
- Runs trusted setup for a tiny multiply circuit
- Generates a proof for a=3, b=5 => c=15
- Serializes VerifyingKey (VK), Proof, and public input c to files
- Reloads them from disk, prepares PVK from VK, registers the circuit, and verifies via `Groth16Verifier`
- Also checks a wrong public input case (expected to be false)

## Run (Windows PowerShell)

```powershell
# Run with feature enabled
cargo run -p vm-runtime --features groth16-verifier --example zk_verify_multiply
```

Expected output (abridged):
- Writes files under `target/tmp/zk_multiply_demo` (vk.bin, proof.bin, c.bin)
- Prints:
  - Verify with correct c => true
  - Verify with wrong c => false

## Notes
- This example uses arkworks (ark-groth16 on BLS12-381) and the optional `zk-groth16-test` crate’s `MultiplyCircuit`.
- In production, you should persist VerifyingKey (or PreparedVerifyingKey) from a trusted setup and distribute it alongside the app; ProvingKey must be kept private or generated on the prover side only.
- The runtime’s `Groth16Verifier` remains feature-gated and does not affect L1 surfaces by default.
