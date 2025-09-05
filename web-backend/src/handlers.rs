use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::info;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::state::{AppState, TaskResponse, TaskStatus, TaskUpdate};
use crate::youtube::ConversionOptions;

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
    State(state): State<AppState>,
    Json(request): Json<VideoInfoRequest>,
) -> Result<Json<VideoInfo>, Response> {
    info!("Getting video info for URL: {}", request.url);
    
    // Check if it's a playlist URL
    if request.url.contains("playlist?list=") {
        match state.youtube_downloader.get_playlist_info(&request.url).await {
            Ok(playlist_info) => {
                return Ok(Json(VideoInfo {
                    title: format!("{} (Playlist - {} videos)", playlist_info.title, playlist_info.video_count),
                    duration: Some(format!("{} videos", playlist_info.video_count)),
                    thumbnail: None,
                    channel: playlist_info.uploader,
                }));
            }
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Failed to get playlist info: {}", e),
                    }),
                ).into_response());
            }
        }
    }
    
    // Single video
    match state.youtube_downloader.get_video_info(&request.url).await {
        Ok(video_info) => {
            Ok(Json(VideoInfo {
                title: video_info.title,
                duration: video_info.duration,
                thumbnail: video_info.thumbnail,
                channel: video_info.channel,
            }))
        }
        Err(e) => {
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Failed to get video info: {}", e),
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
    match state.youtube_downloader.get_available_formats(&request.url).await {
        Ok(formats) => {
            let format_options: Vec<FormatOption> = formats.into_iter().map(|f| FormatOption {
                format_id: f.format_id,
                format: f.ext,
                quality: f.resolution.unwrap_or_else(|| "unknown".to_string()),
                filesize: f.filesize,
            }).collect();
            
            Ok(Json(QualityOptions {
                formats: format_options,
            }))
        }
        Err(_) => {
            // Fallback to default options
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
    }
}

// Start video conversion
pub async fn start_conversion(
    State(state): State<AppState>,
    Json(request): Json<ConversionRequest>,
) -> Result<Json<TaskResponse>, Response> {
    let task_id = Uuid::new_v4().to_string();
    
    let task = TaskResponse {
        id: task_id.clone(),
        url: request.url.clone(),
        format: request.format.clone(),
        quality: request.quality.clone(),
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

    // Start conversion in background
    let state_clone = state.clone();
    let task_id_clone = task_id.clone();
    let request_clone = request.clone();
    
    tokio::spawn(async move {
        let conversion_options = ConversionOptions {
            url: request_clone.url,
            format: request_clone.format,
            quality: request_clone.quality,
            output_dir: format!("{}/{}", state_clone.downloads_dir, task_id_clone),
        };

        // Update task status to converting
        {
            let mut tasks = state_clone.tasks.lock().await;
            if let Some(task) = tasks.get_mut(&task_id_clone) {
                task.status = "processing".to_string();
                task.progress = 10.0;
            }
        }

        // Broadcast update
        let _ = state_clone.task_updates.send(TaskUpdate {
            task_id: task_id_clone.clone(),
            status: TaskStatus::Converting,
            progress: 10.0,
            speed: "Starting...".to_string(),
            eta: "Calculating...".to_string(),
        });

        // Perform actual download
        match state_clone.youtube_downloader.download_video(conversion_options).await {
            Ok(file_path) => {
                // Update task as completed
                {
                    let mut tasks = state_clone.tasks.lock().await;
                    if let Some(task) = tasks.get_mut(&task_id_clone) {
                        task.status = "completed".to_string();
                        task.progress = 100.0;
                        task.file_path = Some(file_path.clone());
                    }
                }

                // Broadcast completion
                let _ = state_clone.task_updates.send(TaskUpdate {
                    task_id: task_id_clone,
                    status: TaskStatus::Completed,
                    progress: 100.0,
                    speed: "Complete".to_string(),
                    eta: "Done".to_string(),
                });
            }
            Err(e) => {
                // Update task as failed
                {
                    let mut tasks = state_clone.tasks.lock().await;
                    if let Some(task) = tasks.get_mut(&task_id_clone) {
                        task.status = format!("failed: {}", e);
                        task.progress = 0.0;
                    }
                }

                // Broadcast failure
                let _ = state_clone.task_updates.send(TaskUpdate {
                    task_id: task_id_clone,
                    status: TaskStatus::Failed(e.to_string()),
                    progress: 0.0,
                    speed: "Failed".to_string(),
                    eta: "Error".to_string(),
                });
            }
        }
    });
    
    Ok(Json(task))
}

// Convert playlist
pub async fn convert_playlist(
    State(state): State<AppState>,
    Json(request): Json<PlaylistRequest>,
) -> Result<Json<Vec<TaskResponse>>, Response> {
    // First get playlist info
    let playlist_info = match state.youtube_downloader.get_playlist_info(&request.url).await {
        Ok(info) => info,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Failed to get playlist info: {}", e),
                }),
            ).into_response());
        }
    };

    let mut tasks = Vec::new();
    
    // Create a task for each video in the playlist
    for (index, video) in playlist_info.videos.iter().enumerate() {
        let task_id = Uuid::new_v4().to_string();
        let video_url = format!("https://www.youtube.com/watch?v={}", video.id);
        
        let task = TaskResponse {
            id: task_id.clone(),
            url: video_url.clone(),
            format: request.format.clone(),
            quality: request.quality.clone(),
            status: "pending".to_string(),
            progress: 0.0,
            created_at: chrono::Utc::now(),
            output_path: None,
            file_path: None,
        };

        // Store task
        {
            let mut task_store = state.tasks.lock().await;
            task_store.insert(task_id.clone(), task.clone());
        }

        tasks.push(task.clone());

        // Start conversion in background for each video
        let state_clone = state.clone();
        let task_id_clone = task_id.clone();
        let request_clone = request.clone();
        let video_title = video.title.clone();
        
        tokio::spawn(async move {
            let conversion_options = ConversionOptions {
                url: video_url,
                format: request_clone.format,
                quality: request_clone.quality,
                output_dir: format!("{}/playlist_{}_{}", state_clone.downloads_dir, index, task_id_clone),
            };

            // Update task status to converting
            {
                let mut tasks = state_clone.tasks.lock().await;
                if let Some(task) = tasks.get_mut(&task_id_clone) {
                    task.status = "processing".to_string();
                    task.progress = 10.0;
                }
            }

            // Broadcast update
            let _ = state_clone.task_updates.send(TaskUpdate {
                task_id: task_id_clone.clone(),
                status: TaskStatus::Converting,
                progress: 10.0,
                speed: format!("Starting {}", video_title),
                eta: "Calculating...".to_string(),
            });

            // Perform actual download
            match state_clone.youtube_downloader.download_video(conversion_options).await {
                Ok(file_path) => {
                    // Update task as completed
                    {
                        let mut tasks = state_clone.tasks.lock().await;
                        if let Some(task) = tasks.get_mut(&task_id_clone) {
                            task.status = "completed".to_string();
                            task.progress = 100.0;
                            task.file_path = Some(file_path.clone());
                        }
                    }

                    // Broadcast completion
                    let _ = state_clone.task_updates.send(TaskUpdate {
                        task_id: task_id_clone,
                        status: TaskStatus::Completed,
                        progress: 100.0,
                        speed: "Complete".to_string(),
                        eta: "Done".to_string(),
                    });
                }
                Err(e) => {
                    // Update task as failed
                    {
                        let mut tasks = state_clone.tasks.lock().await;
                        if let Some(task) = tasks.get_mut(&task_id_clone) {
                            task.status = format!("failed: {}", e);
                            task.progress = 0.0;
                        }
                    }

                    // Broadcast failure
                    let _ = state_clone.task_updates.send(TaskUpdate {
                        task_id: task_id_clone,
                        status: TaskStatus::Failed(format!("Video {}: {}", video_title, e)),
                        progress: 0.0,
                        speed: "Failed".to_string(),
                        eta: "Error".to_string(),
                    });
                }
            }
        });
    }
    
    Ok(Json(tasks))
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
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, Response> {
    // Get task to find file path
    let file_path = {
        let tasks = state.tasks.lock().await;
        match tasks.get(&id) {
            Some(task) => match &task.file_path {
                Some(path) => path.clone(),
                None => {
                    return Err((
                        StatusCode::NOT_FOUND,
                        Json(ErrorResponse {
                            error: "File not ready for download".to_string(),
                        }),
                    ).into_response());
                }
            },
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "Task not found".to_string(),
                    }),
                ).into_response());
            }
        }
    };

    // Check if file exists
    if !tokio::fs::metadata(&file_path).await.is_ok() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "File not found on disk".to_string(),
            }),
        ).into_response());
    }

    // Get filename for Content-Disposition header
    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");

    // Open file
    let file = match File::open(&file_path).await {
        Ok(file) => file,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to open file".to_string(),
                }),
            ).into_response());
        }
    };

    // Create stream
    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    // Determine content type based on file extension
    let content_type = if filename.ends_with(".mp3") {
        "audio/mpeg"
    } else if filename.ends_with(".wav") {
        "audio/wav"
    } else if filename.ends_with(".mp4") {
        "video/mp4"
    } else if filename.ends_with(".webm") {
        "video/webm"
    } else {
        "application/octet-stream"
    };

    Ok((
        StatusCode::OK,
        [
            ("Content-Type", content_type),
            ("Content-Disposition", &format!("attachment; filename=\"{}\"", filename)),
            ("Cache-Control", "no-cache"),
        ],
        body,
    ).into_response())
}
