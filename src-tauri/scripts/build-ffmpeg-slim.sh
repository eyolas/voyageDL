#!/usr/bin/env bash
# Build a slim ffmpeg supporting only what Voyage DL needs:
#   - typical YouTube audio decoders (opus, vorbis, aac, mp3, flac)
#   - image decoders for album covers (mjpeg, png)
#   - MP3 encoder via libmp3lame (static)
#   - minimal muxers/demuxers (mp3, m4a/mp4, webm/matroska, ogg, image2)
#
# Output: binary placed at src-tauri/binaries/ffmpeg-<triple>
#
# macOS prerequisites:
#   brew install lame pkg-config
#   xcode-select --install

set -euo pipefail

FFMPEG_VERSION="${FFMPEG_VERSION:-7.1.1}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAURI_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$TAURI_DIR/.build-ffmpeg}"
OUT_DIR="$TAURI_DIR/binaries"

# Detect the Rust target triple (used by Tauri to name the sidecar)
HOST_TRIPLE="$(rustc -vV | awk '/host:/ {print $2}')"
if [[ -z "$HOST_TRIPLE" ]]; then
  echo "Could not detect Rust target triple (rustc missing?)" >&2
  exit 1
fi

echo "==> ffmpeg $FFMPEG_VERSION for $HOST_TRIPLE"
echo "    build dir : $BUILD_DIR"
echo "    out       : $OUT_DIR/ffmpeg-$HOST_TRIPLE"

# Static libmp3lame via Homebrew (requires: brew install lame)
LAME_PREFIX="$(brew --prefix lame 2>/dev/null || true)"
if [[ -z "$LAME_PREFIX" || ! -f "$LAME_PREFIX/lib/libmp3lame.a" ]]; then
  echo "libmp3lame.a not found. Install it with: brew install lame" >&2
  exit 1
fi

mkdir -p "$BUILD_DIR" "$OUT_DIR"
cd "$BUILD_DIR"

# Homebrew ships both .a and .dylib in the same lib/ dir. On macOS ld prefers
# the .dylib when both exist, so we build a prefix that only contains the
# static archive. That forces ffmpeg to link libmp3lame statically.
LAME_STATIC="$BUILD_DIR/lame-static"
mkdir -p "$LAME_STATIC/lib" "$LAME_STATIC/include"
cp "$LAME_PREFIX/lib/libmp3lame.a" "$LAME_STATIC/lib/"
rsync -a "$LAME_PREFIX/include/" "$LAME_STATIC/include/"

# Download sources if needed
SRC_DIR="ffmpeg-$FFMPEG_VERSION"
if [[ ! -d "$SRC_DIR" ]]; then
  TARBALL="ffmpeg-$FFMPEG_VERSION.tar.xz"
  if [[ ! -f "$TARBALL" ]]; then
    echo "==> Download ffmpeg $FFMPEG_VERSION"
    curl -LO "https://ffmpeg.org/releases/$TARBALL"
  fi
  tar xf "$TARBALL"
fi

cd "$SRC_DIR"

# Minimal configuration.
# - start from --disable-everything and enable only what we use
# - libmp3lame linked statically (extra-ldflags points at the archive)
# - no network, no device, no programs other than ffmpeg
if [[ ! -f ffbuild/config.mak ]] || [[ "${FORCE_RECONFIGURE:-0}" == "1" ]]; then
  echo "==> configure"
  ./configure \
    --prefix="$BUILD_DIR/install" \
    --pkg-config-flags=--static \
    --extra-cflags="-I$LAME_STATIC/include" \
    --extra-ldflags="-L$LAME_STATIC/lib" \
    --disable-everything \
    --disable-autodetect \
    --disable-debug \
    --disable-doc \
    --disable-htmlpages \
    --disable-manpages \
    --disable-podpages \
    --disable-txtpages \
    --disable-network \
    --disable-avdevice \
    --disable-postproc \
    --disable-swscale \
    --disable-programs \
    --enable-ffmpeg \
    --enable-small \
    --enable-pthreads \
    --enable-libmp3lame \
    --enable-encoder=libmp3lame,aac,pcm_s16le,mjpeg,png \
    --enable-decoder=mp3,mp3float,aac,aac_latm,opus,vorbis,flac,pcm_s16le,pcm_s16be,pcm_f32le,mjpeg,png \
    --enable-demuxer=mp3,mov,aac,ogg,matroska,flac,wav,image2,mjpeg_2000,mjpeg,aiff,ac3 \
    --enable-muxer=mp3,ipod,mp4,ogg,flac,wav,adts,matroska,image2 \
    --enable-parser=mpegaudio,aac,aac_latm,opus,vorbis,flac,mjpeg,png \
    --enable-protocol=file,pipe \
    --enable-filter=aresample,aformat,anull,atrim,atempo,volume,acopy,copy \
    --enable-bsf=aac_adtstoasc,null
fi

echo "==> make"
make -j"$(sysctl -n hw.ncpu)"

BIN_SRC="$BUILD_DIR/$SRC_DIR/ffmpeg"
BIN_DST="$OUT_DIR/ffmpeg-$HOST_TRIPLE"

cp "$BIN_SRC" "$BIN_DST"
strip -x "$BIN_DST"

# No ad-hoc signing here — Tauri's bundler signs the .app (and re-signs
# embedded sidecars) using the signingIdentity from tauri.conf.json.

SIZE="$(du -h "$BIN_DST" | awk '{print $1}')"
echo "==> OK: $BIN_DST ($SIZE)"
