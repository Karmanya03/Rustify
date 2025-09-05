use std::collections::HashMap;
use tokio::sync::{Mutex, broadcast};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::youtube::YouTubeDownloader;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppState {
    pub tasks: Arc<Mutex<HashMap<String, TaskResponse>>>,
    pub task_updates: broadcast::Sender<TaskUpdate>,
    pub youtube_downloader: Arc<YouTubeDownloader>,
    pub downloads_dir: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskResponse {
    pub id: String,
    pub url: String,
    pub format: String,
    pub quality: String,
    pub status: String,
    pub progress: f64,
    pub created_at: DateTime<Utc>,
    pub output_path: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskUpdate {
    pub task_id: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub speed: String,
    pub eta: String,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub enum TaskStatus {
    Pending,
    Converting,
    Completed,
    Cancelled,
    Failed(String),
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let (task_updates, _) = broadcast::channel(100);
        let youtube_downloader = Arc::new(YouTubeDownloader::new());
        
        // Check if yt-dlp is available
        if let Err(e) = youtube_downloader.check_dependencies().await {
            tracing::warn!("YouTube downloader dependencies check failed: {}", e);
        }
        
        let downloads_dir = std::env::var("DOWNLOADS_DIR")
            .unwrap_or_else(|_| "./downloads".to_string());
        
        // Create downloads directory if it doesn't exist
        tokio::fs::create_dir_all(&downloads_dir).await?;
        
        Ok(Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            task_updates,
            youtube_downloader,
            downloads_dir,
        })
    }

    pub fn subscribe_to_updates(&self) -> broadcast::Receiver<TaskUpdate> {
        self.task_updates.subscribe()
    }

    #[allow(dead_code)]
    pub async fn get_all_tasks(&self) -> Vec<TaskResponse> {
        let tasks = self.tasks.lock().await;
        tasks.values().cloned().collect()
    }

    #[allow(dead_code)]
    pub async fn broadcast_update(&self, update: TaskUpdate) {
        let _ = self.task_updates.send(update);
    }
}
