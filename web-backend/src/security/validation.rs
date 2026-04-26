use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;

// YouTube URL validation
#[allow(dead_code)]
static YOUTUBE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://(www\.)?(youtube\.com/watch\?v=|youtu\.be/|youtube\.com/playlist\?list=)[a-zA-Z0-9_-]+").unwrap()
});

// Spotify playlist URL validation
#[allow(dead_code)]
static SPOTIFY_PLAYLIST_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(https?://open\.spotify\.com/(intl-[a-z-]+/)?playlist/[A-Za-z0-9]+(\?.*)?|spotify:playlist:[A-Za-z0-9]+)$").unwrap()
});

#[allow(dead_code)]
pub fn validate_youtube_url(url: &str) -> Result<()> {
    if !YOUTUBE_REGEX.is_match(url) {
        return Err(anyhow!("Invalid YouTube URL"));
    }
    Ok(())
}

#[allow(dead_code)]
pub fn validate_playlist_url(url: &str) -> Result<()> {
    if YOUTUBE_REGEX.is_match(url) || SPOTIFY_PLAYLIST_REGEX.is_match(url) {
        return Ok(());
    }

    Err(anyhow!(
        "Invalid playlist URL. Rustify supports YouTube and Spotify playlist links."
    ))
}

#[allow(dead_code)]
pub fn validate_format(format: &str) -> Result<()> {
    match format {
        "mp3" | "flac" | "wav" | "aac" | "ogg" | "mp4" | "webm" => Ok(()),
        _ => Err(anyhow!("Unsupported format")),
    }
}

#[allow(dead_code)]
pub fn validate_quality(quality: &str) -> Result<()> {
    match quality {
        "lossless" | "hd" | "144p" | "240p" | "360p" | "480p" | "720p" | "1080p" | "1440p"
        | "2160p" | "best" | "worst" => Ok(()),
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
