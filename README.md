# Rustify

> Local-first Rust media converter with a shared core for CLI, desktop GUI, and local web usage.

Rustify keeps the website intact and adds native Rust app surfaces around the same engine:

- `rustify-cli` for automation and power use
- `rustify-desktop` for a local GUI
- `web-backend` for the existing website, served locally

It supports YouTube video conversion, YouTube playlist conversion, and Spotify playlist import that resolves each track into a YouTube-backed conversion job.

## Table of Contents

- [Overview](#overview)
- [Feature Matrix](#feature-matrix)
- [Project Layout](#project-layout)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage: CLI](#usage-cli)
- [Usage: Desktop GUI](#usage-desktop-gui)
- [Usage: Local Website](#usage-local-website)
- [Spotify Playlist Imports](#spotify-playlist-imports)
- [Large Playlists and Rate Limits](#large-playlists-and-rate-limits)
- [YouTube Auth and Cookies](#youtube-auth-and-cookies)
- [Audio Quality Notes](#audio-quality-notes)
- [Configuration](#configuration)
- [Development Commands](#development-commands)
- [Troubleshooting](#troubleshooting)
- [Security Notes](#security-notes)

## Overview

Rustify uses:

- `yt-dlp` for media extraction
- `ffmpeg` for remuxing and transcoding
- one shared Rust core so CLI, desktop, and web stay aligned

Spotify support is implemented as playlist import plus track matching. Rustify does not pull media directly from Spotify. It resolves the playlist metadata, then converts each item through a YouTube-backed search flow.

## Feature Matrix

| Capability | CLI | Desktop GUI | Local Website |
| --- | --- | --- | --- |
| Single YouTube video conversion | Yes | Yes | Yes |
| YouTube playlist conversion | Yes | Yes | Yes |
| Spotify playlist import and conversion | Yes | Yes | Yes |
| MP3 output | Yes | Yes | Yes |
| FLAC output | Yes | Yes | Yes |
| WAV output | Yes | Yes | Yes |
| AAC / OGG output | Yes | Yes | Yes |
| MP4 / WebM output | Yes | Yes | Yes |
| Browser-session auth reuse | Yes | Yes | Opt-in |
| Dependency diagnostics | Yes | Indirect | API endpoint |
| Resume-friendly reruns for playlists | Yes | Yes | Yes |

## Project Layout

```text
Rustify/
|-- cli/           # Rust CLI application
|-- core/          # Shared yt-dlp + ffmpeg engine
|-- desktop/       # Tauri desktop app
|-- dist/          # Existing website UI served by web-backend and desktop
`-- web-backend/   # Axum backend for the local website
```

## Prerequisites

Install these before running real downloads:

- Rust toolchain
- `ffmpeg`
- `yt-dlp`

### Windows

Install `yt-dlp`:

```powershell
python -m pip install yt-dlp
```

Check it:

```powershell
yt-dlp --version
python -m yt_dlp --version
```

Install `ffmpeg`:

```powershell
winget install Gyan.FFmpeg
```

Check it:

```powershell
ffmpeg -version
```

### Linux / macOS

Install `yt-dlp`:

```bash
python3 -m pip install yt-dlp
```

Install `ffmpeg` with your package manager, then verify:

```bash
yt-dlp --version
ffmpeg -version
```

## Installation

```powershell
git clone <your-repo-url>
cd Rustify
cargo check --workspace
```

Install the CLI locally:

```powershell
cargo install --path cli --locked
```

That installs the `rustify` command.

## Quick Start

### 1. Verify dependencies

```powershell
cargo run -p rustify-cli -- doctor
```

### 2. Convert one YouTube video to FLAC

```powershell
cargo run -p rustify-cli -- convert "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format flac --quality lossless
```

### 3. Inspect a Spotify playlist

```powershell
cargo run -p rustify-cli -- info "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"
```

### 4. Launch the desktop GUI

```powershell
cargo run -p rustify-desktop
```

### 5. Launch the website locally

```powershell
cargo run -p web-backend
```

Then open [http://127.0.0.1:3001](http://127.0.0.1:3001).

## Usage: CLI

General form:

```powershell
rustify <command> [options]
```

Or from source:

```powershell
cargo run -p rustify-cli -- <command> [options]
```

### Convert a single video

MP3:

```powershell
rustify convert "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format mp3 --quality 320
```

FLAC:

```powershell
rustify convert "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format flac --quality lossless
```

WAV:

```powershell
rustify convert "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format wav --quality hd
```

MP4:

```powershell
rustify convert "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format mp4 --quality 1080p
```

Custom output directory:

```powershell
rustify --output "D:\Media\Rustify" convert "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format flac --quality lossless
```

Custom output filename:

```powershell
rustify convert "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format mp3 --quality 320 --name "my-track"
```

### Convert a playlist

YouTube playlist:

```powershell
rustify playlist "https://www.youtube.com/playlist?list=YOUR_LIST_ID" --format mp3 --quality 320
```

Spotify playlist:

```powershell
rustify playlist "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M" --format flac --quality lossless
```

Run a slice of a large playlist:

```powershell
rustify playlist "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M" --format mp3 --quality 320 --start 201 --limit 100
```

### Batch convert a playlist into multiple formats

```powershell
rustify batch "https://www.youtube.com/playlist?list=YOUR_LIST_ID" --formats "mp3,flac,mp4" --qualities "mp3:320,flac:lossless,mp4:1080p"
```

Spotify batch:

```powershell
rustify batch "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M" --formats "mp3,flac" --qualities "mp3:320,flac:lossless"
```

### Inspect metadata

```powershell
rustify info "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
rustify info "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format json
rustify info "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"
```

### Inspect available source qualities

```powershell
rustify quality "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
rustify quality "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --audio-only
rustify quality "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --video-only
```

### Dependency and auth diagnostics

```powershell
rustify doctor
```

## Usage: Desktop GUI

Run it:

```powershell
cargo run -p rustify-desktop
```

Or from the desktop folder:

```powershell
cd desktop
cargo tauri dev
```

Desktop flow:

1. Paste a YouTube video URL or a YouTube / Spotify playlist URL.
2. Choose format and quality.
3. Pick an output folder if needed.
4. Start conversion.
5. Track progress locally in-app.

Recommended desktop use:

- videos that need local browser-session reuse
- users who want local auth without CLI setup
- large playlist jobs run locally on the same machine

## Usage: Local Website

Run the backend:

```powershell
cargo run -p web-backend
```

Open [http://127.0.0.1:3001](http://127.0.0.1:3001).

The backend serves the existing website from `dist/`, so normal usage should go through the local server rather than opening `dist/index.html` directly.

Website flow:

1. Start the backend.
2. Open the local URL.
3. Paste a YouTube video URL or a YouTube / Spotify playlist URL.
4. Choose format and quality.
5. Start conversion and monitor progress.

### Safer default for web mode

By default, the web backend:

- does not auto-read browser cookies
- runs in public-only auth mode

To opt in locally:

```powershell
$env:RUSTIFY_WEB_ALLOW_BROWSER_COOKIES="true"
cargo run -p web-backend
```

## Spotify Playlist Imports

Supported inputs:

- `https://open.spotify.com/playlist/...`
- `spotify:playlist:...`

How it works:

1. Rustify resolves playlist metadata locally in pages.
2. Each track is turned into a YouTube search-backed conversion job.
3. Output files use stable zero-padded playlist indexes so reruns stay aligned.

Important scope note:

- Rustify does not download audio directly from Spotify.
- Spotify support is playlist import plus track matching.
- Final media extraction still happens through YouTube and `yt-dlp`.

This keeps Spotify support user-friendly without asking users for Spotify secrets.

## Large Playlists and Rate Limits

For very large playlists, Rustify is designed to behave like a resumable local batch job.

What it does:

- resolves Spotify playlists in pages of up to `100` tracks
- retries `429` and `5xx` responses with backoff
- respects `Retry-After` when the upstream service sends it
- inserts configurable pacing between requests and conversion jobs
- writes deterministic indexed output filenames
- skips existing non-empty files on rerun

That means thousands of tracks are handled as a long queue, not as a single fragile request burst.

Recommended chunking for huge libraries:

```powershell
rustify playlist "SPOTIFY_OR_YOUTUBE_PLAYLIST" --format mp3 --quality 320 --start 1 --limit 250
rustify playlist "SPOTIFY_OR_YOUTUBE_PLAYLIST" --format mp3 --quality 320 --start 251 --limit 250
rustify playlist "SPOTIFY_OR_YOUTUBE_PLAYLIST" --format mp3 --quality 320 --start 501 --limit 250
```

Helpful pacing config:

```powershell
rustify config set rate_limits.request_delay_ms 1500
rustify config set rate_limits.max_retries 6
rustify config set rate_limits.backoff_base_ms 2000
```

To resume, rerun the same playlist command against the same output folder. Finished files are skipped automatically.

## YouTube Auth and Cookies

Recommended behavior:

### CLI and desktop

Use `Auto` mode:

1. Try public access first.
2. If YouTube demands auth or anti-bot verification, reuse the user’s own local browser session.
3. Never hardcode cookies.
4. Keep exported cookie files as an advanced fallback only.

### Web backend

Use the safer default:

1. Public-only mode by default.
2. Browser-cookie reuse only when explicitly enabled locally.
3. Never auto-read cookies on a hosted server.

### Why this is the right implementation

Hardcoded cookies are a bad idea because they:

- leak secrets
- risk the account itself
- get copied into logs, backups, screenshots, and Git history

Manual cookie-file export should stay a fallback, not the main UX.

Best practical UX:

- `Auto` mode for CLI and desktop
- browser-session reuse only when needed
- no cookie copying by default
- optional cookie-file override for advanced or headless setups

## Audio Quality Notes

### MP3

- recommended high-quality target: `320 kbps`

### FLAC

Rustify writes FLAC with `ffmpeg` compression level `0`.

That means:

- no additional lossy compression is added by Rustify
- the decoded source is preserved in FLAC form
- bitrate can exceed `900 kbps` depending on the source material

Important reality:

- YouTube source audio is usually already lossy
- FLAC preserves that source cleanly but cannot create new lossless fidelity from a lossy stream

### WAV

Use WAV when you want:

- fully uncompressed PCM output
- larger files
- maximum compatibility with editors and DAWs

## Configuration

Windows CLI config path:

```text
%APPDATA%\rustify\config.json
```

### Show current config

```powershell
rustify config show
```

### Reset config

```powershell
rustify config reset
```

### Set default download directory

```powershell
rustify config set download_dir "D:\Media\Rustify"
```

### Set auth mode

```powershell
rustify config set auth.mode auto
rustify config set auth.mode browser
rustify config set auth.mode cookie-file
rustify config set auth.mode none
```

### Set preferred browser for cookie reuse

```powershell
rustify config set auth.browser edge
rustify config set auth.browser chrome
rustify config set auth.browser firefox
```

### Set a manual cookie file

```powershell
rustify config set auth.cookie_file "D:\Secrets\youtube-cookies.txt"
```

### Tune rate-limit handling

```powershell
rustify config set rate_limits.request_delay_ms 1200
rustify config set rate_limits.max_retries 6
rustify config set rate_limits.backoff_base_ms 2000
```

### Tune Spotify playlist import behavior

```powershell
rustify config set spotify.enabled true
rustify config set spotify.market from_token
rustify config set spotify.fallback_to_page_scrape true
rustify config set spotify.search_suffix "official audio"
rustify config set spotify.page_size 100
```

### Override binary paths

```powershell
rustify config set binaries.yt_dlp "C:\Tools\yt-dlp.exe"
rustify config set binaries.ffmpeg "C:\Tools\ffmpeg.exe"
```

### Helpful environment variables

`YTDLP_PATH`

```powershell
$env:YTDLP_PATH="C:\Tools\yt-dlp.exe"
```

`FFMPEG_PATH`

```powershell
$env:FFMPEG_PATH="C:\Tools\ffmpeg.exe"
```

`DOWNLOADS_DIR`

```powershell
$env:DOWNLOADS_DIR="D:\Media\Rustify"
```

`RUSTIFY_WEB_ALLOW_BROWSER_COOKIES`

```powershell
$env:RUSTIFY_WEB_ALLOW_BROWSER_COOKIES="true"
```

## Development Commands

Check everything:

```powershell
cargo check --workspace
```

Offline check:

```powershell
cargo check --workspace --offline
```

Run tests:

```powershell
cargo test -p rustify-core --offline
```

Run CLI:

```powershell
cargo run -p rustify-cli -- doctor
```

Run desktop app:

```powershell
cargo run -p rustify-desktop
```

Run web backend:

```powershell
cargo run -p web-backend
```

## Troubleshooting

### `yt-dlp` is missing

```powershell
python -m pip install yt-dlp
rustify doctor
```

### `ffmpeg` is missing

Install `ffmpeg`, then verify:

```powershell
ffmpeg -version
rustify doctor
```

### Browser-session reuse is not working

1. Make sure the browser is logged into YouTube.
2. Close and reopen the browser.
3. Set a preferred browser explicitly:

```powershell
rustify config set auth.browser edge
```

4. Run:

```powershell
rustify doctor
```

### Website downloads work publicly but fail for restricted videos

That is expected with the safer web default. Use one of these:

- run the desktop app
- use the CLI
- opt in locally for the backend

```powershell
$env:RUSTIFY_WEB_ALLOW_BROWSER_COOKIES="true"
cargo run -p web-backend
```

### Very large playlist runs are slow

That is normal for a local-first converter, especially when a Spotify playlist must be matched track by track to YouTube sources.

Use one or more of these:

1. Split the job with `--start` and `--limit`.
2. Increase `rate_limits.request_delay_ms` if you are seeing `429` responses.
3. Rerun into the same output folder to resume from completed files.

## Security Notes

- Do not hardcode YouTube cookies.
- Do not commit cookie files.
- Do not store personal browser-session exports in the repository.
- `.gitignore` should keep cookie exports, local secrets, downloads, caches, and temp files out of Git.
- Keep browser-cookie reuse disabled for hosted web deployments.
