#!/bin/env sh

set -xe

FFPROBE=`which ffprobe`
FFMPEG=`which ffmpeg`
DD=`which dd`

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/test-resources"

rm -rf "$TARGET_DIR"
mkdir "$TARGET_DIR"

# Create sample media files
$FFMPEG -y -f lavfi -i color=c=red:s=320x240:d=600 -c:v h264 -t 1 "$TARGET_DIR/red_320x240_h264_1s.mp4"
$FFMPEG -y -f lavfi -i color=c=red:s=320x240:d=1 -c:v libx265 -t 1 "$TARGET_DIR/red_320x240_h265_1s.mp4"

# Create known sized files
$DD if=/dev/urandom of="$TARGET_DIR/100_bytes_file.bin" bs=1 count=100

# Create test lua script
cat > "$TARGET_DIR/test.lua" <<EOF
_trahl.log(_trahl.INFO, "Hello World from Lua")
EOF

cat > "$TARGET_DIR/test2.lua" <<EOF
for i = 1, 10 do
	_trahl.log(_trahl.INFO, "Count is " .. i)
	_trahl.delay_msec(1000)
end
EOF

cat > "$TARGET_DIR/test_transcode.lua" <<EOF
local util = require("util")
local srcfile = _trahl.vars.SRCFILE
local cachedir = _trahl.vars.CACHEDIR
local dstdir = _trahl.vars.DSTDIR
local outfile = cachedir .. "/out.mkv"
local wh = "https://discord.com/api/webhooks/1422425509999935583/h5mDwqjxXW59abMokj1_mOCO2INiFfeEdYixKgknVl_He2N3XxhX2muZQvZu_qQakjtw"

_trahl.log(_trahl.INFO, "filename: " .. srcfile)
local size = util.file_size(srcfile)
_trahl.log(_trahl.INFO, "size: " .. size .. " bytes")
_trahl.log(_trahl.INFO, "cache_dir: " .. cachedir)
_trahl.log(_trahl.INFO, "dst_dir: " .. dstdir)

local args = {
	"-sssi", srcfile,
	"-c:v", "libx265",
	"-preset", "medium",
	"-crf", "28",
	outfile
}

local probe = _trahl.ffprobe(srcfile)
local codec = probe.streams[1].codec_long_name or ""
if codec:lower():find("hevc") or codec:lower():find("h.265") then
	_trahl.log(_trahl.INFO, "Codec is H.265")
	util.discord_message(wh, "Hello From Lua")
	return
else
	_trahl.log(_trahl.INFO, "Codec is not H.265")

	local ok = pcall(function()
		return _trahl.ffmpeg(args)
	end)

	if not ok then
		util.panic("FFMPEG failed")
	end

	_trahl.set_output(outfile, _trahl.O_PRESERVE_DIR)
end
EOF
