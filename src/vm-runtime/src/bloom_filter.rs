// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 布隆过滤器实现
//! 
//! 用于快速检测交易读写集冲突,减少 MVCC 验证开销。
//! 
//! # 设计目标
//! - **误报率**: < 1% (可配置)
//! - **内存占用**: 最小化 (位图存储)
//! - **性能**: 纳秒级查询/插入
//! - **并发安全**: 支持多线程读写
//! 
//! # 使用场景
//! 1. **冲突快速排除**: 如果 Bloom Filter 显示无冲突,则跳过昂贵的精确检查
//! 2. **读写集缓存**: 缓存交易的读写键,加速后续冲突检测
//! 3. **批量提交优化**: 快速筛选可并行提交的交易

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

/// 布隆过滤器
/// 
/// 使用多个哈希函数实现概率性集合成员测试
#[derive(Debug)]
pub struct BloomFilter {
    /// 位数组
    bits: RwLock<Vec<u64>>,
    /// 位数组大小 (bits)
    size: usize,
    /// 哈希函数数量
    hash_count: usize,
    /// 已插入元素数量
    item_count: RwLock<usize>,
}

impl BloomFilter {
    /// 创建新的布隆过滤器
    /// 
    /// # 参数
    /// - `expected_items`: 预期元素数量
    /// - `false_positive_rate`: 期望的误报率 (0.0 - 1.0)
    /// 
    /// # 返回
    /// 新的布隆过滤器实例
    /// 
    /// # 示例
    /// ```
    /// use vm_runtime::bloom_filter::BloomFilter;
    /// 
    /// // 预期 10000 个元素,误报率 1%
    /// let filter = BloomFilter::new(10000, 0.01);
    /// ```
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        // 计算最优位数组大小: m = -n*ln(p) / (ln(2)^2)
        let size = Self::optimal_size(expected_items, false_positive_rate);
        
        // 计算最优哈希函数数量: k = (m/n) * ln(2)
        let hash_count = Self::optimal_hash_count(size, expected_items);
        
        // 初始化位数组 (使用 u64 存储,每个 u64 有 64 位)
        let words = (size + 63) / 64;
        
        Self {
            bits: RwLock::new(vec![0u64; words]),
            size,
            hash_count,
            item_count: RwLock::new(0),
        }
    }
    
    /// 创建具有指定大小和哈希数量的布隆过滤器
    /// 
    /// # 参数
    /// - `size`: 位数组大小 (bits)
    /// - `hash_count`: 哈希函数数量
    pub fn with_size(size: usize, hash_count: usize) -> Self {
        let words = (size + 63) / 64;
        
        Self {
            bits: RwLock::new(vec![0u64; words]),
            size,
            hash_count,
            item_count: RwLock::new(0),
        }
    }
    
    /// 插入元素
    /// 
    /// # 参数
    /// - `item`: 要插入的元素
    /// 
    /// # 示例
    /// ```
    /// # use vm_runtime::bloom_filter::BloomFilter;
    /// let filter = BloomFilter::new(1000, 0.01);
    /// filter.insert(&"key1");
    /// filter.insert(&"key2");
    /// ```
    pub fn insert<T: Hash>(&self, item: &T) {
        let hashes = self.hash(item);
        let mut bits = self.bits.write().unwrap();
        
        for hash in hashes {
            let bit_index = (hash as usize) % self.size;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;
            
            bits[word_index] |= 1u64 << bit_offset;
        }
        
        *self.item_count.write().unwrap() += 1;
    }
    
    /// 检查元素是否可能存在
    /// 
    /// # 参数
    /// - `item`: 要检查的元素
    /// 
    /// # 返回
    /// - `true`: 元素**可能**存在 (有误报可能)
    /// - `false`: 元素**一定不**存在 (100% 准确)
    /// 
    /// # 示例
    /// ```
    /// # use vm_runtime::bloom_filter::BloomFilter;
    /// let filter = BloomFilter::new(1000, 0.01);
    /// filter.insert(&"key1");
    /// 
    /// assert!(filter.contains(&"key1"));  // 一定返回 true
    /// assert!(!filter.contains(&"key2")); // 大概率返回 false
    /// ```
    pub fn contains<T: Hash>(&self, item: &T) -> bool {
        let hashes = self.hash(item);
        let bits = self.bits.read().unwrap();
        
        for hash in hashes {
            let bit_index = (hash as usize) % self.size;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;
            
            if (bits[word_index] & (1u64 << bit_offset)) == 0 {
                return false;
            }
        }
        
        true
    }
    
    /// 清空过滤器
    pub fn clear(&self) {
        let mut bits = self.bits.write().unwrap();
        bits.fill(0);
        *self.item_count.write().unwrap() = 0;
    }
    
    /// 获取已插入元素数量
    pub fn len(&self) -> usize {
        *self.item_count.read().unwrap()
    }
    
    /// 检查过滤器是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// 获取过滤器容量 (位数)
    pub fn capacity(&self) -> usize {
        self.size
    }
    
    /// 获取哈希函数数量
    pub fn hash_count(&self) -> usize {
        self.hash_count
    }
    
    /// 估算当前误报率
    /// 
    /// 基于公式: p ≈ (1 - e^(-kn/m))^k
    /// 其中 k=哈希数,n=元素数,m=位数组大小
    pub fn estimated_false_positive_rate(&self) -> f64 {
        let n = self.len() as f64;
        let m = self.size as f64;
        let k = self.hash_count as f64;
        
        if n == 0.0 {
            return 0.0;
        }
        
        let exp = (-k * n / m).exp();
        (1.0 - exp).powf(k)
    }
    
    // ==================== 内部辅助方法 ====================
    
    /// 计算最优位数组大小
    fn optimal_size(n: usize, p: f64) -> usize {
        let n = n as f64;
        let ln2_squared = std::f64::consts::LN_2 * std::f64::consts::LN_2;
        let m = -(n * p.ln()) / ln2_squared;
        m.ceil() as usize
    }
    
    /// 计算最优哈希函数数量
    fn optimal_hash_count(m: usize, n: usize) -> usize {
        let m = m as f64;
        let n = n as f64;
        let k = (m / n) * std::f64::consts::LN_2;
        k.ceil().max(1.0) as usize
    }
    
    /// 计算多个哈希值
    /// 
    /// 使用双哈希技术: h_i(x) = h1(x) + i * h2(x)
    /// 这样只需要两次哈希计算就能生成 k 个独立哈希值
    fn hash<T: Hash>(&self, item: &T) -> Vec<u64> {
        // 第一个哈希: DefaultHasher
        let mut hasher1 = DefaultHasher::new();
        item.hash(&mut hasher1);
        let h1 = hasher1.finish();
        
        // 第二个哈希: 使用不同的种子
        let mut hasher2 = DefaultHasher::new();
        hasher2.write_u64(h1);
        item.hash(&mut hasher2);
        let h2 = hasher2.finish();
        
        // 生成 k 个哈希值
        (0..self.hash_count)
            .map(|i| h1.wrapping_add((i as u64).wrapping_mul(h2)))
            .collect()
    }

    /// 近似集合相交判断：若任意位同时为 1，则认为“可能相交”；若无重叠位，则“必定不相交”。
    /// 注意：这是近似判断（可能误判相交），但不会漏判（无重叠位一定不相交）。
    pub fn maybe_intersect(&self, other: &Self) -> bool {
        // 若位数组长度不同，按最小长度对齐；不同大小下的位数组无法做精确比较，保守返回 true
        if self.capacity() != other.capacity() || self.hash_count() != other.hash_count() {
            // 尺寸或哈希函数数不同，无法做位级 AND 的可靠比较
            return true;
        }

        let a = self.bits.read().unwrap();
        let b = other.bits.read().unwrap();
        let len = a.len().min(b.len());
        for i in 0..len {
            if (a[i] & b[i]) != 0 {
                return true;
            }
        }
        false
    }
}

/// 线程安全的布隆过滤器集合
/// 
/// 为每个事务维护独立的布隆过滤器
#[derive(Debug)]
pub struct BloomFilterCache {
    /// 读集过滤器 (键 -> 事务ID列表)
    read_filters: RwLock<Vec<BloomFilter>>,
    /// 写集过滤器 (键 -> 事务ID列表)
    write_filters: RwLock<Vec<BloomFilter>>,
    /// 每个过滤器的配置
    expected_items: usize,
    false_positive_rate: f64,
}

impl BloomFilterCache {
    /// 创建新的过滤器缓存
    /// 
    /// # 参数
    /// - `expected_txns`: 预期交易数量
    /// - `expected_items_per_txn`: 每个交易的预期读写键数量
    /// - `false_positive_rate`: 期望的误报率
    pub fn new(
        expected_txns: usize,
        expected_items_per_txn: usize,
        false_positive_rate: f64,
    ) -> Self {
        Self {
            read_filters: RwLock::new(Vec::with_capacity(expected_txns)),
            write_filters: RwLock::new(Vec::with_capacity(expected_txns)),
            expected_items: expected_items_per_txn,
            false_positive_rate,
        }
    }
    
    /// 为新事务创建过滤器
    /// 
    /// # 返回
    /// 事务索引 (用于后续操作)
    pub fn allocate_txn(&self) -> usize {
        let mut read_filters = self.read_filters.write().unwrap();
        let mut write_filters = self.write_filters.write().unwrap();
        
        let txn_index = read_filters.len();
        
        read_filters.push(BloomFilter::new(
            self.expected_items,
            self.false_positive_rate,
        ));
        write_filters.push(BloomFilter::new(
            self.expected_items,
            self.false_positive_rate,
        ));
        
        txn_index
    }
    
    /// 记录读操作
    pub fn record_read(&self, txn_index: usize, key: &[u8]) {
        let filters = self.read_filters.read().unwrap();
        if let Some(filter) = filters.get(txn_index) {
            filter.insert(&key);
        }
    }
    
    /// 记录写操作
    pub fn record_write(&self, txn_index: usize, key: &[u8]) {
        let filters = self.write_filters.read().unwrap();
        if let Some(filter) = filters.get(txn_index) {
            filter.insert(&key);
        }
    }
    
    /// 快速检查两个事务是否可能冲突
    /// 
    /// # 返回
    /// - `true`: 可能冲突 (需要精确检查)
    /// - `false`: 一定不冲突 (可跳过精确检查)
    pub fn may_conflict(&self, txn1: usize, txn2: usize) -> bool {
        let read_filters = self.read_filters.read().unwrap();
        let write_filters = self.write_filters.read().unwrap();

        if txn1 >= read_filters.len() || txn2 >= write_filters.len() {
            return false;
        }

        let r1 = &read_filters[txn1];
        let w1 = &write_filters[txn1];
        let r2 = &read_filters[txn2];
        let w2 = &write_filters[txn2];

        // 三种可能冲突：R1∩W2 ≠ ∅ 或 W1∩R2 ≠ ∅ 或 W1∩W2 ≠ ∅
        r1.maybe_intersect(w2) || w1.maybe_intersect(r2) || w1.maybe_intersect(w2)
    }
    
    /// 清空所有过滤器
    pub fn clear(&self) {
        self.read_filters.write().unwrap().clear();
        self.write_filters.write().unwrap().clear();
    }
    
    /// 获取统计信息
    pub fn stats(&self) -> BloomFilterCacheStats {
        let read_filters = self.read_filters.read().unwrap();
        let write_filters = self.write_filters.read().unwrap();
        
        let total_txns = read_filters.len();
        let total_reads: usize = read_filters.iter().map(|f| f.len()).sum();
        let total_writes: usize = write_filters.iter().map(|f| f.len()).sum();
        
        let avg_fpr_read: f64 = if !read_filters.is_empty() {
            read_filters.iter()
                .map(|f| f.estimated_false_positive_rate())
                .sum::<f64>() / read_filters.len() as f64
        } else {
            0.0
        };
        
        let avg_fpr_write: f64 = if !write_filters.is_empty() {
            write_filters.iter()
                .map(|f| f.estimated_false_positive_rate())
                .sum::<f64>() / write_filters.len() as f64
        } else {
            0.0
        };
        
        BloomFilterCacheStats {
            total_txns,
            total_reads,
            total_writes,
            avg_false_positive_rate_read: avg_fpr_read,
            avg_false_positive_rate_write: avg_fpr_write,
        }
    }
}

/// 布隆过滤器缓存统计信息
#[derive(Debug, Clone)]
pub struct BloomFilterCacheStats {
    pub total_txns: usize,
    pub total_reads: usize,
    pub total_writes: usize,
    pub avg_false_positive_rate_read: f64,
    pub avg_false_positive_rate_write: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bloom_filter_basic() {
        let filter = BloomFilter::new(1000, 0.01);
        
        // 插入一些元素
        filter.insert(&"key1");
        filter.insert(&"key2");
        filter.insert(&"key3");
        
        // 检查存在的元素
        assert!(filter.contains(&"key1"));
        assert!(filter.contains(&"key2"));
        assert!(filter.contains(&"key3"));
        
        // 检查不存在的元素
        assert!(!filter.contains(&"key4"));
        assert!(!filter.contains(&"key5"));
    }
    
    #[test]
    fn test_bloom_filter_false_positive() {
        let filter = BloomFilter::new(100, 0.01);
        
        // 插入 100 个元素
        for i in 0..100 {
            filter.insert(&format!("key{}", i));
        }
        
        // 检查误报率
        let mut false_positives = 0;
        let test_count = 1000;
        
        for i in 100..100 + test_count {
            if filter.contains(&format!("key{}", i)) {
                false_positives += 1;
            }
        }
        
        let actual_fpr = false_positives as f64 / test_count as f64;
        println!("False positive rate: {:.4}", actual_fpr);
        
        // 实际误报率应该接近配置的 1%
        assert!(actual_fpr < 0.05); // 允许一些偏差
    }
    
    #[test]
    fn test_bloom_filter_cache() {
        let cache = BloomFilterCache::new(10, 100, 0.01);
        
        // 分配两个事务
        let txn1 = cache.allocate_txn();
        let txn2 = cache.allocate_txn();
        
        // 记录读写操作
        cache.record_read(txn1, b"key1");
        cache.record_write(txn2, b"key1");
        
        // 检查冲突
        assert!(cache.may_conflict(txn1, txn2));
        
        // 获取统计信息
        let stats = cache.stats();
        assert_eq!(stats.total_txns, 2);
        assert_eq!(stats.total_reads, 1);
        assert_eq!(stats.total_writes, 1);
    }
    
    #[test]
    fn test_bloom_filter_clear() {
        let filter = BloomFilter::new(100, 0.01);
        
        filter.insert(&"key1");
        filter.insert(&"key2");
        
        assert_eq!(filter.len(), 2);
        assert!(filter.contains(&"key1"));
        
        filter.clear();
        
        assert_eq!(filter.len(), 0);
        assert!(!filter.contains(&"key1"));
    }
}
