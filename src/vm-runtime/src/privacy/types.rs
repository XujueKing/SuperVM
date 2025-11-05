// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

// SuperVM 2.0 - Privacy Types
// 架构师: KING XU (CHINA)
//
// 定义隐私层的基础类型

use serde::{Deserialize, Serialize};

/// 公钥 (32 bytes, Ed25519)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey(pub [u8; 32]);

/// 私钥 (32 bytes)
#[derive(Clone, Serialize, Deserialize)]
pub struct SecretKey(pub [u8; 32]);

// 不打印私钥内容
impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKey([REDACTED])")
    }
}

/// Key Image (防止双花)
/// 每个 UTXO 只能生成一次唯一的 Key Image
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyImage(pub [u8; 32]);

/// 环签名
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingSignature {
    /// Ring members (public keys)
    pub ring: Vec<PublicKey>,
    /// 签名数据
    pub signature: Vec<u8>,
    /// Key image (防止双花)
    pub key_image: KeyImage,
}

/// 隐形地址
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthAddress {
    /// 一次性公钥
    pub public_key: PublicKey,
    /// 交易公钥 (用于接收方扫描)
    pub tx_public_key: PublicKey,
}

/// Pedersen Commitment (承诺)
/// C = aG + bH, 其中 a 是金额, b 是致盲因子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment(pub [u8; 32]);

/// Range Proof (范围证明)
/// 证明承诺的金额在 [0, 2^64) 范围内
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeProof {
    /// 证明数据 (Bulletproof format)
    pub proof: Vec<u8>,
}

/// 隐私交易输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyInput {
    /// Key image (防止双花)
    pub key_image: KeyImage,
    /// Ring signature (隐藏真实花费者)
    pub ring_signature: RingSignature,
    /// 金额承诺
    pub commitment: Commitment,
}

/// 隐私交易输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyOutput {
    /// 隐形地址 (隐藏接收方)
    pub stealth_address: StealthAddress,
    /// 金额承诺 (隐藏金额)
    pub commitment: Commitment,
    /// 范围证明 (证明金额有效)
    pub range_proof: RangeProof,
    /// 加密金额 (仅接收方可解密)
    pub encrypted_amount: Vec<u8>,
}

/// 完整的隐私交易 (RingCT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyTransaction {
    /// 版本号
    pub version: u32,
    /// 输入列表
    pub inputs: Vec<PrivacyInput>,
    /// 输出列表
    pub outputs: Vec<PrivacyOutput>,
    /// 交易费 (明文)
    pub fee: u64,
    /// 额外数据
    pub extra: Vec<u8>,
}

/// 钱包密钥对
#[derive(Clone, Serialize, Deserialize)]
pub struct WalletKeys {
    /// 花费私钥
    pub spend_secret: SecretKey,
    /// 花费公钥
    pub spend_public: PublicKey,
    /// 查看私钥 (用于扫描交易)
    pub view_secret: SecretKey,
    /// 查看公钥
    pub view_public: PublicKey,
}

impl std::fmt::Debug for WalletKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletKeys")
            .field("spend_public", &self.spend_public)
            .field("view_public", &self.view_public)
            .field("spend_secret", &"[REDACTED]")
            .field("view_secret", &"[REDACTED]")
            .finish()
    }
}

impl PublicKey {
    /// 创建零公钥 (用于测试)
    pub fn zero() -> Self {
        PublicKey([0u8; 32])
    }
    
    /// 从字节数组创建
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        PublicKey(bytes)
    }
    
    /// 转换为字节数组
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl SecretKey {
    /// 创建零私钥 (仅用于测试,实际使用需要安全随机数)
    #[cfg(test)]
    pub fn zero() -> Self {
        SecretKey([0u8; 32])
    }
    
    /// 从字节数组创建
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        SecretKey(bytes)
    }
    
    /// 安全清零 (drop 时调用)
    pub fn zeroize(&mut self) {
        self.0.fill(0);
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl KeyImage {
    /// 创建零 Key Image
    pub fn zero() -> Self {
        KeyImage([0u8; 32])
    }
    
    /// 从字节数组创建
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        KeyImage(bytes)
    }
    
    /// 转换为字节数组
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl Commitment {
    /// 创建零承诺
    pub fn zero() -> Self {
        Commitment([0u8; 32])
    }
    
    /// 从字节数组创建
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Commitment(bytes)
    }
    
    /// 转换为字节数组
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_key() {
        let pk = PublicKey::zero();
        assert_eq!(pk.to_bytes(), [0u8; 32]);
    }

    #[test]
    fn test_secret_key_security() {
        let mut sk = SecretKey::from_bytes([1u8; 32]);
        sk.zeroize();
        // 验证已清零
        assert_eq!(sk.0, [0u8; 32]);
    }

    #[test]
    fn test_key_image() {
        let ki = KeyImage::zero();
        assert_eq!(ki.to_bytes(), [0u8; 32]);
    }

    #[test]
    fn test_commitment() {
        let c = Commitment::zero();
        assert_eq!(c.to_bytes(), [0u8; 32]);
    }
}
