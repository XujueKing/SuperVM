# 🌐 智能分布式存储与性能优化方案

## 核心理念: **数据随网络拓扑自适应分布**

四层网络中的存储不是静态的,而是根据以下因素**动态调整**:

1. **节点负载** (CPU/内存/磁盘 IO)
2. **网络拓扑** (节点地理位置/延迟/带宽)
3. **数据热度** (访问频率/最近访问时间)
4. **容量状态** (磁盘使用率/剩余空间)
5. **节点健康** (故障率/响应时间/在线时长)

---

## 🏗️ 分布式存储架构

```
┌─────────────────────────────────────────────────────────┐
│                    智能调度层                              │
│  ┌──────────────────────────────────────────────────┐   │
│  │  StorageOrchestrator (存储编排器)                 │   │
│  │  - 监控所有节点状态                                 │   │
│  │  - 决策数据迁移/复制                                │   │
│  │  - 执行自动负载均衡                                 │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
         ↓                ↓                ↓
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   L1 存储    │  │   L2 存储    │  │   L3 存储    │
│  RocksDB     │  │  RocksDB     │  │  LRU Cache   │
│  全量+权威   │  │  部分+活跃   │  │  热点+临时   │
│  10-100 TB   │  │  500GB-2TB   │  │  100GB-1TB   │
│  3副本+BFT   │  │  2副本+RAFT  │  │  无副本      │
└─────────────┘  └─────────────┘  └─────────────┘
         ↓                ↓                ↓
    [热数据自动上浮]  [温数据智能缓存]  [冷数据自动下沉]
```

### 四层存储分工

| 层级 | 存储类型 | 容量范围 | 数据内容 | 副本策略 | 访问模式 |
|-----|---------|---------|---------|---------|---------|
| **L1 Supernode** | RocksDB (NVMe) | 10-100 TB | 全量历史+权威状态 | 3副本 + BFT | 低频全局查询 |
| **L2 Miner** | RocksDB (SSD) | 500GB-2TB | 最近N块+活跃账户 | 2副本 + RAFT | 中频交易执行 |
| **L3 Edge** | LRU Cache (内存) | 100GB-1TB | 热点数据+区域缓存 | 无副本 | 高频用户查询 |
| **L4 Mobile** | SQLite (本地) | 1-10GB | 用户专属+离线队列 | 无副本 | 本地操作 |

---

## 📊 存储分级策略 (三温存储)

### 设计理念

区块链状态数据并非"一视同仁",而是分为:
- **热数据** (1%): 高频访问 (如热门 NFT, DeFi 合约) → NVMe + 大内存
- **温数据** (19%): 中频访问 (如活跃账户) → SATA SSD + 适中缓存
- **冷数据** (80%): 罕见访问 (如历史区块) → HDD + 高压缩

通过**自动分级**,可节省 70% 存储成本,同时保持热数据极致性能。

### 代码实现

```rust
// src/node-core/src/storage/tiered_storage.rs

pub struct TieredStorage {
    hot_tier: HotStorage,     // 热数据: SSD/NVMe + 大内存缓存
    warm_tier: WarmStorage,   // 温数据: SSD + 适中缓存
    cold_tier: ColdStorage,   // 冷数据: HDD + 小缓存/无缓存
    classifier: DataClassifier, // 数据分类器
}

/// 数据温度分类器
pub struct DataClassifier {
    access_tracker: DashMap<Key, AccessStats>,
    hot_threshold: u32,   // 访问次数 > 1000/天
    warm_threshold: u32,  // 访问次数 100-1000/天
}

pub struct AccessStats {
    pub total_accesses: AtomicU64,
    pub last_access_time: AtomicU64,
    pub access_pattern: AccessPattern,  // 随机/顺序/批量
}

#[derive(Debug, Clone)]
pub enum AccessPattern {
    Random,      // 随机访问 (账户余额查询)
    Sequential,  // 顺序访问 (区块扫描)
    Batch,       // 批量访问 (批量转账)
    RareWrite,   // 罕见写入 (历史数据修正)
}

impl DataClassifier {
    /// 根据访问模式决定数据存储层级
    pub fn classify(&self, key: &Key) -> StorageTier {
        let stats = self.access_tracker.get(key);
        
        match stats {
            Some(s) => {
                let accesses_per_day = s.total_accesses.load(Ordering::Relaxed) 
                    / self.days_since_creation();
                let hours_since_access = self.hours_since(s.last_access_time);
                
                // 决策树
                if accesses_per_day > self.hot_threshold && hours_since_access < 1 {
                    StorageTier::Hot
                } else if accesses_per_day > self.warm_threshold && hours_since_access < 24 {
                    StorageTier::Warm
                } else {
                    StorageTier::Cold
                }
            }
            None => StorageTier::Cold,  // 新数据默认冷存储
        }
    }
    
    /// 周期性重新分类 (每小时执行)
    pub async fn reclassify_all(&self) -> Result<ReclassifyReport> {
        let mut moved_to_hot = 0;
        let mut moved_to_warm = 0;
        let mut moved_to_cold = 0;
        
        for entry in self.access_tracker.iter() {
            let key = entry.key();
            let new_tier = self.classify(key);
            let current_tier = self.get_current_tier(key)?;
            
            if new_tier != current_tier {
                self.migrate_data(key, current_tier, new_tier).await?;
                
                match new_tier {
                    StorageTier::Hot => moved_to_hot += 1,
                    StorageTier::Warm => moved_to_warm += 1,
                    StorageTier::Cold => moved_to_cold += 1,
                }
            }
        }
        
        Ok(ReclassifyReport {
            moved_to_hot,
            moved_to_warm,
            moved_to_cold,
            total_keys: self.access_tracker.len(),
        })
    }
}
```

### 三温存储配置

```rust
/// 三温存储配置
pub struct StorageConfig {
    pub hot: HotStorageConfig,
    pub warm: WarmStorageConfig,
    pub cold: ColdStorageConfig,
}

pub struct HotStorageConfig {
    pub device: String,              // "/dev/nvme0n1" (NVMe SSD)
    pub cache_size_mb: usize,        // 16GB 内存缓存
    pub write_buffer_mb: usize,      // 512MB 写缓冲
    pub bloom_filter_bits: usize,    // 10 bits/key
    pub compression: CompressionType, // LZ4 (快速压缩)
    pub max_open_files: usize,       // 10000
    pub target_iops: usize,          // 500K read, 300K write
}

pub struct WarmStorageConfig {
    pub device: String,              // "/dev/sda1" (SATA SSD)
    pub cache_size_mb: usize,        // 4GB 内存缓存
    pub write_buffer_mb: usize,      // 128MB 写缓冲
    pub bloom_filter_bits: usize,    // 8 bits/key
    pub compression: CompressionType, // Zstd (中等压缩)
    pub max_open_files: usize,       // 5000
    pub target_iops: usize,          // 200K read, 100K write
}

pub struct ColdStorageConfig {
    pub device: String,              // "/dev/sdb1" (HDD)
    pub cache_size_mb: usize,        // 512MB 内存缓存
    pub write_buffer_mb: usize,      // 32MB 写缓冲
    pub bloom_filter_bits: usize,    // 5 bits/key
    pub compression: CompressionType, // Zstd level 10 (高压缩)
    pub max_open_files: usize,       // 1000
    pub target_iops: usize,          // 10K read, 5K write
}
```

### 性能对比

| 存储层级 | 设备类型 | 缓存大小 | 读IOPS | 写IOPS | 延迟 | 成本/TB |
|---------|---------|---------|--------|--------|------|---------|
| **Hot** | NVMe SSD | 16GB | 500K | 300K | 0.1ms | $300 |
| **Warm** | SATA SSD | 4GB | 200K | 100K | 1ms | $100 |
| **Cold** | HDD | 512MB | 10K | 5K | 10ms | $20 |

**成本优化示例**:
- 传统方案: 100TB × $300 = **$30,000**
- 三温分级: 1TB(Hot) × $300 + 19TB(Warm) × $100 + 80TB(Cold) × $20 = **$3,500** (节省 88%)

---

## 🔄 自动数据迁移机制

### 迁移触发条件

```rust
// src/node-core/src/storage/data_migration.rs

pub struct DataMigrationManager {
    orchestrator: Arc<StorageOrchestrator>,
    migration_queue: Arc<Mutex<VecDeque<MigrationTask>>>,
    worker_threads: usize,
}

pub struct MigrationTask {
    pub key_range: (Key, Key),       // 数据范围
    pub from_node: NodeId,            // 源节点
    pub to_node: NodeId,              // 目标节点
    pub priority: MigrationPriority,  // 优先级
    pub reason: MigrationReason,      // 迁移原因
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationPriority {
    Critical = 3,  // 节点即将下线/磁盘将满
    High = 2,      // 负载严重不均
    Normal = 1,    // 优化性能
    Low = 0,       // 后台整理
}

#[derive(Debug, Clone)]
pub enum MigrationReason {
    NodeOverload { cpu: f64, disk_io: f64 },  // 节点过载
    DiskFull { usage_percent: f64 },          // 磁盘将满
    HotDataReplication,                       // 热数据复制到更多节点
    ColdDataArchive,                          // 冷数据归档到HDD
    NetworkOptimization,                      // 将数据移到离用户更近的节点
    NodeFailure { failed_node: NodeId },      // 节点故障恢复
}
```

### 自动检测与执行

```rust
impl DataMigrationManager {
    /// 自动检测需要迁移的数据
    pub async fn detect_migration_needs(&self) -> Result<Vec<MigrationTask>> {
        let mut tasks = Vec::new();
        let nodes = self.orchestrator.get_all_nodes().await?;
        
        for node in nodes {
            // 1. 检测磁盘使用率
            if node.disk_usage_percent > 85.0 {
                let task = self.create_disk_relief_task(node).await?;
                tasks.push(task);
            }
            
            // 2. 检测负载不均
            if node.cpu_usage > 80.0 || node.disk_io_usage > 90.0 {
                let task = self.create_load_balance_task(node).await?;
                tasks.push(task);
            }
            
            // 3. 检测热数据需要复制
            let hot_keys = self.orchestrator.get_hot_keys(node.id).await?;
            if hot_keys.len() > 100 {
                let task = self.create_hot_replication_task(node, hot_keys).await?;
                tasks.push(task);
            }
            
            // 4. 检测冷数据可以归档
            let cold_keys = self.orchestrator.get_cold_keys(node.id).await?;
            if cold_keys.len() > 10000 {
                let task = self.create_cold_archive_task(node, cold_keys).await?;
                tasks.push(task);
            }
        }
        
        // 5. 按优先级排序
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(tasks)
    }
    
    /// 执行数据迁移
    pub async fn execute_migration(&self, task: MigrationTask) -> Result<MigrationResult> {
        let start_time = Instant::now();
        
        // 1. 预检查
        let source_available = self.orchestrator.check_node_health(&task.from_node).await?;
        let target_available = self.orchestrator.check_node_health(&task.to_node).await?;
        
        if !source_available || !target_available {
            return Err(anyhow!("Source or target node unavailable"));
        }
        
        // 2. 批量读取源数据 (避免单条读取)
        let data = self.batch_read_range(
            &task.from_node,
            &task.key_range.0,
            &task.key_range.1,
        ).await?;
        
        let total_keys = data.len();
        let total_bytes = data.iter().map(|(_, v)| v.len()).sum::<usize>();
        
        // 3. 批量写入目标节点
        self.batch_write(&task.to_node, data.clone()).await?;
        
        // 4. 验证数据完整性
        let verification_samples = self.sample_keys(&data, 100);
        for (key, expected_value) in verification_samples {
            let actual_value = self.read_single(&task.to_node, &key).await?;
            if actual_value != expected_value {
                return Err(anyhow!("Data verification failed for key {:?}", key));
            }
        }
        
        // 5. 删除源数据 (如果是移动操作)
        if matches!(task.reason, MigrationReason::DiskFull { .. } | MigrationReason::ColdDataArchive) {
            self.batch_delete(&task.from_node, data.keys().cloned().collect()).await?;
        }
        
        Ok(MigrationResult {
            total_keys,
            total_bytes,
            duration: start_time.elapsed(),
            throughput_mbps: (total_bytes as f64 / 1_000_000.0) / start_time.elapsed().as_secs_f64(),
        })
    }
    
    /// 智能负载均衡 (自动在 L1/L2 节点间平衡数据)
    pub async fn auto_rebalance(&self) -> Result<RebalanceReport> {
        let nodes = self.orchestrator.get_all_nodes().await?;
        
        // 1. 计算平均负载
        let avg_disk_usage: f64 = nodes.iter()
            .map(|n| n.disk_usage_percent)
            .sum::<f64>() / nodes.len() as f64;
        
        let avg_cpu_usage: f64 = nodes.iter()
            .map(|n| n.cpu_usage)
            .sum::<f64>() / nodes.len() as f64;
        
        // 2. 识别过载和空闲节点
        let overloaded: Vec<_> = nodes.iter()
            .filter(|n| n.disk_usage_percent > avg_disk_usage + 20.0 
                     || n.cpu_usage > avg_cpu_usage + 20.0)
            .collect();
        
        let underloaded: Vec<_> = nodes.iter()
            .filter(|n| n.disk_usage_percent < avg_disk_usage - 20.0 
                     && n.cpu_usage < avg_cpu_usage - 20.0)
            .collect();
        
        if overloaded.is_empty() || underloaded.is_empty() {
            return Ok(RebalanceReport::no_action_needed());
        }
        
        // 3. 创建迁移任务
        let mut tasks = Vec::new();
        for (over, under) in overloaded.iter().zip(underloaded.iter()) {
            let keys_to_move = self.select_keys_to_move(over, under).await?;
            
            tasks.push(MigrationTask {
                key_range: keys_to_move,
                from_node: over.id,
                to_node: under.id,
                priority: MigrationPriority::Normal,
                reason: MigrationReason::NodeOverload {
                    cpu: over.cpu_usage,
                    disk_io: over.disk_io_usage,
                },
            });
        }
        
        // 4. 执行迁移
        let mut results = Vec::new();
        for task in tasks {
            let result = self.execute_migration(task).await?;
            results.push(result);
        }
        
        Ok(RebalanceReport {
            tasks_executed: results.len(),
            total_keys_moved: results.iter().map(|r| r.total_keys).sum(),
            total_bytes_moved: results.iter().map(|r| r.total_bytes).sum(),
            total_duration: results.iter().map(|r| r.duration).sum(),
        })
    }
}
```

### 迁移场景示例

| 场景 | 触发条件 | 迁移策略 | 预期效果 |
|-----|---------|---------|---------|
| **磁盘将满** | 使用率 > 85% | 冷数据 → HDD 归档 | 释放 50-70% 空间 |
| **节点过载** | CPU > 80% 或 IO > 90% | 均衡数据到空闲节点 | 负载降低 40-60% |
| **热数据涌现** | 访问频率暴增 | 复制到多个 L2/L3 | 延迟降低 80% |
| **节点故障** | 心跳丢失 > 30s | 副本迁移到健康节点 | 恢复时间 < 5min |
| **地理优化** | 跨区域延迟 > 100ms | 数据迁移到本地 L2 | 延迟降低 70% |

---

## 🚀 性能优化提升方案

### 目标性能 (vs Phase 4.2 原计划)

| 操作类型 | 原计划性能 | 优化目标 | 提升倍数 | 优化策略 |
|---------|-----------|---------|---------|---------|
| 随机读 | 100K ops/s | **500K ops/s** | 5× | Bloom filter + 分层缓存 + 并行读 |
| 随机写 | 50K ops/s | **300K ops/s** | 6× | Write batching + WAL优化 + 异步刷盘 |
| 批量写 | 200K ops/s | **1M ops/s** | 5× | 大批量 + Pipeline + 压缩并行 |
| 扫描 | 500 MB/s | **2 GB/s** | 4× | Prefetch + 顺序读优化 + 零拷贝 |
| 点查延迟 P99 | 10 ms | **2 ms** | 5× | 热数据内存化 + NVMe + 索引优化 |

### 优化策略详解

```rust
// src/node-core/src/storage/optimized_rocksdb.rs

pub struct OptimizedRocksDB {
    db: Arc<DB>,
    hot_cache: Arc<DashMap<Key, Value>>,      // 热数据全内存
    write_buffer: Arc<Mutex<WriteBatch>>,     // 写缓冲
    bloom_filter: Arc<BloomFilter>,           // Bloom 过滤器
    prefetch_engine: Arc<PrefetchEngine>,     // 预取引擎
    parallel_reader: Arc<ParallelReader>,     // 并行读取器
}

impl OptimizedRocksDB {
    /// 创建高性能配置
    pub fn new_high_performance(path: &str, tier: StorageTier) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        // 根据存储层级定制配置
        match tier {
            StorageTier::Hot => {
                // 热数据: 极致性能
                opts.set_write_buffer_size(512 * 1024 * 1024);  // 512MB
                opts.set_max_write_buffer_number(6);
                opts.set_min_write_buffer_number_to_merge(2);
                opts.set_level_zero_file_num_compaction_trigger(4);
                opts.set_max_background_jobs(16);               // 16 个后台线程
                opts.set_max_subcompactions(4);
                opts.set_compression_type(DBCompressionType::Lz4);  // 快速压缩
                opts.set_bloom_locality(1);
                opts.set_memtable_prefix_bloom_ratio(0.1);
                opts.set_allow_mmap_reads(true);                // 内存映射读
                opts.set_allow_mmap_writes(true);               // 内存映射写
                
                // 块缓存 16GB
                let cache = Cache::new_lru_cache(16 * 1024 * 1024 * 1024);
                let mut block_opts = BlockBasedOptions::default();
                block_opts.set_block_cache(&cache);
                block_opts.set_block_size(64 * 1024);           // 64KB 块
                block_opts.set_cache_index_and_filter_blocks(true);
                block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
                block_opts.set_bloom_filter(10.0, false);       // 10 bits/key
                opts.set_block_based_table_factory(&block_opts);
            }
            
            StorageTier::Warm => {
                // 温数据: 平衡性能
                opts.set_write_buffer_size(128 * 1024 * 1024);  // 128MB
                opts.set_max_write_buffer_number(4);
                opts.set_max_background_jobs(8);
                opts.set_compression_type(DBCompressionType::Zstd);
                
                // 块缓存 4GB
                let cache = Cache::new_lru_cache(4 * 1024 * 1024 * 1024);
                let mut block_opts = BlockBasedOptions::default();
                block_opts.set_block_cache(&cache);
                block_opts.set_bloom_filter(8.0, false);
                opts.set_block_based_table_factory(&block_opts);
            }
            
            StorageTier::Cold => {
                // 冷数据: 节省空间
                opts.set_write_buffer_size(32 * 1024 * 1024);   // 32MB
                opts.set_max_write_buffer_number(2);
                opts.set_max_background_jobs(4);
                opts.set_compression_type(DBCompressionType::Zstd);
                opts.set_compression_options(10, 0, 0, 0);      // 最高压缩
                
                // 块缓存 512MB
                let cache = Cache::new_lru_cache(512 * 1024 * 1024);
                let mut block_opts = BlockBasedOptions::default();
                block_opts.set_block_cache(&cache);
                block_opts.set_bloom_filter(5.0, false);
                opts.set_block_based_table_factory(&block_opts);
            }
        }
        
        let db = DB::open(&opts, path)?;
        
        Ok(Self {
            db: Arc::new(db),
            hot_cache: Arc::new(DashMap::new()),
            write_buffer: Arc::new(Mutex::new(WriteBatch::default())),
            bloom_filter: Arc::new(BloomFilter::new(1_000_000, 0.01)),
            prefetch_engine: Arc::new(PrefetchEngine::new()),
            parallel_reader: Arc::new(ParallelReader::new(16)),  // 16 并行读
        })
    }
    
    /// 高性能随机读 (目标 500K ops/s)
    pub async fn get_optimized(&self, key: &Key) -> Result<Option<Value>> {
        // Level 1: 热缓存 (内存, <100ns)
        if let Some(value) = self.hot_cache.get(key) {
            return Ok(Some(value.clone()));
        }
        
        // Level 2: Bloom 过滤器快速排除不存在的 key (<1μs)
        if !self.bloom_filter.contains(key) {
            return Ok(None);
        }
        
        // Level 3: RocksDB 块缓存 (~10μs)
        let value = self.db.get(key)?;
        
        // 更新热缓存
        if let Some(ref v) = value {
            self.hot_cache.insert(key.clone(), v.clone());
        }
        
        Ok(value)
    }
    
    /// 批量并行读 (目标 500K ops/s)
    pub async fn multi_get_parallel(&self, keys: Vec<Key>) -> Result<Vec<Option<Value>>> {
        // 1. 拆分为热缓存命中和未命中
        let mut cached = Vec::new();
        let mut uncached_keys = Vec::new();
        let mut uncached_indices = Vec::new();
        
        for (i, key) in keys.iter().enumerate() {
            if let Some(value) = self.hot_cache.get(key) {
                cached.push((i, Some(value.clone())));
            } else {
                uncached_keys.push(key.clone());
                uncached_indices.push(i);
            }
        }
        
        // 2. 并行读取未缓存的 keys (16 线程并行)
        let uncached_values = self.parallel_reader
            .read_batch(&self.db, uncached_keys)
            .await?;
        
        // 3. 合并结果
        let mut result = vec![None; keys.len()];
        for (i, value) in cached {
            result[i] = value;
        }
        for (i, value) in uncached_indices.into_iter().zip(uncached_values.into_iter()) {
            result[i] = value;
        }
        
        Ok(result)
    }
    
    /// 高性能随机写 (目标 300K ops/s)
    pub async fn put_optimized(&self, key: Key, value: Value) -> Result<()> {
        // 1. 立即更新热缓存 (保证读一致性)
        self.hot_cache.insert(key.clone(), value.clone());
        
        // 2. 添加到写缓冲 (异步刷盘)
        {
            let mut buffer = self.write_buffer.lock().await;
            buffer.put(&key, &value);
            
            // 3. 达到阈值时批量刷盘 (1000 条或 1MB)
            if buffer.len() >= 1000 || buffer.size_in_bytes() >= 1_000_000 {
                let batch = std::mem::replace(&mut *buffer, WriteBatch::default());
                drop(buffer);  // 释放锁
                
                // 异步刷盘 (不阻塞后续写入)
                let db = self.db.clone();
                tokio::spawn(async move {
                    if let Err(e) = db.write(batch) {
                        eprintln!("Failed to flush write batch: {}", e);
                    }
                });
            }
        }
        
        // 4. 更新 Bloom 过滤器
        self.bloom_filter.insert(&key);
        
        Ok(())
    }
    
    /// 超高性能批量写 (目标 1M ops/s)
    pub async fn batch_write_optimized(&self, entries: Vec<(Key, Value)>) -> Result<()> {
        let start = Instant::now();
        
        // 1. 批量更新热缓存 (并行)
        entries.par_iter().for_each(|(k, v)| {
            self.hot_cache.insert(k.clone(), v.clone());
        });
        
        // 2. 构建大批量写入
        let mut batch = WriteBatch::default();
        for (key, value) in entries.iter() {
            batch.put(key, value);
            self.bloom_filter.insert(key);
        }
        
        // 3. 启用 Pipeline 模式批量写入
        let mut write_opts = WriteOptions::default();
        write_opts.set_sync(false);        // 异步刷盘
        write_opts.disable_wal(false);     // 保留 WAL (保证持久性)
        
        self.db.write_opt(batch, &write_opts)?;
        
        let duration = start.elapsed();
        let throughput = entries.len() as f64 / duration.as_secs_f64();
        
        if throughput < 1_000_000.0 {
            eprintln!("Warning: Batch write throughput {} ops/s < target 1M ops/s", 
                      throughput as usize);
        }
        
        Ok(())
    }
    
    /// 智能预取 (减少随机读延迟)
    pub async fn prefetch_related_keys(&self, key: &Key) -> Result<()> {
        // 1. 预测可能访问的相关 keys (基于访问模式)
        let related_keys = self.prefetch_engine.predict_next_keys(key).await?;
        
        // 2. 后台批量预取到缓存
        let db = self.db.clone();
        let hot_cache = self.hot_cache.clone();
        
        tokio::spawn(async move {
            for key in related_keys {
                if let Ok(Some(value)) = db.get(&key) {
                    hot_cache.insert(key, value);
                }
            }
        });
        
        Ok(())
    }
}
```

### 并行读取器

```rust
/// 并行读取器
pub struct ParallelReader {
    thread_pool: ThreadPool,
}

impl ParallelReader {
    pub fn new(num_threads: usize) -> Self {
        Self {
            thread_pool: rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        }
    }
    
    /// 并行读取多个 keys
    pub async fn read_batch(
        &self,
        db: &DB,
        keys: Vec<Key>,
    ) -> Result<Vec<Option<Value>>> {
        let (tx, rx) = tokio::sync::mpsc::channel(keys.len());
        
        // 将 keys 分配到线程池并行读取
        let db = Arc::new(db.clone());
        self.thread_pool.scope(|s| {
            for (i, key) in keys.into_iter().enumerate() {
                let db = db.clone();
                let tx = tx.clone();
                s.spawn(move |_| {
                    let value = db.get(&key).ok().flatten();
                    let _ = tx.blocking_send((i, value));
                });
            }
        });
        
        drop(tx);  // 关闭发送端
        
        // 收集结果
        let mut results: Vec<(usize, Option<Value>)> = Vec::new();
        let mut rx = rx;
        while let Some((i, value)) = rx.recv().await {
            results.push((i, value));
        }
        
        // 按原始顺序排序
        results.sort_by_key(|(i, _)| *i);
        Ok(results.into_iter().map(|(_, v)| v).collect())
    }
}
```

### 预取引擎 (机器学习)

```rust
/// 预取引擎 (机器学习预测下一次访问)
pub struct PrefetchEngine {
    access_history: Arc<Mutex<VecDeque<Key>>>,
    pattern_model: Arc<Mutex<HashMap<Key, Vec<Key>>>>,  // key → 后续访问的 keys
}

impl PrefetchEngine {
    pub fn new() -> Self {
        Self {
            access_history: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            pattern_model: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// 预测下一次可能访问的 keys
    pub async fn predict_next_keys(&self, current_key: &Key) -> Result<Vec<Key>> {
        let model = self.pattern_model.lock().await;
        
        Ok(model.get(current_key)
            .cloned()
            .unwrap_or_default())
    }
    
    /// 记录访问并更新模型
    pub async fn record_access(&self, key: Key) -> Result<()> {
        let mut history = self.access_history.lock().await;
        
        // 记录当前访问
        if history.len() >= 1000 {
            history.pop_front();
        }
        history.push_back(key.clone());
        
        // 更新访问模式 (当前 key 后经常访问的 keys)
        if history.len() >= 2 {
            let prev_key = history[history.len() - 2].clone();
            let mut model = self.pattern_model.lock().await;
            
            model.entry(prev_key)
                .or_insert_with(Vec::new)
                .push(key);
        }
        
        Ok(())
    }
}
```

---

## 📈 性能对比总结

### 完整性能指标

| 指标 | Phase 4.2 原计划 | 优化后目标 | 提升倍数 | 关键技术 |
|-----|----------------|----------|---------|---------|
| 随机读 | 100K ops/s | **500K ops/s** | 5× | 热缓存 + Bloom + 并行读 |
| 随机写 | 50K ops/s | **300K ops/s** | 6× | Write batching + 异步刷盘 |
| 批量写 | 200K ops/s | **1M ops/s** | 5× | 大批量 + Pipeline |
| 扫描 | 500 MB/s | **2 GB/s** | 4× | Prefetch + 零拷贝 |
| 延迟 P99 | 10 ms | **2 ms** | 5× | 全内存热数据 |
| 存储成本 | 基准 | **-70%** | - | 三温分级 + 高压缩 |
| 负载均衡 | 手动 | **自动** | - | 智能迁移 + 监控 |

### 实际应用场景性能

| 场景 | 操作类型 | 原性能 | 优化后 | 用户体验提升 |
|-----|---------|-------|-------|------------|
| **NFT 查询** | 随机读 | 100K/s | 500K/s | 页面加载 10ms → 2ms |
| **批量转账** | 批量写 | 200K/s | 1M/s | 1000 笔 5s → 1s |
| **DeFi 交易** | 随机读写 | 混合 | 3-6× | 交易确认 50ms → 10ms |
| **区块扫描** | 顺序读 | 500MB/s | 2GB/s | 100GB 扫描 200s → 50s |
| **历史归档** | 冷数据 | - | -70% 成本 | 长期存储可承受 |

---

## 🎯 实施优先级与时间表

### 优化技术优先级矩阵

| 优化技术 | 性能提升 | 实现复杂度 | 优先级 | 预计周期 | 依赖项 |
|---------|---------|-----------|--------|---------|--------|
| 热数据全内存缓存 | 读 5×, 延迟 10× | 低 | 🔴 高 | 3天 | 无 |
| Write Batching + 异步刷盘 | 写 6× | 中 | 🔴 高 | 5天 | 无 |
| Bloom Filter | 读 2× (避免无效查询) | 低 | 🔴 高 | 2天 | 无 |
| 并行读取 (16线程) | 批量读 4× | 中 | 🟡 中 | 1周 | Rayon |
| 智能预取 | 延迟 -50% | 高 | 🟡 中 | 2周 | 访问统计 |
| 三温存储分级 | 成本 -70% | 高 | 🟢 低 | 2周 | 数据分类器 |
| 自动数据迁移 | 负载均衡 +80% | 高 | 🟢 低 | 3周 | 存储编排器 |

### 分阶段实施计划

#### **Week 1-2: 快速见效阶段**

**目标**: 3-4× 性能提升

**任务**:
- ✅ 实现 DashMap 热数据缓存
- ✅ 实现 WriteBatch 累积刷盘
- ✅ 集成 Bloom Filter
- ✅ RocksDB 配置优化 (Hot tier)

**预期结果**:
- 读性能: 100K → 300K ops/s
- 写性能: 50K → 200K ops/s
- 延迟: 10ms → 4ms

**验收标准**:
```bash
cargo bench --bench optimized_rocksdb_bench
# 随机读 >= 300K ops/s
# 随机写 >= 200K ops/s
# P99 延迟 <= 5ms
```

#### **Week 3-4: 性能优化阶段**

**目标**: 5-6× 性能提升

**任务**:
- ✅ 实现 ParallelReader (16 线程)
- ✅ 配置三温存储 (Hot/Warm/Cold)
- ✅ 实现 DataClassifier (访问统计)
- ✅ 集成 Zstd 压缩

**预期结果**:
- 读性能: 300K → 500K ops/s
- 写性能: 200K → 300K ops/s
- 批量写: 200K → 800K ops/s
- 成本: 基准 → -50%

**验收标准**:
```bash
cargo bench --bench parallel_read_bench
# 批量读 (1000 keys) <= 2ms
# 批量写 (10K entries) <= 10ms
# 冷数据压缩比 >= 3:1
```

#### **Week 5-6: 长期优化阶段**

**目标**: 智能化 + 自动化

**任务**:
- ✅ 实现 PrefetchEngine (机器学习)
- ✅ 实现 DataMigrationManager (自动迁移)
- ✅ 实现 StorageOrchestrator (全局调度)
- ✅ 集成 Prometheus + Grafana 监控

**预期结果**:
- 延迟: 4ms → 2ms (预取生效)
- 负载均衡: 手动 → 自动 (不均衡度 < 20%)
- 成本: -50% → -70% (三温生效)
- 运维: 被动 → 主动 (自动迁移)

**验收标准**:
```bash
# 1. 预取命中率测试
cargo test --test prefetch_accuracy_test
# 命中率 >= 60%

# 2. 自动迁移测试
cargo test --test auto_migration_test
# 磁盘使用率不均衡度 <= 20%

# 3. 端到端性能测试
cargo bench --bench e2e_storage_bench
# 随机读 >= 500K ops/s
# 批量写 >= 1M ops/s
# P99 延迟 <= 2ms
```

---

## 🔍 监控与可观测性

### Prometheus 指标

```rust
// src/node-core/src/storage/metrics.rs

pub struct StorageMetrics {
    // 读写性能
    pub read_ops_total: Counter,
    pub write_ops_total: Counter,
    pub read_latency: Histogram,
    pub write_latency: Histogram,
    
    // 缓存命中率
    pub cache_hits_total: Counter,
    pub cache_misses_total: Counter,
    pub bloom_filter_hits: Counter,
    pub bloom_filter_misses: Counter,
    
    // 存储分级
    pub hot_tier_size_bytes: Gauge,
    pub warm_tier_size_bytes: Gauge,
    pub cold_tier_size_bytes: Gauge,
    pub tier_migrations_total: Counter,
    
    // 数据迁移
    pub migrations_in_progress: Gauge,
    pub migrations_success_total: Counter,
    pub migrations_failure_total: Counter,
    pub migration_throughput_mbps: Gauge,
    
    // 负载均衡
    pub node_disk_usage_percent: GaugeVec,  // label: node_id
    pub node_cpu_usage_percent: GaugeVec,   // label: node_id
    pub load_imbalance_score: Gauge,
}
```

### Grafana 仪表盘

**核心面板**:
1. **性能总览**: 读写 OPS, 延迟 P50/P99, 吞吐量
2. **缓存效率**: 热缓存命中率, Bloom 过滤率, 预取命中率
3. **存储分级**: Hot/Warm/Cold 占比, 迁移趋势, 成本节省
4. **负载均衡**: 节点负载分布, 不均衡度, 迁移任务队列
5. **告警**: 磁盘将满, 节点过载, 迁移失败, 性能下降

---

## 📚 参考文档

- [四层网络硬件部署与算力调度](./四层网络硬件部署与算力调度.md)
- [SuperVM 与数据库的关系](./SuperVM与数据库的关系.md)
- [RocksDB 性能调优指南](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide)
- [Phase 4.2 持久化存储集成](../../ROADMAP.md#phase-42-持久化存储集成)
