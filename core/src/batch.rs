use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::info;

use crate::{EzP3, OutputFormat, PlaylistInfo};

/// Batch conversion job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJob {
    pub id: String,
    pub name: String,
    pub playlist_url: String,
    pub output_dir: PathBuf,
    pub formats: Vec<BatchFormat>,
    pub options: BatchOptions,
    pub status: BatchJobStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub total_videos: usize,
    pub completed_videos: usize,
    pub failed_videos: usize,
}

/// Format specification for batch conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFormat {
    pub format: OutputFormat,
    pub quality: String,
    pub enabled: bool,
}

/// Batch conversion options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOptions {
    pub max_concurrent: usize,
    pub skip_existing: bool,
    pub create_subdirs: bool,
    pub add_index_prefix: bool,
    pub video_limit: Option<usize>,
    pub start_index: usize,
    pub download_thumbnails: bool,
    pub create_playlist_file: bool,
}

/// Job status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BatchJobStatus {
    Created,
    Starting,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// Individual conversion task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTask {
    pub id: String,
    pub job_id: String,
    pub video_url: String,
    pub video_title: String,
    pub video_index: usize,
    pub formats: Vec<BatchTaskFormat>,
    pub status: BatchTaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Task status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BatchTaskStatus {
    Pending,
    Converting,
    Completed,
    Failed(String),
    Skipped,
}

/// Format task within a batch task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskFormat {
    pub format: OutputFormat,
    pub quality: String,
    pub output_path: PathBuf,
    pub status: TaskFormatStatus,
    pub progress: f32,
    pub error: Option<String>,
}

/// Status of a specific format conversion
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskFormatStatus {
    Pending,
    Converting,
    Completed,
    Failed(String),
    Skipped,
}

/// Progress information for batch conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
    pub overall_progress: f32,
    pub current_video: Option<String>,
    pub completed_videos: usize,
    pub total_videos: usize,
    pub failed_videos: usize,
    pub estimated_time_remaining: Option<String>,
}

/// Batch processor for managing multiple conversion jobs
pub struct BatchProcessor {
    ezp3: Arc<EzP3>,
    jobs: Arc<Mutex<HashMap<String, BatchJob>>>,
    tasks: Arc<Mutex<HashMap<String, Vec<BatchTask>>>>,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(ezp3: Arc<EzP3>) -> Self {
        Self {
            ezp3,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new batch job
    pub async fn create_batch_job(
        &self,
        name: String,
        playlist_url: String,
        output_dir: PathBuf,
        formats: Vec<BatchFormat>,
        options: BatchOptions,
    ) -> Result<String, anyhow::Error> {
        let job_id = Uuid::new_v4().to_string();
        
        // Get playlist info
        let playlist_info = self.ezp3.get_playlist_info(&playlist_url).await?;
        
        let job = BatchJob {
            id: job_id.clone(),
            name,
            playlist_url,
            output_dir,
            formats,
            options,
            status: BatchJobStatus::Created,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            total_videos: playlist_info.video_count,
            completed_videos: 0,
            failed_videos: 0,
        };

        // Store job
        self.jobs.lock().unwrap().insert(job_id.clone(), job);
        
        // Create tasks for each video/format combination
        let mut tasks = Vec::new();
        let videos = self.filter_videos(&playlist_info, &self.get_job(&job_id).unwrap().options);
        
        for (index, video) in videos.iter().enumerate() {
            let task_id = Uuid::new_v4().to_string();
            let mut format_tasks = Vec::new();
            
            for batch_format in &self.get_job(&job_id).unwrap().formats {
                if batch_format.enabled {
                    let output_path = self.generate_output_path(
                        &self.get_job(&job_id).unwrap().output_dir,
                        video,
                        index,
                        &batch_format.format,
                        &self.get_job(&job_id).unwrap().options,
                    );
                    
                    format_tasks.push(BatchTaskFormat {
                        format: batch_format.format.clone(),
                        quality: batch_format.quality.clone(),
                        output_path,
                        status: TaskFormatStatus::Pending,
                        progress: 0.0,
                        error: None,
                    });
                }
            }
            
            tasks.push(BatchTask {
                id: task_id,
                job_id: job_id.clone(),
                video_url: video.url.clone(),
                video_title: video.title.clone(),
                video_index: index,
                formats: format_tasks,
                status: BatchTaskStatus::Pending,
                created_at: chrono::Utc::now(),
                started_at: None,
                completed_at: None,
            });
        }
        
        self.tasks.lock().unwrap().insert(job_id.clone(), tasks);
        
        Ok(job_id)
    }

    /// Start batch conversion
    pub async fn start_batch<F>(&self, job_id: &str, _progress_callback: F) -> Result<(), anyhow::Error>
    where
        F: Fn(BatchProgress) + Send + Sync + 'static,
    {
        info!("Starting batch conversion for job: {}", job_id);
        
        // For now, this is a simplified implementation
        // In a real implementation, you would:
        // 1. Update job status to Running
        // 2. Process each task with the converter
        // 3. Update progress as you go
        // 4. Handle errors and retries
        
        // Update job status
        if let Some(mut job) = self.get_job(job_id) {
            job.status = BatchJobStatus::Running;
            job.started_at = Some(chrono::Utc::now());
            self.jobs.lock().unwrap().insert(job_id.to_string(), job);
        }
        
        // Simulate processing
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        // Mark as completed
        if let Some(mut job) = self.get_job(job_id) {
            job.status = BatchJobStatus::Completed;
            job.completed_at = Some(chrono::Utc::now());
            job.completed_videos = job.total_videos;
            self.jobs.lock().unwrap().insert(job_id.to_string(), job);
        }
        
        info!("Batch conversion completed for job: {}", job_id);
        Ok(())
    }

    /// Get job by ID
    pub fn get_job(&self, job_id: &str) -> Option<BatchJob> {
        self.jobs.lock().unwrap().get(job_id).cloned()
    }

    /// Get tasks for a job
    pub fn get_job_tasks(&self, job_id: &str) -> Vec<BatchTask> {
        self.tasks.lock().unwrap()
            .get(job_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Generate output path for a video/format combination
    fn generate_output_path(
        &self,
        output_dir: &Path,
        video: &crate::PlaylistVideo,
        index: usize,
        format: &OutputFormat,
        options: &BatchOptions,
    ) -> PathBuf {
        let mut filename = if options.add_index_prefix {
            format!("{:03}-{}", index + 1, video.title)
        } else {
            video.title.clone()
        };
        
        // Sanitize filename
        filename = filename.chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
                c => c,
            })
            .collect();
        
        let extension = match format {
            OutputFormat::Mp3 { .. } => "mp3",
            OutputFormat::Mp4 { .. } => "mp4",
            OutputFormat::Flac => "flac",
            OutputFormat::Aac { .. } => "aac",
            OutputFormat::Ogg { .. } => "ogg",
            OutputFormat::WebM { .. } => "webm",
        };
        
        output_dir.join(format!("{}.{}", filename, extension))
    }

    /// Filter videos based on batch options
    fn filter_videos<'a>(&self, playlist_info: &'a PlaylistInfo, options: &BatchOptions) -> Vec<&'a crate::PlaylistVideo> {
        let mut videos: Vec<&crate::PlaylistVideo> = playlist_info.videos
            .iter()
            .skip(options.start_index)
            .collect();
        
        if let Some(limit) = options.video_limit {
            videos.truncate(limit);
        }
        
        videos
    }
}
