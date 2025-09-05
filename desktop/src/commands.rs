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
    pub ezp3: Arc<EzP3>,
    pub tasks: Arc<Mutex<HashMap<String, ConversionTask>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ezp3: Arc::new(EzP3::new().expect("Failed to initialize EzP3")),
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tauri::command]
pub async fn get_video_info(url: String, state: State<'_, AppState>) -> Result<VideoInfo, String> {
    state.ezp3
        .get_video_info(&url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_quality_options(url: String, state: State<'_, AppState>) -> Result<QualityOptions, String> {
    state.ezp3
        .get_available_qualities(&url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn convert_video(
    url: String,
    output_path: String,
    format: String,
    quality: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();
    
    // Get video info first
    let video_info = state.ezp3
        .get_video_info(&url)
        .await
        .map_err(|e| e.to_string())?;
    
    // Create task
    let task = ConversionTask {
        id: task_id.clone(),
        url: url.clone(),
        title: video_info.title.clone(),
        output_path: PathBuf::from(&output_path),
        format: format.clone(),
        quality: quality.clone(),
        status: TaskStatus::Pending,
        progress: 0.0,
        speed: "0x".to_string(),
        eta: "Unknown".to_string(),
    };
    
    // Add task to state
    {
        let mut tasks = state.tasks.lock().unwrap();
        tasks.insert(task_id.clone(), task.clone());
    }
    
    // Emit task update
    window.emit("task_update", &task).map_err(|e| e.to_string())?;
    
    // Parse output format
    let output_format = match format.as_str() {
        "mp3" => {
            let bitrate = quality.replace("kbps", "").replace("k", "").parse().unwrap_or(256);
            OutputFormat::Mp3 { bitrate }
        }
        "mp4" => OutputFormat::Mp4 { resolution: quality.clone() },
        "flac" => OutputFormat::Flac,
        "aac" => {
            let bitrate = quality.replace("kbps", "").replace("k", "").parse().unwrap_or(256);
            OutputFormat::Aac { bitrate }
        }
        "ogg" => {
            let q = quality.parse().unwrap_or(5);
            OutputFormat::Ogg { quality: q }
        }
        "webm" => OutputFormat::WebM { resolution: quality.clone() },
        _ => return Err(format!("Unsupported format: {}", format)),
    };
    
    // Start conversion in background
    let ezp3 = Arc::clone(&state.ezp3);
    let tasks = Arc::clone(&state.tasks);
    let task_id_clone = task_id.clone();
    let window_clone = window.clone();
    
    tokio::spawn(async move {
        // Update status to converting
        {
            let mut tasks_guard = tasks.lock().unwrap();
            if let Some(task) = tasks_guard.get_mut(&task_id_clone) {
                task.status = TaskStatus::Converting;
            }
        }
        
        let result = ezp3.convert_video(
            &url,
            PathBuf::from(&output_path),
            output_format,
            &quality,
            {
                let tasks = Arc::clone(&tasks);
                let task_id = task_id_clone.clone();
                let window = window_clone.clone();
                
                move |progress| {
                    // Update task progress
                    {
                        let mut tasks_guard = tasks.lock().unwrap();
                        if let Some(task) = tasks_guard.get_mut(&task_id) {
                            task.progress = progress.percentage as f32;
                            task.speed = progress.speed.clone();
                            task.eta = progress.eta.clone();
                        }
                    }
                    
                    // Emit progress update
                    if let Ok(tasks_guard) = tasks.lock() {
                        if let Some(task) = tasks_guard.get(&task_id) {
                            let _ = window.emit("task_update", task);
                        }
                    }
                }
            }
        ).await;
        
        // Update final status
        {
            let mut tasks_guard = tasks.lock().unwrap();
            if let Some(task) = tasks_guard.get_mut(&task_id_clone) {
                match result {
                    Ok(_) => {
                        task.status = TaskStatus::Completed;
                        task.progress = 100.0;
                    }
                    Err(e) => {
                        task.status = TaskStatus::Failed(e.to_string());
                    }
                }
            }
        }
        
        // Emit final update
        if let Ok(tasks_guard) = tasks.lock() {
            if let Some(task) = tasks_guard.get(&task_id_clone) {
                let _ = window_clone.emit("task_update", task);
            }
        }
    });
    
    Ok(task_id)
}

#[tauri::command]
pub async fn convert_playlist(
    url: String,
    output_dir: String,
    format: String,
    quality: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let batch_id = Uuid::new_v4().to_string();
    let batch_id_return = batch_id.clone();
    
    // Clone values for closure
    let url_clone = url.clone();
    let quality_clone = quality.clone();
    let batch_id_clone = batch_id.clone();
    
    // Parse output format
    let output_format = match format.as_str() {
        "mp3" => {
            let bitrate = quality.replace("kbps", "").replace("k", "").parse().unwrap_or(256);
            OutputFormat::Mp3 { bitrate }
        }
        "mp4" => OutputFormat::Mp4 { resolution: quality.clone() },
        "flac" => OutputFormat::Flac,
        "aac" => {
            let bitrate = quality.replace("kbps", "").replace("k", "").parse().unwrap_or(256);
            OutputFormat::Aac { bitrate }
        }
        "ogg" => {
            let q = quality.parse().unwrap_or(5);
            OutputFormat::Ogg { quality: q }
        }
        "webm" => OutputFormat::WebM { resolution: quality.clone() },
        _ => return Err(format!("Unsupported format: {}", format)),
    };
    
    // Start playlist conversion in background
    let ezp3 = Arc::clone(&state.ezp3);
    let tasks = Arc::clone(&state.tasks);
    let window_clone = window.clone();
    
    tokio::spawn(async move {
        let result = ezp3.convert_playlist(
            &url_clone,
            PathBuf::from(&output_dir),
            output_format,
            &quality_clone,
            {
                let tasks = Arc::clone(&tasks);
                let window = window_clone.clone();
                let batch_id = batch_id_clone.clone();
                
                move |index, progress| {
                    // Create or update task for this video
                    let task_id = format!("{}_{}", batch_id, index);
                    
                    {
                        let mut tasks_guard = tasks.lock().unwrap();
                        if let Some(task) = tasks_guard.get_mut(&task_id) {
                            task.progress = progress.percentage as f32;
                            task.speed = progress.speed.clone();
                            task.eta = progress.eta.clone();
                        } else {
                            // Create new task if it doesn't exist
                            let task = ConversionTask {
                                id: task_id.clone(),
                                url: url.clone(),
                                title: format!("Video {}", index + 1),
                                output_path: PathBuf::from(&output_dir),
                                format: format.clone(),
                                quality: quality.clone(),
                                status: TaskStatus::Converting,
                                progress: progress.percentage as f32,
                                speed: progress.speed.clone(),
                                eta: progress.eta.clone(),
                            };
                            tasks_guard.insert(task_id.clone(), task);
                        }
                    }
                    
                    // Emit progress update
                    if let Ok(tasks_guard) = tasks.lock() {
                        if let Some(task) = tasks_guard.get(&task_id) {
                            let _ = window.emit("task_update", task);
                        }
                    }
                }
            }
        ).await;
        
        // Update final status for all tasks
        match result {
            Ok(results) => {
                for (index, result) in results.iter().enumerate() {
                    let task_id = format!("{}_{}", batch_id, index);
                    let mut tasks_guard = tasks.lock().unwrap();
                    if let Some(task) = tasks_guard.get_mut(&task_id) {
                        match result {
                            Ok(_) => {
                                task.status = TaskStatus::Completed;
                                task.progress = 100.0;
                            }
                            Err(e) => {
                                task.status = TaskStatus::Failed(e.to_string());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                // Mark all tasks as failed
                let mut tasks_guard = tasks.lock().unwrap();
                for (_, task) in tasks_guard.iter_mut() {
                    if task.id.starts_with(&batch_id) {
                        task.status = TaskStatus::Failed(e.to_string());
                    }
                }
            }
        }
        
        // Emit final updates
        if let Ok(tasks_guard) = tasks.lock() {
            for (_, task) in tasks_guard.iter() {
                if task.id.starts_with(&batch_id) {
                    let _ = window_clone.emit("task_update", task);
                }
            }
        }
    });
    
    Ok(batch_id_return)
}

#[tauri::command]
pub fn get_conversion_progress(state: State<'_, AppState>) -> Result<Vec<ConversionTask>, String> {
    let tasks = state.tasks.lock().unwrap();
    Ok(tasks.values().cloned().collect())
}

#[tauri::command]
pub fn cancel_conversion(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut tasks = state.tasks.lock().unwrap();
    if let Some(task) = tasks.get_mut(&task_id) {
        task.status = TaskStatus::Cancelled;
    }
    Ok(())
}

#[tauri::command]
pub async fn select_output_directory() -> Result<Option<String>, String> {
    use tauri::api::dialog::blocking::FileDialogBuilder;
    
    let path = FileDialogBuilder::new()
        .set_title("Select Output Directory")
        .pick_folder();
        
    Ok(path.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn validate_youtube_url(url: String) -> Result<bool, String> {
    Ok(is_valid_youtube_url(&url))
}
