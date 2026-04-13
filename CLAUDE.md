# Voyage DL

## Context

Desktop app for a kid travelling without a phone. The idea: download music as MP3 from YouTube or Deezer before leaving, so it can be played offline.

Initial inspiration: https://github.com/TannerNelson16/playlistdl (a Python/Flask web app). This project keeps the same idea but ships as a native desktop app built with Tauri.

**Note:** the app UI is in French on purpose (it's meant for the author's kid). Code, comments, commits and docs are in English.

## What the app does

1. The user pastes a URL (YouTube video/playlist, or Deezer playlist)
2. The app parses the URL and fetches the track list
3. For a playlist, the user picks which songs to download
4. URLs of the selected songs are shown
5. Download as MP3 (or M4A) into the chosen folder

## Tech stack

- **Frontend**: React 18 + TypeScript + Vite
- **Backend**: Rust via Tauri v2
- **Downloading**: `yt-dlp` (audio extraction) + `ffmpeg` (MP3 conversion), bundled as Tauri sidecars
- **Deezer**: Deezer public API to fetch playlists, then YouTube lookup via `yt-dlp ytsearch` for each track

## Layout

```
src/                    # React + TypeScript frontend
  App.tsx               # Top-level component, screen routing
  components/
    SetupScreen.tsx     # First launch: pick the download folder
    MainScreen.tsx      # Main screen: URL input + analysis
    TrackList.tsx       # Track list with selection
    DownloadProgress.tsx # Download progress
    DownloadQueue.tsx   # Download queue
    Settings.tsx        # Settings modal
    Alert.tsx           # Alert component
  hooks/
    useConfig.ts        # Config management hook

src-tauri/              # Rust + Tauri v2 backend
  src/
    main.rs             # Entry point
    lib.rs              # Module exports
    commands/
      mod.rs            # Shared types (TrackInfo, DownloadSummary, Config)
      config.rs         # Config read/save
      youtube.rs        # YouTube URL analysis via yt-dlp
      deezer.rs         # Deezer integration
      download.rs       # MP3 download with progress
    utils/
      sidecar.rs        # Bundled yt-dlp/ffmpeg binary management
  scripts/
    build-ffmpeg-slim.sh # Custom slim ffmpeg build (macOS only)
```

## Dev prerequisites

- Node.js >= 18
- Rust >= 1.70
- `yt-dlp` and `ffmpeg` on PATH (for dev) — in production the app ships its own sidecars
- Tauri CLI v2: `cargo install tauri-cli --version "^2"`

## Commands

```bash
npm install             # Install frontend deps
cargo tauri dev         # Run in dev mode
cargo tauri build       # Production build (.dmg / .msi)
```

## Conventions

- UI copy is French on purpose (see Context). Everything else — code, comments, commit messages, docs — is English.
- Shared Rust types live in `commands/mod.rs`
- External binaries (yt-dlp, ffmpeg) are managed through Tauri's sidecar system
- On macOS, ffmpeg is built slim from source (~2.5 MB) via `src-tauri/scripts/build-ffmpeg-slim.sh`; on Windows a prebuilt fat binary is downloaded
