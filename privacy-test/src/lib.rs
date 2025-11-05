// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! # Privacy Cryptography 实践教程
//! 
//! 基于 curve25519-dalek 学习 Monero 使用的密码学原语
//! 
//! ## 模块结构
//! - `ristretto_basics`: RistrettoPoint 基础操作
//! - `hash_to_point`: Hash-to-Point 实现 (用于 Key Image)
//! - `pedersen_commitment`: Pedersen 承诺 (隐藏金额)
//! - `simple_ring_signature`: 简单 2-of-3 环签名

pub mod ristretto_basics;
pub mod hash_to_point;
pub mod pedersen_commitment;
pub mod simple_ring_signature;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
