use anyhow::{anyhow, Result};
use regex::Regex;
use once_cell::sync::Lazy;

// YouTube URL validation
#[allow(dead_code)]
static YOUTUBE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://(www\.)?(youtube\.com/watch\?v=|youtu\.be/|youtube\.com/playlist\?list=)[a-zA-Z0-9_-]+").unwrap()
});

#[allow(dead_code)]
pub fn validate_youtube_url(url: &str) -> Result<()> {
    if !YOUTUBE_REGEX.is_match(url) {
        return Err(anyhow!("Invalid YouTube URL"));
    }
    Ok(())
}

#[allow(dead_code)]
pub fn validate_format(format: &str) -> Result<()> {
    match format {
        "mp3" | "mp4" | "wav" | "webm" => Ok(()),
        _ => Err(anyhow!("Unsupported format")),
    }
}

#[allow(dead_code)]
pub fn validate_quality(quality: &str) -> Result<()> {
    match quality {
        "144p" | "240p" | "360p" | "480p" | "720p" | "1080p" | "1440p" | "2160p" | "best" | "worst" => Ok(()),
        _ => Err(anyhow!("Invalid quality setting")),
    }
}

#[allow(dead_code)]
pub fn sanitize_filename(filename: &str) -> String {
    // Simple filename sanitization without external dependency
    filename
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect()
}

#[allow(dead_code)]
pub fn sanitize_html(input: &str) -> String {
    input
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('&', "&amp;")
}
