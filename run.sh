#!/bin/env sh

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"

cargo run -- -m -w -c "$SCRIPT_DIR/config.toml"
