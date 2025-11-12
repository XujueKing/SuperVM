#!/usr/bin/env bash
set -euo pipefail

echo "=== SuperVM Bootstrap (Linux/macOS) ==="

DB_PATH="${DB_PATH:-}" 
FEATURES="${FEATURES:-rocksdb-storage}"
YES="${YES:-0}"

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
if [ -z "$DB_PATH" ]; then
  DB_PATH="$ROOT_DIR/data/rocksdb"
fi

OS="$(uname -s)"
echo "OS: $OS"

# cargo check
if ! command -v cargo >/dev/null 2>&1; then
  echo "⚠️  未检测到 cargo。请先安装 Rust 工具链: https://rustup.rs"
  exit 1
fi

# deps (optional)
if [ "$YES" = "1" ]; then
  if command -v apt >/dev/null 2>&1; then
    sudo apt update
    sudo apt install -y build-essential cmake pkg-config clang git
  elif command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y gcc gcc-c++ cmake make pkg-config git llvm clang
  elif [ "$OS" = "Darwin" ] && command -v brew >/dev/null 2>&1; then
    brew install cmake llvm pkg-config
  fi
fi

# ensure db path
mkdir -p "$DB_PATH"
echo "DB Path: $DB_PATH"

# build
export RUSTFLAGS="-C target-cpu=native"
echo "构建中: cargo build --release --features $FEATURES"
cargo build --release --features "$FEATURES"

echo "\n✅ 构建完成"
echo "下一步:"
echo "  - HTTP 指标服务: cargo run --example storage_metrics_http --features $FEATURES --release"
echo "  - 一致性测试:   cargo run --example persistence_consistency_test --features $FEATURES --release"
echo "  - 数据目录:     $DB_PATH"
