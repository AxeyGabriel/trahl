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
$FFMPEG -y -f lavfi -i color=c=red:s=320x240:d=1 -c:v h264 -t 1 "$TARGET_DIR/red_320x240_h264_1s.mp4"
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
	s_trahl.delay_msec(1000)
end
EOF


