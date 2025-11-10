# SuperVM 三通道路由快速参考

**版本**: Phase 5 (v0.1.0)  
**更新**: 2025-11-10

---

## 📖 快速入门

### 三通道概述

SuperVM 提供三条执行路径，根据对象所有权自动路由：

| 通道 | 适用对象 | 性能 | 特点 |
|------|---------|------|------|
| **FastPath** | Owned (独占) | **29.4M TPS**, 35ns | 零锁、零分配、零冲突 |
| **Consensus** | Shared (共享) | 377K TPS, ~2.7μs | MVCC、并发控制、原子性 |
| **Privacy** | 需隐私保护 | < 50ms (含 ZK) | ZK 证明验证、匿名性 |

---

## 🚀 API 使用

### 1. 基础路由执行

```rust
use vm_runtime::{SuperVM, OwnershipManager, Transaction, Privacy};

// 创建所有权管理器
let ownership = OwnershipManager::new();

// 创建 SuperVM 实例
let vm = SuperVM::new(&ownership);

// 定义事务
let tx = Transaction {
    from: sender_address,
    objects: vec![object_id],
    privacy: Privacy::Public,  // 或 Privacy::Private
};

// 自动路由执行
let result = vm.execute_transaction_routed(tx_id, &tx, || {
    // 业务逻辑
    Ok(42)
});
```

### 2. FastPath 直接执行

适用于：明确知道对象为独占类型，追求极致性能

```rust
use vm_runtime::SuperVM;

let vm = SuperVM::new(&ownership);

// 直接走 FastPath，跳过路由判断
let result = vm.execute_fast_path(tx_id, &tx, || {
    // 轻量级业务逻辑
    let mut acc = 0u64;
    for i in 0..100 {
        acc += i;
    }
    Ok(acc as i32)
});
```

**性能**: 29.4M TPS, 35ns 延迟

### 3. Consensus 路径（MVCC）

适用于：共享对象，需要并发控制

```rust
use vm_runtime::{SuperVM, MvccScheduler};

let scheduler = MvccScheduler::new(/* config */);
let vm = SuperVM::new(&ownership).with_scheduler(&scheduler);

// 自动识别 Shared 对象并走 Consensus 路径
let result = vm.execute_transaction_routed(tx_id, &tx, || {
    // 共享对象操作
    Ok(result)
});
```

**性能**: 377K TPS (纯 Consensus)

### 4. Privacy 路径（ZK 验证）

适用于：需要隐私保护的事务

```rust
use vm_runtime::{Transaction, Privacy};

let tx = Transaction {
    from: sender,
    objects: vec![obj_id],
    privacy: Privacy::Private,  // 标记为隐私事务
};

// SuperVM 会自动走 Privacy 路径并验证 ZK 证明
let result = vm.execute_transaction_routed(tx_id, &tx, || {
    // 隐私业务逻辑
    Ok(result)
});
```

**性能**: < 50ms (含真实 ZK 验证)

---

## 🎛️ 路由配置

### 对象所有权注册

```rust
use vm_runtime::{OwnershipManager, OwnershipType, ObjectMetadata};

let mut ownership = OwnershipManager::new();

// 注册独占对象 (FastPath)
let owned_obj = ObjectMetadata {
    id: object_id,
    version: 0,
    ownership: OwnershipType::Owned(owner_address),
    object_type: "NFT".to_string(),
    created_at: timestamp,
    updated_at: timestamp,
    size: 1024,
    is_deleted: false,
};
ownership.register_object(owned_obj)?;

// 注册共享对象 (Consensus)
let shared_obj = ObjectMetadata {
    id: pool_id,
    ownership: OwnershipType::Shared,
    object_type: "LiquidityPool".to_string(),
    // ... 其他字段
};
ownership.register_object(shared_obj)?;

// 注册不可变对象 (FastPath, 只读)
let immutable_obj = ObjectMetadata {
    id: config_id,
    ownership: OwnershipType::Immutable,
    object_type: "Config".to_string(),
    // ... 其他字段
};
ownership.register_object(immutable_obj)?;
```

### 环境变量配置

```bash
# 自适应路由器配置
export SUPERVM_ADAPTIVE_ENABLED=true
export SUPERVM_ADAPTIVE_TARGET_FAST_RATIO=0.8
export SUPERVM_ADAPTIVE_WINDOW_SIZE=10000

# ZK 验证器模式
export ZK_VERIFIER_MODE=real        # real | mock
export ZK_MOCK_ALWAYS_SUCCEED=true  # 仅 mock 模式
export ZK_MOCK_DELAY_US=5000        # mock 延迟（微秒）

# 性能基准配置
export MIXED_ITERS=500000
export OWNED_RATIO=0.8              # FastPath 比例
export PRIVACY_RATIO=0.0            # Privacy 比例
```

---

## 📊 监控与观测

### Prometheus 指标

```promql
# 三通道吞吐量
rate(vm_routing_fast_total[1m])      # FastPath TPS
rate(vm_routing_consensus_total[1m]) # Consensus TPS
rate(vm_routing_privacy_total[1m])   # Privacy TPS

# FastPath 性能
vm_fast_path_avg_latency_ns          # 平均延迟
vm_fast_path_success_total           # 成功总数

# 回退统计
vm_fast_fallback_total               # Fast→Consensus 回退次数
vm_fast_fallback_ratio               # 回退率

# ZK 验证
vm_zk_verify_total                   # ZK 验证总数
vm_zk_verify_failure_rate            # ZK 验证失败率
vm_zk_verify_latency_p99_ms          # ZK 验证 P99 延迟
```

### HTTP Metrics 端点

```bash
# 启动带 metrics 服务的基准测试
cargo run --release --example mixed_path_bench -- --serve-metrics:8082

# 查询指标
curl http://localhost:8082/metrics
```

### Grafana Dashboard

导入预配置 Dashboard：

```bash
# 导入 JSON
grafana-cli dashboard import grafana-phase5-dashboard.json
```

或手动访问：[http://localhost:3000/dashboards](http://localhost:3000/dashboards)

---

## 🏆 最佳实践

### 1. 对象类型选择

| 场景 | 推荐类型 | 通道 | 理由 |
|------|---------|------|------|
| NFT 转账 | `Owned` | FastPath | 独占所有权，无并发冲突 |
| DEX 流动性池 | `Shared` | Consensus | 多用户并发访问 |
| 系统配置 | `Immutable` | FastPath | 只读访问，零冲突 |
| 隐私转账 | `Owned` + `Privacy::Private` | Privacy | 需要 ZK 证明 |

### 2. 性能优化建议

#### FastPath 优化

```rust
// ✅ 推荐: 闭包内逻辑简洁
vm.execute_fast_path(tx_id, &tx, || {
    let result = value1 + value2;
    Ok(result)
});

// ❌ 避免: 闭包内复杂计算
vm.execute_fast_path(tx_id, &tx, || {
    // 避免在闭包内进行重度计算或 I/O
    let data = expensive_computation();  // 将此移到闭包外
    Ok(data)
});
```

#### 混合负载配置

```rust
// 推荐比例: 80% FastPath + 20% Consensus
// 实测吞吐: 1.20M TPS
let owned_ratio = 0.8;

// 如需极致吞吐: 100% FastPath
// 实测吞吐: 29.4M TPS (仅适用于纯独占场景)
```

### 3. 错误处理

```rust
use vm_runtime::VmError;

match vm.execute_transaction_routed(tx_id, &tx, || Ok(42)) {
    Ok(result) => println!("成功: {}", result),
    Err(VmError::AccessDenied(_)) => {
        // 处理权限错误
    }
    Err(VmError::ObjectNotFound(_)) => {
        // 处理对象不存在
    }
    Err(VmError::ConflictDetected(_)) => {
        // 处理 MVCC 冲突（可重试）
    }
    Err(e) => eprintln!("其他错误: {:?}", e),
}
```

### 4. Privacy 路径使用

```rust
// 生成 ZK 证明（链下）
let (proof, public_inputs) = generate_zk_proof(secret_data)?;

// 构造隐私事务
let tx = Transaction {
    from: sender,
    objects: vec![obj_id],
    privacy: Privacy::Private,
};

// 执行（SuperVM 会自动验证 ZK 证明）
let result = vm.execute_transaction_routed(tx_id, &tx, || {
    // 隐私业务逻辑
    Ok(transfer_amount)
});
```

---

## 🧪 测试与基准

### 运行基准测试

```bash
cd src/vm-runtime

# FastPath 纯性能测试
export FAST_PATH_ITERS=2000000
cargo run --release --example fast_path_bench

# 混合负载测试 (80% Fast + 20% Consensus)
export MIXED_ITERS=500000
export OWNED_RATIO=0.8
cargo run --release --example mixed_path_bench

# 带 Prometheus 监控
cargo run --release --example mixed_path_bench -- --serve-metrics:8082
```

### 性能基准参考

| 配置 | FastPath % | Consensus % | 总 TPS | Fast 延迟 |
|------|-----------|------------|--------|----------|
| 纯 Fast | 100% | 0% | 29.4M | 35ns |
| 80/20 | 80% | 20% | 1.20M | 34ns |
| 50/50 | 50% | 50% | 645K | 34ns |
| 纯 Consensus | 0% | 100% | 377K | ~2.7μs |

---

## 🔧 故障排查

### 常见问题

#### 1. FastPath 成功率低

**症状**: `vm_fast_path_success_total / vm_fast_path_attempts_total < 0.95`

**排查**:
```bash
# 检查对象所有权配置
ownership.get_object_metadata(&obj_id)?
# 确认 OwnershipType::Owned(correct_owner)

# 检查权限匹配
assert_eq!(tx.from, owned_object.owner);
```

#### 2. 高回退率

**症状**: `vm_fast_fallback_ratio > 0.1`

**原因**: FastPath 条件不满足，频繁回退到 Consensus

**解决**:
```rust
// 检查自适应路由器配置
export SUPERVM_ADAPTIVE_TARGET_FAST_RATIO=0.8  # 降低目标比例

// 或检查对象注册是否正确
```

#### 3. ZK 验证失败

**症状**: `vm_zk_verify_failure_rate > 0.05`

**排查**:
```bash
# 检查 ZK 验证器模式
echo $ZK_VERIFIER_MODE

# 切换到 mock 模式测试
export ZK_VERIFIER_MODE=mock
export ZK_MOCK_ALWAYS_SUCCEED=true
```

---

## 📚 相关资源

- [Phase 5 性能报告](../PHASE5-METRICS-2025-11-10.md)
- [ZK 集成指南](./ZK-INTEGRATION.md)
- [对象所有权模型](../src/vm-runtime/src/ownership.rs)
- [SuperVM 源码](../src/vm-runtime/src/supervm.rs)
- [Grafana Dashboard](../grafana-phase5-dashboard.json)

---

## 🎯 总结

**三通道选择决策树**:

```
事务需要隐私？
├─ 是 → Privacy 路径 (< 50ms)
└─ 否 → 对象类型？
    ├─ Owned / Immutable → FastPath (35ns, 29.4M TPS)
    └─ Shared → Consensus (2.7μs, 377K TPS)
```

**关键要点**:
1. ✅ 优先使用 `Owned` 对象 → 获得极致性能
2. ✅ 共享对象使用 `Shared` → 自动并发控制
3. ✅ 隐私场景标记 `Privacy::Private` → ZK 保护
4. ✅ 监控 Prometheus 指标 → 及时发现问题
5. ✅ 根据业务场景调整 `owned_ratio` → 平衡吞吐与功能

**下一步**: [Phase 6: 四层神经网络](../ROADMAP.md#phase-6)
