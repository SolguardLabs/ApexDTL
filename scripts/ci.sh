#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"

add_path() {
    if [[ -d "$1" ]]; then
        export PATH="$1:$PATH"
    fi
}

add_path "$HOME/.cargo/bin"
add_path "$HOME/.bun/bin"
add_path "/mnt/c/Program Files/nodejs"

if [[ -d "/mnt/c/Users" ]]; then
    for WINDOWS_HOME in /mnt/c/Users/*; do
        add_path "$WINDOWS_HOME/.cargo/bin"
        add_path "$WINDOWS_HOME/.bun/bin"
    done
fi

if command -v wslpath >/dev/null 2>&1 && [[ -n "${USERPROFILE:-}" ]]; then
    WINDOWS_PROFILE="$(wslpath "$USERPROFILE" 2>/dev/null || true)"
    if [[ -n "$WINDOWS_PROFILE" ]]; then
        add_path "$WINDOWS_PROFILE/.cargo/bin"
        add_path "$WINDOWS_PROFILE/.bun/bin"
    fi
fi

resolve_cmd() {
    local name="$1"
    local path=""

    if command -v "$name" >/dev/null 2>&1; then
        path="$(command -v "$name")"
        if "$path" --version >/dev/null 2>&1; then
            echo "$path"
            return 0
        fi
    fi

    if command -v "$name.exe" >/dev/null 2>&1; then
        path="$(command -v "$name.exe")"
        if "$path" --version >/dev/null 2>&1; then
            echo "$path"
            return 0
        fi
    fi

    return 1
}

if [[ -z "${CARGO_BIN:-}" ]] && ! CARGO_BIN="$(resolve_cmd cargo)"; then
    echo "No se encontro un cargo ejecutable en PATH." >&2
    exit 127
fi

if [[ -z "${BUN_BIN:-}" ]] && ! BUN_BIN="$(resolve_cmd bun)"; then
    echo "No se encontro un bun ejecutable en PATH." >&2
    echo "En Windows usa Git Bash o PowerShell con 'bun run ci'. En WSL instala Bun Linux." >&2
    exit 127
fi

if [[ -z "${NODE_BIN:-}" ]] && NODE_BIN="$(resolve_cmd node)"; then
    export NODE_BIN
fi

echo "==> Instalando dependencias JavaScript"
"$BUN_BIN" install --frozen-lockfile

echo "==> Verificando formato Rust"
"$CARGO_BIN" fmt --all -- --check

echo "==> Compilando Rust"
"$CARGO_BIN" build --all-targets --locked

echo "==> Ejecutando tests Rust"
"$CARGO_BIN" test --locked

echo "==> Ejecutando clippy"
"$CARGO_BIN" clippy --all-targets --all-features --locked -- -D warnings

echo "==> Verificando formato JavaScript"
"$BUN_BIN" run fmt:check

echo "==> Verificando JavaScript"
"$BUN_BIN" run build

echo "==> Ejecutando tests JavaScript"
"$BUN_BIN" test --timeout 30000 ./tests/node
