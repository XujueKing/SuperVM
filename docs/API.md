# vm-runtime API Documentation

版本: v0.1.0  
最后更新: 2025-01-03

## 概述

`vm-runtime` 是一个基于 wasmtime 的 WebAssembly 运行时库,提供存储抽象、链上下文访问和事件系统。

## 核心类型

### `Runtime<S: Storage>`

WASM 虚拟机运行时,支持自定义存储后端。

#### 泛型参数

- `S`: 实现 `Storage` trait 的存储后端类型

#### 构造函数

```rust
pub fn new(storage: S) -> Self
```

创建新的运行时实例。

**参数**:
- `storage`: 存储后端实例

**示例**:
```rust
use vm_runtime::{Runtime, MemoryStorage};

let runtime = Runtime::new(MemoryStorage::new());
```

#### 方法

##### `execute_add`

```rust
pub fn execute_add(&self, module_bytes: &[u8], a: i32, b: i32) -> Result<i32>
```

执行简单的加法函数(演示用)。

**参数**:
- `module_bytes`: WASM 模块字节码
- `a`: 第一个加数
- `b`: 第二个加数

**返回**: 
- `Ok(i32)`: 计算结果
- `Err`: 执行错误

**示例**:
```rust
let wat = r#"
(module
  (func $add (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
)
"#;
let wasm = wat::parse_str(wat)?;
let result = runtime.execute_add(&wasm, 7, 8)?;
assert_eq!(result, 15);
```

##### `execute_with_context`

```rust
pub fn execute_with_context(
    &self,
    module_bytes: &[u8],
    func_name: &str,
    block_number: u64,
    timestamp: u64,
) -> Result<(i32, Vec<Vec<u8>>, u64, u64)>
```

在指定的区块上下文中执行 WASM 函数,并收集事件。

**参数**:
- `module_bytes`: WASM 模块字节码
- `func_name`: 要调用的导出函数名
- `block_number`: 当前区块号
- `timestamp`: 当前时间戳(Unix 时间)

**返回**:
- `Ok((result, events, block_number, timestamp))`:
  - `result`: 函数返回值(i32)
  - `events`: 收集到的事件列表
  - `block_number`: 传入的区块号
  - `timestamp`: 传入的时间戳
- `Err`: 执行错误

**示例**:
```rust
let wat = r#"
(module
  (import "chain_api" "emit_event" (func $emit_event (param i32 i32) (result i32)))
  (import "chain_api" "block_number" (func $block_number (result i64)))
  (memory (export "memory") 1)
  (data (i32.const 0) "Hello")
  
  (func (export "run") (result i32)
    i32.const 0
    i32.const 5
    call $emit_event
    drop
    call $block_number
    i32.wrap_i64
  )
)
"#;
let wasm = wat::parse_str(wat)?;
let (result, events, bn, ts) = runtime.execute_with_context(
    &wasm,
    "run",
    12345,
    1704067200,
)?;

assert_eq!(events[0], b"Hello");
assert_eq!(bn, 12345);
```

##### `storage`

```rust
pub fn storage(&self) -> Rc<RefCell<S>>
```

获取存储后端的引用(用于直接访问)。

**返回**: 共享的存储引用

---

## Storage Trait

存储抽象接口。

```rust
pub trait Storage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&mut self, key: &[u8]) -> Result<()>;
    fn scan(&self, prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}
```

### 方法

#### `get`

```rust
fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>
```

读取键对应的值。

**参数**:
- `key`: 键字节数组

**返回**:
- `Ok(Some(value))`: 找到值
- `Ok(None)`: 键不存在
- `Err`: 读取错误

#### `set`

```rust
fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()>
```

写入键值对。

**参数**:
- `key`: 键字节数组
- `value`: 值字节数组

**返回**:
- `Ok(())`: 写入成功
- `Err`: 写入错误

#### `delete`

```rust
fn delete(&mut self, key: &[u8]) -> Result<()>
```

删除键。

**参数**:
- `key`: 键字节数组

**返回**:
- `Ok(())`: 删除成功(即使键不存在)
- `Err`: 删除错误

#### `scan`

```rust
fn scan(&self, prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>>
```

扫描指定前缀的所有键值对。

**参数**:
- `prefix`: 键前缀
- `limit`: 最大返回数量

**返回**:
- `Ok(vec)`: 键值对列表
- `Err`: 扫描错误

---

### `MemoryStorage`

基于 `BTreeMap` 的内存存储实现。

```rust
pub struct MemoryStorage {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}
```

#### 构造函数

```rust
pub fn new() -> Self
```

创建空的内存存储。

**示例**:
```rust
use vm_runtime::MemoryStorage;

let storage = MemoryStorage::new();
```

---

## Host Functions

从 WASM 模块中可调用的宿主函数。

### Storage API (`storage_api` 模块)

#### `storage_get`

```
storage_get(key_ptr: i32, key_len: i32) -> i64
```

读取键的值,并缓存到 `last_get`。

**参数**:
- `key_ptr`: 键数据在 WASM 内存中的指针
- `key_len`: 键的长度

**返回**:
- 高 32 位: 是否找到(1=找到, 0=未找到)
- 低 32 位: 值的长度

**WAT 示例**:
```wat
(import "storage_api" "storage_get" (func $storage_get (param i32 i32) (result i64)))
```

#### `storage_read_value`

```
storage_read_value(ptr: i32, len: i32) -> i32
```

从 `last_get` 缓存读取值到 WASM 内存。

**参数**:
- `ptr`: 目标内存地址
- `len`: 读取长度

**返回**: 实际写入的字节数

**WAT 示例**:
```wat
(import "storage_api" "storage_read_value" (func $storage_read_value (param i32 i32) (result i32)))
```

#### `storage_set`

```
storage_set(key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32) -> i32
```

写入键值对。

**参数**:
- `key_ptr`: 键数据指针
- `key_len`: 键长度
- `value_ptr`: 值数据指针
- `value_len`: 值长度

**返回**: 0=成功, 非0=失败

**WAT 示例**:
```wat
(import "storage_api" "storage_set" (func $storage_set (param i32 i32 i32 i32) (result i32)))
```

#### `storage_delete`

```
storage_delete(key_ptr: i32, key_len: i32) -> i32
```

删除键。

**参数**:
- `key_ptr`: 键数据指针
- `key_len`: 键长度

**返回**: 0=成功, 非0=失败

**WAT 示例**:
```wat
(import "storage_api" "storage_delete" (func $storage_delete (param i32 i32) (result i32)))
```

---

### Chain API (`chain_api` 模块)

#### `block_number`

```
block_number() -> i64
```

获取当前区块号。

**返回**: 区块号(u64)

**WAT 示例**:
```wat
(import "chain_api" "block_number" (func $block_number (result i64)))
```

#### `timestamp`

```
timestamp() -> i64
```

获取当前区块时间戳(Unix 时间,秒)。

**返回**: 时间戳(u64)

**WAT 示例**:
```wat
(import "chain_api" "timestamp" (func $timestamp (result i64)))
```

#### `emit_event`

```
emit_event(data_ptr: i32, data_len: i32) -> i32
```

发送事件到宿主。

**参数**:
- `data_ptr`: 事件数据指针
- `data_len`: 事件数据长度

**返回**: 0=成功, 非0=失败

**WAT 示例**:
```wat
(import "chain_api" "emit_event" (func $emit_event (param i32 i32) (result i32)))

;; 发送 "Hello" 事件
(data (i32.const 0) "Hello")
(func $send_event
  i32.const 0
  i32.const 5
  call $emit_event
  drop
)
```

#### `events_len`

```
events_len() -> i32
```

获取当前已发送的事件总数。

**返回**: 事件数量

**WAT 示例**:
```wat
(import "chain_api" "events_len" (func $events_len (result i32)))
```

#### `read_event`

```
read_event(index: i32, ptr: i32, len: i32) -> i32
```

读取指定索引的事件数据。

**参数**:
- `index`: 事件索引(0-based)
- `ptr`: 目标内存地址
- `len`: 缓冲区大小

**返回**: 实际写入的字节数

**WAT 示例**:
```wat
(import "chain_api" "read_event" (func $read_event (param i32 i32 i32) (result i32)))
```

---

### Crypto API (`crypto_api` 模块)

用于在 WASM 中调用常见密码学原语。

#### `sha256`

```
sha256(data_ptr: i32, data_len: i32, output_ptr: i32) -> i32
```

计算 SHA-256 哈希,将 32 字节输出写入 `output_ptr`。

**返回**: 0=成功, 非0=失败

**WAT 示例**:
```wat
(import "crypto_api" "sha256" (func $sha256 (param i32 i32 i32) (result i32)))
;; (call $sha256 <in_ptr> <in_len> <out_ptr>)
```

#### `keccak256`

```
keccak256(data_ptr: i32, data_len: i32, output_ptr: i32) -> i32
```

计算 Keccak-256 哈希(以太坊),输出 32 字节。

**返回**: 0=成功, 非0=失败

**WAT 示例**:
```wat
(import "crypto_api" "keccak256" (func $keccak256 (param i32 i32 i32) (result i32)))
```

#### `verify_secp256k1`

```
verify_secp256k1(msg_ptr: i32, sig_ptr: i32, pubkey_ptr: i32, pubkey_len: i32) -> i32
```

验证 secp256k1 签名。

- `msg_ptr`: 消息哈希(32字节)
- `sig_ptr`: 签名(64字节, r||s)
- `pubkey_ptr`: 公钥(33或65字节)
- `pubkey_len`: 公钥长度

**返回**: 1=有效, 0=无效, -1=错误

**WAT 示例**:
```wat
(import "crypto_api" "verify_secp256k1" (func $verify_secp256k1 (param i32 i32 i32 i32) (result i32)))
```

#### `verify_ed25519`

```
verify_ed25519(msg_ptr: i32, msg_len: i32, sig_ptr: i32, pubkey_ptr: i32) -> i32
```

验证 Ed25519 签名。

- `msg_ptr`: 消息指针
- `msg_len`: 消息长度
- `sig_ptr`: 签名(64字节)
- `pubkey_ptr`: 公钥(32字节)

**返回**: 1=有效, 0=无效, -1=错误

**WAT 示例**:
```wat
(import "crypto_api" "verify_ed25519" (func $verify_ed25519 (param i32 i32 i32 i32) (result i32)))
```

#### `recover_secp256k1_pubkey`

```
recover_secp256k1_pubkey(msg_ptr: i32, sig_ptr: i32, output_ptr: i32) -> i32
```

从带 recovery id 的签名恢复 secp256k1 未压缩公钥(65字节)。

- `msg_ptr`: 消息哈希(32字节)
- `sig_ptr`: 签名(65字节, r||s||v)
- `output_ptr`: 输出缓冲区(65字节)

**返回**: 0=成功, -1=失败

**WAT 示例**:
```wat
(import "crypto_api" "recover_secp256k1_pubkey" (func $recover (param i32 i32 i32) (result i32)))
```

#### `derive_eth_address`

```
derive_eth_address(pubkey_ptr: i32, pubkey_len: i32, output_ptr: i32) -> i32
```

从 secp256k1 公钥派生以太坊地址(20字节)。

支持压缩(33字节)或未压缩(65字节)公钥格式。

算法: `keccak256(uncompressed_pubkey[1..])[12..]`

- `pubkey_ptr`: 公钥指针
- `pubkey_len`: 公钥长度(33或65)
- `output_ptr`: 输出缓冲区(20字节)

**返回**: 0=成功, -1=失败

**WAT 示例**:
```wat
(import "crypto_api" "derive_eth_address" (func $derive (param i32 i32 i32) (result i32)))
```

---

## 错误处理

所有 API 使用 `anyhow::Result` 返回值。

常见错误类型:
- **模块加载失败**: WASM 字节码无效
- **函数未找到**: 导出函数不存在
- **内存访问错误**: 越界或未导出 memory
- **存储错误**: 底层存储操作失败
- **类型不匹配**: 函数签名不符

**示例**:
```rust
match runtime.execute_with_context(&wasm, "main", 1, 1000) {
    Ok((result, events, _, _)) => {
        println!("Success: {}, events: {}", result, events.len());
    }
    Err(e) => {
        eprintln!("Execution failed: {}", e);
    }
}
```

---

## 完整示例

### 存储与事件示例

```rust
use vm_runtime::{Runtime, MemoryStorage};
use anyhow::Result;

fn main() -> Result<()> {
    let runtime = Runtime::new(MemoryStorage::new());
    
    let wat = r#"
    (module
      (import "storage_api" "storage_set" 
        (func $storage_set (param i32 i32 i32 i32) (result i32)))
      (import "chain_api" "emit_event" 
        (func $emit_event (param i32 i32) (result i32)))
      (import "chain_api" "block_number" 
        (func $block_number (result i64)))
      
      (memory (export "memory") 1)
      (data (i32.const 0) "user_id")
      (data (i32.const 10) "alice")
      (data (i32.const 20) "UserRegistered")
      
      (func (export "register") (result i32)
        ;; 写入存储
        i32.const 0
        i32.const 7
        i32.const 10
        i32.const 5
        call $storage_set
        drop
        
        ;; 发送事件
        i32.const 20
        i32.const 14
        call $emit_event
        drop
        
        ;; 返回区块号
        call $block_number
        i32.wrap_i64
      )
    )
    "#;
    
    let wasm = wat::parse_str(wat)?;
    let (result, events, block_num, ts) = runtime.execute_with_context(
        &wasm,
        "register",
        100,
        1704067200,
    )?;
    
    println!("Block: {}, Result: {}", block_num, result);
    println!("Events: {:?}", 
        events.iter()
            .map(|e| String::from_utf8_lossy(e))
            .collect::<Vec<_>>()
    );
    
    // 验证存储
    let value = runtime.storage()
        .borrow()
        .get(b"user_id")?
        .unwrap();
    assert_eq!(value, b"alice");
    
    Ok(())
}
```

---

## 性能考虑

1. **内存复制**: Host 函数通过指针访问 WASM 内存,避免不必要的复制
2. **存储缓存**: `storage_get` 缓存结果到 `last_get`,减少重复查询
3. **事件收集**: 使用 `Vec` 追加,amortized O(1) 复杂度
4. **JIT 编译**: wasmtime 提供 JIT 编译优化

## 线程安全

- `Runtime` 本身不是 `Send` 或 `Sync`
- 如需多线程,为每个线程创建独立的 `Runtime` 实例
- 存储后端可以是线程安全的(如使用 `Arc<Mutex<Storage>>`)

## 版本兼容性

- v0.1.0: 初始 API,可能有破坏性变更
- v1.0.0 前: 不保证向后兼容

---

*生成于 vm-runtime v0.1.0*
