// CSRF Protection - OWASP A01: Broken Access Control Prevention
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use base64::{Engine as _, engine::general_purpose};
use crate::security::{log_security_event, SecurityEventType, SecuritySeverity};

#[derive(Clone)]
#[allow(dead_code)]
pub struct CsrfProtection {
    tokens: Arc<Mutex<HashMap<String, CsrfToken>>>,
    secret: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CsrfToken {
    token: String,
    expires_at: Instant,
    ip_address: String,
}

#[derive(Serialize, Deserialize)]
pub struct CsrfTokenResponse {
    pub csrf_token: String,
    pub expires_in: u64,
}

#[allow(dead_code)]
impl CsrfProtection {
    pub fn new(secret: String) -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
            secret,
        }
    }

    pub fn generate_token(&self, ip: &str) -> String {
        use sha2::{Sha256, Digest};
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let nonce: u64 = rng.gen();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut hasher = Sha256::new();
        hasher.update(self.secret.as_bytes());
        hasher.update(ip.as_bytes());
        hasher.update(&nonce.to_be_bytes());
        hasher.update(&timestamp.to_be_bytes());

        let hash = hasher.finalize();
        let token = general_purpose::STANDARD.encode(&hash);

        // Store token with expiration
        let csrf_token = CsrfToken {
            token: token.clone(),
            expires_at: Instant::now() + Duration::from_secs(3600), // 1 hour
            ip_address: ip.to_string(),
        };

        let mut tokens = self.tokens.lock().unwrap();
        tokens.insert(token.clone(), csrf_token);

        // Cleanup expired tokens
        self.cleanup_expired_tokens(&mut tokens);

        token
    }

    pub fn validate_token(&self, token: &str, ip: &str) -> bool {
        let mut tokens = self.tokens.lock().unwrap();
        
        if let Some(csrf_token) = tokens.get(token) {
            // Check if token is expired
            if Instant::now() > csrf_token.expires_at {
                tokens.remove(token);
                return false;
            }

            // Check if IP matches (prevent token theft)
            if csrf_token.ip_address != ip {
                let ip_clone = ip.to_string();
                let token_ip = csrf_token.ip_address.clone();
                tokio::spawn(async move {
                    log_security_event(
                        &ip_clone,
                        None,
                        SecurityEventType::CSRFAttempt,
                        &format!("CSRF token IP mismatch: token IP {} vs request IP {}", token_ip, ip_clone),
                        SecuritySeverity::High,
                    ).await;
                });
                return false;
            }

            // Token is valid, remove it (one-time use)
            tokens.remove(token);
            true
        } else {
            let ip_clone = ip.to_string();
            tokio::spawn(async move {
                log_security_event(
                    &ip_clone,
                    None,
                    SecurityEventType::CSRFAttempt,
                    "Invalid CSRF token provided",
                    SecuritySeverity::Medium,
                ).await;
            });
            false
        }
    }

    fn cleanup_expired_tokens(&self, tokens: &mut HashMap<String, CsrfToken>) {
        let now = Instant::now();
        tokens.retain(|_, token| now <= token.expires_at);
    }
}

// CSRF middleware for state-changing operations
#[allow(dead_code)]
pub async fn csrf_middleware(
    headers: HeaderMap,
    State(csrf): State<CsrfProtection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method().clone();
    
    // Only check CSRF for state-changing methods
    if matches!(method.as_str(), "POST" | "PUT" | "DELETE" | "PATCH") {
        let ip = extract_ip_from_request(&request);
        
        // Get CSRF token from header
        let token = headers
            .get("x-csrf-token")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                let ip_clone = ip.clone();
                tokio::spawn(async move {
                    log_security_event(
                        &ip_clone,
                        None,
                        SecurityEventType::CSRFAttempt,
                        "Missing CSRF token in request",
                        SecuritySeverity::Medium,
                    ).await;
                });
                StatusCode::FORBIDDEN
            })?;

        // Validate CSRF token
        if !csrf.validate_token(token, &ip) {
            let ip_clone = ip.clone();
            tokio::spawn(async move {
                log_security_event(
                    &ip_clone,
                    None,
                    SecurityEventType::CSRFAttempt,
                    "Invalid CSRF token validation failed",
                    SecuritySeverity::High,
                ).await;
            });
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(next.run(request).await)
}

#[allow(dead_code)]
fn extract_ip_from_request(request: &Request) -> String {
    // Try to get real IP from headers (for reverse proxy setups)
    if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // Fallback to connection info (this would need to be passed through somehow)
    "unknown".to_string()
}

// Double Submit Cookie pattern (alternative CSRF protection)
#[derive(Clone)]
#[allow(dead_code)]
pub struct DoubleSubmitCookie {
    secret: String,
}

#[allow(dead_code)]
impl DoubleSubmitCookie {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    pub fn generate_cookie_token(&self) -> String {
        use sha2::{Sha256, Digest};
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let nonce: u64 = rng.gen();
        
        let mut hasher = Sha256::new();
        hasher.update(self.secret.as_bytes());
        hasher.update(nonce.to_be_bytes());
        
        general_purpose::STANDARD.encode(hasher.finalize())
    }

    pub fn validate_double_submit(&self, cookie_token: &str, header_token: &str) -> bool {
        // Both tokens must match and be valid
        !cookie_token.is_empty() && 
        !header_token.is_empty() && 
        cookie_token == header_token &&
        self.is_valid_token_format(cookie_token)
    }

    fn is_valid_token_format(&self, token: &str) -> bool {
        // Check if token is base64 and reasonable length
        general_purpose::STANDARD.decode(token).is_ok() && token.len() >= 32 && token.len() <= 128
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_token_generation() {
        let csrf = CsrfProtection::new("test_secret".to_string());
        let token1 = csrf.generate_token("127.0.0.1");
        let token2 = csrf.generate_token("127.0.0.1");
        
        // Tokens should be different
        assert_ne!(token1, token2);
        assert!(!token1.is_empty());
        assert!(!token2.is_empty());
    }

    #[tokio::test]
    async fn test_csrf_token_validation() {
        let csrf = CsrfProtection::new("test_secret".to_string());
        let token = csrf.generate_token("127.0.0.1");
        
        // Valid token should validate
        assert!(csrf.validate_token(&token, "127.0.0.1"));
        
        // Token should be one-time use
        assert!(!csrf.validate_token(&token, "127.0.0.1"));
    }

    #[test]
    fn test_double_submit_cookie() {
        let dsc = DoubleSubmitCookie::new("test_secret".to_string());
        let token = dsc.generate_cookie_token();
        
        assert!(dsc.validate_double_submit(&token, &token));
        assert!(!dsc.validate_double_submit(&token, "different_token"));
        assert!(!dsc.validate_double_submit("", &token));
    }
}
