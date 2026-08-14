#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 127; }
command -v bun >/dev/null 2>&1 || { echo "bun is required" >&2; exit 127; }

cargo test --all-targets --locked
bun test --timeout 30000 ./tests/node
