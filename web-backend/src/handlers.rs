use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rustify_core::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use tracing::{info, warn, error};

use crate::state::{AppState, TaskResponse};
use crate::security::validation::{validate_youtube_url, validate_format, validate_quality, sanitize_filename, sanitize_html};

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
    State(state): State<AppState>,
    Json(request): Json<VideoInfoRequest>,
) -> Result<Json<VideoInfo>, Response> {
    // Validate input
    if let Err(e) = validate_youtube_url(&request.url) {
        warn!("Invalid URL provided: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid URL: {}", e),
            }),
        ).into_response());
    }

    info!("Getting video info for URL: {}", request.url);

    match state.ezp3.get_video_info(&request.url).await {
        Ok(info) => Ok(Json(info)),
        Err(e) => {
            error!("Failed to get video info: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: sanitize_html(&e.to_string()),
                }),
            ).into_response())
        }
    }
}

// Get quality options for a video
pub async fn get_quality_options(
    State(state): State<AppState>,
    Json(request): Json<VideoInfoRequest>,
) -> Result<Json<QualityOptions>, Response> {
    match state.ezp3.get_available_qualities(&request.url).await {
        Ok(options) => Ok(Json(options)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response()),
    }
}

// Start video conversion
pub async fn start_conversion(
    State(state): State<AppState>,
    Json(request): Json<ConversionRequest>,
) -> Result<Json<TaskResponse>, Response> {
    // Validate input
    if let Err(e) = validate_youtube_url(&request.url) {
        warn!("Invalid URL provided for conversion: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid URL: {}", e),
            }),
        ).into_response());
    }

    if let Err(e) = validate_format(&request.format) {
        warn!("Invalid format provided: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid format: {}", e),
            }),
        ).into_response());
    }

    if let Err(e) = validate_quality(&request.quality) {
        warn!("Invalid quality provided: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid quality: {}", e),
            }),
        ).into_response());
    }

    let task_id = Uuid::new_v4().to_string();
    
    info!("Starting conversion for URL: {} with format: {} and quality: {}", request.url, request.format, request.quality);
    
    // Get video info for task creation
    let video_info = match state.ezp3.get_video_info(&request.url).await {
        Ok(info) => info,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response())
        }
    };
    
    // Create task response
    let task = TaskResponse {
        id: task_id.clone(),
        url: request.url.clone(),
        title: video_info.title.clone(),
        format: request.format.clone(),
        quality: request.quality.clone(),
        status: "pending".to_string(),
        progress: 0.0,
        output_path: request.output_path.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        file_path: None,
    };
    
    // Store task in state
    {
        let mut tasks = state.tasks.lock().await;
        tasks.insert(task_id.clone(), task.clone());
    }
    
    // Start conversion in background
    let state_clone = state.clone();
    let request_clone = request.clone();
    let task_id_clone = task_id.clone();
    
    tokio::spawn(async move {
        let downloads_dir = PathBuf::from("./downloads");
        if !downloads_dir.exists() {
            let _ = std::fs::create_dir_all(&downloads_dir);
        }
        
        let filename = format!("{}.{}", 
            sanitize_filename(&video_info.title),
            match request_clone.format.as_str() {
                "mp3" => "mp3",
                "wav" => match request_clone.quality.as_str() {
                    "lossless" | "hd" => "flac", // Use FLAC for lossless audio
                    _ => "wav"
                },
                "mp4" => "mp4",
                "webm" => "webm",
                _ => "mp3"
            }
        );
        
        let output_path = downloads_dir.join(filename);
        
        let format = match request_clone.format.as_str() {
            "mp4" => OutputFormat::Mp4 { resolution: request_clone.quality.clone() },
            "mp3" => {
                let bitrate = match request_clone.quality.as_str() {
                    "320" => 320, // Apple Music quality - no compression
                    "256" => 256, // High quality
                    "192" => 192, // Standard quality
                    "128" => 128, // Basic quality
                    "96" => 96,   // Low quality
                    _ => 320,     // Default to highest quality
                };
                OutputFormat::Mp3 { bitrate }
            },
            "wav" => {
                match request_clone.quality.as_str() {
                    "lossless" => OutputFormat::Flac, // Use FLAC for lossless
                    "hd" => OutputFormat::Flac,       // Use FLAC for HD audio
                    _ => OutputFormat::Flac,          // Default to lossless
                }
            },
            "webm" => OutputFormat::WebM { resolution: request_clone.quality.clone() },
            _ => OutputFormat::Mp3 { bitrate: 320 }, // Default to highest quality MP3
        };
        
        let progress_callback = {
            let task_id = task_id_clone.clone();
            let state = state_clone.clone();
            move |progress: ConversionProgress| {
                let task_id = task_id.clone();
                let state = state.clone();
                let rt = tokio::runtime::Handle::current();
                rt.spawn(async move {
                    let mut tasks = state.tasks.lock().await;
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.progress = progress.percentage as f32;
                        task.status = "processing".to_string();
                    }
                });
            }
        };
        
        let result = state_clone
            .ezp3
            .convert_video(
                &request_clone.url,
                output_path.clone(),
                format,
                &request_clone.quality,
                progress_callback,
            )
            .await;
        
        // Update task status
        let mut tasks = state_clone.tasks.lock().await;
        if let Some(task) = tasks.get_mut(&task_id_clone) {
            match result {
                Ok(_) => {
                    task.status = "completed".to_string();
                    task.progress = 100.0;
                    task.output_path = Some(output_path.to_string_lossy().to_string());
                    task.file_path = Some(output_path.to_string_lossy().to_string());
                }
                Err(e) => {
                    task.status = format!("failed: {}", e);
                    task.progress = 0.0;
                }
            }
        }
    });
    
    Ok(Json(task))
}

// Convert playlist
pub async fn convert_playlist(
    State(_state): State<AppState>,
    Json(_request): Json<PlaylistRequest>,
) -> Result<Json<Vec<TaskResponse>>, Response> {
    // For now, return empty array as placeholder
    // TODO: Implement playlist conversion
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
        )
            .into_response()),
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
        )
            .into_response()),
    }
}

// Download file
pub async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, Response> {
    let tasks = state.tasks.lock().await;
    let task = tasks.get(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Task not found".to_string(),
            }),
        )
            .into_response()
    })?;
    
    if task.status != "completed" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Task is not completed".to_string(),
            }),
        )
            .into_response());
    }
    
    let file_path = task.output_path.as_ref().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Output file path not available".to_string(),
            }),
        )
            .into_response()
    })?;
    
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "File not found on disk".to_string(),
            }),
        )
            .into_response());
    }
    
    let file_content = match tokio::fs::read(path).await {
        Ok(content) => content,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to read file".to_string(),
                }),
            )
                .into_response())
        }
    };
    
    let filename = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");
    
    let content_type = match path.extension().and_then(|ext| ext.to_str()) {
        Some("mp3") => "audio/mpeg",
        Some("mp4") => "video/mp4",
        Some("wav") => "audio/wav",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    };
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Content-Disposition", format!("attachment; filename=\"{}\"", filename))
        .header("Content-Length", file_content.len())
        .body(axum::body::Body::from(file_content))
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create response".to_string(),
                }),
            )
                .into_response()
        })?;
    
    Ok(response)
}
