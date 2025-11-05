# PoC: vm-runtime + node-core

开发者：king

本 PoC 展示如何在 `vm-runtime` 中使用 `wasmtime` 加载并执行一个简单的 wasm 模块（由 WAT 编写），以及如何在 `node-core` 中调用这个运行时。

如何运行

1. 确保已安装 Rust toolchain（stable）
2. 在仓库根目录运行：

```powershell
cargo run -p node-core
```

你应该在日志中看到 PoC 输出 `add(7,8) => 15`。

运行测试

```powershell
cargo test -p vm-runtime
```

---

## License

本仓库代码以 GPL-3.0-or-later 许可协议发布，详见根目录 `LICENSE`。
