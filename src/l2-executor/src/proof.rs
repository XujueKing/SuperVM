use core::fmt;

/// 32-byte承诺，用于表示执行轨迹或聚合根
pub type TraceDigest = [u8; 32];

/// 单个 zkVM 证明 (占位实现)
#[derive(Clone, PartialEq, Eq)]
pub struct Proof {
    pub program_id: String,
    pub public_outputs: Vec<u64>,
    pub steps: usize,
    pub trace_commitment: TraceDigest,
}

impl fmt::Debug for Proof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Proof{{program_id: {}, steps: {}, trace_commitment: {} bytes}}",
            self.program_id,
            self.steps,
            self.trace_commitment.len()
        )
    }
}

impl Proof {
    pub fn new(program_id: impl Into<String>, public_outputs: Vec<u64>, steps: usize, trace_commitment: TraceDigest) -> Self {
        Self {
            program_id: program_id.into(),
            public_outputs,
            steps,
            trace_commitment,
        }
    }
}

/// 多证明聚合结果
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregatedProof {
    pub root: TraceDigest,
    pub proof_count: usize,
}

impl AggregatedProof {
    pub fn empty() -> Self {
        Self { root: [0u8; 32], proof_count: 0 }
    }
}
