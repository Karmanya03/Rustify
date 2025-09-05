use rustify_core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Extracting,
    Converting,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskUpdate {
    pub task_id: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub speed: String,
    pub eta: String,
}

#[derive(Clone)]
pub struct AppState {
    pub ezp3: Arc<EzP3>,
    pub tasks: Arc<Mutex<HashMap<String, TaskResponse>>>,
    pub task_broadcaster: broadcast::Sender<TaskUpdate>,
    #[allow(dead_code)]
    pub output_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskResponse {
    pub id: String,
    pub url: String,
    pub title: String,
    pub format: String,
    pub quality: String,
    pub status: String,
    pub progress: f32,
    pub output_path: Option<String>,
    pub created_at: String,
    pub file_path: Option<String>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let ezp3 = Arc::new(EzP3::new()?);
        let tasks = Arc::new(Mutex::new(HashMap::new()));
        let (task_broadcaster, _) = broadcast::channel(1000);
        
        // Create output directory
        let output_dir = std::env::current_dir()?.join("downloads");
        tokio::fs::create_dir_all(&output_dir).await?;
        
        Ok(Self {
            ezp3,
            tasks,
            task_broadcaster,
            output_dir,
        })
    }
    
    #[allow(dead_code)]
    pub async fn add_task(&self, task: TaskResponse) {
        let mut tasks = self.tasks.lock().await;
        tasks.insert(task.id.clone(), task);
    }
    
    #[allow(dead_code)]
    pub async fn update_task<F>(&self, task_id: &str, updater: F) -> Option<TaskResponse>
    where
        F: FnOnce(&mut TaskResponse),
    {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.get_mut(task_id) {
            updater(task);
            
            // Broadcast update
            let update = TaskUpdate {
                task_id: task.id.clone(),
                status: TaskStatus::Pending, // Convert string to enum if needed
                progress: task.progress,
                speed: "0x".to_string(),
                eta: "Unknown".to_string(),
            };
            let _ = self.task_broadcaster.send(update);
            
            Some(task.clone())
        } else {
            None
        }
    }
    
    #[allow(dead_code)]
    pub async fn get_task(&self, task_id: &str) -> Option<TaskResponse> {
        let tasks = self.tasks.lock().await;
        tasks.get(task_id).cloned()
    }
    
    pub async fn get_all_tasks(&self) -> Vec<TaskResponse> {
        let tasks = self.tasks.lock().await;
        tasks.values().cloned().collect()
    }
    
    #[allow(dead_code)]
    pub async fn remove_task(&self, task_id: &str) -> Option<TaskResponse> {
        let mut tasks = self.tasks.lock().await;
        tasks.remove(task_id)
    }
    
    #[allow(dead_code)]
    pub fn generate_task_id(&self) -> String {
        Uuid::new_v4().to_string()
    }
    
    pub fn subscribe_to_updates(&self) -> broadcast::Receiver<TaskUpdate> {
        self.task_broadcaster.subscribe()
    }
}
