# SuperVM 四层网络硬件部署与算力调度方案

> **作者**: KING XU (CHINA) | **创建时间**: 2025-11-06

---

## 📋 目录

1. [核心理念](#核心理念)
2. [四层硬件规格](#四层硬件规格)
3. [内核安装与适配](#内核安装与适配)
4. [任务分工机制](#任务分工机制)
5. [存储分层管理](#存储分层管理)
6. [算力调度策略](#算力调度策略)
7. [**神经网络路由寻址系统**](#神经网络路由寻址系统) ⭐ **最新**
8. [实施路线图](#实施路线图)

---

## 🎯 核心理念

### SuperVM 分布式架构哲学

```
传统区块链:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
所有节点运行相同软件,执行相同任务
❌ 浪费资源 (高性能服务器做简单查询)
❌ 无法扩展 (受限于最弱节点)
❌ 成本高昂 (所有节点需高端硬件)

SuperVM 四层网络:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
根据硬件能力,自动分配不同任务
✅ 资源优化 (充分利用每个节点的能力)
✅ 水平扩展 (弱节点处理简单任务)
✅ 成本降低 (不需要所有节点都是高配)
✅ 全网协同 (任务自动路由到合适节点)
```

### 设计原则

1. **一核多态**: 同一 SuperVM 内核,根据硬件自动调整功能
2. **任务分层**: 复杂任务(共识/ZK)→强节点,简单任务(查询/转发)→弱节点
3. **存储分级**: 全量状态→L1,部分状态→L2,热数据→L3,本地缓存→L4
4. **算力池化**: 所有节点贡献算力,系统智能调度
5. **自动降级**: 硬件不足时自动降级功能(完整节点→轻节点)

---

## 🖥️ 四层硬件规格

### L1: 超算节点 (Supercomputing Nodes)

**角色**: 共识参与者、完整状态存储、复杂计算

#### 硬件要求

```yaml
最低配置:
  CPU: 32 核心 (Intel Xeon Silver / AMD EPYC)
  RAM: 128 GB DDR4
  存储: 2 TB NVMe SSD (RocksDB)
  网络: 10 Gbps
  GPU: 无 (可选)

推荐配置:
  CPU: 64-128 核心 (Intel Xeon Platinum / AMD EPYC 9654)
  RAM: 512 GB - 1 TB DDR5
  存储: 10 TB NVMe SSD (RAID 0)
  网络: 25-100 Gbps
  GPU: NVIDIA H100 (可选,用于 ZK 加速)

高端配置 (H200 8卡):
  CPU: 2× AMD EPYC 9654 (192 核心)
  RAM: 2 TB DDR5
  存储: 100 TB NVMe SSD
  网络: 100 Gbps
  GPU: 8× NVIDIA H200 (用于 ZK/AI)
```

#### 工作负载

```rust
// L1 节点主要任务
enum L1Task {
    Consensus,              // BFT 共识
    StateValidation,        // 完整状态验证
    BlockProduction,        // 区块生成
    CrossShardSync,         // 跨分片同步
    ZkProofGeneration,      // ZK 证明生成 (可选 GPU)
    ArchiveStorage,         // 历史数据归档
    ComplexQuery,           // 复杂查询 (聚合/分析)
}
```

#### 预期性能

```
TPS: 10-20K (共识受限)
存储: 10-100 TB 全量状态
查询延迟: 10-50 ms
区块时间: 1-3 秒
网络带宽: 1-10 GB/s
算力占用: 50-80% CPU
```

---

### L2: 矿机节点 (Mining Nodes)

**角色**: 交易执行、区块打包、MVCC 并行调度

#### 硬件要求

```yaml
最低配置:
  CPU: 16 核心
  RAM: 64 GB
  存储: 500 GB NVMe SSD
  网络: 1 Gbps
  GPU: 无

推荐配置:
  CPU: 32-64 核心 (高主频)
  RAM: 128-256 GB
  存储: 2 TB NVMe SSD
  网络: 10 Gbps
  GPU: RTX 4090 (可选,用于密码学)

特殊配置 (游戏服务器):
  CPU: 64 核心
  RAM: 512 GB
  存储: 5 TB NVMe SSD
  网络: 25 Gbps
  GPU: RTX 4090 × 2 (用于游戏渲染/AI)
```

#### 工作负载

```rust
// L2 节点主要任务
enum L2Task {
    TxExecution,            // 交易执行 (MVCC)
    TxValidation,           // 交易验证
    MempoolManagement,      // 交易池管理
    BlockBuilding,          // 区块构建
    StateUpdates,           // 状态更新
    EventEmission,          // 事件发送
    LoadBalancing,          // 负载均衡
    
    // 游戏场景专用
    GameStateUpdate,        // 游戏状态更新
    PhysicsSimulation,      // 物理模拟
    AIComputation,          // AI 计算
}
```

#### 预期性能

```
TPS: 100-200K (MVCC 并行)
存储: 500 GB - 2 TB (最近状态)
查询延迟: 1-5 ms
区块打包: < 100 ms
网络带宽: 100 MB - 1 GB/s
算力占用: 70-90% CPU
```

---

### L3: 边缘节点 (Edge Nodes)

**角色**: 区域缓存、交易转发、快速响应

#### 硬件要求

```yaml
最低配置:
  CPU: 4 核心 (ARM/x86)
  RAM: 8 GB
  存储: 100 GB SSD
  网络: 100 Mbps
  GPU: 无

推荐配置:
  CPU: 8-16 核心
  RAM: 16-32 GB
  存储: 256 GB SSD
  网络: 1 Gbps
  GPU: 无

边缘服务器 (企业/ISP):
  CPU: 16 核心
  RAM: 64 GB
  存储: 1 TB SSD
  网络: 10 Gbps
  GPU: 无
```

#### 工作负载

```rust
// L3 节点主要任务
enum L3Task {
    RegionalCache,          // 区域缓存 (LRU)
    TxRouting,              // 交易路由
    TxForwarding,           // 交易转发
    QueryResponse,          // 查询响应
    StateSync,              // 状态同步
    UserSession,            // 用户会话管理
    
    // CDN 功能
    AssetCaching,           // 资产缓存 (NFT/图片)
    ContentDelivery,        // 内容分发
}
```

#### 预期性能

```
TPS: 1M+ (缓存命中)
存储: 100 GB - 1 TB (热数据)
查询延迟: < 10 ms
缓存命中率: 80-95%
网络带宽: 10-100 MB/s
算力占用: 20-50% CPU
```

---

### L4: 移动节点 (Mobile/IoT Nodes)

**角色**: 轻客户端、本地缓存、即时反馈、**轻量级路由参与者** ⭐ 新增

#### 硬件要求

```yaml
移动设备 (手机/平板):
  CPU: 4-8 核心 (ARM)
  RAM: 4-8 GB
  存储: 64-256 GB
  网络: 4G/5G/WiFi/蓝牙
  GPU: 集成显卡
  路由缓存: 100-500 节点 (根据内存动态调整)

IoT 设备:
  CPU: 1-2 核心 (ARM Cortex)
  RAM: 512 MB - 2 GB
  存储: 8-64 GB
  网络: WiFi/BLE/LoRa
  GPU: 无
  路由缓存: 50-100 节点 (最小配置)

桌面轻节点:
  CPU: 4 核心
  RAM: 8 GB
  存储: 100 GB
  网络: WiFi/有线
  GPU: 无
  路由缓存: 200-500 节点 (较好配置)
```

#### 工作负载

```rust
// L4 节点主要任务
enum L4Task {
    LocalCache,             // 本地缓存
    TxSigning,              // 交易签名
    TxSubmission,           // 交易提交
    BalanceQuery,           // 余额查询
    EventListening,         // 事件监听
    
    // ⭐ 新增: 轻量级路由参与
    RoutingCache,           // 路由缓存 (100-500 节点)
    RouteRelay,             // 路由中继 (为其他 L4 提供查询)
    NatAssist,              // NAT 穿透协助 (充当 STUN 服务器)
    LocalDiscovery,         // 本地发现 (mDNS/蓝牙)
    
    // 批量操作
    OfflineQueue,           // 离线队列
    BatchSync,              // 批量同步
    
    // 游戏客户端
    LocalStatePredict,      // 本地状态预测
    AssetRendering,         // 资产渲染
}
```

#### 预期性能

```
TPS: 本地操作 (无限制)
存储: 1-10 GB (用户数据)
查询延迟: < 1 ms (本地)
同步周期: 1-10 分钟
网络带宽: 1-10 MB/s
算力占用: 5-20% CPU
电池影响: 最小化
```

---

## 🔧 内核安装与适配

### 统一内核,多重配置

**核心理念**: 同一个 SuperVM 内核二进制,根据硬件自动适配

```rust
// src/node-core/src/main.rs

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 检测硬件能力
    let hardware = HardwareDetector::detect()?;
    
    // 2. 自动决定节点类型
    let node_type = NodeType::auto_detect(&hardware)?;
    
    // 3. 加载对应配置
    let config = Config::load_for_node_type(node_type)?;
    
    // 4. 启动节点
    let node = SuperVMNode::new(hardware, config)?;
    node.start().await?;
    
    Ok(())
}
```

### 硬件检测

```rust
// src/node-core/src/hardware_detector.rs

pub struct HardwareCapability {
    pub cpu_cores: usize,
    pub memory_gb: usize,
    pub disk_gb: usize,
    pub network_mbps: usize,
    pub has_gpu: bool,
    pub gpu_memory_gb: usize,
    pub arch: Architecture,  // x86_64, ARM64, ...
}

impl HardwareDetector {
    pub fn detect() -> Result<HardwareCapability> {
        let cpu_cores = num_cpus::get();
        let memory_gb = Self::detect_memory()?;
        let disk_gb = Self::detect_disk()?;
        let network_mbps = Self::detect_network()?;
        let (has_gpu, gpu_memory_gb) = Self::detect_gpu()?;
        let arch = Self::detect_arch();
        
        Ok(HardwareCapability {
            cpu_cores,
            memory_gb,
            disk_gb,
            network_mbps,
            has_gpu,
            gpu_memory_gb,
            arch,
        })
    }
    
    fn detect_memory() -> Result<usize> {
        #[cfg(target_os = "linux")]
        {
            // 读取 /proc/meminfo
            let content = std::fs::read_to_string("/proc/meminfo")?;
            // 解析 MemTotal
            // ...
        }
        
        #[cfg(target_os = "windows")]
        {
            // 使用 Windows API
            // ...
        }
        
        #[cfg(target_os = "macos")]
        {
            // 使用 sysctl
            // ...
        }
    }
}
```

### 节点类型自动决策

```rust
// src/node-core/src/node_type.rs

#[derive(Debug, Clone, Copy)]
pub enum NodeType {
    L1Supernode,    // 超算节点
    L2Miner,        // 矿机节点
    L3Edge,         // 边缘节点
    L4Mobile,       // 移动节点
}

impl NodeType {
    pub fn auto_detect(hw: &HardwareCapability) -> Result<Self> {
        // 决策树算法
        if hw.cpu_cores >= 32 && hw.memory_gb >= 128 && hw.disk_gb >= 2000 {
            Ok(NodeType::L1Supernode)
        } else if hw.cpu_cores >= 16 && hw.memory_gb >= 64 && hw.disk_gb >= 500 {
            Ok(NodeType::L2Miner)
        } else if hw.cpu_cores >= 4 && hw.memory_gb >= 8 && hw.disk_gb >= 100 {
            Ok(NodeType::L3Edge)
        } else {
            Ok(NodeType::L4Mobile)
        }
    }
    
    /// 手动指定节点类型 (命令行参数)
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "l1" | "supernode" => Ok(NodeType::L1Supernode),
            "l2" | "miner" => Ok(NodeType::L2Miner),
            "l3" | "edge" => Ok(NodeType::L3Edge),
            "l4" | "mobile" => Ok(NodeType::L4Mobile),
            _ => Err(anyhow!("Unknown node type: {}", s)),
        }
    }
}
```

### 配置文件结构

```toml
# config/l1_supernode.toml

[node]
type = "L1Supernode"
name = "supernode-asia-01"
region = "Asia/Shanghai"

[hardware]
cpu_cores = 64
memory_gb = 512
disk_gb = 10000
network_mbps = 25000

[consensus]
enable = true
algorithm = "BFT"
validators = 100
block_time_ms = 2000

[storage]
backend = "RocksDB"
path = "/data/supervm/state"
cache_gb = 64
enable_pruning = false  # 保留完整历史

[execution]
parallel = true
mvcc = true
max_tps = 20000

[network]
listen = "0.0.0.0:9000"
peers_l1 = ["supernode-us-01:9000", "supernode-eu-01:9000"]
peers_l2 = []  # 不直接连接 L2
```

```toml
# config/l2_miner.toml

[node]
type = "L2Miner"
name = "miner-01"
region = "Asia/Shanghai"

[hardware]
cpu_cores = 32
memory_gb = 128
disk_gb = 2000

[consensus]
enable = false  # L2 不参与共识

[storage]
backend = "RocksDB"
path = "/data/supervm/state"
cache_gb = 16
enable_pruning = true
prune_keep_blocks = 10000  # 保留最近 10000 区块

[execution]
parallel = true
mvcc = true
max_tps = 200000

[network]
listen = "0.0.0.0:9001"
peers_l1 = ["supernode-asia-01:9000"]  # 连接到 L1
peers_l2 = ["miner-02:9001", "miner-03:9001"]  # P2P 网络
peers_l3 = []  # 监听 L3 连接
```

```toml
# config/l3_edge.toml

[node]
type = "L3Edge"
name = "edge-shanghai"
region = "Asia/Shanghai"

[hardware]
cpu_cores = 8
memory_gb = 16
disk_gb = 256

[consensus]
enable = false

[storage]
backend = "LRU"  # 仅内存缓存
cache_gb = 4
enable_pruning = true
prune_keep_blocks = 1000

[execution]
parallel = false  # L3 不执行交易,仅转发
mvcc = false

[network]
listen = "0.0.0.0:9002"
peers_l2 = ["miner-01:9001"]  # 连接到 L2
peers_l3 = ["edge-beijing:9002", "edge-guangzhou:9002"]
peers_l4 = []  # 监听 L4 连接

[cache]
strategy = "LRU"
max_entries = 100000
ttl_seconds = 3600
prefetch = true  # 预取热点数据
```

```toml
# config/l4_mobile.toml

[node]
type = "L4Mobile"
name = "mobile-client"

[hardware]
cpu_cores = 4
memory_gb = 4
disk_gb = 10

[consensus]
enable = false

[storage]
backend = "SQLite"  # 轻量级数据库
path = "./supervm.db"
cache_mb = 100

[execution]
parallel = false
mvcc = false

[network]
peers_l3 = ["edge-shanghai:9002"]  # 仅连接最近的 L3
sync_interval_seconds = 60  # 每分钟同步一次
batch_size = 100  # 批量操作

[offline]
enable_queue = true
max_queue_size = 1000
```

### 一键安装脚本

```bash
#!/bin/bash
# install.sh - SuperVM 自动安装脚本

echo "🚀 SuperVM 安装向导"
echo "===================="

# 1. 检测操作系统
OS=$(uname -s)
ARCH=$(uname -m)
echo "检测到系统: $OS $ARCH"

# 2. 检测硬件
CPU_CORES=$(nproc)
MEMORY_GB=$(($(free -g | awk '/^Mem:/{print $2}')))
DISK_GB=$(($(df -BG / | tail -1 | awk '{print $4}' | tr -d 'G')))

echo "硬件配置:"
echo "  CPU 核心: $CPU_CORES"
echo "  内存: ${MEMORY_GB} GB"
echo "  磁盘: ${DISK_GB} GB"

# 3. 自动推荐节点类型
if [ $CPU_CORES -ge 32 ] && [ $MEMORY_GB -ge 128 ]; then
    RECOMMENDED="L1 超算节点"
    NODE_TYPE="l1"
elif [ $CPU_CORES -ge 16 ] && [ $MEMORY_GB -ge 64 ]; then
    RECOMMENDED="L2 矿机节点"
    NODE_TYPE="l2"
elif [ $CPU_CORES -ge 4 ] && [ $MEMORY_GB -ge 8 ]; then
    RECOMMENDED="L3 边缘节点"
    NODE_TYPE="l3"
else
    RECOMMENDED="L4 移动节点"
    NODE_TYPE="l4"
fi

echo ""
echo "推荐节点类型: $RECOMMENDED"
read -p "是否接受推荐? (Y/n): " ACCEPT

if [ "$ACCEPT" != "n" ]; then
    echo "将安装 $RECOMMENDED"
else
    echo "请选择节点类型:"
    echo "  1) L1 超算节点"
    echo "  2) L2 矿机节点"
    echo "  3) L3 边缘节点"
    echo "  4) L4 移动节点"
    read -p "选择 (1-4): " CHOICE
    
    case $CHOICE in
        1) NODE_TYPE="l1" ;;
        2) NODE_TYPE="l2" ;;
        3) NODE_TYPE="l3" ;;
        4) NODE_TYPE="l4" ;;
        *) echo "无效选择"; exit 1 ;;
    esac
fi

# 4. 下载二进制
echo ""
echo "下载 SuperVM 二进制..."
DOWNLOAD_URL="https://github.com/XujueKing/SuperVM/releases/latest/download/supervm-${OS}-${ARCH}"
wget -O /usr/local/bin/supervm "$DOWNLOAD_URL"
chmod +x /usr/local/bin/supervm

# 5. 下载配置文件
echo "下载配置文件..."
CONFIG_URL="https://github.com/XujueKing/SuperVM/releases/latest/download/config-${NODE_TYPE}.toml"
mkdir -p ~/.supervm
wget -O ~/.supervm/config.toml "$CONFIG_URL"

# 6. 初始化数据目录
echo "初始化数据目录..."
mkdir -p ~/.supervm/data
mkdir -p ~/.supervm/logs

# 7. 创建 systemd 服务 (Linux)
if [ "$OS" = "Linux" ]; then
    echo "创建 systemd 服务..."
    cat > /etc/systemd/system/supervm.service <<EOF
[Unit]
Description=SuperVM Node
After=network.target

[Service]
Type=simple
User=$USER
ExecStart=/usr/local/bin/supervm --config ~/.supervm/config.toml
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable supervm
    
    echo "启动 SuperVM 节点..."
    systemctl start supervm
    
    echo ""
    echo "✅ 安装完成!"
    echo "查看状态: systemctl status supervm"
    echo "查看日志: journalctl -u supervm -f"
else
    echo ""
    echo "✅ 安装完成!"
    echo "启动节点: supervm --config ~/.supervm/config.toml"
fi
```

---

## 🎯 任务分工机制

### 智能任务路由

```rust
// src/node-core/src/task_router.rs

pub struct TaskRouter {
    local_capability: HardwareCapability,
    node_type: NodeType,
    peers: Vec<PeerNode>,
}

pub struct PeerNode {
    pub id: NodeId,
    pub node_type: NodeType,
    pub capability: HardwareCapability,
    pub load: f64,  // 0.0-1.0
    pub latency_ms: u64,
}

impl TaskRouter {
    /// 决定任务应该在哪里执行
    pub async fn route_task(&self, task: Task) -> TaskDestination {
        match task {
            // 本地可处理的任务
            Task::SimpleQuery(_) if self.can_handle_locally(&task) => {
                TaskDestination::Local
            }
            
            // 需要转发到更强节点
            Task::ZkProof(_) if self.node_type != NodeType::L1Supernode => {
                let best_l1 = self.find_best_peer(NodeType::L1Supernode);
                TaskDestination::Remote(best_l1)
            }
            
            // 需要分布式执行
            Task::LargeComputation(_) => {
                let workers = self.find_available_workers();
                TaskDestination::Distributed(workers)
            }
            
            _ => TaskDestination::Local,
        }
    }
    
    fn can_handle_locally(&self, task: &Task) -> bool {
        match (self.node_type, task) {
            (NodeType::L1Supernode, _) => true,  // L1 可处理所有任务
            (NodeType::L2Miner, Task::TxExecution(_)) => true,
            (NodeType::L3Edge, Task::Query(_)) => true,
            (NodeType::L4Mobile, Task::LocalOp(_)) => true,
            _ => false,
        }
    }
}
```

### 任务类型定义

```rust
// src/node-core/src/task.rs

#[derive(Debug, Clone)]
pub enum Task {
    // L1 专属任务
    Consensus(ConsensusTask),
    ZkProof(ZkProofTask),
    StateValidation(StateValidationTask),
    
    // L2 专属任务
    TxExecution(TxExecutionTask),
    BlockBuilding(BlockBuildingTask),
    StateUpdate(StateUpdateTask),
    
    // L3 专属任务
    Query(QueryTask),
    TxForwarding(TxForwardingTask),
    CacheUpdate(CacheUpdateTask),
    
    // L4 专属任务
    LocalOp(LocalOpTask),
    TxSigning(TxSigningTask),
    
    // 跨层任务
    LargeComputation(LargeComputationTask),
    DataSync(DataSyncTask),
}

impl Task {
    /// 任务的计算复杂度 (0-100)
    pub fn complexity(&self) -> u8 {
        match self {
            Task::Consensus(_) => 90,
            Task::ZkProof(_) => 95,
            Task::TxExecution(_) => 60,
            Task::Query(_) => 20,
            Task::LocalOp(_) => 10,
            _ => 50,
        }
    }
    
    /// 任务需要的最低节点类型
    pub fn required_node_type(&self) -> NodeType {
        match self {
            Task::Consensus(_) | Task::ZkProof(_) => NodeType::L1Supernode,
            Task::TxExecution(_) | Task::BlockBuilding(_) => NodeType::L2Miner,
            Task::Query(_) | Task::TxForwarding(_) => NodeType::L3Edge,
            _ => NodeType::L4Mobile,
        }
    }
}
```

### 负载均衡

```rust
// src/node-core/src/load_balancer.rs

pub struct LoadBalancer {
    nodes: DashMap<NodeId, NodeInfo>,
}

pub struct NodeInfo {
    pub node_type: NodeType,
    pub current_load: AtomicU8,  // 0-100
    pub queue_length: AtomicUsize,
    pub last_heartbeat: AtomicU64,
}

impl LoadBalancer {
    /// 选择最佳节点执行任务
    pub fn select_node(&self, task: &Task) -> Option<NodeId> {
        let required_type = task.required_node_type();
        
        // 1. 过滤符合条件的节点
        let candidates: Vec<_> = self.nodes
            .iter()
            .filter(|n| n.node_type >= required_type)
            .filter(|n| n.current_load.load(Ordering::Relaxed) < 80)
            .collect();
        
        if candidates.is_empty() {
            return None;
        }
        
        // 2. 计算每个节点的得分
        let mut best_node = None;
        let mut best_score = f64::NEG_INFINITY;
        
        for node in candidates {
            let score = self.calculate_score(node, task);
            if score > best_score {
                best_score = score;
                best_node = Some(*node.key());
            }
        }
        
        best_node
    }
    
    fn calculate_score(&self, node: &NodeInfo, task: &Task) -> f64 {
        let load = node.current_load.load(Ordering::Relaxed) as f64 / 100.0;
        let queue = node.queue_length.load(Ordering::Relaxed) as f64;
        
        // 得分 = 能力 - 负载 - 队列
        let capability = match node.node_type {
            NodeType::L1Supernode => 1.0,
            NodeType::L2Miner => 0.7,
            NodeType::L3Edge => 0.4,
            NodeType::L4Mobile => 0.1,
        };
        
        capability - (load * 0.5) - (queue * 0.01)
    }
}
```

---

## 💾 存储分层管理

### 四层存储策略

```
L1: 完整状态 (100%)
├── RocksDB (10-100 TB)
├── 所有历史区块
├── 所有历史交易
└── 所有状态变更

L2: 部分状态 (最近 N 个区块)
├── RocksDB (500 GB - 2 TB)
├── 最近 10000 区块
├── 活跃账户状态
└── 定期从 L1 裁剪

L3: 热点数据 (高频访问)
├── LRU Cache (100 GB - 1 TB)
├── 热门账户余额
├── NFT 元数据
└── 游戏实时状态

L4: 本地缓存 (用户专属)
├── SQLite (1-10 GB)
├── 用户账户
├── 最近交易
└── 离线队列
```

### 状态同步协议

```rust
// src/node-core/src/state_sync.rs

pub struct StateSyncProtocol {
    local_node_type: NodeType,
    peers: HashMap<NodeType, Vec<PeerConnection>>,
}

impl StateSyncProtocol {
    /// L4 → L3 同步
    pub async fn sync_l4_to_l3(&self, user_data: UserData) -> Result<()> {
        let l3_peer = self.find_nearest_l3()?;
        
        // 1. 批量提交交易
        if user_data.pending_txs.len() > 0 {
            l3_peer.batch_submit(user_data.pending_txs).await?;
        }
        
        // 2. 获取最新状态
        let latest_state = l3_peer.query_user_state(user_data.address).await?;
        
        // 3. 更新本地缓存
        self.update_local_cache(latest_state)?;
        
        Ok(())
    }
    
    /// L3 → L2 同步
    pub async fn sync_l3_to_l2(&self, cache_miss: Vec<Key>) -> Result<()> {
        let l2_peer = self.find_best_l2()?;
        
        // 1. 批量查询缺失数据
        let data = l2_peer.batch_query(cache_miss).await?;
        
        // 2. 更新 L3 缓存
        self.update_cache(data)?;
        
        Ok(())
    }
    
    /// L2 → L1 同步
    pub async fn sync_l2_to_l1(&self, block: Block) -> Result<()> {
        let l1_peer = self.find_l1_validator()?;
        
        // 1. 提交区块到 L1 共识
        l1_peer.submit_block(block).await?;
        
        // 2. 等待确认
        let confirmed = l1_peer.wait_confirmation(block.hash()).await?;
        
        // 3. 裁剪旧数据 (如果需要)
        if self.should_prune() {
            self.prune_old_blocks().await?;
        }
        
        Ok(())
    }
}
```

### 智能缓存策略

```rust
// src/node-core/src/cache.rs

pub struct SmartCache {
    lru: LruCache<Key, Value>,
    access_freq: DashMap<Key, AtomicU64>,
    prefetch_enabled: bool,
}

impl SmartCache {
    /// 预取热点数据
    pub async fn prefetch_hot_data(&self) -> Result<()> {
        if !self.prefetch_enabled {
            return Ok(());
        }
        
        // 1. 分析访问频率
        let hot_keys: Vec<_> = self.access_freq
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
            .collect();
        
        // 2. 按频率排序
        let mut hot_keys = hot_keys;
        hot_keys.sort_by(|a, b| b.1.cmp(&a.1));
        
        // 3. 预取 Top 1000
        let top_keys: Vec<_> = hot_keys.iter().take(1000).map(|(k, _)| k.clone()).collect();
        let data = self.batch_fetch_from_upper_layer(top_keys).await?;
        
        // 4. 更新缓存
        for (key, value) in data {
            self.lru.put(key, value);
        }
        
        Ok(())
    }
}
```

---

## ⚡ 算力调度策略

### 全网算力池

```rust
// src/node-core/src/compute_pool.rs

pub struct ComputePool {
    nodes: DashMap<NodeId, ComputeNode>,
    task_queue: Arc<Mutex<VecDeque<ComputeTask>>>,
}

pub struct ComputeNode {
    pub id: NodeId,
    pub node_type: NodeType,
    pub cpu_cores: usize,
    pub available_cores: AtomicUsize,
    pub gpu_available: bool,
    pub current_tasks: DashMap<TaskId, ComputeTask>,
}

impl ComputePool {
    /// 提交计算任务到全网算力池
    pub async fn submit_task(&self, task: ComputeTask) -> Result<TaskId> {
        let task_id = TaskId::new();
        
        // 1. 评估任务需求
        let requirement = task.compute_requirement();
        
        // 2. 查找合适的节点
        let suitable_nodes = self.find_suitable_nodes(&requirement)?;
        
        if suitable_nodes.is_empty() {
            // 无可用节点,加入队列
            self.task_queue.lock().await.push_back(task);
            return Ok(task_id);
        }
        
        // 3. 选择最佳节点 (负载最低)
        let best_node = self.select_best_node(&suitable_nodes);
        
        // 4. 分配任务
        self.assign_task(best_node, task_id, task).await?;
        
        Ok(task_id)
    }
    
    /// 分布式并行计算 (MapReduce)
    pub async fn distributed_compute<T, R>(
        &self,
        data: Vec<T>,
        map_fn: fn(T) -> R,
        reduce_fn: fn(Vec<R>) -> R,
    ) -> Result<R> {
        // 1. 将数据分片
        let chunk_size = (data.len() + self.nodes.len() - 1) / self.nodes.len();
        let chunks: Vec<_> = data.chunks(chunk_size).collect();
        
        // 2. 分发到各节点 (Map 阶段)
        let mut futures = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let node = self.nodes.iter().nth(i % self.nodes.len()).unwrap();
            let future = node.execute_map(chunk, map_fn);
            futures.push(future);
        }
        
        // 3. 等待所有节点完成
        let results = futures::future::join_all(futures).await;
        
        // 4. Reduce 阶段
        let final_result = reduce_fn(results);
        
        Ok(final_result)
    }
}
```

### ZK 证明的 GPU 加速调度

```rust
// src/node-core/src/zk_scheduler.rs

pub struct ZkProofScheduler {
    gpu_nodes: Vec<NodeId>,  // 有 GPU 的 L1 节点
    cpu_fallback: Vec<NodeId>,  // CPU fallback
}

impl ZkProofScheduler {
    /// 调度 ZK 证明任务
    pub async fn schedule_proof(&self, proof_task: ZkProofTask) -> Result<Proof> {
        // 1. 优先尝试 GPU 节点
        if let Some(gpu_node) = self.find_available_gpu_node() {
            match self.submit_to_gpu(gpu_node, proof_task.clone()).await {
                Ok(proof) => return Ok(proof),
                Err(e) => {
                    warn!("GPU proof failed: {}, fallback to CPU", e);
                }
            }
        }
        
        // 2. GPU 不可用或失败,fallback 到 CPU
        let cpu_node = self.find_available_cpu_node()?;
        let proof = self.submit_to_cpu(cpu_node, proof_task).await?;
        
        Ok(proof)
    }
    
    /// 批量 ZK 证明 (充分利用 GPU)
    pub async fn batch_prove(&self, tasks: Vec<ZkProofTask>) -> Result<Vec<Proof>> {
        // 1. 收集所有 GPU 节点
        let gpu_nodes: Vec<_> = self.gpu_nodes
            .iter()
            .filter(|id| self.is_node_available(id))
            .collect();
        
        if gpu_nodes.is_empty() {
            // 无 GPU,全部用 CPU
            return self.cpu_batch_prove(tasks).await;
        }
        
        // 2. 任务分片 (每个 GPU 节点处理一部分)
        let chunk_size = (tasks.len() + gpu_nodes.len() - 1) / gpu_nodes.len();
        
        // 3. 并行提交
        let mut futures = Vec::new();
        for (i, chunk) in tasks.chunks(chunk_size).enumerate() {
            let node = gpu_nodes[i % gpu_nodes.len()];
            let future = self.submit_batch_to_gpu(*node, chunk.to_vec());
            futures.push(future);
        }
        
        // 4. 汇总结果
        let results = futures::future::try_join_all(futures).await?;
        let proofs = results.into_iter().flatten().collect();
        
        Ok(proofs)
    }
}
```

### 动态负载调整

```rust
// src/node-core/src/load_adjuster.rs

pub struct LoadAdjuster {
    metrics: Arc<Mutex<NodeMetrics>>,
}

pub struct NodeMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_io: f64,
    pub network_io: f64,
    pub task_queue_length: usize,
}

impl LoadAdjuster {
    /// 根据负载动态调整节点行为
    pub async fn adjust(&self) -> Result<()> {
        let metrics = self.metrics.lock().await;
        
        // 1. CPU 过载 → 降低并行度
        if metrics.cpu_usage > 0.9 {
            self.reduce_parallelism().await?;
            self.reject_new_tasks().await?;
        }
        
        // 2. 内存不足 → 清理缓存
        if metrics.memory_usage > 0.85 {
            self.clear_cache().await?;
            self.trigger_gc().await?;
        }
        
        // 3. 磁盘 I/O 高 → 限流
        if metrics.disk_io > 0.8 {
            self.throttle_disk_ops().await?;
        }
        
        // 4. 网络拥堵 → 批量传输
        if metrics.network_io > 0.8 {
            self.enable_batch_mode().await?;
        }
        
        // 5. 任务队列积压 → 请求支援
        if metrics.task_queue_length > 1000 {
            self.request_help_from_peers().await?;
        }
        
        Ok(())
    }
}
```

---

## 🌐 神经网络路由寻址系统

### 核心问题

传统 P2P 网络的痛点:
```
❌ 节点发现慢 (DHT 查询需要多跳,延迟高)
❌ NAT 穿透成功率低 (STUN/TURN 成功率 60-70%)
❌ 连接建立慢 (需要多次握手尝试)
❌ 无法感知节点能力 (不知道对方是 L1/L2/L3/L4)
❌ 无法智能路由 (无法根据任务类型选择最佳节点)
```

**SuperVM 解决方案**: 类似 DNS 的分层寻址服务 + 智能路由

### 设计理念

```
传统 DNS:
用户 → 本地 DNS → 根 DNS → TLD DNS → 权威 DNS → IP地址

SuperVM 神经网络寻址:
L4 客户端 → L3 边缘节点 (区域路由表) → L2 矿机 (全局路由表) → L1 超算 (根路由表) → 目标节点

特点:
✅ 分层缓存 (L3 缓存热门节点,L2 缓存全局路由,L1 是权威源)
✅ 就近服务 (L4 优先查询最近的 L3)
✅ 能力感知 (每个节点记录自己的硬件能力和任务类型)
✅ 智能路由 (根据任务复杂度自动选择最佳节点)
✅ 快速穿透 (L3 节点充当 relay,成功率 95%+)
```

### 架构设计

#### 1. 节点 ID 与地址系统

```rust
// src/node-core/src/addressing.rs

use libp2p::PeerId;
use std::net::IpAddr;

/// 节点全局唯一标识符
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct NodeAddress {
    /// libp2p PeerId (基于公钥生成,全局唯一)
    pub peer_id: PeerId,
    
    /// 节点类型 (L1/L2/L3/L4)
    pub node_type: NodeType,
    
    /// 地理位置 (区域代码)
    pub region: Region,
    
    /// 公网地址 (如果有)
    pub public_addrs: Vec<Multiaddr>,
    
    /// 内网地址
    pub private_addrs: Vec<Multiaddr>,
    
    /// NAT 类型
    pub nat_type: NatType,
    
    /// 硬件能力
    pub capability: HardwareCapability,
    
    /// 当前负载 (0-100)
    pub load: u8,
    
    /// 最后心跳时间
    pub last_seen: u64,
}

/// NAT 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    Public,              // 公网 IP
    FullCone,            // 完全锥形 NAT (易穿透)
    RestrictedCone,      // 受限锥形 NAT
    PortRestricted,      // 端口受限锥形 NAT
    Symmetric,           // 对称型 NAT (难穿透)
    Unknown,
}

/// 地理区域
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Region {
    // 亚洲
    AsiaCN,        // 中国
    AsiaJP,        // 日本
    AsiaSG,        // 新加坡
    AsiaKR,        // 韩国
    
    // 北美
    NAWest,        // 北美西部
    NAEast,        // 北美东部
    
    // 欧洲
    EUWest,        // 西欧
    EUEast,        // 东欧
    
    // 其他
    Other,
}

impl Region {
    /// 计算两个区域之间的延迟估计 (ms)
    pub fn latency_to(&self, other: &Region) -> u64 {
        match (self, other) {
            (a, b) if a == b => 5,              // 同区域 5ms
            (Region::AsiaCN, Region::AsiaJP) => 30,
            (Region::AsiaCN, Region::AsiaSG) => 50,
            (Region::AsiaCN, Region::NAWest) => 150,
            (Region::AsiaCN, Region::EUWest) => 200,
            _ => 100,  // 默认跨区域 100ms
        }
    }
}
```

#### 2. 四层路由表

```rust
// src/node-core/src/routing_table.rs

use dashmap::DashMap;
use lru::LruCache;

/// 路由表接口
pub trait RoutingTable: Send + Sync {
    /// 注册节点
    async fn register(&self, node: NodeAddress) -> Result<()>;
    
    /// 查询节点
    async fn lookup(&self, peer_id: &PeerId) -> Option<NodeAddress>;
    
    /// 根据条件查询节点
    async fn find_nodes(&self, filter: NodeFilter) -> Vec<NodeAddress>;
    
    /// 心跳更新
    async fn heartbeat(&self, peer_id: &PeerId, load: u8) -> Result<()>;
    
    /// 删除下线节点
    async fn remove(&self, peer_id: &PeerId) -> Result<()>;
}

/// L1 根路由表 (权威路由表)
pub struct L1RootRoutingTable {
    /// 所有节点的完整信息 (持久化到 RocksDB)
    nodes: Arc<RocksDB>,
    
    /// 内存索引 (PeerId → NodeAddress)
    index: DashMap<PeerId, NodeAddress>,
    
    /// 按区域索引
    region_index: DashMap<Region, Vec<PeerId>>,
    
    /// 按节点类型索引
    type_index: DashMap<NodeType, Vec<PeerId>>,
}

impl L1RootRoutingTable {
    /// 创建根路由表
    pub fn new(db: Arc<RocksDB>) -> Self {
        Self {
            nodes: db,
            index: DashMap::new(),
            region_index: DashMap::new(),
            type_index: DashMap::new(),
        }
    }
    
    /// 从 RocksDB 加载全部节点
    pub async fn load_from_db(&self) -> Result<()> {
        let mut iter = self.nodes.iterator(IteratorMode::Start);
        while let Some((key, value)) = iter.next() {
            let peer_id = PeerId::from_bytes(&key)?;
            let node: NodeAddress = bincode::deserialize(&value)?;
            
            self.index.insert(peer_id, node.clone());
            
            // 更新索引
            self.region_index
                .entry(node.region)
                .or_insert(Vec::new())
                .push(peer_id);
            
            self.type_index
                .entry(node.node_type)
                .or_insert(Vec::new())
                .push(peer_id);
        }
        
        Ok(())
    }
    
    /// 查询最佳节点 (根据任务和区域)
    pub async fn find_best_node(
        &self,
        task_type: TaskType,
        requester_region: Region,
    ) -> Option<NodeAddress> {
        let required_type = task_type.required_node_type();
        
        // 1. 获取符合类型的节点
        let candidates = self.type_index
            .get(&required_type)?
            .value()
            .clone();
        
        // 2. 按距离和负载排序
        let mut scored: Vec<_> = candidates
            .iter()
            .filter_map(|peer_id| self.index.get(peer_id))
            .map(|node| {
                let latency = requester_region.latency_to(&node.region);
                let load = node.load as u64;
                let score = 1000 - latency - load * 5;
                (score, node.clone())
            })
            .collect();
        
        scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        
        scored.first().map(|(_, node)| node.clone())
    }
}

/// L2 全局路由表 (缓存最近 10 万个节点)
pub struct L2GlobalRoutingTable {
    /// LRU 缓存 (最近访问的节点)
    cache: Arc<Mutex<LruCache<PeerId, NodeAddress>>>,
    
    /// 区域索引 (快速查询同区域节点)
    region_cache: DashMap<Region, Vec<PeerId>>,
    
    /// 上游 L1 节点
    l1_nodes: Vec<PeerId>,
}

impl L2GlobalRoutingTable {
    pub fn new(capacity: usize, l1_nodes: Vec<PeerId>) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            region_cache: DashMap::new(),
            l1_nodes,
        }
    }
    
    /// 查询节点 (缓存未命中则查询 L1)
    pub async fn lookup(&self, peer_id: &PeerId) -> Option<NodeAddress> {
        // 1. 查缓存
        if let Some(node) = self.cache.lock().await.get(peer_id) {
            return Some(node.clone());
        }
        
        // 2. 缓存未命中,查询 L1
        let node = self.query_l1(peer_id).await?;
        
        // 3. 更新缓存
        self.cache.lock().await.put(*peer_id, node.clone());
        
        Some(node)
    }
    
    async fn query_l1(&self, peer_id: &PeerId) -> Option<NodeAddress> {
        // 随机选择一个 L1 节点查询
        let l1 = self.l1_nodes.choose(&mut rand::thread_rng())?;
        
        // RPC 查询
        let response = self.rpc_call(l1, "routing.lookup", peer_id).await.ok()?;
        
        Some(response)
    }
}

/// L3 区域路由表 (缓存同区域节点)
pub struct L3RegionalRoutingTable {
    /// 本区域
    local_region: Region,
    
    /// LRU 缓存 (最近访问的 1 万个节点)
    cache: Arc<Mutex<LruCache<PeerId, NodeAddress>>>,
    
    /// 同区域节点列表
    regional_nodes: DashMap<PeerId, NodeAddress>,
    
    /// 上游 L2 节点
    l2_nodes: Vec<PeerId>,
}

impl L3RegionalRoutingTable {
    /// 查询节点 (优先同区域)
    pub async fn lookup(&self, peer_id: &PeerId) -> Option<NodeAddress> {
        // 1. 查同区域节点
        if let Some(node) = self.regional_nodes.get(peer_id) {
            return Some(node.clone());
        }
        
        // 2. 查缓存
        if let Some(node) = self.cache.lock().await.get(peer_id) {
            return Some(node.clone());
        }
        
        // 3. 查询 L2
        let node = self.query_l2(peer_id).await?;
        
        // 4. 更新缓存
        self.cache.lock().await.put(*peer_id, node.clone());
        
        Some(node)
    }
    
    /// 广播本地节点到区域
    pub async fn broadcast_local_node(&self, node: NodeAddress) -> Result<()> {
        if node.region == self.local_region {
            self.regional_nodes.insert(node.peer_id, node.clone());
        }
        
        Ok(())
    }
}

/// L4 本地路由表 (轻量级参与路由)
/// 
/// **设计理念**: L4 移动节点虽然资源有限,但也可以参与网络路由:
/// 1. **临时缓存**: 缓存最近连接的 100-500 个节点 (根据内存动态调整)
/// 2. **路由中继**: 可以为其他 L4 节点提供路由查询服务 (减轻 L3 负载)
/// 3. **穿透协助**: 可以充当 NAT 打洞的协调者 (帮助其他 L4 节点穿透)
/// 4. **P2P 发现**: 可以通过蓝牙/WiFi Direct 在本地网络发现其他 L4 节点
pub struct L4LocalRoutingTable {
    /// 小型 LRU 缓存 (动态大小: 100-500 节点)
    cache: Arc<Mutex<LruCache<PeerId, NodeAddress>>>,
    
    /// 最近连接的 L3 节点 (作为上游路由)
    l3_nodes: Vec<PeerId>,
    
    /// 同局域网的 L4 节点 (通过 mDNS/蓝牙发现)
    local_l4_peers: DashMap<PeerId, NodeAddress>,
    
    /// 是否启用路由中继功能 (默认关闭,节省资源)
    enable_relay: AtomicBool,
    
    /// 是否启用 NAT 协助功能
    enable_nat_assist: AtomicBool,
    
    /// 路由查询统计 (用于判断是否升级为中继节点)
    query_stats: Arc<Mutex<QueryStats>>,
}

#[derive(Default)]
pub struct QueryStats {
    /// 被请求次数
    request_count: u64,
    /// 缓存命中次数
    cache_hit_count: u64,
    /// 上次统计重置时间
    last_reset: u64,
}

impl L4LocalRoutingTable {
    pub fn new(cache_capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(cache_capacity))),
            l3_nodes: Vec::new(),
            local_l4_peers: DashMap::new(),
            enable_relay: AtomicBool::new(false),
            enable_nat_assist: AtomicBool::new(false),
            query_stats: Arc::new(Mutex::new(QueryStats::default())),
        }
    }
    
    /// 根据设备内存自动调整缓存大小
    pub fn auto_adjust_cache_size(available_memory_mb: usize) -> usize {
        match available_memory_mb {
            0..=512 => 50,        // 低内存设备: 50 节点
            513..=1024 => 100,    // 中等设备: 100 节点
            1025..=2048 => 200,   // 较好设备: 200 节点
            2049..=4096 => 300,   // 高端设备: 300 节点
            _ => 500,             // 旗舰设备: 500 节点
        }
    }
    
    /// 查询节点 (多级查找)
    pub async fn lookup(&self, peer_id: &PeerId) -> Option<NodeAddress> {
        // 更新统计
        self.query_stats.lock().await.request_count += 1;
        
        // 1. 查本地缓存
        if let Some(node) = self.cache.lock().await.get(peer_id) {
            self.query_stats.lock().await.cache_hit_count += 1;
            return Some(node.clone());
        }
        
        // 2. 查同局域网的 L4 节点
        if let Some(node) = self.local_l4_peers.get(peer_id) {
            let result = node.clone();
            self.cache.lock().await.put(*peer_id, result.clone());
            return Some(result);
        }
        
        // 3. 询问其他 L4 节点 (如果他们启用了中继)
        if let Some(node) = self.query_peer_l4(peer_id).await {
            self.cache.lock().await.put(*peer_id, node.clone());
            return Some(node);
        }
        
        // 4. 查询上游 L3 节点
        let l3 = self.l3_nodes.first()?;
        let node = self.query_l3(l3, peer_id).await?;
        
        // 5. 更新缓存
        self.cache.lock().await.put(*peer_id, node.clone());
        
        Some(node)
    }
    
    /// 向其他 L4 节点查询 (P2P 协助)
    async fn query_peer_l4(&self, peer_id: &PeerId) -> Option<NodeAddress> {
        // 遍历同局域网的 L4 节点
        for entry in self.local_l4_peers.iter() {
            let peer = entry.value();
            
            // 只查询启用了中继功能的节点
            if !peer.capability.enable_relay {
                continue;
            }
            
            // RPC 查询: "routing.lookup"
            if let Ok(node) = self.rpc_call(&peer.peer_id, "routing.lookup", peer_id).await {
                return Some(node);
            }
        }
        
        None
    }
    
    /// 为其他节点提供路由查询服务 (中继功能)
    pub async fn handle_relay_query(&self, peer_id: &PeerId) -> Option<NodeAddress> {
        // 只有启用中继时才响应
        if !self.enable_relay.load(Ordering::Relaxed) {
            return None;
        }
        
        // 查询本地缓存
        self.cache.lock().await.get(peer_id).cloned()
    }
    
    /// 注册本地发现的 L4 节点
    pub async fn register_local_peer(&self, node: NodeAddress) -> Result<()> {
        self.local_l4_peers.insert(node.peer_id, node.clone());
        
        // 同时加入缓存
        self.cache.lock().await.put(node.peer_id, node);
        
        Ok(())
    }
    
    /// 自动判断是否应该启用中继功能
    pub async fn auto_enable_relay(&self) -> Result<()> {
        let stats = self.query_stats.lock().await;
        
        // 策略: 如果被请求次数 > 100 且缓存命中率 > 50%, 启用中继
        if stats.request_count > 100 {
            let hit_rate = stats.cache_hit_count as f64 / stats.request_count as f64;
            if hit_rate > 0.5 {
                self.enable_relay.store(true, Ordering::Relaxed);
                info!("L4 节点缓存命中率高 ({:.2}%), 自动启用路由中继功能", hit_rate * 100.0);
            }
        }
        
        Ok(())
    }
    
    /// NAT 打洞协助 (为其他 L4 节点提供 STUN 服务)
    pub async fn assist_nat_traversal(
        &self,
        requester: &PeerId,
        target: &PeerId,
    ) -> Result<NatAssistResult> {
        if !self.enable_nat_assist.load(Ordering::Relaxed) {
            return Err(anyhow!("NAT assist disabled"));
        }
        
        // 1. 检查是否知道目标节点
        let target_node = self.cache.lock().await.get(target).cloned()
            .ok_or_else(|| anyhow!("Target not in cache"))?;
        
        // 2. 充当 STUN 服务器,告诉请求者目标的公网地址
        let stun_info = StunInfo {
            target_public_addr: target_node.public_addrs.first().cloned(),
            target_nat_type: target_node.nat_type,
            suggested_strategy: self.suggest_connection_strategy(
                &requester, 
                &target_node
            ),
        };
        
        Ok(NatAssistResult::Success(stun_info))
    }
    
    fn suggest_connection_strategy(
        &self,
        requester: &PeerId,
        target: &NodeAddress,
    ) -> ConnectionStrategy {
        match target.nat_type {
            NatType::Public => ConnectionStrategy::Direct,
            NatType::FullCone | NatType::RestrictedCone => ConnectionStrategy::HolePunch,
            NatType::Symmetric => ConnectionStrategy::NeedRelay,
            NatType::Unknown => ConnectionStrategy::TryAll,
        }
    }
    
    /// 蓝牙/WiFi Direct 本地发现
    pub async fn discover_local_peers(&self) -> Result<Vec<NodeAddress>> {
        let mut discovered = Vec::new();
        
        // 1. mDNS 发现 (WiFi)
        #[cfg(feature = "mdns")]
        {
            let mdns_peers = self.mdns_discover().await?;
            discovered.extend(mdns_peers);
        }
        
        // 2. 蓝牙发现 (移动设备)
        #[cfg(feature = "bluetooth")]
        {
            let bt_peers = self.bluetooth_discover().await?;
            discovered.extend(bt_peers);
        }
        
        // 3. 注册到本地路由表
        for peer in &discovered {
            self.register_local_peer(peer.clone()).await?;
        }
        
        Ok(discovered)
    }
}

#[derive(Debug, Clone)]
pub struct StunInfo {
    pub target_public_addr: Option<Multiaddr>,
    pub target_nat_type: NatType,
    pub suggested_strategy: ConnectionStrategy,
}

#[derive(Debug, Clone)]
pub enum ConnectionStrategy {
    Direct,       // 直接连接
    HolePunch,    // NAT 打洞
    NeedRelay,    // 需要中继
    TryAll,       // 尝试所有方法
}

pub enum NatAssistResult {
    Success(StunInfo),
    TargetNotFound,
    NotSupported,
}
```

#### 3. 智能寻址协议

```rust
// src/node-core/src/addressing_protocol.rs

/// 寻址请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressQuery {
    /// 目标节点 ID (可选,如果为空则根据过滤条件查询)
    pub target: Option<PeerId>,
    
    /// 过滤条件
    pub filter: Option<NodeFilter>,
    
    /// 请求者信息
    pub requester: NodeAddress,
    
    /// 任务类型 (用于智能路由)
    pub task_type: Option<TaskType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeFilter {
    /// 节点类型
    pub node_type: Option<NodeType>,
    
    /// 区域
    pub region: Option<Region>,
    
    /// 最大延迟 (ms)
    pub max_latency: Option<u64>,
    
    /// 最大负载
    pub max_load: Option<u8>,
    
    /// 硬件要求
    pub min_capability: Option<HardwareCapability>,
}

/// 寻址响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressResponse {
    /// 找到的节点
    pub nodes: Vec<NodeAddress>,
    
    /// 建议的连接方式
    pub connection_hints: Vec<ConnectionHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionHint {
    /// 直连 (公网 IP)
    Direct { addr: Multiaddr },
    
    /// NAT 穿透
    HolePunching { 
        public_addr: Multiaddr,
        private_addr: Multiaddr,
        nat_type: NatType,
    },
    
    /// 中继 (通过 L3 节点)
    Relay { 
        relay_node: PeerId,
        relay_addr: Multiaddr,
    },
}

/// 寻址服务
pub struct AddressingService {
    local_address: NodeAddress,
    routing_table: Arc<dyn RoutingTable>,
}

impl AddressingService {
    /// 处理寻址查询
    pub async fn handle_query(&self, query: AddressQuery) -> Result<AddressResponse> {
        // 1. 如果有明确目标,直接查询
        if let Some(target) = query.target {
            if let Some(node) = self.routing_table.lookup(&target).await {
                let hints = self.generate_connection_hints(&node, &query.requester).await;
                return Ok(AddressResponse {
                    nodes: vec![node],
                    connection_hints: hints,
                });
            }
        }
        
        // 2. 如果有过滤条件,查询符合条件的节点
        if let Some(filter) = query.filter {
            let nodes = self.routing_table.find_nodes(filter).await;
            
            // 3. 智能排序 (延迟 + 负载 + 能力)
            let mut scored: Vec<_> = nodes.into_iter()
                .map(|node| {
                    let score = self.calculate_score(&node, &query);
                    (score, node)
                })
                .collect();
            
            scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
            
            // 4. 返回 Top 10
            let top_nodes: Vec<_> = scored.into_iter()
                .take(10)
                .map(|(_, node)| node)
                .collect();
            
            let hints = self.generate_connection_hints_batch(&top_nodes, &query.requester).await;
            
            return Ok(AddressResponse {
                nodes: top_nodes,
                connection_hints: hints,
            });
        }
        
        Err(anyhow!("No target or filter specified"))
    }
    
    fn calculate_score(&self, node: &NodeAddress, query: &AddressQuery) -> u64 {
        let mut score = 1000u64;
        
        // 1. 延迟惩罚
        let latency = query.requester.region.latency_to(&node.region);
        score = score.saturating_sub(latency);
        
        // 2. 负载惩罚
        score = score.saturating_sub(node.load as u64 * 5);
        
        // 3. 能力加分
        if node.capability.cpu_cores >= 64 {
            score += 50;
        }
        if node.capability.has_gpu {
            score += 100;
        }
        
        // 4. NAT 类型加分 (易穿透的优先)
        match node.nat_type {
            NatType::Public => score += 100,
            NatType::FullCone => score += 80,
            NatType::RestrictedCone => score += 60,
            NatType::PortRestricted => score += 40,
            NatType::Symmetric => score += 20,
            NatType::Unknown => {},
        }
        
        score
    }
    
    async fn generate_connection_hints(
        &self,
        target: &NodeAddress,
        requester: &NodeAddress,
    ) -> Vec<ConnectionHint> {
        let mut hints = Vec::new();
        
        // 1. 如果目标有公网 IP,直连
        if target.nat_type == NatType::Public {
            for addr in &target.public_addrs {
                hints.push(ConnectionHint::Direct { addr: addr.clone() });
            }
            return hints;
        }
        
        // 2. 如果双方 NAT 可穿透,尝试打洞
        if self.can_hole_punch(&requester.nat_type, &target.nat_type) {
            hints.push(ConnectionHint::HolePunching {
                public_addr: target.public_addrs.first().cloned().unwrap(),
                private_addr: target.private_addrs.first().cloned().unwrap(),
                nat_type: target.nat_type,
            });
        }
        
        // 3. 否则使用中继
        if let Some(relay) = self.find_relay_node(requester, target).await {
            hints.push(ConnectionHint::Relay {
                relay_node: relay.peer_id,
                relay_addr: relay.public_addrs.first().cloned().unwrap(),
            });
        }
        
        hints
    }
    
    fn can_hole_punch(&self, nat1: &NatType, nat2: &NatType) -> bool {
        match (nat1, nat2) {
            (NatType::Public, _) | (_, NatType::Public) => true,
            (NatType::FullCone, _) | (_, NatType::FullCone) => true,
            (NatType::RestrictedCone, NatType::RestrictedCone) => true,
            (NatType::PortRestricted, NatType::PortRestricted) => true,
            _ => false,  // 对称型 NAT 难以穿透
        }
    }
    
    async fn find_relay_node(
        &self,
        requester: &NodeAddress,
        target: &NodeAddress,
    ) -> Option<NodeAddress> {
        // 查找同区域的 L3 节点作为中继
        let filter = NodeFilter {
            node_type: Some(NodeType::L3Edge),
            region: Some(requester.region),
            max_latency: Some(50),
            max_load: Some(70),
            min_capability: None,
        };
        
        let l3_nodes = self.routing_table.find_nodes(filter).await;
        l3_nodes.into_iter().next()
    }
}
```

#### 4. NAT 穿透增强

```rust
// src/node-core/src/nat_traversal.rs

use stun::client::StunClient;

pub struct NatTraversalService {
    local_addr: SocketAddr,
    stun_servers: Vec<String>,
}

impl NatTraversalService {
    /// 检测 NAT 类型
    pub async fn detect_nat_type(&self) -> Result<NatType> {
        // 1. 使用 STUN 协议检测
        let stun_client = StunClient::new(self.stun_servers[0].clone());
        
        // 测试 1: 获取公网地址
        let public_addr = stun_client.get_mapped_address().await?;
        
        if public_addr.ip() == self.local_addr.ip() {
            return Ok(NatType::Public);  // 公网 IP
        }
        
        // 测试 2: 不同 STUN 服务器返回的地址是否一致
        let public_addr2 = StunClient::new(self.stun_servers[1].clone())
            .get_mapped_address()
            .await?;
        
        if public_addr != public_addr2 {
            return Ok(NatType::Symmetric);  // 对称型 NAT
        }
        
        // 测试 3: 尝试从不同端口连接
        // ...更多检测逻辑
        
        Ok(NatType::FullCone)
    }
    
    /// ICE 协议打洞
    pub async fn ice_hole_punch(
        &self,
        target: &NodeAddress,
        relay: Option<NodeAddress>,
    ) -> Result<Connection> {
        // 1. 收集候选地址 (ICE Candidates)
        let mut candidates = Vec::new();
        
        // 1.1 Host candidate (本地地址)
        candidates.push(Candidate::Host(self.local_addr));
        
        // 1.2 Server reflexive candidate (STUN 地址)
        if let Ok(public_addr) = self.stun_discovery().await {
            candidates.push(Candidate::ServerReflexive(public_addr));
        }
        
        // 1.3 Relay candidate (TURN 地址)
        if let Some(relay_node) = relay {
            if let Ok(relay_addr) = self.turn_allocate(&relay_node).await {
                candidates.push(Candidate::Relay(relay_addr));
            }
        }
        
        // 2. 交换候选地址 (通过信令服务器 or L3 中继)
        let target_candidates = self.exchange_candidates(target, &candidates).await?;
        
        // 3. 连接性检查 (按优先级尝试)
        for local in &candidates {
            for remote in &target_candidates {
                if let Ok(conn) = self.try_connect(local, remote).await {
                    return Ok(conn);
                }
            }
        }
        
        Err(anyhow!("NAT traversal failed"))
    }
}

#[derive(Debug, Clone)]
pub enum Candidate {
    Host(SocketAddr),              // 本地地址
    ServerReflexive(SocketAddr),   // STUN 映射地址
    Relay(SocketAddr),             // TURN 中继地址
}
```

#### 5. 实时寻址性能

```rust
// 性能指标

pub struct AddressingMetrics {
    /// 平均查询延迟
    pub avg_lookup_latency: Duration,
    
    /// 缓存命中率
    pub cache_hit_rate: f64,
    
    /// NAT 穿透成功率
    pub nat_success_rate: f64,
    
    /// 中继使用率
    pub relay_usage_rate: f64,
    
    /// L4 参与度 (启用路由中继的 L4 节点比例)
    pub l4_participation_rate: f64,
}

// 预期性能:
// 
// L4 → L4 查询: < 5 ms (本地缓存/同局域网 P2P) ⭐ 新增
// L4 → L3 查询: < 10 ms (同区域缓存命中)
// L4 → L2 查询: < 50 ms (跨区域查询)
// L4 → L1 查询: < 100 ms (全局查询)
// 
// 缓存命中率:
// L4: 30-40% (本地热点,常用联系人) ⭐ 新增
// L3: 80-90% (区域热点)
// L2: 60-70% (全局热点)
// L1: 100% (权威数据)
// 
// NAT 穿透成功率:
// 无协助: 70-80%
// 有 L4 协助: 85-90% ⭐ 新增
// 有 L3 中继: 95%+
// 
// 连接建立时间:
// L4 P2P直连: < 50 ms (同局域网) ⭐ 新增
// 直连: < 100 ms
// 打洞: < 500 ms
// 中继: < 200 ms
// 
// L4 参与度:
// 高内存设备 (>4GB): 50-70% 启用路由中继
// 中等设备 (2-4GB): 20-30% 启用路由中继
// 低内存设备 (<2GB): 5-10% 启用路由中继
```

#### 6. L4 节点参与路由的创新设计 ⭐ **核心创新**

##### 设计理念: "人人为我,我为人人"

```
传统 P2P 网络:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
轻节点 → 完全依赖强节点 (DHT/中继)
❌ 轻节点是"寄生者",消耗资源但不贡献
❌ 强节点负载过重,容易成为瓶颈
❌ 网络扩展性差 (轻节点越多,负载越重)

SuperVM 四层协同网络:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
L4 轻节点 → 根据能力贡献路由服务
✅ L4 节点也是"贡献者",缓存常用节点
✅ L4 之间可以 P2P 互助 (减轻 L3 负载)
✅ 网络扩展性好 (节点越多,路由越快)
✅ 局域网优化 (WiFi/蓝牙发现,延迟 < 5ms)
```

##### 三级参与模式

```rust
// L4 节点根据硬件能力和使用情况,自动选择参与级别

pub enum L4ParticipationLevel {
    /// 被动模式 (只消费,不贡献)
    /// - 低内存设备 (<1GB)
    /// - 省电模式
    /// - 流量受限
    Passive {
        cache_size: 50,           // 最小缓存
        relay: false,             // 不提供中继
        nat_assist: false,        // 不协助穿透
    },
    
    /// 标准模式 (适度参与)
    /// - 中等设备 (2-4GB)
    /// - 正常使用
    Standard {
        cache_size: 100-200,      // 中等缓存
        relay: true,              // 启用路由中继 (如命中率>50%)
        nat_assist: true,         // 协助 NAT 穿透
        local_discovery: true,    // 本地发现 (WiFi/蓝牙)
    },
    
    /// 积极模式 (主动贡献)
    /// - 高端设备 (>4GB)
    /// - WiFi/充电中
    /// - 无流量限制
    Active {
        cache_size: 300-500,      // 大容量缓存
        relay: true,              // 强制启用中继
        nat_assist: true,         // 积极协助穿透
        local_discovery: true,    // 本地发现
        preload: true,            // 预加载热门节点
    },
}
```

##### L4 本地网络协同

```
场景 1: 同一 WiFi 网络下的多个 L4 设备
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[手机A] ──┐
          ├─→ [本地 mDNS 发现] ──→ 延迟 < 5 ms
[手机B] ──┤                          不消耗流量
          │                          不通过 L3
[平板C] ──┘

优势:
✅ 延迟极低 (< 5 ms, 本地局域网)
✅ 零流量消耗 (WiFi 内网通信)
✅ 减轻 L3 负载 (不需要上传查询)
✅ 隐私增强 (不暴露给上游节点)

场景 2: 蓝牙近距离发现
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[手机A] <─── 蓝牙 (10米范围) ───> [手机B]

适用场景:
- 线下支付/转账 (面对面交易)
- 游戏组队 (本地多人游戏)
- 文件分享 (点对点传输)

优势:
✅ 无需网络 (离线可用)
✅ 零流量
✅ 隐私最佳 (完全点对点)

场景 3: L4 之间的路由互助
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[手机A] 需要连接 [手机X]
   ↓
查询本地缓存 → 未找到
   ↓
查询同局域网的 [手机B] → [手机B] 缓存命中! 返回 [手机X] 地址
   ↓
[手机A] 直接连接 [手机X]

效果:
- [手机A] 不需要查询 L3 (节省 10ms + 流量)
- [手机B] 缓存被利用 (资源不浪费)
- 网络整体负载降低
```

##### L4 NAT 穿透互助

```rust
// 场景: L4-A 需要连接 L4-B, 但都在 NAT 后面

pub async fn l4_assisted_nat_traversal(
    node_a: &L4Node,
    node_b: &L4Node,
    assistant: &L4Node,  // 第三方 L4 节点协助
) -> Result<Connection> {
    // 1. L4-Assistant 充当 STUN 服务器
    let a_public = assistant.detect_peer_address(node_a).await?;
    let b_public = assistant.detect_peer_address(node_b).await?;
    
    // 2. L4-Assistant 告诉双方对方的公网地址
    assistant.send_stun_info(node_a, b_public).await?;
    assistant.send_stun_info(node_b, a_public).await?;
    
    // 3. L4-A 和 L4-B 同时向对方发起连接 (打洞)
    let conn = tokio::try_join!(
        node_a.punch_hole(b_public),
        node_b.punch_hole(a_public),
    )?;
    
    Ok(conn.0)  // 返回成功建立的连接
}

// 优势:
// - 不需要 L3 参与 (减轻 L3 负载)
// - 成功率提升: 70% → 85% (L4 协助)
// - 延迟更低 (L4 通常在同区域)
```

##### 自动能力检测与升级

```rust
// L4 节点自动监控自己的使用情况,决定是否升级参与级别

impl L4Node {
    pub async fn auto_adjust_participation(&mut self) -> Result<()> {
        let stats = self.routing_table.query_stats.lock().await;
        
        // 条件 1: 被请求次数多 → 说明其他节点需要我的帮助
        let high_demand = stats.request_count > 100;
        
        // 条件 2: 缓存命中率高 → 说明我的缓存有价值
        let high_hit_rate = stats.cache_hit_count as f64 / stats.request_count as f64 > 0.5;
        
        // 条件 3: 设备状态良好
        let good_battery = self.battery_level > 50;
        let on_wifi = self.network_type == NetworkType::WiFi;
        let available_memory = self.available_memory_mb() > 1024;
        
        // 决策: 是否升级为积极模式
        if high_demand && high_hit_rate && good_battery && on_wifi && available_memory {
            self.participation_level = L4ParticipationLevel::Active {
                cache_size: 500,
                relay: true,
                nat_assist: true,
                local_discovery: true,
                preload: true,
            };
            
            info!("L4 节点升级为积极模式,开始主动贡献路由服务");
            
            // 预加载热门节点
            self.preload_popular_nodes().await?;
        }
        
        Ok(())
    }
}

### 关键优势

#### 1. **类 DNS 的分层缓存**

```
查询路径:
L4 客户端 → L3 (缓存 80% 命中) → L2 (缓存 60% 命中) → L1 (权威)

平均查询延迟:
- 80% 请求在 L3 命中: < 10 ms
- 15% 请求在 L2 命中: < 50 ms
- 5% 请求在 L1 查询: < 100 ms

加权平均: 0.8×10 + 0.15×50 + 0.05×100 = 20.5 ms
```

#### 2. **能力感知路由**

```rust
// 根据任务自动选择最佳节点

Task::ZkProof(_) 
  → 查询: node_type=L1, has_gpu=true, region=nearest
  → 返回: 最近的带 GPU 的 L1 节点

Task::Query(_)
  → 查询: node_type=L3, region=same, max_latency=20ms
  → 返回: 同区域的 L3 边缘节点

Task::TxExecution(_)
  → 查询: node_type=L2, max_load=70, region=nearest
  → 返回: 负载最低的 L2 矿机节点
```

#### 3. **智能 NAT 穿透 (多级协助)**

```
传统 P2P (STUN/TURN):
成功率: 60-70%
延迟: 高 (需要多次尝试)
成本: 高 (需要专用 TURN 服务器)

SuperVM (多级协助):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Level 1: L4 互助穿透 (成功率 85%) ⭐ 新增
  - L4 节点充当 STUN 服务器
  - 适用于: 两个 L4 节点连接
  - 延迟: < 300 ms
  - 成本: 零 (P2P 互助)

Level 2: L3 中继辅助 (成功率 95%)
  - L3 节点充当中继
  - 适用于: L4-L2 连接, 或 L4-L4 穿透失败
  - 延迟: < 500 ms
  - 成本: 低 (利用现有 L3 节点)

Level 3: L1 强制中继 (成功率 100%)
  - L1 节点充当中继 (公网 IP)
  - 适用于: 所有方法都失败
  - 延迟: < 1000 ms
  - 成本: 中 (L1 资源宝贵)

优势:
✅ 多级 fallback,成功率接近 100%
✅ 优先使用 L4 互助 (零成本)
✅ 失败自动升级到更强节点
✅ 连接建立后可升级为直连
```

#### 4. **L4 全员参与,网络自愈** ⭐ **核心创新**

```
传统模式:
轻节点 (消费) ──→ 强节点 (提供)
- 轻节点越多,强节点越累
- 网络扩展性差
- 单点瓶颈风险高

SuperVM 模式:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
每个节点既是消费者,也是贡献者

[L4-A] ←→ [L4-B]  (P2P 互助)
  ↑         ↑
  └─────┬───┘
        ↓
      [L3]  (仅处理 L4 无法解决的查询)
        ↓
      [L2]  (处理跨区域查询)
        ↓
      [L1]  (权威路由表)

效果:
✅ 70% 的 L4-L4 查询在 L4 层解决 (不消耗 L3)
✅ 节点越多,网络越快 (缓存命中率提升)
✅ 自愈能力强 (L4 可以互相备份)
✅ 成本分摊 (每个节点贡献一点,整体收益大)

数据:
- 1000 个 L4 节点,50% 启用中继
- 平均每个 L4 缓存 200 节点
- 理论路由容量: 1000 × 200 × 0.5 = 100K 节点信息
- 实际可服务: 10M+ L4 节点 (考虑缓存重叠)
```

#### 5. **实时负载感知**

```rust
// 每个节点定期心跳 (10秒一次)

impl RoutingTable {
    pub async fn heartbeat_loop(&self) {
        loop {
            // 1. 收集本地负载
            let metrics = self.collect_metrics();
            
            // 2. 更新路由表
            self.update_local_load(metrics.cpu_usage).await;
            
            // 3. 广播到上游 (L4→L3, L3→L2, L2→L1)
            self.broadcast_heartbeat(metrics).await;

#### 4. **实时负载感知**

```rust
// 每个节点定期心跳 (10秒一次)

impl RoutingTable {
    pub async fn heartbeat_loop(&self) {
        loop {
            // 1. 收集本地负载
            let metrics = self.collect_metrics();
            
            // 2. 更新路由表
            self.update_local_load(metrics.cpu_usage).await;
            
            // 3. 广播到上游 (L4→L3, L3→L2, L2→L1)
            self.broadcast_heartbeat(metrics).await;
            
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}

// 效果:
// - 过载节点自动从路由表降权
// - 新节点快速加入路由表
// - 下线节点快速清除
```

### 实施计划

在 **Phase 6.4: P2P 网络与通信 (3 周)** 中实现:

**Week 1: 基础寻址系统**
- [ ] 实现 `NodeAddress` 和地址系统
- [ ] 实现四层路由表 (L1/L2/L3/L4)
- [ ] 实现 `AddressingService` 查询协议
- [ ] NAT 类型检测 (STUN)

**Week 2: 智能路由与穿透**
- [ ] 实现智能节点选择算法
- [ ] 实现 ICE 协议打洞
- [ ] 实现 L3 中继服务
- [ ] 实现连接提示生成

**Week 3: 优化与测试**
- [ ] 缓存优化 (LRU + 预取)
- [ ] 心跳机制和负载更新
- [ ] NAT 穿透成功率测试
- [ ] 寻址延迟基准测试
- [ ] 跨区域连接测试

### 技术栈

```rust
// 依赖 crates

[dependencies]
libp2p = { version = "0.53", features = [
    "kad",           // Kademlia DHT
    "mdns",          // 本地网络发现
    "relay",         // 中继协议
    "dcutr",         // 直连升级
    "noise",         // 加密
    "yamux",         // 多路复用
] }

stun = "0.5"         // STUN 协议 (NAT 检测)
ice = "0.9"          // ICE 协议 (打洞)
lru = "0.12"         // LRU 缓存
dashmap = "5.5"      // 并发哈希表
bincode = "1.3"      // 序列化
```

### 对比传统方案

| 特性 | 传统 P2P (DHT) | SuperVM 神经网络寻址 (完整) |
|------|---------------|---------------------------|
| **查询延迟** | 100-500 ms (多跳) | **5-50 ms** (L4 P2P: 5ms, L3: 10ms) ⭐ |
| **缓存命中率** | 无 | **L4: 30-40%, L3: 80-90%** ⭐ |
| **轻节点参与** | 否 (仅消费) | **是 (50-70% L4 贡献路由)** ⭐ |
| **能力感知** | 否 | 是 (硬件/负载/NAT) |
| **智能路由** | 否 | 是 (任务匹配) |
| **NAT 穿透** | 60-70% | **85% (L4), 95%+ (L3)** ⭐ |
| **负载均衡** | 随机 | 智能 (延迟+负载) |
| **区域优化** | 否 | 是 (就近服务) |
| **本地发现** | 否 | **是 (WiFi/蓝牙 < 5ms)** ⭐ |
| **离线可用** | 否 | **部分 (蓝牙 P2P)** ⭐ |
| **网络扩展性** | 差 (节点多负载重) | **好 (节点多路由快)** ⭐ |
| **中继成本** | 高 (专用 TURN) | **低 (P2P 互助)** ⭐ |

---

## 🗺️ 实施路线图

### Phase 6.1: 四层网络基础框架 (4 周)

**Week 1: 硬件检测与节点类型决策**
- [ ] 实现 `HardwareDetector`
- [ ] 实现 `NodeType::auto_detect()`
- [ ] 创建配置文件模板 (L1/L2/L3/L4)
- [ ] 实现命令行参数解析

**Week 2: 任务路由与分发**
- [ ] 实现 `TaskRouter`
- [ ] 定义 `Task` 枚举和属性
- [ ] 实现任务复杂度评估
- [ ] 实现任务路由决策树

**Week 3: 负载均衡与调度**
- [ ] 实现 `LoadBalancer`
- [ ] 实现节点得分算法
- [ ] 实现心跳和健康检查
- [ ] 实现动态负载调整

**Week 4: 测试与文档**
- [ ] 单元测试 (覆盖率 > 80%)
- [ ] 集成测试 (4 层网络模拟)
- [ ] 性能基准测试
- [ ] 部署文档和用户指南

### Phase 6.2: 存储分层管理 (3 周)

**Week 1: L1/L2 存储实现**
- [ ] L1 RocksDB 完整状态
- [ ] L2 RocksDB 裁剪策略
- [ ] 状态同步协议 (L2→L1)
- [ ] 区块归档机制

**Week 2: L3/L4 缓存实现**
- [ ] L3 LRU 缓存
- [ ] L3 预取策略
- [ ] L4 SQLite 轻量存储
- [ ] 状态同步协议 (L4→L3, L3→L2)

**Week 3: 测试与优化**
- [ ] 存储性能测试
- [ ] 缓存命中率测试
- [ ] 数据一致性测试
- [ ] 同步延迟测试

### Phase 6.3: 算力池与分布式计算 (4 周)

**Week 1: 计算池框架**
- [ ] 实现 `ComputePool`
- [ ] 实现 `ComputeNode`
- [ ] 任务队列管理
- [ ] 节点注册与发现

**Week 2: 任务调度**
- [ ] 任务分配算法
- [ ] 分布式 MapReduce
- [ ] 任务失败重试
- [ ] 结果汇总

**Week 3: GPU 加速集成**
- [ ] ZK 证明 GPU 调度
- [ ] GPU 节点管理
- [ ] CPU fallback 机制
- [ ] 批量证明优化

**Week 4: 测试与优化**
- [ ] 算力池性能测试
- [ ] 分布式计算测试
- [ ] GPU 加速效果验证
- [ ] 负载均衡测试

### Phase 6.4: P2P 网络与通信 (3 周)

**Week 1: 神经网络寻址系统 (基础架构)** ⭐ **核心**
- [ ] 实现 `NodeAddress` 和地址系统
  - [ ] `NodeAddress` 结构体 (PeerId + 硬件能力 + NAT类型 + 区域)
  - [ ] `Region` 枚举和延迟估计
  - [ ] `NatType` 检测 (STUN 协议集成)
- [ ] 实现四层路由表
  - [ ] `L1RootRoutingTable` (RocksDB 持久化 + 完整索引)
  - [ ] `L2GlobalRoutingTable` (LRU 缓存 10万节点)
  - [ ] `L3RegionalRoutingTable` (区域缓存 1万节点)
  - [ ] `L4LocalRoutingTable` (本地缓存 100节点)
- [ ] 实现 `RoutingTable` trait (注册/查询/心跳/删除)
- [ ] 单元测试 (路由表基本操作)

**Week 2: 智能路由与快速穿透** ⭐ **核心**
- [ ] 实现 `AddressingService` 寻址协议
  - [ ] `AddressQuery` 查询请求 (支持过滤条件)
  - [ ] `AddressResponse` 响应 (返回节点 + 连接提示)
  - [ ] 智能节点选择算法 (延迟 + 负载 + 能力评分)
- [ ] 实现 NAT 穿透增强
  - [ ] `NatTraversalService` (NAT 类型检测)
  - [ ] ICE 协议打洞 (候选地址收集 + 连接性检查)
  - [ ] L3 中继服务 (自动选择最近 L3 作为 relay)
- [ ] 实现 `ConnectionHint` 生成
  - [ ] 直连提示 (公网 IP)
  - [ ] 打洞提示 (STUN 地址 + NAT 类型)
  - [ ] 中继提示 (L3 节点地址)
- [ ] 集成测试 (不同 NAT 场景穿透测试)

**Week 3: libp2p 集成与优化** 
- [ ] libp2p 网络初始化 (transport + noise + yamux)
- [ ] 节点发现优化
  - [ ] mDNS (本地网络快速发现)
  - [ ] Kademlia DHT (全局发现 + 备份)
  - [ ] 神经网络寻址 (主要方式,取代传统 DHT)
- [ ] 连接管理
  - [ ] 连接池 (复用连接)
  - [ ] 心跳机制 (10秒一次,更新负载)
  - [ ] 自动重连 (连接断开自动恢复)
- [ ] 消息协议
  - [ ] Protobuf 序列化 (寻址查询/响应)
  - [ ] 请求/响应模式 (RPC)
  - [ ] 发布/订阅模式 (心跳广播)
- [ ] 性能测试与优化
  - [ ] 寻址延迟测试 (目标: L3 < 10ms, L2 < 50ms, L1 < 100ms)
  - [ ] 缓存命中率测试 (目标: L3 80%+, L2 60%+)
  - [ ] NAT 穿透成功率测试 (目标: 95%+)
  - [ ] 跨区域连接测试 (全球节点模拟)
  - [ ] 网络分区恢复测试
  - [ ] 带宽优化 (压缩 + 批量传输)

### Phase 6.5: 生产部署 (2 周)

**Week 1: 部署工具**
- [ ] 一键安装脚本
- [ ] Docker 镜像
- [ ] Kubernetes 配置
- [ ] 监控 Dashboard

**Week 2: 文档与培训**
- [ ] 部署指南
- [ ] 运维手册
- [ ] 故障排查
- [ ] 用户培训材料

---

## 📊 预期效果

### 性能提升

```
单机 SuperVM (当前):
- TPS: 187K (低竞争)
- 扩展性: 受限于单机硬件
- 成本: 高 (需高端服务器)

四层网络 SuperVM (Phase 6 完成后):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
L1 (10 节点):      10-20K TPS × 10  = 100-200K TPS
L2 (100 节点):     100-200K TPS × 100 = 10-20M TPS
L3 (1000 节点):    查询响应 1M+ QPS
L4 (无限):         本地操作无限制
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
总吞吐量: 10-20M TPS (理论)
查询 QPS: 1M+
全球延迟: < 100 ms (跨洲)
           < 10 ms (同区域)
```

### 成本优化

```
传统方案 (所有节点高配):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
100 节点 × $5000/月 = $500K/月

四层网络方案:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
L1 (10 节点):    $10K/月 × 10  = $100K/月
L2 (100 节点):   $2K/月 × 100  = $200K/月
L3 (1000 节点):  $100/月 × 1000 = $100K/月
L4 (用户设备):   $0
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
总成本: $400K/月 (节省 20%)
```

### 算力利用率

```
传统方案:
- 平均算力利用率: 30-50%
- 峰值浪费: 50-70% 算力闲置

四层网络方案:
- 平均算力利用率: 70-90%
- 峰值调度: 动态借用全网算力
- 算力共享: 95%+ 利用率
```

---

## 📚 参考文档

### 相关设计文档
- [docs/architecture-2.0.md](../architecture-2.0.md) - 完整架构设计
- [docs/phase1-implementation.md](../phase1-implementation.md) - 实施计划
- [docs/scenario-analysis-game-defi.md](../scenario-analysis-game-defi.md) - 场景分析
- [ROADMAP.md Phase 6](../../ROADMAP.md#phase-6-四层神经网络) - 开发计划

### 技术参考
- [Sui Network Architecture](https://docs.sui.io/learn/architecture)
- [Solana Cluster Architecture](https://docs.solana.com/cluster/overview)
- [IPFS Distributed Storage](https://docs.ipfs.io/concepts/)
- [libp2p Networking](https://docs.libp2p.io/)

---

## 💡 总结

### 核心要点

1. **一核多态**: 同一 SuperVM 内核,根据硬件自动适配 L1/L2/L3/L4
2. **智能路由**: 任务自动路由到最合适的节点执行
3. **存储分层**: 完整状态→部分状态→热点缓存→本地缓存
4. **算力池化**: 全网算力按需调度,充分利用
5. **自动降级**: 硬件不足时自动降级功能

### 关键优势

✅ **成本优化**: 不需要所有节点都是高配  
✅ **灵活扩展**: 可动态增加各层节点  
✅ **高可用性**: 多层冗余,单点故障不影响全局  
✅ **全球部署**: 就近服务,降低延迟  
✅ **算力共享**: 充分利用闲置资源  

### 下一步行动

```bash
# 1. 硬件检测原型
cargo run --bin hardware-detector

# 2. 配置文件生成
./scripts/generate-config.sh --node-type l2

# 3. 本地四层网络模拟
docker-compose -f docker/4layer-network.yml up

# 4. 性能基准测试
cargo bench --bench network_bench
```

---

**文档版本**: v1.0  
**最后更新**: 2025-11-06  
**维护者**: KING XU (CHINA)
