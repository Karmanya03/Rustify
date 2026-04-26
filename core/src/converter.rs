use crate::{runtime, AppConfig};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// Output format specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputFormat {
    Mp3 { bitrate: u32 },
    Mp4 { resolution: String },
    Flac,
    Wav,
    Aac { bitrate: u32 },
    Ogg { quality: u8 },
    WebM { resolution: String },
}

/// Conversion progress information.
#[derive(Debug, Clone)]
pub struct ConversionProgress {
    pub percentage: f64,
    pub speed: String,
    pub eta: String,
    pub fps: Option<f32>,
    pub bitrate: Option<String>,
}

/// Conversion settings.
#[derive(Debug, Clone)]
pub struct ConversionSettings {
    pub input_url: String,
    pub output_path: PathBuf,
    pub format: OutputFormat,
    pub quality: String,
    pub metadata: Option<MediaMetadata>,
    pub preserve_quality: bool,
    pub use_hardware_acceleration: bool,
    pub thread_count: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct MediaMetadata {
    pub title: String,
    pub uploader: String,
    pub duration_seconds: Option<u64>,
}

/// Shared converter implementation used by every app surface.
pub struct Converter {
    config: AppConfig,
}

impl Converter {
    pub fn new(config: AppConfig) -> Result<Self> {
        info!("Initializing Rustify converter");
        Ok(Self { config })
    }

    /// Convert one video with progress tracking.
    pub async fn convert_with_progress<F>(
        &self,
        settings: ConversionSettings,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(ConversionProgress) + Send + Sync + 'static,
    {
        let callback: Arc<dyn Fn(ConversionProgress) + Send + Sync> = Arc::new(progress_callback);
        callback(ConversionProgress {
            percentage: 1.0,
            speed: "Preparing".to_string(),
            eta: "Starting".to_string(),
            fps: None,
            bitrate: None,
        });

        info!(
            "Starting conversion: {} -> {:?}",
            settings.input_url, settings.output_path
        );
        self.download_and_convert(&settings, callback.clone())
            .await?;
        info!("Conversion completed successfully");
        Ok(())
    }

    /// Convert multiple videos sequentially.
    pub async fn batch_convert<F>(
        &self,
        settings_list: Vec<ConversionSettings>,
        progress_callback: F,
    ) -> Result<Vec<Result<()>>>
    where
        F: Fn(usize, ConversionProgress) + Send + Sync + 'static + Clone,
    {
        info!(
            "Starting batch conversion of {} videos",
            settings_list.len()
        );

        let mut results = Vec::with_capacity(settings_list.len());
        let total = settings_list.len();
        for (index, settings) in settings_list.into_iter().enumerate() {
            if matches!(tokio::fs::metadata(&settings.output_path).await, Ok(metadata) if metadata.len() > 0)
            {
                progress_callback(
                    index,
                    ConversionProgress {
                        percentage: 100.0,
                        speed: "Skipped existing output".to_string(),
                        eta: "Done".to_string(),
                        fps: None,
                        bitrate: None,
                    },
                );
                results.push(Ok(()));
                continue;
            }

            let callback = progress_callback.clone();
            let result = self
                .convert_with_progress(settings, move |progress| callback(index, progress))
                .await;
            results.push(result);

            if index + 1 < total && self.config.rate_limits.request_delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(
                    self.config.rate_limits.request_delay_ms,
                ))
                .await;
            }
        }

        Ok(results)
    }

    pub fn check_ytdlp() -> bool {
        runtime::resolve_ytdlp(&AppConfig::default()).is_some()
    }

    pub fn check_ffmpeg() -> bool {
        runtime::resolve_ffmpeg(&AppConfig::default()).is_some()
    }

    async fn download_and_convert(
        &self,
        settings: &ConversionSettings,
        progress_callback: Arc<dyn Fn(ConversionProgress) + Send + Sync>,
    ) -> Result<()> {
        if let Some(parent) = settings.output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        if tokio::fs::try_exists(&settings.output_path).await? {
            tokio::fs::remove_file(&settings.output_path).await?;
        }

        let temp_dir = std::env::temp_dir().join(format!("rustify-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await?;

        let result = match &settings.format {
            OutputFormat::Flac => {
                self.download_source_audio(settings, &temp_dir, progress_callback.clone())
                    .await?;
                self.transcode_with_ffmpeg(
                    settings,
                    &temp_dir,
                    "flac",
                    &["-c:a", "flac", "-compression_level", "0"],
                    progress_callback.clone(),
                )
                .await
            }
            OutputFormat::Wav => {
                self.download_source_audio(settings, &temp_dir, progress_callback.clone())
                    .await?;
                let pcm_codec = if settings.quality.eq_ignore_ascii_case("hd") {
                    "pcm_s24le"
                } else {
                    "pcm_s16le"
                };

                self.transcode_with_ffmpeg(
                    settings,
                    &temp_dir,
                    "wav",
                    &["-c:a", pcm_codec],
                    progress_callback.clone(),
                )
                .await
            }
            _ => {
                self.run_ytdlp_direct(settings, &temp_dir, progress_callback.clone())
                    .await
            }
        };

        if let Err(error) = tokio::fs::remove_dir_all(&temp_dir).await {
            tracing::debug!("Skipping temp cleanup for {:?}: {}", temp_dir, error);
        }

        result
    }

    async fn run_ytdlp_direct(
        &self,
        settings: &ConversionSettings,
        temp_dir: &Path,
        progress_callback: Arc<dyn Fn(ConversionProgress) + Send + Sync>,
    ) -> Result<()> {
        let output_template = temp_template(temp_dir);
        let mut args = vec![
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
            "--output".to_string(),
            output_template,
            "--add-metadata".to_string(),
        ];

        match &settings.format {
            OutputFormat::Mp3 { bitrate } => {
                args.extend([
                    "--format".to_string(),
                    "bestaudio/best".to_string(),
                    "--extract-audio".to_string(),
                    "--audio-format".to_string(),
                    "mp3".to_string(),
                    "--audio-quality".to_string(),
                    format!("{bitrate}K"),
                ]);
            }
            OutputFormat::Aac { bitrate } => {
                args.extend([
                    "--format".to_string(),
                    "bestaudio/best".to_string(),
                    "--extract-audio".to_string(),
                    "--audio-format".to_string(),
                    "aac".to_string(),
                    "--audio-quality".to_string(),
                    format!("{bitrate}K"),
                ]);
            }
            OutputFormat::Ogg { quality } => {
                args.extend([
                    "--format".to_string(),
                    "bestaudio/best".to_string(),
                    "--extract-audio".to_string(),
                    "--audio-format".to_string(),
                    "vorbis".to_string(),
                    "--audio-quality".to_string(),
                    quality.to_string(),
                ]);
            }
            OutputFormat::Mp4 { resolution } => {
                args.extend([
                    "--format".to_string(),
                    build_video_selector("mp4", resolution),
                    "--merge-output-format".to_string(),
                    "mp4".to_string(),
                ]);
            }
            OutputFormat::WebM { resolution } => {
                args.extend([
                    "--format".to_string(),
                    build_video_selector("webm", resolution),
                    "--merge-output-format".to_string(),
                    "webm".to_string(),
                ]);
            }
            OutputFormat::Flac | OutputFormat::Wav => {
                unreachable!("lossless formats are handled by ffmpeg")
            }
        }

        args.push(settings.input_url.clone());

        let callback = progress_callback.clone();
        runtime::run_ytdlp_with_progress(
            &self.config,
            &args,
            Arc::new(move |progress| {
                callback(ConversionProgress {
                    percentage: progress.percentage.clamp(1.0, 99.0),
                    speed: progress.speed.clone(),
                    eta: progress.eta.clone(),
                    fps: None,
                    bitrate: None,
                });
            }),
        )
        .await?;

        let preferred_extension = extension_for_format(&settings.format);
        let produced_file = find_output_file(temp_dir, preferred_extension).await?;
        move_into_place(&produced_file, &settings.output_path).await?;

        progress_callback(ConversionProgress {
            percentage: 100.0,
            speed: "Complete".to_string(),
            eta: "Done".to_string(),
            fps: None,
            bitrate: None,
        });

        Ok(())
    }

    async fn download_source_audio(
        &self,
        settings: &ConversionSettings,
        temp_dir: &Path,
        progress_callback: Arc<dyn Fn(ConversionProgress) + Send + Sync>,
    ) -> Result<PathBuf> {
        let output_template = temp_template(temp_dir);
        let args = vec![
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
            "--format".to_string(),
            "bestaudio/best".to_string(),
            "--output".to_string(),
            output_template,
            settings.input_url.clone(),
        ];

        let callback = progress_callback.clone();
        runtime::run_ytdlp_with_progress(
            &self.config,
            &args,
            Arc::new(move |progress| {
                callback(ConversionProgress {
                    percentage: (progress.percentage * 0.7).clamp(1.0, 70.0),
                    speed: progress.speed.clone(),
                    eta: progress.eta.clone(),
                    fps: None,
                    bitrate: None,
                });
            }),
        )
        .await?;

        find_output_file(temp_dir, None).await
    }

    async fn transcode_with_ffmpeg(
        &self,
        settings: &ConversionSettings,
        temp_dir: &Path,
        extension: &str,
        codec_args: &[&str],
        progress_callback: Arc<dyn Fn(ConversionProgress) + Send + Sync>,
    ) -> Result<()> {
        let source_file = find_output_file(temp_dir, None).await?;
        progress_callback(ConversionProgress {
            percentage: 75.0,
            speed: "Transcoding".to_string(),
            eta: "Finalizing".to_string(),
            fps: None,
            bitrate: None,
        });

        let temp_output = temp_dir.join(format!("output.{extension}"));
        let mut args = vec![
            "-y".to_string(),
            "-i".to_string(),
            path_string(&source_file),
            "-vn".to_string(),
        ];

        if let Some(metadata) = &settings.metadata {
            if !metadata.title.is_empty() {
                args.extend(["-metadata".to_string(), format!("title={}", metadata.title)]);
            }
            if !metadata.uploader.is_empty() {
                args.extend([
                    "-metadata".to_string(),
                    format!("artist={}", metadata.uploader),
                ]);
            }
        }

        args.extend(codec_args.iter().map(|value| value.to_string()));
        args.push(path_string(&temp_output));

        runtime::run_ffmpeg(&self.config, &args).await?;

        progress_callback(ConversionProgress {
            percentage: 95.0,
            speed: "Writing output".to_string(),
            eta: "Almost done".to_string(),
            fps: None,
            bitrate: None,
        });

        move_into_place(&temp_output, &settings.output_path).await?;

        progress_callback(ConversionProgress {
            percentage: 100.0,
            speed: "Complete".to_string(),
            eta: "Done".to_string(),
            fps: None,
            bitrate: None,
        });

        Ok(())
    }
}

fn temp_template(temp_dir: &Path) -> String {
    let raw = temp_dir.join("%(title).120B-%(id)s.%(ext)s");
    path_string(&raw)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn extension_for_format(format: &OutputFormat) -> Option<&'static str> {
    match format {
        OutputFormat::Mp3 { .. } => Some("mp3"),
        OutputFormat::Mp4 { .. } => Some("mp4"),
        OutputFormat::Flac => Some("flac"),
        OutputFormat::Wav => Some("wav"),
        OutputFormat::Aac { .. } => Some("aac"),
        OutputFormat::Ogg { .. } => Some("ogg"),
        OutputFormat::WebM { .. } => Some("webm"),
    }
}

fn build_video_selector(container: &str, resolution: &str) -> String {
    let height = resolution
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .unwrap_or(1080);

    if container == "mp4" {
        format!(
            "bestvideo[ext=mp4][height<={height}]+bestaudio[ext=m4a]/best[ext=mp4][height<={height}]/bestvideo[height<={height}]+bestaudio/best"
        )
    } else {
        format!(
            "bestvideo[ext=webm][height<={height}]+bestaudio[ext=webm]/best[ext=webm][height<={height}]/bestvideo[height<={height}]+bestaudio/best"
        )
    }
}

async fn find_output_file(temp_dir: &Path, preferred_extension: Option<&str>) -> Result<PathBuf> {
    let mut entries = tokio::fs::read_dir(temp_dir).await?;
    let mut fallback = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let is_match = preferred_extension.map(|extension| {
            path.extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case(extension))
                .unwrap_or(false)
        });

        if is_match.unwrap_or(false) {
            return Ok(path);
        }

        fallback = Some(path);
    }

    fallback.ok_or_else(|| anyhow!("No output file was generated"))
}

async fn move_into_place(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    if tokio::fs::try_exists(to).await? {
        tokio::fs::remove_file(to).await?;
    }

    match tokio::fs::rename(from, to).await {
        Ok(()) => Ok(()),
        Err(_) => {
            tokio::fs::copy(from, to)
                .await
                .with_context(|| format!("Failed to copy {:?} to {:?}", from, to))?;
            tokio::fs::remove_file(from).await.ok();
            Ok(())
        }
    }
}
