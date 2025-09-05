// Simple Input Validation
use regex::Regex;
use once_cell::sync::Lazy;

#[allow(dead_code)] // These functions are security utilities and should remain available
static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://(www\.)?(youtube\.com/watch\?v=|youtu\.be/)[a-zA-Z0-9_-]+").unwrap()
});

static SQL_INJECTION_PATTERNS: Lazy<Vec<&str>> = Lazy::new(|| {
    vec![
        "' OR '1'='1",
        "' OR 1=1",
        "'; DROP TABLE",
        "'; DELETE FROM",
        "UNION SELECT",
        "/*",
        "*/",
        "--",
        "xp_cmdshell",
    ]
});

static XSS_PATTERNS: Lazy<Vec<&str>> = Lazy::new(|| {
    vec![
        "<script",
        "</script>",
        "javascript:",
        "vbscript:",
        "onload=",
        "onerror=",
        "onclick=",
        "eval(",
        "alert(",
    ]
});

pub fn validate_youtube_url(url: &str) -> Result<(), String> {
    // Length check
    if url.len() > 2048 {
        return Err("URL too long".to_string());
    }

    // Pattern check
    if !URL_REGEX.is_match(url) {
        return Err("Invalid YouTube URL".to_string());
    }

    // Injection check
    if contains_injection_patterns(url) {
        return Err("Suspicious content detected".to_string());
    }

    Ok(())
}

pub fn validate_format(format: &str) -> Result<(), String> {
    let allowed_formats = ["mp3", "mp4", "wav", "flac", "m4a"];
    
    if !allowed_formats.contains(&format) {
        return Err("Invalid format".to_string());
    }

    Ok(())
}

pub fn validate_quality(quality: &str) -> Result<(), String> {
    let allowed_qualities = ["96", "128", "192", "256", "320", "flac"];
    
    if !allowed_qualities.contains(&quality) {
        return Err("Invalid quality".to_string());
    }

    Ok(())
}

pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .take(255)
        .collect()
}

pub fn sanitize_html(input: &str) -> String {
    input
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('&', "&amp;")
}

fn contains_injection_patterns(input: &str) -> bool {
    let input_lower = input.to_lowercase();
    
    SQL_INJECTION_PATTERNS.iter().any(|&pattern| input_lower.contains(&pattern.to_lowercase())) ||
    XSS_PATTERNS.iter().any(|&pattern| input_lower.contains(&pattern.to_lowercase()))
}
