pub mod extractor;
pub mod converter;
pub mod quality;
pub mod utils;
pub mod batch;

pub use extractor::*;
pub use converter::*;
pub use quality::*;
pub use utils::*;
pub use batch::*;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Main EzP3 conversion pipeline
pub struct EzP3 {
    extractor: YouTubeExtractor,
    converter: Converter,
}

impl EzP3 {
    /// Create a new EzP3 instance
    pub fn new() -> Result<Self> {
        Ok(Self {
            extractor: YouTubeExtractor::new(),
            converter: Converter::new()?,
        })
    }
    
    /// Create a batch processor for this EzP3 instance
    pub fn create_batch_processor(self: Arc<Self>) -> BatchProcessor {
        BatchProcessor::new(self)
    }

    /// Convert YouTube video to specified format
    pub async fn convert_video<F>(
        &self,
        url: &str,
        output_path: PathBuf,
        format: OutputFormat,
        quality: &str,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(ConversionProgress) + Send + 'static,
    {
        info!("Starting conversion: {} -> {:?}", url, output_path);

        // Extract video information
        let video_info = self.extractor.extract_info(url).await?;
        info!("Extracted video info: {}", video_info.title);

        // Get the best format for the requested quality
        let format_type = match format {
            OutputFormat::Mp3 { .. } => "audio",
            OutputFormat::Flac => "audio", 
            OutputFormat::Aac { .. } => "audio",
            OutputFormat::Ogg { .. } => "audio",
            _ => "video",
        };

        let best_format = self.extractor
            .get_best_format(&video_info, format_type, quality)
            .ok_or_else(|| anyhow::anyhow!("No suitable format found for quality: {}", quality))?;

        info!("Selected format: {} ({})", best_format.format_id, best_format.ext);

        // Create conversion settings
        let settings = ConversionSettings {
            input_url: best_format.url.clone(),
            output_path,
            format,
            preserve_quality: true,
            use_hardware_acceleration: true,
            thread_count: Some(num_cpus::get()),
        };

        // Convert
        self.converter.convert_with_progress(settings, progress_callback).await?;

        info!("Conversion completed successfully");
        Ok(())
    }

    /// Convert playlist
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
        // Extract playlist info (simplified - you'd implement proper playlist extraction)
        info!("Converting playlist: {}", playlist_url);
        
        // This is a placeholder - implement proper playlist extraction
        let video_urls = [playlist_url.to_string()]; // Replace with actual playlist extraction

        let mut conversion_settings = Vec::new();
        for (index, url) in video_urls.iter().enumerate() {
            let video_info = self.extractor.extract_info(url).await?;
            let safe_title = sanitize_filename(&video_info.title);
            let extension = get_extension_for_format(&format);
            let output_path = output_dir.join(format!("{:03}-{}.{}", index + 1, safe_title, extension));

            let format_type = match format {
                OutputFormat::Mp3 { .. } => "audio",
                OutputFormat::Flac => "audio",
                OutputFormat::Aac { .. } => "audio", 
                OutputFormat::Ogg { .. } => "audio",
                _ => "video",
            };

            if let Some(best_format) = self.extractor.get_best_format(&video_info, format_type, quality) {
                conversion_settings.push(ConversionSettings {
                    input_url: best_format.url.clone(),
                    output_path,
                    format: format.clone(),
                    preserve_quality: true,
                    use_hardware_acceleration: true,
                    thread_count: Some(num_cpus::get()),
                });
            }
        }

        self.converter.batch_convert(conversion_settings, progress_callback).await
    }

    /// Get video information without converting
    pub async fn get_video_info(&self, url: &str) -> Result<VideoInfo> {
        self.extractor.extract_info(url).await
    }
    
    /// Get playlist information
    pub async fn get_playlist_info(&self, url: &str) -> Result<PlaylistInfo> {
        self.extractor.extract_playlist_info(url).await
    }

    /// List available qualities for a video
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
        OutputFormat::Aac { .. } => "aac",
        OutputFormat::Ogg { .. } => "ogg",
        OutputFormat::WebM { .. } => "webm",
    }
}
