use sha2::{Digest, Sha256};

use crate::proof::TraceDigest;

/// 执行轨迹，用于生成承诺/证明
#[derive(Clone, Debug)]
pub struct ExecutionTrace {
    pub states: Vec<u128>,
}

impl ExecutionTrace {
    pub fn new(states: Vec<u128>) -> Self {
        Self { states }
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn commitment(&self) -> TraceDigest {
        let mut hasher = Sha256::new();
        for value in &self.states {
            hasher.update(value.to_le_bytes());
        }
        hasher.finalize().into()
    }
}

/// L2 程序需要提供生成轨迹与公共输出的能力
pub trait TraceProgram {
    fn id(&self) -> &'static str;
    fn generate_trace(&self, witness: &[u64]) -> ExecutionTrace;
    fn public_outputs(&self, witness: &[u64]) -> Vec<u64>;
}

/// 将 witness 转换为连续字节序列 (little-endian)
fn witness_to_bytes(witness: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(witness.len() * 8);
    for value in witness {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// SHA256 轨迹程序：展示 STARK-style hash trace 的 PoC
pub struct Sha256Program {
    chunk_words: usize,
}

impl Sha256Program {
    pub fn new(chunk_words: usize) -> Self {
        assert!(chunk_words > 0, "chunk_words must be positive");
        Self { chunk_words }
    }

    fn chunk_bytes(&self) -> usize {
        self.chunk_words * 8
    }

    fn digest_states(&self, message: &[u8]) -> Vec<u128> {
        let chunk_len = self.chunk_bytes().max(8);
        let mut hasher = Sha256::new();
        let mut states = Vec::new();

        if message.is_empty() {
            hasher.update([0u8; 1]);
            let digest = hasher.finalize();
            states.extend_from_slice(&split_digest(&digest));
            return states;
        }

        for chunk in message.chunks(chunk_len) {
            hasher.update(chunk);
            let digest = hasher.clone().finalize();
            states.extend_from_slice(&split_digest(&digest));
        }

        states
    }
}

impl Default for Sha256Program {
    fn default() -> Self {
        Self { chunk_words: 4 }
    }
}

impl TraceProgram for Sha256Program {
    fn id(&self) -> &'static str {
        "sha256.v0"
    }

    fn generate_trace(&self, witness: &[u64]) -> ExecutionTrace {
        let bytes = witness_to_bytes(witness);
        ExecutionTrace::new(self.digest_states(&bytes))
    }

    fn public_outputs(&self, witness: &[u64]) -> Vec<u64> {
        let bytes = witness_to_bytes(witness);
        let digest = Sha256::digest(&bytes);
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&digest[..8]);
        vec![u64::from_be_bytes(arr)]
    }
}

fn split_digest(digest: &[u8]) -> [u128; 2] {
    let mut left = [0u8; 16];
    let mut right = [0u8; 16];
    left.copy_from_slice(&digest[..16]);
    right.copy_from_slice(&digest[16..]);
    [u128::from_be_bytes(left), u128::from_be_bytes(right)]
}
/// Fibonacci 示例程序 (Phase 8 PoC)
pub struct FibonacciProgram {
    rounds: u32,
}

impl FibonacciProgram {
    pub fn new(rounds: u32) -> Self {
        assert!(rounds > 0, "rounds must be positive");
        Self { rounds }
    }

    fn compute_sequence(&self, witness: &[u64]) -> Vec<u128> {
        let a0 = *witness.get(0).unwrap_or(&0) as u128;
        let a1 = *witness.get(1).unwrap_or(&1) as u128;
        let mut seq = Vec::with_capacity(self.rounds as usize + 2);
        seq.push(a0);
        seq.push(a1);
        for _ in 0..self.rounds {
            let next = seq[seq.len() - 1] + seq[seq.len() - 2];
            seq.push(next);
        }
        seq
    }
}

impl TraceProgram for FibonacciProgram {
    fn id(&self) -> &'static str {
        "fib.v0"
    }

    fn generate_trace(&self, witness: &[u64]) -> ExecutionTrace {
        ExecutionTrace::new(self.compute_sequence(witness))
    }

    fn public_outputs(&self, witness: &[u64]) -> Vec<u64> {
        let trace = self.compute_sequence(witness);
        let idx = self.rounds as usize;
        let value = trace.get(idx).copied().unwrap_or_default() as u64;
        vec![value]
    }
}
