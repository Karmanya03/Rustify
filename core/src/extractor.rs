use crate::{runtime, AppConfig};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

/// YouTube video information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub duration: u64,
    pub uploader: String,
    pub upload_date: String,
    pub view_count: Option<u64>,
    pub formats: Vec<FormatInfo>,
    pub thumbnails: Vec<Thumbnail>,
}

/// Video/audio format information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub format_id: String,
    pub url: String,
    pub ext: String,
    pub format_note: Option<String>,
    pub acodec: Option<String>,
    pub vcodec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub abr: Option<f32>,
    pub vbr: Option<f32>,
    pub filesize: Option<u64>,
    pub quality: i32,
}

/// Thumbnail information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Playlist information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistInfo {
    pub id: String,
    pub title: String,
    pub uploader: String,
    pub video_count: usize,
    pub videos: Vec<PlaylistVideo>,
}

/// Individual video in a playlist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistVideo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub duration: Option<u64>,
    pub uploader: String,
}

/// Shared yt-dlp backed extractor used by every interface.
pub struct YouTubeExtractor {
    config: AppConfig,
}

impl YouTubeExtractor {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Extract video information from a URL.
    pub async fn extract_info(&self, url: &str) -> Result<VideoInfo> {
        info!("Extracting video info for: {}", url);

        let args = vec![
            "--dump-single-json".to_string(),
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
            url.to_string(),
        ];
        let output = runtime::run_ytdlp_capture(&self.config, &args).await?;
        let value: Value = serde_json::from_str(output.stdout.trim())
            .map_err(|error| anyhow!("Failed to parse yt-dlp JSON: {error}"))?;

        parse_video_info(&value, url)
    }

    /// Extract playlist information.
    pub async fn extract_playlist_info(&self, url: &str) -> Result<PlaylistInfo> {
        info!("Extracting playlist info for: {}", url);

        if crate::spotify::is_valid_spotify_playlist_url(url) {
            let playlist = crate::spotify::resolve_playlist(&self.config, url).await?;
            let videos = playlist
                .tracks
                .iter()
                .map(|track| PlaylistVideo {
                    id: track.id.clone(),
                    title: track.display_title(),
                    url: track.search_query(&self.config),
                    duration: track.duration_ms.map(|value| value / 1_000),
                    uploader: if track.artists.is_empty() {
                        playlist.owner.clone()
                    } else {
                        track.artists.join(", ")
                    },
                })
                .collect::<Vec<_>>();

            return Ok(PlaylistInfo {
                id: playlist.id,
                title: playlist.title,
                uploader: playlist.owner,
                video_count: playlist.total_tracks.max(videos.len()),
                videos,
            });
        }

        let args = vec![
            "--dump-single-json".to_string(),
            "--flat-playlist".to_string(),
            "--yes-playlist".to_string(),
            "--no-warnings".to_string(),
            url.to_string(),
        ];
        let output = runtime::run_ytdlp_capture(&self.config, &args).await?;
        let value: Value = serde_json::from_str(output.stdout.trim())
            .map_err(|error| anyhow!("Failed to parse playlist JSON: {error}"))?;

        let entries = value["entries"].as_array().cloned().unwrap_or_default();
        let videos = entries
            .iter()
            .filter_map(|entry| {
                let id = entry["id"].as_str()?.to_string();
                let video_url = entry["url"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}"));

                Some(PlaylistVideo {
                    id,
                    title: entry["title"].as_str().unwrap_or("Untitled").to_string(),
                    url: video_url,
                    duration: entry["duration"]
                        .as_u64()
                        .or_else(|| entry["duration"].as_f64().map(|value| value as u64)),
                    uploader: entry["uploader"].as_str().unwrap_or("").to_string(),
                })
            })
            .collect::<Vec<_>>();

        Ok(PlaylistInfo {
            id: value["id"].as_str().unwrap_or("").to_string(),
            title: value["title"].as_str().unwrap_or("Playlist").to_string(),
            uploader: value["uploader"].as_str().unwrap_or("").to_string(),
            video_count: videos.len(),
            videos,
        })
    }

    /// Get the best source format for audio or video.
    pub fn get_best_format<'a>(
        &self,
        video_info: &'a VideoInfo,
        format_type: &str,
        quality: &str,
    ) -> Option<&'a FormatInfo> {
        match format_type {
            "audio" => video_info
                .formats
                .iter()
                .filter(|format| {
                    format.vcodec.is_none()
                        && format.acodec.is_some()
                        && format.acodec.as_deref() != Some("none")
                })
                .max_by_key(|format| format.abr.unwrap_or(0.0) as u32),
            "video" => {
                let target_height = match quality {
                    "144p" => 144,
                    "240p" => 240,
                    "360p" => 360,
                    "480p" => 480,
                    "720p" => 720,
                    "1080p" | "1080p60" => 1080,
                    "1440p" => 1440,
                    "2160p" | "4k" | "4K" => 2160,
                    _ => 1080,
                };

                video_info
                    .formats
                    .iter()
                    .filter(|format| {
                        format.vcodec.is_some()
                            && format.vcodec.as_deref() != Some("none")
                            && format.height.is_some()
                    })
                    .min_by_key(|format| (format.height.unwrap() as i32 - target_height).abs())
            }
            _ => video_info.formats.first(),
        }
    }

    /// Extract a YouTube video ID from a URL.
    pub fn extract_video_id(&self, url: &str) -> Result<String> {
        crate::utils::extract_video_id(url)
            .ok_or_else(|| anyhow!("Could not extract video ID from URL: {url}"))
    }
}

fn parse_video_info(value: &Value, fallback_url: &str) -> Result<VideoInfo> {
    let id = value["id"]
        .as_str()
        .map(str::to_string)
        .or_else(|| crate::utils::extract_video_id(fallback_url))
        .unwrap_or_default();

    let formats = value["formats"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(parse_format_info)
        .collect::<Vec<_>>();

    let thumbnails = value["thumbnails"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|thumbnail| {
            Some(Thumbnail {
                url: thumbnail["url"].as_str()?.to_string(),
                width: thumbnail["width"].as_u64().map(|width| width as u32),
                height: thumbnail["height"].as_u64().map(|height| height as u32),
            })
        })
        .collect::<Vec<_>>();

    Ok(VideoInfo {
        id,
        title: value["title"].as_str().unwrap_or("Untitled").to_string(),
        duration: value["duration"]
            .as_u64()
            .or_else(|| value["duration"].as_f64().map(|seconds| seconds as u64))
            .unwrap_or_default(),
        uploader: value["uploader"].as_str().unwrap_or("").to_string(),
        upload_date: value["upload_date"].as_str().unwrap_or("").to_string(),
        view_count: value["view_count"].as_u64(),
        formats,
        thumbnails,
    })
}

fn parse_format_info(format: Value) -> FormatInfo {
    let acodec = format["acodec"].as_str().map(str::to_string);
    let vcodec = format["vcodec"].as_str().map(str::to_string);

    FormatInfo {
        format_id: format["format_id"].as_str().unwrap_or("").to_string(),
        url: format["url"].as_str().unwrap_or("").to_string(),
        ext: format["ext"].as_str().unwrap_or("").to_string(),
        format_note: format["format_note"]
            .as_str()
            .or_else(|| format["format"].as_str())
            .map(str::to_string),
        acodec: acodec.filter(|value| value != "none"),
        vcodec: vcodec.filter(|value| value != "none"),
        width: format["width"].as_u64().map(|width| width as u32),
        height: format["height"].as_u64().map(|height| height as u32),
        fps: format["fps"].as_f64().map(|fps| fps as f32),
        abr: format["abr"].as_f64().map(|abr| abr as f32),
        vbr: format["vbr"]
            .as_f64()
            .or_else(|| format["tbr"].as_f64())
            .map(|vbr| vbr as f32),
        filesize: format["filesize"]
            .as_u64()
            .or_else(|| format["filesize_approx"].as_u64()),
        quality: format["height"]
            .as_i64()
            .or_else(|| format["quality"].as_i64())
            .unwrap_or_default() as i32,
    }
}
