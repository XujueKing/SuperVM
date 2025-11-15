# SuperVM Plugin SDK (Rust) — 草案 README

目标：为插件作者提供一个最小的 Rust SDK，包含：
- 与 Host 的 gRPC 客户端/服务绑定样板（基于 proto/plugin_host.proto）
- Native ABI 的辅助宏/类型（生成 manifest json、处理回调 vtable）
- 示例：一个最小的 Bitcoin 子模块 skeleton（只做 Register + Heartbeat）

计划目录结构：
- src/
  - lib.rs           # SDK 公共类型与 Host vtable 定义
  - grpc/            # 由 protobuf 生成的 gRPC 绑定
  - examples/        # 示例插件（binary）

快速开始（草案）：
- 克隆仓库
- 进入 sdk/plugin-sdk-rs
- cargo build --examples
- 运行 examples 中的示例以查看 Register 流程

注意：本 README 为占位草案；随 proto 与 ABI 定稿会补全代码示例与 API 文档。