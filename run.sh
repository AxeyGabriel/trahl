#!/bin/env sh

MODE=$1
ARGS=""

if [[ "$MODE" == "master" ]]; then
	ARGS="-m"
elif [[ "$MODE" == "worker" ]]; then
	ARGS="-w"
else
	ARGS="-m -w"
fi

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"

cargo run -- $ARGS -c "$SCRIPT_DIR/config.toml"
