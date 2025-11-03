# vm-runtime 扩展 - 阶段计划

开发者：king

## 1. Host Functions 基础设施

### 存储操作 ✅ 已完成
- [x] Key-Value 存储接口设计 - `Storage` trait
- [x] Get/Set/Delete/Scan API - 完整实现
  - [x] `storage_get(key_ptr, key_len) -> i64`
  - [x] `storage_read_value(ptr, len) -> i32`
  - [x] `storage_set(key_ptr, key_len, value_ptr, value_len) -> i32`
  - [x] `storage_delete(key_ptr, key_len) -> i32`
- [x] MemoryStorage 实现 (BTreeMap 后端)
- [ ] 状态缓存与批量提交 (待优化)
- [ ] 存储费用计算 (待实现)

**实现说明**:
- 使用 `Rc<RefCell<Storage>>` 共享存储状态
- `storage_get` 缓存结果到 `last_get`
- `storage_read_value` 从缓存读取,避免重复查询
- 所有函数已通过单元测试验证

### 链上下文 ✅ 部分完成
- [x] Block/Transaction 信息结构
  - [x] block_number: u64
  - [x] timestamp: u64
- [x] 时间戳访问 - `timestamp() -> i64`
- [x] 区块号访问 - `block_number() -> i64`
- [ ] 随机数生成 (待实现)
- [ ] Gas 计量与限制 (待实现)
- [ ] 交易发送者信息 (待实现)

**实现说明**:
- 通过 `HostState` 结构传递上下文
- `execute_with_context` API 接受并返回上下文信息
- 已在 Demo 2 中演示完整流程

### 密码学 ✅ 部分完成
- [x] keccak256 哈希
- [x] 其他哈希函数 (sha256)
- [x] secp256k1 签名验证
- [x] ed25519 签名验证
- [x] 公钥恢复 (secp256k1 recover)
- [ ] 地址派生 (待实现)

**实现说明**:
- 新增 `crypto_api` 模块,提供 `sha256/keccak256/verify_secp256k1/verify_ed25519/recover_secp256k1_pubkey`
- 在 `node-core` Demo 3 中演示哈希计算,并通过事件输出 32 字节哈希
- 已添加单元与集成测试,`cargo test -p vm-runtime` 全部通过

### 事件系统 ✅ 已完成
- [x] 事件数据结构 - `Vec<Vec<u8>>`
- [x] 发送接口 - `emit_event(data_ptr, data_len) -> i32`
- [x] 读取接口:
  - [x] `events_len() -> i32`
  - [x] `read_event(index, ptr, len) -> i32`
- [x] execute_with_context 返回完整事件列表
- [ ] 索引支持 (待实现,用于查询优化)

**实现说明**:
- 事件存储在 `HostState.events: Vec<Vec<u8>>`
- 支持任意二进制数据
- 已在 Demo 2 中展示 "UserAction" 和 "BlockProcessed" 事件

## 2. 并行执行 PoC

### 调度系统
- [ ] 账户访问模式分析
- [ ] 依赖图构建
- [ ] 并行调度算法

### 冲突检测
- [ ] 读写集收集
- [ ] 冲突检测算法
- [ ] 性能优化

### 状态管理
- [ ] 快照与回滚
- [ ] 批量提交优化
- [ ] MVCC 研究

## 验证与测试
- [ ] 单元测试覆盖
- [ ] 集成测试
- [ ] 性能基准测试
- [ ] 并发正确性测试

## 预期产出
1. vm-runtime 扩展实现：
   - host functions 完整支持
   - 并行执行引擎
2. 示例合约与测试用例
3. 性能测试报告

## 时间预估
- Host Functions 基础设施：2-3 周
- 并行执行 PoC：3-4 周
- 测试与优化：1-2 周