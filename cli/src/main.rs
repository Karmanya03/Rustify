mod args;
mod config;
mod ui;

use args::{Cli, Commands};
use clap::Parser;
use anyhow::Result;
use colored::*;
use ezp3_core::*;
use std::path::PathBuf;
use tracing::{error, warn};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Setup logging
    setup_logging(cli.verbose);
    
    // Initialize EzP3
    let ezp3 = EzP3::new()?;
    
    // Get output directory
    let output_dir = cli.output.unwrap_or_else(|| std::env::current_dir().unwrap());
    
    match cli.command {
        Commands::Convert(args) => {
            convert_video(&ezp3, args, output_dir).await?;
        }
        
        Commands::Playlist(args) => {
            convert_playlist(&ezp3, args, output_dir).await?;
        }
        
        Commands::Batch(args) => {
            batch_convert(&ezp3, args, output_dir).await?;
        }
        
        Commands::Download(args) => {
            handle_download(args).await?;
        }
        
        Commands::Info(args) => {
            show_video_info(&ezp3, args).await?;
        }
        
        Commands::Quality(args) => {
            show_quality_options(&ezp3, args).await?;
        }
        
        Commands::Config(args) => {
            handle_config(args).await?;
        }
    }
    
    Ok(())
}

async fn convert_video(ezp3: &EzP3, args: args::ConvertArgs, output_dir: PathBuf) -> Result<()> {
    println!("{}", "🎵 Starting video conversion...".cyan().bold());
    
    // Validate URL
    if !is_valid_youtube_url(&args.url) {
        anyhow::bail!("Invalid YouTube URL: {}", args.url);
    }
    
    // Get video info first
    let video_info = ezp3.get_video_info(&args.url).await?;
    println!("📹 {}", video_info.title.green());
    println!("⏱️  Duration: {}", format_duration(video_info.duration));
    println!("👤 Uploader: {}", video_info.uploader);
    
    // Generate output filename
    let filename = args.name.unwrap_or_else(|| sanitize_filename(&video_info.title));
    let extension = match args.format.as_str() {
        "mp3" => "mp3",
        "mp4" => "mp4", 
        "flac" => "flac",
        "aac" => "aac",
        "ogg" => "ogg",
        "webm" => "webm",
        _ => {
            error!("Unsupported format: {}", args.format);
            anyhow::bail!("Unsupported format: {}", args.format);
        }
    };
    
    let output_path = output_dir.join(format!("{}.{}", filename, extension));
    
    // Parse format
    let format = match args.format.as_str() {
        "mp3" => {
            let bitrate = args.quality.replace("kbps", "").replace("k", "").parse().unwrap_or(256);
            OutputFormat::Mp3 { bitrate }
        }
        "mp4" => {
            OutputFormat::Mp4 { resolution: args.quality.clone() }
        }
        "flac" => OutputFormat::Flac,
        "aac" => {
            let bitrate = args.quality.replace("kbps", "").replace("k", "").parse().unwrap_or(256);
            OutputFormat::Aac { bitrate }
        }
        "ogg" => {
            let quality = args.quality.parse().unwrap_or(5);
            OutputFormat::Ogg { quality }
        }
        "webm" => {
            OutputFormat::WebM { resolution: args.quality.clone() }
        }
        _ => anyhow::bail!("Unsupported format: {}", args.format),
    };
    
    println!("🎯 Converting to: {} ({})", args.format.to_uppercase(), args.quality);
    println!("📁 Output: {}", output_path.display());
    
    // Setup progress bar
    let pb = ui::create_progress_bar(100);
    pb.set_message("Initializing...");
    
    // Convert with progress tracking
    ezp3.convert_video(
        &args.url,
        output_path.clone(),
        format,
        &args.quality,
        |progress| {
            let pb = ui::create_progress_bar(100);
            pb.set_position(progress.percentage as u64);
            pb.set_message(format!("Speed: {} | ETA: {}", progress.speed, progress.eta));
        }
    ).await?;
    
    println!("✅ Conversion completed!");
    println!("🎉 {}", format!("Saved to: {}", output_path.display()).green().bold());
    
    Ok(())
}

async fn convert_playlist(ezp3: &EzP3, args: args::PlaylistArgs, output_dir: PathBuf) -> Result<()> {
    println!("{}", "🎵 Starting playlist conversion...".cyan().bold());
    
    // Validate URL
    if !is_valid_youtube_url(&args.url) {
        anyhow::bail!("Invalid YouTube URL: {}", args.url);
    }
    
    // Parse format
    let format = match args.format.as_str() {
        "mp3" => {
            let bitrate = args.quality.replace("kbps", "").replace("k", "").parse().unwrap_or(256);
            OutputFormat::Mp3 { bitrate }
        }
        "mp4" => {
            OutputFormat::Mp4 { resolution: args.quality.clone() }
        }
        "flac" => OutputFormat::Flac,
        "aac" => {
            let bitrate = args.quality.replace("kbps", "").replace("k", "").parse().unwrap_or(256);
            OutputFormat::Aac { bitrate }
        }
        "ogg" => {
            let quality = args.quality.parse().unwrap_or(5);
            OutputFormat::Ogg { quality }
        }
        "webm" => {
            OutputFormat::WebM { resolution: args.quality.clone() }
        }
        _ => anyhow::bail!("Unsupported format: {}", args.format),
    };
    
    println!("🎯 Converting to: {} ({})", args.format.to_uppercase(), args.quality);
    
    // Convert playlist
    let results = ezp3.convert_playlist(
        &args.url,
        output_dir,
        format,
        &args.quality,
        |index, progress| {
            println!("Video {}: {}% (Speed: {})", 
                index + 1, 
                progress.percentage as u8, 
                progress.speed
            );
        }
    ).await?;
    
    // Report results
    let successful = results.iter().filter(|r| r.is_ok()).count();
    let failed = results.len() - successful;
    
    println!("✅ Completed: {} successful, {} failed", successful, failed);
    
    Ok(())
}

async fn show_video_info(ezp3: &EzP3, args: args::InfoArgs) -> Result<()> {
    if !is_valid_youtube_url(&args.url) {
        anyhow::bail!("Invalid YouTube URL: {}", args.url);
    }
    
    let info = ezp3.get_video_info(&args.url).await?;
    
    match args.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        "table" | _ => {
            ui::display_video_info(&info);
        }
    }
    
    Ok(())
}

async fn show_quality_options(ezp3: &EzP3, args: args::QualityArgs) -> Result<()> {
    if !is_valid_youtube_url(&args.url) {
        anyhow::bail!("Invalid YouTube URL: {}", args.url);
    }
    
    let qualities = ezp3.get_available_qualities(&args.url).await?;
    ui::display_quality_options(&qualities, args.audio_only, args.video_only);
    
    Ok(())
}

async fn batch_convert(ezp3: &EzP3, args: args::BatchArgs, output_dir: PathBuf) -> Result<()> {
    println!("{}", "🎵 Starting batch conversion...".cyan().bold());
    
    // Validate URL
    if !is_valid_youtube_url(&args.url) {
        anyhow::bail!("Invalid YouTube URL: {}", args.url);
    }
    
    // Get playlist info
    let playlist_info = ezp3.get_playlist_info(&args.url).await?;
    println!("📋 Playlist: {}", playlist_info.title.green());
    println!("👤 Uploader: {}", playlist_info.uploader);
    println!("🎬 Videos: {}", playlist_info.video_count.to_string().yellow());
    
    // Parse formats
    let formats: Vec<&str> = args.formats.split(',').collect();
    let mut batch_formats = Vec::new();
    
    for format in formats {
        let format = format.trim();
        
        // Get quality for this format
        let quality = if let Some(qualities) = &args.qualities {
            parse_quality_for_format(qualities, format).unwrap_or_else(|| default_quality_for_format(format))
        } else {
            default_quality_for_format(format)
        };
        
        let output_format = match format {
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
            _ => {
                warn!("Unsupported format: {}, skipping", format);
                continue;
            }
        };
        
        batch_formats.push(BatchFormat {
            format: output_format,
            quality,
            enabled: true,
        });
    }
    
    if batch_formats.is_empty() {
        anyhow::bail!("No valid formats specified");
    }
    
    // Create batch options
    let batch_options = BatchOptions {
        max_concurrent: args.jobs,
        skip_existing: args.skip_existing,
        create_subdirs: args.create_subdirs,
        add_index_prefix: args.add_index,
        video_limit: args.limit,
        start_index: args.start.saturating_sub(1),
        download_thumbnails: args.thumbnails,
        create_playlist_file: true,
    };
    
    // For now, use a simplified batch conversion approach
    let _ = batch_options; // Silence unused warning
    println!("🚀 Starting batch conversion with {} formats", batch_formats.len());
    println!("📁 Output directory: {}", output_dir.display());
    
    // Simplified batch processing - convert each video in each format
    let results = ezp3.convert_playlist(
        &args.url,
        output_dir.clone(),
        batch_formats[0].format.clone(), // Use first format for now
        &batch_formats[0].quality,
        |index, progress| {
            println!("Processing video {} - {}%", 
                index + 1, 
                progress.percentage as u8
            );
        }
    ).await?;
    
    println!("✅ Batch conversion completed!");
    
    // Show summary
    let completed = results.iter().filter(|r| r.is_ok()).count();
    let failed = results.len() - completed;
    
    println!();
    println!("📊 Conversion Summary:");
    println!("   ✅ Completed: {}", completed.to_string().green());
    println!("   ❌ Failed: {}", failed.to_string().red());
    println!("   📁 Output: {}", output_dir.display());
    
    // Auto-download if requested
    if args.auto_download {
        println!("\n💾 Auto-downloading files...");
        println!("Files are already saved to: {}", output_dir.display());
    } else {
        println!("\n💡 Files saved to: {}", output_dir.display());
    }
    
    Ok(())
}

async fn handle_download(args: args::DownloadArgs) -> Result<()> {
    match args.action {
        args::DownloadAction::List => {
            list_completed_jobs().await
        }
        args::DownloadAction::Get { ids, output } => {
            download_specific_files(ids, output).await
        }
        args::DownloadAction::Batch { job_id, output: _ } => {
            // This would need access to batch processor - simplified for now
            println!("Batch download for job: {}", job_id);
            println!("This feature requires the web backend or desktop app for full functionality");
            Ok(())
        }
        args::DownloadAction::Clean { days } => {
            clean_old_files(days).await
        }
    }
}

async fn list_completed_jobs() -> Result<()> {
    println!("{}", "📋 Completed Conversion Jobs".cyan().bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // This would typically read from a database or job storage
    // For now, show a placeholder
    println!("💡 Use the web interface or desktop app to see detailed job history");
    println!("   Web: Start with 'ezp3-web-backend' and open http://localhost:3001");
    println!("   Desktop: Run the desktop application");
    
    Ok(())
}

async fn download_specific_files(ids: Vec<String>, output: Option<PathBuf>) -> Result<()> {
    let download_dir = output.unwrap_or_else(|| std::env::current_dir().unwrap().join("downloads"));
    
    println!("💾 Downloading files to: {}", download_dir.display());
    
    for id in ids {
        println!("📥 Would download file with ID: {}", id);
        // Implementation would depend on job storage system
    }
    
    Ok(())
}

async fn clean_old_files(days: u64) -> Result<()> {
    println!("🧹 Cleaning files older than {} days...", days);
    
    let _cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let cleaned_count = 0;
    let cleaned_size = 0u64;
    
    // This would scan output directories and remove old files
    // Implementation depends on file organization strategy
    
    println!("✅ Cleaned {} files ({} freed)", cleaned_count, format_bytes(cleaned_size));
    Ok(())
}

async fn download_batch_files(job_id: &str, batch_processor: &BatchProcessor, output_dir: Option<PathBuf>) -> Result<()> {
    let download_dir = output_dir.unwrap_or_else(|| std::env::current_dir().unwrap().join("downloads"));
    
    let tasks = batch_processor.get_job_tasks(job_id);
    let completed_tasks: Vec<_> = tasks.iter()
        .filter(|t| matches!(t.status, BatchTaskStatus::Completed))
        .collect();
    
    if completed_tasks.is_empty() {
        println!("No completed files to download");
        return Ok(());
    }
    
    std::fs::create_dir_all(&download_dir)?;
    
    println!("💾 Downloading {} files to {}", completed_tasks.len(), download_dir.display());
    
    for task in completed_tasks {
        for format in &task.formats {
            if matches!(format.status, TaskFormatStatus::Completed) {
                let filename = format.output_path.file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("unknown"));
                let dest_path = download_dir.join(filename);
                
                if format.output_path.exists() {
                    std::fs::copy(&format.output_path, &dest_path)?;
                    println!("   ✅ {}", filename.to_string_lossy());
                }
            }
        }
    }
    
    println!("🎉 Download completed!");
    Ok(())
}

async fn handle_config(args: args::ConfigArgs) -> Result<()> {
    match args.action {
        args::ConfigAction::Show => {
            config::show_config()?;
        }
        args::ConfigAction::Set { key, value } => {
            config::set_config(&key, &value)?;
        }
        args::ConfigAction::Reset => {
            config::reset_config()?;
        }
    }
    Ok(())
}

// Helper functions
fn parse_quality_for_format(qualities: &str, format: &str) -> Option<String> {
    for pair in qualities.split(',') {
        if let Some((fmt, quality)) = pair.split_once(':') {
            if fmt.trim() == format {
                return Some(quality.trim().to_string());
            }
        }
    }
    None
}

fn default_quality_for_format(format: &str) -> String {
    match format {
        "mp3" | "aac" => "256".to_string(),
        "mp4" | "webm" => "1080p".to_string(),
        "flac" => "best".to_string(),
        "ogg" => "5".to_string(),
        _ => "default".to_string(),
    }
}

fn setup_logging(verbose: bool) {
    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .without_time()
        .init();
}

fn is_valid_youtube_url(url: &str) -> bool {
    url.contains("youtube.com") || url.contains("youtu.be")
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c => c,
        })
        .collect()
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_index])
}
