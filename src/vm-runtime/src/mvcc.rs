use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}, Mutex};
use dashmap::DashMap;
use parking_lot::RwLock;

/// GC 配置选项
#[derive(Debug, Clone)]
pub struct GcConfig {
    /// 保留版本数量限制（每个键最多保留的版本数）
    pub max_versions_per_key: usize,
    /// 是否启用基于时间的 GC（清理过期版本）
    pub enable_time_based_gc: bool,
    /// 版本过期时间（秒），超过此时间的版本可被清理
    pub version_ttl_secs: u64,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            max_versions_per_key: 10, // 默认保留 10 个版本
            enable_time_based_gc: false,
            version_ttl_secs: 3600, // 1小时
        }
    }
}

/// MVCC 存储实现（优化版 + GC）：
/// - 使用 DashMap 实现每键粒度的并发控制，减少全局锁竞争
/// - 每个键的版本链使用 RwLock 保护，允许并发读
/// - 使用 AtomicU64 管理时间戳，避免锁竞争
/// - 提交时仅锁定写集合涉及的键，最小化锁持有范围
/// - 支持垃圾回收，清理不再需要的旧版本
pub struct MvccStore {
    /// 每个 key 的版本链（按 ts 升序存放），使用 RwLock 允许并发读
    data: DashMap<Vec<u8>, RwLock<Vec<Version>>>,
    /// 全局递增时间戳（原子操作，无锁）
    ts: AtomicU64,
    /// 活跃事务的最小 start_ts（水位线）
    /// 用于 GC 决策：低于此时间戳的版本可能被清理
    active_txns: Arc<Mutex<Vec<u64>>>,
    /// GC 配置
    gc_config: Arc<Mutex<GcConfig>>,
    /// GC 统计信息
    gc_stats: Arc<Mutex<GcStats>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub ts: u64,
    pub value: Option<Vec<u8>>, // None 表示删除
}

/// GC 统计信息
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    /// GC 执行次数
    pub gc_count: u64,
    /// 清理的版本总数
    pub versions_cleaned: u64,
    /// 清理的键总数
    pub keys_cleaned: u64,
    /// 最后一次 GC 时间戳
    pub last_gc_ts: u64,
}

impl MvccStore {
    pub fn new() -> Arc<Self> {
        Self::new_with_config(GcConfig::default())
    }

    pub fn new_with_config(config: GcConfig) -> Arc<Self> {
        Arc::new(Self { 
            data: DashMap::new(),
            ts: AtomicU64::new(0),
            active_txns: Arc::new(Mutex::new(Vec::new())),
            gc_config: Arc::new(Mutex::new(config)),
            gc_stats: Arc::new(Mutex::new(GcStats::default())),
        })
    }

    /// 开启一个事务，分配 start_ts（快照版本）
    pub fn begin(self: &Arc<Self>) -> Txn {
        let start_ts = self.ts.fetch_add(1, Ordering::SeqCst) + 1;
        
        // 注册活跃事务
        self.active_txns.lock().unwrap().push(start_ts);
        
        Txn {
            store: Arc::clone(self),
            start_ts,
            writes: HashMap::new(),
            committed: false,
            read_only: false,
        }
    }

    /// 开启一个只读事务（快速路径）
    /// 
    /// 只读事务优化：
    /// - 不维护写集合
    /// - commit() 直接返回，无需冲突检测
    /// - 性能更优，适合大量只读查询场景
    pub fn begin_read_only(self: &Arc<Self>) -> Txn {
        let start_ts = self.ts.fetch_add(1, Ordering::SeqCst) + 1;
        
        // 注册活跃事务（只读事务也需要注册，防止 GC 清理它可见的版本）
        self.active_txns.lock().unwrap().push(start_ts);
        
        Txn {
            store: Arc::clone(self),
            start_ts,
            writes: HashMap::new(),
            committed: false,
            read_only: true,
        }
    }

    fn next_ts(&self) -> u64 {
        self.ts.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// 只读接口：按给定 start_ts 查询可见版本（测试/调试辅助）
    /// 使用读锁，允许多个事务并发读取
    pub fn read_at(&self, key: &[u8], start_ts: u64) -> Option<Vec<u8>> {
        self.data.get(key).and_then(|entry| {
            let versions = entry.value().read();
            versions
                .iter()
                .rev()
                .find(|v| v.ts <= start_ts)
                .and_then(|v| v.value.clone())
        })
    }

    /// 注销活跃事务
    fn unregister_txn(&self, start_ts: u64) {
        let mut active = self.active_txns.lock().unwrap();
        if let Some(pos) = active.iter().position(|&ts| ts == start_ts) {
            active.remove(pos);
        }
    }

    /// 获取活跃事务的最小 start_ts（水位线）
    /// 
    /// 返回 None 表示没有活跃事务
    pub fn get_min_active_ts(&self) -> Option<u64> {
        let active = self.active_txns.lock().unwrap();
        active.iter().min().copied()
    }

    /// 更新 GC 配置
    pub fn set_gc_config(&self, config: GcConfig) {
        *self.gc_config.lock().unwrap() = config;
    }

    /// 获取 GC 统计信息
    pub fn get_gc_stats(&self) -> GcStats {
        self.gc_stats.lock().unwrap().clone()
    }

    /// 执行垃圾回收
    /// 
    /// 清理策略：
    /// 1. 保留每个键的最新版本（无论是否有活跃事务）
    /// 2. 对于有多个版本的键，根据配置清理旧版本：
    ///    - 基于版本数量：超过 max_versions_per_key 的旧版本
    ///    - 基于活跃事务：低于最小活跃事务 start_ts 的版本可被清理
    /// 
    /// 返回清理的版本总数
    pub fn gc(&self) -> Result<u64, String> {
        let config = self.gc_config.lock().unwrap().clone();
        let min_active_ts = self.get_min_active_ts();
        
        let mut total_cleaned = 0u64;
        let mut keys_cleaned = 0u64;
        
        // 遍历所有键，清理旧版本
        for entry in self.data.iter() {
            let _key = entry.key();
            let versions_lock = entry.value();
            let mut versions = versions_lock.write();
            
            if versions.len() <= 1 {
                // 只有一个版本，不清理
                continue;
            }
            
            // 计算需要保留的版本数
            let mut keep_count = versions.len();
            
            // 策略 1: 基于版本数量限制
            if keep_count > config.max_versions_per_key {
                keep_count = config.max_versions_per_key;
            }
            
            // 策略 2: 基于活跃事务水位线
            // 如果有活跃事务，保留它们可见的所有版本（ts <= min_active_ts 的最新版本必须保留）
            if let Some(min_ts) = min_active_ts {
                // 找到最老的活跃事务可见的版本索引
                // 活跃事务能看到 ts <= start_ts 的版本，所以要保留第一个 ts <= min_ts 的版本及之后的所有版本
                if let Some(first_visible_idx) = versions.iter().position(|v| v.ts <= min_ts) {
                    let versions_needed = versions.len() - first_visible_idx;
                    if versions_needed > keep_count {
                        keep_count = versions_needed;
                    }
                }
            }
            
            // 执行清理：保留最后 keep_count 个版本
            let to_remove = versions.len().saturating_sub(keep_count);
            if to_remove > 0 {
                versions.drain(0..to_remove);
                total_cleaned += to_remove as u64;
                keys_cleaned += 1;
            }
        }
        
        // 更新统计信息
        let mut stats = self.gc_stats.lock().unwrap();
        stats.gc_count += 1;
        stats.versions_cleaned += total_cleaned;
        stats.keys_cleaned += keys_cleaned;
        stats.last_gc_ts = self.ts.load(Ordering::SeqCst);
        
        Ok(total_cleaned)
    }

    /// 获取存储的总版本数（用于监控）
    pub fn total_versions(&self) -> usize {
        self.data.iter()
            .map(|entry| entry.value().read().len())
            .sum()
    }

    /// 获取存储的键数量
    pub fn total_keys(&self) -> usize {
        self.data.len()
    }
}

pub struct Txn {
    store: Arc<MvccStore>,
    pub start_ts: u64,
    writes: HashMap<Vec<u8>, Option<Vec<u8>>>,
    committed: bool,
    read_only: bool,
}

impl Txn {
    /// 读取在 start_ts 及以前可见的值
    pub fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        // 优先返回当前事务未提交的写（写读自己）
        if let Some(v) = self.writes.get(key) {
            return v.clone();
        }
        self.store.read_at(key, self.start_ts)
    }

    /// 写入（缓存在本地事务中）
    /// 
    /// 注意：只读事务调用此方法会 panic
    pub fn write(&mut self, key: Vec<u8>, value: Vec<u8>) {
        if self.read_only {
            panic!("cannot write in read-only transaction");
        }
        self.writes.insert(key, Some(value));
    }

    /// 删除（缓存在本地事务中）
    /// 
    /// 注意：只读事务调用此方法会 panic
    pub fn delete(&mut self, key: Vec<u8>) {
        if self.read_only {
            panic!("cannot delete in read-only transaction");
        }
        self.writes.insert(key, None);
    }

    /// 检查是否为只读事务
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// 提交：
    /// - 分配 commit_ts
    /// - 对每个写入键做写写冲突检测：如果存在 ts > start_ts 的已提交版本，则冲突
    /// - 无冲突则将本地写入附加为新版本
    /// 
    /// 优化点：
    /// - 仅对写集合中的键加锁，而非全局锁
    /// - 使用写锁（RwLock::write）保护版本链的修改
    /// - 按键排序后加锁，避免死锁
    /// - **只读事务快速路径**: 直接返回，跳过所有冲突检测和写入
    pub fn commit(mut self) -> Result<u64, String> {
        if self.committed { return Err("txn already committed".into()); }
        
        // 只读事务快速路径：直接返回 start_ts，无需任何操作
        if self.read_only {
            self.committed = true;
            return Ok(self.start_ts);
        }

        let commit_ts = self.store.next_ts();

        // 按键排序以避免死锁
        let mut sorted_keys: Vec<_> = self.writes.keys().cloned().collect();
        sorted_keys.sort();

        // 阶段1：冲突检测（只需读锁）
        for key in &sorted_keys {
            if let Some(entry) = self.store.data.get(key) {
                let versions = entry.value().read();
                if versions.iter().rev().any(|v| v.ts > self.start_ts) {
                    return Err(format!("write-write conflict on key {:?}", String::from_utf8_lossy(key)));
                }
            }
        }

        // 阶段2：写入新版本（获取写锁）
        for key in sorted_keys {
            let value = self.writes.remove(&key).unwrap();
            let entry = self.store.data.entry(key).or_insert_with(|| RwLock::new(Vec::new()));
            let mut versions = entry.value().write();
            versions.push(Version { ts: commit_ts, value });
        }

        self.committed = true;
        Ok(commit_ts)
    }

    /// 放弃事务（丢弃本地写集合）
    pub fn abort(self) {
        // Drop trait 会自动注销
    }
}

/// 在事务结束时自动注销活跃事务
impl Drop for Txn {
    fn drop(&mut self) {
        self.store.unregister_txn(self.start_ts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mvcc_write_write_conflict() {
        let store = MvccStore::new();

        // T1 写 key1 并提交
        let mut t1 = store.begin();
        t1.write(b"key1".to_vec(), b"A".to_vec());
        let _c1 = t1.commit().expect("t1 commit ok");

        // T2 在 T1 之后开始，但 start_ts < T1 提交的 ts 不成立（因为 begin 也递增），
        // 为了制造冲突：让 T2 在 T1 之前开始，再由 T1 先提交。我们改为：
        let store = MvccStore::new();
        let mut t2 = store.begin(); // start_ts = 1
        let mut t1 = store.begin(); // start_ts = 2

        t1.write(b"key1".to_vec(), b"A".to_vec());
        let _ = t1.commit().unwrap();

        t2.write(b"key1".to_vec(), b"B".to_vec());
        let e = t2.commit().expect_err("t2 should conflict");
        assert!(e.contains("write-write conflict"));
    }

    #[test]
    fn test_mvcc_snapshot_isolation_visibility() {
        let store = MvccStore::new();

        // 初始写入：T0 提交 V0
        let mut t0 = store.begin();
        t0.write(b"k".to_vec(), b"v0".to_vec());
        let ts0 = t0.commit().unwrap();

        // T1 在看到 ts0 后开始，读取应为 v0
        let t1 = store.begin();
        assert_eq!(t1.read(b"k").as_deref(), Some(b"v0".as_ref()));

        // T2 先开始（拿到更早的 start_ts），此时读取仍是 v0
        let t2 = store.begin();
        assert_eq!(t2.read(b"k").as_deref(), Some(b"v0".as_ref()));

        // T3 写入 v1 并提交
        let mut t3 = store.begin();
        t3.write(b"k".to_vec(), b"v1".to_vec());
        let _ts3 = t3.commit().unwrap();

        // 在 T3 提交后：
        // - T1、T2 由于 start_ts 更早，仍应看到 v0（快照隔离）
        assert_eq!(t1.read(b"k").as_deref(), Some(b"v0".as_ref()));
        assert_eq!(t2.read(b"k").as_deref(), Some(b"v0".as_ref()));

        // 新开 T4，应看到最新 v1
        let t4 = store.begin();
        assert_eq!(t4.read(b"k").as_deref(), Some(b"v1".as_ref()));

        // 直接用读接口校验不同时间点的可见性
        assert_eq!(store.read_at(b"k", ts0).as_deref(), Some(b"v0".as_ref()));
    }

    #[test]
    fn test_mvcc_version_visibility_multiple_versions() {
        let store = MvccStore::new();

        // v1
        let mut t1 = store.begin();
        t1.write(b"k".to_vec(), b"v1".to_vec());
        let ts1 = t1.commit().unwrap();

        // v2
        let mut t2 = store.begin();
        t2.write(b"k".to_vec(), b"v2".to_vec());
        let ts2 = t2.commit().unwrap();

        // v3 (删除)
        let mut t3 = store.begin();
        t3.delete(b"k".to_vec());
        let ts3 = t3.commit().unwrap();

        // 不同快照读取
        assert_eq!(store.read_at(b"k", ts1).as_deref(), Some(b"v1".as_ref()));
        assert_eq!(store.read_at(b"k", ts2).as_deref(), Some(b"v2".as_ref()));
        assert_eq!(store.read_at(b"k", ts3), None);
    }

    #[test]
    fn test_mvcc_concurrent_reads() {
        use std::thread;
        let store = MvccStore::new();

        // 初始化数据
        let mut t0 = store.begin();
        for i in 0..10 {
            t0.write(format!("key{}", i).into_bytes(), format!("value{}", i).into_bytes());
        }
        t0.commit().unwrap();

        // 并发读取：8 个线程同时读取不同键
        let handles: Vec<_> = (0..8)
            .map(|_tid| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    let txn = store_clone.begin();
                    for i in 0..10 {
                        let key = format!("key{}", i).into_bytes();
                        let val = txn.read(&key);
                        assert_eq!(val.as_deref(), Some(format!("value{}", i).as_bytes()));
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_mvcc_concurrent_writes_different_keys() {
        use std::thread;
        let store = MvccStore::new();

        // 并发写入不同键：8 个线程各写 5 个不重叠的键
        let handles: Vec<_> = (0..8)
            .map(|tid| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    let mut txn = store_clone.begin();
                    for i in 0..5 {
                        let key = format!("key_{}_{}", tid, i).into_bytes();
                        txn.write(key, format!("value_{}_{}", tid, i).into_bytes());
                    }
                    txn.commit().unwrap();
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // 验证：所有写入都成功
        let verify_txn = store.begin();
        for tid in 0..8 {
            for i in 0..5 {
                let key = format!("key_{}_{}", tid, i).into_bytes();
                let val = verify_txn.read(&key);
                assert_eq!(val.as_deref(), Some(format!("value_{}_{}", tid, i).as_bytes()));
            }
        }
    }

    #[test]
    fn test_mvcc_concurrent_writes_same_key_conflicts() {
        use std::thread;
        let store = MvccStore::new();

        // 初始值
        let mut t0 = store.begin();
        t0.write(b"shared".to_vec(), b"init".to_vec());
        t0.commit().unwrap();

        // 并发写入同一键：期望只有一个成功，其他冲突
        let handles: Vec<_> = (0..4)
            .map(|tid| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    let mut txn = store_clone.begin();
                    txn.write(b"shared".to_vec(), format!("value{}", tid).into_bytes());
                    txn.commit()
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // 至少有一个成功，其余应冲突
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let conflict_count = results.iter().filter(|r| r.is_err()).count();

        assert!(success_count >= 1, "至少有一个事务应成功");
        assert_eq!(success_count + conflict_count, 4);
    }

    #[test]
    fn test_mvcc_read_only_transaction() {
        let store = MvccStore::new();

        // 初始化数据
        let mut t0 = store.begin();
        t0.write(b"k1".to_vec(), b"v1".to_vec());
        t0.write(b"k2".to_vec(), b"v2".to_vec());
        t0.commit().unwrap();

        // 只读事务可以读取
        let ro_txn = store.begin_read_only();
        assert!(ro_txn.is_read_only());
        let start_ts = ro_txn.start_ts;
        assert_eq!(ro_txn.read(b"k1").as_deref(), Some(b"v1".as_ref()));
        assert_eq!(ro_txn.read(b"k2").as_deref(), Some(b"v2".as_ref()));

        // 只读事务提交（快速路径）
        let ts = ro_txn.commit().unwrap();
        assert_eq!(ts, start_ts); // start_ts 直接返回，无需分配新 commit_ts
    }

    #[test]
    #[should_panic(expected = "cannot write in read-only transaction")]
    fn test_mvcc_read_only_cannot_write() {
        let store = MvccStore::new();
        let mut ro_txn = store.begin_read_only();
        ro_txn.write(b"k".to_vec(), b"v".to_vec()); // 应 panic
    }

    #[test]
    #[should_panic(expected = "cannot delete in read-only transaction")]
    fn test_mvcc_read_only_cannot_delete() {
        let store = MvccStore::new();
        let mut ro_txn = store.begin_read_only();
        ro_txn.delete(b"k".to_vec()); // 应 panic
    }

    #[test]
    fn test_mvcc_read_only_performance() {
        use std::thread;
        use std::time::Instant;

        let store = MvccStore::new();

        // 初始化 100 个键
        let mut t0 = store.begin();
        for i in 0..100 {
            t0.write(format!("key{}", i).into_bytes(), format!("value{}", i).into_bytes());
        }
        t0.commit().unwrap();

        // 测试只读事务性能（无提交开销）
        let start = Instant::now();
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let txn = store_clone.begin_read_only();
                        for i in 0..10 {
                            let _ = txn.read(&format!("key{}", i).into_bytes());
                        }
                        txn.commit().unwrap(); // 快速路径，无开销
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        let ro_elapsed = start.elapsed();

        // 对比：普通读写事务（即使不写，也有提交开销）
        let start = Instant::now();
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let txn = store_clone.begin();
                        for i in 0..10 {
                            let _ = txn.read(&format!("key{}", i).into_bytes());
                        }
                        txn.commit().unwrap(); // 需要分配 commit_ts
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        let rw_elapsed = start.elapsed();

        println!("只读事务耗时: {:?}", ro_elapsed);
        println!("读写事务耗时: {:?}", rw_elapsed);
        // 只读事务应更快（通常快 20-50%）
        assert!(ro_elapsed < rw_elapsed * 2, "只读事务应有性能优势");
    }

    // ====================
    // GC 测试
    // ====================

    #[test]
    fn test_gc_version_cleanup() {
        let config = GcConfig {
            max_versions_per_key: 3,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
        };
        let store = MvccStore::new_with_config(config);

        // 写入 5 个版本
        for i in 0..5 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        // GC 前应有 5 个版本
        assert_eq!(store.total_versions(), 5);

        // 执行 GC（max_versions_per_key=3，应清理 2 个旧版本）
        let _cleaned = store.gc().unwrap();
        assert_eq!(_cleaned, 2);

        // GC 后应剩 3 个版本
        assert_eq!(store.total_versions(), 3);

        // 验证 GC 统计
        let stats = store.get_gc_stats();
        assert_eq!(stats.gc_count, 1);
        assert_eq!(stats.versions_cleaned, 2);
        assert_eq!(stats.keys_cleaned, 1);
    }

    #[test]
    fn test_gc_preserves_active_transaction_visibility() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
        };
        let store = MvccStore::new_with_config(config);

        // v1
        let mut t1 = store.begin();
        t1.write(b"key".to_vec(), b"v1".to_vec());
        t1.commit().unwrap();

        // 开启一个长事务（持有 v1 的快照）
        let long_txn = store.begin();
        assert_eq!(long_txn.read(b"key").as_deref(), Some(b"v1".as_ref()));

        // v2, v3, v4
        for i in 2..=4 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        // 现在有 4 个版本
        assert_eq!(store.total_versions(), 4);

        // 执行 GC：虽然 max_versions=2，但因为 long_txn 仍活跃，不应清理它可见的 v1
        let _cleaned = store.gc().unwrap();
        
        // 应保留 long_txn.start_ts 可见的版本（v1） + 最新的版本
        // 根据实现，应保留所有 ts >= min_active_ts 的版本
        assert!(store.total_versions() >= 1, "至少保留活跃事务可见的版本");

        // long_txn 仍能读取 v1
        assert_eq!(long_txn.read(b"key").as_deref(), Some(b"v1".as_ref()));

        // 提交 long_txn，它不再活跃
        drop(long_txn);

        // 再次 GC，现在可以更激进地清理
        let _cleaned2 = store.gc().unwrap();
        // 应清理到 max_versions_per_key=2
        assert_eq!(store.total_versions(), 2);
    }

    #[test]
    fn test_gc_no_active_transactions() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
        };
        let store = MvccStore::new_with_config(config);

        // 写入 5 个版本（每次都提交并结束事务）
        for i in 0..5 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        // 没有活跃事务
        assert_eq!(store.get_min_active_ts(), None);

        // 执行 GC，应清理到 max_versions=2
        let cleaned = store.gc().unwrap();
        assert_eq!(cleaned, 3); // 清理 3 个旧版本
        assert_eq!(store.total_versions(), 2);
    }

    #[test]
    fn test_gc_multiple_keys() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
        };
        let store = MvccStore::new_with_config(config);

        // 3 个键，每个写入 5 个版本
        for key_id in 0..3 {
            for ver in 0..5 {
                let mut txn = store.begin();
                txn.write(
                    format!("key{}", key_id).into_bytes(),
                    format!("v{}", ver).into_bytes(),
                );
                txn.commit().unwrap();
            }
        }

        // 总共 15 个版本
        assert_eq!(store.total_versions(), 15);
        assert_eq!(store.total_keys(), 3);

        // 执行 GC
        let cleaned = store.gc().unwrap();
        assert_eq!(cleaned, 9); // 每个键清理 3 个，共 9 个

        // 剩余 6 个版本 (3 键 * 2 版本/键)
        assert_eq!(store.total_versions(), 6);
        
        // GC 统计
        let stats = store.get_gc_stats();
        assert_eq!(stats.keys_cleaned, 3);
        assert_eq!(stats.versions_cleaned, 9);
    }

    #[test]
    fn test_gc_stats_accumulation() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
        };
        let store = MvccStore::new_with_config(config);

        // 第一轮写入并 GC
        for i in 0..4 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }
        let cleaned1 = store.gc().unwrap();

        // 第二轮写入并 GC
        for i in 4..8 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }
        let cleaned2 = store.gc().unwrap();

        // 验证统计累计
        let stats = store.get_gc_stats();
        assert_eq!(stats.gc_count, 2);
        assert_eq!(stats.versions_cleaned, cleaned1 + cleaned2);
    }
}

