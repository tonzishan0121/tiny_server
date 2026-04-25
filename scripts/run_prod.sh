#!/usr/bin/env bash
# Start tiny_server in release mode (Linux / macOS).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"
cargo build --release

exec ./target/release/tiny_server
