// Rate Limiting - OWASP A04: Insecure Design Prevention
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    extract::ConnectInfo,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::security::{log_security_event, SecurityEventType, SecuritySeverity};

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, ClientRequests>>>,
    max_requests_per_minute: u32,
    lockout_duration: Duration,
    max_failed_attempts: u32,
}

#[derive(Debug, Clone)]
struct ClientRequests {
    count: u32,
    window_start: Instant,
    failed_attempts: u32,
    locked_until: Option<Instant>,
}

impl RateLimiter {
    pub fn new(max_requests_per_minute: u32, max_failed_attempts: u32) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests_per_minute,
            lockout_duration: Duration::from_minutes(15),
            max_failed_attempts,
        }
    }

    pub fn check_rate_limit(&self, ip: &str) -> Result<(), StatusCode> {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        let client_requests = requests.entry(ip.to_string()).or_insert(ClientRequests {
            count: 0,
            window_start: now,
            failed_attempts: 0,
            locked_until: None,
        });

        // Check if client is locked out
        if let Some(locked_until) = client_requests.locked_until {
            if now < locked_until {
                tokio::spawn(log_security_event(
                    ip,
                    None,
                    SecurityEventType::RateLimitExceeded,
                    "Client still in lockout period",
                    SecuritySeverity::Medium,
                ));
                return Err(StatusCode::TOO_MANY_REQUESTS);
            } else {
                // Lockout period expired, reset
                client_requests.locked_until = None;
                client_requests.failed_attempts = 0;
            }
        }

        // Reset window if minute has passed
        if now.duration_since(client_requests.window_start) > Duration::from_secs(60) {
            client_requests.count = 0;
            client_requests.window_start = now;
        }

        // Check rate limit
        if client_requests.count >= self.max_requests_per_minute {
            client_requests.failed_attempts += 1;
            
            // Lock out client if too many failed attempts
            if client_requests.failed_attempts >= self.max_failed_attempts {
                client_requests.locked_until = Some(now + self.lockout_duration);
                tokio::spawn(log_security_event(
                    ip,
                    None,
                    SecurityEventType::RateLimitExceeded,
                    &format!("Client locked out after {} failed attempts", self.max_failed_attempts),
                    SecuritySeverity::High,
                ));
            }

            tokio::spawn(log_security_event(
                ip,
                None,
                SecurityEventType::RateLimitExceeded,
                &format!("Rate limit exceeded: {}/{} requests", client_requests.count, self.max_requests_per_minute),
                SecuritySeverity::Medium,
            ));

            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        client_requests.count += 1;
        Ok(())
    }

    pub fn record_successful_request(&self, ip: &str) {
        let mut requests = self.requests.lock().unwrap();
        if let Some(client_requests) = requests.get_mut(ip) {
            // Reset failed attempts on successful request
            client_requests.failed_attempts = 0;
        }
    }

    // Cleanup old entries
    pub fn cleanup_old_entries(&self) {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        let cutoff = Duration::from_minutes(10);

        requests.retain(|_, client_requests| {
            now.duration_since(client_requests.window_start) < cutoff
        });
    }
}

// Rate limiting middleware
pub async fn rate_limit_middleware<B>(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let ip = addr.ip().to_string();
    
    // Get rate limiter from app state (you'll need to add this to your app state)
    // For now, create a temporary one
    let rate_limiter = RateLimiter::new(30, 5); // 30 requests per minute, 5 failed attempts before lockout
    
    // Check rate limit
    rate_limiter.check_rate_limit(&ip)?;
    
    let response = next.run(request).await;
    
    // Record successful request
    rate_limiter.record_successful_request(&ip);
    
    Ok(response)
}

// Concurrent connection limiter
#[derive(Clone)]
pub struct ConnectionLimiter {
    active_connections: Arc<Mutex<HashMap<String, u32>>>,
    max_connections_per_ip: u32,
}

impl ConnectionLimiter {
    pub fn new(max_connections_per_ip: u32) -> Self {
        Self {
            active_connections: Arc::new(Mutex::new(HashMap::new())),
            max_connections_per_ip,
        }
    }

    pub fn can_connect(&self, ip: &str) -> bool {
        let connections = self.active_connections.lock().unwrap();
        let current_connections = connections.get(ip).unwrap_or(&0);
        *current_connections < self.max_connections_per_ip
    }

    pub fn add_connection(&self, ip: &str) -> Result<(), ()> {
        let mut connections = self.active_connections.lock().unwrap();
        let current = connections.entry(ip.to_string()).or_insert(0);
        
        if *current >= self.max_connections_per_ip {
            return Err(());
        }
        
        *current += 1;
        Ok(())
    }

    pub fn remove_connection(&self, ip: &str) {
        let mut connections = self.active_connections.lock().unwrap();
        if let Some(current) = connections.get_mut(ip) {
            if *current > 0 {
                *current -= 1;
            }
            if *current == 0 {
                connections.remove(ip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(2, 3);
        
        // First two requests should pass
        assert!(limiter.check_rate_limit("127.0.0.1").is_ok());
        assert!(limiter.check_rate_limit("127.0.0.1").is_ok());
        
        // Third request should fail
        assert!(limiter.check_rate_limit("127.0.0.1").is_err());
    }

    #[test]
    fn test_connection_limiter() {
        let limiter = ConnectionLimiter::new(2);
        
        assert!(limiter.add_connection("127.0.0.1").is_ok());
        assert!(limiter.add_connection("127.0.0.1").is_ok());
        assert!(limiter.add_connection("127.0.0.1").is_err());
        
        limiter.remove_connection("127.0.0.1");
        assert!(limiter.add_connection("127.0.0.1").is_ok());
    }
}
