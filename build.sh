#!/bin/bash

# EzP3 Build Script for Unix/Linux/macOS

set -e

echo "🚀 Building EzP3 YouTube Converter..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if FFmpeg is installed
if ! command -v ffmpeg &> /dev/null; then
    echo "⚠️  FFmpeg is not found in PATH"
    echo "   Please install FFmpeg:"
    echo "   - Ubuntu/Debian: sudo apt install ffmpeg"
    echo "   - macOS: brew install ffmpeg"
    echo "   - Or download from: https://ffmpeg.org/download.html"
    echo "   Continuing build anyway..."
fi

# Create build directory
mkdir -p dist

echo "📦 Building core library..."
cd core && cargo build --release && cd ..

echo "🖥️  Building CLI application..."
cd cli && cargo build --release && cd ..
cp target/release/ezp3 dist/ 2>/dev/null || echo "CLI binary not found in expected location"

echo "🌐 Building web backend..."
cd web-backend && cargo build --release && cd ..
cp target/release/ezp3-web-backend dist/ 2>/dev/null || echo "Web backend binary not found"

# Build desktop app if Tauri is available
if command -v cargo-tauri &> /dev/null; then
    echo "🖱️  Building desktop application..."
    cd desktop && cargo tauri build && cd ..
    echo "✅ Desktop app built successfully"
else
    echo "⚠️  Tauri CLI not found. Skipping desktop build."
    echo "   Install with: cargo install tauri-cli"
fi

echo "🧪 Running tests..."
cargo test --workspace

echo "✅ Build completed successfully!"
echo ""
echo "📁 Built artifacts:"
echo "   CLI: ./dist/ezp3"
echo "   Web Backend: ./dist/ezp3-web-backend"
echo "   Desktop: ./desktop/src-tauri/target/release/"
echo ""
echo "🎉 EzP3 is ready to use!"
echo ""
echo "Quick start:"
echo "   CLI: ./dist/ezp3 convert 'https://youtube.com/watch?v=...' --format mp3"
echo "   Web: ./dist/ezp3-web-backend"
echo "   Desktop: run the executable in desktop/src-tauri/target/release/"
