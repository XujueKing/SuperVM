// 开发者：king
// Developer: king

//! WebAssembly host functions 实现

use crate::Storage;
use anyhow::{Result, anyhow};
use std::rc::Rc;
use std::cell::RefCell;
use wasmtime::{Caller, Memory};

/// 从 WASM 内存读取字节切片
pub fn read_memory<T>(mem: &Memory, caller: &Caller<'_, T>, ptr: i32, len: i32) -> Result<Vec<u8>> {
    if ptr < 0 || len < 0 {
        return Err(anyhow!("Invalid memory access: negative pointer or length"));
    }
    
    let ptr = ptr as usize;
    let len = len as usize;
    
    let data_size = mem.data_size(caller);
    if ptr + len > data_size {
        return Err(anyhow!("Memory access out of bounds"));
    }

    let data = mem.data(caller);
    Ok(data[ptr..ptr + len].to_vec())
}

/// 向 WASM 内存写入字节切片，返回写入的长度
pub fn write_memory<T>(mem: &Memory, caller: &mut Caller<'_, T>, ptr: i32, data: &[u8]) -> Result<i32> {
    if ptr < 0 {
        return Err(anyhow!("Invalid memory access: negative pointer"));
    }
    
    let ptr = ptr as usize;
    let data_size = mem.data_size(&*caller);
    if ptr + data.len() > data_size {
        return Err(anyhow!("Memory access out of bounds"));
    }
    
    let target = &mut mem.data_mut(caller)[ptr..ptr + data.len()];
    target.copy_from_slice(data);
    Ok(data.len() as i32)
}

/// 存储操作的运行时状态
pub struct HostState<S: Storage> {
    pub storage: Rc<RefCell<S>>,
    pub memory: Option<Memory>,
    pub last_get: Option<Vec<u8>>,
    // 存放由 guest 通过 emit_event 提交的事件（字节数组）
    pub events: Vec<Vec<u8>>,
    // 链上下文（示例）
    pub block_number: u64,
    pub timestamp: u64,
    // 读写集追踪 (用于并行执行)
    pub read_write_set: crate::parallel::ReadWriteSet,
}

/// 存储相关的 host functions
pub mod storage_api {
    use super::*;

    /// storage_get(key_ptr: i32, key_len: i32) -> i64
    /// 
    /// 返回值：
    /// - 0 表示 key 不存在
    /// - 正数表示值的长度，可通过 storage_read_value 读取
    /// - 负数表示错误码
    pub fn storage_get(
        mut caller: Caller<'_, HostState<impl Storage>>,
        key_ptr: i32,
        key_len: i32,
    ) -> Result<i64> {
        // clone the Memory handle to avoid holding an immutable borrow on caller
        let memory = caller.data().memory.clone()
            .ok_or_else(|| anyhow!("No memory exported"))?;

        // 读取 key
        let key = read_memory(&memory, &caller, key_ptr, key_len)?;

        // 追踪读操作
        caller.data_mut().read_write_set.add_read(key.clone());

        // 查询存储
        let storage_rc = caller.data().storage.clone();
        let storage_ref = storage_rc.borrow();
        match storage_ref.get(&key)? {
            Some(value) => {
                // 缓存结果以便后续读取
                caller.data_mut().last_get = Some(value.clone());
                Ok(value.len() as i64)
            }
            None => {
                caller.data_mut().last_get = None;
                Ok(0)
            }
        }
    }
    
    /// storage_read_value(value_ptr: i32, value_len: i32) -> i32
    /// 
    /// 读取上一次 storage_get 查询到的值
    pub fn storage_read_value(
        mut caller: Caller<'_, HostState<impl Storage>>,
        value_ptr: i32,
        value_len: i32,
    ) -> Result<i32> {
        let memory = caller.data().memory.clone()
            .ok_or_else(|| anyhow!("No memory exported"))?;

        let data = caller.data().last_get.clone().ok_or_else(|| anyhow!("No cached value"))?;
        let write_len = std::cmp::min(data.len(), value_len as usize);

        // 写入内存
        write_memory(&memory, &mut caller, value_ptr, &data[..write_len])?;
        Ok(write_len as i32)
    }

    /// storage_set(key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32) -> i32
    /// 
    /// 返回值：
    /// - 0 表示成功
    /// - 非 0 表示错误码
    pub fn storage_set(
        mut caller: Caller<'_, HostState<impl Storage>>,
        key_ptr: i32,
        key_len: i32,
        value_ptr: i32,
        value_len: i32,
    ) -> Result<i32> {
        // clone the Memory handle to avoid holding an immutable borrow while we later
        // mutably borrow `caller` to write into memory
        let memory = caller.data().memory.clone()
            .ok_or_else(|| anyhow!("No memory exported"))?;
            
    // 读取 key 和 value
    let key = read_memory(&memory, &caller, key_ptr, key_len)?;
    let value = read_memory(&memory, &caller, value_ptr, value_len)?;
        
        // 追踪写操作
        caller.data_mut().read_write_set.add_write(key.clone());
        
        // 写入存储
        caller.data_mut().storage.borrow_mut().set(&key, &value)?;
        Ok(0)
    }

    /// storage_delete(key_ptr: i32, key_len: i32) -> i32
    /// 
    /// 返回值：
    /// - 0 表示成功
    /// - 非 0 表示错误码
    pub fn storage_delete(
        mut caller: Caller<'_, HostState<impl Storage>>,
        key_ptr: i32,
        key_len: i32,
    ) -> Result<i32> {
        let memory = caller.data().memory.clone()
            .ok_or_else(|| anyhow!("No memory exported"))?;
            
        // 读取 key
        let key = read_memory(&memory, &caller, key_ptr, key_len)?;
        
        // 追踪写操作 (删除也是写)
        caller.data_mut().read_write_set.add_write(key.clone());
        
        // 从存储中删除
        caller.data_mut().storage.borrow_mut().delete(&key)?;
        Ok(0)
    }
}

/// 链上下文与事件相关的 host functions
pub mod chain_api {
    use super::*;

    /// block_number() -> i64
    pub fn block_number(
        caller: Caller<'_, HostState<impl Storage>>,
    ) -> Result<i64> {
        Ok(caller.data().block_number as i64)
    }

    /// timestamp() -> i64
    pub fn timestamp(
        caller: Caller<'_, HostState<impl Storage>>,
    ) -> Result<i64> {
        Ok(caller.data().timestamp as i64)
    }

    /// emit_event(ptr: i32, len: i32) -> i32
    ///
    /// 从 guest 内存读取事件字节并入队到 HostState.events
    pub fn emit_event(
        mut caller: Caller<'_, HostState<impl Storage>>,
        ptr: i32,
        len: i32,
    ) -> Result<i32> {
        let memory = caller.data().memory.clone()
            .ok_or_else(|| anyhow!("No memory exported"))?;

    let data = read_memory(&memory, &caller, ptr, len)?;
        caller.data_mut().events.push(data);
        Ok(0)
    }

    /// events_len() -> i32
    /// 返回当前事件队列长度
    pub fn events_len(
        caller: Caller<'_, HostState<impl Storage>>,
    ) -> Result<i32> {
        Ok(caller.data().events.len() as i32)
    }

    /// read_event(index: i32, ptr: i32, len: i32) -> i32
    /// 从宿主事件队列读取指定索引的事件，写入 guest 内存，返回写入的字节数
    pub fn read_event(
        mut caller: Caller<'_, HostState<impl Storage>>,
        index: i32,
        ptr: i32,
        len: i32,
    ) -> Result<i32> {
        // clone Memory to avoid holding an immutable borrow of caller across the
        // subsequent mutable borrow when writing into guest memory
        let memory = caller.data().memory.clone()
            .ok_or_else(|| anyhow!("No memory exported"))?;

        if index < 0 {
            return Err(anyhow!("Invalid event index"));
        }
        let idx = index as usize;
        let ev = caller.data().events.get(idx)
            .ok_or_else(|| anyhow!("Event index out of range"))?.clone();

    let write_len = std::cmp::min(ev.len(), len as usize);
    write_memory(&memory, &mut caller, ptr, &ev[..write_len])?;
        Ok(write_len as i32)
    }
}

    /// 密码学相关的 host functions
    pub mod crypto_api {
        use super::*;
        use crate::crypto;
    
        /// SHA-256 哈希
        /// 
        /// # 参数
        /// - data_ptr: 输入数据指针
        /// - data_len: 输入数据长度
        /// - output_ptr: 输出缓冲区指针 (32 字节)
        /// 
        /// # 返回
        /// 0 表示成功, 非 0 表示失败
        pub fn sha256<S: Storage>(
            mut caller: Caller<'_, HostState<S>>,
            data_ptr: i32,
            data_len: i32,
            output_ptr: i32,
        ) -> Result<i32> {
            let memory = caller.data().memory.clone()
                .ok_or_else(|| anyhow!("No memory exported"))?;
        
            // 读取输入数据
            let data = read_memory(&memory, &caller, data_ptr, data_len)?;
        
            // 计算哈希
            let hash = crypto::sha256(&data);
        
            // 写入结果
            write_memory(&memory, &mut caller, output_ptr, &hash)?;
            Ok(0)
        }
    
        /// Keccak-256 哈希 (以太坊)
        /// 
        /// # 参数
        /// - data_ptr: 输入数据指针
        /// - data_len: 输入数据长度
        /// - output_ptr: 输出缓冲区指针 (32 字节)
        /// 
        /// # 返回
        /// 0 表示成功, 非 0 表示失败
        pub fn keccak256<S: Storage>(
            mut caller: Caller<'_, HostState<S>>,
            data_ptr: i32,
            data_len: i32,
            output_ptr: i32,
        ) -> Result<i32> {
            let memory = caller.data().memory.clone()
                .ok_or_else(|| anyhow!("No memory exported"))?;
        
            // 读取输入数据
            let data = read_memory(&memory, &caller, data_ptr, data_len)?;
        
            // 计算哈希
            let hash = crypto::keccak256(&data);
        
            // 写入结果
            write_memory(&memory, &mut caller, output_ptr, &hash)?;
            Ok(0)
        }
    
        /// 验证 secp256k1 签名
        /// 
        /// # 参数
        /// - msg_ptr: 消息哈希指针 (32 字节)
        /// - sig_ptr: 签名指针 (64 字节)
        /// - pubkey_ptr: 公钥指针 (33 或 65 字节)
        /// - pubkey_len: 公钥长度
        /// 
        /// # 返回
        /// 1 表示验证成功, 0 表示失败, 负数表示错误
        pub fn verify_secp256k1<S: Storage>(
            caller: Caller<'_, HostState<S>>,
            msg_ptr: i32,
            sig_ptr: i32,
            pubkey_ptr: i32,
            pubkey_len: i32,
        ) -> Result<i32> {
            let memory = caller.data().memory.clone()
                .ok_or_else(|| anyhow!("No memory exported"))?;
        
            // 读取数据
            let message = read_memory(&memory, &caller, msg_ptr, 32)?;
            let signature = read_memory(&memory, &caller, sig_ptr, 64)?;
            let pubkey = read_memory(&memory, &caller, pubkey_ptr, pubkey_len)?;
        
            // 验证签名
            match crypto::verify_secp256k1(&message, &signature, &pubkey) {
                Ok(valid) => Ok(if valid { 1 } else { 0 }),
                Err(_) => Ok(-1),
            }
        }
    
        /// 验证 Ed25519 签名
        /// 
        /// # 参数
        /// - msg_ptr: 消息指针
        /// - msg_len: 消息长度
        /// - sig_ptr: 签名指针 (64 字节)
        /// - pubkey_ptr: 公钥指针 (32 字节)
        /// 
        /// # 返回
        /// 1 表示验证成功, 0 表示失败, 负数表示错误
        pub fn verify_ed25519<S: Storage>(
            caller: Caller<'_, HostState<S>>,
            msg_ptr: i32,
            msg_len: i32,
            sig_ptr: i32,
            pubkey_ptr: i32,
        ) -> Result<i32> {
            let memory = caller.data().memory.clone()
                .ok_or_else(|| anyhow!("No memory exported"))?;
        
            // 读取数据
            let message = read_memory(&memory, &caller, msg_ptr, msg_len)?;
            let signature = read_memory(&memory, &caller, sig_ptr, 64)?;
            let pubkey = read_memory(&memory, &caller, pubkey_ptr, 32)?;
        
            // 验证签名
            match crypto::verify_ed25519(&message, &signature, &pubkey) {
                Ok(valid) => Ok(if valid { 1 } else { 0 }),
                Err(_) => Ok(-1),
            }
        }
    
        /// 从 secp256k1 签名恢复公钥
        /// 
        /// # 参数
        /// - msg_ptr: 消息哈希指针 (32 字节)
        /// - sig_ptr: 签名指针 (65 字节,包含 recovery id)
        /// - output_ptr: 输出缓冲区指针 (65 字节)
        /// 
        /// # 返回
        /// 0 表示成功, 负数表示失败
        pub fn recover_secp256k1_pubkey<S: Storage>(
            mut caller: Caller<'_, HostState<S>>,
            msg_ptr: i32,
            sig_ptr: i32,
            output_ptr: i32,
        ) -> Result<i32> {
            let memory = caller.data().memory.clone()
                .ok_or_else(|| anyhow!("No memory exported"))?;
        
            // 读取数据
            let message = read_memory(&memory, &caller, msg_ptr, 32)?;
            let signature = read_memory(&memory, &caller, sig_ptr, 65)?;
        
            // 恢复公钥
            match crypto::recover_secp256k1_pubkey(&message, &signature) {
                Ok(pubkey) => {
                    write_memory(&memory, &mut caller, output_ptr, &pubkey)?;
                    Ok(0)
                },
                Err(_) => Ok(-1),
            }
        }

        /// 从公钥推导以太坊地址 (20 字节)
        ///
        /// # 参数
        /// - pubkey_ptr: 公钥指针 (33 或 65 字节)
        /// - pubkey_len: 公钥长度
        /// - output_ptr: 输出缓冲区指针 (20 字节)
        ///
        /// # 返回
        /// 0 表示成功, 负数表示失败
        pub fn derive_eth_address<S: Storage>(
            mut caller: Caller<'_, HostState<S>>,
            pubkey_ptr: i32,
            pubkey_len: i32,
            output_ptr: i32,
        ) -> Result<i32> {
            let memory = caller.data().memory.clone()
                .ok_or_else(|| anyhow!("No memory exported"))?;

            let pubkey = read_memory(&memory, &caller, pubkey_ptr, pubkey_len)?;
            match crypto::derive_eth_address(&pubkey) {
                Ok(addr) => {
                    write_memory(&memory, &mut caller, output_ptr, &addr)?;
                    Ok(0)
                }
                Err(_) => Ok(-1),
            }
        }
    }
