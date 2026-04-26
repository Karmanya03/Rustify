mod args;
mod config;
mod ui;

use anyhow::{anyhow, Result};
use args::{BatchArgs, Cli, Commands, ConvertArgs, PlaylistArgs};
use clap::Parser;
use colored::*;
use rustify_core::*;
use std::path::PathBuf;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    let app_config = config::load_config()?;
    let rustify = EzP3::with_config(app_config.clone())?;
    let output_dir = cli
        .output
        .or_else(|| app_config.download_dir.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    match cli.command {
        Commands::Convert(args) => convert_video(&rustify, args, output_dir).await?,
        Commands::Playlist(args) => convert_playlist(&rustify, args, output_dir).await?,
        Commands::Batch(args) => batch_convert(&rustify, args, output_dir).await?,
        Commands::Info(args) => show_video_info(&rustify, args).await?,
        Commands::Quality(args) => show_quality_options(&rustify, args).await?,
        Commands::Config(args) => handle_config(args)?,
        Commands::Doctor => ui::display_dependency_status(&rustify.dependency_status().await),
    }

    Ok(())
}

async fn convert_video(rustify: &EzP3, args: ConvertArgs, output_dir: PathBuf) -> Result<()> {
    ensure_youtube_url(&args.url)?;

    let info = rustify.get_video_info(&args.url).await?;
    let format = parse_output_format(&args.format, &args.quality)?;
    let extension = extension_for_format(&format);
    let filename = args
        .name
        .unwrap_or_else(|| sanitize_filename(&info.title));
    let output_path = output_dir.join(format!("{filename}.{extension}"));

    println!("{}", "Starting conversion".cyan().bold());
    println!("Title   : {}", info.title.green());
    println!("Format  : {} ({})", args.format.to_uppercase(), args.quality);
    println!("Output  : {}", output_path.display());

    let progress_bar = ui::create_progress_bar(100);
    let progress_bar_handle = progress_bar.clone();

    rustify
        .convert_video(&args.url, output_path.clone(), format, &args.quality, move |progress| {
            progress_bar_handle.set_position(progress.percentage.round() as u64);
            progress_bar_handle.set_message(format!("{} | ETA {}", progress.speed, progress.eta));
        })
        .await?;

    progress_bar.finish_with_message("Done");
    println!("{}", format!("Saved to {}", output_path.display()).green().bold());
    Ok(())
}

async fn convert_playlist(rustify: &EzP3, args: PlaylistArgs, output_dir: PathBuf) -> Result<()> {
    ensure_playlist_url(&args.url)?;
    let format = parse_output_format(&args.format, &args.quality)?;
    let playlist = rustify.get_playlist_info(&args.url).await?;
    let selected_videos = select_playlist_videos(&playlist.videos, args.start, args.limit);
    let index_width = playlist_index_width(playlist.video_count.max(playlist.videos.len()));

    println!("{}", "Starting playlist conversion".cyan().bold());
    println!("Playlist : {}", playlist.title.green());
    println!("Videos   : {}", selected_videos.len());
    println!("Format   : {} ({})", args.format.to_uppercase(), args.quality);

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (index, video) in selected_videos.iter().enumerate() {
        let safe_title = sanitize_filename(&video.title);
        let output_path = playlist_output_path(
            &output_dir,
            index + args.start,
            &safe_title,
            extension_for_format(&format),
            index_width,
        );

        let progress_bar = ui::create_progress_bar(100);
        let progress_bar_handle = progress_bar.clone();
        println!("Converting {}", video.title);

        if output_exists(&output_path).await {
            progress_bar.finish_with_message("Skipped");
            println!("{}", format!("Already exists: {}", output_path.display()).yellow());
            succeeded += 1;
            maybe_pause_between_jobs(rustify, index + 1 < selected_videos.len()).await;
            continue;
        }

        match rustify
            .convert_video(&video.url, output_path.clone(), format.clone(), &args.quality, move |progress| {
                progress_bar_handle.set_position(progress.percentage.round() as u64);
                progress_bar_handle
                    .set_message(format!("{} | ETA {}", progress.speed, progress.eta));
            })
            .await
        {
            Ok(()) => {
                progress_bar.finish_with_message("Done");
                succeeded += 1;
            }
            Err(error) => {
                progress_bar.abandon_with_message("Failed");
                eprintln!("{} {}", "Failed:".red().bold(), error);
                failed += 1;
            }
        }

        maybe_pause_between_jobs(rustify, index + 1 < selected_videos.len()).await;
    }

    println!(
        "{}",
        format!("Playlist finished: {succeeded} succeeded, {failed} failed").green()
    );
    Ok(())
}

async fn batch_convert(rustify: &EzP3, args: BatchArgs, output_dir: PathBuf) -> Result<()> {
    ensure_playlist_url(&args.url)?;
    let playlist = rustify.get_playlist_info(&args.url).await?;
    let selected_videos = select_playlist_videos(&playlist.videos, args.start, args.limit);
    let formats = parse_batch_formats(&args.formats, args.qualities.as_deref())?;
    let index_width = playlist_index_width(playlist.video_count.max(playlist.videos.len()));
    let total_jobs = selected_videos.len() * formats.len();

    println!("{}", "Starting batch conversion".cyan().bold());
    println!("Playlist : {}", playlist.title.green());
    println!("Videos   : {}", selected_videos.len());
    println!("Formats  : {}", formats.len());

    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut processed_jobs = 0usize;

    for (video_index, video) in selected_videos.iter().enumerate() {
        for (format_name, quality, format) in &formats {
            let subdir = output_dir.join(format_name);
            let safe_title = sanitize_filename(&video.title);
            let output_path = playlist_output_path(
                &subdir,
                video_index + args.start,
                &safe_title,
                extension_for_format(format),
                index_width,
            );
            let label = format!("{} -> {} ({})", video.title, format_name, quality);
            let progress_bar = ui::create_progress_bar(100);
            let progress_bar_handle = progress_bar.clone();
            println!("{label}");

            if output_exists(&output_path).await {
                progress_bar.finish_with_message("Skipped");
                println!("{}", format!("Already exists: {}", output_path.display()).yellow());
                succeeded += 1;
                processed_jobs += 1;
                maybe_pause_between_jobs(rustify, processed_jobs < total_jobs).await;
                continue;
            }

            match rustify
                .convert_video(&video.url, output_path.clone(), format.clone(), quality, move |progress| {
                    progress_bar_handle.set_position(progress.percentage.round() as u64);
                    progress_bar_handle
                        .set_message(format!("{} | ETA {}", progress.speed, progress.eta));
                })
                .await
            {
                Ok(()) => {
                    progress_bar.finish_with_message("Done");
                    succeeded += 1;
                }
                Err(error) => {
                    progress_bar.abandon_with_message("Failed");
                    eprintln!("{} {}", "Failed:".red().bold(), error);
                    failed += 1;
                }
            }

            processed_jobs += 1;
            maybe_pause_between_jobs(rustify, processed_jobs < total_jobs).await;
        }
    }

    println!(
        "{}",
        format!("Batch finished: {succeeded} succeeded, {failed} failed").green()
    );
    Ok(())
}

async fn show_video_info(rustify: &EzP3, args: args::InfoArgs) -> Result<()> {
    ensure_supported_info_url(&args.url)?;

    if is_playlist_url(&args.url) {
        let info = rustify.get_playlist_info(&args.url).await?;
        if args.format.eq_ignore_ascii_case("json") {
            println!("{}", serde_json::to_string_pretty(&info)?);
        } else {
            println!("Playlist : {}", info.title);
            println!("Uploader : {}", info.uploader);
            println!("Videos   : {}", info.video_count);
        }
        return Ok(());
    }

    let info = rustify.get_video_info(&args.url).await?;
    if args.format.eq_ignore_ascii_case("json") {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        ui::display_video_info(&info);
    }

    Ok(())
}

async fn show_quality_options(rustify: &EzP3, args: args::QualityArgs) -> Result<()> {
    ensure_youtube_url(&args.url)?;
    let qualities = rustify.get_available_qualities(&args.url).await?;
    ui::display_quality_options(&qualities, args.audio_only, args.video_only);
    Ok(())
}

fn handle_config(args: args::ConfigArgs) -> Result<()> {
    match args.action {
        args::ConfigAction::Show => config::show_config(),
        args::ConfigAction::Set { key, value } => config::set_config(&key, &value),
        args::ConfigAction::Reset => config::reset_config(),
    }
}

fn parse_output_format(format: &str, quality: &str) -> Result<OutputFormat> {
    match format.trim().to_ascii_lowercase().as_str() {
        "mp3" => Ok(OutputFormat::Mp3 {
            bitrate: parse_audio_bitrate(quality, 320)?,
        }),
        "flac" => Ok(OutputFormat::Flac),
        "wav" => Ok(OutputFormat::Wav),
        "aac" => Ok(OutputFormat::Aac {
            bitrate: parse_audio_bitrate(quality, 256)?,
        }),
        "ogg" => Ok(OutputFormat::Ogg {
            quality: quality.parse::<u8>().unwrap_or(6),
        }),
        "mp4" => Ok(OutputFormat::Mp4 {
            resolution: quality.to_string(),
        }),
        "webm" => Ok(OutputFormat::WebM {
            resolution: quality.to_string(),
        }),
        other => Err(anyhow!("Unsupported format: {other}")),
    }
}

fn parse_audio_bitrate(value: &str, default: u32) -> Result<u32> {
    let digits = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return Ok(default);
    }

    digits
        .parse::<u32>()
        .map_err(|error| anyhow!("Invalid bitrate '{}': {}", value, error))
}

fn extension_for_format(format: &OutputFormat) -> &'static str {
    match format {
        OutputFormat::Mp3 { .. } => "mp3",
        OutputFormat::Mp4 { .. } => "mp4",
        OutputFormat::Flac => "flac",
        OutputFormat::Wav => "wav",
        OutputFormat::Aac { .. } => "aac",
        OutputFormat::Ogg { .. } => "ogg",
        OutputFormat::WebM { .. } => "webm",
    }
}

fn ensure_youtube_url(url: &str) -> Result<()> {
    if is_valid_youtube_url(url) {
        Ok(())
    } else {
        Err(anyhow!("Invalid YouTube URL: {url}"))
    }
}

fn ensure_playlist_url(url: &str) -> Result<()> {
    if is_supported_playlist_url(url) {
        Ok(())
    } else {
        Err(anyhow!(
            "Invalid playlist URL: {url}. Rustify supports YouTube and Spotify playlist links."
        ))
    }
}

fn ensure_supported_info_url(url: &str) -> Result<()> {
    if is_playlist_url(url) || is_valid_youtube_url(url) {
        Ok(())
    } else {
        Err(anyhow!(
            "Invalid URL: {url}. Rustify info supports YouTube videos plus YouTube or Spotify playlists."
        ))
    }
}

fn is_playlist_url(url: &str) -> bool {
    url.contains("playlist?list=") || is_valid_spotify_playlist_url(url)
}

fn select_playlist_videos<'a>(
    videos: &'a [PlaylistVideo],
    start: usize,
    limit: Option<usize>,
) -> Vec<&'a PlaylistVideo> {
    let offset = start.saturating_sub(1);
    let mut selected = videos.iter().skip(offset).collect::<Vec<_>>();
    if let Some(limit) = limit {
        selected.truncate(limit);
    }
    selected
}

fn playlist_index_width(total_items: usize) -> usize {
    total_items.max(1).to_string().len().max(3)
}

fn playlist_output_path(
    output_dir: &std::path::Path,
    index: usize,
    safe_title: &str,
    extension: &str,
    index_width: usize,
) -> PathBuf {
    output_dir.join(format!(
        "{:0width$}-{}.{}",
        index,
        safe_title,
        extension,
        width = index_width
    ))
}

async fn output_exists(path: &std::path::Path) -> bool {
    matches!(tokio::fs::metadata(path).await, Ok(metadata) if metadata.len() > 0)
}

async fn maybe_pause_between_jobs(rustify: &EzP3, should_pause: bool) {
    if should_pause && rustify.config().rate_limits.request_delay_ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(
            rustify.config().rate_limits.request_delay_ms,
        ))
        .await;
    }
}

fn parse_batch_formats(
    formats: &str,
    qualities: Option<&str>,
) -> Result<Vec<(String, String, OutputFormat)>> {
    let quality_map = qualities.unwrap_or_default();
    let mut parsed = Vec::new();

    for format_name in formats.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        let quality = quality_for_format(quality_map, format_name)
            .unwrap_or_else(|| default_quality_for_format(format_name).to_string());
        let format = parse_output_format(format_name, &quality)?;
        parsed.push((format_name.to_string(), quality, format));
    }

    if parsed.is_empty() {
        return Err(anyhow!("No valid formats were provided"));
    }

    Ok(parsed)
}

fn quality_for_format(quality_map: &str, format_name: &str) -> Option<String> {
    for pair in quality_map.split(',') {
        let (format, quality) = pair.split_once(':')?;
        if format.trim().eq_ignore_ascii_case(format_name) {
            return Some(quality.trim().to_string());
        }
    }

    None
}

fn default_quality_for_format(format_name: &str) -> &'static str {
    match format_name.to_ascii_lowercase().as_str() {
        "mp3" => "320",
        "flac" => "lossless",
        "wav" => "lossless",
        "aac" => "256",
        "ogg" => "6",
        "mp4" => "1080p",
        "webm" => "1080p",
        _ => "320",
    }
}

fn setup_logging(verbose: bool) {
    let level = if verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .without_time()
        .init();
}
