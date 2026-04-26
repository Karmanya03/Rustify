use chrono::{DateTime, Utc};
use rustify_core::{AppConfig, AuthMode, EzP3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub rustify: Arc<EzP3>,
    pub tasks: Arc<Mutex<HashMap<String, TaskResponse>>>,
    pub task_updates: broadcast::Sender<TaskUpdate>,
    pub downloads_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskResponse {
    pub id: String,
    pub title: Option<String>,
    pub url: String,
    pub format: String,
    pub quality: String,
    pub status: String,
    pub progress: f64,
    pub created_at: DateTime<Utc>,
    pub output_path: Option<String>,
    pub file_path: Option<String>,
    pub playlist_files: Option<Vec<String>>,
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
        let mut config = AppConfig::default();
        let allow_browser_cookies = std::env::var("RUSTIFY_WEB_ALLOW_BROWSER_COOKIES")
            .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        if !allow_browser_cookies {
            config.auth.mode = AuthMode::None;
        }
        let rustify = EzP3::with_config(config)?;
        let downloads_dir = std::env::var("DOWNLOADS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./downloads"));

        tokio::fs::create_dir_all(&downloads_dir).await?;

        Ok(Self {
            rustify: Arc::new(rustify),
            tasks: Arc::new(Mutex::new(HashMap::new())),
            task_updates,
            downloads_dir,
        })
    }

    pub fn subscribe_to_updates(&self) -> broadcast::Receiver<TaskUpdate> {
        self.task_updates.subscribe()
    }
}
