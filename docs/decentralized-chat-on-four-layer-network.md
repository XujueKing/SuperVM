# 去中心化聊天应用指南（基于四层神经网络）

> 目标：实现“接入即服务”（Access-as-a-Service, AaaS）的去中心化聊天，无需中心化应用服务器；用户可选将聊天记录加密备份到 L1 网络（或以更经济的方式锚定至 L1）。

---

## 结论与能力边界

- 可以：
  - 无中心应用服务器的消息传输、发现、路由、穿透与离线转发（L4/L3/L2 协同）；
  - 端到端加密（Signal/X3DH + Double Ratchet），端上存储加密；
  - “接入即服务”：客户端直接接入四层网络（L4→L3→L2→L1），通过神经网络寻址与路由获取连接能力；
  - 用户自选“聊天记录加密备份”到上层网络：
    - 方案 A：完整密文归档（成本较高）
    - 方案 B：去中心化存储（L2/IPFS 等）+ L1 哈希锚定（推荐，显著降费）
- 注意事项：
  - 移动端推送在平台层（iOS/Android）可能仍需系统推送通道，可采用“仅推送唤醒令牌，不承载聊天数据”的合规中继。此中继不保存业务数据，不构成中心化聊天服务器；
  - 受限网络下仅使用合规传输（白名单协议/端口），NAT 穿透失败时走 L3/L1 合规中继；不违规“翻墙”；
  - L1 备份会产生费用与隐私风险（尽管是加密密文，仍应提供可撤回/可轮换密钥与最小可见元数据设计）。

---

## 四层角色映射（聊天场景）

- L4（移动/桌面轻客户端）
  - 端到端加密与会话状态（双棘轮）
  - 本地密文存储、离线消息队列、最近联系人/路由缓存（100–500 节点）
  - 本地发现（mDNS/蓝牙，低延迟邻近传输）、轻量路由中继（可选）
- L3（边缘节点）
  - 区域级“接入即服务”与消息转发（store-and-forward，TTL 控制）
  - 主题/会话索引缓存、Presence 扩散、近端可靠投递
  - NAT 中继、带宽整形、速率限制与反垃圾
- L2（执行/矿机）
  - 全局路由/寻址缓存（K/V + LRU），跨区域消息路径优化
  - 大群组/公共频道的 Gossip 传输优化（分片/分层）
- L1（超算/共识）
  - 可选“持久化锚定层”：
    - 方案 A：加密归档的全量存证
    - 方案 B：外部存储（IPFS/L2 存储）的哈希锚定（推荐）
  - DID/身份注册与密钥轮换锚定（可选）

---

## 身份与加密

- 身份/DID：
  - user_master_pk/sk（长周期） → device_pk/sk（短周期）
  - 可选在 L1 注册 DID 文档（包含公钥、旋转策略、撤销列表）。
- 会话建立：X3DH（或 Noise IK）握手交换预密钥，落地 Double Ratchet。
- 加密与完整性：
  - Envelope = header（from, to, ts, msg_id, ttl, type） + ciphertext + mac
  - 每条消息唯一 msg_id（nonce/UUID），端上去重与乱序重排。
- 元数据最小化：
  - 传输层仅见到路由目标与加密负载；
  - 群组采用“会话密钥派生 + 成员密钥封装”，支持成员加入/退出重密钥。

---

## 传输与路由

- 寻址：使用“神经网络寻址”
  - L4 → 先查本地缓存/邻近 L4 → L3（区域）→ L2（全局）→ L1（权威）
  - 依据延迟/负载/NAT 类型择优；失败自动升级中继级别（L4 互助 → L3 → L1）。
- 传输协议：
  - 1:1/小群：优先直接 P2P（DCUtR/ICE）；失败走 L3 合规中继；
  - 大群/频道：Gossipsub（带基于区域/兴趣的分层拓扑 + Flood protect）。
- 可靠性：
  - At-least-once 投递 + 端上去重；应用 ACK/NACK + 重传窗口；
  - L3 store-and-forward（TTL、上限、速率），L4 离线队列。

---

## 存储与备份（可选 L1）

- 端上（L4）：
  - 全量密文消息、本地索引（倒排/会话 Bloom ）、加盐搜索（可选）
  - 安全区/加密文件系统 + 密钥分离（PIN/生物）
- 边缘（L3）：
  - 仅临时缓存与转发队列（TTL、配额、不可读密文、匿名索引）
- 备份路径：
  - 方案 A：直接 L1 归档（加密后分段上链）— 成本高，适合企业/合规留痕；
  - 方案 B（推荐）：
    1) 将密文包归档至 L2/去中心化存储（如 IPFS/自建 L2 存储）
    2) 把每个归档包的哈希与访问策略锚定到 L1（小交易）
  - 恢复：
    - 从 L1 取锚定 → 拉取外部密文包 → 端上解密/重建索引
- 密钥管理：
  - 备份密钥独立于会话密钥；支持密钥轮换与“过期/销毁”策略
  - 提供“删除锚定 + 删除外部副本”的一键清除（尊重不可篡改与多副本边界，尽可能做到可撤回）。

---

## 合规与受限网络

- 仅使用合规传输（TCP/TLS、QUIC、WebSocket 等白名单端口）；
- NAT 穿透失败自动改用 L3/L1 合规中继；
- 策略引擎：
```toml
[policy]
allowed_transports = ["tcp", "tls", "quic", "wss"]
max_ttl_seconds = 86400
max_forward_hops = 3
region_geofence = ["CN", "SG", "JP"]

[privacy]
store_and_forward = true
l3_cache_ttl_seconds = 600
anonymize_indices = true

[backup]
mode = "anchor"        # none | full | anchor
bucket = "chat-archive"
anchor_chain = "L1"
rotation_days = 30
```

---

## SDK 合同（精简草案）

- 初始化/接入
```rust
struct ChatConfig { policy: Policy, backup: Backup, identity: Identity };
async fn init(cfg: ChatConfig) -> Result<Client>;
async fn connect(client: &Client) -> Result<()>; // 连接四层网络（自动寻址/中继）
```
- 消息收发
```rust
struct SendOptions { ttl: u32, require_ack: bool, priority: u8 }
async fn send_text(to: PeerId, text: String, opt: SendOptions) -> Result<MsgId>;
async fn send_blob(to: PeerId, bytes: Vec<u8>, opt: SendOptions) -> Result<MsgId>;
async fn recv() -> Stream<EncryptedEnvelope>;  // 端上解密后回调上层
```
- 会话与群组
```rust
async fn create_session(peer: PeerId) -> Result<SessionId>;          // X3DH + Ratchet
async fn create_group(members: Vec<PeerId>) -> Result<GroupId>;      // 会话密钥封装
async fn rotate_group_key(g: GroupId) -> Result<()>;                 // 成员变更
```
- 备份与恢复
```rust
async fn backup_now(strategy: BackupMode) -> Result<AnchorTxId>;     // full/anchor
async fn restore(anchor: AnchorRef) -> Result<RestoreReport>;
```
- 事件
```rust
enum Event { Connected, Disconnected, Msg(MsgMeta), Ack(MsgId), Typing(PeerId) }
fn events() -> Stream<Event>;
```

---

## 消息模型（建议）

```json
{
  "header": {
    "msg_id": "uuid",
    "from": "peer-id",
    "to": "peer-id | group-id",
    "type": "text|image|file|typing|ack",
    "ts": 1730956800,
    "ttl": 600,
    "hop": 0
  },
  "ciphertext": "base64",
  "ratchet": "header-fragment",
  "mac": "base64"
}
```

- 去重：端上维护 msg_id → seen 的 Bloom/HashSet；
- ACK：`type=ack` 携带被确认 msg_id 列表；
- 顺序：按 ts + 本地接收序列，若乱序则 UI 侧重排；
- 传输：L4→直连优先，失败走 L3；群组使用 gossipsub 主题（带分层与流控）。

---

## 开发路线图（6 周，MVP）

- Week 1：身份与加密
  - DID/密钥管理、X3DH 握手、Double Ratchet 会话
- Week 2：网络与寻址
  - 接入四层网络、NAT 穿透与中继、会话保持
- Week 3：消息通道
  - 1:1/小群直连、群组 gossipsub、ACK/重传
- Week 4：存储与搜索
  - 端上加密存储、索引、模糊搜索（可选）
- Week 5：备份与恢复
  - L2/IPFS 归档 + L1 锚定、恢复向导、密钥轮换
- Week 6：合规与运维
  - 策略引擎、告警与可观测性、速率限制与防滥用

验收指标：
- P2P 直连成功率 ≥ 85%（受限网络下总体 ≥ 95% 含中继）
- 单聊 P50 延迟 ≤ 100ms（同区域）
- 群聊 1k 人频道消息传播 P95 ≤ 800ms（同区域分层拓扑）
- 端上 CPU ≤ 15%，内存 ≤ 200MB（前台）
- 备份/恢复成功率 99%，锚定成本可配置

---

## 运维与可观测性

- 指标
  - 连接：直连率/中继率、握手时延、失败原因分类
  - 消息：发送/接收 QPS、ACK 丢失率、重传率、去重命中
  - 备份：归档大小、锚定频率、失败重试次数
- 日志与审计
  - 严格避免明文内容；仅结构化元数据（必要最小原则）
- 异常处置
  - 中继退避、黑名单/信誉分、设备限速、验证码/微量 PoW（反滥用）

---

## 最佳实践与建议

- 默认不启用 L1 全量备份；采用“外部存储 + L1 锚定”的经济模式；
- 移动端使用系统推送仅传递唤醒令牌，避免承载聊天内容；
- 端上提供“隐私模式”：不留本地索引，仅密文与极简元数据；
- 群组采用子群/分层主题，减少风暴；
- 合规策略与区域地理围栏默认开启。

---

## 参考
- 四层网络：`docs/four-layer-network-deployment-and-compute-scheduling.md`
- 受限网络可用性：`docs/restricted-network-availability.md`
- 分布式存储与优化：`docs/intelligent-distributed-storage-and-optimization.md`

---

## Try it（本地/跨网 PoC）

本地局域网（mDNS 自动发现）：

```powershell
cargo run -p node-core --example chat_poc
```

然后在任意一侧输入消息回车，另一侧应能收到。该 PoC 使用 mDNS 自动发现和 gossipsub 主题广播，演示“接入即服务”的最小路径；跨网测试可在一方添加可达地址拨号或引入公网中继，详见后续 SDK 扩展。

跨网（已知对端可达地址）：

1) 在 A 侧启动后，观察输出的 Listening on 地址，例如：

```
Listening on /ip4/0.0.0.0/tcp/40045
Listening on /ip4/192.168.1.20/tcp/40045
```

2) 在 B 侧使用 --dial 拨号：

```powershell
cargo run -p node-core --example chat_poc -- --dial /ip4/192.168.1.20/tcp/40045
```

拨号成功后即可互相收发消息。
