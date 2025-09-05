// OWASP Top 10 Security Implementation for Rustify
pub mod headers;
pub mod validation;
pub mod csrf;

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    CSRFAttempt,
    RateLimitExceeded,
    InvalidInput,
    SuspiciousActivity,
    AuthenticationFailure,
    AuthorizationFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: SystemTime,
    pub ip_address: String,
    pub user_id: Option<String>,
    pub event_type: SecurityEventType,
    pub description: String,
    pub severity: SecuritySeverity,
}

#[allow(dead_code)]
pub async fn log_security_event(
    ip: &str,
    user_id: Option<String>,
    event_type: SecurityEventType,
    description: &str,
    severity: SecuritySeverity,
) {
    let event = SecurityEvent {
        timestamp: SystemTime::now(),
        ip_address: ip.to_string(),
        user_id,
        event_type,
        description: description.to_string(),
        severity,
    };

    // Log to console (in production, this should go to a proper logging system)
    eprintln!("SECURITY EVENT: {:?}", event);
    
    // TODO: In production, send to SIEM/logging system
    // - Send to centralized logging
    // - Store in security events database
    // - Alert on high/critical severity events
}
