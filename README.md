<p align="center">
  <img src="assets/Rustify-logo.png" alt="Rustify logo" width="320" />
</p>

<h1 align="center">Rustify</h1>

<p align="center"><strong>Local-First Rust Media Conversion Framework.</strong><br/>One shared core for CLI, Desktop GUI, and local Web usage.</p>

<p align="center">
  <img alt="release" src="https://img.shields.io/badge/release-v1.0.1-red" />
  <img alt="license" src="https://img.shields.io/badge/license-MIT-red" />
  <img alt="written in" src="https://img.shields.io/badge/written%20in-Rust-orange" />
  <img alt="workspace" src="https://img.shields.io/badge/workspace-cli%20%7C%20core%20%7C%20desktop%20%7C%20web--backend-blue" />
</p>

<p align="center">
  <img alt="single video" src="https://img.shields.io/badge/single%20video-YouTube-success" />
  <img alt="playlist" src="https://img.shields.io/badge/playlist-YouTube%20%2B%20Spotify-success" />
  <img alt="formats" src="https://img.shields.io/badge/formats-mp3%20flac%20wav%20aac%20ogg%20mp4%20webm-success" />
  <img alt="dependencies" src="https://img.shields.io/badge/engine-yt--dlp%20%2B%20ffmpeg-blue" />
</p>

<p align="center">
  <img alt="auth" src="https://img.shields.io/badge/auth-public%20first%20%2B%20optional%20browser%20session-informational" />
  <img alt="web default" src="https://img.shields.io/badge/web%20default-browser%20cookies%20disabled-important" />
  <img alt="resume" src="https://img.shields.io/badge/playlist%20reruns-resume%20friendly-9cf" />
</p>

---

<p align="center">
  <a href="#what-is-this">What is this</a> |
  <a href="#install">Install</a> |
  <a href="#quick-start">Quick Start</a> |
  <a href="#commands">Commands</a> |
  <a href="#desktop-gui">Desktop GUI</a> |
  <a href="#local-web-backend">Local Web</a> |
  <a href="#spotify-playlists">Spotify</a> |
  <a href="#configuration">Configuration</a> |
  <a href="#architecture">Architecture</a> |
  <a href="#faq">FAQ</a>
</p>

---

## What is this

Rustify is a local-first media converter built in Rust.

It uses:
- `yt-dlp` for media extraction
- `ffmpeg` for remuxing/transcoding
- one shared Rust core so CLI, desktop, and web stay aligned

Rustify supports:
- single YouTube video conversion
- YouTube playlist conversion
- Spotify playlist import that resolves tracks into YouTube-backed conversion jobs

Important: Rustify does not download media directly from Spotify.

## Setup Guide

Follow these steps in order to get Rustify running on your machine.

### 1. Install Dependencies

Rustify requires `ffmpeg` and `yt-dlp` to be installed on your system.

**Windows:**
```powershell
# Install via winget (recommended)
winget install Gyan.FFmpeg
python -m pip install yt-dlp

# Verify installation
ffmpeg -version
yt-dlp --version
```

**Linux / macOS:**
```bash
# Install via your package manager (e.g., brew or apt)
brew install ffmpeg
python3 -m pip install yt-dlp

# Verify
ffmpeg -version
yt-dlp --version
```

### 2. Download and Install

```powershell
# Clone the repository
git clone https://github.com/Karmanya03/Rustify.git
cd Rustify

# Install the CLI command globally (optional)
cargo install --path cli --locked
```

### 3. Quick Configuration

Run the auto-setup script for your OS to automatically detect your browser and set up the default download folder.

**Windows:**
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-windows.ps1
```

**Linux / macOS:**
```bash
chmod +x scripts/setup-linux.sh
./scripts/setup-linux.sh
```

**Headless (Server):**
```bash
./scripts/setup-linux.sh --headless
```

### 4. Running Rustify

You can run Rustify in three ways:

*   **CLI:** Use the `rustify` command (if installed) or `cargo run -p rustify-cli`.
*   **Desktop App:** `cargo run -p rustify-desktop`
*   **Web Interface:** `cargo run -p web-backend` (then open `http://127.0.0.1:3001`)

---

## Quick Start Examples

```powershell
# Run a dependency check
rustify doctor

# Convert a single YouTube video to MP3 (320kbps)
rustify convert "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format mp3 --quality 320

# Convert a Spotify playlist to FLAC
rustify playlist "https://open.spotify.com/playlist/..." --format flac --quality lossless
```

Available commands:
- `convert` - convert one video
- `playlist` - convert YouTube or Spotify playlists
- `batch` - convert playlist to multiple formats
- `info` - inspect metadata
- `quality` - list available source qualities
- `config` - show/set/reset configuration
- `doctor` - dependency and auth diagnostics

Examples:

```powershell
# single video to MP3
rustify convert "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format mp3 --quality 320

# Spotify playlist to FLAC
rustify playlist "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M" --format flac --quality lossless

# batch playlist into multiple formats
rustify batch "https://www.youtube.com/playlist?list=YOUR_LIST_ID" --formats "mp3,flac,mp4" --qualities "mp3:320,flac:lossless,mp4:1080p"

# inspect metadata as JSON
rustify info "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format json

# quality inspection
rustify quality "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

## Desktop GUI

Run desktop app:

```powershell
cargo run -p rustify-desktop
```

Or:

```powershell
cd desktop
cargo tauri dev
```

Flow:
1. Paste a YouTube video URL or YouTube/Spotify playlist URL
2. Choose format and quality
3. Pick output folder
4. Start conversion and track progress

## Local Web Backend

Run backend:

```powershell
cargo run -p web-backend
```

Open: http://127.0.0.1:3001

The backend serves the site from `dist/`.
Use the server, not direct file-open of `dist/index.html`.

Safer web default:
- browser cookie auto-read is disabled
- backend runs in public-first mode

Optional local override:

```powershell
$env:RUSTIFY_WEB_ALLOW_BROWSER_COOKIES="true"
cargo run -p web-backend
```

## Spotify Playlists

Supported inputs:
- `https://open.spotify.com/playlist/...`
- `spotify:playlist:...`

How it works:
1. Rustify resolves playlist metadata
2. each track becomes a YouTube-backed conversion job
3. output files are zero-padded and deterministic for reruns

Large playlist behavior:
- paging support
- retry/backoff on `429` and `5xx`
- `Retry-After` support
- resume-friendly reruns by skipping existing non-empty files

Chunking example:

```powershell
rustify playlist "SPOTIFY_OR_YOUTUBE_PLAYLIST" --format mp3 --quality 320 --start 1 --limit 250
rustify playlist "SPOTIFY_OR_YOUTUBE_PLAYLIST" --format mp3 --quality 320 --start 251 --limit 250
rustify playlist "SPOTIFY_OR_YOUTUBE_PLAYLIST" --format mp3 --quality 320 --start 501 --limit 250
```

### Manual Configuration

If you want to customize settings manually, use the `rustify config` command:

```powershell
# Show current config
rustify config show

# Set a custom download directory
rustify config set download_dir "D:\Media\Rustify"

# Set authentication mode
rustify config set auth.mode browser
rustify config set auth.browser chrome

# Set Spotify integration (resolves tracks via YouTube)
rustify config set spotify.enabled true
```

**Config File Location:**
- **Windows:** `%APPDATA%\rustify\config.json`
- **Linux/macOS:** `~/.config/rustify/config.json`

Environment variable overrides:
- `YTDLP_PATH`
- `FFMPEG_PATH`
- `DOWNLOADS_DIR`
- `RUSTIFY_WEB_ALLOW_BROWSER_COOKIES`

## Architecture

Workspace crates:
- `core/` - shared conversion engine
- `cli/` - command line interface
- `desktop/` - Tauri desktop app
- `web-backend/` - Axum backend for local web UI

Feature matrix:

| Capability                   | CLI | Desktop  | Local Web    |
| ---------------------------- | --- | -------- | ------------ |
| Single YouTube conversion    | Yes | Yes      | Yes          |
| YouTube playlist conversion  | Yes | Yes      | Yes          |
| Spotify playlist import      | Yes | Yes      | Yes          |
| MP3 / FLAC / WAV / AAC / OGG | Yes | Yes      | Yes          |
| MP4 / WebM video output      | Yes | Yes      | Yes          |
| Browser-session auth reuse   | Yes | Yes      | Opt-in       |
| Dependency diagnostics       | Yes | Indirect | API endpoint |

## FAQ

### Is this for local use only?
Yes. Rustify is designed for local-first workflows.

### Does Rustify bypass restrictions with hardcoded cookies?
No. Hardcoded cookies are intentionally not used.

### Why do some videos fail in web mode but work in desktop/CLI?
Web mode defaults to safer public-first behavior. Restricted videos can need local browser-session reuse.

### Can Rustify create true lossless quality from lossy sources?
No. FLAC preserves decoded source quality but cannot create fidelity beyond the source stream.

### How do I run quality gates before release?

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Development

```powershell
cargo check --workspace
cargo test -p rustify-core --offline
cargo run -p rustify-cli -- doctor
cargo run -p rustify-desktop
cargo run -p web-backend
```

## License

MIT - see [LICENSE](LICENSE).
