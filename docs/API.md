# vm-runtime API Documentation

版本: v0.9.0  
最后更新: 2025-11-05

开发者/作者：King Xujue

## 概述

`vm-runtime` 是一个基于 wasmtime 的 WebAssembly 运行时库，提供存储抽象、链上下文访问和事件系统。

## 核心类型

### Runtime&lt;S: Storage&gt;

WASM 虚拟机运行时，支持自定义存储后端。

#### 泛型参数

- `S`: 实现 `Storage` trait 的存储后端类型

#### 构造函数

**new**
```rust
pub fn new(storage: S) -> Self
```
创建新的运行时实例。

**参数**: `storage` - 存储后端实现

**示例**:
```rust
use vm_runtime::{Runtime, MemoryStorage};
let runtime = Runtime::new(MemoryStorage::new());
```

**new_with_routing (v2.0+)**
```rust
pub fn new_with_routing(storage: S) -> Self
```
创建带路由能力的运行时（支持对象所有权和MVCC调度）。

---

## Storage Trait

存储抽象接口。

```rust
pub trait Storage {
    fn get(&amp;self, key: &amp;[u8]) -> Result&lt;Option&lt;Vec&lt;u8&gt;&gt;&gt;;
    fn set(&amp;mut self, key: &amp;[u8], value: &amp;[u8]) -> Result&lt;()&gt;;
    fn delete(&amp;mut self, key: &amp;[u8]) -> Result&lt;()&gt;;
}
```

### 方法

**get** - 读取键值  
**set** - 写入键值对  
**delete** - 删除键值对  

---

## MVCC Store (v0.7.0+)

多版本并发控制存储。

### MvccStore

提供快照隔离的事务支持。

**方法**:
- `new()` - 创建新的 MVCC 存储实例
- `begin()` - 开始新事务，返回事务句柄
- `enable_auto_gc(config)` - 启用自动垃圾回收
- `gc_now()` - 立即执行垃圾回收

**示例**:
```rust
use vm_runtime::MvccStore;

let store = MvccStore::new();
let mut txn = store.begin();
txn.write(b"key", b"value")?;
txn.commit()?;
```

### Txn

事务句柄。

**方法**:
- `read(&amp;mut self, key: &amp;[u8])` - 读取键值（v0.9.0+ 需要 &amp;mut self）
- `write(&amp;mut self, key, value)` - 写入键值
- `commit(self)` - 提交事务
- `abort(self)` - 放弃事务

---

## 并行调度器 (v0.9.0+)

### MvccScheduler

基于 MVCC 的并行事务调度器。

**方法**:
- `new()` - 创建默认配置的调度器
- `execute_batch(store, transactions)` - 批量并行执行事务
- `stats()` - 获取调度器统计信息

**示例**:
```rust
let scheduler = MvccScheduler::new();
let store = MvccStore::new();

let txns = vec![
    (1, |txn: &amp;mut Txn| {
        txn.write(b"key1", b"value1")?;
        Ok(0)
    }),
];

let result = scheduler.execute_batch(&amp;store, txns);
```

---

## 对象所有权模型 (v2.0+)

### OwnershipManager

Sui 风格的对象所有权管理。

**方法**:
- `create_object(id, owner, ownership_type)` - 创建新对象
- `transfer_object(object_id, from, to)` - 转移对象所有权
- `access_object(object_id, accessor, access_type)` - 检查对象访问权限
- `freeze_object(object_id, owner)` - 冻结对象为不可变
- `share_object(object_id, owner)` - 将对象转为共享

---

## SuperVM 统一接口 (v2.0+)

### SuperVM

统一的虚拟机入口，支持公开/私有模式路由。

**方法**:
```rust
pub fn execute_transaction(
    &amp;self,
    tx: VmTransaction,
    privacy: Privacy,
) -> Result&lt;ExecutionReceipt&gt;
```

执行交易（根据隐私模式路由）。

**示例**:
```rust
use vm_runtime::{SuperVM, Privacy, VmTransaction};

let vm = SuperVM::new(MemoryStorage::new());
let receipt = vm.execute_transaction(tx, Privacy::Public)?;
```

---

## Host Functions

WASM 模块可导入的 host 函数。

### storage_api

- `storage_get(key_ptr, key_len)` - 读取存储值
- `storage_set(key_ptr, key_len, value_ptr, value_len)` - 写入存储值
- `storage_delete(key_ptr, key_len)` - 删除存储值

### chain_api

- `block_number()` - 获取当前区块号
- `timestamp()` - 获取当前时间戳
- `emit_event(data_ptr, data_len)` - 发出事件

### crypto_api

- `sha256(input_ptr, input_len, output_ptr)` - 计算 SHA256 哈希
- `verify_ed25519(msg_ptr, msg_len, sig_ptr, pubkey_ptr)` - 验证 Ed25519 签名

---

## 版本历史

### v0.9.0 (2025-06-03)
-  Write Skew 修复（读集合跟踪 + 三阶段提交）
-  金额守恒验证通过
-  性能优化：187K TPS（低竞争），85K TPS（高竞争）

### v0.7.0
-  自适应 GC（自动垃圾回收）
-  MVCC 存储实现

### v0.6.0
-  并行执行引擎
-  冲突检测与依赖分析

---

## 许可证

本项目采用 GPL-3.0-or-later 许可证。详见根目录 LICENSE 文件。

版权所有  2025 XujueKing &lt;leadbrand@me.com&gt;