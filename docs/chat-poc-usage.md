# 去中心化聊天 PoC 使用指南

## 功能特性

该 PoC 实现了以下功能：

- **本地发现**：mDNS 自动发现局域网内的其他节点

- **消息广播**：使用 gossipsub 协议进行 pub/sub 消息传播

- **跨网连接**：支持 --dial 手动拨号连接远程节点

- **QUIC 传输**：支持 QUIC 协议（UDP），对 NAT 穿透更友好

- **中继支持**：通过 relay 协议实现 NAT 穿透

- **DCUtR**：直接连接升级，在中继基础上建立点对点连接

## 使用方法

### 1. 本地局域网测试（mDNS 自动发现）

最简单的使用方式，在同一局域网内启动多个实例：

```powershell

# 终端 1

cargo run -p node-core --example chat_poc

# 终端 2

cargo run -p node-core --example chat_poc

```

两个实例会通过 mDNS 自动发现对方，然后可以互相发送消息。

### 2. 跨网连接（已知对端地址）

如果对端有公网地址或在同一可达网络内：

1) 在 A 侧启动后，观察输出的 Listening on 地址：

```

Local peer id: 12D3KooW...
Listening on /ip4/0.0.0.0/tcp/40045
Listening on /ip4/192.168.1.20/tcp/40045

```

2) 在 B 侧使用 --dial 拨号：

```powershell
cargo run -p node-core --example chat_poc -- --dial /ip4/192.168.1.20/tcp/40045

```

### 3. QUIC 传输模式

QUIC 基于 UDP，在某些 NAT 环境下穿透能力更强：

```powershell

# 使用 QUIC

cargo run -p node-core --example chat_poc -- --quic

# QUIC + 拨号

cargo run -p node-core --example chat_poc -- --quic --dial /ip4/1.2.3.4/udp/40046/quic-v1

```

### 4. 通过中继穿透 NAT

当两个节点都在 NAT 后面无法直接连接时，可以使用中继服务器。

**步骤 1**：启动中继节点（需要公网地址或可被两端访问）

```powershell
cargo run -p node-core --example chat_poc -- --quic

```

观察输出，记录 peer ID 和地址：

```

Local peer id: 12D3KooWAbC123...
Listening on /ip4/0.0.0.0/tcp/40045
Listening on /ip4/1.2.3.4/udp/40046/quic-v1

```

**步骤 2**：NAT 后的客户端 A 连接到中继

```powershell
cargo run -p node-core --example chat_poc -- --quic --relay /ip4/1.2.3.4/udp/40046/quic-v1/p2p/12D3KooWAbC123...

```

**步骤 3**：NAT 后的客户端 B 也连接到同一中继

```powershell
cargo run -p node-core --example chat_poc -- --quic --relay /ip4/1.2.3.4/udp/40046/quic-v1/p2p/12D3KooWAbC123...

```

两个客户端会：
1. 首先通过中继服务器建立连接（relayed connection）
2. 然后 DCUtR 协议会尝试在中继基础上协商直接连接
3. 如果成功，会升级为直接的点对点连接（hole punching）

## 参数说明

- `--dial <MULTIADDR>`：拨号到指定的 multiaddr 地址

- `--quic`：使用 QUIC 传输而不是 TCP

- `--relay <MULTIADDR>`：连接到指定的中继服务器

## 示例输出

成功连接后，你会看到类似的输出：

```

Local peer id: 12D3KooWAbC...
Listening on /ip4/0.0.0.0/tcp/52341
Listening on /ip4/192.168.1.100/tcp/52341
Chat PoC started. Type and press Enter to publish.

Identified 12D3KooWXyz...: protocols=["/ipfs/ping/1.0.0", "/ipfs/id/1.0.0", ...]

```

然后输入消息回车，对端会显示：

```

<12D3KooWAbC...> Hello World!

```

## 架构说明

该 PoC 演示了四层网络中的 L4（客户端）功能：

- **L4 层**：客户端直接通过 libp2p 实现去中心化通信
  - mDNS：本地网络自动发现（邻近感知）
  - gossipsub：pub/sub 消息广播
  - identify：节点身份识别
  - ping：连接保活
  - QUIC/TCP：传输层协议
  - relay client：NAT 穿透中继客户端
  - DCUtR：直接连接升级

这实现了"接入即服务"（Access-as-a-Service）的核心理念：客户端无需中心化服务器即可发现、连接和通信。

## 下一步

- [ ] 实现端到端加密（X3DH + Double Ratchet）

- [ ] 添加离线消息队列和 TTL 管理

- [ ] 实现消息去重和顺序保证

- [ ] 添加群组支持和成员管理

- [ ] 实现 L1 备份功能（加密归档或哈希锚定）

- [ ] 封装为易用的 SDK

详见完整设计文档：`docs/decentralized-chat-on-four-layer-network.md`
