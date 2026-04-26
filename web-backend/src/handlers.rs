use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rustify_core::{
    format_duration, is_supported_playlist_url, is_valid_spotify_playlist_url,
    is_valid_youtube_url, sanitize_filename, ConversionProgress, OutputFormat,
    QualityOptions as CoreQualityOptions,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tracing::info;
use uuid::Uuid;

use crate::state::{AppState, TaskResponse, TaskStatus, TaskUpdate};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoInfoRequest {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversionRequest {
    pub url: String,
    pub format: String,
    pub quality: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistRequest {
    pub url: String,
    pub format: String,
    pub quality: String,
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
    pub uploader: Option<String>,
    pub view_count: Option<u64>,
    pub video_count: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityOptions {
    pub audio: Vec<FormatOption>,
    pub video: Vec<FormatOption>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormatOption {
    pub label: String,
    pub codec: String,
    pub detail: String,
    pub filesize: Option<u64>,
}

pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "rustify-web-backend",
        "version": "0.2.0"
    }))
}

pub async fn dependency_check(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.rustify.dependency_status().await)
}

pub async fn get_video_info(
    State(state): State<AppState>,
    Json(request): Json<VideoInfoRequest>,
) -> Result<Json<VideoInfo>, Response> {
    ensure_info_url(&request.url)?;

    if is_playlist_url(&request.url) {
        let playlist = state
            .rustify
            .get_playlist_info(&request.url)
            .await
            .map_err(bad_request)?;
        return Ok(Json(VideoInfo {
            title: playlist.title,
            duration: None,
            thumbnail: None,
            uploader: Some(playlist.uploader),
            view_count: None,
            video_count: Some(playlist.video_count),
        }));
    }

    let info = state
        .rustify
        .get_video_info(&request.url)
        .await
        .map_err(bad_request)?;

    Ok(Json(VideoInfo {
        title: info.title,
        duration: Some(format_duration(info.duration)),
        thumbnail: info
            .thumbnails
            .first()
            .map(|thumbnail| thumbnail.url.clone()),
        uploader: Some(info.uploader),
        view_count: info.view_count,
        video_count: None,
    }))
}

pub async fn get_quality_options(
    State(state): State<AppState>,
    Json(request): Json<VideoInfoRequest>,
) -> Result<Json<QualityOptions>, Response> {
    ensure_youtube_url(&request.url)?;
    let qualities = state
        .rustify
        .get_available_qualities(&request.url)
        .await
        .map_err(bad_request)?;

    Ok(Json(map_quality_options(&qualities)))
}

pub async fn start_conversion(
    State(state): State<AppState>,
    Json(request): Json<ConversionRequest>,
) -> Result<Json<TaskResponse>, Response> {
    ensure_youtube_url(&request.url)?;

    let task_id = Uuid::new_v4().to_string();
    let video_info = state
        .rustify
        .get_video_info(&request.url)
        .await
        .map_err(bad_request)?;
    let output_format = parse_output_format(&request.format, &request.quality)
        .map_err(|error| bad_request(anyhow::anyhow!(error)))?;
    let output_dir = state.downloads_dir.join(&task_id);
    let output_path = output_dir.join(format!(
        "{}.{}",
        sanitize_filename(&video_info.title),
        extension_for_format(&output_format)
    ));

    let task = TaskResponse {
        id: task_id.clone(),
        title: Some(video_info.title.clone()),
        url: request.url.clone(),
        format: request.format.clone(),
        quality: request.quality.clone(),
        status: "pending".to_string(),
        progress: 0.0,
        created_at: chrono::Utc::now(),
        output_path: Some(output_dir.to_string_lossy().to_string()),
        file_path: None,
        playlist_files: None,
    };

    {
        let mut tasks = state.tasks.lock().await;
        tasks.insert(task_id.clone(), task.clone());
    }
    let _ = state.task_updates.send(TaskUpdate {
        task_id: task_id.clone(),
        status: TaskStatus::Pending,
        progress: 0.0,
        speed: "Queued".to_string(),
        eta: "Starting".to_string(),
    });

    let state_clone = state.clone();
    let request_clone = request.clone();
    tokio::spawn(async move {
        update_task(&state_clone, &task_id, |task| {
            task.status = "processing".to_string();
            task.progress = 1.0;
        })
        .await;

        let rustify = Arc::clone(&state_clone.rustify);
        let result: anyhow::Result<()> = rustify
            .convert_video(
                &request_clone.url,
                output_path.clone(),
                output_format,
                &request_clone.quality,
                {
                    let state = state_clone.clone();
                    let task_id = task_id.clone();
                    move |progress| {
                        let state = state.clone();
                        let task_id = task_id.clone();
                        tokio::spawn(async move {
                            apply_progress_update(&state, &task_id, &progress).await;
                        });
                    }
                },
            )
            .await;

        match result {
            Ok(()) => {
                update_task(&state_clone, &task_id, |task| {
                    task.status = "completed".to_string();
                    task.progress = 100.0;
                    task.file_path = Some(output_path.to_string_lossy().to_string());
                })
                .await;
                let _ = state_clone.task_updates.send(TaskUpdate {
                    task_id,
                    status: TaskStatus::Completed,
                    progress: 100.0,
                    speed: "Complete".to_string(),
                    eta: "Done".to_string(),
                });
            }
            Err(error) => {
                update_task(&state_clone, &task_id, |task| {
                    task.status = format!("failed: {}", error);
                })
                .await;
                let _ = state_clone.task_updates.send(TaskUpdate {
                    task_id,
                    status: TaskStatus::Failed(error.to_string()),
                    progress: 0.0,
                    speed: "Failed".to_string(),
                    eta: "Error".to_string(),
                });
            }
        }
    });

    Ok(Json(task))
}

pub async fn convert_playlist(
    State(state): State<AppState>,
    Json(request): Json<PlaylistRequest>,
) -> Result<Json<TaskResponse>, Response> {
    ensure_playlist_url(&request.url)?;

    let task_id = Uuid::new_v4().to_string();
    let playlist_info = state
        .rustify
        .get_playlist_info(&request.url)
        .await
        .map_err(bad_request)?;
    let output_format = parse_output_format(&request.format, &request.quality)
        .map_err(|error| bad_request(anyhow::anyhow!(error)))?;
    let output_dir = state.downloads_dir.join(format!("playlist-{task_id}"));

    let task = TaskResponse {
        id: task_id.clone(),
        title: Some(playlist_info.title.clone()),
        url: request.url.clone(),
        format: request.format.clone(),
        quality: request.quality.clone(),
        status: "pending".to_string(),
        progress: 0.0,
        created_at: chrono::Utc::now(),
        output_path: Some(output_dir.to_string_lossy().to_string()),
        file_path: None,
        playlist_files: None,
    };

    {
        let mut tasks = state.tasks.lock().await;
        tasks.insert(task_id.clone(), task.clone());
    }
    let _ = state.task_updates.send(TaskUpdate {
        task_id: task_id.clone(),
        status: TaskStatus::Pending,
        progress: 0.0,
        speed: "Queued".to_string(),
        eta: "Starting".to_string(),
    });

    let state_clone = state.clone();
    let request_clone = request.clone();
    let total_videos = playlist_info.video_count.max(1) as f64;
    tokio::spawn(async move {
        update_task(&state_clone, &task_id, |task| {
            task.status = "processing".to_string();
            task.progress = 1.0;
        })
        .await;

        let rustify = Arc::clone(&state_clone.rustify);
        let output_dir_for_collect = output_dir.clone();
        let result: anyhow::Result<Vec<anyhow::Result<()>>> = rustify
            .convert_playlist(
                &request_clone.url,
                output_dir.clone(),
                output_format,
                &request_clone.quality,
                {
                    let state = state_clone.clone();
                    let task_id = task_id.clone();
                    move |index, progress| {
                        let overall = (((index as f64) + (progress.percentage / 100.0))
                            / total_videos)
                            * 100.0;
                        let state = state.clone();
                        let task_id = task_id.clone();
                        tokio::spawn(async move {
                            update_task(&state, &task_id, |task| {
                                task.progress = overall.clamp(0.0, 99.0);
                            })
                            .await;
                            let _ = state.task_updates.send(TaskUpdate {
                                task_id,
                                status: TaskStatus::Converting,
                                progress: overall.clamp(0.0, 99.0),
                                speed: progress.speed.clone(),
                                eta: progress.eta.clone(),
                            });
                        });
                    }
                },
            )
            .await;

        match result {
            Ok(results) => {
                let failed = results.iter().filter(|result| result.is_err()).count();
                let files = collect_playlist_files(&output_dir_for_collect)
                    .await
                    .unwrap_or_default();
                update_task(&state_clone, &task_id, |task| {
                    task.status = if failed == 0 {
                        "completed".to_string()
                    } else {
                        format!("completed_with_errors: {} failed", failed)
                    };
                    task.progress = 100.0;
                    task.file_path = files.first().cloned();
                    task.playlist_files = Some(files.clone());
                })
                .await;
                let _ = state_clone.task_updates.send(TaskUpdate {
                    task_id,
                    status: TaskStatus::Completed,
                    progress: 100.0,
                    speed: "Complete".to_string(),
                    eta: "Done".to_string(),
                });
            }
            Err(error) => {
                update_task(&state_clone, &task_id, |task| {
                    task.status = format!("failed: {}", error);
                })
                .await;
                let _ = state_clone.task_updates.send(TaskUpdate {
                    task_id,
                    status: TaskStatus::Failed(error.to_string()),
                    progress: 0.0,
                    speed: "Failed".to_string(),
                    eta: "Error".to_string(),
                });
            }
        }
    });

    Ok(Json(task))
}

pub async fn get_all_tasks(State(state): State<AppState>) -> impl IntoResponse {
    let tasks = state.tasks.lock().await;
    let task_list: Vec<TaskResponse> = tasks.values().cloned().collect();
    Json(task_list)
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>, Response> {
    let tasks = state.tasks.lock().await;
    match tasks.get(&id) {
        Some(task) => Ok(Json(task.clone())),
        None => Err(not_found("Task not found")),
    }
}

pub async fn cancel_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    let mut tasks = state.tasks.lock().await;
    match tasks.get_mut(&id) {
        Some(task) => {
            task.status = "cancelled".to_string();
            let _ = state.task_updates.send(TaskUpdate {
                task_id: id.clone(),
                status: TaskStatus::Cancelled,
                progress: task.progress,
                speed: "Cancelled".to_string(),
                eta: "Stopped".to_string(),
            });
            Ok(Json(serde_json::json!({"message": "Task cancelled"})))
        }
        None => Err(not_found("Task not found")),
    }
}

pub async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, Response> {
    let file_path = {
        let tasks = state.tasks.lock().await;
        let Some(task) = tasks.get(&id) else {
            return Err(not_found("Task not found"));
        };
        let Some(file_path) = task.file_path.clone() else {
            return Err(not_found("File not ready for download"));
        };
        file_path
    };

    serve_download_file(&file_path).await
}

pub async fn download_playlist_file(
    State(state): State<AppState>,
    Path((task_id, file_index)): Path<(String, usize)>,
) -> Result<Response, Response> {
    let file_path = {
        let tasks = state.tasks.lock().await;
        let Some(task) = tasks.get(&task_id) else {
            return Err(not_found("Task not found"));
        };

        if let Some(playlist_files) = &task.playlist_files {
            if let Some(file_path) = playlist_files.get(file_index) {
                file_path.clone()
            } else {
                return Err(not_found("File index out of range"));
            }
        } else if let Some(file_path) = &task.file_path {
            file_path.clone()
        } else {
            return Err(not_found("File not ready for download"));
        }
    };

    serve_download_file(&file_path).await
}

pub async fn clear_completed_tasks(State(state): State<AppState>) -> impl IntoResponse {
    info!("Clearing completed tasks");

    let mut files_to_delete = Vec::new();
    {
        let mut tasks = state.tasks.lock().await;
        tasks.retain(|_, task| {
            let should_delete = task.status == "completed"
                || task.status.starts_with("failed")
                || task.status.starts_with("completed_with_errors")
                || task.status == "cancelled";

            if should_delete {
                if let Some(file_path) = &task.file_path {
                    files_to_delete.push(file_path.clone());
                }
                if let Some(playlist_files) = &task.playlist_files {
                    files_to_delete.extend(playlist_files.clone());
                }
            }

            !should_delete
        });
    }

    for file_path in files_to_delete {
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    let _ = cleanup_empty_directories(&state.downloads_dir).await;

    Json(serde_json::json!({
        "message": "Completed tasks cleared successfully",
        "status": "success"
    }))
}

pub async fn clear_all_tasks(State(state): State<AppState>) -> impl IntoResponse {
    info!("Clearing all tasks");

    let mut files_to_delete = Vec::new();
    {
        let mut tasks = state.tasks.lock().await;
        for task in tasks.values() {
            if let Some(file_path) = &task.file_path {
                files_to_delete.push(file_path.clone());
            }
            if let Some(playlist_files) = &task.playlist_files {
                files_to_delete.extend(playlist_files.clone());
            }
        }
        tasks.clear();
    }

    for file_path in files_to_delete {
        let _ = tokio::fs::remove_file(&file_path).await;
    }
    let _ = cleanup_all_directories(&state.downloads_dir).await;

    Json(serde_json::json!({
        "message": "All tasks cleared successfully",
        "status": "success"
    }))
}

fn map_quality_options(qualities: &CoreQualityOptions) -> QualityOptions {
    QualityOptions {
        audio: qualities
            .audio_qualities
            .iter()
            .map(|quality| FormatOption {
                label: format!("{} kbps {}", quality.bitrate, quality.format),
                codec: quality.codec.clone(),
                detail: quality
                    .sample_rate
                    .map(|sample_rate| format!("{sample_rate} Hz"))
                    .unwrap_or_else(|| "source rate".to_string()),
                filesize: quality.file_size,
            })
            .collect(),
        video: qualities
            .video_qualities
            .iter()
            .map(|quality| FormatOption {
                label: quality.resolution.clone(),
                codec: quality.codec.clone(),
                detail: quality
                    .fps
                    .map(|fps| format!("{fps:.0} fps"))
                    .unwrap_or_else(|| "source fps".to_string()),
                filesize: quality.file_size,
            })
            .collect(),
    }
}

fn parse_output_format(format: &str, quality: &str) -> Result<OutputFormat, String> {
    match format.trim().to_ascii_lowercase().as_str() {
        "mp3" => Ok(OutputFormat::Mp3 {
            bitrate: parse_audio_bitrate(quality, 320)?,
        }),
        "flac" => Ok(OutputFormat::Flac),
        "wav" => Ok(OutputFormat::Wav),
        "aac" => Ok(OutputFormat::Aac {
            bitrate: parse_audio_bitrate(quality, 256)?,
        }),
        "ogg" => Ok(OutputFormat::Ogg {
            quality: quality.parse::<u8>().unwrap_or(6),
        }),
        "mp4" => Ok(OutputFormat::Mp4 {
            resolution: quality.to_string(),
        }),
        "webm" => Ok(OutputFormat::WebM {
            resolution: quality.to_string(),
        }),
        other => Err(format!("Unsupported format: {other}")),
    }
}

fn parse_audio_bitrate(value: &str, default: u32) -> Result<u32, String> {
    let digits = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return Ok(default);
    }

    digits
        .parse::<u32>()
        .map_err(|error| format!("Invalid bitrate '{value}': {error}"))
}

fn extension_for_format(format: &OutputFormat) -> &'static str {
    match format {
        OutputFormat::Mp3 { .. } => "mp3",
        OutputFormat::Mp4 { .. } => "mp4",
        OutputFormat::Flac => "flac",
        OutputFormat::Wav => "wav",
        OutputFormat::Aac { .. } => "aac",
        OutputFormat::Ogg { .. } => "ogg",
        OutputFormat::WebM { .. } => "webm",
    }
}

#[allow(clippy::result_large_err)]
fn ensure_youtube_url(url: &str) -> Result<(), Response> {
    if is_valid_youtube_url(url) {
        Ok(())
    } else {
        Err(bad_request(anyhow::anyhow!("Invalid YouTube URL: {url}")))
    }
}

#[allow(clippy::result_large_err)]
fn ensure_playlist_url(url: &str) -> Result<(), Response> {
    if is_supported_playlist_url(url) {
        Ok(())
    } else {
        Err(bad_request(anyhow::anyhow!(
            "Invalid playlist URL: {url}. Rustify supports YouTube and Spotify playlist links."
        )))
    }
}

#[allow(clippy::result_large_err)]
fn ensure_info_url(url: &str) -> Result<(), Response> {
    if is_valid_youtube_url(url) || is_valid_spotify_playlist_url(url) {
        Ok(())
    } else {
        Err(bad_request(anyhow::anyhow!(
            "Invalid URL: {url}. Rustify info supports YouTube videos and YouTube / Spotify playlists."
        )))
    }
}

fn is_playlist_url(url: &str) -> bool {
    url.contains("playlist?list=") || is_valid_spotify_playlist_url(url)
}

fn bad_request(error: impl std::fmt::Display) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}

fn not_found(message: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
        .into_response()
}

async fn apply_progress_update(state: &AppState, task_id: &str, progress: &ConversionProgress) {
    update_task(state, task_id, |task| {
        task.progress = progress.percentage;
        task.status = "processing".to_string();
    })
    .await;
    let _ = state.task_updates.send(TaskUpdate {
        task_id: task_id.to_string(),
        status: TaskStatus::Converting,
        progress: progress.percentage,
        speed: progress.speed.clone(),
        eta: progress.eta.clone(),
    });
}

async fn update_task(state: &AppState, task_id: &str, mut update: impl FnMut(&mut TaskResponse)) {
    let mut tasks = state.tasks.lock().await;
    if let Some(task) = tasks.get_mut(task_id) {
        update(task);
    }
}

async fn serve_download_file(file_path: &str) -> Result<Response, Response> {
    if tokio::fs::metadata(file_path).await.is_err() {
        return Err(not_found("File not found on disk"));
    }

    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");

    let file = File::open(file_path)
        .await
        .map_err(|_| not_found("Failed to open file"))?;
    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    Ok((
        StatusCode::OK,
        [
            ("Content-Type", content_type_for_filename(filename)),
            (
                "Content-Disposition",
                &format!("attachment; filename=\"{}\"", filename),
            ),
            ("Cache-Control", "no-cache"),
        ],
        body,
    )
        .into_response())
}

fn content_type_for_filename(filename: &str) -> &'static str {
    if filename.ends_with(".mp3") {
        "audio/mpeg"
    } else if filename.ends_with(".flac") {
        "audio/flac"
    } else if filename.ends_with(".wav") {
        "audio/wav"
    } else if filename.ends_with(".aac") {
        "audio/aac"
    } else if filename.ends_with(".ogg") {
        "audio/ogg"
    } else if filename.ends_with(".mp4") {
        "video/mp4"
    } else if filename.ends_with(".webm") {
        "video/webm"
    } else {
        "application/octet-stream"
    }
}

async fn collect_playlist_files(output_dir: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let mut files = Vec::new();
    let mut entries = tokio::fs::read_dir(output_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            files.push(path.to_string_lossy().to_string());
        }
    }
    files.sort();
    Ok(files)
}

async fn cleanup_empty_directories(
    downloads_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !downloads_dir.exists() {
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(downloads_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let mut dir_entries = tokio::fs::read_dir(&path).await?;
            if dir_entries.next_entry().await?.is_none() {
                let _ = tokio::fs::remove_dir(&path).await;
            }
        }
    }

    Ok(())
}

async fn cleanup_all_directories(
    downloads_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !downloads_dir.exists() {
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(downloads_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let _ = tokio::fs::remove_dir_all(&path).await;
        }
    }

    Ok(())
}
