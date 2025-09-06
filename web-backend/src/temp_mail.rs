use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct TempMailAccount {
    pub email: String,
    pub password: String,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TempMailResponse {
    mail: String,
    #[serde(rename = "sid_token")]
    sid_token: String,
}

#[derive(Debug, Deserialize)]
struct EmailMessage {
    id: String,
    from: String,
    subject: String,
    body: String,
    date: String,
}

pub struct TempMailService {
    client: Client,
    base_url: String,
}

impl TempMailService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://www.1secmail.com/api/v1/".to_string(),
        }
    }

    /// Generate a random temporary email address
    pub async fn generate_temp_email(&self) -> Result<TempMailAccount> {
        let response = self
            .client
            .get(&format!("{}?action=genRandomMailbox&count=1", self.base_url))
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to generate temp email: {}", response.status()));
        }

        let emails: Vec<String> = response.json().await?;
        
        if emails.is_empty() {
            return Err(anyhow!("No email addresses returned"));
        }

        let email = emails[0].clone();
        let password = self.generate_random_password();

        Ok(TempMailAccount {
            email,
            password,
            token: None,
        })
    }

    /// Check for new messages in the inbox
    pub async fn check_inbox(&self, email: &str) -> Result<Vec<EmailMessage>> {
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid email format"));
        }

        let (login, domain) = (parts[0], parts[1]);

        let response = self
            .client
            .get(&format!(
                "{}?action=getMessages&login={}&domain={}",
                self.base_url, login, domain
            ))
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to check inbox: {}", response.status()));
        }

        let messages: Vec<EmailMessage> = response.json().await?;
        Ok(messages)
    }

    /// Read a specific message
    pub async fn read_message(&self, email: &str, message_id: &str) -> Result<EmailMessage> {
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid email format"));
        }

        let (login, domain) = (parts[0], parts[1]);

        let response = self
            .client
            .get(&format!(
                "{}?action=readMessage&login={}&domain={}&id={}",
                self.base_url, login, domain, message_id
            ))
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to read message: {}", response.status()));
        }

        let message: EmailMessage = response.json().await?;
        Ok(message)
    }

    /// Wait for verification email from YouTube
    pub async fn wait_for_youtube_verification(&self, email: &str, timeout_minutes: u64) -> Result<String> {
        let timeout = Duration::from_secs(timeout_minutes * 60);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout {
            match self.check_inbox(email).await {
                Ok(messages) => {
                    for message in messages {
                        if message.from.contains("youtube") || 
                           message.from.contains("google") ||
                           message.subject.to_lowercase().contains("verify") ||
                           message.subject.to_lowercase().contains("confirm") {
                            
                            info!("Found YouTube verification email: {}", message.subject);
                            
                            // Read the full message to get verification link
                            if let Ok(full_message) = self.read_message(email, &message.id).await {
                                if let Some(verification_link) = self.extract_verification_link(&full_message.body) {
                                    return Ok(verification_link);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to check inbox: {}", e);
                }
            }

            // Wait 30 seconds before checking again
            tokio::time::sleep(Duration::from_secs(30)).await;
        }

        Err(anyhow!("Verification email not received within {} minutes", timeout_minutes))
    }

    fn extract_verification_link(&self, email_body: &str) -> Option<String> {
        // Look for common YouTube/Google verification link patterns
        let patterns = [
            r"https://accounts\.google\.com/[^\s]+",
            r"https://www\.youtube\.com/[^\s]+verify[^\s]*",
            r"https://[^\s]*google[^\s]*verify[^\s]*",
        ];

        for pattern in &patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.find(email_body) {
                    return Some(captures.as_str().to_string());
                }
            }
        }

        None
    }

    fn generate_random_password(&self) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                abcdefghijklmnopqrstuvwxyz\
                                0123456789!@#$%^&*";
        let mut rng = rand::thread_rng();

        (0..16)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

/// Alternative temp mail services as fallbacks
pub struct TempMailAlternatives {
    client: Client,
}

impl TempMailAlternatives {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Use temp-mail.org API (requires API key but more reliable)
    pub async fn get_temp_mail_org_account(&self) -> Result<TempMailAccount> {
        // This would require an API key from temp-mail.org
        // For now, we'll use the free 1secmail service
        
        let service = TempMailService::new();
        service.generate_temp_email().await
    }

    /// Use guerrillamail.com API
    pub async fn get_guerrilla_mail_account(&self) -> Result<TempMailAccount> {
        let response = self
            .client
            .get("https://api.guerrillamail.com/ajax.php?f=get_email_address")
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get guerrilla mail: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await?;
        
        if let Some(email) = json["email_addr"].as_str() {
            Ok(TempMailAccount {
                email: email.to_string(),
                password: self.generate_random_password(),
                token: json["sid_token"].as_str().map(|s| s.to_string()),
            })
        } else {
            Err(anyhow!("Invalid response from guerrilla mail"))
        }
    }

    fn generate_random_password(&self) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                abcdefghijklmnopqrstuvwxyz\
                                0123456789";
        let mut rng = rand::thread_rng();

        (0..12)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}
