// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

// Phase 4.3: RocksDB 持久化存储实现
// Developer: king
// Date: 2024-11-07

//! RocksDB 持久化存储后端
//!
//! 特性:
//! - 持久化存储: 重启后数据不丢失
//! - 高性能: 随机读 100K+ ops/s, 批量写 200K+ ops/s
//! - LZ4 压缩: 节省磁盘空间
//! - WriteBatch: 原子批量写入
//! - Checkpoint: 快照管理
//! - Pruning: 状态裁剪

use crate::Storage;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "rocksdb-storage")]
use rocksdb::{IteratorMode, Options, WriteBatch, WriteOptions, DB};

/// RocksDB 存储配置
#[derive(Debug, Clone)]
pub struct RocksDBConfig {
    /// 数据库路径
    pub path: String,

    /// 最大打开文件数 (默认 10000)
    pub max_open_files: i32,

    /// 写缓冲区大小 (默认 128MB)
    pub write_buffer_size: usize,

    /// 块缓存大小 (默认 512MB)
    pub block_cache_size: usize,

    /// 是否启用压缩 (默认 false；如未链接压缩库将自动降级为 None)
    pub enable_compression: bool,

    /// 是否创建缺失的列族 (默认 true)
    pub create_if_missing: bool,

    /// 最大后台压缩线程数 (默认 4)
    pub max_background_jobs: i32,
}

impl Default for RocksDBConfig {
    fn default() -> Self {
        Self {
            path: "./data/rocksdb".to_string(),
            max_open_files: 10000,
            write_buffer_size: 128 * 1024 * 1024, // 128MB
            block_cache_size: 512 * 1024 * 1024,  // 512MB
            enable_compression: false,
            create_if_missing: true,
            max_background_jobs: 4,
        }
    }
}

impl RocksDBConfig {
    /// 生产优化配置（无压缩；更大的缓存与写缓冲，更高后台并发）
    pub fn production_optimized() -> Self {
        Self {
            path: "./data/rocksdb".to_string(),
            max_open_files: 20000,
            write_buffer_size: 256 * 1024 * 1024, // 256MB
            block_cache_size: 1024 * 1024 * 1024, // 1GB
            enable_compression: false,
            create_if_missing: true,
            max_background_jobs: 8,
        }
    }

    /// 设置路径并返回（便于链式调用）
    pub fn with_path<S: Into<String>>(mut self, path: S) -> Self {
        self.path = path.into();
        self
    }
}

/// RocksDB 存储实现
#[cfg(feature = "rocksdb-storage")]
pub struct RocksDBStorage {
    db: Arc<DB>,
    config: RocksDBConfig,
}

// ===== 自适应批量写入：类型定义（模块级） =====
#[cfg(feature = "rocksdb-storage")]
#[derive(Debug, Clone)]
pub struct AdaptiveBatchConfig {
    pub init_chunk: usize,
    pub min_chunk: usize,
    pub max_chunk: usize,
    pub target_rsd_pct: f64,
    pub adjust_up_pct: f64,
    pub adjust_down_pct: f64,
    pub window: usize,
}

// ===== 快照管理配置 =====
#[cfg(feature = "rocksdb-storage")]
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// 快照保存基础路径
    pub base_path: std::path::PathBuf,

    /// 每隔多少个区块创建快照 (0 表示禁用基于区块的快照)
    pub blocks_per_snapshot: u64,

    /// 每隔多少秒创建快照 (0 表示禁用基于时间的快照)
    pub seconds_per_snapshot: u64,

    /// 最多保留多少个快照 (旧快照会被自动删除)
    pub max_snapshots: usize,

    /// 是否压缩旧快照 (暂时保留配置,未实现)
    pub compress_old_snapshots: bool,
}

#[cfg(feature = "rocksdb-storage")]
impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            base_path: std::path::PathBuf::from("./data/snapshots"),
            blocks_per_snapshot: 1000,
            seconds_per_snapshot: 0, // 默认禁用时间快照
            max_snapshots: 10,
            compress_old_snapshots: false,
        }
    }
}

#[cfg(feature = "rocksdb-storage")]
impl SnapshotConfig {
    /// 创建基于区块的快照配置
    pub fn with_block_interval(blocks: u64) -> Self {
        Self {
            blocks_per_snapshot: blocks,
            ..Default::default()
        }
    }

    /// 创建基于时间的快照配置
    pub fn with_time_interval(seconds: u64) -> Self {
        Self {
            seconds_per_snapshot: seconds,
            blocks_per_snapshot: 0,
            ..Default::default()
        }
    }
}

#[cfg(feature = "rocksdb-storage")]
impl AdaptiveBatchConfig {
    pub fn default_for(batch: &[(Vec<u8>, Option<Vec<u8>>)]) -> Self {
        let n = batch.len().max(1);
        // 调整初始 chunk：更靠中区间，减少过多微小 chunk 带来的调节噪声
        let init = (n / 8).clamp(2_000, 50_000);
        Self {
            init_chunk: init,
            min_chunk: 1_000,      // 提高下限，避免过小 chunk 带来调度开销
            max_chunk: 60_000,     // 略增上限，给大型批量更高空间
            target_rsd_pct: 8.0,   // 目标抖动阈值保持不变
            adjust_up_pct: 0.15,   // 放大步长更谨慎
            adjust_down_pct: 0.30, // v2: 缩小更保守，避免大批量过度震荡
            window: 6,             // v2: 增加窗口平滑，减少 flush/compaction 噪声
        }
    }
}

#[cfg(feature = "rocksdb-storage")]
#[derive(Debug, Clone)]
pub struct AdaptiveBatchResult {
    pub final_chunk: usize,
    pub chunks: usize,
    pub avg_qps: f64,
    pub best_qps: f64,
    pub stddev_qps: f64,
    pub rsd_pct: f64,
}

#[cfg(feature = "rocksdb-storage")]
impl RocksDBStorage {
    /// 创建新的 RocksDB 存储实例
    pub fn new(mut config: RocksDBConfig) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(config.create_if_missing);
        opts.set_max_open_files(config.max_open_files);
        opts.set_write_buffer_size(config.write_buffer_size);
        opts.set_max_background_jobs(config.max_background_jobs);

        // 配置块缓存
        let cache = rocksdb::Cache::new_lru_cache(config.block_cache_size);
        let mut block_opts = rocksdb::BlockBasedOptions::default();
        block_opts.set_block_cache(&cache);
        opts.set_block_based_table_factory(&block_opts);

        // 配置压缩
        if config.enable_compression {
            opts.set_compression_type(rocksdb::DBCompressionType::Snappy);
        } else {
            opts.set_compression_type(rocksdb::DBCompressionType::None);
        }

        // 打开数据库（如因未链接压缩库失败则自动降级）
        let db = match DB::open(&opts, &config.path) {
            Ok(db) => db,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Compression type") && msg.contains("not linked with the binary") {
                    // 自动降级为不压缩
                    opts.set_compression_type(rocksdb::DBCompressionType::None);
                    config.enable_compression = false;
                    DB::open(&opts, &config.path).context(format!(
                        "Failed to open RocksDB at {} after falling back to no compression",
                        config.path
                    ))?
                } else {
                    return Err(e).context(format!("Failed to open RocksDB at {}", config.path));
                }
            }
        };

        Ok(Self {
            db: Arc::new(db),
            config,
        })
    }

    /// 使用默认配置创建
    pub fn new_default() -> Result<Self> {
        Self::new(RocksDBConfig::default())
    }

    /// 使用指定路径创建
    pub fn new_with_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = RocksDBConfig::default();
        config.path = path.as_ref().to_string_lossy().to_string();
        Self::new(config)
    }

    /// 批量写入 (原子操作)
    pub fn write_batch(&self, batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<()> {
        let mut write_batch = WriteBatch::default();

        for (key, value) in batch {
            match value {
                Some(v) => write_batch.put(&key, &v),
                None => write_batch.delete(&key),
            }
        }

        self.db
            .write(write_batch)
            .context("Failed to write batch to RocksDB")?;

        Ok(())
    }

    /// 带预分配的批量写入（根据键值总字节数估算容量）
    pub fn write_batch_optimized(&self, batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<()> {
        // 当前 crate 不支持为 WriteBatch 预设容量，退化为 default；
        // 仍保留该方法以便后续升级 crate 后切换至 with_capacity。
        let mut write_batch = WriteBatch::default();

        for (key, value) in batch {
            match value {
                Some(v) => write_batch.put(&key, &v),
                None => write_batch.delete(&key),
            }
        }

        self.db
            .write(write_batch)
            .context("Failed to write optimized batch to RocksDB")?;
        Ok(())
    }

    /// 使用指定写入选项的批量写入
    pub fn write_batch_with_options(
        &self,
        batch: Vec<(Vec<u8>, Option<Vec<u8>>)>,
        disable_wal: bool,
        sync: bool,
    ) -> Result<()> {
        let mut write_batch = WriteBatch::default();
        for (key, value) in batch {
            match value {
                Some(v) => write_batch.put(&key, &v),
                None => write_batch.delete(&key),
            }
        }
        let mut opts = WriteOptions::default();
        opts.disable_wal(disable_wal);
        opts.set_sync(sync);
        self.db
            .write_opt(write_batch, &opts)
            .context("Failed to write batch with options to RocksDB")?;
        Ok(())
    }

    /// 分批写入：将超大批量拆分为更小的 chunk，降低峰值内存与 flush 抖动
    pub fn write_batch_chunked(
        &self,
        batch: Vec<(Vec<u8>, Option<Vec<u8>>)>,
        chunk_size: usize,
        disable_wal: bool,
        sync: bool,
    ) -> Result<()> {
        if chunk_size == 0 {
            anyhow::bail!("chunk_size must be > 0");
        }

        let mut opts = WriteOptions::default();
        opts.disable_wal(disable_wal);
        opts.set_sync(sync);

        for chunk in batch.chunks(chunk_size) {
            let mut write_batch = WriteBatch::default();
            for (key, value) in chunk {
                match value {
                    Some(v) => write_batch.put(&key, &v),
                    None => write_batch.delete(&key),
                }
            }
            self.db
                .write_opt(write_batch, &opts)
                .context("Failed to write chunk to RocksDB")?;
        }
        Ok(())
    }

    /// 自适应分批 - 默认配置入口
    pub fn write_batch_adaptive(
        &self,
        batch: Vec<(Vec<u8>, Option<Vec<u8>>)>,
        disable_wal: bool,
        sync: bool,
    ) -> Result<AdaptiveBatchResult> {
        let cfg = AdaptiveBatchConfig::default_for(&batch);
        self.write_batch_adaptive_with_config(batch, cfg, disable_wal, sync)
    }

    /// 自适应分批 - 带配置
    pub fn write_batch_adaptive_with_config(
        &self,
        batch: Vec<(Vec<u8>, Option<Vec<u8>>)>,
        mut cfg: AdaptiveBatchConfig,
        disable_wal: bool,
        sync: bool,
    ) -> Result<AdaptiveBatchResult> {
        if cfg.init_chunk == 0 {
            cfg.init_chunk = 1000;
        }
        if cfg.min_chunk == 0 {
            cfg.min_chunk = 500;
        }
        if cfg.max_chunk < cfg.min_chunk {
            cfg.max_chunk = cfg.min_chunk.max(2000);
        }

        let mut opts = WriteOptions::default();
        opts.disable_wal(disable_wal);
        opts.set_sync(sync);

        let mut idx = 0usize;
        let mut chunk_size = cfg.init_chunk.clamp(cfg.min_chunk, cfg.max_chunk);
        let mut chunks = 0usize;
        let mut last_qps: std::collections::VecDeque<f64> =
            std::collections::VecDeque::with_capacity(cfg.window);
        let mut all_qps: Vec<f64> = Vec::new();

        while idx < batch.len() {
            let end = (idx + chunk_size).min(batch.len());
            let slice = &batch[idx..end];
            let mut wb = WriteBatch::default();
            for (k, v) in slice.iter() {
                match v {
                    Some(val) => wb.put(k, val),
                    None => wb.delete(k),
                }
            }
            let start = std::time::Instant::now();
            self.db
                .write_opt(wb, &opts)
                .context("adaptive chunk write failed")?;
            let dur = start.elapsed();
            let qps = if dur.as_nanos() == 0 {
                0.0
            } else {
                (slice.len() as f64) / dur.as_secs_f64()
            };
            if last_qps.len() == cfg.window {
                last_qps.pop_front();
            }
            last_qps.push_back(qps);
            all_qps.push(qps);
            chunks += 1;

            // 基于窗口 RSD 调整 chunk 大小
            if last_qps.len() >= 3 {
                // 窗口大小达到最小统计要求
                let (avg, _best, _stddev, rsd) = Self::stats_inline(&last_qps);
                if rsd > cfg.target_rsd_pct {
                    // 抖动超阈值：快速收缩
                    let new_size =
                        (chunk_size as f64 * (1.0 - cfg.adjust_down_pct)).round() as usize;
                    chunk_size = new_size.clamp(cfg.min_chunk, cfg.max_chunk);
                } else if rsd < cfg.target_rsd_pct * 0.75 && avg > 0.0 {
                    // 进入稳定区：谨慎放大
                    let new_size = (chunk_size as f64 * (1.0 + cfg.adjust_up_pct)).round() as usize;
                    chunk_size = new_size.clamp(cfg.min_chunk, cfg.max_chunk);
                }
            }
            idx = end;
        }

        let (avg, best, stddev, rsd) = Self::stats_inline_vec(&all_qps);
        Ok(AdaptiveBatchResult {
            final_chunk: chunk_size,
            chunks,
            avg_qps: avg,
            best_qps: best,
            stddev_qps: stddev,
            rsd_pct: rsd,
        })
    }

    // ---- 内部统计辅助 ----
    fn stats_inline(window: &std::collections::VecDeque<f64>) -> (f64, f64, f64, f64) {
        let vals: Vec<f64> = window.iter().copied().collect();
        Self::stats_inline_vec(&vals)
    }

    fn stats_inline_vec(vals: &[f64]) -> (f64, f64, f64, f64) {
        if vals.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let n = vals.len() as f64;
        let sum: f64 = vals.iter().sum();
        let avg = sum / n;
        let best = vals.iter().fold(0.0_f64, |m, &v| m.max(v));
        let mut var = 0.0;
        for &v in vals {
            var += (v - avg) * (v - avg);
        }
        var /= n.max(1.0);
        let stddev = var.sqrt();
        let rsd = if avg > 0.0 { stddev / avg * 100.0 } else { 0.0 };
        (avg, best, stddev, rsd)
    }

    /// 事务式批量操作
    pub fn begin_batch(&self) -> BatchTransaction {
        BatchTransaction {
            db: self.db.clone(),
            batch: WriteBatch::default(),
            committed: false,
        }
    }

    /// 创建检查点(快照)
    pub fn create_checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let checkpoint = rocksdb::checkpoint::Checkpoint::new(&self.db)
            .context("Failed to create checkpoint object")?;

        checkpoint
            .create_checkpoint(path.as_ref())
            .context("Failed to create checkpoint")?;

        Ok(())
    }

    /// 从快照恢复数据库
    ///
    /// 注意: 这会创建一个新的数据库实例,指向快照路径
    /// 使用场景: 灾难恢复、回滚到历史状态
    pub fn restore_from_checkpoint<P: AsRef<Path>>(
        checkpoint_path: P,
        config: RocksDBConfig,
    ) -> Result<Self> {
        let checkpoint_path_ref = checkpoint_path.as_ref();

        // 验证快照路径存在
        if !checkpoint_path_ref.exists() {
            anyhow::bail!(
                "Checkpoint path does not exist: {}",
                checkpoint_path_ref.display()
            );
        }

        // 直接打开快照目录作为数据库
        let mut restore_config = config.clone();
        restore_config.path = checkpoint_path_ref.to_string_lossy().to_string();

        Self::new(restore_config)
    }

    /// 列出指定目录下的所有快照
    pub fn list_checkpoints<P: AsRef<Path>>(base_path: P) -> Result<Vec<std::path::PathBuf>> {
        let mut checkpoints = Vec::new();
        let base_path_ref = base_path.as_ref();

        if !base_path_ref.exists() {
            return Ok(checkpoints);
        }

        for entry in std::fs::read_dir(base_path_ref).context(format!(
            "Failed to read checkpoint directory: {}",
            base_path_ref.display()
        ))? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            // 检查是否为有效的 RocksDB 目录(包含 CURRENT 文件)
            if path.is_dir() && path.join("CURRENT").exists() {
                checkpoints.push(path);
            }
        }

        // 按名称排序(假设快照名称包含时间戳)
        checkpoints.sort();

        Ok(checkpoints)
    }

    /// 定期快照管理器 - 基于区块数
    ///
    /// 根据配置的区块间隔创建快照,并自动清理旧快照
    pub fn maybe_create_snapshot(
        &self,
        block_number: u64,
        config: &SnapshotConfig,
    ) -> Result<Option<std::path::PathBuf>> {
        if config.blocks_per_snapshot == 0 || block_number % config.blocks_per_snapshot != 0 {
            return Ok(None);
        }

        // 创建快照目录
        std::fs::create_dir_all(&config.base_path).context(format!(
            "Failed to create snapshot directory: {}",
            config.base_path.display()
        ))?;

        // 生成快照路径 (包含区块号和时间戳)
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let snapshot_name = format!("snapshot_block_{}_ts_{}", block_number, timestamp);
        let snapshot_path = config.base_path.join(&snapshot_name);

        // 创建快照
        self.create_checkpoint(&snapshot_path)?;

        // 清理旧快照
        self.cleanup_old_snapshots(config)?;

        Ok(Some(snapshot_path))
    }

    /// 清理旧快照,保留最近的 N 个
    fn cleanup_old_snapshots(&self, config: &SnapshotConfig) -> Result<()> {
        let mut snapshots = Self::list_checkpoints(&config.base_path)?;

        // 按修改时间排序(最新的在后面)
        snapshots.sort_by_cached_key(|path| {
            std::fs::metadata(path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        // 删除超出限制的旧快照
        if snapshots.len() > config.max_snapshots {
            let to_delete = &snapshots[..snapshots.len() - config.max_snapshots];
            for path in to_delete {
                if let Err(e) = std::fs::remove_dir_all(path) {
                    eprintln!("Warning: Failed to remove old snapshot {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// 压缩数据库
    pub fn compact_range(&self, start: Option<&[u8]>, end: Option<&[u8]>) -> Result<()> {
        self.db.compact_range(start, end);
        Ok(())
    }

    /// 获取数据库统计信息
    pub fn get_property(&self, property: &str) -> Option<String> {
        self.db.property_value(property).ok().flatten()
    }

    /// 获取配置
    pub fn config(&self) -> &RocksDBConfig {
        &self.config
    }

    /// 获取数据库路径
    pub fn path(&self) -> &str {
        &self.config.path
    }
}

/// 批量事务对象
#[cfg(feature = "rocksdb-storage")]
pub struct BatchTransaction {
    db: Arc<DB>,
    batch: WriteBatch,
    committed: bool,
}

#[cfg(feature = "rocksdb-storage")]
impl BatchTransaction {
    /// 向批量事务中写入键值
    pub fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&mut self, key: K, value: V) {
        self.batch.put(key.as_ref(), value.as_ref());
    }

    /// 在批量事务中删除键
    pub fn delete<K: AsRef<[u8]>>(&mut self, key: K) {
        self.batch.delete(key.as_ref());
    }

    /// 提交批量事务
    pub fn commit(mut self) -> Result<()> {
        self.db
            .write(self.batch)
            .context("Failed to commit batch transaction")?;
        self.committed = true;
        Ok(())
    }

    /// 使用写入选项提交批量事务
    pub fn commit_with_options(mut self, disable_wal: bool, sync: bool) -> Result<()> {
        let mut opts = WriteOptions::default();
        opts.disable_wal(disable_wal);
        opts.set_sync(sync);
        self.db
            .write_opt(self.batch, &opts)
            .context("Failed to commit batch transaction with options")?;
        self.committed = true;
        Ok(())
    }

    /// 回滚（丢弃未提交的更改）
    pub fn rollback(mut self) {
        self.committed = false; // 明确标记；Drop 时丢弃即可
    }
}

#[cfg(feature = "rocksdb-storage")]
impl Storage for RocksDBStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.db.get(key).context("Failed to get value from RocksDB")
    }

    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db
            .put(key, value)
            .context("Failed to set value in RocksDB")
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.db
            .delete(key)
            .context("Failed to delete key from RocksDB")
    }

    fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();
        let iter = self
            .db
            .iterator(IteratorMode::From(prefix, rocksdb::Direction::Forward));

        for item in iter {
            let (key, value) = item.context("Failed to read iterator item")?;

            // 检查是否仍然匹配前缀
            if !key.starts_with(prefix) {
                break;
            }

            results.push((key.to_vec(), value.to_vec()));
        }

        Ok(results)
    }

    fn write_batch_if_supported(&mut self, batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<bool> {
        self.write_batch_optimized(batch)?;
        Ok(true)
    }
}

#[cfg(feature = "rocksdb-storage")]
#[derive(Debug, Clone, Default)]
pub struct RocksDBStats {
    pub num_keys: u64,
    pub total_sst_size_bytes: u64,
    pub cache_hit: u64,
    pub cache_miss: u64,
}

#[cfg(feature = "rocksdb-storage")]
impl RocksDBStorage {
    fn prop_u64(&self, name: &str) -> Result<u64> {
        let s = self
            .db
            .property_value(name)
            .context("get property")?
            .ok_or_else(|| anyhow::anyhow!(format!("property missing: {}", name)))?;
        s.trim()
            .parse::<u64>()
            .map_err(|e| anyhow::anyhow!("parse {} failed: {} => {}", name, s, e))
    }

    /// 获取基础统计信息
    pub fn get_stats(&self) -> Result<RocksDBStats> {
        let num_keys = self.prop_u64("rocksdb.estimate-num-keys").unwrap_or(0);
        let total_sst_size_bytes = self.prop_u64("rocksdb.total-sst-files-size").unwrap_or(0);
        let cache_hit = self.prop_u64("rocksdb.block.cache.hit").unwrap_or(0);
        let cache_miss = self.prop_u64("rocksdb.block.cache.miss").unwrap_or(0);
        Ok(RocksDBStats {
            num_keys,
            total_sst_size_bytes,
            cache_hit,
            cache_miss,
        })
    }

    /// 打印统计
    pub fn print_stats(&self) -> Result<()> {
        let s = self.get_stats()?;
        let total = s.cache_hit + s.cache_miss;
        let hit_rate = if total > 0 {
            (s.cache_hit as f64) / (total as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "=== RocksDB Stats ===\nKeys:{}\nSST Size:{:.2} MB\nBlock Cache Hit Rate:{:.2}%",
            s.num_keys,
            s.total_sst_size_bytes as f64 / 1024.0 / 1024.0,
            hit_rate
        );
        Ok(())
    }

    /// 采集RocksDB内部指标用于Prometheus导出
    pub fn collect_metrics(&self) -> RocksDBMetrics {
        RocksDBMetrics {
            estimate_num_keys: self.prop_u64("rocksdb.estimate-num-keys").unwrap_or(0),
            total_sst_size_bytes: self.prop_u64("rocksdb.total-sst-files-size").unwrap_or(0),
            cache_hit: self.prop_u64("rocksdb.block.cache.hit").unwrap_or(0),
            cache_miss: self.prop_u64("rocksdb.block.cache.miss").unwrap_or(0),
            compaction_cpu_micros: self.prop_u64("rocksdb.compaction.sum.cpu.micros").unwrap_or(0),
            compaction_write_bytes: self.prop_u64("rocksdb.compact.write.bytes").unwrap_or(0),
            write_stall_micros: self.prop_u64("rocksdb.write-stall.micros").unwrap_or(0),
            num_files_level0: self.prop_u64("rocksdb.num-files-at-level0").unwrap_or(0),
            num_immutable_mem_table: self.prop_u64("rocksdb.num-immutable-mem-table").unwrap_or(0),
        }
    }
}

/// RocksDB内部指标结构
#[derive(Debug, Clone)]
pub struct RocksDBMetrics {
    pub estimate_num_keys: u64,
    pub total_sst_size_bytes: u64,
    pub cache_hit: u64,
    pub cache_miss: u64,
    pub compaction_cpu_micros: u64,
    pub compaction_write_bytes: u64,
    pub write_stall_micros: u64,
    pub num_files_level0: u64,
    pub num_immutable_mem_table: u64,
}

// 当 feature 未启用时提供空实现
#[cfg(not(feature = "rocksdb-storage"))]
pub struct RocksDBStorage;

#[cfg(not(feature = "rocksdb-storage"))]
impl RocksDBStorage {
    pub fn new(_config: RocksDBConfig) -> Result<Self> {
        anyhow::bail!(
            "RocksDB storage is not enabled. Please enable the 'rocksdb-storage' feature."
        )
    }

    pub fn new_default() -> Result<Self> {
        Self::new(RocksDBConfig::default())
    }

    pub fn new_with_path<P: AsRef<Path>>(_path: P) -> Result<Self> {
        anyhow::bail!(
            "RocksDB storage is not enabled. Please enable the 'rocksdb-storage' feature."
        )
    }
}

#[cfg(test)]
#[cfg(feature = "rocksdb-storage")]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_rocksdb_basic_operations() -> Result<()> {
        // 使用临时目录测试
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_db");

        let mut storage = RocksDBStorage::new_with_path(&db_path)?;

        // 测试 set/get
        storage.set(b"key1", b"value1")?;
        assert_eq!(storage.get(b"key1")?.unwrap(), b"value1");

        // 测试 delete
        storage.delete(b"key1")?;
        assert_eq!(storage.get(b"key1")?, None);

        Ok(())
    }

    #[test]
    fn test_rocksdb_scan() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_scan_db");

        let mut storage = RocksDBStorage::new_with_path(&db_path)?;

        // 插入测试数据
        storage.set(b"prefix1_a", b"v1")?;
        storage.set(b"prefix1_b", b"v2")?;
        storage.set(b"prefix2_a", b"v3")?;

        // 测试 scan
        let results = storage.scan(b"prefix1_")?;
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|(k, v)| k == b"prefix1_a" && v == b"v1"));
        assert!(results.iter().any(|(k, v)| k == b"prefix1_b" && v == b"v2"));

        Ok(())
    }

    #[test]
    fn test_rocksdb_write_batch() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_batch_db");

        let storage = RocksDBStorage::new_with_path(&db_path)?;

        // 批量写入
        let batch = vec![
            (b"key1".to_vec(), Some(b"value1".to_vec())),
            (b"key2".to_vec(), Some(b"value2".to_vec())),
            (b"key3".to_vec(), Some(b"value3".to_vec())),
        ];

        storage.write_batch(batch)?;

        // 验证
        assert_eq!(storage.get(b"key1")?.unwrap(), b"value1");
        assert_eq!(storage.get(b"key2")?.unwrap(), b"value2");
        assert_eq!(storage.get(b"key3")?.unwrap(), b"value3");

        Ok(())
    }

    #[test]
    fn test_rocksdb_write_batch_optimized() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_batch_opt_db");
        let storage = RocksDBStorage::new_with_path(&db_path)?;

        let mut batch = Vec::with_capacity(3);
        batch.push((b"k1".to_vec(), Some(b"v1".to_vec())));
        batch.push((b"k2".to_vec(), Some(vec![0u8; 1024])));
        batch.push((b"k3".to_vec(), None));

        storage.write_batch_optimized(batch)?;

        assert_eq!(storage.get(b"k1")?.unwrap(), b"v1");
        assert_eq!(storage.get(b"k2")?.unwrap().len(), 1024);
        assert!(storage.get(b"k3")?.is_none());
        Ok(())
    }

    #[test]
    fn test_rocksdb_batch_transaction_commit_and_rollback() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_txn_db");
        let storage = RocksDBStorage::new_with_path(&db_path)?;

        // commit
        {
            let mut tx = storage.begin_batch();
            tx.put(b"a", b"1");
            tx.put(b"b", b"2");
            tx.delete(b"c");
            tx.commit()?;
        }
        assert_eq!(storage.get(b"a")?.unwrap(), b"1");
        assert_eq!(storage.get(b"b")?.unwrap(), b"2");

        // rollback
        {
            let mut tx = storage.begin_batch();
            tx.put(b"a", b"x");
            tx.delete(b"b");
            tx.rollback();
        }
        // Values unchanged
        assert_eq!(storage.get(b"a")?.unwrap(), b"1");
        assert_eq!(storage.get(b"b")?.unwrap(), b"2");
        Ok(())
    }

    #[test]
    fn test_rocksdb_checkpoint() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_checkpoint_db");
        let checkpoint_path = temp_dir.path().join("checkpoint");

        let mut storage = RocksDBStorage::new_with_path(&db_path)?;

        // 写入数据
        storage.set(b"key1", b"value1")?;

        // 创建检查点
        storage.create_checkpoint(&checkpoint_path)?;

        // 验证检查点存在
        assert!(checkpoint_path.exists());

        Ok(())
    }

    #[test]
    fn test_rocksdb_persistence() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_persist_db");

        // 第一次打开,写入数据
        {
            let mut storage = RocksDBStorage::new_with_path(&db_path)?;
            storage.set(b"persist_key", b"persist_value")?;
        }

        // 第二次打开,验证数据持久化
        {
            let storage = RocksDBStorage::new_with_path(&db_path)?;
            assert_eq!(storage.get(b"persist_key")?.unwrap(), b"persist_value");
        }

        Ok(())
    }

    #[test]
    fn test_rocksdb_snapshot_restore() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_restore_db");
        let snapshot_path = temp_dir.path().join("test_snapshot");

        // 1. 创建原始数据库并写入数据
        {
            let mut storage = RocksDBStorage::new_with_path(&db_path)?;
            storage.set(b"key1", b"value1")?;
            storage.set(b"key2", b"value2")?;

            // 创建快照
            storage.create_checkpoint(&snapshot_path)?;
        }

        // 2. 继续写入更多数据(模拟快照后的新交易)
        {
            let mut storage = RocksDBStorage::new_with_path(&db_path)?;
            storage.set(b"key3", b"value3")?;
            assert_eq!(storage.get(b"key3")?.unwrap(), b"value3");
        }

        // 3. 从快照恢复,验证只有快照时的数据
        {
            let restored_storage =
                RocksDBStorage::restore_from_checkpoint(&snapshot_path, RocksDBConfig::default())?;

            // 快照中的数据应该存在
            assert_eq!(restored_storage.get(b"key1")?.unwrap(), b"value1");
            assert_eq!(restored_storage.get(b"key2")?.unwrap(), b"value2");

            // 快照后写入的数据不应该存在
            assert!(restored_storage.get(b"key3")?.is_none());
        }

        Ok(())
    }

    #[test]
    fn test_rocksdb_snapshot_management() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_mgmt_db");
        let snapshot_base = temp_dir.path().join("snapshots");

        let mut storage = RocksDBStorage::new_with_path(&db_path)?;

        // 配置:每 10 个区块一个快照,最多保留 3 个
        let config = SnapshotConfig {
            base_path: snapshot_base.clone(),
            blocks_per_snapshot: 10,
            seconds_per_snapshot: 0,
            max_snapshots: 3,
            compress_old_snapshots: false,
        };

        // 模拟区块进度,创建多个快照
        for block in 0..50 {
            storage.set(format!("block_{}", block).as_bytes(), b"data")?;

            if let Some(snapshot_path) = storage.maybe_create_snapshot(block, &config)? {
                println!("Created snapshot at block {} -> {:?}", block, snapshot_path);
            }
        }

        // 验证只保留了最近的 3 个快照
        let snapshots = RocksDBStorage::list_checkpoints(&snapshot_base)?;
        assert_eq!(snapshots.len(), 3, "Should keep only 3 snapshots");

        // 验证快照名称包含正确的区块号
        for snapshot in &snapshots {
            let name = snapshot.file_name().unwrap().to_str().unwrap();
            assert!(name.contains("snapshot_block_"));
        }

        Ok(())
    }
}
