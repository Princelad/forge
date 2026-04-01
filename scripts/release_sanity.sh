#!/usr/bin/env bash
set -euo pipefail

# Release readiness sanity checks:
# 1) release build
# 2) binary size threshold
# 3) benchmark harness compile checks

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "[1/3] Building release binary..."
cargo build --release

BIN_PATH="target/release/forge"
if [[ ! -f "$BIN_PATH" ]]; then
  echo "Release binary missing at $BIN_PATH"
  exit 1
fi

if stat --version >/dev/null 2>&1; then
  SIZE_BYTES="$(stat -c%s "$BIN_PATH")"
else
  SIZE_BYTES="$(stat -f%z "$BIN_PATH")"
fi

MAX_BYTES="${FORGE_RELEASE_MAX_BYTES:-12000000}"

echo "[2/3] Checking binary size..."
echo "Binary size: ${SIZE_BYTES} bytes"
echo "Threshold:   ${MAX_BYTES} bytes"

if (( SIZE_BYTES > MAX_BYTES )); then
  echo "Release binary exceeds threshold. Set FORGE_RELEASE_MAX_BYTES to adjust if needed."
  exit 1
fi

echo "[3/3] Compiling benchmark targets (sanity pass)..."
cargo bench --bench data_operations --bench git_operations --no-run

echo "Release sanity checks passed."
