#!/bin/bash
# ============================================================
# Download sidecars (yt-dlp + ffmpeg) for Tauri bundling
#
# Usage:
#   ./scripts/download-sidecars.sh          # Download for current platform
#   ./scripts/download-sidecars.sh windows  # Download Windows binaries (for CI)
#   ./scripts/download-sidecars.sh macos    # Download macOS binaries
#   ./scripts/download-sidecars.sh all      # Download for all platforms
# ============================================================

set -e

BINARIES_DIR="src-tauri/binaries"
mkdir -p "$BINARIES_DIR"

# yt-dlp latest release
YTDLP_VERSION="2024.12.23"
YTDLP_BASE="https://github.com/yt-dlp/yt-dlp/releases/latest/download"

# ffmpeg static builds
FFMPEG_BASE="https://github.com/eugeneware/ffmpeg-static/releases/latest/download"

download_windows() {
    echo "📥 Downloading Windows binaries..."

    # yt-dlp Windows
    echo "  → yt-dlp (Windows x86_64)"
    curl -L -o "$BINARIES_DIR/yt-dlp-x86_64-pc-windows-msvc.exe" \
        "$YTDLP_BASE/yt-dlp.exe"

    # ffmpeg Windows
    echo "  → ffmpeg (Windows x86_64)"
    curl -L -o "$BINARIES_DIR/ffmpeg-x86_64-pc-windows-msvc.exe" \
        "$FFMPEG_BASE/ffmpeg-win32-x64"

    echo "✅ Windows binaries ready"
}

download_macos_arm() {
    echo "📥 Downloading macOS ARM binaries..."

    # yt-dlp macOS universal
    echo "  → yt-dlp (macOS ARM)"
    curl -L -o "$BINARIES_DIR/yt-dlp-aarch64-apple-darwin" \
        "$YTDLP_BASE/yt-dlp_macos"
    chmod +x "$BINARIES_DIR/yt-dlp-aarch64-apple-darwin"

    # ffmpeg macOS ARM
    echo "  → ffmpeg (macOS ARM)"
    curl -L -o "$BINARIES_DIR/ffmpeg-aarch64-apple-darwin" \
        "$FFMPEG_BASE/ffmpeg-darwin-arm64"
    chmod +x "$BINARIES_DIR/ffmpeg-aarch64-apple-darwin"

    echo "✅ macOS ARM binaries ready"
}

download_macos_intel() {
    echo "📥 Downloading macOS Intel binaries..."

    # yt-dlp macOS universal
    echo "  → yt-dlp (macOS Intel)"
    curl -L -o "$BINARIES_DIR/yt-dlp-x86_64-apple-darwin" \
        "$YTDLP_BASE/yt-dlp_macos"
    chmod +x "$BINARIES_DIR/yt-dlp-x86_64-apple-darwin"

    # ffmpeg macOS Intel
    echo "  → ffmpeg (macOS Intel)"
    curl -L -o "$BINARIES_DIR/ffmpeg-x86_64-apple-darwin" \
        "$FFMPEG_BASE/ffmpeg-darwin-x64"
    chmod +x "$BINARIES_DIR/ffmpeg-x86_64-apple-darwin"

    echo "✅ macOS Intel binaries ready"
}

# Detect current platform or use argument
TARGET="${1:-auto}"

case "$TARGET" in
    windows)
        download_windows
        ;;
    macos)
        download_macos_arm
        download_macos_intel
        ;;
    all)
        download_windows
        download_macos_arm
        download_macos_intel
        ;;
    auto)
        OS="$(uname -s)"
        ARCH="$(uname -m)"
        case "$OS" in
            Darwin)
                if [ "$ARCH" = "arm64" ]; then
                    download_macos_arm
                else
                    download_macos_intel
                fi
                ;;
            MINGW*|MSYS*|CYGWIN*)
                download_windows
                ;;
            *)
                echo "❌ Unsupported platform: $OS $ARCH"
                echo "Usage: $0 [windows|macos|all]"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "Usage: $0 [windows|macos|all|auto]"
        exit 1
        ;;
esac

echo ""
echo "📦 Sidecars in $BINARIES_DIR:"
ls -lh "$BINARIES_DIR/"
echo ""
echo "🎉 Done! You can now run 'cargo tauri build' to bundle everything."
