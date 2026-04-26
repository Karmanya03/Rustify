use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rustify")]
#[command(about = "Local-first YouTube downloader and converter")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override the output directory from config.
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Convert a single video.
    Convert(ConvertArgs),
    /// Convert a playlist.
    Playlist(PlaylistArgs),
    /// Batch convert a playlist into multiple formats.
    Batch(BatchArgs),
    /// Get video information.
    Info(InfoArgs),
    /// List available source qualities.
    Quality(QualityArgs),
    /// Show or update Rustify CLI configuration.
    Config(ConfigArgs),
    /// Inspect local dependencies and auth strategy.
    Doctor,
}

#[derive(Args, Clone)]
pub struct ConvertArgs {
    /// YouTube video URL.
    pub url: String,

    /// Output format (mp3, flac, wav, aac, ogg, mp4, webm).
    #[arg(short, long, default_value = "mp3")]
    pub format: String,

    /// Quality selector (audio bitrates, lossless, or video resolution).
    #[arg(short, long, default_value = "320")]
    pub quality: String,

    /// Output filename without extension.
    #[arg(short = 'n', long)]
    pub name: Option<String>,
}

#[derive(Args, Clone)]
pub struct PlaylistArgs {
    /// YouTube or Spotify playlist URL.
    pub url: String,

    /// Output format (mp3, flac, wav, aac, ogg, mp4, webm).
    #[arg(short, long, default_value = "mp3")]
    pub format: String,

    /// Quality selector (audio bitrates, lossless, or video resolution).
    #[arg(short, long, default_value = "320")]
    pub quality: String,

    /// Download only the first N videos after applying --start.
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Start from playlist item N.
    #[arg(long, default_value = "1")]
    pub start: usize,
}

#[derive(Args, Clone)]
pub struct BatchArgs {
    /// YouTube or Spotify playlist URL.
    pub url: String,

    /// Output formats (comma-separated: mp3,flac,mp4).
    #[arg(short, long, default_value = "mp3")]
    pub formats: String,

    /// Optional quality map like mp3:320,flac:lossless,mp4:1080p.
    #[arg(short, long)]
    pub qualities: Option<String>,

    /// Download only the first N videos after applying --start.
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Start from playlist item N.
    #[arg(long, default_value = "1")]
    pub start: usize,
}

#[derive(Args)]
pub struct InfoArgs {
    /// YouTube video URL, or a YouTube / Spotify playlist URL.
    pub url: String,

    /// Output format (json or table).
    #[arg(long, default_value = "table")]
    pub format: String,
}

#[derive(Args)]
pub struct QualityArgs {
    /// YouTube video URL.
    pub url: String,

    /// Show only audio qualities.
    #[arg(long)]
    pub audio_only: bool,

    /// Show only video qualities.
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
    /// Show the current config file.
    Show,
    /// Set a config value.
    Set {
        /// Key like download_dir, auth.mode, auth.browser, auth.cookie_file.
        key: String,
        /// Value for the key.
        value: String,
    },
    /// Reset config to defaults.
    Reset,
}
