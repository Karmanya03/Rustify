pub mod batch;
pub mod config;
pub mod converter;
pub mod extractor;
pub mod quality;
pub mod runtime;
pub mod spotify;
pub mod utils;

pub use batch::*;
pub use config::*;
pub use converter::*;
pub use extractor::*;
pub use quality::*;
pub use runtime::*;
pub use spotify::*;
pub use utils::*;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Main Rustify conversion pipeline shared by the CLI, desktop app, and web backend.
pub struct EzP3 {
    config: AppConfig,
    extractor: YouTubeExtractor,
    converter: Converter,
}

impl EzP3 {
    /// Create a new Rustify instance with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(AppConfig::default())
    }

    /// Create a new Rustify instance with the provided configuration.
    pub fn with_config(config: AppConfig) -> Result<Self> {
        Ok(Self {
            extractor: YouTubeExtractor::new(config.clone()),
            converter: Converter::new(config.clone())?,
            config,
        })
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub async fn dependency_status(&self) -> DependencyStatus {
        runtime::dependency_status(&self.config).await
    }

    /// Create a batch processor for this Rustify instance.
    pub fn create_batch_processor(self: Arc<Self>) -> BatchProcessor {
        BatchProcessor::new(self)
    }

    /// Convert a single YouTube video into the requested output format.
    pub async fn convert_video<F>(
        &self,
        url: &str,
        output_path: PathBuf,
        format: OutputFormat,
        quality: &str,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(ConversionProgress) + Send + Sync + 'static,
    {
        info!("Starting conversion: {} -> {:?}", url, output_path);

        let video_info = self.extractor.extract_info(url).await?;
        info!("Extracted video info: {}", video_info.title);

        let settings = ConversionSettings {
            input_url: url.to_string(),
            output_path,
            format,
            quality: quality.to_string(),
            metadata: Some(MediaMetadata {
                title: video_info.title.clone(),
                uploader: video_info.uploader.clone(),
                duration_seconds: Some(video_info.duration),
            }),
            preserve_quality: true,
            use_hardware_acceleration: true,
            thread_count: Some(num_cpus::get()),
        };

        self.converter
            .convert_with_progress(settings, progress_callback)
            .await?;

        info!("Conversion completed successfully");
        Ok(())
    }

    /// Convert every video from a playlist sequentially.
    pub async fn convert_playlist<F>(
        &self,
        playlist_url: &str,
        output_dir: PathBuf,
        format: OutputFormat,
        quality: &str,
        progress_callback: F,
    ) -> Result<Vec<Result<()>>>
    where
        F: Fn(usize, ConversionProgress) + Send + Sync + 'static + Clone,
    {
        info!("Converting playlist: {}", playlist_url);

        let playlist_info = self.extractor.extract_playlist_info(playlist_url).await?;
        let conversion_settings =
            build_playlist_conversion_settings(&playlist_info, &output_dir, &format, quality);

        self.converter
            .batch_convert(conversion_settings, progress_callback)
            .await
    }

    /// Get video information without downloading the file.
    pub async fn get_video_info(&self, url: &str) -> Result<VideoInfo> {
        self.extractor.extract_info(url).await
    }

    /// Get playlist information without downloading the files.
    pub async fn get_playlist_info(&self, url: &str) -> Result<PlaylistInfo> {
        self.extractor.extract_playlist_info(url).await
    }

    /// List the qualities and source variants available for a video.
    pub async fn get_available_qualities(&self, url: &str) -> Result<QualityOptions> {
        let info = self.extractor.extract_info(url).await?;
        Ok(analyze_available_qualities(&info))
    }
}

fn get_extension_for_format(format: &OutputFormat) -> &'static str {
    match format {
        OutputFormat::Mp3 { .. } => "mp3",
        OutputFormat::Mp4 { .. } => "mp4",
        OutputFormat::Flac => "flac",
        OutputFormat::Wav => "wav",
        OutputFormat::Aac { .. } => "aac",
        OutputFormat::Ogg { .. } => "ogg",
        OutputFormat::WebM { .. } => "webm",
    }
}

fn build_playlist_conversion_settings(
    playlist_info: &PlaylistInfo,
    output_dir: &std::path::Path,
    format: &OutputFormat,
    quality: &str,
) -> Vec<ConversionSettings> {
    let extension = get_extension_for_format(format);
    let index_width =
        playlist_index_width(playlist_info.video_count.max(playlist_info.videos.len()));

    playlist_info
        .videos
        .iter()
        .enumerate()
        .map(|(index, video)| {
            let safe_title = sanitize_filename(&video.title);
            let output_path = output_dir.join(format!(
                "{:0width$}-{}.{}",
                index + 1,
                safe_title,
                extension,
                width = index_width
            ));

            ConversionSettings {
                input_url: video.url.clone(),
                output_path,
                format: format.clone(),
                quality: quality.to_string(),
                metadata: Some(MediaMetadata {
                    title: video.title.clone(),
                    uploader: video.uploader.clone(),
                    duration_seconds: video.duration,
                }),
                preserve_quality: true,
                use_hardware_acceleration: true,
                thread_count: Some(num_cpus::get()),
            }
        })
        .collect()
}

fn playlist_index_width(total_items: usize) -> usize {
    total_items.max(1).to_string().len().max(3)
}
