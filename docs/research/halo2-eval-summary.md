# Halo2 评估总结

开发者/作者：King Xujue

完成时间: 2025-02-28  
项目位置: `halo2-eval/`  
任务状态: ✅ 完成

---

## 实现内容

### 1) 项目结构

```

halo2-eval/
├── Cargo.toml           # halo2_proofs 0.3 + halo2curves 0.6
├── src/
│   ├── lib.rs          # MulCircuit (PLONK-style Gate)
│   └── bin/
│       ├── bench.rs    # MockProver 快速测试
│       └── kzg_bench.rs # KZG 真实证明基准测试

```

### 2) 电路实现: MulCircuit（乘法）

约束逻辑:

```rust
// PLONK 风格约束: q_mul * (a * b - c) = 0
// 公开: c (product)
// 私有: a, b (factors)

```

实现要点:

- 使用 SimpleFloorPlanner

- 自定义 gate: `q_mul * (a*b - c) = 0`

- Instance 列绑定公开输入 

- 公开列通过 constrain_instance 约束公开输入

### 3) 测试状态

- MockProver: ✅ 通过（k=8 → ~1.5ms 合成检查）

- KZG 真实证明: ✅ Setup → ✅ Keygen → ✅ Prove → ✅ Verify 全流程通过

---

## 性能基准

环境: Windows 11 / Rust 1.75 / Release / BN256

| k值 | 电路行数 | Setup+Keygen | Prove | Verify | 证明大小 |
|-----|---------|--------------|-------|--------|----------|
| 6   | 64      | 49.5 ms      | 50.6 ms | 3.3 ms | 1600 B   |
| 8   | 256     | 85.6 ms      | 106.2 ms| 4.8 ms | 1728 B   |
| 10  | 1024    | 431.3 ms     | 186.8 ms| 10.1 ms| 1856 B   |

对比（Groth16/arkworks, Combined 电路 ~72 约束）:

| 指标       | Groth16 | Halo2(k=8) | 说明 |
|------------|---------|------------|------|
| Setup      | 26.8 ms | 85.6 ms    | Groth16 更快 |
| Prove      | 10.0 ms | 106.2 ms   | Groth16 ~10.6× 更快 |
| Verify     | 3.6 ms  | 4.8 ms     | Groth16 更快（接近） |
| 证明大小   | 128 B   | 1728 B     | Groth16 更小（~13.5×） |
| Setup 类型 | 每电路  | 通用/透明  | Halo2 无需 Trusted Setup |

---

## 关键发现

### 1. 证明大小差异 ✨

- **Groth16**: 128 bytes（恒定，2×G1 + 1×G2）

- **Halo2**: ~1.7KB（k=8），随 k 值增长

- **分析**: Groth16 证明大小与电路复杂度无关，Halo2 随 k 值线性增长

- **结论**: **链上验证场景 Groth16 优势明显**（Gas 成本低）

### 2. 验证时间对比

- **Groth16**: 3.6ms（恒定，3 次配对）

- **Halo2**: 4.8ms（k=8），随 k 值增长

- **分析**: 两者验证时间相近，但 Groth16 更稳定

- **结论**: **Groth16 验证成本更可预测**

### 3. 证明时间对比 ⚠️

- **Groth16**: 10.0ms（~72 约束）

- **Halo2**: 106.2ms（k=8，256 行）

- **分析**: Halo2 证明时间是 Groth16 的 **10.6 倍**

- **结论**: **Groth16 证明效率显著更高**

### 4. Setup 灵活性 ✨

- **Groth16**: 需为每个电路单独 Trusted Setup

- **Halo2**: **通用 Setup**，一次可用于任意电路

- **分析**: Halo2 开发迭代更灵活，Groth16 生产优化更友好

- **结论**: **Halo2 适合快速原型，Groth16 适合生产部署**

### 5. 扩展性观察

- **Groth16**: 证明时间与约束数亚线性增长（实测 ×1.7）

- **Halo2**: 证明时间随 k 值快速增长（k=6→8: ×2.1, k=8→10: ×1.8）

- **结论**: **Groth16 扩展性更好**

---

## 技术架构差异

### Groth16 (R1CS)

```

电路 → R1CS 约束 → QAP 多项式 → Trusted Setup → CRS
     ↓
  Prove: 多项式求值 + 配对计算 → 128 bytes 证明
     ↓
  Verify: 3 次配对运算 → Bool

```

**特点**:

- 约束系统：R1CS（Rank-1 Constraint System）

- 证明大小：恒定 128 bytes

- 验证成本：恒定 3 次配对

- Setup：每个电路需独立 Trusted Setup

### Halo2 (PLONK)

```

电路 → 自定义 Gate → 置换证明 → 通用 Setup → SRS
     ↓
  Prove: KZG 承诺 + PLONK 证明 → ~1.7KB 证明
     ↓
  Verify: KZG 验证 + 置换检查 → Bool

```

**特点**:

- 约束系统：PLONK（自定义 Gate）

- 证明大小：随电路大小增长（~1.7KB for k=8）

- 验证成本：随 k 值增长

- Setup：通用 Setup（一次即可）

---

## 技术评估

优势:

- 透明/通用 Setup（无需每电路 Trusted Setup）

- 原生支持递归与聚合

- Gate 灵活，表达能力强，适合 zkVM 类场景

劣势:

- 证明生成更慢，证明体积更大

- API 偏底层（Layouter/Rotation 等），学习曲线陡

适用场景:

- ✅ 需要递归/聚合（跨链桥、批量证明）

- ✅ 通用/多变电路（合约隐私、zkVM）

- ❌ 极致链上成本敏感（优先 Groth16）

---

## 使用场景建议

### ✅ 适合 Groth16 的场景

1. **链上隐私交易**
   - 需要最小证明大小（降低链上存储成本）
   - 需要恒定验证时间（Gas 成本可预测）
   - 电路相对固定（RingCT 模式）

2. **高频验证场景**
   - 验证次数远多于证明次数
   - 对验证效率极度敏感
   - 如：Layer2 Rollup 的批量交易验证

3. **生产环境优化**
   - 电路已稳定，不需频繁更新
   - 追求极致性能
   - Trusted Setup 可接受

### ✅ 适合 Halo2 的场景

1. **递归证明**
   - 需要聚合多个证明
   - 跨链桥场景（聚合多链证明）
   - zkRollup 批量证明聚合

2. **快速迭代开发**
   - 电路需要频繁修改
   - 原型验证阶段
   - 通用 Setup 避免重复 Trusted Setup

3. **透明 Setup**
   - 无法接受 Trusted Setup 的信任假设
   - 需要完全透明的系统
   - 去中心化场景

---

## SuperVM 选型建议

1) RingCT 隐私交易（不建议 Halo2，推荐 Groth16）

- 目标: 最小验证成本与最小证明大小

- 方案: arkworks Groth16 + BN254

2) 通用合约隐私（建议 Halo2/Marlin）

- 目标: 电路频繁演进，避免重复 Setup

- 方案: Halo2（或 arkworks Marlin）

3) 跨链隐私桥（建议 Halo2）

- 目标: 递归聚合 N 个证明为 1 个

- 方案: Halo2 递归/聚合策略

---

## 实现难点与对策

1) Layouter 抽象复杂 → 提供清晰的 region.assign_advice 模式与注释
2) 公开输入绑定 → 在 synthesize 中使用 constrain_instance 明确约束
3) k 值选择 → 依据电路行数估算，预留 1-2 档余量（简单: k=6~8；中等: k=10~12）

---

## SuperVM 技术选型

### 推荐策略：**混合使用** ✨

#### Phase 1: 开发与原型（Halo2）

- 使用 Halo2 快速迭代电路设计

- 通用 Setup 加速开发流程

- 验证核心功能可行性

#### Phase 2: 生产优化（Groth16）

- 将稳定电路迁移到 Groth16

- 最小化证明大小和验证成本

- 链上部署使用 Groth16

#### Phase 3: 混合架构（Groth16 + Halo2）

- **链上部分**: Groth16 验证（Gas 低）

- **链下聚合**: Halo2 递归（聚合多个 Groth16 证明）

- **最优平衡**: 发挥两者各自优势

### 具体实施

1. **隐私交易（RingCT）**: ✅ Groth16
   - 证明大小 128 bytes
   - 验证时间 3.6ms
   - 链上 Gas 成本低

2. **跨链桥**: ✅ Halo2
   - 递归聚合多链证明
   - 降低目标链验证次数

3. **zkVM 开发**: ✅ Halo2
   - 电路快速迭代
   - 通用 Setup 灵活

---

## 代码示例

### 运行 MockProver 测试

```bash
cd halo2-eval
cargo test

```

### 运行 KZG 基准测试

```bash
cd halo2-eval
cargo run --release --bin kzg_bench

```

**预期输出**:

```

=== Halo2 (KZG/Bn256) 性能基准测试 ===

k=6 (2^6=64 行): Setup+Keygen=49.53ms | Prove=50.64ms | Verify=3.32ms | 证明大小=1600 bytes
k=8 (2^8=256 行): Setup+Keygen=85.60ms | Prove=106.19ms | Verify=4.78ms | 证明大小=1728 bytes
k=10 (2^10=1024 行): Setup+Keygen=431.32ms | Prove=186.83ms | Verify=10.13ms | 证明大小=1856 bytes

对比 Groth16 (arkworks):
  Groth16: Setup=26.8ms | Prove=10.0ms | Verify=3.6ms | 证明大小=128 bytes

```

---

## 经验教训

### 1. API 版本兼容性

- halo2_proofs 0.3 的 API 与 0.2/0.1 有显著差异

- 需要查阅具体版本文档，不能直接套用教程代码

- SingleVerifier 是 0.3 的推荐验证策略

### 2. 电路设计差异

- Groth16: R1CS 约束（a*b=c 形式）

- Halo2: 自定义 Gate + 置换（更灵活但更复杂）

- Halo2 学习曲线更陡

### 3. 性能权衡

- 证明大小：Groth16 绝对优势

- Setup 灵活性：Halo2 绝对优势

- 需要根据具体场景选择

### 4. 生态成熟度

- Groth16: 生产环境广泛使用（Zcash, Filecoin）

- Halo2: 相对较新（Zcash Orchard 升级）

- arkworks 生态更完善

---

## 下一步计划

短期（已完成）

- [x] MulCircuit 基础电路与 MockProver 验证

- [x] KZG 真实证明与基准数据

- [x] Groth16 对比分析与结论

中期（Week 4-5）
1. **Groth16 生产实现**
   - 完善隐私交易电路（RingCT）
   - 批量验证优化
   - 链上 Solidity 验证器

2. **Halo2 递归探索**
   - 实现简单递归电路
   - 验证聚合能力
   - 评估性能开销

3. **混合架构 PoC**
   - Groth16 链上验证
   - Halo2 链下聚合
   - 跨链桥原型
---

## 参考资料

官方:

- Halo2 Book: https://zcash.github.io/halo2/

- API Docs: https://docs.rs/halo2_proofs/

论文:

- Halo: https://eprint.iacr.org/2019/1021

- PLONK: https://eprint.iacr.org/2019/953

代码:

- 本项目: `halo2-eval/`

- 对比: `zk-groth16-test/`

---

## 总结

✅ **Halo2 评估完成**  
✅ **性能数据收集完毕**  
✅ **技术选型明确**  

结论:

- Groth16: 证明小/验证快，适合生产与链上成本敏感场景

- Halo2: 透明 Setup/支持递归，适合通用电路与聚合需求

- 综合建议: 按场景混合使用（链上 Groth16，链下 Halo2 聚合）

---

**里程碑意义**:

- 完成了 Week 3-4 的 zkSNARK 技术评估

- 为 SuperVM 隐私层提供了清晰的技术路线

- 建立了 Groth16 + Halo2 的完整实现基础

—

© 2025 SuperVM Privacy Research | GPL-3.0 License




