use std::collections::HashMap;
use tokio::sync::{Mutex, broadcast};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct AppState {
    pub tasks: std::sync::Arc<Mutex<HashMap<String, TaskResponse>>>,
    pub task_updates: broadcast::Sender<TaskUpdate>,
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
        
        Ok(Self {
            tasks: std::sync::Arc::new(Mutex::new(HashMap::new())),
            task_updates,
        })
    }

    pub fn subscribe_to_updates(&self) -> broadcast::Receiver<TaskUpdate> {
        self.task_updates.subscribe()
    }

    pub async fn get_all_tasks(&self) -> Vec<TaskResponse> {
        let tasks = self.tasks.lock().await;
        tasks.values().cloned().collect()
    }

    pub async fn broadcast_update(&self, update: TaskUpdate) {
        let _ = self.task_updates.send(update);
    }
}
