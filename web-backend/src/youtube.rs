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

// Simple function to get the right yt-dlp command
fn get_yt_dlp_command() -> Vec<String> {
    // For hosting environments (Render.com, production), use direct yt-dlp
    if std::env::var("RENDER").is_ok() || 
       std::env::var("NODE_ENV").map_or(false, |v| v == "production") ||
       std::env::var("RAILWAY_ENVIRONMENT").is_ok() {
        return vec!["yt-dlp".to_string()];
    }
    
    // For local development, try different commands
    let possible_commands = [
        vec!["yt-dlp".to_string()],
        vec!["python".to_string(), "-m".to_string(), "yt_dlp".to_string()],
        vec!["python3".to_string(), "-m".to_string(), "yt_dlp".to_string()],
        vec!["py".to_string(), "-m".to_string(), "yt_dlp".to_string()],
    ];

    for cmd in &possible_commands {
        if test_command(cmd) {
            info!("Found yt-dlp executable: {:?}", cmd);
            return cmd.clone();
        }
    }

    warn!("No yt-dlp executable found, using default: yt-dlp");
    vec!["yt-dlp".to_string()]
}

fn test_command(cmd: &[String]) -> bool {
    if cmd.is_empty() {
        return false;
    }

    let mut command = std::process::Command::new(&cmd[0]);
    for arg in &cmd[1..] {
        command.arg(arg);
    }
    command.arg("--version");
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());

    command.status().map(|status| status.success()).unwrap_or(false)
}

// Simple function to check if yt-dlp is available
pub async fn check_dependencies() -> Result<()> {
    let yt_dlp_cmd = get_yt_dlp_command();
    let mut command = Command::new(&yt_dlp_cmd[0]);
    for arg in &yt_dlp_cmd[1..] {
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
            4. Download from: https://github.com/yt-dlp/yt-dlp"
        ));
    }

    Ok(())
}

// Main download function - super simple and reliable
pub async fn download_video(options: ConversionOptions) -> Result<String> {
    // Create output directory
    fs::create_dir_all(&options.output_dir).await?;

    // Use absolute path
    let output_dir = std::path::Path::new(&options.output_dir)
        .canonicalize()
        .map_err(|e| anyhow!("Failed to resolve output directory: {}", e))?;

    let format_arg = match options.format.as_str() {
        "mp3" => "bestaudio[ext=m4a]/bestaudio[ext=webm]/bestaudio[ext=mp3]/bestaudio".to_string(),
        "wav" => "bestaudio[ext=wav]/bestaudio[ext=m4a]/bestaudio".to_string(),
        "mp4" => format!("best[ext=mp4][height<={}]/best[ext=mp4]/best", get_quality_height(&options.quality)),
        "webm" => format!("best[ext=webm][height<={}]/best[ext=webm]/best", get_quality_height(&options.quality)),
        _ => return Err(anyhow!("Unsupported format: {}", options.format)),
    };

    let output_pattern = format!("{}/%(title).50s.%(ext)s", output_dir.to_string_lossy().replace('\\', "/"));

    // Get the yt-dlp command
    let yt_dlp_cmd = get_yt_dlp_command();
    let mut command = Command::new(&yt_dlp_cmd[0]);
    
    // Add base yt-dlp arguments if it's a python module call
    for arg in &yt_dlp_cmd[1..] {
        command.arg(arg);
    }
    
    // Add download arguments
    command
        .arg("--format").arg(&format_arg)
        .arg("--output").arg(&output_pattern)
        .arg("--no-playlist")
        .arg("--user-agent").arg("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
        .arg("--extractor-retries").arg("5")
        .arg("--geo-bypass")
        .arg("--ignore-errors")
        .arg(&options.url);

    info!("Executing simplified yt-dlp command: {:?}", yt_dlp_cmd);
    let output = command.output().await.map_err(|e| anyhow!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        let stdout_msg = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!("Download failed.\nStderr: {}\nStdout: {}", error_msg, stdout_msg));
    }

    find_downloaded_file(&output_dir).await
}

async fn find_downloaded_file(output_dir: &std::path::Path) -> Result<String> {
    let mut entries = fs::read_dir(&output_dir).await?;
    let mut newest_file = None;
    let mut newest_time = std::time::SystemTime::UNIX_EPOCH;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if modified > newest_time {
                        newest_time = modified;
                        newest_file = Some(path);
                    }
                }
            }
        }
    }

    newest_file
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .ok_or_else(|| anyhow!("No downloaded file found"))
}

fn get_quality_height(quality: &str) -> String {
    match quality {
        "1080p60" | "1080p" => "1080".to_string(),
        "720p60" | "720p" => "720".to_string(),
        "480p" => "480".to_string(),
        "360p" => "360".to_string(),
        _ => "720".to_string(), // Default to 720p
    }
}

// Get video info function
pub async fn get_video_info(url: &str) -> Result<VideoInfo> {
    let yt_dlp_cmd = get_yt_dlp_command();
    let mut command = Command::new(&yt_dlp_cmd[0]);
    
    // Add base yt-dlp arguments if it's a python module call
    for arg in &yt_dlp_cmd[1..] {
        command.arg(arg);
    }
    
    command
        .arg("--dump-json")
        .arg("--no-playlist")
        .arg(url);

    let output = command.output().await.map_err(|e| anyhow!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to get video info: {}", error_msg));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| anyhow!("Failed to parse JSON response: {}", e))?;

    Ok(VideoInfo {
        id: json_value["id"].as_str().unwrap_or("").to_string(),
        title: json_value["title"].as_str().unwrap_or("Unknown").to_string(),
        duration: json_value["duration"].as_f64().map(|d| format!("{}s", d as u64)),
        thumbnail: json_value["thumbnail"].as_str().map(|s| s.to_string()),
        channel: json_value["uploader"].as_str().map(|s| s.to_string()),
        upload_date: json_value["upload_date"].as_str().map(|s| s.to_string()),
        view_count: json_value["view_count"].as_u64(),
        description: json_value["description"].as_str().map(|s| s.to_string()),
    })
}

// Get available formats
pub async fn get_formats(url: &str) -> Result<Vec<FormatInfo>> {
    let yt_dlp_cmd = get_yt_dlp_command();
    let mut command = Command::new(&yt_dlp_cmd[0]);
    
    // Add base yt-dlp arguments if it's a python module call
    for arg in &yt_dlp_cmd[1..] {
        command.arg(arg);
    }
    
    command
        .arg("--list-formats")
        .arg("--no-playlist")
        .arg(url);

    let output = command.output().await.map_err(|e| anyhow!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to get formats: {}", error_msg));
    }

    // Parse the format list (simplified)
    let mut formats = Vec::new();
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines() {
        if line.contains("mp4") || line.contains("webm") || line.contains("m4a") {
            formats.push(FormatInfo {
                format_id: "auto".to_string(),
                ext: if line.contains("mp4") { "mp4".to_string() } 
                     else if line.contains("webm") { "webm".to_string() }
                     else { "m4a".to_string() },
                resolution: None,
                fps: None,
                filesize: None,
                audio_codec: None,
                video_codec: None,
            });
        }
    }
    
    Ok(formats)
}

// Get playlist info function
pub async fn get_playlist_info(url: &str) -> Result<PlaylistInfo> {
    let yt_dlp_cmd = get_yt_dlp_command();
    let mut command = Command::new(&yt_dlp_cmd[0]);
    
    // Add base yt-dlp arguments if it's a python module call
    for arg in &yt_dlp_cmd[1..] {
        command.arg(arg);
    }
    
    command
        .arg("--dump-json")
        .arg("--flat-playlist")
        .arg(url);

    let output = command.output().await.map_err(|e| anyhow!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to get playlist info: {}", error_msg));
    }

    let json_lines = String::from_utf8_lossy(&output.stdout);
    let mut videos = Vec::new();
    
    for line in json_lines.lines() {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
            videos.push(VideoInfo {
                id: json_value["id"].as_str().unwrap_or("").to_string(),
                title: json_value["title"].as_str().unwrap_or("Unknown").to_string(),
                duration: json_value["duration"].as_f64().map(|d| format!("{}s", d as u64)),
                thumbnail: json_value["thumbnail"].as_str().map(|s| s.to_string()),
                channel: json_value["uploader"].as_str().map(|s| s.to_string()),
                upload_date: json_value["upload_date"].as_str().map(|s| s.to_string()),
                view_count: json_value["view_count"].as_u64(),
                description: json_value["description"].as_str().map(|s| s.to_string()),
            });
        }
    }

    Ok(PlaylistInfo {
        id: "playlist".to_string(),
        title: "Playlist".to_string(),
        uploader: None,
        video_count: videos.len(),
        videos,
    })
}

// Download playlist function
pub async fn download_playlist(options: ConversionOptions) -> Result<Vec<String>> {
    // For now, just download the first video as a simple implementation
    // In a full implementation, you'd iterate through all playlist videos
    let file_path = download_video(options).await?;
    Ok(vec![file_path])
}
