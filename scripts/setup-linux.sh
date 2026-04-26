#!/usr/bin/env bash
# setup-linux.sh - Auto Setup for Rustify on Linux/macOS

DL="$HOME/Downloads/Rustify"
mkdir -p "$DL"

YT="$(command -v yt-dlp || true)"
FF="$(command -v ffmpeg || true)"

if command -v google-chrome >/dev/null 2>&1 || command -v chromium >/dev/null 2>&1; then 
    BROWSER="chrome"
elif command -v microsoft-edge >/dev/null 2>&1; then 
    BROWSER="edge"
elif command -v firefox >/dev/null 2>&1; then 
    BROWSER="firefox"
else 
    BROWSER="firefox"
fi

rustify config reset
rustify config set download_dir "$DL"

# If --headless flag is passed, disable browser session reuse
if [ "$1" = "--headless" ]; then
    echo "Running in headless mode. Disabling browser session reuse."
    rustify config set auth.mode none
else
    rustify config set auth.mode auto
    rustify config set auth.browser "$BROWSER"
fi

[ -n "$YT" ] && rustify config set binaries.yt_dlp "$YT"
[ -n "$FF" ] && rustify config set binaries.ffmpeg "$FF"

rustify config show
