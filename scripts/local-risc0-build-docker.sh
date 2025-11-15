#!/usr/bin/env bash
set -euo pipefail

echo "[info] Starting local RISC0 build via Docker"

# Ensure we run from repo root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# 1) Check docker availability
if ! command -v docker >/dev/null 2>&1; then
  echo "[error] docker not found in PATH. Please install Docker Desktop and enable WSL integration."
  echo "[hint] https://docs.docker.com/desktop/wsl/"
  exit 1
fi
docker --version || true

# 2) Check cargo availability (host)
if ! command -v cargo >/dev/null 2>&1; then
  echo "[error] cargo not found in PATH. Install Rust (rustup) in this environment or run from your Rust dev shell."
  echo "[hint] curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi
cargo --version || true

# 3) Environment to force Docker guest build
export RISC0_USE_DOCKER=1
export RISC0_TOOLCHAIN=docker
export RISC0_DEV_MODE=1

echo "[info] Env set: RISC0_USE_DOCKER=$RISC0_USE_DOCKER, RISC0_TOOLCHAIN=$RISC0_TOOLCHAIN, RISC0_DEV_MODE=$RISC0_DEV_MODE"

pushd "$REPO_ROOT/src/l2-executor" >/dev/null

echo "[step] Building (release) with RISC0 guest via Docker..."
set +e
cargo build --release --features risc0-poc --example risc0_performance_comparison 2>&1 | tee "$REPO_ROOT/ci_artifacts/local_build.log"
BUILD_ERR=${PIPESTATUS[0]}
set -e

if [[ $BUILD_ERR -ne 0 ]]; then
  echo "[error] Build failed with code $BUILD_ERR"
  tail -n 200 "$REPO_ROOT/ci_artifacts/local_build.log" || true
  exit $BUILD_ERR
fi

echo "[ok] Build succeeded"

echo "[step] Running example: target/release/examples/risc0_performance_comparison"
"$REPO_ROOT/src/l2-executor/target/release/examples/risc0_performance_comparison" 2>&1 | tee "$REPO_ROOT/ci_artifacts/local_run.log"

echo "[done] Logs saved to ci_artifacts/local_build.log and ci_artifacts/local_run.log"
popd >/dev/null
