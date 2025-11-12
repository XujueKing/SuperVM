// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

// 开发者：king
// Developer: king

//! 存储接口与默认实现
//! 
//! Phase 4.3: 支持多种存储后端
//! - MemoryStorage: 内存存储 (测试用)
//! - RocksDBStorage: 持久化存储 (生产用)

use anyhow::Result;

// Phase 4.3: RocksDB 持久化存储
#[cfg(feature = "rocksdb-storage")]
pub mod rocksdb_storage;

#[cfg(feature = "rocksdb-storage")]
pub use rocksdb_storage::{RocksDBStorage, RocksDBConfig, AdaptiveBatchConfig, AdaptiveBatchResult, RocksDBMetrics};

/// 存储接口，定义了虚拟机可以使用的存储操作
pub trait Storage {
    /// 根据键获取值
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// 设置键值对
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()>;

    /// 删除键值对
    fn delete(&mut self, key: &[u8]) -> Result<()>;

    /// 根据前缀扫描键值对
    fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    /// 可选批量写入支持：返回 true 表示底层已原子批量处理，false 表示不支持需调用者逐条写入
    fn write_batch_if_supported(&mut self, _batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<bool> { Ok(false) }
}

/// 内存存储实现，用于测试
#[derive(Default)]
pub struct MemoryStorage {
    data: std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemoryStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.data.get(key).cloned())
    }

    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.data.remove(key);
        Ok(())
    }

    fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self.data
            .range(prefix.to_vec()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }

    fn write_batch_if_supported(&mut self, batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<bool> {
        // MemoryStorage 直接逐条应用
        for (k,v) in batch.into_iter() {
            match v {
                Some(val) => { self.data.insert(k, val); },
                None => { self.data.remove(&k); },
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() -> Result<()> {
        let mut storage = MemoryStorage::new();
        
        // 测试 set/get
        storage.set(b"key1", b"value1")?;
        assert_eq!(storage.get(b"key1")?.unwrap(), b"value1");
        
        // 测试 delete
        storage.delete(b"key1")?;
        assert_eq!(storage.get(b"key1")?, None);
        
        // 测试 scan
        storage.set(b"prefix1_a", b"v1")?;
        storage.set(b"prefix1_b", b"v2")?;
        storage.set(b"prefix2_a", b"v3")?;
        
        let results = storage.scan(b"prefix1_")?;
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|(k, v)| k == b"prefix1_a" && v == b"v1"));
        assert!(results.iter().any(|(k, v)| k == b"prefix1_b" && v == b"v2"));
        
        Ok(())
    }
}