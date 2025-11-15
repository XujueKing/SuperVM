#!/usr/bin/env bash
set -euo pipefail

echo "[info] Native (non-Docker) RISC0 build start"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT/src/l2-executor"

LOG_DIR="$REPO_ROOT/ci_artifacts"
mkdir -p "$LOG_DIR"

echo "[step] Checking required commands"
command -v cargo >/dev/null || { echo "[error] cargo not found"; exit 1; }
command -v rustup >/dev/null || { echo "[error] rustup not found"; exit 1; }
command -v rzup >/dev/null || echo "[warn] rzup not found in PATH (will attempt install if missing)"

if ! command -v rzup >/dev/null; then
  echo "[step] Installing rzup (one-time)"
  curl -L https://risczero.com/install | bash
  export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
fi

echo "[step] Verifying cargo-risczero"
if ! command -v cargo-risczero >/dev/null; then
  echo "[warn] cargo-risczero missing, installing via cargo install"
  cargo install cargo-risczero || echo "[warn] cargo-risczero install failed; continuing to try build"
fi

echo "[info] Tool versions:" | tee "$LOG_DIR/native_versions.txt"
{ rzup --version || true; cargo risczero --version || true; rustup toolchain list; } >> "$LOG_DIR/native_versions.txt" 2>&1

echo "[step] Ensuring RISC0 toolchain present"
if ! rustup toolchain list | grep -q '^risc0'; then
  echo "[info] Running rzup install"
  rzup install || { echo "[error] rzup install failed"; exit 1; }
fi

echo "[step] Matching crate versions"
grep -E 'risc0-zkvm|risc0-build' Cargo.toml | tee "$LOG_DIR/native_versions.txt" >/dev/null

export RISC0_DEV_MODE=1
unset RISC0_USE_DOCKER RISC0_TOOLCHAIN

echo "[step] Building release example (risc0_performance_comparison)"
set +e
cargo build --release --features risc0-poc --example risc0_performance_comparison 2>&1 | tee "$LOG_DIR/native_build.log"
BUILD_ERR=${PIPESTATUS[0]}
set -e

if [ $BUILD_ERR -ne 0 ]; then
  echo "[error] Build failed code=$BUILD_ERR"
  tail -n 120 "$LOG_DIR/native_build.log" || true
  exit $BUILD_ERR
fi

echo "[ok] Build succeeded"
BIN="./target/release/examples/risc0_performance_comparison"
if [ ! -x "$BIN" ]; then
  echo "[error] Built binary not found: $BIN"
  exit 2
fi

echo "[step] Running example"
"$BIN" 2>&1 | tee "$LOG_DIR/native_run.log"
echo "[done] Logs: $LOG_DIR/native_build.log, $LOG_DIR/native_run.log"
