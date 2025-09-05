#!/bin/bash

# Rustify YouTube Downloader Setup Script
# This script installs the required dependencies for YouTube downloading

echo "🚀 Setting up Rustify YouTube Downloader dependencies..."

# Check if Python is installed
if ! command -v python3 &> /dev/null && ! command -v python &> /dev/null; then
    echo "❌ Python is not installed. Please install Python first:"
    echo "   - Windows: Download from https://python.org"
    echo "   - macOS: brew install python3"
    echo "   - Ubuntu/Debian: sudo apt install python3 python3-pip"
    echo "   - CentOS/RHEL: sudo yum install python3 python3-pip"
    exit 1
fi

# Check if pip is available
if ! command -v pip3 &> /dev/null && ! command -v pip &> /dev/null; then
    echo "❌ pip is not installed. Please install pip first."
    exit 1
fi

# Install yt-dlp
echo "📦 Installing yt-dlp..."
if command -v pip3 &> /dev/null; then
    pip3 install --upgrade yt-dlp
else
    pip install --upgrade yt-dlp
fi

if [ $? -eq 0 ]; then
    echo "✅ yt-dlp installed successfully!"
else
    echo "❌ Failed to install yt-dlp. Please try:"
    echo "   pip3 install --upgrade yt-dlp"
    echo "   or"
    echo "   pip install --upgrade yt-dlp"
    exit 1
fi

# Check if ffmpeg is installed
if ! command -v ffmpeg &> /dev/null; then
    echo "⚠️  ffmpeg is not installed. Installing ffmpeg is recommended for audio conversion."
    echo "   - Windows: Download from https://ffmpeg.org or use chocolatey: choco install ffmpeg"
    echo "   - macOS: brew install ffmpeg"
    echo "   - Ubuntu/Debian: sudo apt install ffmpeg"
    echo "   - CentOS/RHEL: sudo yum install ffmpeg"
    echo ""
    echo "   Without ffmpeg, you may have limited audio format options."
else
    echo "✅ ffmpeg is already installed!"
fi

# Test yt-dlp installation
echo "🧪 Testing yt-dlp installation..."
if yt-dlp --version &> /dev/null; then
    VERSION=$(yt-dlp --version)
    echo "✅ yt-dlp is working! Version: $VERSION"
else
    echo "❌ yt-dlp test failed. Please check your installation."
    exit 1
fi

echo ""
echo "🎉 Setup complete! You can now use Rustify to download YouTube videos and playlists."
echo ""
echo "📝 Usage:"
echo "   1. Start the Rustify web server: cargo run"
echo "   2. Open your browser to http://localhost:3001"
echo "   3. Paste a YouTube URL and start downloading!"
echo ""
echo "📋 Supported formats:"
echo "   - MP3 (audio only) - various bitrates"
echo "   - WAV (audio only) - lossless quality"
echo "   - MP4 (video) - 360p to 1080p"
echo "   - WebM (video) - 360p to 1080p"
echo ""
echo "🔗 Supports both individual videos and entire playlists!"
