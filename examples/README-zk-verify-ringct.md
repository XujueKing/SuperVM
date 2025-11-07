# zk_verify_ringct Example

End-to-end demo for Groth16 verification of ringct_v1 (SimpleRingCT: single input/output) under the feature flag `groth16-verifier`.

What it does:
- Generates an example SimpleRingCT circuit with:
  - 1 input UTXO (Pedersen commitment)
  - 1 output UTXO (Pedersen commitment)
  - Merkle proof for ring membership (ring size=5, depth=3)
- Runs trusted setup for the RingCT circuit
- Serializes VerifyingKey (VK), Proof, and public inputs (5 Fr values)
- Reloads them from disk, prepares PVK, registers the circuit, and verifies via `Groth16Verifier`
- Also checks a wrong public inputs case (expected to be false)

## Run (Windows PowerShell)

```powershell
# Run with feature enabled
cargo run -p vm-runtime --features groth16-verifier --example zk_verify_ringct
```

Expected output (abridged):
- Prints input/output commitment coordinates and merkle root
- Writes files under `target/tmp/zk_ringct_demo` (vk.bin, proof.bin, public_inputs.bin)
- Prints:
  - Verify with correct public inputs => true
  - Verify with wrong public inputs => false

## Public Inputs Protocol

RingCT v1 has **5 public Fr values** encoded with length prefix:
- `[u32_le length=5]`
- `input_commitment_x`: Fr (x coordinate of input Pedersen commitment)
- `input_commitment_y`: Fr (y coordinate of input Pedersen commitment)
- `output_commitment_x`: Fr (x coordinate of output Pedersen commitment)
- `output_commitment_y`: Fr (y coordinate of output Pedersen commitment)
- `merkle_root`: Fr (Merkle tree root for ring membership)

This uses the **generic vec encoding** path (`register_circuit_with_pvk_fr_vec`).

## Notes
- SimpleRingCT combines ring signature (sender anonymity), Pedersen commitment (amount hiding), and range proof (prevent negative values).
- The circuit uses Poseidon hash for Merkle tree (simplified parameters for demo; production should use standard parameters).
- Pedersen commitments use `ark-ed-on-bls12-381-bandersnatch` curve.
- The runtime's `Groth16Verifier` remains feature-gated and does not affect L1 surfaces by default.
