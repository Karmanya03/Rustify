mod handlers;
mod websocket;
mod state;
mod security;
mod youtube;

use axum::{
    http::{header, Method},
    routing::{get, post},
    Router,
    middleware,
};
use state::AppState;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{info, Level};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    // Initialize application state
    let state = AppState::new().await?;

    // Flexible CORS configuration for development and production
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,       header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-csrf-token")
        ])
        .allow_credentials(false) // Set to false for broader compatibility
        .max_age(Duration::from_secs(300));

    // Build the application routes with security middleware
    let app = Router::new()
        // API routes
        .route("/api/info", post(handlers::get_video_info))
        .route("/api/quality", post(handlers::get_quality_options))
        .route("/api/convert", post(handlers::start_conversion))
        .route("/api/playlist", post(handlers::convert_playlist))
        .route("/api/tasks", get(handlers::get_all_tasks))
        .route("/api/tasks/:id", get(handlers::get_task))
        .route("/api/tasks/:id", axum::routing::delete(handlers::cancel_task))
        .route("/api/download/:id", get(handlers::download_file))
        .route("/api/download/:task_id/:file_index", get(handlers::download_playlist_file))
        .route("/api/health", get(handlers::health_check))
        .route("/api/dependencies", get(handlers::dependency_check))
        .route("/health", get(handlers::health_check))
        
        // WebSocket for real-time updates
        .route("/ws", get(websocket::websocket_handler))
        
        // Serve static files (frontend)
        .nest_service("/", ServeDir::new("./dist")
            .fallback(ServeFile::new("./dist/index.html")))
        
        // Apply security middleware layers
        .layer(
            ServiceBuilder::new()
                // Security headers
                .layer(middleware::from_fn(security::headers::security_headers_middleware))
                // Request tracing
                .layer(TraceLayer::new_for_http())
                // CORS (after security headers)
                .layer(cors)
        )
        .with_state(state);

    // Start the server with dynamic port for hosting platforms
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
    
    // Use 127.0.0.1 for local development, 0.0.0.0 for production deployment (Render.com, etc.)
    let host = std::env::var("HOST")
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    
    let addr = if host == "0.0.0.0" {
        SocketAddr::from(([0, 0, 0, 0], port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], port))
    };
    
    info!("🚀 Rustify Web Server starting on http://{}:{}", host, port);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
