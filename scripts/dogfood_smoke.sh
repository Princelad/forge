#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BIN_PATH="$ROOT_DIR/target/release/forge"
TIMEOUT_SECS="${FORGE_DOGFOOD_TIMEOUT_SECS:-3}"

if [[ ! -x "$BIN_PATH" ]]; then
  echo "Missing release binary at $BIN_PATH"
  echo "Run: cargo build --release"
  exit 1
fi

if [[ "$#" -lt 1 ]]; then
  echo "Usage: scripts/dogfood_smoke.sh <repo-path> [<repo-path> ...]"
  exit 1
fi

for repo in "$@"; do
  echo "== $repo =="
  (
    cd "$repo"
    set +e
    timeout "$TIMEOUT_SECS" "$BIN_PATH" >/tmp/forge-dogfood.out 2>/tmp/forge-dogfood.err
    code=$?
    set -e
    if [[ "$code" == "124" ]]; then
      echo "startup_ok_timeout"
    elif [[ "$code" == "0" ]]; then
      echo "startup_ok_exit"
    else
      echo "startup_error_${code}"
      if [[ -s /tmp/forge-dogfood.err ]]; then
        echo "stderr:"
        cat /tmp/forge-dogfood.err
      fi
    fi
  )
done
