# EzP3 - High-Performance YouTube Video Converter

A blazingly fast, high-quality YouTube video converter built with Rust that supports multiple formats and quality options with zero compression loss.

## Features

- 🚀 **Ultra-fast conversion** using Rust and optimized FFmpeg bindings
- 🎵 **MP3 conversion** with quality options (128kbps, 192kbps, 256kbps, 320kbps)
- 🎬 **MP4 conversion** with multiple resolutions (720p, 1080p, 1440p, 4K)
- 🔥 **No compression loss** - preserves original quality
- ⚡ **Parallel processing** for batch conversions
- 🖥️ **Multiple interfaces**: CLI, Desktop GUI, Web interface
- 📊 **Real-time progress tracking**
- 🎯 **Smart quality detection** and optimization

## Supported Formats

### Audio Formats
- MP3 (128kbps, 192kbps, 256kbps, 320kbps)
- FLAC (lossless)
- AAC (high quality)
- OGG Vorbis

### Video Formats  
- MP4 (H.264/H.265)
- WebM (VP9)
- AVI
- MOV

### Quality Options
- **Audio**: 128kbps to 320kbps MP3, FLAC lossless
- **Video**: 720p, 1080p, 1440p, 4K (with original quality preservation)

## Architecture

```
ezp3/
├── core/           # Core conversion logic and YouTube API handling
├── cli/            # Command-line interface
├── desktop/        # Tauri-based desktop application  
├── web-backend/    # Web API server
├── web-frontend/   # React/Next.js web interface
├── ffmpeg-sys/     # FFmpeg Rust bindings
└── docs/           # Documentation
```

## Quick Start

### Prerequisites
- Rust 1.70+
- FFmpeg (automatically downloaded)
- Node.js 18+ (for web frontend)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/ezp3.git
cd ezp3

# Build all components
cargo build --release

# Install CLI tool
cargo install --path cli

# Run desktop app
cd desktop && cargo tauri dev

# Start web server
cd web-backend && cargo run
```

### Usage

#### CLI
```bash
# Convert to MP3 320kbps
ezp3 convert "https://youtube.com/watch?v=..." --format mp3 --quality 320

# Convert to MP4 1080p
ezp3 convert "https://youtube.com/watch?v=..." --format mp4 --quality 1080p

# Batch convert playlist
ezp3 playlist "https://youtube.com/playlist?list=..." --format mp3 --quality 256
```

#### Desktop App
- Drag & drop YouTube URLs
- Select output format and quality
- Monitor conversion progress
- Batch processing support

#### Web Interface
- Clean, modern UI
- Real-time progress tracking
- Download management
- Mobile responsive

## Performance

- **Speed**: Up to 10x faster than traditional converters
- **Quality**: Bit-perfect audio extraction
- **Memory**: Low memory footprint with streaming processing
- **CPU**: Multi-threaded processing utilizing all cores

## Development

### Building from Source

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Disclaimer

This tool is for educational and personal use only. Please respect YouTube's Terms of Service and copyright laws.
