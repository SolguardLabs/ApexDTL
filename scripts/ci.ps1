$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw "cargo is required" }
if (-not (Get-Command bun -ErrorAction SilentlyContinue)) { throw "bun is required" }

bun install --frozen-lockfile
cargo fmt --all -- --check
cargo build --all-targets --locked
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
bun run fmt:check
bun run build
bun test --timeout 30000 ./tests/node
