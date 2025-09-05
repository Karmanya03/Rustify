# Development Setup Guide

This guide will help you set up the development environment for EzP3.

## Prerequisites

### Required Software

1. **Rust** (1.70 or later)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **FFmpeg** (latest version)
   - **Windows**: Download from [ffmpeg.org](https://ffmpeg.org/download.html) or use `choco install ffmpeg`
   - **macOS**: `brew install ffmpeg`
   - **Ubuntu/Debian**: `sudo apt install ffmpeg`

3. **Node.js** (18+) - for web frontend and Tauri
   ```bash
   # Install from nodejs.org or use a version manager
   ```

4. **Tauri CLI** (for desktop app)
   ```bash
   cargo install tauri-cli
   ```

### Optional Dependencies

- **youtube-dl** or **yt-dlp** for video extraction
- **SQLite** for database features (optional)

## Project Structure

```
ezp3/
├── core/           # Core conversion library
├── cli/            # Command-line interface
├── desktop/        # Tauri desktop application
├── web-backend/    # Web API server
├── web-frontend/   # React web interface (to be created)
├── docs/           # Documentation
└── scripts/        # Build and deployment scripts
```

## Development Workflow

### 1. Clone and Setup

```bash
git clone <repository-url>
cd ezp3
```

### 2. Build All Components

**Windows:**
```powershell
.\build.ps1
```

**Unix/Linux/macOS:**
```bash
chmod +x build.sh
./build.sh
```

### 3. Development Commands

#### Core Library
```bash
cd core
cargo test
cargo build
```

#### CLI Development
```bash
cd cli
cargo run -- convert "https://youtube.com/watch?v=dQw4w9WgXcQ" --format mp3
```

#### Desktop App Development
```bash
cd desktop
cargo tauri dev
```

#### Web Backend Development
```bash
cd web-backend
cargo run
# Server starts on http://localhost:3001
```

### 4. Testing

Run all tests:
```bash
cargo test --workspace
```

Run specific component tests:
```bash
cd core && cargo test
cd cli && cargo test
```

### 5. Building for Release

```bash
cargo build --release --workspace
```

## IDE Setup

### VS Code (Recommended)

Install these extensions:
- `rust-analyzer` - Rust language support
- `Even Better TOML` - TOML file support
- `Tauri` - Tauri development support
- `Error Lens` - Inline error display

### Settings

Create `.vscode/settings.json`:
```json
{
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.checkOnSave.command": "clippy"
}
```

## Environment Variables

Create a `.env` file in the project root:
```bash
# Optional: Custom FFmpeg path
FFMPEG_PATH=/usr/local/bin/ffmpeg

# Optional: Default output directory
EZAP3_OUTPUT_DIR=./downloads

# Optional: Enable debug logging
RUST_LOG=debug
```

## Debugging

### CLI Debugging
```bash
RUST_LOG=debug cargo run --bin ezp3 -- convert "..." --format mp3
```

### Web Backend Debugging
```bash
RUST_LOG=debug cargo run --bin ezp3-web-backend
```

### Desktop App Debugging
```bash
cd desktop
cargo tauri dev
# Opens with DevTools enabled
```

## Performance Optimization

### Build Optimizations

Add to `Cargo.toml` for release builds:
```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
```

### FFmpeg Optimization

The application automatically detects and uses:
- Hardware acceleration when available
- Optimal thread count based on CPU cores
- Efficient streaming for large files

## Common Issues

### FFmpeg Not Found
- Ensure FFmpeg is in your PATH
- On Windows, you may need to restart your terminal after installation
- Check with: `ffmpeg -version`

### Tauri Build Fails
- Ensure you have all Tauri prerequisites: https://tauri.app/v1/guides/getting-started/prerequisites
- Install missing dependencies for your platform

### YouTube Extraction Fails
- This may be due to YouTube changes
- Update youtube-dl/yt-dlp to the latest version
- Check if the URL format is supported

### Performance Issues
- Enable hardware acceleration in your system
- Increase available RAM
- Use SSD storage for temporary files

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass: `cargo test --workspace`
6. Run clippy: `cargo clippy --workspace`
7. Format code: `cargo fmt --all`
8. Commit changes: `git commit -m 'Add amazing feature'`
9. Push to branch: `git push origin feature/amazing-feature`
10. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
