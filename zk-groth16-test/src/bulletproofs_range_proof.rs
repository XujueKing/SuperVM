// SPDX-License-Identifier: GPL-3.0-or-later
// Bulletproofs Range Proof 实现 (透明Setup)
// 用途: 证明某值在 [0, 2^n) 范围内，无需Trusted Setup

use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek_ng::ristretto::CompressedRistretto;
use curve25519_dalek_ng::scalar::Scalar;
use merlin::Transcript;
use rand::rngs::OsRng;
use std::time::Instant;

/// Bulletproofs Range Proof 证明器
pub struct BulletproofsRangeProver {
    /// Bulletproofs生成器 (支持最多max_bits位)
    bp_gens: BulletproofGens,
    /// Pedersen承诺生成器
    pc_gens: PedersenGens,
    /// 支持的最大位数
    max_bits: usize,
}

impl BulletproofsRangeProver {
    /// 创建新的证明器
    /// 
    /// # 参数
    /// - `max_bits`: 支持的最大位数 (通常为64)
    pub fn new(max_bits: usize) -> Self {
        Self {
            bp_gens: BulletproofGens::new(max_bits, 1), // 1个承诺
            pc_gens: PedersenGens::default(),
            max_bits,
        }
    }

    /// 生成范围证明
    /// 
    /// # 参数
    /// - `value`: 要证明的值 (必须在 [0, 2^n_bits) 范围内)
    /// - `blinding`: 盲化因子 (用于Pedersen承诺)
    /// - `n_bits`: 范围位数 (必须 <= max_bits)
    /// 
    /// # 返回
    /// - `Ok((proof, commitment))`: 证明和承诺
    /// - `Err(msg)`: 错误信息
    pub fn prove_range(
        &self,
        value: u64,
        blinding: &Scalar,
        n_bits: usize,
    ) -> Result<(RangeProof, CompressedRistretto), String> {
        if n_bits > self.max_bits {
            return Err(format!(
                "n_bits {} exceeds max_bits {}",
                n_bits, self.max_bits
            ));
        }

        // 检查值是否在范围内
        if n_bits < 64 && value >= (1u64 << n_bits) {
            return Err(format!("Value {} out of range [0, 2^{})", value, n_bits));
        }

        let mut transcript = Transcript::new(b"SuperVM-Bulletproofs-RangeProof");

        // 生成证明
        let (proof, commitment) = RangeProof::prove_single(
            &self.bp_gens,
            &self.pc_gens,
            &mut transcript,
            value,
            blinding,
            n_bits,
        )
        .map_err(|e| format!("Bulletproofs prove error: {:?}", e))?;

        Ok((proof, commitment))
    }

    /// 生成范围证明 (自动生成随机盲化因子)
    pub fn prove_range_auto_blinding(
        &self,
        value: u64,
        n_bits: usize,
    ) -> Result<(RangeProof, CompressedRistretto, Scalar), String> {
        let blinding = Scalar::random(&mut rand::thread_rng());
        let (proof, commitment) = self.prove_range(value, &blinding, n_bits)?;
        Ok((proof, commitment, blinding))
    }

    /// 验证范围证明
    /// 
    /// # 参数
    /// - `proof`: 范围证明
    /// - `commitment`: Pedersen承诺
    /// - `n_bits`: 范围位数
    /// 
    /// # 返回
    /// - `Ok(true)`: 验证通过
    /// - `Err(msg)`: 验证失败
    pub fn verify_range(
        &self,
        proof: &RangeProof,
        commitment: &CompressedRistretto,
        n_bits: usize,
    ) -> Result<bool, String> {
        let mut transcript = Transcript::new(b"SuperVM-Bulletproofs-RangeProof");

        proof
            .verify_single(
                &self.bp_gens,
                &self.pc_gens,
                &mut transcript,
                commitment,
                n_bits,
            )
            .map_err(|e| format!("Bulletproofs verify error: {:?}", e))?;

        Ok(true)
    }

    /// 批量验证范围证明 (关键优化!)
    /// 
    /// Bulletproofs批量验证比逐个验证快很多 (约3-5倍)
    /// 
    /// # 参数
    /// - `proofs`: 证明列表
    /// - `commitments`: 承诺列表
    /// - `n_bits`: 范围位数 (所有证明必须相同)
    pub fn verify_batch(
        &self,
        proofs: &[RangeProof],
        commitments: &[CompressedRistretto],
        n_bits: usize,
    ) -> Result<bool, String> {
        if proofs.len() != commitments.len() {
            return Err(format!(
                "Proofs count {} != commitments count {}",
                proofs.len(),
                commitments.len()
            ));
        }

        // Bulletproofs 4.0 的批量验证：逐个验证但共享生成器
        // 注意: bulletproofs 5.0+ 才有真正的批量验证API
        for (proof, commitment) in proofs.iter().zip(commitments.iter()) {
            let mut transcript = Transcript::new(b"SuperVM-Bulletproofs-RangeProof");
            proof
                .verify_single(
                    &self.bp_gens,
                    &self.pc_gens,
                    &mut transcript,
                    commitment,
                    n_bits,
                )
                .map_err(|e| format!("Bulletproofs batch verify error: {:?}", e))?;
        }

        Ok(true)
    }

    /// 获取证明大小 (字节)
    pub fn proof_size(proof: &RangeProof) -> usize {
        // Bulletproofs证明大小约为 2*log2(n) * 32 + 5*32 bytes
        // 对于64-bit: 约 672-736 bytes
        // 这里返回序列化后的近似大小
        let bytes = bincode::serialize(proof).unwrap_or_default();
        bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_64bit_range_proof() {
        let prover = BulletproofsRangeProver::new(64);

        // 测试有效值 (在范围内)
        let value = 12345678u64;
        let (proof, commitment, _blinding) = prover
            .prove_range_auto_blinding(value, 64)
            .expect("Prove failed");

        // 验证
        assert!(prover.verify_range(&proof, &commitment, 64).is_ok());

        println!("✓ 64-bit Range Proof: value={}, proof_size={} bytes", 
                 value, BulletproofsRangeProver::proof_size(&proof));
    }

    #[test]
    fn test_32bit_range_proof() {
        let prover = BulletproofsRangeProver::new(64);

        // 测试32位范围
        let value = 42u64;
        let (proof, commitment, _) = prover
            .prove_range_auto_blinding(value, 32)
            .expect("Prove failed");

        assert!(prover.verify_range(&proof, &commitment, 32).is_ok());

        println!("✓ 32-bit Range Proof: value={}, proof_size={} bytes", 
                 value, BulletproofsRangeProver::proof_size(&proof));
    }

    #[test]
    fn test_out_of_range_fails() {
        let prover = BulletproofsRangeProver::new(64);

        // 尝试证明超出范围的值 (256 >= 2^8)
        let result = prover.prove_range_auto_blinding(256, 8);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of range"));
    }

    #[test]
    fn test_batch_verification() {
        let prover = BulletproofsRangeProver::new(64);

        // 生成10个证明
        let values = vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
        let mut proofs = Vec::new();
        let mut commitments = Vec::new();

        for &value in &values {
            let (proof, commitment, _) = prover
                .prove_range_auto_blinding(value, 64)
                .expect("Prove failed");
            proofs.push(proof);
            commitments.push(commitment);
        }

        // 批量验证
        let start = Instant::now();
        assert!(prover.verify_batch(&proofs, &commitments, 64).is_ok());
        let batch_time = start.elapsed();

        println!("✓ Batch verification: {} proofs in {:?}", proofs.len(), batch_time);
        println!("  Average: {:?} per proof", batch_time / proofs.len() as u32);
    }

    #[test]
    fn test_invalid_proof_fails() {
        let prover = BulletproofsRangeProver::new(64);

        // 生成有效证明
        let (proof, commitment, _) = prover
            .prove_range_auto_blinding(100, 64)
            .expect("Prove failed");

        // 篡改承诺 (创建无效的承诺)
        let mut fake_commitment_bytes = commitment.to_bytes();
        fake_commitment_bytes[0] ^= 0xFF; // 翻转第一个字节
        let fake_commitment = CompressedRistretto(fake_commitment_bytes);

        // 验证应该失败
        let result = prover.verify_range(&proof, &fake_commitment, 64);
        assert!(result.is_err());
    }

    #[test]
    fn test_proof_size_comparison() {
        let prover = BulletproofsRangeProver::new(64);

        // 测试不同位数的证明大小
        for n_bits in [8, 16, 32, 64] {
            let (proof, _, _) = prover
                .prove_range_auto_blinding(100, n_bits)
                .expect("Prove failed");
            let size = BulletproofsRangeProver::proof_size(&proof);
            println!("{}-bit Range Proof size: {} bytes", n_bits, size);
        }

        // Bulletproofs证明大小是对数增长: O(log n_bits)
        // 预期: 8-bit ≈ 640B, 16-bit ≈ 672B, 32-bit ≈ 704B, 64-bit ≈ 736B
    }
}
