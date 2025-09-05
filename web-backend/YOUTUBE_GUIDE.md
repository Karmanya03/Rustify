# 🎵 Rustify YouTube Converter - Complete Guide

A modern, high-performance YouTube video and playlist converter built with Rust and Axum.

## ✨ Features

### 🎬 Single Video Conversion
- Download individual YouTube videos
- Multiple format support (MP3, WAV, MP4, WebM)
- Quality selection (320kbps MP3, 1080p video, etc.)
- Real-time progress tracking

### 🎵 Playlist Conversion
- Convert entire YouTube playlists
- Batch download all videos in a playlist
- Individual progress tracking for each video
- Automatic file organization

### 🚀 Modern Web Interface
- Responsive design with glassmorphism effects
- Real-time WebSocket updates
- Task management and monitoring
- Download progress visualization

## 📋 Prerequisites

Before using Rustify, you need to install the required dependencies:

### 🔧 Automatic Setup (Recommended)

**Windows:**
```powershell
.\setup.ps1
```

**Linux/macOS:**
```bash
chmod +x setup.sh
./setup.sh
```

### 🛠️ Manual Setup

1. **Install Python** (if not already installed)
   - Windows: Download from [python.org](https://python.org)
   - macOS: `brew install python3`
   - Ubuntu/Debian: `sudo apt install python3 python3-pip`

2. **Install yt-dlp** (required)
   ```bash
   pip install --upgrade yt-dlp
   ```

3. **Install ffmpeg** (recommended for audio conversion)
   - Windows: Download from [ffmpeg.org](https://ffmpeg.org) or `choco install ffmpeg`
   - macOS: `brew install ffmpeg`
   - Ubuntu/Debian: `sudo apt install ffmpeg`

## 🚀 Usage

### Starting the Server

```bash
# Development mode
cargo run

# Production mode
cargo build --release
./target/release/web-backend
```

The server will start on `http://localhost:3001` by default.

### 🌐 Web Interface

1. Open your browser to `http://localhost:3001`
2. Switch between tabs:
   - **Single Video**: Convert individual YouTube videos
   - **Playlist**: Convert entire YouTube playlists
   - **Tasks**: Monitor download progress and manage files

### 📱 Single Video Download

1. Paste a YouTube video URL
2. Select output format (MP3, WAV, MP4, WebM)
3. Choose quality settings
4. Click "Convert & Download"
5. Monitor progress in real-time
6. Download completed file

### 📂 Playlist Download

1. Paste a YouTube playlist URL
2. Select output format and quality
3. Click "Convert Playlist"
4. All videos will be queued for download
5. Monitor individual progress for each video
6. Download completed files individually

## 🎯 Supported Formats

### 🎵 Audio Formats
- **MP3**: 96kbps, 128kbps, 192kbps, 256kbps, 320kbps
- **WAV**: Lossless, HD Audio

### 🎬 Video Formats
- **MP4**: 360p, 480p, 720p, 720p60, 1080p, 1080p60
- **WebM**: 360p, 480p, 720p, 720p60, 1080p, 1080p60

## ⭐ Quality Recommendations

- **320kbps MP3**: Apple Music equivalent quality, perfect balance
- **WAV Lossless**: CD quality for audiophiles
- **1080p MP4**: Full HD video quality
- **720p60 MP4**: Smooth motion for gaming videos

## 📁 File Organization

Downloads are organized as follows:
```
downloads/
├── {task_id}/                    # Single video downloads
│   └── Video Title.mp3
└── playlist_{n}_{task_id}/       # Playlist downloads
    └── Video Title.mp3
```

## 🔧 Configuration

### Environment Variables

```bash
# Custom port
PORT=8080

# Custom host (for deployment)
HOST=0.0.0.0

# Downloads directory
DOWNLOADS_DIR=./downloads
```

## 🛠️ Troubleshooting

### Common Issues

1. **"yt-dlp not found"**
   - Run the setup script: `.\setup.ps1` (Windows) or `./setup.sh` (Linux/macOS)
   - Manual install: `pip install yt-dlp`

2. **"ffmpeg not found"**
   - Install ffmpeg for audio conversion support
   - Video downloads will still work without ffmpeg

3. **Download fails**
   - Check internet connection
   - Verify YouTube URL is valid and accessible
   - Some videos may be geo-blocked or age-restricted

4. **Slow downloads**
   - YouTube may rate-limit downloads
   - Try again later or use different quality settings

### 📊 Debug Mode

Enable debug logging for detailed information:
```bash
RUST_LOG=debug cargo run
```

## 🏗️ Development

### Project Structure

```
src/
├── main.rs          # Application entry point
├── handlers.rs      # API endpoint handlers
├── websocket.rs     # WebSocket handling
├── state.rs         # Application state
├── youtube.rs       # YouTube downloading logic
└── security/        # Security middleware
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Check for issues
cargo clippy
```

## 🚀 Deployment

### Production Considerations

1. **Reverse Proxy**: Use nginx or similar for production
2. **SSL/TLS**: Enable HTTPS for secure downloads
3. **Rate Limiting**: Implement rate limiting for API endpoints
4. **Monitoring**: Use structured logging for monitoring
5. **Storage**: Consider cloud storage for downloads

## 🔒 Security & Legal

### Security Features
- CORS protection
- Security headers (CSP, HSTS, etc.)
- Input validation and sanitization
- Secure error handling

### Legal Disclaimer
This tool is for educational and personal use only. Please respect YouTube's Terms of Service and copyright laws. Users are responsible for ensuring they have the right to download and use any content.

## 📞 Support

If you encounter issues:

1. Check the troubleshooting section above
2. Run the setup script to ensure dependencies are installed
3. Enable debug logging to see detailed error messages
4. Check that your YouTube URL is valid and accessible

## 🎉 What's New

### Latest Features
- ✅ Complete playlist conversion support
- ✅ Real-time progress tracking
- ✅ Modern glassmorphism UI
- ✅ WebSocket real-time updates
- ✅ Multiple quality options
- ✅ Automatic dependency checking
- ✅ Cross-platform setup scripts

Enjoy converting your favorite YouTube content with Rustify! 🎵🚀
