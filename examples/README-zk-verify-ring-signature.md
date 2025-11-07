# zk_verify_ring_signature Example

End-to-end demo for Groth16 verification of ring_signature_v1 (ring size=5) under the feature flag `groth16-verifier`.

What it does:
- Sets up Poseidon config for hashing (same parameters as in zk-groth16-test)
- Generates a ring signature with 5 members (real signer at index 2)
- Runs trusted setup for the ring signature circuit
- Serializes VerifyingKey (VK), Proof, and public input (key_image) to files
- Reloads them from disk, prepares PVK, registers the circuit, and verifies via `Groth16Verifier`
- Also checks a wrong key_image case (expected to be false)

## Run (Windows PowerShell)

```powershell
# Run with feature enabled
cargo run -p vm-runtime --features groth16-verifier --example zk_verify_ring_signature
```

Expected output (abridged):
- Writes files under `target/tmp/zk_ring_signature_demo` (vk.bin, proof.bin, key_image.bin)
- Prints:
  - Verify with correct key_image => true
  - Verify with wrong key_image => false

## Public Inputs Protocol

- **key_image**: single Fr (CanonicalSerialize uncompressed)
  - The circuit verifies that the prover knows a secret key corresponding to one of the ring members and that the key_image is derived correctly from that secret.

## Notes
- This example uses arkworks (ark-groth16 on BLS12-381) with Poseidon hash for key image generation.
- In production, you should persist VerifyingKey from a trusted setup and distribute it; ProvingKey must be kept private.
- The runtime's `Groth16Verifier` remains feature-gated and does not affect L1 surfaces by default.
- Ring signature provides sender anonymity: the verifier learns that one of the ring members signed, but not which one.
