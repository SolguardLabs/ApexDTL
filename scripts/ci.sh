#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 127; }
command -v bun >/dev/null 2>&1 || { echo "bun is required" >&2; exit 127; }

export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"

echo "==> Install locked JavaScript dependencies"
bun install --frozen-lockfile
echo "==> Check Rust formatting"
cargo fmt --all -- --check
echo "==> Build all Rust targets"
cargo build --all-targets --locked
echo "==> Run Rust tests"
cargo test --all-targets --locked
echo "==> Run Clippy"
cargo clippy --all-targets --all-features --locked -- -D warnings
echo "==> Check JavaScript formatting and syntax"
bun run fmt:check
bun run build
echo "==> Run JavaScript integration tests"
bun test --timeout 30000 ./tests/node
