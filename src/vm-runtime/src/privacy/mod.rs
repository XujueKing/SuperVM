// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

// SuperVM 2.0 - Privacy Layer
// 架构师: KING XU (CHINA)
// Phase 2: Privacy Layer Implementation
// 
// 本模块实现隐私交易功能,包括:
// - Ring Signatures (环签名)
// - Stealth Addresses (隐形地址)
// - Pedersen Commitments (承诺)
// - Range Proofs (范围证明)
// - Zero-Knowledge Proofs (零知识证明)
// - Mixing Pool (混币池)

pub mod types;
pub mod ring_signature;
pub mod stealth_address;
pub mod commitment;
pub mod range_proof;
pub mod zksnark;  // Phase 2.2.4
#[cfg(feature = "groth16-verifier")]
pub mod groth16_verifier; // Optional: Groth16 backend adapter
// pub mod ringct;   // Phase 2.2.5
// pub mod mixing;   // Phase 2.2.6

pub use types::*;
pub use zksnark::{ZkVerifier, NoopVerifier, ZkError, ZkCircuitId};
#[cfg(feature = "groth16-verifier")]
pub use groth16_verifier::Groth16Verifier;
// pub use ring_signature::*;
// pub use stealth_address::*;
// pub use commitment::*;
// pub use range_proof::*;

/// Privacy Layer 版本
pub const PRIVACY_VERSION: &str = "2.0.0-alpha";

/// 默认 Ring Size (环大小)
pub const DEFAULT_RING_SIZE: usize = 11;

/// 最小 Ring Size
pub const MIN_RING_SIZE: usize = 3;

/// 最大 Ring Size
pub const MAX_RING_SIZE: usize = 64;

/// Range Proof 位数 (支持 0 到 2^64-1)
pub const RANGE_PROOF_BITS: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(MIN_RING_SIZE <= DEFAULT_RING_SIZE);
        assert!(DEFAULT_RING_SIZE <= MAX_RING_SIZE);
        assert_eq!(RANGE_PROOF_BITS, 64);
    }
}
