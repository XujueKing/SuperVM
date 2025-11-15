//! 统一 zkVM 后端 trait 定义
//!
//! 提供抽象接口,支持多种 zkVM 实现 (RISC0, Halo2, SP1 等)

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 零知识虚拟机后端统一接口
///
/// 所有 zkVM 实现 (RISC0, Halo2, SP1 等) 都应实现此 trait,
/// 以便在 L2 执行层中可互换使用。
pub trait ZkVmBackend: Send + Sync {
    /// 证明类型 (每个后端有自己的证明格式)
    type Proof: Clone + Serialize + for<'de> Deserialize<'de>;
    
    /// 程序标识类型 (可以是哈希、ELF 引用等)
    type ProgramId: Clone;
    
    /// 公开输入/输出类型
    type PublicIO: Clone;

    /// 为给定程序和输入生成零知识证明
    ///
    /// # 参数
    /// - `program_id`: 程序标识 (如 ELF 哈希、电路描述等)
    /// - `private_inputs`: 私有输入 (witness)
    /// - `public_inputs`: 公开输入
    ///
    /// # 返回
    /// - `Ok(proof)`: 生成的证明及公开输出
    /// - `Err(e)`: 证明生成失败
    fn prove(
        &self,
        program_id: &Self::ProgramId,
        private_inputs: &[u8],
        public_inputs: &Self::PublicIO,
    ) -> Result<(Self::Proof, Self::PublicIO)>;

    /// 验证零知识证明
    ///
    /// # 参数
    /// - `program_id`: 程序标识
    /// - `proof`: 待验证的证明
    /// - `public_inputs`: 公开输入
    /// - `public_outputs`: 公开输出
    ///
    /// # 返回
    /// - `Ok(true)`: 验证通过
    /// - `Ok(false)`: 验证失败
    /// - `Err(e)`: 验证过程出错
    fn verify(
        &self,
        program_id: &Self::ProgramId,
        proof: &Self::Proof,
        public_inputs: &Self::PublicIO,
        public_outputs: &Self::PublicIO,
    ) -> Result<bool>;

    /// 获取后端名称 (如 "risc0", "halo2", "sp1")
    fn backend_name(&self) -> &'static str;

    /// 获取证明大小 (字节)
    fn proof_size(&self, proof: &Self::Proof) -> usize {
        bincode::serialize(proof)
            .map(|bytes| bytes.len())
            .unwrap_or(0)
    }

    /// 批量验证多个证明 (可选优化)
    ///
    /// 默认实现: 逐个调用 verify
    fn batch_verify(
        &self,
        proofs: &[(Self::ProgramId, Self::Proof, Self::PublicIO, Self::PublicIO)],
    ) -> Result<bool> {
        for (program_id, proof, inputs, outputs) in proofs {
            if !self.verify(program_id, proof, inputs, outputs)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// 证明聚合器接口 (可选特性)
///
/// 支持将多个证明压缩为单个递归证明
pub trait ProofAggregator: ZkVmBackend {
    /// 聚合多个证明为单个递归证明
    ///
    /// # 参数
    /// - `proofs`: 待聚合的证明列表
    ///
    /// # 返回
    /// - `Ok(aggregated_proof)`: 聚合后的单个证明
    /// - `Err(e)`: 聚合失败
    fn aggregate(&self, proofs: &[Self::Proof]) -> Result<Self::Proof>;

    /// 获取聚合压缩比 (如 100:1 表示可将 100 个证明压缩为 1 个)
    fn compression_ratio(&self) -> usize {
        100 // 默认目标压缩比
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock 实现用于测试 trait 约束
    #[derive(Clone, Serialize, Deserialize)]
    struct MockProof(Vec<u8>);

    struct MockBackend;

    impl ZkVmBackend for MockBackend {
        type Proof = MockProof;
        type ProgramId = String;
        type PublicIO = Vec<u64>;

        fn prove(
            &self,
            _program_id: &Self::ProgramId,
            _private_inputs: &[u8],
            public_inputs: &Self::PublicIO,
        ) -> Result<(Self::Proof, Self::PublicIO)> {
            Ok((MockProof(vec![0x42]), public_inputs.clone()))
        }

        fn verify(
            &self,
            _program_id: &Self::ProgramId,
            _proof: &Self::Proof,
            _public_inputs: &Self::PublicIO,
            _public_outputs: &Self::PublicIO,
        ) -> Result<bool> {
            Ok(true)
        }

        fn backend_name(&self) -> &'static str {
            "mock"
        }
    }

    #[test]
    fn trait_basic_usage() {
        let backend = MockBackend;
        let (proof, outputs) = backend
            .prove(&"test_program".to_string(), &[], &vec![42])
            .unwrap();
        assert!(backend
            .verify(&"test_program".to_string(), &proof, &vec![42], &outputs)
            .unwrap());
        assert_eq!(backend.backend_name(), "mock");
    }
}
