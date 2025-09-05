# High-Quality Audio Conversion Guide

## Apple Music Quality Standards

### Audio Quality Specifications:
- **Apple Music**: 256kbps AAC (equivalent to ~320kbps MP3)
- **Lossless**: FLAC 16-bit/44.1kHz (CD Quality)
- **Hi-Res**: FLAC 24-bit/96kHz or higher

## Implementation with yt-dlp + ffmpeg

### 1. Install Required Tools:
```bash
# Windows (using winget)
winget install yt-dlp
winget install ffmpeg

# Or download from:
# https://github.com/yt-dlp/yt-dlp/releases
# https://ffmpeg.org/download.html
```

### 2. yt-dlp Quality Settings:

#### For 320kbps MP3 (Apple Music equivalent):
```rust
let ytdlp_args = vec![
    "--extract-audio",
    "--audio-format", "mp3",
    "--audio-quality", "320K",
    "--embed-metadata",
    "--add-metadata",
    "--format", "bestaudio[ext=m4a]/bestaudio",
    url,
    "-o", output_path
];
```

#### For Lossless FLAC:
```rust
let ytdlp_args = vec![
    "--extract-audio", 
    "--audio-format", "flac",
    "--audio-quality", "0",  // Best quality
    "--embed-metadata",
    "--add-metadata", 
    "--format", "bestaudio",
    url,
    "-o", output_path
];
```

### 3. ffmpeg Quality Settings:

#### 320kbps MP3 (Apple Music Quality):
```rust
let ffmpeg_args = vec![
    "-i", input_file,
    "-c:a", "libmp3lame",    // LAME encoder
    "-b:a", "320k",          // 320kbps bitrate
    "-q:a", "0",             // Highest quality
    "-joint_stereo", "0",    // No joint stereo for max quality
    "-f", "mp3",
    output_file
];
```

#### FLAC Lossless:
```rust
let ffmpeg_args = vec![
    "-i", input_file,
    "-c:a", "flac",
    "-compression_level", "8",  // Maximum compression (still lossless)
    "-f", "flac",
    output_file  
];
```

### 4. Quality Comparison:

| Format | Bitrate | Quality Level | Use Case |
|--------|---------|---------------|----------|
| MP3 320kbps | 320k | Apple Music equivalent | Best compatibility |
| AAC 256kbps | 256k | Apple Music native | iOS/Apple devices |
| FLAC Lossless | ~1411k | Perfect quality | Audiophiles |
| Hi-Res FLAC | 2304k+ | Studio quality | Professional use |

### 5. Implementation in Rust:

```rust
async fn convert_with_ytdlp(url: &str, format: OutputFormat, output_path: &Path) -> Result<()> {
    match format {
        OutputFormat::Mp3 { bitrate: 320 } => {
            // Use yt-dlp to get best audio + ffmpeg for 320k MP3
            let mut cmd = Command::new("yt-dlp");
            cmd.args(&[
                "--extract-audio",
                "--audio-format", "mp3", 
                "--audio-quality", "320K",
                "--embed-metadata",
                "--format", "bestaudio[ext=m4a]/bestaudio",
                url,
                "-o", output_path.to_str().unwrap()
            ]);
            
            let output = cmd.output().await?;
            if !output.status.success() {
                return Err(anyhow::anyhow!("yt-dlp failed: {}", 
                    String::from_utf8_lossy(&output.stderr)));
            }
        },
        
        OutputFormat::Flac => {
            // Lossless FLAC conversion
            let mut cmd = Command::new("yt-dlp");
            cmd.args(&[
                "--extract-audio",
                "--audio-format", "flac",
                "--audio-quality", "0",
                "--embed-metadata", 
                "--format", "bestaudio",
                url,
                "-o", output_path.to_str().unwrap()
            ]);
            
            let output = cmd.output().await?;
            if !output.status.success() {
                return Err(anyhow::anyhow!("yt-dlp failed: {}", 
                    String::from_utf8_lossy(&output.stderr)));
            }
        },
        
        _ => {
            // Handle other formats...
        }
    }
    
    Ok(())
}
```

### 6. Quality Validation:

To ensure Apple Music quality:
1. **Source**: Always use best available audio track
2. **Encoding**: Use LAME encoder for MP3 with V0 or 320k CBR
3. **Metadata**: Preserve all metadata and artwork
4. **No Re-encoding**: Avoid multiple compression stages

### 7. File Size Estimates:

- **320kbps MP3**: ~2.4MB per minute
- **256kbps AAC**: ~1.9MB per minute  
- **FLAC Lossless**: ~8.5MB per minute
- **Hi-Res FLAC**: ~17MB per minute

This ensures your 320kbps output matches Apple Music's quality standards with minimal compression artifacts.
