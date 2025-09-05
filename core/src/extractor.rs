use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

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

/// Video/Audio format information
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
    pub abr: Option<f32>, // Audio bitrate
    pub vbr: Option<f32>, // Video bitrate
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

/// YouTube extractor
pub struct YouTubeExtractor {
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl YouTubeExtractor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for YouTubeExtractor {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

impl YouTubeExtractor {
    /// Extract video information from URL
    pub async fn extract_info(&self, url: &str) -> Result<VideoInfo> {
        info!("Extracting video info for: {}", url);
        
        // For now, return dummy data with realistic titles
        // In real implementation, this would use youtube_dl crate or external yt-dlp
        warn!("Using placeholder video info - would use yt-dlp in real implementation");
        
        // Extract video ID from URL for more realistic titles
        let video_id = self.extract_video_id(url).unwrap_or_else(|_| "unknown".to_string());
        let realistic_title = match video_id.as_str() {
            "dQw4w9WgXcQ" => "Rick Astley - Never Gonna Give You Up (Official Video)",
            "9bZkp7q19f0" => "PSY - GANGNAM STYLE(강남스타일) M/V",
            "MHsI8hJmggI" => "Sample Music Video - High Quality Audio Test",
            "kJQP7kiw5Fk" => "Luis Fonsi - Despacito ft. Daddy Yankee",
            "fJ9rUzIMcZQ" => "Queen - Bohemian Rhapsody (Official Video)",
            "hTWKbfoikeg" => "Nirvana - Smells Like Teen Spirit (Official Music Video)",
            _ => "Sample YouTube Video - Test Audio"
        };
        
        Ok(VideoInfo {
            id: video_id,
            title: realistic_title.to_string(),
            duration: 180, // 3 minutes
            uploader: "Test Channel".to_string(),
            upload_date: "2024-01-01".to_string(),
            view_count: Some(1000000),
            formats: vec![
                FormatInfo {
                    format_id: "22".to_string(),
                    url: url.to_string(),
                    ext: "mp4".to_string(),
                    format_note: Some("720p".to_string()),
                    acodec: Some("aac".to_string()),
                    vcodec: Some("avc1".to_string()),
                    width: Some(1280),
                    height: Some(720),
                    fps: Some(30.0),
                    abr: Some(128.0),
                    vbr: Some(1000.0),
                    filesize: Some(50_000_000),
                    quality: 720,
                },
                FormatInfo {
                    format_id: "140".to_string(),
                    url: url.to_string(),
                    ext: "m4a".to_string(),
                    format_note: Some("audio only".to_string()),
                    acodec: Some("aac".to_string()),
                    vcodec: None,
                    width: None,
                    height: None,
                    fps: None,
                    abr: Some(128.0),
                    vbr: None,
                    filesize: Some(5_000_000),
                    quality: 0,
                },
            ],
            thumbnails: vec![
                Thumbnail {
                    url: "https://example.com/thumb.jpg".to_string(),
                    width: Some(1280),
                    height: Some(720),
                },
            ],
        })
    }

    /// Extract playlist information
    pub async fn extract_playlist_info(&self, url: &str) -> Result<PlaylistInfo> {
        info!("Extracting playlist info for: {}", url);
        
        // Placeholder implementation
        warn!("Using placeholder playlist info - would use yt-dlp in real implementation");
        
        Ok(PlaylistInfo {
            id: "dummy_playlist_id".to_string(),
            title: format!("Test Playlist from {}", url),
            uploader: "Test Channel".to_string(),
            video_count: 5,
            videos: vec![
                PlaylistVideo {
                    id: "video1".to_string(),
                    title: "Test Video 1".to_string(),
                    url: url.to_string(),
                    duration: Some(180),
                    uploader: "Test Channel".to_string(),
                },
                PlaylistVideo {
                    id: "video2".to_string(),
                    title: "Test Video 2".to_string(),
                    url: url.to_string(),
                    duration: Some(240),
                    uploader: "Test Channel".to_string(),
                },
            ],
        })
    }

    /// Get best format for quality and type
    pub fn get_best_format<'a>(&self, video_info: &'a VideoInfo, format_type: &str, quality: &str) -> Option<&'a FormatInfo> {
        match format_type {
            "audio" => {
                // Return audio-only format
                video_info.formats.iter()
                    .filter(|f| f.vcodec.is_none() && f.acodec.is_some())
                    .max_by_key(|f| f.abr.unwrap_or(0.0) as u32)
            }
            "video" => {
                // Return video format based on quality
                let target_height = match quality {
                    "144p" => 144,
                    "240p" => 240,
                    "360p" => 360,
                    "480p" => 480,
                    "720p" => 720,
                    "1080p" => 1080,
                    "1440p" => 1440,
                    "2160p" => 2160,
                    _ => 720, // default
                };

                video_info.formats.iter()
                    .filter(|f| f.vcodec.is_some() && f.height.is_some())
                    .min_by_key(|f| (f.height.unwrap() as i32 - target_height).abs())
            }
            _ => video_info.formats.first(),
        }
    }

    /// Extract video ID from URL
    pub fn extract_video_id(&self, url: &str) -> Result<String> {
        // Simple implementation - would be more robust in real app
        if url.contains("youtube.com/watch") {
            if let Some(start) = url.find("v=") {
                let id_start = start + 2;
                let id_end = url[id_start..].find('&').unwrap_or(url[id_start..].len());
                return Ok(url[id_start..id_start + id_end].to_string());
            }
        } else if url.contains("youtu.be/") {
            if let Some(start) = url.find("youtu.be/") {
                let id_start = start + 9;
                let id_end = url[id_start..].find('?').unwrap_or(url[id_start..].len());
                return Ok(url[id_start..id_start + id_end].to_string());
            }
        }
        
        anyhow::bail!("Could not extract video ID from URL: {}", url);
    }
}
