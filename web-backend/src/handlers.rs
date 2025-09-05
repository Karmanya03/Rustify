use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{info, warn, error};

use crate::state::{AppState, TaskResponse};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoInfoRequest {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversionRequest {
    pub url: String,
    pub format: String,
    pub quality: String,
    pub output_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistRequest {
    pub url: String,
    pub format: String,
    pub quality: String,
    pub output_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub duration: Option<String>,
    pub thumbnail: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityOptions {
    pub formats: Vec<FormatOption>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormatOption {
    pub format_id: String,
    pub format: String,
    pub quality: String,
    pub filesize: Option<u64>,
}

// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "rustify-web-backend",
        "version": "0.1.0"
    }))
}

// Get video information
pub async fn get_video_info(
    State(_state): State<AppState>,
    Json(request): Json<VideoInfoRequest>,
) -> Result<Json<VideoInfo>, Response> {
    info!("Getting video info for URL: {}", request.url);
    
    // Mock response for now
    Ok(Json(VideoInfo {
        title: "Sample Video".to_string(),
        duration: Some("3:45".to_string()),
        thumbnail: Some("https://example.com/thumb.jpg".to_string()),
        channel: Some("Sample Channel".to_string()),
    }))
}

// Get quality options for a video
pub async fn get_quality_options(
    State(_state): State<AppState>,
    Json(_request): Json<VideoInfoRequest>,
) -> Result<Json<QualityOptions>, Response> {
    Ok(Json(QualityOptions {
        formats: vec![
            FormatOption {
                format_id: "mp4_720p".to_string(),
                format: "mp4".to_string(),
                quality: "720p".to_string(),
                filesize: Some(100_000_000),
            },
            FormatOption {
                format_id: "mp4_1080p".to_string(),
                format: "mp4".to_string(),
                quality: "1080p".to_string(),
                filesize: Some(200_000_000),
            },
        ],
    }))
}

// Start video conversion
pub async fn start_conversion(
    State(state): State<AppState>,
    Json(request): Json<ConversionRequest>,
) -> Result<Json<TaskResponse>, Response> {
    let task_id = Uuid::new_v4().to_string();
    
    let task = TaskResponse {
        id: task_id.clone(),
        url: request.url,
        format: request.format,
        quality: request.quality,
        status: "pending".to_string(),
        progress: 0.0,
        created_at: chrono::Utc::now(),
        output_path: None,
        file_path: None,
    };

    // Store task
    {
        let mut tasks = state.tasks.lock().await;
        tasks.insert(task_id.clone(), task.clone());
    }
    
    Ok(Json(task))
}

// Convert playlist
pub async fn convert_playlist(
    State(_state): State<AppState>,
    Json(_request): Json<PlaylistRequest>,
) -> Result<Json<Vec<TaskResponse>>, Response> {
    // For now, return empty array as placeholder
    Ok(Json(vec![]))
}

// Get all tasks
pub async fn get_all_tasks(State(state): State<AppState>) -> impl IntoResponse {
    let tasks = state.tasks.lock().await;
    let task_list: Vec<TaskResponse> = tasks.values().cloned().collect();
    Json(task_list)
}

// Get specific task
pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>, Response> {
    let tasks = state.tasks.lock().await;
    match tasks.get(&id) {
        Some(task) => Ok(Json(task.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Task not found".to_string(),
            }),
        ).into_response()),
    }
}

// Cancel task
pub async fn cancel_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    let mut tasks = state.tasks.lock().await;
    match tasks.get_mut(&id) {
        Some(task) => {
            task.status = "cancelled".to_string();
            Ok(Json(serde_json::json!({"message": "Task cancelled"})))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Task not found".to_string(),
            }),
        ).into_response()),
    }
}

// Download file
pub async fn download_file(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<Response, Response> {
    // Mock download - return empty file for now
    Ok((
        StatusCode::OK,
        [("Content-Type", "application/octet-stream")],
        "Mock file content".as_bytes(),
    ).into_response())
}
