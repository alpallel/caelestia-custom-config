#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release
install -Dm755 target/release/autoclicker-config "$HOME/.local/bin/autoclicker-config"
