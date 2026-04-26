use rustify_core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{State, Window};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionTask {
    pub id: String,
    pub url: String,
    pub title: String,
    pub output_path: PathBuf,
    pub format: String,
    pub quality: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub speed: String,
    pub eta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Downloading,
    Converting,
    Completed,
    Failed(String),
    Cancelled,
}

pub struct AppState {
    pub rustify: Arc<EzP3>,
    pub tasks: Arc<Mutex<HashMap<String, ConversionTask>>>,
    pub default_output_dir: PathBuf,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            rustify: Arc::new(EzP3::new().expect("Failed to initialize Rustify")),
            tasks: Arc::new(Mutex::new(HashMap::new())),
            default_output_dir: dirs::download_dir()
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from(".")),
        }
    }
}

#[tauri::command]
pub async fn get_video_info(url: String, state: State<'_, AppState>) -> Result<VideoInfo, String> {
    state
        .rustify
        .get_video_info(&url)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_playlist_info(
    url: String,
    state: State<'_, AppState>,
) -> Result<PlaylistInfo, String> {
    state
        .rustify
        .get_playlist_info(&url)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_quality_options(
    url: String,
    state: State<'_, AppState>,
) -> Result<QualityOptions, String> {
    state
        .rustify
        .get_available_qualities(&url)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn convert_video(
    url: String,
    output_dir: Option<String>,
    format: String,
    quality: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if !is_valid_youtube_url(&url) {
        return Err(format!("Invalid YouTube URL: {url}"));
    }

    let task_id = Uuid::new_v4().to_string();
    let video_info = state
        .rustify
        .get_video_info(&url)
        .await
        .map_err(|error| error.to_string())?;
    let output_format = parse_output_format(&format, &quality)?;
    let output_path = build_output_path(
        output_dir.as_deref(),
        &state.default_output_dir,
        &video_info.title,
        &output_format,
    );

    let task = ConversionTask {
        id: task_id.clone(),
        url: url.clone(),
        title: video_info.title.clone(),
        output_path: output_path.clone(),
        format: format.clone(),
        quality: quality.clone(),
        status: TaskStatus::Pending,
        progress: 0.0,
        speed: "Queued".to_string(),
        eta: "Starting".to_string(),
    };

    {
        let mut tasks = state.tasks.lock().unwrap();
        tasks.insert(task_id.clone(), task.clone());
    }
    window.emit("task_update", &task).map_err(|error| error.to_string())?;

    let rustify = Arc::clone(&state.rustify);
    let tasks = Arc::clone(&state.tasks);
    let window_handle = window.clone();
    let task_id_clone = task_id.clone();
    let quality_clone = quality.clone();
    let url_clone = url.clone();

    tokio::spawn(async move {
        update_task(&tasks, &task_id_clone, |task| {
            task.status = TaskStatus::Converting;
        });

        let result = rustify
            .convert_video(
                &url_clone,
                output_path.clone(),
                output_format,
                &quality_clone,
                {
                    let tasks = Arc::clone(&tasks);
                    let task_id = task_id_clone.clone();
                    let window = window_handle.clone();
                    move |progress| {
                        update_task(&tasks, &task_id, |task| {
                            task.progress = progress.percentage as f32;
                            task.speed = progress.speed.clone();
                            task.eta = progress.eta.clone();
                        });

                        emit_task_update(&tasks, &window, &task_id);
                    }
                },
            )
            .await;

        update_task(&tasks, &task_id_clone, |task| match result {
            Ok(_) => {
                task.status = TaskStatus::Completed;
                task.progress = 100.0;
                task.speed = "Complete".to_string();
                task.eta = "Done".to_string();
            }
            Err(ref error) => {
                task.status = TaskStatus::Failed(error.to_string());
                task.speed = "Failed".to_string();
                task.eta = "Error".to_string();
            }
        });
        emit_task_update(&tasks, &window_handle, &task_id_clone);
    });

    Ok(task_id)
}

#[tauri::command]
pub async fn convert_playlist(
    url: String,
    output_dir: Option<String>,
    format: String,
    quality: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if !is_supported_playlist_url(&url) {
        return Err(format!(
            "Invalid playlist URL: {url}. Rustify supports YouTube and Spotify playlist links."
        ));
    }

    let batch_id = Uuid::new_v4().to_string();
    let playlist_info = state
        .rustify
        .get_playlist_info(&url)
        .await
        .map_err(|error| error.to_string())?;
    let output_format = parse_output_format(&format, &quality)?;
    let output_root = output_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| state.default_output_dir.join(sanitize_filename(&playlist_info.title)));

    let task = ConversionTask {
        id: batch_id.clone(),
        url: url.clone(),
        title: playlist_info.title.clone(),
        output_path: output_root.clone(),
        format: format.clone(),
        quality: quality.clone(),
        status: TaskStatus::Pending,
        progress: 0.0,
        speed: "Queued".to_string(),
        eta: "Starting".to_string(),
    };

    {
        let mut tasks = state.tasks.lock().unwrap();
        tasks.insert(batch_id.clone(), task.clone());
    }
    window.emit("task_update", &task).map_err(|error| error.to_string())?;

    let rustify = Arc::clone(&state.rustify);
    let tasks = Arc::clone(&state.tasks);
    let window_handle = window.clone();
    let batch_id_clone = batch_id.clone();
    let quality_clone = quality.clone();
    let url_clone = url.clone();
    let total_videos = playlist_info.video_count.max(1) as f64;

    tokio::spawn(async move {
        update_task(&tasks, &batch_id_clone, |task| {
            task.status = TaskStatus::Converting;
        });
        emit_task_update(&tasks, &window_handle, &batch_id_clone);

        let result = rustify
            .convert_playlist(
                &url_clone,
                output_root,
                output_format,
                &quality_clone,
                {
                    let tasks = Arc::clone(&tasks);
                    let task_id = batch_id_clone.clone();
                    let window = window_handle.clone();
                    move |index, progress| {
                        let overall = (((index as f64) + (progress.percentage / 100.0)) / total_videos)
                            * 100.0;
                        update_task(&tasks, &task_id, |task| {
                            task.progress = overall.clamp(0.0, 100.0) as f32;
                            task.speed = progress.speed.clone();
                            task.eta = progress.eta.clone();
                        });
                        emit_task_update(&tasks, &window, &task_id);
                    }
                },
            )
            .await;

        let (status, progress, speed, eta) = match result {
            Ok(results) => {
                let failed = results.iter().filter(|entry| entry.is_err()).count();
                if failed == 0 {
                    (
                        TaskStatus::Completed,
                        100.0,
                        "Complete".to_string(),
                        "Done".to_string(),
                    )
                } else {
                    (
                        TaskStatus::Failed(format!(
                            "{failed} item(s) failed during playlist conversion"
                        )),
                        100.0,
                        format!("Completed with {failed} failure(s)"),
                        "Done".to_string(),
                    )
                }
            }
            Err(error) => (
                TaskStatus::Failed(error.to_string()),
                0.0,
                "Failed".to_string(),
                "Error".to_string(),
            ),
        };

        update_task(&tasks, &batch_id_clone, |task| {
            task.status = status.clone();
            task.progress = progress;
            task.speed = speed.clone();
            task.eta = eta.clone();
        });
        emit_task_update(&tasks, &window_handle, &batch_id_clone);
    });

    Ok(batch_id)
}

#[tauri::command]
pub fn get_conversion_progress(state: State<'_, AppState>) -> Result<Vec<ConversionTask>, String> {
    let tasks = state.tasks.lock().unwrap();
    Ok(tasks.values().cloned().collect())
}

#[tauri::command]
pub fn cancel_conversion(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    update_task(&state.tasks, &task_id, |task| {
        task.status = TaskStatus::Cancelled;
        task.speed = "Cancelled".to_string();
        task.eta = "Stopped".to_string();
    });
    Ok(())
}

#[tauri::command]
pub fn clear_completed_tasks(state: State<'_, AppState>) -> Result<(), String> {
    let mut tasks = state.tasks.lock().unwrap();
    tasks.retain(|_, task| {
        !matches!(
            task.status,
            TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Cancelled
        )
    });
    Ok(())
}

#[tauri::command]
pub async fn select_output_directory() -> Result<Option<String>, String> {
    use tauri::api::dialog::blocking::FileDialogBuilder;

    let path = FileDialogBuilder::new()
        .set_title("Select Output Directory")
        .pick_folder();

    Ok(path.map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn get_default_output_directory(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.default_output_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn validate_youtube_url(url: String) -> Result<bool, String> {
    Ok(is_valid_youtube_url(&url) || is_valid_spotify_playlist_url(&url))
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

fn build_output_path(
    requested_dir: Option<&str>,
    default_dir: &PathBuf,
    title: &str,
    format: &OutputFormat,
) -> PathBuf {
    let directory = requested_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| default_dir.clone());
    directory.join(format!(
        "{}.{}",
        sanitize_filename(title),
        extension_for_format(format)
    ))
}

fn update_task(
    tasks: &Arc<Mutex<HashMap<String, ConversionTask>>>,
    task_id: &str,
    mut update: impl FnMut(&mut ConversionTask),
) {
    let mut guard = tasks.lock().unwrap();
    if let Some(task) = guard.get_mut(task_id) {
        update(task);
    }
}

fn emit_task_update(
    tasks: &Arc<Mutex<HashMap<String, ConversionTask>>>,
    window: &Window,
    task_id: &str,
) {
    if let Some(task) = tasks.lock().unwrap().get(task_id).cloned() {
        let _ = window.emit("task_update", task);
    }
}
