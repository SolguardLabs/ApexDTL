$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

cargo test --all-targets --locked
bun test --timeout 30000 ./tests/node
