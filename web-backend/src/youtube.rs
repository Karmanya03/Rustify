use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::fs;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub duration: Option<String>,
    pub thumbnail: Option<String>,
    pub channel: Option<String>,
    pub upload_date: Option<String>,
    pub view_count: Option<u64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistInfo {
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub video_count: usize,
    pub videos: Vec<VideoInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    pub fps: Option<f64>,
    pub filesize: Option<u64>,
    pub audio_codec: Option<String>,
    pub video_codec: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub url: String,
    pub format: String,
    pub quality: String,
    pub output_dir: String,
}

#[derive(Debug, Clone)]
pub struct YouTubeDownloader {
    pub yt_dlp_path: String,
    pub ffmpeg_path: String,
}

impl YouTubeDownloader {
    pub fn new() -> Self {
        // Try different possible yt-dlp commands
        let yt_dlp_path = Self::find_yt_dlp_executable();
        
        Self {
            yt_dlp_path,
            ffmpeg_path: "ffmpeg".to_string(),
        }
    }

    fn find_yt_dlp_executable() -> String {
        // Try different possible yt-dlp commands in order of preference
        let possible_commands = [
            "yt-dlp",
            "yt-dlp.exe",
            "python -m yt_dlp",
            "python3 -m yt_dlp",
            "py -m yt_dlp",
        ];

        for cmd in &possible_commands {
            if Self::test_command(cmd) {
                info!("Found yt-dlp executable: {}", cmd);
                return cmd.to_string();
            }
        }

        warn!("No yt-dlp executable found, using default: yt-dlp");
        "yt-dlp".to_string()
    }

    fn test_command(cmd: &str) -> bool {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return false;
        }

        let mut command = std::process::Command::new(parts[0]);
        for arg in &parts[1..] {
            command.arg(arg);
        }
        command.arg("--version");
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());

        command.status().map(|status| status.success()).unwrap_or(false)
    }

    pub async fn check_dependencies(&self) -> Result<()> {
        // Check yt-dlp with the found command
        let parts: Vec<&str> = self.yt_dlp_path.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("Invalid yt-dlp command"));
        }

        let mut command = Command::new(parts[0]);
        for arg in &parts[1..] {
            command.arg(arg);
        }
        command.arg("--version");

        let yt_dlp_check = command.output().await;

        if yt_dlp_check.is_err() {
            return Err(anyhow!(
                "yt-dlp not found. Please install yt-dlp using one of these methods:\n\
                1. pip install yt-dlp\n\
                2. pip3 install yt-dlp\n\
                3. py -m pip install yt-dlp\n\
                4. Download from: https://github.com/yt-dlp/yt-dlp/releases"
            ));
        }

        // Check ffmpeg
        let ffmpeg_check = Command::new(&self.ffmpeg_path)
            .arg("-version")
            .output()
            .await;

        if ffmpeg_check.is_err() {
            warn!("ffmpeg not found. Some features may not work properly.");
        }

        Ok(())
    }

    async fn execute_yt_dlp_command(&self, args: &[&str]) -> Result<std::process::Output> {
        let parts: Vec<&str> = self.yt_dlp_path.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("Invalid yt-dlp command"));
        }

        let mut command = Command::new(parts[0]);
        
        // Add any additional parts of the command (like "python -m yt_dlp")
        for arg in &parts[1..] {
            command.arg(arg);
        }
        
        // Add the actual arguments
        for arg in args {
            command.arg(arg);
        }

        command.output().await.map_err(|e| anyhow!("Failed to execute yt-dlp: {}", e))
    }

    pub async fn get_video_info(&self, url: &str) -> Result<VideoInfo> {
        let output = self.execute_yt_dlp_command(&[
            "--dump-json",
            "--no-download",
            url,
        ]).await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get video info: {}", error_msg));
        }

        let json_str = String::from_utf8(output.stdout)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

        Ok(VideoInfo {
            id: json_value["id"].as_str().unwrap_or("unknown").to_string(),
            title: json_value["title"].as_str().unwrap_or("Unknown").to_string(),
            duration: json_value["duration_string"].as_str().map(|s| s.to_string()),
            thumbnail: json_value["thumbnail"].as_str().map(|s| s.to_string()),
            channel: json_value["uploader"].as_str().map(|s| s.to_string()),
            upload_date: json_value["upload_date"].as_str().map(|s| s.to_string()),
            view_count: json_value["view_count"].as_u64(),
            description: json_value["description"].as_str().map(|s| s.to_string()),
        })
    }

    pub async fn get_playlist_info(&self, url: &str) -> Result<PlaylistInfo> {
        let output = self.execute_yt_dlp_command(&[
            "--dump-json",
            "--flat-playlist",
            "--no-download",
            url,
        ]).await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get playlist info: {}", error_msg));
        }

        let json_str = String::from_utf8(output.stdout)?;
        let mut videos = Vec::new();
        let mut playlist_title = "Unknown Playlist".to_string();
        let mut playlist_id = "unknown".to_string();
        let mut uploader = None;

        for line in json_str.lines() {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
                if json_value["_type"].as_str() == Some("playlist") {
                    playlist_title = json_value["title"].as_str().unwrap_or("Unknown Playlist").to_string();
                    playlist_id = json_value["id"].as_str().unwrap_or("unknown").to_string();
                    uploader = json_value["uploader"].as_str().map(|s| s.to_string());
                } else if json_value["_type"].as_str() == Some("url") {
                    videos.push(VideoInfo {
                        id: json_value["id"].as_str().unwrap_or("unknown").to_string(),
                        title: json_value["title"].as_str().unwrap_or("Unknown").to_string(),
                        duration: json_value["duration_string"].as_str().map(|s| s.to_string()),
                        thumbnail: json_value["thumbnail"].as_str().map(|s| s.to_string()),
                        channel: json_value["uploader"].as_str().map(|s| s.to_string()),
                        upload_date: json_value["upload_date"].as_str().map(|s| s.to_string()),
                        view_count: json_value["view_count"].as_u64(),
                        description: json_value["description"].as_str().map(|s| s.to_string()),
                    });
                }
            }
        }

        Ok(PlaylistInfo {
            id: playlist_id,
            title: playlist_title,
            uploader,
            video_count: videos.len(),
            videos,
        })
    }

    pub async fn get_available_formats(&self, url: &str) -> Result<Vec<FormatInfo>> {
        let output = self.execute_yt_dlp_command(&[
            "--list-formats",
            "--dump-json",
            "--no-download",
            url,
        ]).await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get formats: {}", error_msg));
        }

        let json_str = String::from_utf8(output.stdout)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

        let mut formats = Vec::new();
        if let Some(format_array) = json_value["formats"].as_array() {
            for format_obj in format_array {
                formats.push(FormatInfo {
                    format_id: format_obj["format_id"].as_str().unwrap_or("unknown").to_string(),
                    ext: format_obj["ext"].as_str().unwrap_or("unknown").to_string(),
                    resolution: format_obj["resolution"].as_str().map(|s| s.to_string()),
                    fps: format_obj["fps"].as_f64(),
                    filesize: format_obj["filesize"].as_u64(),
                    audio_codec: format_obj["acodec"].as_str().map(|s| s.to_string()),
                    video_codec: format_obj["vcodec"].as_str().map(|s| s.to_string()),
                });
            }
        }

        Ok(formats)
    }

    pub async fn download_video(&self, options: ConversionOptions) -> Result<String> {
        // Create output directory
        fs::create_dir_all(&options.output_dir).await?;

        let output_pattern = format!("{}/%(title)s.%(ext)s", options.output_dir);

        let format_args = match options.format.as_str() {
            "mp3" => {
                let quality = self.get_audio_quality(&options.quality);
                vec![
                    "--extract-audio".to_string(),
                    "--audio-format".to_string(), "mp3".to_string(),
                    "--audio-quality".to_string(), quality,
                    "--output".to_string(), output_pattern,
                ]
            }
            "wav" => {
                vec![
                    "--extract-audio".to_string(),
                    "--audio-format".to_string(), "wav".to_string(),
                    "--output".to_string(), output_pattern,
                ]
            }
            "mp4" => {
                let format_selector = self.get_video_format_selector(&options.quality);
                vec![
                    "--format".to_string(), format_selector,
                    "--output".to_string(), output_pattern,
                ]
            }
            "webm" => {
                let format_selector = format!("{}[ext=webm]", self.get_video_format_selector(&options.quality));
                vec![
                    "--format".to_string(), format_selector,
                    "--output".to_string(), output_pattern,
                ]
            }
            _ => return Err(anyhow!("Unsupported format: {}", options.format)),
        };

        let mut args: Vec<&str> = format_args.iter().map(|s| s.as_str()).collect();
        args.push(&options.url);

        let output = self.execute_yt_dlp_command(&args).await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Download failed: {}", error_msg));
        }

        // Find the downloaded file
        let mut entries = fs::read_dir(&options.output_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(_file_name) = path.file_name() {
                    return Ok(path.to_string_lossy().to_string());
                }
            }
        }

        Err(anyhow!("Downloaded file not found"))
    }

    fn get_audio_quality(&self, quality: &str) -> String {
        match quality {
            "320" => "0".to_string(), // Best quality
            "256" => "2".to_string(),
            "192" => "3".to_string(),
            "128" => "5".to_string(),
            "96" => "7".to_string(),
            _ => "0".to_string(), // Default to best
        }
    }

    fn get_video_format_selector(&self, quality: &str) -> String {
        match quality {
            "1080p60" => "best[height<=1080][fps>=60]".to_string(),
            "1080p" => "best[height<=1080]".to_string(),
            "720p60" => "best[height<=720][fps>=60]".to_string(),
            "720p" => "best[height<=720]".to_string(),
            "480p" => "best[height<=480]".to_string(),
            "360p" => "best[height<=360]".to_string(),
            _ => "best".to_string(),
        }
    }
}
