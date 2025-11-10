# ZK 验证器集成指南

## 概述

SuperVM 支持零知识证明（ZK）验证集成，提供灵活的后端切换、完整的性能指标监控和便捷的测试支持。

## 架构设计

### ZkVerifier Trait

所有 ZK 验证器都实现 `ZkVerifier` trait：

```rust
pub trait ZkVerifier: Send + Sync {
    /// 验证证明
    fn verify(&self, proof_bytes: &[u8], public_inputs_bytes: &[u8]) 
        -> Result<bool, ZkError>;
    
    /// 获取验证器类型描述
    fn verifier_type(&self) -> &str;
    
    /// 获取后端枚举
    fn backend(&self) -> ZkBackend;
}
```

### 支持的后端类型

```rust
pub enum ZkBackend {
    Groth16Bls12_381,  // Groth16 + BLS12-381 曲线
    Plonk,             // Plonk (预留)
    Mock,              // Mock 验证器（测试用）
}
```

## Feature Gates

### 可用 Features

| Feature | 说明 | 依赖 |
|---------|------|------|
| `groth16-verifier` | 启用 Groth16 真实验证器 | `ark-groth16`, `ark-bls12-381` |
| 无 feature | 仅提供 trait 定义和 Mock 实现 | 无 |

### 编译配置

```toml
# Cargo.toml
[dependencies]
vm-runtime = { path = "...", features = ["groth16-verifier"] }
```

```bash
# 命令行
cargo build --features groth16-verifier
cargo test --features groth16-verifier
```

## 验证器实现

### 1. Groth16Verifier

基于 BLS12-381 曲线的 Groth16 验证器。

```rust
use vm_runtime::zk_verifier::{Groth16Verifier, ZkVerifier};

// 从验证密钥创建
let verifier = Groth16Verifier::new(&verifying_key);

// 测试模式（生成临时 setup）
let verifier = Groth16Verifier::new_for_testing()?;

// 验证证明
let result = verifier.verify(&proof_bytes, &public_input_bytes)?;
```

### 2. MockVerifier

用于测试和 CI 的可配置验证器。

```rust
use vm_runtime::zk_verifier::MockVerifier;

// 总是成功
let verifier = MockVerifier::new_always_succeed();

// 总是失败
let verifier = MockVerifier::new_always_fail();

// 带延迟（模拟真实验证延迟）
let verifier = MockVerifier::new_with_delay(
    true,      // succeed
    10_000     // 10ms delay
);

// 验证调用计数（用于测试断言）
assert_eq!(verifier.call_count(), 0);
verifier.verify(&[], &[])?;
assert_eq!(verifier.call_count(), 1);
```

## 环境变量配置

支持通过环境变量动态切换验证器，便于 CI/本地测试。

### 环境变量

| 变量 | 值 | 默认 | 说明 |
|------|----|----|------|
| `ZK_VERIFIER_MODE` | `real` \| `mock` | `real` | 验证器模式 |
| `ZK_MOCK_ALWAYS_SUCCEED` | `true` \| `false` | `true` | Mock 验证结果 |
| `ZK_MOCK_DELAY_US` | 数字（微秒） | `0` | Mock 延迟 |

### 使用示例

```rust
use vm_runtime::zk_verifier::create_verifier_from_env;

// 根据环境变量创建验证器
let verifier = create_verifier_from_env();
```

```bash
# CI 测试 - 使用 mock
export ZK_VERIFIER_MODE=mock
export ZK_MOCK_ALWAYS_SUCCEED=true
cargo test

# 性能测试 - 模拟延迟
export ZK_VERIFIER_MODE=mock
export ZK_MOCK_DELAY_US=5000  # 5ms
cargo run --example zk_perf_test

# 生产环境 - 真实验证
export ZK_VERIFIER_MODE=real
cargo run --features groth16-verifier
```

## Prometheus 指标

### 指标列表

#### 验证次数

```promql
# 总验证次数
vm_zk_verify_total

# 失败次数
vm_zk_verify_failures_total
```

#### 失败率

```promql
# 失败率（0.0-1.0）
vm_zk_verify_failure_rate

# PromQL 计算失败率
rate(vm_zk_verify_failures_total[5m]) / rate(vm_zk_verify_total[5m])
```

#### 延迟指标

```promql
# 平均延迟
vm_zk_verify_latency_avg_ms

# P50/P90/P99 延迟
vm_zk_verify_latency_p50_ms
vm_zk_verify_latency_p90_ms
vm_zk_verify_latency_p99_ms
```

#### 后端类型分布

```promql
# 按后端类型分组
vm_zk_backend_count{backend="groth16-bls12-381"}
vm_zk_backend_count{backend="plonk"}
vm_zk_backend_count{backend="mock"}

# 查询各后端占比
sum by (backend) (vm_zk_backend_count)
```

### 指标收集

```rust
use vm_runtime::metrics::MetricsCollector;
use vm_runtime::zk_verifier::ZkBackend;
use std::time::{Duration, Instant};

let metrics = MetricsCollector::new();

// 记录 ZK 验证
let start = Instant::now();
let success = verifier.verify(&proof, &inputs)?;
let duration = start.elapsed();

metrics.record_zk_verify(
    verifier.backend(),  // ZkBackend 枚举
    success,             // 是否成功
    duration             // 耗时
);

// 导出 Prometheus 格式
let prom_output = metrics.export_prometheus();
```

## 典型用例

### 用例 1: 单元测试使用 Mock

```rust
#[cfg(test)]
mod tests {
    use vm_runtime::zk_verifier::MockVerifier;
    
    #[test]
    fn test_transaction_with_zk_proof() {
        // 使用 mock 避免真实 ZK setup 开销
        let verifier = MockVerifier::new_always_succeed();
        
        let tx = create_test_transaction();
        let result = process_tx_with_zk(&tx, &verifier);
        
        assert!(result.is_ok());
        assert_eq!(verifier.call_count(), 1);
    }
}
```

### 用例 2: CI 环境使用环境变量

```yaml
# .github/workflows/test.yml
jobs:
  test:
    runs-on: ubuntu-latest
    env:
      ZK_VERIFIER_MODE: mock
      ZK_MOCK_ALWAYS_SUCCEED: true
    steps:
      - run: cargo test --all-features
```

### 用例 3: 多后端切换

```rust
use vm_runtime::zk_verifier::{Groth16Verifier, MockVerifier, ZkVerifier};
use std::sync::Arc;

fn get_verifier(use_production: bool) -> Arc<dyn ZkVerifier> {
    if use_production {
        Arc::new(Groth16Verifier::new(&production_vk))
    } else {
        Arc::new(MockVerifier::new_with_delay(true, 1000))
    }
}

// 运行时切换
let verifier = get_verifier(cfg!(feature = "production"));
```

### 用例 4: Prometheus 监控集成

```rust
use vm_runtime::metrics::MetricsCollector;
use tiny_http::{Server, Response};
use std::sync::Arc;

let metrics = Arc::new(MetricsCollector::new());
let server = Server::http("0.0.0.0:9090").unwrap();

for request in server.incoming_requests() {
    if request.url() == "/metrics" {
        let prom = metrics.export_prometheus();
        let response = Response::from_string(prom);
        request.respond(response).ok();
    }
}
```

### 用例 5: 延迟模拟性能测试

```rust
use vm_runtime::zk_verifier::MockVerifier;
use std::time::Instant;

// 模拟真实 ZK 验证延迟（~10ms）
let verifier = MockVerifier::new_with_delay(true, 10_000);

let start = Instant::now();
for _ in 0..100 {
    verifier.verify(&[], &[]).unwrap();
}
let elapsed = start.elapsed();

println!("100 次验证耗时: {:?}", elapsed);
println!("平均延迟: {:?}", elapsed / 100);
```

## 最佳实践

### 1. Feature 分离

```rust
// 生产代码
#[cfg(feature = "groth16-verifier")]
use vm_runtime::zk_verifier::Groth16Verifier;

// 测试代码
#[cfg(test)]
use vm_runtime::zk_verifier::MockVerifier;
```

### 2. 环境感知配置

```rust
use vm_runtime::zk_verifier::create_verifier_from_env;

let verifier = if cfg!(test) {
    std::env::set_var("ZK_VERIFIER_MODE", "mock");
    create_verifier_from_env()
} else {
    create_verifier_from_env()
};
```

### 3. 指标告警

```yaml
# prometheus-alerts.yml
groups:
  - name: zk_verification
    rules:
      - alert: HighZKFailureRate
        expr: vm_zk_verify_failure_rate > 0.05
        for: 5m
        annotations:
          summary: "ZK 验证失败率过高 ({{ $value }})"
      
      - alert: ZKLatencyHigh
        expr: vm_zk_verify_latency_p99_ms > 100
        for: 10m
        annotations:
          summary: "ZK 验证 P99 延迟超过 100ms"
```

### 4. 日志集成

```rust
use tracing::{info, warn};

let start = Instant::now();
let result = verifier.verify(proof, inputs);
let elapsed = start.elapsed();

match result {
    Ok(true) => info!(
        backend = ?verifier.backend(),
        latency_ms = elapsed.as_millis(),
        "ZK verification succeeded"
    ),
    Ok(false) => warn!("ZK verification failed (invalid proof)"),
    Err(e) => warn!(error = ?e, "ZK verification error"),
}
```

## 性能优化

### 1. 验证器复用

```rust
use std::sync::Arc;

// 验证器是 Send + Sync，可安全共享
let verifier = Arc::new(Groth16Verifier::new(&vk));

// 多线程共享
let verifier_clone = Arc::clone(&verifier);
std::thread::spawn(move || {
    verifier_clone.verify(&proof, &inputs).unwrap();
});
```

### 2. 批量验证（未来支持）

```rust
// TODO: 批量验证接口
trait BatchZkVerifier: ZkVerifier {
    fn verify_batch(&self, proofs: &[ProofBatch]) -> Result<Vec<bool>, ZkError>;
}
```

## 故障排查

### 常见问题

#### 1. Feature 未启用

```
error: could not find `zk_verifier` in the crate root
```

**解决**：启用 `groth16-verifier` feature

```bash
cargo build --features groth16-verifier
```

#### 2. 验证失败

```
ZkError::VerificationError("Pairing check failed")
```

**排查**：
- 检查 proof 和 public inputs 是否匹配
- 确认 verifying key 来自同一 setup
- 验证序列化/反序列化正确性

#### 3. Mock 未按预期工作

**排查**：
```rust
// 打印环境变量
println!("MODE: {}", std::env::var("ZK_VERIFIER_MODE").unwrap_or_default());

// 检查 verifier 类型
println!("Type: {}", verifier.verifier_type());
println!("Backend: {:?}", verifier.backend());
```

## 扩展开发

### 添加新的 ZK 后端

1. **扩展枚举**

```rust
pub enum ZkBackend {
    Groth16Bls12_381,
    Plonk,
    PlonkBn254,  // 新后端
    Mock,
}
```

2. **实现 Verifier**

```rust
pub struct PlonkBn254Verifier {
    vk: VerifyingKey<Bn254>,
}

impl ZkVerifier for PlonkBn254Verifier {
    fn verify(&self, proof: &[u8], inputs: &[u8]) -> Result<bool, ZkError> {
        // 实现验证逻辑
    }
    
    fn verifier_type(&self) -> &str {
        "Plonk-BN254"
    }
    
    fn backend(&self) -> ZkBackend {
        ZkBackend::PlonkBn254
    }
}
```

3. **更新指标收集**

```rust
// metrics.rs
match backend {
    ZkBackend::PlonkBn254 => {
        self.zk_backend_plonk_bn254_count.fetch_add(1, Ordering::Relaxed);
    }
    // ...
}
```

## 相关资源

- [Groth16 论文](https://eprint.iacr.org/2016/260.pdf)
- [arkworks-rs](https://github.com/arkworks-rs/groth16)
- [Prometheus 最佳实践](https://prometheus.io/docs/practices/)
- [SuperVM 文档目录](./INDEX.md)

## 更新日志

- **2025-11-10**: 初版文档，包含 trait 扩展、Mock verifier、环境变量配置、Prometheus 指标
