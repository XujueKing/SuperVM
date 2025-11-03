// 开发者：king
// Developer: king

//! 存储接口与默认实现

use anyhow::Result;

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