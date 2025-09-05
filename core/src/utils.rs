use std::path::Path;
use regex::Regex;
use chrono::Utc;

/// Sanitize filename for safe filesystem usage
pub fn sanitize_filename(name: &str) -> String {
    // Remove or replace invalid filename characters
    let invalid_chars = Regex::new(r#"[<>:"/\\|?*]"#).unwrap();
    let sanitized = invalid_chars.replace_all(name, "_");
    
    // Trim whitespace and dots
    let trimmed = sanitized.trim_matches(|c: char| c.is_whitespace() || c == '.');
    
    // Limit length to 200 characters
    let limited = if trimmed.len() > 200 {
        &trimmed[..200]
    } else {
        trimmed
    };
    
    // Ensure not empty
    if limited.is_empty() {
        "untitled".to_string()
    } else {
        limited.to_string()
    }
}

/// Format duration from seconds to human-readable format
pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}

/// Parse duration string (HH:MM:SS or MM:SS) to seconds
pub fn parse_duration(duration_str: &str) -> Option<u64> {
    let parts: Vec<&str> = duration_str.split(':').collect();
    
    match parts.len() {
        2 => {
            // MM:SS format
            let minutes = parts[0].parse::<u64>().ok()?;
            let seconds = parts[1].parse::<u64>().ok()?;
            Some(minutes * 60 + seconds)
        }
        3 => {
            // HH:MM:SS format
            let hours = parts[0].parse::<u64>().ok()?;
            let minutes = parts[1].parse::<u64>().ok()?;
            let seconds = parts[2].parse::<u64>().ok()?;
            Some(hours * 3600 + minutes * 60 + seconds)
        }
        _ => None,
    }
}

/// Generate output filename with timestamp
pub fn generate_output_filename(title: &str, extension: &str, add_timestamp: bool) -> String {
    let clean_title = sanitize_filename(title);
    
    if add_timestamp {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        format!("{}_{}.{}", clean_title, timestamp, extension)
    } else {
        format!("{}.{}", clean_title, extension)
    }
}

/// Check if FFmpeg is available
pub fn check_ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get number of CPU cores for optimal threading
pub fn get_optimal_thread_count() -> usize {
    let cpu_count = num_cpus::get();
    // Use 75% of available cores, minimum 1, maximum 8
    ((cpu_count * 3) / 4).clamp(1, 8)
}

/// Validate YouTube URL format
pub fn is_valid_youtube_url(url: &str) -> bool {
    let youtube_patterns = [
        r"^https?://(www\.)?youtube\.com/watch\?v=[\w-]+",
        r"^https?://(www\.)?youtu\.be/[\w-]+",
        r"^https?://(www\.)?youtube\.com/playlist\?list=[\w-]+",
        r"^https?://(www\.)?youtube\.com/channel/[\w-]+",
        r"^https?://(www\.)?youtube\.com/@[\w-]+",
    ];
    
    youtube_patterns.iter().any(|pattern| {
        Regex::new(pattern).unwrap().is_match(url)
    })
}

/// Extract video ID from YouTube URL
pub fn extract_video_id(url: &str) -> Option<String> {
    // For youtube.com/watch?v=ID
    if let Some(captures) = Regex::new(r"[?&]v=([^&]+)").unwrap().captures(url) {
        return Some(captures[1].to_string());
    }
    
    // For youtu.be/ID
    if let Some(captures) = Regex::new(r"youtu\.be/([^?]+)").unwrap().captures(url) {
        return Some(captures[1].to_string());
    }
    
    None
}

/// Format bytes as human-readable size
pub fn format_bytes(bytes: u64) -> String {
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

/// Calculate download speed from bytes and duration
pub fn calculate_speed(bytes: u64, duration_secs: f64) -> String {
    if duration_secs <= 0.0 {
        return "0 B/s".to_string();
    }
    
    let bytes_per_sec = bytes as f64 / duration_secs;
    format!("{}/s", format_bytes(bytes_per_sec as u64))
}

/// Estimate time remaining based on current progress
pub fn estimate_eta(total_bytes: u64, downloaded_bytes: u64, speed_bytes_per_sec: f64) -> String {
    if speed_bytes_per_sec <= 0.0 || downloaded_bytes >= total_bytes {
        return "Unknown".to_string();
    }
    
    let remaining_bytes = total_bytes - downloaded_bytes;
    let eta_seconds = remaining_bytes as f64 / speed_bytes_per_sec;
    
    if eta_seconds > 3600.0 {
        format!("{:.0}h {:.0}m", eta_seconds / 3600.0, (eta_seconds % 3600.0) / 60.0)
    } else if eta_seconds > 60.0 {
        format!("{:.0}m {:.0}s", eta_seconds / 60.0, eta_seconds % 60.0)
    } else {
        format!("{:.0}s", eta_seconds)
    }
}

/// Create directory if it doesn't exist
pub fn ensure_directory_exists(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Get file extension from path
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
}

/// Check if file has video extension
pub fn is_video_file(path: &Path) -> bool {
    if let Some(ext) = get_file_extension(path) {
        matches!(ext.as_str(), "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm")
    } else {
        false
    }
}

/// Check if file has audio extension
pub fn is_audio_file(path: &Path) -> bool {
    if let Some(ext) = get_file_extension(path) {
        matches!(ext.as_str(), "mp3" | "flac" | "aac" | "ogg" | "wav" | "m4a")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello<World>"), "Hello_World_");
        assert_eq!(sanitize_filename("  test.txt  "), "test.txt");
        assert_eq!(sanitize_filename(""), "untitled");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(65), "01:05");
        assert_eq!(format_duration(3661), "01:01:01");
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("01:05"), Some(65));
        assert_eq!(parse_duration("01:01:01"), Some(3661));
        assert_eq!(parse_duration("invalid"), None);
    }

    #[test]
    fn test_validate_youtube_url() {
        assert!(is_valid_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_valid_youtube_url("https://youtu.be/dQw4w9WgXcQ"));
        assert!(!is_valid_youtube_url("https://example.com"));
    }

    #[test]
    fn test_extract_video_id() {
        assert_eq!(extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"), Some("dQw4w9WgXcQ".to_string()));
        assert_eq!(extract_video_id("https://youtu.be/dQw4w9WgXcQ"), Some("dQw4w9WgXcQ".to_string()));
    }
}
