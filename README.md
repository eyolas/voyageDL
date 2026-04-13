# Voyage DL

Desktop app to download music from YouTube and Deezer playlists as MP3 files for offline listening.

Built with **Tauri v2** (Rust backend + React frontend), using `yt-dlp` and `ffmpeg` under the hood.

## Features

- Paste a YouTube video/playlist URL or a Deezer playlist URL
- Automatic track listing with selection (pick which songs to download)
- MP3 download with real-time progress tracking
- Download queue management
- Deezer integration: fetches playlist tracks via the Deezer API, then finds them on YouTube for download
- Configurable download folder
- Cross-platform: macOS (.dmg) and Windows (.msi / .nsis)

## Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://www.rust-lang.org/tools/install) >= 1.70
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) in your PATH
- [ffmpeg](https://ffmpeg.org/download.html) in your PATH
- Tauri CLI v2:
  ```bash
  cargo install tauri-cli --version "^2"
  ```

## Getting Started

```bash
# Install frontend dependencies
npm install

# Run in development mode (frontend + backend)
cargo tauri dev
```

The app opens automatically. In dev mode, `yt-dlp` and `ffmpeg` must be in your PATH.

## Build for Production

```bash
cargo tauri build
```

- **macOS**: `.dmg` in `src-tauri/target/release/bundle/dmg/`
- **Windows**: `.msi` in `src-tauri/target/release/bundle/msi/`, `.exe` in `src-tauri/target/release/bundle/nsis/`

### Bundling yt-dlp & ffmpeg

For a standalone app, include the binaries as Tauri sidecars:

```bash
mkdir -p src-tauri/binaries

# macOS (Apple Silicon)
cp $(which yt-dlp) src-tauri/binaries/yt-dlp-aarch64-apple-darwin
cp $(which ffmpeg) src-tauri/binaries/ffmpeg-aarch64-apple-darwin

# macOS (Intel)
# yt-dlp-x86_64-apple-darwin, ffmpeg-x86_64-apple-darwin

# Windows
# yt-dlp-x86_64-pc-windows-msvc.exe, ffmpeg-x86_64-pc-windows-msvc.exe
```

File names must follow the format: `{name}-{target-triple}[.exe]`. Tauri automatically selects the right binary for the platform.

## Tech Stack

| Layer    | Technology                                      |
| -------- | ----------------------------------------------- |
| Frontend | React 18 + TypeScript + Vite                    |
| Backend  | Rust via Tauri v2                                |
| Audio    | yt-dlp (extraction) + ffmpeg (MP3 conversion)   |
| Deezer   | Deezer public API + yt-dlp ytsearch             |

## Project Structure

```
src/                          # Frontend (React + TypeScript)
├── App.tsx                   # Main component, screen routing
├── components/
│   ├── SetupScreen.tsx       # First launch: choose download folder
│   ├── MainScreen.tsx        # Main screen: URL input & analysis
│   ├── TrackList.tsx         # Track list with selection
│   ├── DownloadProgress.tsx  # Download progress display
│   ├── DownloadQueue.tsx     # Download queue
│   ├── Settings.tsx          # Settings modal
│   └── Alert.tsx             # Alert component
└── hooks/
    └── useConfig.ts          # Config management hook

src-tauri/                    # Backend (Rust + Tauri v2)
├── src/
│   ├── main.rs               # Entry point
│   ├── lib.rs                # Module exports
│   ├── commands/
│   │   ├── mod.rs            # Shared types (TrackInfo, DownloadSummary, Config)
│   │   ├── config.rs         # Config read/write
│   │   ├── youtube.rs        # YouTube URL analysis via yt-dlp
│   │   ├── deezer.rs         # Deezer integration
│   │   └── download.rs       # MP3 download with progress
│   └── utils/
│       └── sidecar.rs        # yt-dlp/ffmpeg sidecar management
└── tauri.conf.json           # Tauri configuration
```

## How It Works

1. **YouTube**: calls `yt-dlp --dump-json` to fetch metadata, then `yt-dlp -x --audio-format mp3` to download
2. **Deezer**: fetches the playlist via the Deezer public API, searches each track on YouTube with `yt-dlp "ytsearch1:artist title"`, and downloads the result as MP3

## License

MIT
