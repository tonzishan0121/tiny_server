#!/usr/bin/env bash
# Start tiny_server in development mode (Linux / macOS).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"
cargo build

exec ./target/debug/tiny_server
