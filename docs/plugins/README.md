# SuperVM 插件架构文档

> **更新日期**: 2025-11-10  
> **版本**: v0 (草案)  
> **状态**: 规范制定中

---

## 📖 概述

本目录包含 SuperVM 的**热插拔子模块/插件系统**的完整规范、接口定义、SDK 与示例。该系统允许第三方或官方实现完整的原链节点子模块(如 Bitcoin Core、Geth、Solana)，并通过标准化接口将其区块、交易、状态镜像到 SuperVM 的统一 IR 层。

### 🎯 核心目标

- **开放性**: 允许任何开发者实现链适配器,无需获得预先许可

- **双模支持**: 优先本地原生插件(高性能),同时保留 gRPC 作为容器化/远程部署路径

- **安全可控**: 提供三级运行策略(Strict/Permissive/Dev)与沙箱隔离

- **统一镜像**: 通过 TxIR/BlockIR/StateIR 实现跨链状态的统一查询与验证

---

## 📚 文档索引

### 核心规范

- **[PLUGIN-SPEC.md](./PLUGIN-SPEC.md)** - 插件规范总览(目标、生命周期、ABI、安全策略)

- **[plugin-manifest.schema.json](./plugin-manifest.schema.json)** *(待添加)* - 插件清单 JSON Schema

- **[submodule-adapter.md](./submodule-adapter.md)** *(待添加)* - SubmoduleAdapter trait 详细说明

### 接口定义

- **[proto/plugin_host.proto](../../proto/plugin_host.proto)** - gRPC 数据平面与控制 RPC 定义

- **[ir/schema/](../../ir/schema/)** - TxIR/BlockIR/StateIR JSON Schema (v0)

### 示例与 SDK

- **[example-plugin.yaml](./example-plugin.yaml)** - 插件清单示例(Bitcoin 子模块)

- **[sdk/plugin-sdk-rs/](../../sdk/plugin-sdk-rs/)** - Rust SDK (gRPC 绑定 + Native ABI 辅助)

---

## 🔧 快速开始

### 1. 理解插件类型

SuperVM 支持两种插件接入方式:

| 类型 | 通信方式 | 适用场景 | 性能 | 隔离性 |
|------|---------|---------|------|-------|
| **Native Plugin** | C ABI / 共享内存 | 官方子模块、需要极低延迟 | ⚡ 极高 | 🔒 进程内隔离 |
| **Remote Plugin (gRPC)** | gRPC over TCP/Unix Socket | 第三方实现、容器化部署 | ✅ 高 | 🔐 进程间隔离 |

### 2. 选择运行策略

| 策略 | 签名要求 | 能力审核 | 资源限制 | 适用环境 |
|------|---------|---------|---------|---------|
| **Dev** | ❌ 无 | ❌ 无 | ⚠️ 宽松 | 本地开发、测试网 |
| **Permissive** | ✅ 可选 | ✅ 白名单 | ✅ 配额 | 联盟链、企业网络 |
| **Strict** | ✅ 必须 | ✅ 完整审计 | ✅ 严格 | 主网、生产环境 |

### 3. 实现插件的步骤

#### 方案 A: Native Plugin (Rust/C/C++)

1. 实现 C ABI 导出函数 (`plugin_init`, `plugin_start`, `plugin_stop`, `plugin_get_manifest_json`)
2. 使用 Host 提供的 vtable 回调 (`host_log`, `host_submit_tx`, `host_report_metric`)
3. 编写 `plugin.yaml` 清单并声明 capabilities
4. 编译为动态库 (.so / .dll / .dylib)
5. 配置 Host 加载路径并启动

参考: [sdk/plugin-sdk-rs/examples/native-skeleton.rs](../../sdk/plugin-sdk-rs/) *(待添加)*

#### 方案 B: Remote Plugin (gRPC)

1. 实现 `PluginHost` gRPC 服务 (proto/plugin_host.proto)
2. 处理 `Register`, `StreamBlocks`, `SubmitTx` 等 RPC
3. 编写 `plugin.yaml` 清单(指定 `runMode: grpc` 与监听地址)
4. 部署为独立进程/容器
5. 配置 Host 连接 endpoint 并启动

参考: [sdk/plugin-sdk-rs/examples/grpc-skeleton.rs](../../sdk/plugin-sdk-rs/) *(待添加)*

---

## 🔐 安全与沙箱

### 最小权限原则

- 插件默认**无法访问宿主文件系统、网络、系统调用**

- 必须在清单中显式声明 capabilities (如 `block_stream`, `submit_tx`, `rpc_proxy`)

- Host 根据运行策略决定是否授予权限

### 推荐隔离措施

- **Native Plugin**: 使用 seccomp-bpf / AppArmor / SELinux 限制系统调用

- **Remote Plugin**: 运行在非特权容器 (cgroups + namespace)

- **网络隔离**: 通过 Host 代理控制外部 P2P 连接

- **资源配额**: CPU/内存/存储限制由 Host 强制执行

### 审计日志

所有插件的关键操作(提交交易、状态查询、错误事件)均记录到审计日志,支持 Prometheus metrics 导出。

---

## 📐 架构图

```

┌─────────────────────────────────────────────────────────────────┐
│                        SuperVM Host                             │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │          Plugin Manager (加载/卸载/监控)                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│         ▲                                     ▲                  │
│         │ C ABI                               │ gRPC             │
│         ▼                                     ▼                  │
│  ┌─────────────┐                      ┌──────────────┐          │
│  │ Native      │                      │ Remote       │          │
│  │ Plugin      │                      │ Plugin       │          │
│  │ (.so/.dll)  │                      │ (Process)    │          │
│  └─────────────┘                      └──────────────┘          │
│         │                                     │                  │
│         ▼                                     ▼                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │        Unified State Mirror (TxIR/BlockIR/StateIR)        │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                              ▼                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │   RocksDB (持久化 IR 镜像 + 索引)                        │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘

```

---

## 🛠️ 开发工具链

### SDK 与工具

- **Rust SDK**: `sdk/plugin-sdk-rs` - gRPC 客户端/服务器生成、ABI 辅助宏

- **protoc 插件**: 自动生成 Rust/Go/Python 绑定

- **清单校验器**: `tools/validate-manifest.sh` *(待添加)* - 验证 plugin.yaml 合法性

- **插件测试框架**: `tests/plugin-integration/` *(待添加)* - 集成测试模板

### 调试与监控

- **日志**: 插件通过 `host_log` 回调或 gRPC 发送日志到 Host

- **Metrics**: 通过 `host_report_metric` 或 gRPC 上报到 Prometheus

- **健康检查**: gRPC `Health` RPC 或 Native Plugin 心跳机制

---

## 📋 待办事项 (v0 Roadmap)

- [ ] 补全 `plugin-manifest.schema.json` 并提供校验工具

- [ ] 添加 `submodule-adapter.md` (SubmoduleAdapter trait 详细说明)

- [ ] 完善 Rust SDK (添加 prost/tonic 生成的 gRPC 绑定)

- [ ] 提供 Native Plugin 与 Remote Plugin 的完整示例代码

- [ ] 集成到 ROADMAP Phase 10 M1 (标记"插件规范 v0 发布")

- [ ] 添加插件签名/验证工具与流程文档

- [ ] 编写 Chaos 测试(插件崩溃/超时/资源耗尽场景)

---

## 🔗 相关文档

- [ROADMAP.md - Phase 10: 多链协议适配层](../../ROADMAP.md#phase-10)

- [MULTICHAIN-ARCHITECTURE-VISION.md - 热插拔子模块架构](../MULTICHAIN-ARCHITECTURE-VISION.md)

- [evm-adapter-design.md - SubmoduleAdapter 设计](../evm-adapter-design.md)

- [autoscale/ - 自适应运行模式](../autoscale/)

- [ir/schema/ - 统一 IR 格式](../../ir/schema/)

---

## 📞 联系与贡献

- 问题反馈: 提交 GitHub Issue 并标记 `plugin-system` label

- 贡献指南: 参见 [CONTRIBUTING.md](../../CONTRIBUTING.md)

- 插件示例仓库: *(待建立)* `supervm-plugin-examples`

---

**版权声明**: 本文档采用 GPL-3.0-or-later 许可协议发布
