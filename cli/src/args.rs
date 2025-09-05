use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ezp3")]
#[command(about = "Ultra-fast YouTube video converter")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output directory (default: current directory)
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Convert a single video
    Convert(ConvertArgs),
    
    /// Convert a playlist
    Playlist(PlaylistArgs),
    
    /// Batch convert with multiple formats
    Batch(BatchArgs),
    
    /// Download manager for converted files
    Download(DownloadArgs),
    
    /// Get video information
    Info(InfoArgs),
    
    /// List available qualities
    Quality(QualityArgs),
    
    /// Download configuration
    Config(ConfigArgs),
}

#[derive(Args)]
pub struct ConvertArgs {
    /// YouTube video URL
    pub url: String,

    /// Output format (mp3, mp4, flac, aac, ogg, webm)
    #[arg(short, long, default_value = "mp3")]
    pub format: String,

    /// Quality (audio: 128, 192, 256, 320; video: 720p, 1080p, 1440p, 4k)
    #[arg(short, long, default_value = "256")]
    pub quality: String,

    /// Output filename (without extension)
    #[arg(short = 'n', long)]
    pub name: Option<String>,

    /// Preserve original quality (no re-encoding when possible)
    #[arg(long)]
    pub preserve: bool,

    /// Use hardware acceleration
    #[arg(long)]
    pub hardware: bool,

    /// Number of threads for conversion
    #[arg(short, long)]
    pub threads: Option<usize>,
}

#[derive(Args)]
pub struct PlaylistArgs {
    /// YouTube playlist URL
    pub url: String,

    /// Output format (mp3, mp4, flac, aac, ogg, webm)
    #[arg(short, long, default_value = "mp3")]
    pub format: String,

    /// Quality (audio: 128, 192, 256, 320; video: 720p, 1080p, 1440p, 4k)
    #[arg(short, long, default_value = "256")]
    pub quality: String,

    /// Download only first N videos
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Start from video number N
    #[arg(long, default_value = "1")]
    pub start: usize,

    /// Parallel downloads
    #[arg(short, long, default_value = "3")]
    pub parallel: usize,
}

#[derive(Args)]
pub struct BatchArgs {
    /// YouTube playlist URL
    pub url: String,

    /// Output formats (comma-separated: mp3,mp4,flac)
    #[arg(short, long, default_value = "mp3")]
    pub formats: String,

    /// Quality for each format (format:quality,format:quality)
    /// Example: mp3:320,mp4:1080p,flac:best
    #[arg(short, long)]
    pub qualities: Option<String>,

    /// Maximum concurrent downloads
    #[arg(short = 'j', long, default_value = "3")]
    pub jobs: usize,

    /// Skip existing files
    #[arg(long)]
    pub skip_existing: bool,

    /// Create subdirectories for each format
    #[arg(long, default_value = "true")]
    pub create_subdirs: bool,

    /// Add index prefix to filenames (001-filename.ext)
    #[arg(long, default_value = "true")]
    pub add_index: bool,

    /// Download only first N videos
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Start from video number N
    #[arg(long, default_value = "1")]
    pub start: usize,

    /// Download thumbnails
    #[arg(long)]
    pub thumbnails: bool,

    /// Auto-download after conversion
    #[arg(long)]
    pub auto_download: bool,
}

#[derive(Args)]
pub struct DownloadArgs {
    #[command(subcommand)]
    pub action: DownloadAction,
}

#[derive(Subcommand)]
pub enum DownloadAction {
    /// List all completed conversions
    List,
    
    /// Download specific files by ID
    Get {
        /// Job/Task IDs to download
        ids: Vec<String>,
        /// Download directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Download all files from a batch job
    Batch {
        /// Batch job ID
        job_id: String,
        /// Download directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Clean up old conversion files
    Clean {
        /// Delete files older than N days
        #[arg(long, default_value = "7")]
        days: u64,
    },
}

#[derive(Args)]
pub struct InfoArgs {
    /// YouTube video URL
    pub url: String,

    /// Output format (json, yaml, table)
    #[arg(long, default_value = "table")]
    pub format: String,
}

#[derive(Args)]
pub struct QualityArgs {
    /// YouTube video URL
    pub url: String,

    /// Show only audio qualities
    #[arg(long)]
    pub audio_only: bool,

    /// Show only video qualities
    #[arg(long)]
    pub video_only: bool,
}

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    
    /// Reset configuration to defaults
    Reset,
}
