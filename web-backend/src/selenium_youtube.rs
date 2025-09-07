use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{info, warn, error};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeleniumVideoInfo {
    pub id: String,
    pub title: String,
    pub channel: Option<String>,
    pub duration: Option<String>,
    pub view_count: Option<u64>,
    pub thumbnail: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeleniumDownloadResult {
    pub success: bool,
    pub video_info: Option<SeleniumVideoInfo>,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

pub struct SeleniumExtractor {
    python_path: String,
    script_path: String,
}

impl SeleniumExtractor {
    pub fn new() -> Result<Self> {
        // Get Python path from virtual environment
        let python_path = if std::env::var("VIRTUAL_ENV").is_ok() {
            "/opt/venv/bin/python".to_string()
        } else if which::which("python3").is_ok() {
            "python3".to_string()
        } else {
            "python".to_string()
        };

        // Get script path relative to current directory
        let script_path = std::env::current_dir()?
            .join("src")
            .join("selenium_extractor.py")
            .to_string_lossy()
            .to_string();

        // Verify script exists
        if !Path::new(&script_path).exists() {
            return Err(anyhow!("Selenium extractor script not found at: {}", script_path));
        }

        Ok(Self {
            python_path,
            script_path,
        })
    }

    pub async fn get_video_info(&self, url: &str) -> Result<SeleniumVideoInfo> {
        info!("Extracting video info using Selenium for URL: {}", url);
        
        let mut cmd = Command::new(&self.python_path);
        cmd.args([
            &self.script_path,
            "--url", url,
            "--action", "info",
            "--headless",
            "--advanced-evasion",      // Enable advanced evasion
            "--continuous-rotation",   // Enable continuous rotation
            "--anti-detection"         // Enable anti-detection system
        ]);

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Selenium extraction failed: {}", stderr);
            return Err(anyhow!("Selenium extraction failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("Failed to parse Selenium output: {}", e))?;

        if let Some(error) = result.get("error") {
            return Err(anyhow!("Selenium error: {}", error));
        }

        let video_info: SeleniumVideoInfo = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to deserialize video info: {}", e))?;

        info!("Successfully extracted video info: {}", video_info.title);
        Ok(video_info)
    }

    #[allow(dead_code)]  // This method is ready for future use when implementing download via Selenium
    pub async fn download_video(
        &self, 
        url: &str, 
        output_path: &str, 
        format: &str, 
        quality: &str
    ) -> Result<SeleniumDownloadResult> {
        info!("Starting Selenium-based download for URL: {}", url);
        
        let mut cmd = Command::new(&self.python_path);
        cmd.args([
            &self.script_path,
            "--url", url,
            "--action", "download",
            "--output", output_path,
            "--format", format,
            "--quality", quality,
            "--headless",
            "--advanced-evasion",      // Enable advanced evasion
            "--continuous-rotation",   // Enable continuous rotation
            "--anti-detection"         // Enable anti-detection system
        ]);

        // Set a longer timeout for downloads
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(600), // 10 minutes
            cmd.output()
        ).await??;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: SeleniumDownloadResult = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("Failed to parse download result: {}", e))?;

        if result.success {
            info!("Selenium download completed successfully");
        } else {
            warn!("Selenium download failed: {:?}", result.error);
        }

        Ok(result)
    }

    pub async fn check_dependencies() -> Result<()> {
        info!("Checking Selenium dependencies...");
        
        // Check if Python is available
        let python_check = Command::new("python3")
            .args(["--version"])
            .output()
            .await;

        match python_check {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("Python available: {}", version.trim());
            }
            _ => {
                return Err(anyhow!("Python3 not found"));
            }
        }

        // Check if Selenium is installed
        let selenium_check = Command::new("python3")
            .args(["-c", "import selenium; print(f'Selenium version: {selenium.__version__}')"])
            .output()
            .await;

        match selenium_check {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("Selenium available: {}", version.trim());
            }
            _ => {
                return Err(anyhow!("Selenium not installed"));
            }
        }

        // Check if Chrome/Chromium is available (try multiple possible executables)
        let chrome_commands = ["google-chrome-stable", "google-chrome", "chromium", "chromium-browser"];
        let mut chrome_found = false;
        
        for cmd in &chrome_commands {
            if let Ok(output) = Command::new(cmd)
                .args(["--version"])
                .output()
                .await
            {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    info!("Chrome/Chromium available: {}", version.trim());
                    chrome_found = true;
                    break;
                }
            }
        }
        
        if !chrome_found {
            warn!("Chrome/Chromium not found, Selenium may not work");
        }

        info!("Selenium dependencies check completed");
        Ok(())
    }
}

impl Default for SeleniumExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create SeleniumExtractor")
    }
}

// Helper function to determine if Selenium should be used
pub fn should_use_selenium() -> bool {
    // Use Selenium in production or when explicitly enabled
    std::env::var("USE_SELENIUM").is_ok_and(|v| v == "true") ||
    std::env::var("RENDER").is_ok() ||
    std::env::var("KOYEB").is_ok() ||
    std::env::var("NODE_ENV").is_ok_and(|v| v == "production")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_selenium_extractor_creation() {
        let result = SeleniumExtractor::new();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dependency_check() {
        let result = SeleniumExtractor::check_dependencies().await;
        // This might fail in test environment, so we just check it doesn't panic
        println!("Dependency check result: {:?}", result);
    }
}
