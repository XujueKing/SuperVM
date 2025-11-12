// SPDX-License-Identifier: GPL-3.0-or-later
// 统一 RangeProof 抽象：支持 Groth16 与 Bulletproofs 两种后端


/// 统一的范围证明接口
/// - 以关联类型形式暴露后端的 Proof 与 Commitment 类型
/// - 验证与批量验证的签名保持一致
pub trait RangeProofApi {
    type Proof;
    type Commitment;

    fn prove(&self, value: u64, n_bits: usize) -> Result<(Self::Proof, Self::Commitment), String>;
    fn verify(
        &self,
        proof: &Self::Proof,
        commitment: &Self::Commitment,
        n_bits: usize,
    ) -> Result<bool, String>;
    fn verify_batch(
        &self,
        proofs: &[Self::Proof],
        commitments: &[Self::Commitment],
        n_bits: usize,
    ) -> Result<bool, String>;
    fn proof_size(proof: &Self::Proof) -> usize;
}

// ------------------------ Bulletproofs Adapter ------------------------

use crate::bulletproofs_range_proof::BulletproofsRangeProver;
use bulletproofs::RangeProof as BpRangeProof;
use curve25519_dalek_ng::ristretto::CompressedRistretto as BpCommitment;
use curve25519_dalek_ng::scalar::Scalar;

impl RangeProofApi for BulletproofsRangeProver {
    type Proof = BpRangeProof;
    type Commitment = BpCommitment;

    fn prove(&self, value: u64, n_bits: usize) -> Result<(Self::Proof, Self::Commitment), String> {
        let blinding = Scalar::random(&mut rand::thread_rng());
        self.prove_range(value, &blinding, n_bits)
    }

    fn verify(
        &self,
        proof: &Self::Proof,
        commitment: &Self::Commitment,
        n_bits: usize,
    ) -> Result<bool, String> {
        self.verify_range(proof, commitment, n_bits)
    }

    fn verify_batch(
        &self,
        proofs: &[Self::Proof],
        commitments: &[Self::Commitment],
        n_bits: usize,
    ) -> Result<bool, String> {
        self.verify_batch(proofs, commitments, n_bits)
    }

    fn proof_size(proof: &Self::Proof) -> usize {
        // 使用 Bulletproofs 自带序列化（to_bytes）计算长度
        proof.to_bytes().len()
    }
}

// ------------------------ Groth16 Adapter ------------------------

use crate::range_proof::RangeProofCircuit;
use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16, PreparedVerifyingKey, ProvingKey};
use ark_snark::SNARK;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::Mutex;

/// Groth16 的 Commitment 在该模式下等价于公开输入（c = value）
pub struct Groth16RangeProver {
    // 按位数缓存参数，避免重复 Setup
    cache: Mutex<HashMap<usize, (ProvingKey<Bls12_381>, PreparedVerifyingKey<Bls12_381>)>>,
}

impl Groth16RangeProver {
    pub fn new() -> Self {
        Self { cache: Mutex::new(HashMap::new()) }
    }

    fn ensure_params(&self, n_bits: usize) -> Result<(ProvingKey<Bls12_381>, PreparedVerifyingKey<Bls12_381>), String> {
        // 尝试命中缓存
        if let Some(found) = self.cache.lock().unwrap().get(&n_bits).cloned() {
            return Ok(found);
        }
        // 生成并缓存
        let rng = &mut OsRng;
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            RangeProofCircuit::new(None, n_bits),
            rng,
        ).map_err(|e| format!("Groth16 setup failed: {:?}", e))?;
        let pvk = prepare_verifying_key(&params.vk);
        let pk = params;
        let pair = (pk, pvk);
        self.cache.lock().unwrap().insert(n_bits, (
            pair.0.clone(), pair.1.clone()
        ));
        Ok(pair)
    }
}

impl RangeProofApi for Groth16RangeProver {
    type Proof = ark_groth16::Proof<Bls12_381>;
    type Commitment = Fr; // 公开输入 c = value

    fn prove(&self, value: u64, n_bits: usize) -> Result<(Self::Proof, Self::Commitment), String> {
        let (pk, _) = self.ensure_params(n_bits)?;
        let rng = &mut OsRng;
        let proof = Groth16::<Bls12_381>::prove(
            &pk,
            RangeProofCircuit::new(Some(value), n_bits),
            rng,
        ).map_err(|e| format!("Groth16 prove failed: {:?}", e))?;
        Ok((proof, Fr::from(value)))
    }

    fn verify(
        &self,
        proof: &Self::Proof,
        commitment: &Self::Commitment,
        n_bits: usize,
    ) -> Result<bool, String> {
        let (_, pvk) = self.ensure_params(n_bits)?;
        let ok = Groth16::<Bls12_381>::verify_proof(&pvk, proof, &[*commitment])
            .map_err(|e| format!("Groth16 verify error: {:?}", e))?;
        if ok { Ok(true) } else { Err("Groth16 verification failed".into()) }
    }

    fn verify_batch(
        &self,
        proofs: &[Self::Proof],
        commitments: &[Self::Commitment],
        n_bits: usize,
    ) -> Result<bool, String> {
        if proofs.len() != commitments.len() {
            return Err(format!("Proofs {} != commitments {}", proofs.len(), commitments.len()));
        }
        let (_, pvk) = self.ensure_params(n_bits)?;
        for (p, c) in proofs.iter().zip(commitments.iter()) {
            let ok = Groth16::<Bls12_381>::verify_proof(&pvk, p, &[*c])
                .map_err(|e| format!("Groth16 verify error: {:?}", e))?;
            if !ok { return Err("Groth16 batch verification failed".into()); }
        }
        Ok(true)
    }

    fn proof_size(_proof: &Self::Proof) -> usize {
        // 近似：Groth16 (BLS12-381) 证明大小 ~ (G1 48 + G2 96 + G1 48) = 192 字节
        192
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_smoke_bulletproofs() {
        let bp = crate::bulletproofs_range_proof::BulletproofsRangeProver::new(64);
        let (proof, commit) = RangeProofApi::prove(&bp, 123456u64, 64).expect("bp prove");
        assert!(RangeProofApi::verify(&bp, &proof, &commit, 64).is_ok());
        let size = <crate::bulletproofs_range_proof::BulletproofsRangeProver as RangeProofApi>::proof_size(&proof);
        assert!(size > 0);
    }

    #[test]
    fn trait_smoke_groth16() {
        let g = Groth16RangeProver::new();
        let (proof, c) = RangeProofApi::prove(&g, 42u64, 8).expect("g16 prove");
        assert!(RangeProofApi::verify(&g, &proof, &c, 8).is_ok());
        let size = <Groth16RangeProver as RangeProofApi>::proof_size(&proof);
        assert!(size > 0);
    }
}
