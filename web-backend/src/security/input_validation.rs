// Input Validation - OWASP A03: Injection Prevention
use regex::Regex;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use std::collections::HashSet;
use crate::security::{log_security_event, SecurityEventType, SecuritySeverity};

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ConvertRequest {
    #[validate(custom = "validate_youtube_url")]
    pub url: String,
    
    #[validate(custom = "validate_format")]
    pub format: String,
    
    #[validate(custom = "validate_quality")]
    pub quality: String,
    
    #[validate(length(max = 255))]
    pub output_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct PlaylistRequest {
    #[validate(custom = "validate_youtube_playlist_url")]
    pub url: String,
    
    #[validate(custom = "validate_format")]
    pub format: String,
    
    #[validate(custom = "validate_quality")]
    pub quality: String,
    
    #[validate(range(min = 1, max = 100))]
    pub max_videos: Option<u32>,
}

// URL validation - prevents injection and SSRF
pub fn validate_youtube_url(url: &str) -> Result<(), ValidationError> {
    // Check for injection patterns
    if contains_injection_patterns(url) {
        tokio::spawn(log_security_event(
            "unknown",
            None,
            SecurityEventType::SQLInjectionAttempt,
            &format!("Injection pattern detected in URL: {}", url),
            SecuritySeverity::High,
        ));
        return Err(ValidationError::new("injection_detected"));
    }

    // Check URL length
    if url.len() > 2048 {
        return Err(ValidationError::new("url_too_long"));
    }

    // Parse URL
    let parsed_url = url::Url::parse(url)
        .map_err(|_| ValidationError::new("invalid_url"))?;

    // Check scheme (prevent file:// and other protocols)
    if parsed_url.scheme() != "https" && parsed_url.scheme() != "http" {
        tokio::spawn(log_security_event(
            "unknown",
            None,
            SecurityEventType::SuspiciousUrl,
            &format!("Invalid URL scheme: {}", parsed_url.scheme()),
            SecuritySeverity::Medium,
        ));
        return Err(ValidationError::new("invalid_scheme"));
    }

    // Check domain (prevent SSRF)
    let host = parsed_url.host_str().ok_or_else(|| ValidationError::new("no_host"))?;
    
    let allowed_domains = vec![
        "youtube.com",
        "www.youtube.com",
        "youtu.be",
        "m.youtube.com",
    ];

    if !allowed_domains.iter().any(|&domain| host == domain) {
        tokio::spawn(log_security_event(
            "unknown",
            None,
            SecurityEventType::SuspiciousUrl,
            &format!("Unauthorized domain: {}", host),
            SecuritySeverity::Medium,
        ));
        return Err(ValidationError::new("unauthorized_domain"));
    }

    // Check for path traversal attempts
    if parsed_url.path().contains("..") || parsed_url.path().contains("%2e%2e") {
        tokio::spawn(log_security_event(
            "unknown",
            None,
            SecurityEventType::PathTraversal,
            &format!("Path traversal attempt: {}", parsed_url.path()),
            SecuritySeverity::High,
        ));
        return Err(ValidationError::new("path_traversal"));
    }

    Ok(())
}

pub fn validate_youtube_playlist_url(url: &str) -> Result<(), ValidationError> {
    validate_youtube_url(url)?;
    
    // Additional playlist-specific validation
    if !url.contains("list=") && !url.contains("playlist") {
        return Err(ValidationError::new("not_playlist_url"));
    }

    Ok(())
}

// Format validation
pub fn validate_format(format: &str) -> Result<(), ValidationError> {
    let allowed_formats = ["mp3", "mp4", "wav", "flac", "m4a"];
    
    if !allowed_formats.contains(&format) {
        tokio::spawn(log_security_event(
            "unknown",
            None,
            SecurityEventType::InvalidInput,
            &format!("Invalid format requested: {}", format),
            SecuritySeverity::Low,
        ));
        return Err(ValidationError::new("invalid_format"));
    }

    Ok(())
}

// Quality validation
pub fn validate_quality(quality: &str) -> Result<(), ValidationError> {
    let allowed_qualities = ["96", "128", "192", "256", "320", "flac"];
    
    if !allowed_qualities.contains(&quality) {
        tokio::spawn(log_security_event(
            "unknown",
            None,
            SecurityEventType::InvalidInput,
            &format!("Invalid quality requested: {}", quality),
            SecuritySeverity::Low,
        ));
        return Err(ValidationError::new("invalid_quality"));
    }

    Ok(())
}

// Injection detection patterns
fn contains_injection_patterns(input: &str) -> bool {
    let sql_patterns = [
        "' OR '1'='1",
        "' OR 1=1",
        "'; DROP TABLE",
        "'; DELETE FROM",
        "UNION SELECT",
        "INSERT INTO",
        "UPDATE SET",
        "' UNION",
        "/*",
        "*/",
        "--",
        "xp_cmdshell",
        "sp_executesql",
    ];

    let xss_patterns = [
        "<script",
        "</script>",
        "javascript:",
        "vbscript:",
        "onload=",
        "onerror=",
        "onclick=",
        "onmouseover=",
        "eval(",
        "alert(",
        "confirm(",
        "prompt(",
    ];

    let input_lower = input.to_lowercase();
    
    sql_patterns.iter().any(|&pattern| input_lower.contains(&pattern.to_lowercase())) ||
    xss_patterns.iter().any(|&pattern| input_lower.contains(&pattern.to_lowercase()))
}

// Sanitize filename to prevent path traversal
pub fn sanitize_filename(filename: &str) -> String {
    use sanitize_filename::sanitize;
    
    // Remove path traversal attempts
    let clean = filename.replace("..", "").replace("/", "").replace("\\", "");
    
    // Use sanitize_filename crate for additional cleaning
    let sanitized = sanitize(&clean);
    
    // Limit length
    if sanitized.len() > 255 {
        sanitized.chars().take(255).collect()
    } else {
        sanitized
    }
}

// HTML/XML sanitization to prevent XSS
pub fn sanitize_html(input: &str) -> String {
    input
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('&', "&amp;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injection_detection() {
        assert!(contains_injection_patterns("' OR '1'='1"));
        assert!(contains_injection_patterns("<script>alert('xss')</script>"));
        assert!(!contains_injection_patterns("https://youtube.com/watch?v=abc123"));
    }

    #[test]
    fn test_filename_sanitization() {
        assert_eq!(sanitize_filename("../../../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize_filename("test<script>alert()</script>.mp3"), "testscriptalertscript.mp3");
    }
}
