#!/bin/env sh

set -xe

FFPROBE=`which ffprobe`
FFMPEG=`which ffmpeg`
DD=`which dd`

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/test-resources"
TEST_LIBRARY_DIR="$TARGET_DIR/test-library"

rm -rf "$TARGET_DIR"
mkdir "$TARGET_DIR"

# Create sample media files
$FFMPEG -y -f lavfi -i testsrc=size=1280x720:rate=30 -f lavfi -i sine=frequency=1000:sample_rate=44100 -t 60 -c:v libx264 -preset veryslow -crf 23 -c:a aac "$TARGET_DIR/red_320x240_h264_big.mp4"
$FFMPEG -y -f lavfi -i color=c=red:s=320x240:d=600 -c:v h264 -t 1 "$TARGET_DIR/red_320x240_h264_1s.mp4"
$FFMPEG -y -f lavfi -i color=c=red:s=320x240:d=1 -c:v libx265 -t 1 "$TARGET_DIR/red_320x240_h265_1s.mp4"

# Create known sized files
$DD if=/dev/urandom of="$TARGET_DIR/100_bytes_file.bin" bs=1 count=100

# Create test library
LVL1_N=20
LVL2_N=20
TEST1_N=5
TEST2_N=5
for i in $(seq 1 $LVL1_N); do
	(
    LEVEL1="$TEST_LIBRARY_DIR/dir_$i"
    mkdir -p "$LEVEL1"

    # Inner loop: 10 folders inside each level 1
    for j in $(seq 1 $LVL2_N); do
		echo "Creating directory: $i:$j"
        LEVEL2="$LEVEL1/subdir_$j"
        mkdir -p "$LEVEL2"

        echo "Generating files in: $LEVEL2"

        # 2 H.264 videos
        for k in $(seq 1 $TEST1_N); do
            TARGET="$LEVEL2/red_320x240_h264_big_$k.mp4"
			ln -s "$TARGET_DIR/red_320x240_h264_big.mp4" "$TARGET"
        done

        # 3 H.265 videos
        for k in $(seq 1 $TEST2_N); do
            TARGET="$LEVEL2/red_320x240_h265_1s_$k.mp4"
			ln -s "$TARGET_DIR/red_320x240_h265_1s.mp4" "$TARGET"
        done
    done
	) &
done
wait
for k in $(seq 1 $TEST1_N); do
	TARGET="$TEST_LIBRARY_DIR/red_320x240_h264_big_$k.mp4"
	ln -s "$TARGET_DIR/red_320x240_h264_big.mp4" "$TARGET"
done
for k in $(seq 1 $TEST2_N); do
	TARGET="$TEST_LIBRARY_DIR/red_320x240_h265_1s_$k.mp4"
	ln -s "$TARGET_DIR/red_320x240_h265_1s.mp4" "$TARGET"
done

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
local UTILS = require("utils")
local INTEGRATIONS = require("integrations")

local vars      = _trahl.vars
local srcfile   = vars.SRCFILE
local cachedir  = vars.CACHEDIR
local dstdir    = vars.DSTDIR
local remuxed   = string.format("%s/remux.mkv", cachedir)
local outfile   = string.format("%s/%s.mkv", cachedir, UTILS.strip_ext(UTILS.file_name(srcfile)))

-- FFmpeg argument builders
local function build_remux_args(input, output)
    return { "-i", input, "-c", "copy", output }
end

local function build_transcode_args(input, output)
    return {
        "-i", input,
        "-c:v", "libx265",
        "-preset", "medium",
        "-crf", "28",
        "-threads", "16",
        output
    }
end

-- Utility: run ffmpeg
local function run_ffmpeg(stage, duration, args)
    _trahl.milestone(stage)
    local ok, err = pcall(function()
        return _trahl.ffmpeg(duration, args)
    end)
    if not ok then
        UTILS.panic(string.format("FFmpeg failed during %s: %s", stage, err or "unknown error"))
    end
end

-- Step 1: Probe
_trahl.milestone("Probing")
local probe = _trahl.ffprobe(srcfile)
local stream = probe.streams[1]
local duration = tonumber(stream.duration) or 0
local codec = (stream.codec_long_name or ""):lower()

-- Step 2: Skip if already HEVC
if UTILS.matches_regex(codec, "(hevc|h.265)") then
    _trahl.log(_trahl.INFO, "Codec is already H.265, skipping transcode")
    INTEGRATIONS.discord_message(webhook, string.format("✅ %s is already H.265", UTILS.file_name(srcfile)))
    return
end

-- Step 3: Remux
run_ffmpeg("Remuxing to MKV", duration, build_remux_args(srcfile, remuxed))

-- Step 4: Transcode
_trahl.log(_trahl.INFO, "Transcoding to H.265")
run_ffmpeg("Transcoding", duration, build_transcode_args(remuxed, outfile))

-- Step 5: Output
_trahl.set_output(outfile, _trahl.O_PRESERVE_DIR)
INTEGRATIONS.discord_message(webhook, string.format("🎬 Transcoding complete: %s", UTILS.file_name(outfile)))
EOF
