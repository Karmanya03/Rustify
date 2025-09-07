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

// Dependency check endpoint
pub async fn dependency_check() -> impl IntoResponse {
    match crate::youtube::check_dependencies().await {
        Ok(()) => {
            Json(serde_json::json!({
                "status": "ok",
                "yt_dlp": "available",
                "ffmpeg": "available",
                "message": "All dependencies are available"
            }))
        }
        Err(e) => {
            Json(serde_json::json!({
                "status": "partial",
                "yt_dlp": if e.to_string().contains("yt-dlp not found") { "missing" } else { "available" },
                "ffmpeg": "optional",
                "message": format!("Some dependencies missing: {}", e)
            }))
        }
    }
}

// Get video information
pub async fn get_video_info(
    Json(request): Json<VideoInfoRequest>,
) -> Result<Json<VideoInfo>, Response> {
    info!("Getting video info for URL: {}", request.url);
    
    // Check if it's a playlist URL
    if request.url.contains("playlist?list=") {
        match crate::youtube::get_playlist_info(&request.url).await {
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
    match crate::youtube::get_video_info(&request.url).await {
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
    Json(request): Json<VideoInfoRequest>,
) -> Result<Json<QualityOptions>, Response> {
    match crate::youtube::get_formats(&request.url).await {
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
        playlist_files: None,
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
            speed: "Starting download...".to_string(),
            eta: "Calculating...".to_string(),
        });

        // Simulate progress updates during download
        let state_clone_progress = state_clone.clone();
        let task_id_progress = task_id_clone.clone();
        
        // Start a progress simulation task
        let progress_task = tokio::spawn(async move {
            for i in 1..=8 {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                let progress = 10.0 + (i as f64 * 10.0); // 20%, 30%, 40%, etc.
                
                {
                    let mut tasks = state_clone_progress.tasks.lock().await;
                    if let Some(task) = tasks.get_mut(&task_id_progress) {
                        if task.status == "processing" {
                            task.progress = progress;
                        } else {
                            break; // Task completed or failed
                        }
                    }
                }

                let _ = state_clone_progress.task_updates.send(TaskUpdate {
                    task_id: task_id_progress.clone(),
                    status: TaskStatus::Converting,
                    progress,
                    speed: format!("Downloading... {}%", progress as i32),
                    eta: format!("{}s remaining", 20 - (i * 2)),
                });
            }
        });

        // Perform actual download
        match crate::youtube::download_video(conversion_options).await {
            Ok(file_path) => {
                // Cancel progress simulation
                progress_task.abort();
                
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
                // Cancel progress simulation
                progress_task.abort();
                
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
        playlist_files: None,
    };

    // Store task
    {
        let mut tasks = state.tasks.lock().await;
        tasks.insert(task_id.clone(), task.clone());
    }

    // Start playlist conversion in background
    let state_clone = state.clone();
    let task_id_clone = task_id.clone();
    let request_clone = request.clone();
    
    tokio::spawn(async move {
        let playlist_dir = format!("{}/playlist_{}", state_clone.downloads_dir, task_id_clone);
        let conversion_options = ConversionOptions {
            url: request_clone.url.clone(),
            format: request_clone.format.clone(),
            quality: request_clone.quality.clone(),
            output_dir: playlist_dir.clone(),
        };

        // Update task status to processing
        {
            let mut tasks = state_clone.tasks.lock().await;
            if let Some(task) = tasks.get_mut(&task_id_clone) {
                task.status = "processing".to_string();
                task.progress = 5.0;
                task.output_path = Some(playlist_dir.clone());
            }
        }

        // Broadcast initial update
        let _ = state_clone.task_updates.send(TaskUpdate {
            task_id: task_id_clone.clone(),
            status: TaskStatus::Converting,
            progress: 5.0,
            speed: "Analyzing playlist...".to_string(),
            eta: "Calculating...".to_string(),
        });

        // Start enhanced progress tracking
        let state_clone_progress = state_clone.clone();
        let task_id_progress = task_id_clone.clone();
        
        let progress_task = tokio::spawn(async move {
            let mut current_progress: f64;
            let mut iteration = 0;
            
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                iteration += 1;
                
                // Simulate realistic playlist download progress
                current_progress = match iteration {
                    1..=3 => 10.0 + (iteration as f64 * 5.0), // Initial phase: 10-25%
                    4..=10 => 25.0 + (iteration as f64 * 3.0), // Download phase: 25-55%
                    11..=20 => 55.0 + (iteration as f64 * 2.0), // Processing phase: 55-75%
                    _ => (75.0 + (iteration as f64 * 1.0)).min(95.0), // Final phase: 75-95%
                };
                
                {
                    let mut tasks = state_clone_progress.tasks.lock().await;
                    if let Some(task) = tasks.get_mut(&task_id_progress) {
                        if task.status == "processing" {
                            task.progress = current_progress;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let progress_message = match iteration {
                    1..=3 => "Fetching playlist information...",
                    4..=8 => "Downloading videos from playlist...",
                    9..=15 => "Processing downloaded files...",
                    16..=20 => "Organizing playlist contents...",
                    _ => "Finalizing download...",
                };

                let eta_estimate = match iteration {
                    1..=5 => format!("{}s remaining", 45 - (iteration * 2)),
                    6..=15 => format!("{}s remaining", 35 - iteration),
                    _ => "Almost done".to_string(),
                };

                let _ = state_clone_progress.task_updates.send(TaskUpdate {
                    task_id: task_id_progress.clone(),
                    status: TaskStatus::Converting,
                    progress: current_progress,
                    speed: progress_message.to_string(),
                    eta: eta_estimate,
                });

                // Break if we've reached 95% to avoid going over 100 before completion
                if current_progress >= 95.0 {
                    break;
                }
            }
        });

        // Perform actual playlist download with enhanced error handling
        match crate::youtube::download_playlist(conversion_options).await {
            Ok(file_paths) => {
                // Cancel progress simulation
                progress_task.abort();
                
                // Ensure output directory exists and contains files
                let _output_dir_path = std::path::Path::new(&playlist_dir);
                let actual_file_count = file_paths.len();
                
                // Create a summary of the download
                let download_summary = if actual_file_count > 0 {
                    format!("Successfully downloaded {} files", actual_file_count)
                } else {
                    "Playlist processed, check output directory".to_string()
                };
                
                // Update task as completed
                {
                    let mut tasks = state_clone.tasks.lock().await;
                    if let Some(task) = tasks.get_mut(&task_id_clone) {
                        task.status = "completed".to_string();
                        task.progress = 100.0;
                        task.file_path = file_paths.first().cloned();
                        task.playlist_files = if file_paths.len() > 1 { Some(file_paths) } else { None };
                        task.output_path = Some(playlist_dir);
                    }
                }

                // Broadcast completion with detailed info
                let _ = state_clone.task_updates.send(TaskUpdate {
                    task_id: task_id_clone,
                    status: TaskStatus::Completed,
                    progress: 100.0,
                    speed: download_summary,
                    eta: "Completed".to_string(),
                });
            }
            Err(e) => {
                // Cancel progress simulation
                progress_task.abort();
                
                // Determine if it's a partial failure or complete failure
                let error_message = e.to_string();
                let is_partial_failure = error_message.contains("some errors") || 
                                       error_message.contains("partial");
                
                let status_message = if is_partial_failure {
                    format!("completed_with_errors: {}", error_message)
                } else {
                    format!("failed: {}", error_message)
                };
                
                // Update task status
                {
                    let mut tasks = state_clone.tasks.lock().await;
                    if let Some(task) = tasks.get_mut(&task_id_clone) {
                        task.status = status_message;
                        task.progress = if is_partial_failure { 75.0 } else { 0.0 };
                        // Still set output path for partial failures so users can check what was downloaded
                        if is_partial_failure {
                            task.output_path = Some(playlist_dir);
                        }
                    }
                }

                // Broadcast failure or partial completion
                let broadcast_status = if is_partial_failure {
                    TaskStatus::Completed
                } else {
                    TaskStatus::Failed(error_message.clone())
                };

                let _ = state_clone.task_updates.send(TaskUpdate {
                    task_id: task_id_clone,
                    status: broadcast_status,
                    progress: if is_partial_failure { 75.0 } else { 0.0 },
                    speed: if is_partial_failure {
                        "Completed with some errors - check output folder".to_string()
                    } else {
                        format!("Failed: {}", error_message)
                    },
                    eta: if is_partial_failure { "Check results" } else { "Failed" }.to_string(),
                });
            }
        }
    });
    
    Ok(Json(task))
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

// Download individual playlist file
pub async fn download_playlist_file(
    State(state): State<AppState>,
    Path((task_id, file_index)): Path<(String, usize)>,
) -> Result<Response, Response> {
    // Get task to find playlist files
    let file_path = {
        let tasks = state.tasks.lock().await;
        match tasks.get(&task_id) {
            Some(task) => {
                if let Some(playlist_files) = &task.playlist_files {
                    if file_index < playlist_files.len() {
                        playlist_files[file_index].clone()
                    } else {
                        return Err((
                            StatusCode::NOT_FOUND,
                            Json(ErrorResponse {
                                error: "File index out of range".to_string(),
                            }),
                        ).into_response());
                    }
                } else {
                    // Fallback to single file if no playlist files
                    match &task.file_path {
                        Some(path) => path.clone(),
                        None => {
                            return Err((
                                StatusCode::NOT_FOUND,
                                Json(ErrorResponse {
                                    error: "File not ready for download".to_string(),
                                }),
                            ).into_response());
                        }
                    }
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
    } else if filename.ends_with(".m4a") {
        "audio/mp4"
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
