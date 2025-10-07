#!/bin/sh
set -e

: "${CONFIG_FILE:=/config.toml}"
: "${MODE:=}"

args=("-c" "$CONFIG_FILE")

if [[ "$MODE" == *master* ]]; then
	args+=("-m")
fi

if [[ "$MODE" == *worker* ]]; then
	args+=("-w")
fi

exec trahl "${args[@]}" "$@"
