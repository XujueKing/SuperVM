# 去中心化聊天 PoC 实现完成

## 实现概述

我们成功实现了基于四层神经网络架构的去中心化聊天应用 PoC,展示了"接入即服务"（Access-as-a-Service）的核心能力。

## 已完成功能

### ✅ 核心功能

1. **本地发现** (mDNS)
   - 自动发现局域网内的其他节点
   - 零配置本地通信

2. **消息广播** (gossipsub)
   - Pub/Sub 模式的消息传播
   - 消息认证和签名
   - 支持主题订阅

3. **跨网连接** (--dial)
   - 手动拨号连接远程节点
   - 支持多种 multiaddr 格式

4. **QUIC 传输** (--quic)
   - 基于 UDP 的现代传输协议
   - 更好的 NAT 穿透能力
   - 内置加密和多路复用

5. **中继支持** (--relay)
   - 通过中继服务器实现 NAT 穿透
   - Circuit Relay v2 协议

6. **直接连接升级** (DCUtR)
   - 在中继基础上协商直接连接
   - 实现 NAT 打洞

### ✅ 辅助功能

- **身份识别** (identify): 节点能力发现和协议协商
- **连接保活** (ping): 确保连接活跃

## 文件结构

```
src/node-core/
├── Cargo.toml                          # 添加了 libp2p 依赖（含 QUIC/relay/dcutr）
└── examples/
    └── chat_poc.rs                     # 主要实现文件

docs/
├── decentralized-chat-on-four-layer-network.md  # 完整架构设计文档
└── chat-poc-usage.md                   # 使用指南
```

## 技术栈

- **libp2p 0.53**: 去中心化网络通信框架
  - gossipsub: Pub/Sub 消息传播
  - mDNS: 本地网络发现
  - QUIC: UDP 传输协议
  - relay: 中继客户端
  - DCUtR: 直接连接升级
  - noise: 传输加密
  - yamux: 流多路复用
- **tokio**: 异步运行时
- **clap**: 命令行参数解析

## 快速开始

### 1. 本地测试（同一局域网）

终端 1:
```powershell
cargo run -p node-core --example chat_poc
```

终端 2:
```powershell
cargo run -p node-core --example chat_poc
```

两个实例会自动发现并连接。输入消息回车即可发送。

### 2. 跨网测试（已知对端地址）

终端 1:
```powershell
cargo run -p node-core --example chat_poc
# 观察输出的地址,例如 /ip4/192.168.1.100/tcp/51574
```

终端 2:
```powershell
cargo run -p node-core --example chat_poc -- --dial /ip4/192.168.1.100/tcp/51574
```

### 3. NAT 穿透测试（通过中继）

中继节点（需要公网地址）:
```powershell
cargo run -p node-core --example chat_poc -- --quic
# 记录输出的 peer ID,例如 12D3KooWAbC...
```

客户端 A（NAT 后）:
```powershell
cargo run -p node-core --example chat_poc -- --quic --relay /ip4/1.2.3.4/udp/40046/quic-v1/p2p/12D3KooWAbC...
```

客户端 B（NAT 后）:
```powershell
cargo run -p node-core --example chat_poc -- --quic --relay /ip4/1.2.3.4/udp/40046/quic-v1/p2p/12D3KooWAbC...
```

## 实现亮点

### 1. 真正的去中心化
- ✅ 无中心化应用服务器
- ✅ 点对点消息传输
- ✅ 自动网络发现
- ✅ 分布式路由

### 2. 四层网络架构体现

当前 PoC 主要展示了 **L4 层**（客户端层）的能力:

- **本地发现**: mDNS 实现邻近感知（四层网络中的 L4 本地优化）
- **消息广播**: gossipsub 实现去中心化消息传播
- **NAT 穿透**: relay + DCUtR 实现跨网络通信（体现 L3 边缘中继能力）
- **智能路由**: libp2p 的自动路由选择（为 L2 全局路由打基础）

### 3. 生产级协议选择

- **gossipsub**: libp2p 生态成熟的 pub/sub 协议,已在 Filecoin、Ethereum 2.0 等项目中使用
- **QUIC**: IETF 标准化的传输协议,Chrome、HTTP/3 的底层技术
- **noise**: 现代密码学框架,用于 WireGuard、libp2p 等
- **relay v2**: 经过实战验证的 NAT 穿透方案

## 架构映射

| 功能 | 四层网络对应 | 实现状态 |
|------|-------------|---------|
| 本地发现 | L4 (邻近感知) | ✅ mDNS |
| 消息传输 | L4 (端到端) | ✅ gossipsub |
| 边缘中继 | L3 (NAT 穿透) | ✅ relay client |
| 直接升级 | L3→L4 (优化路径) | ✅ DCUtR |
| 全局路由 | L2 (跨区域) | 🔄 待集成 |
| 持久化锚定 | L1 (共识层) | 📋 计划中 |

## 下一步计划

### 阶段 2: SDK 封装
- [ ] 创建 `chat-sdk` crate
- [ ] 提供易用的 API (init/connect/send/recv/backup/restore)
- [ ] 添加会话管理和群组支持
- [ ] 实现消息去重和顺序保证

### 阶段 3: 安全增强
- [ ] X3DH 密钥交换
- [ ] Double Ratchet 端到端加密
- [ ] 元数据最小化
- [ ] 密钥轮换

### 阶段 4: L1 备份集成
- [ ] 实现加密归档（完整备份模式）
- [ ] 实现外部存储 + L1 哈希锚定（推荐模式）
- [ ] 添加备份/恢复工作流
- [ ] 费用估算和优化

### 阶段 5: 生产化
- [ ] 离线消息队列
- [ ] TTL 和消息过期管理
- [ ] 性能优化和压力测试
- [ ] 监控和可观测性
- [ ] 多平台支持（移动端）

## 相关文档

- [完整架构设计](../docs/decentralized-chat-on-four-layer-network.md)
- [使用指南](../docs/chat-poc-usage.md)
- [四层网络部署](../docs/four-layer-network-deployment-and-compute-scheduling.md)
- [受限网络可用性](../docs/restricted-network-availability.md)

## 贡献者

- 实现时间: 2025年
- 基础设施: SuperVM 四层神经网络架构

## 许可证

与主项目相同
