# SuperVM Native Monitor - 原生监控客户端设计

> **版本**: v0.1 (草案)  
> **更新日期**: 2025-11-10  
> **目标**: 替代 Grafana + Prometheus, 提供零依赖的跨平台原生监控解决方案

---

## 🎯 设计目标

### 核心需求

- ✅ **零依赖部署**: 单一可执行文件, 无需 Docker/Node.js/Python

- ✅ **跨平台**: Windows/Linux/macOS 原生支持

- ✅ **高性能**: < 50MB 内存占用, < 5% CPU 使用率

- ✅ **实时监控**: 支持毫秒级数据更新

- ✅ **离线分析**: 本地时序数据库, 支持历史查询

- ✅ **易用性**: 类 VS Code 的现代 UI/UX

### 替代现有方案

| 现状 | 问题 | 原生监控客户端解决方案 |
|------|------|---------------------|
| Prometheus + Grafana | 需要运行两个服务器进程 | 单一可执行文件 |
| Web UI | 依赖浏览器 | 原生窗口应用 |
| 配置复杂 | 需要编写 YAML/JSON | 图形化配置界面 |
| 资源占用高 | ~200MB+ 内存 | < 50MB 内存 |

---

## 🏗️ 技术架构

### 1. GUI 框架选型: egui

**推荐理由**:

- 纯 Rust 生态 (零 JavaScript 依赖)

- 即时模式 GUI (简化状态管理)

- 跨平台原生渲染 (OpenGL/Vulkan/DirectX/Metal)

- 低延迟 (< 1ms frame time)

- 内存占用小 (~20MB 基础占用)

**替代方案对比**:
| 框架 | 优势 | 劣势 | 推荐度 |
|------|------|------|-------|
| **egui** | 纯 Rust, 高性能, 跨平台 | UI 组件相对简单 | ⭐⭐⭐⭐⭐ |
| Tauri | Web 技术栈, UI 丰富 | 体积大, 依赖 WebView | ⭐⭐⭐ |
| iced | 声明式 UI, Elm 架构 | 生态不成熟 | ⭐⭐⭐ |
| gtk-rs | 成熟的 GTK 绑定 | Linux 优先, 跨平台体验差 | ⭐⭐ |

### 2. 系统架构

```

┌──────────────────────────────────────────────────────────────┐
│            SuperVM Native Monitor (可执行文件)               │
├──────────────────────────────────────────────────────────────┤
│  UI Layer (egui)                                             │
│  ┌────────────────┬────────────────┬────────────────┐        │
│  │  Dashboard     │  Metrics View  │  Node Manager  │        │
│  │  - TPS         │  - Real-time   │  - Connect     │        │
│  │  - Latency     │  - History     │  - Config      │        │
│  │  - Success%    │  - Alerts      │  - Logs        │        │
│  └────────────────┴────────────────┴────────────────┘        │
│  ┌────────────────┬────────────────┬────────────────┐        │
│  │  TX Explorer   │  State Viewer  │  Settings      │        │
│  │  - Search      │  - MVCC Ver    │  - Themes      │        │
│  │  - Details     │  - RocksDB     │  - Export      │        │
│  └────────────────┴────────────────┴────────────────┘        │
├──────────────────────────────────────────────────────────────┤
│  Data Collection Layer                                       │
│  ┌──────────────────────────────────────────────┐            │
│  │  Metrics Collector (复用 vm-runtime/metrics) │            │
│  │  - Pull from /metrics endpoint               │            │
│  │  - Parse Prometheus format                   │            │
│  │  - Store to time-series DB                   │            │
│  └──────────────────────────────────────────────┘            │
├──────────────────────────────────────────────────────────────┤
│  Storage Layer                                               │
│  ┌────────────────┬────────────────┬────────────────┐        │
│  │  RocksDB       │  Alert Rules   │  Config        │        │
│  │  (Time-Series) │  (JSON)        │  (TOML)        │        │
│  └────────────────┴────────────────┴────────────────┘        │
├──────────────────────────────────────────────────────────────┤
│  Communication Layer                                         │
│  ┌────────────────┬────────────────┬────────────────┐        │
│  │  HTTP Client   │  gRPC Client   │  WebSocket     │        │
│  │  (/metrics)    │  (Node API)    │  (Real-time)   │        │
│  └────────────────┴────────────────┴────────────────┘        │
├──────────────────────────────────────────────────────────────┤
│  Rendering Backend (wgpu)                                    │
│  OpenGL / Vulkan / DirectX 12 / Metal                        │
└──────────────────────────────────────────────────────────────┘

```

### 3. 核心功能模块

#### 3.1 Dashboard (仪表盘)

```rust
// 主要显示内容

- 实时 TPS (Transactions Per Second)

- 平均延迟 (ms)

- 成功率 (%)

- MVCC 版本数

- RocksDB 写入速率

- 内存使用量

- CPU 使用率

```

**UI 设计** (egui 伪代码):

```rust
fn dashboard_ui(ui: &mut egui::Ui, metrics: &Metrics) {
    ui.heading("SuperVM 实时监控");
    
    ui.horizontal(|ui| {
        // TPS 卡片
        metric_card(ui, "TPS", metrics.tps, "🚀");
        // 延迟卡片
        metric_card(ui, "Latency", metrics.latency_ms, "⏱️");
        // 成功率卡片
        metric_card(ui, "Success Rate", metrics.success_rate, "✅");
    });
    
    // 实时图表
    egui_plot::Plot::new("tps_chart")
        .height(200.0)
        .show(ui, |plot_ui| {
            plot_ui.line(/* TPS 时序数据 */);
        });
}

```

#### 3.2 Real-time Charts (实时图表)

- 使用 **egui_plot** 或 **plotters**

- 支持缩放、平移、导出

- 多曲线叠加显示

- 时间范围选择器 (1min / 5min / 1hour / 1day)

**图表类型**:

```

1. 折线图: TPS / Latency / Success Rate
2. 柱状图: Transaction Distribution
3. 热力图: MVCC Conflict Heatmap
4. 饼图: Transaction Type Distribution

```

#### 3.3 Node Manager (节点管理)

```rust
功能:

- 连接到 SuperVM 节点 (HTTP/gRPC)

- 显示节点状态 (在线/离线)

- 配置连接参数 (IP/Port/Auth)

- 多节点切换

- 自动发现 (mDNS)

```

#### 3.4 Transaction Explorer (交易浏览器)

```rust
功能:

- 搜索交易 (by hash / sender / receiver)

- 显示交易详情 (inputs/outputs/gas/logs)

- MVCC 版本历史

- 依赖关系图

```

#### 3.5 State Viewer (状态查看器)

```rust
功能:

- RocksDB 键值对查看

- MVCC 版本链浏览

- Snapshot 管理

- Pruning 进度

```

#### 3.6 Alert Engine (告警引擎)

```rust
规则配置:

- TPS < 1000 持续 30s → 警告

- Success Rate < 95% → 紧急

- Latency > 100ms → 警告

- RocksDB 写入失败 → 紧急

通知方式:

- 系统通知 (Windows Toast / macOS Notification Center)

- 邮件 (SMTP)

- Webhook (自定义)

```

---

## 📦 依赖清单

```toml
[dependencies]

# GUI 框架

egui = "0.28"
eframe = "0.28"  # egui 桌面应用封装
egui_plot = "0.28"  # 图表组件

# 渲染后端

wgpu = "0.19"  # 跨平台 GPU API

# 数据采集

reqwest = { version = "0.12", features = ["json"] }  # HTTP 客户端
tonic = "0.11"  # gRPC 客户端
tokio = { version = "1", features = ["full"] }

# 时序数据库

rocksdb = "0.22"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 图表渲染 (备选)

plotters = "0.3"
plotters-backend = "0.3"

# 系统通知

notify-rust = "4"  # 跨平台通知

# 配置管理

toml = "0.8"
directories = "5"  # XDG 目录

# 日志

tracing = "0.1"
tracing-subscriber = "0.3"

```

---

## 🚀 实施路径

### Phase 1: MVP (2 周)

**目标**: 基础 UI + 连接到 SuperVM /metrics 端点

- [ ] 搭建 egui 项目结构

- [ ] 实现 HTTP 客户端拉取 /metrics

- [ ] 解析 Prometheus 格式数据

- [ ] 显示基础 Dashboard (TPS/Latency/Success Rate)

- [ ] 实时刷新 (1s 间隔)

**交付物**:

- `native-monitor/` crate

- 可运行的 Windows exe

- 基础文档

### Phase 2: 图表与历史 (2 周)

**目标**: 实时图表 + 本地时序存储

- [ ] 集成 egui_plot 绘制实时图表

- [ ] RocksDB 时序存储 (ring buffer)

- [ ] 时间范围选择器

- [ ] 数据导出 (CSV/JSON)

**交付物**:

- 折线图/柱状图支持

- 历史数据查询 UI

- 数据保留策略 (默认 7 天)

### Phase 3: 高级功能 (2 周)

**目标**: 节点管理 + 告警 + 多平台打包

- [ ] 节点连接管理 UI

- [ ] 告警规则配置

- [ ] 系统通知集成

- [ ] Linux/macOS 打包

**交付物**:

- 多节点支持

- 告警引擎 v1

- 跨平台二进制 (Windows/Linux/macOS)

### Phase 4: 优化与发布 (1 周)

**目标**: 性能优化 + 用户体验优化

- [ ] 内存占用优化 (< 50MB)

- [ ] 启动时间优化 (< 500ms)

- [ ] 主题支持 (Dark/Light)

- [ ] 快捷键支持

- [ ] 自动更新机制

**交付物**:

- v1.0.0 正式版

- 用户手册

- 安装包 (MSI/DEB/DMG)

---

## 📊 性能目标

| 指标 | 目标值 | 现状 (Grafana) |
|------|-------|---------------|
| 内存占用 | < 50MB | ~200MB |
| CPU 使用率 | < 5% | ~10% |
| 启动时间 | < 500ms | ~3s |
| 数据刷新延迟 | < 100ms | ~1s |
| 安装包大小 | < 30MB | ~500MB (Docker) |

---

## 🎨 UI/UX 设计原则

### 1. 类 VS Code 风格

- 侧边栏 (左侧): 导航菜单

- 主区域 (中间): Dashboard / Charts

- 面板 (下方): Logs / Alerts

- 状态栏 (底部): 连接状态 / 数据刷新频率

### 2. 配色方案

```rust
// Dark Theme (默认)
background: #1e1e1e
foreground: #d4d4d4
accent: #007acc
success: #4ec9b0
warning: #ce9178
error: #f48771

// Light Theme
background: #ffffff
foreground: #1e1e1e
accent: #0066b8

```

### 3. 快捷键

```

Ctrl+R: 刷新数据
Ctrl+N: 新建连接
Ctrl+T: 切换主题
Ctrl+,: 打开设置
Ctrl+Shift+L: 查看日志
F5: 强制重新连接

```

---

## 🔧 配置文件

**位置**: `~/.config/supervm-monitor/config.toml` (Linux/macOS)  
**位置**: `%APPDATA%\SuperVM\monitor\config.toml` (Windows)

```toml
[connection]
default_endpoint = "http://localhost:8080"
refresh_interval_ms = 1000
timeout_ms = 5000

[storage]
retention_days = 7
db_path = "~/.local/share/supervm-monitor/db"

[alerts]
enabled = true
[[alerts.rules]]
name = "Low TPS"
condition = "tps < 1000"
duration_s = 30
severity = "warning"

[[alerts.rules]]
name = "High Latency"
condition = "latency_ms > 100"
duration_s = 10
severity = "critical"

[ui]
theme = "dark"
font_size = 14
window_size = [1280, 720]

```

---

## 🧪 测试策略

### 1. 单元测试

- Metrics 解析器

- 时序数据存储

- 告警规则引擎

### 2. 集成测试

- 连接到 mock SuperVM 节点

- 数据刷新流程

- 告警触发逻辑

### 3. UI 测试

- 手动 UI 测试清单

- 截图回归测试 (可选)

### 4. 性能测试

```bash

# 内存占用测试

valgrind --tool=massif ./native-monitor

# CPU 占用测试

perf record ./native-monitor

# 启动时间测试

hyperfine './native-monitor --benchmark'

```

---

## 📝 待决事项

1. **图表库选择**: egui_plot vs plotters vs 自定义 (待基准测试)
2. **时序数据库**: RocksDB vs sled vs 自定义 ring buffer
3. **自动更新机制**: GitHub Releases vs 自建服务器
4. **许可证**: MIT vs Apache-2.0 vs GPL (需要决策)

---

## 🔗 参考资源

- egui 官方示例: https://github.com/emilk/egui

- egui_plot 文档: https://docs.rs/egui_plot

- Prometheus 数据格式: https://prometheus.io/docs/instrumenting/exposition_formats/

- VS Code UI 设计: https://code.visualstudio.com/api/references/theme-color

---

**下一步行动**: 创建 `native-monitor/` crate 并实现 Phase 1 MVP
