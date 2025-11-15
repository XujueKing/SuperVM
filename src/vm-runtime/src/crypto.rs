// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 密码学功能模块
//!
//! 提供哈希、签名验证等密码学原语的实现

use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature as Ed25519Signature, VerifyingKey as Ed25519VerifyingKey};
use k256::ecdsa::{
    signature::Verifier, Signature as K256Signature, VerifyingKey as K256VerifyingKey,
};
use sha2::{Digest as Sha2Digest, Sha256};
use sha3::Keccak256;

/// 计算 SHA-256 哈希
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// 计算 Keccak-256 哈希 (以太坊使用)
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// 验证 secp256k1 签名 (以太坊/比特币)
///
/// # 参数
/// - `message`: 消息哈希 (32 字节)
/// - `signature`: 签名数据 (64 字节: r + s)
/// - `pubkey`: 公钥 (33 或 65 字节,压缩或未压缩格式)
pub fn verify_secp256k1(message: &[u8], signature: &[u8], pubkey: &[u8]) -> Result<bool> {
    if message.len() != 32 {
        return Err(anyhow!("Message must be 32 bytes"));
    }
    if signature.len() != 64 {
        return Err(anyhow!("Signature must be 64 bytes"));
    }

    // 解析签名
    let sig =
        K256Signature::from_slice(signature).map_err(|e| anyhow!("Invalid signature: {}", e))?;

    // 解析公钥
    let vk = K256VerifyingKey::from_sec1_bytes(pubkey)
        .map_err(|e| anyhow!("Invalid public key: {}", e))?;

    // 验证签名
    Ok(vk.verify(message, &sig).is_ok())
}

/// 验证 Ed25519 签名 (Solana)
///
/// # 参数
/// - `message`: 原始消息
/// - `signature`: 签名数据 (64 字节)
/// - `pubkey`: 公钥 (32 字节)
pub fn verify_ed25519(message: &[u8], signature: &[u8], pubkey: &[u8]) -> Result<bool> {
    if signature.len() != 64 {
        return Err(anyhow!("Signature must be 64 bytes"));
    }
    if pubkey.len() != 32 {
        return Err(anyhow!("Public key must be 32 bytes"));
    }

    // 解析签名
    let sig =
        Ed25519Signature::from_slice(signature).map_err(|e| anyhow!("Invalid signature: {}", e))?;

    // 解析公钥
    let vk = Ed25519VerifyingKey::from_bytes(pubkey.try_into().unwrap())
        .map_err(|e| anyhow!("Invalid public key: {}", e))?;

    // 验证签名
    Ok(vk.verify(message, &sig).is_ok())
}

/// 从 secp256k1 签名恢复公钥 (以太坊地址恢复)
///
/// # 参数
/// - `message`: 消息哈希 (32 字节)
/// - `signature`: 签名数据 (65 字节: r + s + v)
///
/// # 返回
/// 恢复的公钥 (65 字节,未压缩格式)
pub fn recover_secp256k1_pubkey(message: &[u8], signature: &[u8]) -> Result<Vec<u8>> {
    if message.len() != 32 {
        return Err(anyhow!("Message must be 32 bytes"));
    }
    if signature.len() != 65 {
        return Err(anyhow!("Signature must be 65 bytes (with recovery id)"));
    }

    let recovery_id = signature[64];
    let sig_bytes = &signature[..64];

    // 解析签名
    let sig =
        K256Signature::from_slice(sig_bytes).map_err(|e| anyhow!("Invalid signature: {}", e))?;

    // 解析 recovery id
    let rec_id = k256::ecdsa::RecoveryId::try_from(recovery_id)
        .map_err(|e| anyhow!("Invalid recovery id: {}", e))?;

    // 恢复公钥
    let vk = K256VerifyingKey::recover_from_prehash(message, &sig, rec_id)
        .map_err(|e| anyhow!("Failed to recover public key: {}", e))?;

    // 返回未压缩格式 (65 字节)
    let pubkey_bytes = vk.to_encoded_point(false);
    Ok(pubkey_bytes.as_bytes().to_vec())
}

/// 从 secp256k1 公钥推导以太坊地址
///
/// 支持压缩(33字节)或未压缩(65字节) SEC1 格式公钥。
/// 算法: keccak256(uncompressed_pubkey[1..]) 的后 20 字节。
pub fn derive_eth_address(pubkey: &[u8]) -> Result<[u8; 20]> {
    // 解析为验证公钥
    let vk = K256VerifyingKey::from_sec1_bytes(pubkey)
        .map_err(|e| anyhow!("Invalid public key: {}", e))?;
    // 导出未压缩点 (0x04 || X(32) || Y(32))
    let uncompressed = vk.to_encoded_point(false);
    let bytes = uncompressed.as_bytes();
    if bytes.len() != 65 || bytes[0] != 0x04 {
        return Err(anyhow!("Unexpected uncompressed format"));
    }
    // 对 64 字节 (X||Y) 做 keccak
    let hash = keccak256(&bytes[1..]);
    // 最后 20 字节
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;

    #[test]
    fn test_sha256() {
        let data = b"hello world";
        let hash = sha256(data);

        // 已知的 SHA-256("hello world")
        let expected =
            hex::decode("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
                .unwrap();
        assert_eq!(hash.as_slice(), expected.as_slice());
    }

    #[test]
    fn test_keccak256() {
        let data = b"hello world";
        let hash = keccak256(data);

        // 已知的 Keccak-256("hello world")
        let expected =
            hex::decode("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad")
                .unwrap();
        assert_eq!(hash.as_slice(), expected.as_slice());
    }

    #[test]
    fn test_secp256k1_verify() {
        // 测试向量来自标准测试套件
        // 这里使用简化的测试,实际应该使用真实的签名数据
        let message = [0u8; 32];
        let signature = [0u8; 64];
        let pubkey = [0u8; 33]; // 压缩格式

        // 预期失败,因为这是无效的签名
        let result = verify_secp256k1(&message, &signature, &pubkey);
        assert!(result.is_err() || !result.unwrap());
    }

    #[test]
    fn test_ed25519_verify() {
        let message = b"test message";
        let signature = [0u8; 64];
        let pubkey = [0u8; 32];

        // 预期失败,因为这是无效的签名
        let result = verify_ed25519(message, &signature, &pubkey);
        assert!(result.is_err() || !result.unwrap());
    }

    #[test]
    fn test_derive_eth_address() {
        // 使用固定私钥生成公钥,测试地址派生逻辑
        let sk_bytes = [1u8; 32];
        let sk = SigningKey::from_bytes((&sk_bytes).into()).unwrap();
        let vk = sk.verifying_key();
        let pub_uncompressed = vk.to_encoded_point(false);
        let addr = derive_eth_address(pub_uncompressed.as_bytes()).unwrap();

        // 与手动计算对比
        let hash = keccak256(&pub_uncompressed.as_bytes()[1..]);
        let mut expect = [0u8; 20];
        expect.copy_from_slice(&hash[12..]);
        assert_eq!(addr, expect);
    }
}
