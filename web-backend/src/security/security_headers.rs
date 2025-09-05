// Security Headers - OWASP A05: Security Misconfiguration Prevention
use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};

// Security headers middleware
pub async fn security_headers_middleware<B>(
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // Add security headers
    add_security_headers(headers);
    
    response
}

pub fn add_security_headers(headers: &mut HeaderMap) {
    // Prevent XSS attacks (A03: Injection)
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );

    // Prevent clickjacking (A05: Security Misconfiguration)
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );

    // XSS Protection
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );

    // Referrer Policy - prevent information leakage
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Permissions Policy - restrict browser features
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(
            "camera=(), microphone=(), geolocation=(), payment=(), usb=(), magnetometer=(), gyroscope=(), speaker=()"
        ),
    );

    // Content Security Policy - prevent XSS and data injection
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; \
             font-src 'self'; \
             connect-src 'self' ws: wss:; \
             media-src 'none'; \
             object-src 'none'; \
             base-uri 'self'; \
             form-action 'self'; \
             frame-ancestors 'none'; \
             upgrade-insecure-requests"
        ),
    );

    // HTTP Strict Transport Security (HSTS) - enforce HTTPS
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );

    // Expect-CT - Certificate Transparency
    headers.insert(
        HeaderName::from_static("expect-ct"),
        HeaderValue::from_static("max-age=86400, enforce"),
    );

    // Remove potentially revealing headers
    headers.remove("server");
    headers.remove("x-powered-by");
    
    // Add custom security identifier (optional)
    headers.insert(
        HeaderName::from_static("x-security-policy"),
        HeaderValue::from_static("enforced"),
    );

    // Cache control for sensitive content
    headers.insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static("no-cache, no-store, must-revalidate, private"),
    );

    headers.insert(
        HeaderName::from_static("pragma"),
        HeaderValue::from_static("no-cache"),
    );

    headers.insert(
        HeaderName::from_static("expires"),
        HeaderValue::from_static("0"),
    );
}

// CORS configuration for security
pub fn configure_cors() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{CorsLayer, Any};
    use axum::http::Method;

    CorsLayer::new()
        .allow_origin("http://localhost:3001".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            HeaderName::from_static("x-csrf-token"),
        ])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(300)) // 5 minutes
}

// Request size limiter
pub fn request_size_limit() -> tower_http::limit::RequestBodyLimitLayer {
    tower_http::limit::RequestBodyLimitLayer::new(1024 * 1024) // 1MB limit
}

// Compression with security considerations
pub fn compression_layer() -> tower_http::compression::CompressionLayer {
    use tower_http::compression::CompressionLayer;
    // Be careful with compression to avoid CRIME/BREACH attacks
    CompressionLayer::new()
}

// Request timeout
pub fn timeout_layer() -> tower::timeout::TimeoutLayer {
    tower::timeout::TimeoutLayer::new(std::time::Duration::from_secs(30))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_security_headers() {
        let mut headers = HeaderMap::new();
        add_security_headers(&mut headers);

        assert!(headers.contains_key("x-content-type-options"));
        assert!(headers.contains_key("x-frame-options"));
        assert!(headers.contains_key("content-security-policy"));
        assert!(headers.contains_key("strict-transport-security"));
        
        // Ensure revealing headers are removed
        assert!(!headers.contains_key("server"));
        assert!(!headers.contains_key("x-powered-by"));
    }
}
