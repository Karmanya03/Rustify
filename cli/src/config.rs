use anyhow::{anyhow, Context, Result};
use rustify_core::{AppConfig, AuthMode, BrowserKind};
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_config() -> Result<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config from {}", path.display()))?;
    let config = serde_json::from_str::<AppConfig>(&contents)
        .with_context(|| format!("Failed to parse config from {}", path.display()))?;

    Ok(config)
}

pub fn save_config(config: &AppConfig) -> Result<PathBuf> {
    let path = config_path()?;
    ensure_parent_dir(&path)?;
    let contents = serde_json::to_string_pretty(config)?;
    fs::write(&path, contents)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(path)
}

pub fn show_config() -> Result<()> {
    let config = load_config()?;
    let path = config_path()?;
    println!("Config file: {}", path.display());
    println!("{}", serde_json::to_string_pretty(&config)?);
    Ok(())
}

pub fn set_config(key: &str, value: &str) -> Result<()> {
    let mut config = load_config()?;

    match key {
        "download_dir" => config.download_dir = Some(PathBuf::from(value)),
        "concurrent_downloads" => {
            config.concurrent_downloads = value
                .parse::<usize>()
                .context("concurrent_downloads must be a positive integer")?;
        }
        "auth.mode" => {
            config.auth.mode = match value.trim().to_ascii_lowercase().as_str() {
                "auto" => AuthMode::Auto,
                "browser" => AuthMode::Browser,
                "cookie-file" | "cookie_file" => AuthMode::CookieFile,
                "none" => AuthMode::None,
                other => return Err(anyhow!("Unsupported auth.mode value: {other}")),
            };
        }
        "auth.browser" => {
            config.auth.browser = Some(value.parse::<BrowserKind>().map_err(anyhow::Error::msg)?);
        }
        "auth.cookie_file" => {
            config.auth.cookie_file = Some(PathBuf::from(value));
        }
        "auth.fallback_to_public" => {
            config.auth.fallback_to_public = value
                .parse::<bool>()
                .context("auth.fallback_to_public must be true or false")?;
        }
        "binaries.yt_dlp" => config.binaries.yt_dlp = Some(PathBuf::from(value)),
        "binaries.ffmpeg" => config.binaries.ffmpeg = Some(PathBuf::from(value)),
        "rate_limits.request_delay_ms" => {
            config.rate_limits.request_delay_ms = value
                .parse::<u64>()
                .context("rate_limits.request_delay_ms must be a non-negative integer")?;
        }
        "rate_limits.max_retries" => {
            config.rate_limits.max_retries = value
                .parse::<u32>()
                .context("rate_limits.max_retries must be a non-negative integer")?;
        }
        "rate_limits.backoff_base_ms" => {
            config.rate_limits.backoff_base_ms = value
                .parse::<u64>()
                .context("rate_limits.backoff_base_ms must be a non-negative integer")?;
        }
        "spotify.enabled" => {
            config.spotify.enabled = value
                .parse::<bool>()
                .context("spotify.enabled must be true or false")?;
        }
        "spotify.market" => {
            config.spotify.market = Some(value.trim().to_string());
        }
        "spotify.fallback_to_page_scrape" => {
            config.spotify.fallback_to_page_scrape = value
                .parse::<bool>()
                .context("spotify.fallback_to_page_scrape must be true or false")?;
        }
        "spotify.search_suffix" => {
            config.spotify.search_suffix = value.to_string();
        }
        "spotify.page_size" => {
            config.spotify.page_size = value
                .parse::<usize>()
                .context("spotify.page_size must be a positive integer")?
                .clamp(1, 100);
        }
        other => {
            return Err(anyhow!(
                "Unknown config key: {other}. Supported keys: download_dir, concurrent_downloads, auth.mode, auth.browser, auth.cookie_file, auth.fallback_to_public, binaries.yt_dlp, binaries.ffmpeg, rate_limits.request_delay_ms, rate_limits.max_retries, rate_limits.backoff_base_ms, spotify.enabled, spotify.market, spotify.fallback_to_page_scrape, spotify.search_suffix, spotify.page_size"
            ));
        }
    }

    let path = save_config(&config)?;
    println!("Updated config at {}", path.display());
    Ok(())
}

pub fn reset_config() -> Result<()> {
    let config = AppConfig::default();
    let path = save_config(&config)?;
    println!("Reset config at {}", path.display());
    Ok(())
}

pub fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow!("Could not locate a config directory"))?;
    Ok(base.join("rustify").join("config.json"))
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    Ok(())
}
