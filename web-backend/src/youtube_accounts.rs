use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn, error};
use crate::temp_mail::{TempMailService, TempMailAccount};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeAccount {
    pub email: String,
    pub password: String,
    pub cookies: Option<String>,
    pub po_token: Option<String>,
    pub visitor_data: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub usage_count: u32,
    pub is_banned: bool,
}

#[derive(Debug, Clone)]
pub struct YouTubeAccountManager {
    client: Client,
    temp_mail: TempMailService,
    accounts: Vec<YouTubeAccount>,
    max_usage_per_account: u32,
    account_rotation_hours: u64,
}

impl YouTubeAccountManager {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
                .build()
                .unwrap(),
            temp_mail: TempMailService::new(),
            accounts: Vec::new(),
            max_usage_per_account: 50, // Conservative limit for free accounts
            account_rotation_hours: 24, // Rotate accounts every 24 hours
        }
    }

    /// Create a new throwaway YouTube account automatically
    pub async fn create_throwaway_account(&mut self) -> Result<YouTubeAccount> {
        info!("Creating new throwaway YouTube account...");

        // Step 1: Generate temporary email
        let temp_account = self.temp_mail.generate_temp_email().await?;
        info!("Generated temp email: {}", temp_account.email);

        // Step 2: Create Google account using automated browser
        let google_account = self.create_google_account(&temp_account).await?;
        info!("Created Google account for: {}", google_account.email);

        // Step 3: Extract cookies and visitor data
        let (cookies, visitor_data) = self.extract_youtube_data(&google_account).await?;

        let youtube_account = YouTubeAccount {
            email: google_account.email,
            password: google_account.password,
            cookies: Some(cookies),
            po_token: None, // Will be extracted later if needed
            visitor_data: Some(visitor_data),
            created_at: chrono::Utc::now(),
            last_used: None,
            usage_count: 0,
            is_banned: false,
        };

        self.accounts.push(youtube_account.clone());
        info!("Successfully created throwaway YouTube account");

        Ok(youtube_account)
    }

    /// Get a fresh account for downloading (with rotation logic)
    pub async fn get_fresh_account(&mut self) -> Result<&mut YouTubeAccount> {
        // Clean up old/banned accounts
        self.cleanup_accounts();

        // Find a usable account
        if let Some(account) = self.find_usable_account() {
            return Ok(account);
        }

        // No usable accounts, create a new one
        warn!("No usable accounts found, creating new throwaway account...");
        self.create_throwaway_account().await?;

        // Get the newly created account
        self.accounts.last_mut()
            .ok_or_else(|| anyhow!("Failed to get newly created account"))
    }

    /// Extract PO Token from account (when needed for specific videos)
    pub async fn extract_po_token(&self, account: &YouTubeAccount) -> Result<String> {
        info!("Extracting PO Token for account: {}", account.email);

        // Use headless browser to extract PO token
        let po_token = self.extract_po_token_with_browser(account).await?;

        info!("Successfully extracted PO Token");
        Ok(po_token)
    }

    /// Generate yt-dlp arguments with account authentication
    pub fn generate_yt_dlp_args(&self, account: &YouTubeAccount, use_po_token: bool) -> Vec<String> {
        let mut args = Vec::new();

        // Add visitor data if available
        if let Some(visitor_data) = &account.visitor_data {
            args.extend([
                "--extractor-args".to_string(),
                format!("youtubetab:skip=webpage"),
                "--extractor-args".to_string(),
                format!("youtube:player_skip=webpage,configs;visitor_data={}", visitor_data),
            ]);
        }

        // Add PO token if requested and available
        if use_po_token {
            if let Some(po_token) = &account.po_token {
                args.extend([
                    "--extractor-args".to_string(),
                    format!("youtube:po_token={}", po_token),
                ]);
            }
        }

        // Add cookies if available
        if let Some(_cookies) = &account.cookies {
            // Save cookies to temporary file and use with yt-dlp
            if let Ok(cookies_file) = self.save_cookies_to_temp_file(account) {
                args.extend([
                    "--cookies".to_string(),
                    cookies_file,
                ]);
            }
        }

        // Add enhanced bot protection headers
        args.extend([
            "--user-agent".to_string(),
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".to_string(),
            "--add-header".to_string(),
            "Accept-Language:en-US,en;q=0.9".to_string(),
            "--add-header".to_string(),
            "Accept:text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8".to_string(),
            "--add-header".to_string(),
            "Accept-Encoding:gzip, deflate, br".to_string(),
            "--add-header".to_string(),
            "DNT:1".to_string(),
            "--add-header".to_string(),
            "Connection:keep-alive".to_string(),
            "--add-header".to_string(),
            "Upgrade-Insecure-Requests:1".to_string(),
        ]);

        // Add retry and bypass options
        args.extend([
            "--extractor-retries".to_string(),
            "5".to_string(),
            "--fragment-retries".to_string(),
            "5".to_string(),
            "--retry-sleep".to_string(),
            "exp=1:120".to_string(), // Exponential backoff
            "--geo-bypass".to_string(),
            "--no-check-certificate".to_string(),
            "--ignore-errors".to_string(),
        ]);

        args
    }

    // Private helper methods

    async fn create_google_account(&self, temp_account: &TempMailAccount) -> Result<TempMailAccount> {
        // This would use a headless browser automation library like selenium or playwright
        // For now, we'll simulate the process
        
        warn!("Google account creation would require browser automation");
        warn!("This is a placeholder - implement with headless browser");
        
        // Return the temp account as-is for now
        Ok(temp_account.clone())
    }

    async fn extract_youtube_data(&self, account: &TempMailAccount) -> Result<(String, String)> {
        // Extract cookies and visitor data using browser automation
        
        warn!("YouTube data extraction would require browser automation");
        warn!("This is a placeholder - implement with headless browser");
        
        // Return dummy data for now
        let cookies = "dummy_cookies=placeholder".to_string();
        let visitor_data = "dummy_visitor_data".to_string();
        
        Ok((cookies, visitor_data))
    }

    async fn extract_po_token_with_browser(&self, account: &YouTubeAccount) -> Result<String> {
        // Use browser automation to extract PO token
        
        warn!("PO Token extraction would require browser automation");
        warn!("This is a placeholder - implement with headless browser");
        
        // Return dummy token for now
        Ok("dummy_po_token".to_string())
    }

    fn save_cookies_to_temp_file(&self, account: &YouTubeAccount) -> Result<String> {
        if let Some(cookies) = &account.cookies {
            let temp_file = std::env::temp_dir().join(format!("yt_cookies_{}.txt", account.email.replace("@", "_")));
            std::fs::write(&temp_file, cookies)?;
            Ok(temp_file.to_string_lossy().to_string())
        } else {
            Err(anyhow!("No cookies available for account"))
        }
    }

    fn cleanup_accounts(&mut self) {
        let now = chrono::Utc::now();
        
        self.accounts.retain(|account| {
            // Remove banned accounts
            if account.is_banned {
                info!("Removing banned account: {}", account.email);
                return false;
            }

            // Remove accounts that are too old
            let account_age = now.signed_duration_since(account.created_at);
            if account_age.num_hours() > self.account_rotation_hours as i64 {
                info!("Removing old account: {} (age: {} hours)", account.email, account_age.num_hours());
                return false;
            }

            // Remove accounts that have been used too much
            if account.usage_count >= self.max_usage_per_account {
                info!("Removing overused account: {} (usage: {})", account.email, account.usage_count);
                return false;
            }

            true
        });
    }

    fn find_usable_account(&mut self) -> Option<&mut YouTubeAccount> {
        let now = chrono::Utc::now();
        
        self.accounts
            .iter_mut()
            .find(|account| {
                !account.is_banned && 
                account.usage_count < self.max_usage_per_account &&
                account.last_used.map_or(true, |last_used| {
                    now.signed_duration_since(last_used).num_minutes() > 5 // 5 minute cooldown
                })
            })
    }

    /// Mark account as used
    pub fn mark_account_used(&mut self, email: &str) {
        if let Some(account) = self.accounts.iter_mut().find(|a| a.email == email) {
            account.usage_count += 1;
            account.last_used = Some(chrono::Utc::now());
        }
    }

    /// Mark account as banned
    pub fn mark_account_banned(&mut self, email: &str) {
        if let Some(account) = self.accounts.iter_mut().find(|a| a.email == email) {
            account.is_banned = true;
            error!("Marked account as banned: {}", email);
        }
    }
}
