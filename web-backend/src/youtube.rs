use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
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
}

impl YouTubeDownloader {
    pub fn new() -> Self {
        // Try different possible yt-dlp commands
        let yt_dlp_path = Self::find_yt_dlp_executable();
        
        Self {
            yt_dlp_path,
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

        // Note: ffmpeg is not required for web version since we download existing formats
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
            "--user-agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            "--add-header", "Accept-Language:en-US,en;q=0.9",
            "--extractor-retries", "3",
            "--geo-bypass",
            "--no-check-certificate",
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
            "--user-agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            "--add-header", "Accept-Language:en-US,en;q=0.9",
            "--extractor-retries", "3",
            "--geo-bypass",
            "--no-check-certificate",
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
            "--user-agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            "--add-header", "Accept-Language:en-US,en;q=0.9",
            "--extractor-retries", "3",
            "--geo-bypass",
            "--no-check-certificate",
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

        // Use absolute path and escape spaces for Windows
        let output_dir = std::path::Path::new(&options.output_dir)
            .canonicalize()
            .map_err(|e| anyhow!("Failed to resolve output directory: {}", e))?;

        // Render.com optimized bot protection (minimal, proven options only)
        let bot_bypass_args = vec![
            // Essential user agent spoofing (Linux for Render.com)
            "--user-agent".to_string(),
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".to_string(),
            // Basic headers that work reliably
            "--add-header".to_string(),
            "Accept-Language:en-US,en;q=0.9".to_string(),
            // Conservative retry strategy for free tier
            "--extractor-retries".to_string(),
            "3".to_string(),
            "--fragment-retries".to_string(),
            "3".to_string(),
            "--retry-sleep".to_string(),
            "linear=2:10:20".to_string(),
            // Essential bypass options that work on all systems
            "--geo-bypass".to_string(),
            "--no-check-certificate".to_string(),
            "--ignore-errors".to_string(),
        ];

        let mut format_args = match options.format.as_str() {
            "mp3" => {
                // Download best audio in existing format, no conversion
                vec![
                    "--format".to_string(), "bestaudio[ext=m4a]/bestaudio[ext=webm]/bestaudio[ext=mp3]/bestaudio".to_string(),
                    "--output".to_string(), format!("{}/%(title).50s.%(ext)s", 
                        output_dir.to_string_lossy().replace('\\', "/")),
                    "--no-playlist".to_string(),
                    "--no-post-overwrites".to_string(),
                ]
            }
            "wav" => {
                // Download audio in existing format, no conversion to WAV
                vec![
                    "--format".to_string(), "bestaudio[ext=wav]/bestaudio[ext=m4a]/bestaudio".to_string(),
                    "--output".to_string(), format!("{}/%(title).50s.%(ext)s", 
                        output_dir.to_string_lossy().replace('\\', "/")),
                    "--no-playlist".to_string(),
                ]
            }
            "mp4" => {
                // Download MP4 directly without any processing
                vec![
                    "--format".to_string(), format!("best[ext=mp4][height<={}]/best[ext=mp4]/best", 
                        self.get_quality_height(&options.quality)),
                    "--output".to_string(), format!("{}/%(title).50s.%(ext)s", 
                        output_dir.to_string_lossy().replace('\\', "/")),
                    "--no-playlist".to_string(),
                ]
            }
            "webm" => {
                // Download WebM directly
                vec![
                    "--format".to_string(), format!("best[ext=webm][height<={}]/best[ext=webm]/best", 
                        self.get_quality_height(&options.quality)),
                    "--output".to_string(), format!("{}/%(title).50s.%(ext)s", 
                        output_dir.to_string_lossy().replace('\\', "/")),
                    "--no-playlist".to_string(),
                ]
            }
            _ => return Err(anyhow!("Unsupported format: {}", options.format)),
        };

        // Add bot protection bypass arguments to format args
        format_args.extend(bot_bypass_args);

        let mut args: Vec<&str> = format_args.iter().map(|s| s.as_str()).collect();
        args.push(&options.url);

        info!("Executing yt-dlp with args: {:?}", args);
        let output = self.execute_yt_dlp_command(&args).await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow!("Download failed.\nStderr: {}\nStdout: {}", error_msg, stdout_msg));
        }

        // Find the downloaded file
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

        if let Some(file_path) = newest_file {
            Ok(file_path.to_string_lossy().to_string())
        } else {
            Err(anyhow!("Downloaded file not found in directory: {}", output_dir.display()))
        }
    }

    pub async fn download_playlist(&self, options: ConversionOptions) -> Result<Vec<String>> {
        // Create output directory
        fs::create_dir_all(&options.output_dir).await?;

        // Use absolute path and escape spaces for Windows
        let output_dir = std::path::Path::new(&options.output_dir)
            .canonicalize()
            .map_err(|e| anyhow!("Failed to resolve output directory: {}", e))?;
        
        let output_pattern = format!(
            "{}/%(playlist_index)02d - %(title).50s.%(ext)s", 
            output_dir.to_string_lossy().replace('\\', "/")
        );

        // Render.com optimized bot protection (minimal, proven options only)
        let bot_bypass_args = vec![
            // Essential user agent spoofing (Linux for Render.com)
            "--user-agent".to_string(),
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".to_string(),
            // Basic headers that work reliably
            "--add-header".to_string(),
            "Accept-Language:en-US,en;q=0.9".to_string(),
            // Conservative retry strategy for free tier
            "--extractor-retries".to_string(),
            "3".to_string(),
            "--fragment-retries".to_string(),
            "3".to_string(),
            "--retry-sleep".to_string(),
            "linear=2:10:20".to_string(),
            // Essential bypass options that work on all systems
            "--geo-bypass".to_string(),
            "--no-check-certificate".to_string(),
            "--ignore-errors".to_string(),
        ];

        // First, get playlist info to know how many videos we're dealing with
        let mut info_args = vec![
            "--dump-json",
            "--flat-playlist",
            "--playlist-end",
            "1",
        ];
        
        // Add bot bypass args to info command
        for arg in &bot_bypass_args {
            info_args.push(arg.as_str());
        }
        info_args.push(&options.url);
        
        info!("Getting playlist info with args: {:?}", info_args);
        let info_output = self.execute_yt_dlp_command(&info_args).await?;
        
        if !info_output.status.success() {
            let error_msg = String::from_utf8_lossy(&info_output.stderr);
            return Err(anyhow!("Failed to get playlist info: {}", error_msg));
        }

        // Parse playlist info to get video count
        let playlist_info = String::from_utf8_lossy(&info_output.stdout);
        let video_count = playlist_info.lines()
            .filter(|line| line.contains("\"_type\": \"url\"") || line.contains("\"id\":"))
            .count();
        
        info!("Playlist contains approximately {} videos", video_count);

        let mut format_args = match options.format.as_str() {
            "mp3" => {
                // Download best audio in existing format, no conversion
                vec![
                    "--format".to_string(), "bestaudio[ext=m4a]/bestaudio[ext=webm]/bestaudio[ext=mp3]/bestaudio".to_string(),
                    "--output".to_string(), output_pattern,
                    "--yes-playlist".to_string(),
                    "--no-post-overwrites".to_string(),
                    "--ignore-errors".to_string(), // Continue on individual video errors
                    "--no-abort-on-error".to_string(),
                ]
            }
            "wav" => {
                // Download audio in existing format, no conversion to WAV
                vec![
                    "--format".to_string(), "bestaudio[ext=wav]/bestaudio[ext=m4a]/bestaudio".to_string(),
                    "--output".to_string(), output_pattern,
                    "--yes-playlist".to_string(),
                    "--ignore-errors".to_string(),
                    "--no-abort-on-error".to_string(),
                ]
            }
            "mp4" => {
                // Download MP4 directly without any processing
                vec![
                    "--format".to_string(), format!("best[ext=mp4][height<={}]/best[ext=mp4]/best", 
                        self.get_quality_height(&options.quality)),
                    "--output".to_string(), output_pattern,
                    "--yes-playlist".to_string(),
                    "--ignore-errors".to_string(),
                    "--no-abort-on-error".to_string(),
                ]
            }
            "webm" => {
                // Download WebM directly
                vec![
                    "--format".to_string(), format!("best[ext=webm][height<={}]/best[ext=webm]/best", 
                        self.get_quality_height(&options.quality)),
                    "--output".to_string(), output_pattern,
                    "--yes-playlist".to_string(),
                    "--ignore-errors".to_string(),
                    "--no-abort-on-error".to_string(),
                ]
            }
            _ => return Err(anyhow!("Unsupported format: {}", options.format)),
        };

        // Add bot protection bypass arguments to format args
        format_args.extend(bot_bypass_args);

        let mut args: Vec<&str> = format_args.iter().map(|s| s.as_str()).collect();
        args.push(&options.url);

        info!("Executing yt-dlp playlist download with args: {:?}", args);
        
        // Execute the download with streaming output for progress tracking
        let mut child = tokio::process::Command::new("python")
            .args(&["-m", "yt_dlp"])
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn yt-dlp process: {}", e))?;

        // Read output for progress tracking
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;

        let mut stdout_reader = BufReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);

        // Read output streams
        let stdout_task = tokio::spawn(async move {
            let mut line = String::new();
            let mut output = Vec::new();
            loop {
                line.clear();
                match stdout_reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        output.extend_from_slice(line.as_bytes());
                        // Log progress for debugging
                        if line.contains("[download]") || line.contains("Downloading") {
                            info!("Download progress: {}", line.trim());
                        }
                    }
                    Err(_) => break,
                }
            }
            output
        });

        let stderr_task = tokio::spawn(async move {
            let mut line = String::new();
            let mut output = Vec::new();
            loop {
                line.clear();
                match stderr_reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        output.extend_from_slice(line.as_bytes());
                        // Log errors/warnings
                        if !line.trim().is_empty() {
                            warn!("yt-dlp stderr: {}", line.trim());
                        }
                    }
                    Err(_) => break,
                }
            }
            output
        });

        // Wait for process to complete
        let status = child.wait().await.map_err(|e| anyhow!("Failed to wait for yt-dlp process: {}", e))?;
        
        // Collect outputs
        let stdout_output = stdout_task.await.map_err(|e| anyhow!("Failed to read stdout: {}", e))?;
        let stderr_output = stderr_task.await.map_err(|e| anyhow!("Failed to read stderr: {}", e))?;

        if !status.success() {
            let error_msg = String::from_utf8_lossy(&stderr_output);
            let stdout_msg = String::from_utf8_lossy(&stdout_output);
            
            // Check if it's a partial failure (some videos downloaded)
            if stdout_msg.contains("has already been downloaded") || 
               stdout_msg.contains("[download] Downloading") {
                warn!("Playlist download completed with some errors: {}", error_msg);
            } else {
                return Err(anyhow!("Playlist download failed.\nStderr: {}\nStdout: {}", error_msg, stdout_msg));
            }
        }

        // Find all downloaded files
        let mut entries = fs::read_dir(&output_dir).await?;
        let mut downloaded_files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                // Check if this is a newly downloaded file (has the expected extensions)
                if file_name.ends_with(&format!(".{}", options.format)) ||
                   file_name.ends_with(".mp3") || file_name.ends_with(".mp4") || 
                   file_name.ends_with(".wav") || file_name.ends_with(".webm") ||
                   file_name.ends_with(".m4a") {
                    downloaded_files.push(path.to_string_lossy().to_string());
                }
            }
        }

        if downloaded_files.is_empty() {
            Err(anyhow!("No files downloaded for playlist in directory: {}", output_dir.display()))
        } else {
            downloaded_files.sort(); // Sort files for consistent ordering
            info!("Successfully downloaded {} files from playlist", downloaded_files.len());
            Ok(downloaded_files)
        }
    }

    fn get_quality_height(&self, quality: &str) -> String {
        match quality {
            "1080p60" | "1080p" => "1080".to_string(),
            "720p60" | "720p" => "720".to_string(),
            "480p" => "480".to_string(),
            "360p" => "360".to_string(),
            _ => "720".to_string(), // Default to 720p
        }
    }
}
