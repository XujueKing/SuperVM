# SuperVM - ZK 隐私层开发路线图

**开发者**: king  
**开始时间**: 2025年2月  
**当前阶段**: ✅ 技术评估完成 → 🚀 生产实现启动

---

## 📋 总体目标

为 SuperVM 构建高性能、可验证的隐私交易层，支持：
- **隐私转账**: RingCT 风格的混币交易
- **跨链隐私**: 跨链桥的零知识证明聚合
- **zkVM 基础**: 为未来 zkVM 开发建立技术基础

---

## 🎯 里程碑规划

### ✅ Phase 1: 技术选型与原型（Week 1-4）

**目标**: 评估 Groth16、PLONK、Halo2 技术栈，选择最佳方案。

#### Week 1-2: Groth16 评估 ✅
- [x] 学习 Monero RingCT 原理
- [x] 实现 curve25519 Pedersen 承诺
- [x] arkworks-rs 生态调研
- [x] **Groth16 基础电路实现**
  - [x] Simple Circuit（单个约束）
  - [x] Pedersen Commitment Circuit（10 约束）
  - [x] 64-bit Range Proof Circuit（64 约束）
  - [x] Combined Circuit（Pedersen + Range, 72 约束）
- [x] **性能基准测试**
  - Simple: 6.9ms setup, 4.4ms prove, 2.0ms verify, 128 bytes
  - Combined: 26.8ms setup, 10.0ms prove, 3.6ms verify, 128 bytes
- [x] 文档：`docs/research/zk-evaluation.md`

**交付物**: `zk-groth16-test/` crate + 性能基准数据

#### Week 3-4: Halo2 评估 ✅
- [x] **Halo2 电路实现**
  - [x] Multiply Circuit（PLONK-style Gate）
  - [x] MockProver 功能验证
  - [x] KZG 真实证明实现
- [x] **性能基准测试**
  - k=6: 49.5ms setup, 50.6ms prove, 3.3ms verify, 1600 bytes
  - k=8: 85.6ms setup, 106.2ms prove, 4.8ms verify, 1728 bytes
  - k=10: 431.3ms setup, 186.8ms prove, 10.1ms verify, 1856 bytes
- [x] **Groth16 vs Halo2 全面对比**
  - 证明大小：Groth16 小 **13.5×**
  - 证明时间：Groth16 快 **10.6×**
  - Setup 灵活性：Halo2 通用 Setup 优势
  - 递归能力：Halo2 原生支持
- [x] **技术选型结论**
  - 链上隐私交易 → Groth16（证明小、验证快）
  - 跨链桥聚合 → Halo2（递归支持）
  - zkVM 开发 → Halo2（开发迭代快）
  - 推荐：混合策略，发挥各自优势
- [x] 文档：`docs/research/halo2-eval-summary.md`

**交付物**: `halo2-eval/` crate + 技术选型建议

---

### 🚀 Phase 2: 生产级 RingCT 实现（Week 5-8）

**目标**: 基于 Groth16 实现链上隐私交易，验证链上部署可行性。

#### Week 5-6: RingCT 电路完善
- [ ] **扩展 Combined Circuit**
  - [x] 支持多输入/多输出（UTXO 模型）
  - [x] 实现环签名电路（Ring Signature）
    - 完成情况：简化版环签名电路（Key Image + 成员验证）已实现，4 项单元测试通过
    - 约束指标：ring_size=3 共 253 约束，约 84 约束/成员
    - 报告文档：`zk-groth16-test/RING_SIGNATURE_REPORT.md`
  - [x] 添加金额隐藏与平衡证明
  - [x] 将环签名电路集成到 Multi-UTXO 交易（Key Image 公开输入 + 环成员资格 + 双花检查）
  - [ ] 集成 Bulletproofs Range Proof（更高效）
- [ ] **性能优化**
  - [ ] 约束数优化（目标 <200 约束）
    - 参考报告：`zk-groth16-test/OPTIMIZATION_REPORT.md`
  - [ ] 并行化证明生成
  - [ ] 批量验证支持
- [x] **测试覆盖**
  - [x] 功能测试（正常/异常流）
  - [x] 边界测试（最大环大小、极限金额）
  - [x] 对抗性测试（双花、伪造签名）
  - 报告文档：`zk-groth16-test/ADVERSARIAL_TESTS_REPORT.md`
  - [ ] 性能回归测试

**关键指标**:
- 证明时间 < 100ms（单核）
- 验证时间 < 10ms
- 证明大小 = 128 bytes（Groth16 恒定）

#### Week 7-8: 链上验证器与集成
- [ ] **Solidity 验证器生成**
  - [ ] 使用 arkworks 导出验证密钥
  - [ ] 生成 Griddycode EVM 验证合约
  - [ ] 优化 Gas 成本（目标 <200k Gas）
- [ ] **SuperVM 集成**
  - [ ] 定义隐私交易 API
  - [ ] 实现 UTXO 状态管理
  - [ ] 集成证明验证逻辑
- [ ] **端到端测试**
  - [ ] 本地测试链部署
  - [ ] 发送隐私交易并验证
  - [ ] 性能压测（TPS 测试）

**交付物**: `supervm-ringct/` crate + Solidity 验证器 + 集成测试

---

### 🔗 Phase 3: 跨链桥与递归证明（Week 9-12）

**目标**: 使用 Halo2 实现跨链证明聚合，降低目标链验证成本。

#### Week 9-10: Halo2 递归电路
- [ ] **递归证明实现**
  - [ ] 实现简单递归电路（验证单个 Groth16 证明）
  - [ ] 扩展到批量聚合（N 个证明 → 1 个 Halo2 证明）
  - [ ] 测试递归深度限制
- [ ] **性能评估**
  - [ ] 聚合 10/50/100 个证明的开销
  - [ ] 证明大小增长曲线
  - [ ] 验证时间对比

#### Week 11-12: 跨链桥原型
- [ ] **跨链架构设计**
  - [ ] 定义跨链消息格式
  - [ ] 实现链下聚合服务
  - [ ] 目标链验证合约
- [ ] **PoC 实现**
  - [ ] 源链：发送多个隐私交易
  - [ ] 链下：Halo2 聚合证明
  - [ ] 目标链：验证单个聚合证明
- [ ] **压力测试**
  - [ ] 吞吐量测试（聚合速度）
  - [ ] 成本分析（Gas 对比）

**交付物**: `supervm-bridge/` crate + 跨链桥 PoC

---

### 🏗️ Phase 4: zkVM 基础设施（Week 13-16）

**目标**: 探索 zkVM 开发路径，为通用可验证计算奠基。

#### Week 13-14: zkVM 调研与选型
- [ ] **现有 zkVM 评估**
  - [ ] RISC Zero（STARK-based）
  - [ ] zkMIPS（MIPS 指令集）
  - [ ] SP1（Succinct zkVM）
  - [ ] Polygon Miden（STARK + WASM）
- [ ] **技术对比**
  - [ ] 指令集架构（RISC-V vs MIPS vs WASM）
  - [ ] 证明系统（STARK vs SNARK）
  - [ ] 性能基准（证明时间、大小）
  - [ ] 生态成熟度

#### Week 15-16: zkVM PoC
- [ ] **选择方案实施**
  - [ ] 集成现有 zkVM 或自研简化版
  - [ ] 实现示例程序（如 Fibonacci）
  - [ ] 生成并验证执行证明
- [ ] **与 SuperVM 集成探索**
  - [ ] 定义 zkVM 调用接口
  - [ ] WASM → zkVM 转换可行性
  - [ ] 性能开销评估

**交付物**: zkVM 技术报告 + PoC 代码

---

## 📊 技术栈总览

### 已选定技术

| 组件 | 技术栈 | 理由 |
|------|--------|------|
| **链上隐私交易** | arkworks (Groth16) | 证明小（128B）、验证快（3.6ms）、Gas 低 |
| **跨链聚合** | Halo2 (KZG) | 递归证明、通用 Setup |
| **zkVM 开发** | Halo2 / RISC Zero | 开发迭代快、生态支持 |
| **曲线** | Bn254 (BN128) | EVM 原生支持、配对友好 |
| **哈希** | Poseidon / Blake2s | 零知识友好 |

### 依赖库版本

```toml
# Groth16
ark-groth16 = "0.4"
ark-bn254 = "0.4"
ark-ff = "0.4"
ark-std = "0.4"
ark-relations = "0.4"

# Halo2
halo2_proofs = "0.3"
halo2curves = "0.6"

# 工具
rand = "0.8"
blake2 = "0.10"
```

---

## 🎯 关键性能指标

### RingCT 隐私交易
- **证明生成**: < 100ms（单核）
- **验证时间**: < 10ms
- **证明大小**: 128 bytes（Groth16 恒定）
- **Gas 成本**: < 200k（Solidity 验证器）
- **吞吐量**: > 100 TPS（批量验证）

### 跨链桥聚合
- **聚合比例**: 100:1（100 个证明 → 1 个聚合证明）
- **聚合时间**: < 10s（100 个证明）
- **聚合证明大小**: < 10KB
- **Gas 节省**: > 90%（vs 验证 100 次）

---

## 🚧 已知挑战与缓解

### 1. Trusted Setup 风险
- **挑战**: Groth16 需要 Trusted Setup，存在信任假设
- **缓解**:
  - 使用 Powers of Tau 公共参数（如 Zcash Ceremony）
  - 提供 Halo2 替代方案（无 Trusted Setup）
  - 长期目标：迁移到 STARK（完全透明）

### 2. 性能 vs 隐私权衡
- **挑战**: 环签名大小影响性能（环越大越慢）
- **缓解**:
  - 提供多档环大小选项（10/50/100）
  - 批量验证优化
  - 链下预聚合

### 3. 链上 Gas 成本
- **挑战**: Pairing 操作 Gas 高（~45k per pairing）
- **缓解**:
  - 使用 Bn254（EVM 预编译支持）
  - 批量验证（均摊成本）
  - Layer2 部署（Gas 更低）

### 4. 递归证明复杂性
- **挑战**: Halo2 递归实现复杂，性能开销大
- **缓解**:
  - 从简单递归开始（1 层）
  - 使用成熟库（如 halo2_gadgets）
  - 性能不达标则使用批量验证替代

---

## 📚 学习路径建议

### 零知识证明基础
1. ✅ Pedersen Commitment
2. ✅ Range Proof（Bulletproofs/zkSNARK）
3. ✅ Groth16 原理与实现
4. ✅ PLONK/Halo2 原理
5. ⏳ 递归证明
6. ⏳ STARK 基础

### 隐私协议
1. ✅ Monero RingCT
2. ⏳ Zcash Sapling/Orchard
3. ⏳ Aztec Connect
4. ⏳ Tornado Cash 原理

### zkVM 与可验证计算
1. ⏳ RISC-V 指令集
2. ⏳ Cairo/Starknet
3. ⏳ RISC Zero
4. ⏳ Polygon Miden

---

## 📈 成功指标

### 技术指标
- ✅ Groth16 电路实现并通过测试
- ✅ Halo2 电路实现并通过测试
- ✅ 性能基准数据收集完成
- ✅ 技术选型报告完成
- ⏳ RingCT 链上部署成功
- ⏳ 跨链桥 PoC 运行成功
- ⏳ zkVM 示例程序运行

### 性能指标
- ✅ Groth16 证明 < 50ms（Simple Circuit）
- ⏳ RingCT 证明 < 100ms
- ⏳ 验证 Gas < 200k
- ⏳ 跨链聚合 100:1 成功

### 文档指标
- ✅ zk-evaluation.md（技术对比）
- ✅ halo2-eval-summary.md（Halo2 总结）
- ⏳ RingCT 设计文档
- ⏳ 跨链桥架构文档
- ⏳ zkVM 技术报告

---

## 🔗 相关资源

### 项目代码
- `zk-groth16-test/`: Groth16 实现与测试
- `halo2-eval/`: Halo2 实现与测试
- `docs/research/`: 技术研究文档

### 外部参考
- [arkworks-rs](https://github.com/arkworks-rs): Rust zkSNARK 库
- [halo2](https://github.com/zcash/halo2): Zcash Halo2 实现
- [Monero RingCT](https://www.getmonero.org/resources/moneropedia/ringCT.html)
- [Zcash Protocol](https://zips.z.cash/)

---

## 📝 变更日志

### 2025-11-05
- ✅ 完成 Groth16 评估（Week 1-2）
- ✅ 完成 Halo2 评估（Week 3-4）
- ✅ 完成技术选型结论
- 🚀 启动 Phase 2: RingCT 生产实现

### 2025-11 (初始)
- 📋 创建 ZK 隐私层路线图
- 🎯 定义 4 个 Phase 开发计划
- 📊 确定技术栈与性能指标

---

## 👥 贡献指南

欢迎参与 SuperVM 隐私层开发！

### 如何贡献
1. Fork 仓库并创建功能分支
2. 实现功能并添加测试
3. 更新文档（如需要）
4. 提交 Pull Request

### 开发规范
- 使用 `cargo fmt` 格式化代码
- 运行 `cargo clippy` 检查 lint
- 确保所有测试通过 `cargo test`
- 性能测试使用 `cargo bench`

### 联系方式
- Issues: 提交 bug 或功能请求
- Discussions: 技术讨论与提问

---

**下一步**: 开始 Week 5-6 的 RingCT 电路完善工作！🚀
