#!/usr/bin/env bash
# Builds a trimmed FFmpeg (LGPL, static) for silky-runtime's `ffmpeg` feature:
# the ASF demuxer with the VC-1 and WMA Pro decoders, nothing else.
#
#   tools/ffmpeg/build.sh [prefix]        # default prefix: <workspace>/target/ffmpeg
#
# Environment:
#   FFMPEG_SRC   an existing FFmpeg source tree to build instead of fetching one
#   FFMPEG_TAG   release tag to fetch (default below)
#   JOBS         parallel jobs (default: all cores)
set -euo pipefail

TAG="${FFMPEG_TAG:-n7.1}"
# The commit each supported tag points at; a fetch that differs is refused.
COMMIT_n7_1="b08d7969c550a804a59511c7b83f2dd8cc0499b8"

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
prefix="${1:-$root/target/ffmpeg}"
jobs="${JOBS:-$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 2)}"
work="$root/target/ffmpeg-build"

src="${FFMPEG_SRC:-}"
if [ -z "$src" ]; then
    src="$work/src"
    if [ ! -d "$src/.git" ]; then
        mkdir -p "$work"
        git clone --depth 1 --branch "$TAG" https://github.com/FFmpeg/FFmpeg.git "$src"
    fi
    want_var="COMMIT_${TAG//[.-]/_}"
    want="${!want_var:-}"
    if [ -n "$want" ] && [ "$(git -C "$src" rev-parse HEAD)" != "$want" ]; then
        echo "FFmpeg $TAG is not the commit this script pins ($want)" >&2
        exit 1
    fi
fi

opts=(
    --prefix="$prefix"
    --disable-everything --disable-programs --disable-doc --disable-network
    --disable-autodetect --disable-avdevice --disable-swscale --disable-swresample
    --disable-postproc --disable-avfilter
    --enable-static --disable-shared --enable-pic
    --enable-demuxer=asf --enable-decoder=vc1,wmapro --enable-parser=vc1
)
if ! command -v nasm >/dev/null 2>&1 && ! command -v yasm >/dev/null 2>&1; then
    opts+=(--disable-x86asm)
fi

mkdir -p "$work/obj"
cd "$work/obj"
"$src/configure" "${opts[@]}"
make -j"$jobs"
make install

echo
du -h "$prefix"/lib/libav*.a
