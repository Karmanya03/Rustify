use serde::{Deserialize, Serialize};
use crate::{VideoInfo, FormatInfo};

/// Available quality options for a video
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityOptions {
    pub audio_qualities: Vec<AudioQuality>,
    pub video_qualities: Vec<VideoQuality>,
    pub best_audio: Option<AudioQuality>,
    pub best_video: Option<VideoQuality>,
}

/// Audio quality information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioQuality {
    pub bitrate: u32,
    pub format: String,
    pub codec: String,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub file_size: Option<u64>,
}

/// Video quality information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoQuality {
    pub resolution: String,
    pub width: u32,
    pub height: u32,
    pub fps: Option<f32>,
    pub codec: String,
    pub bitrate: Option<u32>,
    pub file_size: Option<u64>,
}

/// Quality presets for quick selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityPreset {
    /// Best available quality (highest resolution/bitrate)
    Best,
    /// Good quality with smaller file size
    Good,
    /// Standard quality for most uses
    Standard,
    /// Lower quality for bandwidth saving
    Low,
    /// Custom quality settings
    Custom {
        audio_bitrate: Option<u32>,
        video_resolution: Option<String>,
    },
}

/// Analyze available qualities from video info
pub fn analyze_available_qualities(info: &VideoInfo) -> QualityOptions {
    let mut audio_qualities = Vec::new();
    let mut video_qualities = Vec::new();

    // Extract audio qualities
    for format in &info.formats {
        if format.acodec.is_some() && format.vcodec.is_none() {
            if let Some(audio_quality) = parse_audio_quality(format) {
                audio_qualities.push(audio_quality);
            }
        }
    }

    // Extract video qualities
    for format in &info.formats {
        if format.vcodec.is_some() && format.width.is_some() && format.height.is_some() {
            if let Some(video_quality) = parse_video_quality(format) {
                video_qualities.push(video_quality);
            }
        }
    }

    // Sort by quality (descending)
    audio_qualities.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));
    video_qualities.sort_by(|a, b| (b.width * b.height).cmp(&(a.width * a.height)));

    // Remove duplicates
    audio_qualities.dedup_by(|a, b| a.bitrate == b.bitrate && a.codec == b.codec);
    video_qualities.dedup_by(|a, b| a.width == b.width && a.height == b.height);

    let best_audio = audio_qualities.first().cloned();
    let best_video = video_qualities.first().cloned();

    QualityOptions {
        audio_qualities,
        video_qualities,
        best_audio,
        best_video,
    }
}

fn parse_audio_quality(format: &FormatInfo) -> Option<AudioQuality> {
    let codec = format.acodec.as_ref()?.clone();
    let bitrate = format.abr.unwrap_or(128.0) as u32;
    
    // Determine format from codec
    let format_name = match codec.as_str() {
        "mp3" | "libmp3lame" => "MP3",
        "aac" | "libfdk_aac" => "AAC",
        "flac" => "FLAC",
        "vorbis" | "libvorbis" => "OGG",
        "opus" | "libopus" => "Opus",
        _ => "Unknown",
    };

    Some(AudioQuality {
        bitrate,
        format: format_name.to_string(),
        codec,
        sample_rate: None, // Could be extracted from format details
        channels: None,    // Could be extracted from format details
        file_size: format.filesize,
    })
}

fn parse_video_quality(format: &FormatInfo) -> Option<VideoQuality> {
    let width = format.width?;
    let height = format.height?;
    let codec = format.vcodec.as_ref()?.clone();
    
    let resolution = match height {
        2160 => "4K (2160p)".to_string(),
        1440 => "1440p".to_string(),
        1080 => "1080p".to_string(),
        720 => "720p".to_string(),
        480 => "480p".to_string(),
        360 => "360p".to_string(),
        240 => "240p".to_string(),
        144 => "144p".to_string(),
        _ => format!("{}p", height),
    };

    let bitrate = format.vbr.map(|b| b as u32);

    Some(VideoQuality {
        resolution,
        width,
        height,
        fps: format.fps,
        codec,
        bitrate,
        file_size: format.filesize,
    })
}

/// Get optimal quality settings based on preset
pub fn get_preset_settings(preset: QualityPreset, available: &QualityOptions) -> (Option<u32>, Option<String>) {
    match preset {
        QualityPreset::Best => {
            let audio_bitrate = available.best_audio.as_ref().map(|a| a.bitrate);
            let video_resolution = available.best_video.as_ref().map(|v| v.resolution.clone());
            (audio_bitrate, video_resolution)
        }
        
        QualityPreset::Good => {
            // Select second-best or 256kbps audio and 1080p video
            let audio_bitrate = available.audio_qualities
                .iter()
                .find(|a| a.bitrate <= 256)
                .or(available.audio_qualities.get(1))
                .map(|a| a.bitrate);
                
            let video_resolution = available.video_qualities
                .iter()
                .find(|v| v.height <= 1080)
                .map(|v| v.resolution.clone());
                
            (audio_bitrate, video_resolution)
        }
        
        QualityPreset::Standard => {
            // 192kbps audio and 720p video
            let audio_bitrate = available.audio_qualities
                .iter()
                .find(|a| a.bitrate <= 192)
                .map(|a| a.bitrate)
                .or(Some(192));
                
            let video_resolution = available.video_qualities
                .iter()
                .find(|v| v.height <= 720)
                .map(|v| v.resolution.clone())
                .or(Some("720p".to_string()));
                
            (audio_bitrate, video_resolution)
        }
        
        QualityPreset::Low => {
            // 128kbps audio and 480p video
            let audio_bitrate = Some(128);
            let video_resolution = available.video_qualities
                .iter()
                .find(|v| v.height <= 480)
                .map(|v| v.resolution.clone())
                .or(Some("480p".to_string()));
                
            (audio_bitrate, video_resolution)
        }
        
        QualityPreset::Custom { audio_bitrate, video_resolution } => {
            (audio_bitrate, video_resolution)
        }
    }
}

/// Estimate file size based on quality settings
pub fn estimate_file_size(duration_seconds: u64, audio_bitrate: Option<u32>, video_resolution: Option<&str>) -> u64 {
    let mut total_bitrate = 0u32;
    
    // Add audio bitrate
    if let Some(audio_br) = audio_bitrate {
        total_bitrate += audio_br;
    }
    
    // Estimate video bitrate based on resolution
    if let Some(resolution) = video_resolution {
        let video_bitrate = match resolution {
            s if s.contains("2160") || s.contains("4K") => 15000, // 15 Mbps for 4K
            s if s.contains("1440") => 8000,  // 8 Mbps for 1440p
            s if s.contains("1080") => 4000,  // 4 Mbps for 1080p
            s if s.contains("720") => 2000,   // 2 Mbps for 720p
            s if s.contains("480") => 1000,   // 1 Mbps for 480p
            _ => 500,                         // 500 kbps for lower resolutions
        };
        total_bitrate += video_bitrate;
    }
    
    // Convert to bytes: (bitrate in kbps * duration in seconds) / 8 / 1000
    (total_bitrate as u64 * duration_seconds * 1000) / 8
}

/// Get human-readable file size
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
