# SuperVM 插件规范（草案）

说明：本规范定义 SuperVM 支持的“链子模块 / 插件”（chain submodule / plugin）接口，包括本地原生插件 ABI、基于 gRPC 的数据平面 contract、插件清单（manifest）格式、运行策略、安全要求与示例。目标是：让第三方能实现可插拔的完整链节点子模块（例如 Bitcoin / Geth / Solana），并以受控方式将其镜像/汇入 SuperVM 的统一 IR。

目录

- 目标与设计原则

- 插件类型

- 插件生命周期（启动/登记/心跳/卸载）

- 本地插件（Native）ABI 参考（C ABI / vtable）

- gRPC 数据平面概览（proto 文件引用）

- 插件清单（plugin.yaml）字段说明与示例

- 安全与 Sandbox 建议

- 运行策略：Strict / Permissive / Dev

- 签名、可信度与发布建议

一、目标与设计原则

- 最小耦合：Host（SuperVM）向插件暴露尽量少的宿主能力（日志、配置、metrics、RPC 管道）。

- 双重通道：优先支持本地原生插件（高性能、低延迟），同时保留 gRPC 作为网络/容器化部署的兼容路径。

- 明确契约：用 protobuf 定义数据流（区块、交易、日志）与控制 RPC；用 manifest 描述插件元数据与能力。

- 安全第一：插件在受限环境运行（容器 / namespace / cgroups / seccomp），并要求可选的签名校验与权限声明。

二、插件类型

- Native Plugin：以本机二进制（Rust/C/C++）的形式通过 C ABI 接入 Host 进程或作为受管子进程通过共享内存/IPC。

- Remote Plugin（gRPC）：运行在独立进程/容器，通过 gRPC 与 Host 通信（数据平面）。

三、生命周期（最小接口）

- Register(capabilities) -> Host 返回 plugin_id 与 assigned resources

- Heartbeat() -> 维持存活状态

- StreamBlocks(stream) -> 双向或单向流用于发送/接收区块、回滚（reorg）事件

- SubmitTx(tx) -> Host 接收并路由到链网络（如需要）

- Stop/Unload -> 清理并安全断开

四、本地插件 ABI（草案）
注：本节提供最小 C ABI，供在进程内直接加载的 native 插件实现。

导出函数（C ABI, extern "C"）:

- const char* plugin_get_manifest_json();
  - 返回插件清单的 JSON 字符串（UTF-8），Host 用于展示与权限检查。

- int plugin_init(void* host_api_vtable);
  - 初始化插件，Host 会传入一组回调函数指针（日志、metrics、submit_tx 等）。返回 0 成功，非 0 错误码。

- int plugin_start();
  - 启动插件的主循环/线程。返回 0 成功。

- int plugin_stop();
  - 请求停止并回收资源；阻塞直到完成或超时。返回 0 成功。

- void plugin_free_string(char* s);
  - 释放通过 plugin_get_manifest_json 等返回的字符串（若插件分配内存）。

Host API vtable（插件可回调的函数，示例）:

- void host_log(int level, const char* msg);

- int host_submit_tx(const char* chain_id, const uint8_t* tx, size_t len);

- int host_report_metric(const char* name, double value);

- int host_request_shutdown(int reason_code);

注意：ABI 应保证向后兼容。后期可通过版本号字段与 capability negotiation 扩展能力。

五、gRPC 数据平面（参见 proto/plugin_host.proto）

- Register / RegisterResponse

- StreamBlocks: 服务端 streaming 区块（BlockMessage），并支持 Reorg/Undo 消息

- SubmitTx RPC: 单次 RPC 提交交易

- Bidirectional stream 可用于高吞吐场景（区块/交易/日志流）

六、插件清单（plugin.yaml）字段（示例见下）

- id: 插件唯一 id（例如 org.example.bitcoind）

- name, version

- chain: 比如 bitcoin / ethereum / solana

- capabilities: [block_stream, submit_tx, rpc_proxy, archive_indexer]

- binary: 指向二进制或容器镜像

- signature: 可选签名字段

- resources: cpu, memory, storage

- runMode: native | grpc

- security: sandbox: true/false, seccomp_profile: "baseline"

七、安全与 Sandbox 建议

- 必须在非特权容器/进程中运行插件（最小权限原则）。

- Host 应支持多级策略：Dev(无校验)、Permissive(仅白名单 capability)、Strict(签名 & 白名单)。

- 网络隔离：推荐通过代理控制插件到外部 P2P 网络的出入流量。

- 审计日志：所有提交 tx、执行关键事件需有可追溯日志。

八、运行策略（简述）

- Dev：允许未签名插件加载，便于开发。

- Permissive：允许 signed 或白名单清单，启用资源配额。

- Strict：必须通过发布签名与完整能力审核；默认用于生产网络。

九、签名与可信度

- 建议使用 PKI 签名插件二进制，并在 manifest 中包含公钥指纹。

- Host 提供可配置的信任根（root-of-trust）来校验插件签名。

十、示例与参考

- 插件清单示例：见 docs/plugins/example-plugin.yaml

- proto: proto/plugin_host.proto

版本与兼容性

- 这是 v0 草案，后续会以向后兼容的方式演进。任何 ABI/Proto 的不兼容改动需通过 major 版本发布。
