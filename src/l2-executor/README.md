# L2 Executor (zkVM + Aggregation Skeleton)

SuperVM 的 L2 执行层负责通用零知识证明 (zkVM) 与多证明聚合。该 crate 提供：

- `TraceZkVm`: 基于执行轨迹承诺的轻量 zkVM 骨架，用于演示如何生成/验证证明
- `FibonacciProgram`: 参考程序，展示如何将任意算法映射为约束 + 公共输出
- `Sha256Program`: STARK 风格哈希轨迹示例，可选择 chunk 大小
- `MerkleAggregator`: 聚合多个证明，形成单一提交 (hash tree) 以降低上链成本

## 快速开始

```powershell
cd d:\WEB3_AI开发\虚拟机开发
cargo test -p l2-executor
```

测试会生成 Fibonacci 与 SHA256 证明并验证，同时演示双证明聚合。真正的 zkVM/递归聚合将在后续迭代替换为 RISC Zero / Halo2 实现，本骨架主要用于对接 L1 ExecutionEngine 与 Phase 8 规划。
