use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::fs;
use tracing::{info, warn, error};
use crate::selenium_youtube::{SeleniumExtractor, should_use_selenium};

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
    // For hosting environments (Render.com, Koyeb, Railway, production), use direct yt-dlp
    if std::env::var("RENDER").is_ok() || 
       std::env::var("KOYEB").is_ok() ||
       std::env::var("NODE_ENV").is_ok_and(|v| v == "production") ||
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

// Main download function with enhanced bot protection
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
    
    // Enhanced bot protection arguments - MAXIMUM AGGRESSION
    command
        .arg("--format").arg(&format_arg)
        .arg("--output").arg(&output_pattern)
        .arg("--no-playlist")
        // Cookie strategies (try Chrome first, fallback to others)
        .arg("--cookies-from-browser").arg("chrome,firefox,edge,safari")  // Try multiple browsers
        .arg("--user-agent").arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        // Comprehensive browser headers
        .arg("--add-header").arg("Accept:text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .arg("--add-header").arg("Accept-Language:en-US,en;q=0.9")
        .arg("--add-header").arg("Accept-Encoding:gzip, deflate, br")
        .arg("--add-header").arg("DNT:1")
        .arg("--add-header").arg("Connection:keep-alive")
        .arg("--add-header").arg("Upgrade-Insecure-Requests:1")
        .arg("--add-header").arg("Sec-Fetch-Dest:document")
        .arg("--add-header").arg("Sec-Fetch-Mode:navigate")
        .arg("--add-header").arg("Sec-Fetch-Site:none")
        .arg("--add-header").arg("Sec-Fetch-User:?1")
        .arg("--add-header").arg("Cache-Control:max-age=0")
        // Advanced retry and timing
        .arg("--extractor-retries").arg("15")  // More retries
        .arg("--sleep-requests").arg("1")      // Wait between requests
        .arg("--sleep-interval").arg("3")      // Wait on errors  
        .arg("--max-sleep-interval").arg("15") // Max wait time
        .arg("--retries").arg("10")            // Connection retries
        // Bypass mechanisms
        .arg("--geo-bypass")
        .arg("--geo-bypass-country").arg("US") // Try US bypass
        .arg("--ignore-errors")
        .arg("--no-check-certificate")         // Ignore SSL issues
        // Age and access controls
        .arg("--age-limit").arg("0")           // Bypass age restrictions
        .arg("--extractor-args").arg("youtube:player_client=web") // Use web client
        .arg(&options.url);

    info!("Executing yt-dlp with enhanced bot protection");
    let output = command.output().await.map_err(|e| anyhow!("Failed to execute yt-dlp: {}", e))?;

    // If chrome cookies failed, try alternative approaches
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        
        // Check if it's a bot detection error
        if error_msg.contains("Sign in to confirm you're not a bot") || error_msg.contains("cookies") {
            warn!("Chrome cookies failed, trying alternative approaches...");
            return try_alternative_download(&yt_dlp_cmd, &options, &output_pattern, &format_arg).await;
        }
        
        let stdout_msg = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!("Download failed.\nStderr: {}\nStdout: {}", error_msg, stdout_msg));
    }

    find_downloaded_file(&output_dir).await
}

// Alternative download strategies if primary method fails
async fn try_alternative_download(
    yt_dlp_cmd: &[String], 
    options: &ConversionOptions, 
    output_pattern: &str, 
    format_arg: &str
) -> Result<String> {
    let advanced_strategies = [
        // Strategy 1: Try bypass with embedded player
        vec!["--extractor-args", "youtube:player_client=web_embedded", "--referer", "https://www.youtube.com/"],
        
        // Strategy 2: Use TV client (often works when web fails)
        vec!["--extractor-args", "youtube:player_client=tv", "--user-agent", "Mozilla/5.0 (SMART-TV; Linux; Tizen 2.4.0) AppleWebKit/538.1 (KHTML, like Gecko) Version/2.4.0 TV Safari/538.1"],
        
        // Strategy 3: iOS client simulation
        vec!["--extractor-args", "youtube:player_client=ios", "--user-agent", "com.google.ios.youtube/17.49.4 (iPhone; U; CPU OS 14_2 like Mac OS X)"],
        
        // Strategy 4: Android client with specific version
        vec!["--extractor-args", "youtube:player_client=android", "--user-agent", "com.google.android.youtube/17.49.4 (Linux; U; Android 11) gzip"],
        
        // Strategy 5: Try with Firefox cookies + specific client
        vec!["--cookies-from-browser", "firefox", "--extractor-args", "youtube:player_client=web"],
        
        // Strategy 6: Use age-restricted bypass
        vec!["--extractor-args", "youtube:skip=translated_subs", "--age-limit", "0"],
        
        // Strategy 7: Force IPv4 (sometimes IPv6 is blocked)
        vec!["--force-ipv4", "--extractor-args", "youtube:player_client=web"],
        
        // Strategy 8: Use proxy-like headers
        vec![
            "--add-header", "X-Forwarded-For:8.8.8.8",
            "--add-header", "X-Real-IP:8.8.8.8", 
            "--extractor-args", "youtube:player_client=web"
        ],
        
        // Strategy 9: Simulate older browser
        vec![
            "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/100.0.0.0 Safari/537.36",
            "--extractor-args", "youtube:player_client=web"
        ],
        
        // Strategy 10: Use minimal extraction (fastest, sometimes bypasses checks)
        vec!["--no-check-certificate", "--prefer-insecure", "--extractor-args", "youtube:player_client=web_embedded"],
        
        // Strategy 11: Try with different locale
        vec![
            "--add-header", "Accept-Language:en-GB,en;q=0.9",
            "--extractor-args", "youtube:player_client=web,youtube:lang=en"
        ],
        
        // Strategy 12: Aggressive retry with backoff
        vec!["--retries", "15", "--retry-sleep", "exponential:1:5:10", "--extractor-args", "youtube:player_client=android"],
    ];

    for (i, strategy) in advanced_strategies.iter().enumerate() {
        info!("Trying advanced strategy {}: {:?}", i + 1, strategy);
        
        let mut command = Command::new(&yt_dlp_cmd[0]);
        
        // Add base yt-dlp arguments
        for arg in &yt_dlp_cmd[1..] {
            command.arg(arg);
        }
        
        // Basic arguments
        command
            .arg("--format").arg(format_arg)
            .arg("--output").arg(output_pattern)
            .arg("--no-playlist")
            .arg("--geo-bypass")
            .arg("--ignore-errors")
            .arg("--no-warnings")
            .arg("--quiet");  // Reduce output for cleaner logs
            
        // Add strategy-specific arguments
        for arg in strategy {
            command.arg(arg);
        }
        
        command.arg(&options.url);

        // Add longer timeout for difficult videos
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(120), // 2 minute timeout
            command.output()
        ).await;

        match output {
            Ok(Ok(process_output)) if process_output.status.success() => {
                info!("🎉 Advanced strategy {} SUCCEEDED!", i + 1);
                let output_dir = std::path::Path::new(&options.output_dir)
                    .canonicalize()
                    .map_err(|e| anyhow!("Failed to resolve output directory: {}", e))?;
                return find_downloaded_file(&output_dir).await;
            },
            Ok(Ok(process_output)) => {
                let error_msg = String::from_utf8_lossy(&process_output.stderr);
                warn!("Strategy {} failed: {}", i + 1, error_msg);
                
                // Add delay between attempts to avoid rate limiting
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            },
            Ok(Err(e)) => {
                warn!("Strategy {} execution error: {}", i + 1, e);
            },
            Err(_) => {
                warn!("Strategy {} timed out after 2 minutes", i + 1);
            }
        }
    }

    // Last resort: Try the nuclear option - completely different approach
    info!("🚨 Trying NUCLEAR OPTION - minimal extraction");
    try_nuclear_option(yt_dlp_cmd, options, output_pattern, format_arg).await
}

// Nuclear option: Minimal extraction with maximum compatibility
async fn try_nuclear_option(
    yt_dlp_cmd: &[String], 
    options: &ConversionOptions, 
    output_pattern: &str, 
    _format_arg: &str
) -> Result<String> {
    let mut command = Command::new(&yt_dlp_cmd[0]);
    
    // Add base yt-dlp arguments
    for arg in &yt_dlp_cmd[1..] {
        command.arg(arg);
    }
    
    // Nuclear option: Use the most compatible settings possible
    command
        .arg("--format").arg("best/worst")  // Accept any quality
        .arg("--output").arg(output_pattern)
        .arg("--no-playlist")
        .arg("--no-check-certificate")
        .arg("--prefer-insecure")
        .arg("--ignore-errors")
        .arg("--no-warnings")
        .arg("--extract-flat")  // Minimal extraction
        .arg("--skip-download")  // Just get info first
        .arg(&options.url);

    info!("🔥 NUCLEAR OPTION: Testing video accessibility");
    let test_output = command.output().await;
    
    match test_output {
        Ok(output) if output.status.success() => {
            info!("✅ Video is accessible, proceeding with nuclear download");
            
            // Now try actual download with minimal options
            let mut download_command = Command::new(&yt_dlp_cmd[0]);
            for arg in &yt_dlp_cmd[1..] {
                download_command.arg(arg);
            }
            
            download_command
                .arg("--format").arg("worst")  // Lowest quality for maximum compatibility
                .arg("--output").arg(output_pattern)
                .arg("--no-playlist")
                .arg("--no-check-certificate")
                .arg("--prefer-insecure")
                .arg("--ignore-errors")
                .arg("--extractor-args").arg("youtube:player_client=web_embedded")
                .arg(&options.url);
                
            let final_output = download_command.output().await.map_err(|e| anyhow!("Nuclear download failed: {}", e))?;
            
            if final_output.status.success() {
                info!("🎊 NUCLEAR OPTION SUCCEEDED!");
                let output_dir = std::path::Path::new(&options.output_dir)
                    .canonicalize()
                    .map_err(|e| anyhow!("Failed to resolve output directory: {}", e))?;
                return find_downloaded_file(&output_dir).await;
            }
        },
        _ => {
            let error_msg = test_output.map(|o| String::from_utf8_lossy(&o.stderr).to_string())
                .unwrap_or_else(|e| format!("Command execution failed: {}", e));
            warn!("Nuclear test failed: {}", error_msg);
        }
    }

    Err(anyhow!(
        "🚫 ALL STRATEGIES EXHAUSTED: YouTube is aggressively blocking this video.\n\
        Attempted 12 advanced strategies + nuclear option.\n\
        This might be:\n\
        1. Region-locked content\n\
        2. Age-restricted content requiring account\n\
        3. Recently uploaded content with enhanced protection\n\
        4. YouTube actively blocking your IP range\n\
        \n\
        Suggestions:\n\
        - Try a different video\n\
        - Check if video is publicly accessible in browser\n\
        - Video might be region-locked or private"
    ))
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

// Enhanced get video info function with Selenium fallback
pub async fn get_video_info(url: &str) -> Result<VideoInfo> {
    // Try traditional yt-dlp first
    match get_video_info_ytdlp(url).await {
        Ok(info) => {
            info!("Successfully extracted video info using yt-dlp");
            Ok(info)
        }
        Err(e) => {
            warn!("yt-dlp failed: {}, trying Selenium fallback", e);
            
            // Try Selenium fallback if enabled
            if should_use_selenium() {
                match get_video_info_selenium(url).await {
                    Ok(info) => {
                        info!("Successfully extracted video info using Selenium");
                        Ok(info)
                    }
                    Err(selenium_err) => {
                        error!("Both yt-dlp and Selenium failed. yt-dlp: {}, Selenium: {}", e, selenium_err);
                        Err(anyhow!("All extraction methods failed. Last error: {}", selenium_err))
                    }
                }
            } else {
                Err(e)
            }
        }
    }
}

// Original yt-dlp method
async fn get_video_info_ytdlp(url: &str) -> Result<VideoInfo> {
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

// Selenium-based fallback method
async fn get_video_info_selenium(url: &str) -> Result<VideoInfo> {
    let extractor = SeleniumExtractor::new()
        .map_err(|e| anyhow!("Failed to create Selenium extractor: {}", e))?;
    
    let selenium_info = extractor.get_video_info(url).await
        .map_err(|e| anyhow!("Selenium extraction failed: {}", e))?;
    
    // Convert SeleniumVideoInfo to VideoInfo
    Ok(VideoInfo {
        id: selenium_info.id,
        title: selenium_info.title,
        duration: selenium_info.duration,
        thumbnail: selenium_info.thumbnail,
        channel: selenium_info.channel,
        upload_date: None, // Not extracted by Selenium version
        view_count: selenium_info.view_count,
        description: None, // Not extracted by Selenium version
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
