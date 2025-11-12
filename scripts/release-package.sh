#!/usr/bin/env bash
set -euo pipefail

echo "=== SuperVM Release Packaging (Unix) ==="
FEATURES=${FEATURES:-rocksdb-storage}
OUT_DIR=${OUT_DIR:-dist}
NAME=${NAME:-SuperVM}

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

export RUSTFLAGS="-C target-cpu=native"
echo "Building release with features: $FEATURES"
cargo build --release --features "$FEATURES"

STAMP=$(date +%Y%m%d_%H%M%S)
ARCH=$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)
PKG_NAME="$NAME-$ARCH-$STAMP"
STAGE="$ROOT_DIR/dist/$PKG_NAME"
mkdir -p "$STAGE/bin" "$STAGE/docs"

# Collect binaries (examples)
BIN="$ROOT_DIR/target/release/examples"
for t in storage_metrics_http persistence_consistency_test zk_parallel_http_bench routing_metrics_http_demo; do
  if [ -f "$BIN/$t" ]; then cp "$BIN/$t" "$STAGE/bin/"; fi
  if [ -f "$BIN/$t.exe" ]; then cp "$BIN/$t.exe" "$STAGE/bin/"; fi
done

# Copy other release bins if any
find "$ROOT_DIR/target/release" -maxdepth 1 -type f -perm -111 -printf "%f\n" 2>/dev/null | while read -r f; do
  case "$f" in
    *.d|*.so|*.rlib|*.a|*.dll|*.exe) ;; # skip libs
    *) cp -f "$ROOT_DIR/target/release/$f" "$STAGE/bin/" 2>/dev/null || true ;;
  esac
done

# Copy docs
for d in README.md ROADMAP.md LICENSE \
         docs/ROCKSDB-WINDOWS-DEPLOYMENT.md \
         docs/ROCKSDB-LINUX-DEPLOYMENT.md \
         docs/ROCKSDB-MACOS-DEPLOYMENT.md; do
  if [ -f "$d" ]; then
    mkdir -p "$STAGE/$(dirname "$d")"
    cp "$d" "$STAGE/$d"
  fi
done

mkdir -p "$OUT_DIR"
tar -C "$ROOT_DIR/dist" -czf "$OUT_DIR/$PKG_NAME.tar.gz" "$PKG_NAME"
echo "\n✅ Package created: $OUT_DIR/$PKG_NAME.tar.gz"
