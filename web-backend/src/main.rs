mod handlers;
mod security;
mod state;
mod websocket;

use axum::{
    http::{header, Method},
    middleware,
    routing::{get, post},
    Router,
};
use state::AppState;
use std::net::SocketAddr;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    let state = AppState::new().await?;

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-csrf-token"),
        ])
        .allow_credentials(false)
        .max_age(Duration::from_secs(300));

    let app = Router::new()
        .route("/api/info", post(handlers::get_video_info))
        .route("/api/quality", post(handlers::get_quality_options))
        .route("/api/convert", post(handlers::start_conversion))
        .route("/api/playlist", post(handlers::convert_playlist))
        .route("/api/tasks", get(handlers::get_all_tasks))
        .route("/api/tasks/{id}", get(handlers::get_task))
        .route(
            "/api/tasks/{id}",
            axum::routing::delete(handlers::cancel_task),
        )
        .route("/api/download/{id}", get(handlers::download_file))
        .route(
            "/api/download/{task_id}/{file_index}",
            get(handlers::download_playlist_file),
        )
        .route("/api/health", get(handlers::health_check))
        .route("/api/dependencies", get(handlers::dependency_check))
        .route(
            "/api/clear-completed",
            post(handlers::clear_completed_tasks),
        )
        .route("/api/clear-all", post(handlers::clear_all_tasks))
        .route("/health", get(handlers::health_check))
        .route("/ws", get(websocket::websocket_handler))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(
                    security::headers::security_headers_middleware,
                ))
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        )
        .fallback_service(ServeDir::new("./dist").fallback(ServeFile::new("./dist/index.html")))
        .with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr = if host == "0.0.0.0" {
        SocketAddr::from(([0, 0, 0, 0], port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], port))
    };

    info!("Rustify web server listening on http://{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
