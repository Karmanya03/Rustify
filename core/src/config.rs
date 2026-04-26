use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub download_dir: Option<PathBuf>,
    pub concurrent_downloads: usize,
    pub auth: AuthConfig,
    pub binaries: BinaryConfig,
    pub rate_limits: RateLimitConfig,
    pub spotify: SpotifyConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            download_dir: dirs::download_dir().or_else(|| std::env::current_dir().ok()),
            concurrent_downloads: 3,
            auth: AuthConfig::default(),
            binaries: BinaryConfig::default(),
            rate_limits: RateLimitConfig::default(),
            spotify: SpotifyConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub browser: Option<BrowserKind>,
    pub cookie_file: Option<PathBuf>,
    pub fallback_to_public: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::Auto,
            browser: None,
            cookie_file: None,
            fallback_to_public: true,
        }
    }
}

impl AuthConfig {
    pub fn browser_candidates(&self) -> Vec<BrowserKind> {
        if let Some(browser) = &self.browser {
            return vec![browser.clone()];
        }

        default_browser_order()
    }

    pub fn describe(&self) -> String {
        match self.mode {
            AuthMode::Auto => "Auto browser session reuse when YouTube asks for auth".to_string(),
            AuthMode::Browser => {
                let browser = self
                    .browser
                    .as_ref()
                    .map(BrowserKind::as_yt_dlp_name)
                    .unwrap_or("auto-detected browser");
                format!("Always use browser cookies from {browser}")
            }
            AuthMode::CookieFile => {
                let path = self
                    .cookie_file
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "configured cookie file".to_string());
                format!("Use exported cookie file at {path}")
            }
            AuthMode::None => "Never use browser cookies".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMode {
    Auto,
    Browser,
    CookieFile,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BrowserKind {
    Edge,
    Chrome,
    Firefox,
    Chromium,
    Brave,
    Safari,
}

impl BrowserKind {
    pub fn as_yt_dlp_name(&self) -> &'static str {
        match self {
            BrowserKind::Edge => "edge",
            BrowserKind::Chrome => "chrome",
            BrowserKind::Firefox => "firefox",
            BrowserKind::Chromium => "chromium",
            BrowserKind::Brave => "brave",
            BrowserKind::Safari => "safari",
        }
    }
}

impl std::str::FromStr for BrowserKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "edge" => Ok(Self::Edge),
            "chrome" => Ok(Self::Chrome),
            "firefox" => Ok(Self::Firefox),
            "chromium" => Ok(Self::Chromium),
            "brave" => Ok(Self::Brave),
            "safari" => Ok(Self::Safari),
            other => Err(format!("Unsupported browser: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BinaryConfig {
    pub yt_dlp: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    pub request_delay_ms: u64,
    pub max_retries: u32,
    pub backoff_base_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            request_delay_ms: 900,
            max_retries: 4,
            backoff_base_ms: 1_500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpotifyConfig {
    pub enabled: bool,
    pub market: Option<String>,
    pub fallback_to_page_scrape: bool,
    pub search_suffix: String,
    pub page_size: usize,
}

impl Default for SpotifyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            market: Some("from_token".to_string()),
            fallback_to_page_scrape: true,
            search_suffix: "official audio".to_string(),
            page_size: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub yt_dlp: ToolStatus,
    pub ffmpeg: ToolStatus,
    pub auth_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub available: bool,
    pub command: Option<String>,
    pub version: Option<String>,
    pub message: Option<String>,
}

impl ToolStatus {
    pub fn missing(message: impl Into<String>) -> Self {
        Self {
            available: false,
            command: None,
            version: None,
            message: Some(message.into()),
        }
    }
}

pub fn default_browser_order() -> Vec<BrowserKind> {
    if cfg!(target_os = "windows") {
        vec![BrowserKind::Edge, BrowserKind::Chrome, BrowserKind::Firefox]
    } else if cfg!(target_os = "macos") {
        vec![BrowserKind::Safari, BrowserKind::Chrome, BrowserKind::Firefox]
    } else {
        vec![BrowserKind::Firefox, BrowserKind::Chrome, BrowserKind::Chromium]
    }
}
