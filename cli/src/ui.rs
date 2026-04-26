use indicatif::{ProgressBar, ProgressStyle};
use rustify_core::{DependencyStatus, QualityOptions, VideoInfo};

pub fn create_progress_bar(total: u64) -> ProgressBar {
    let progress_bar = ProgressBar::new(total);
    let style = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>3}% {msg}",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("=>-");
    progress_bar.set_style(style);
    progress_bar
}

pub fn display_video_info(info: &VideoInfo) {
    println!("Title      : {}", info.title);
    println!("Uploader   : {}", info.uploader);
    println!("Duration   : {}", crate::format_duration(info.duration));
    println!("Video ID   : {}", info.id);
    if let Some(view_count) = info.view_count {
        println!("Views      : {}", view_count);
    }
    if !info.upload_date.is_empty() {
        println!("Upload Date: {}", info.upload_date);
    }
    println!("Formats    : {}", info.formats.len());
}

pub fn display_quality_options(qualities: &QualityOptions, audio_only: bool, video_only: bool) {
    if !video_only {
        println!("Audio:");
        if qualities.audio_qualities.is_empty() {
            println!("  No audio-only sources reported by yt-dlp");
        } else {
            for quality in &qualities.audio_qualities {
                let sample_rate = quality
                    .sample_rate
                    .map(|value| format!("{value} Hz"))
                    .unwrap_or_else(|| "source".to_string());
                println!(
                    "  {} kbps {:<6} {:<12} {}",
                    quality.bitrate, quality.format, quality.codec, sample_rate
                );
            }
        }
    }

    if !audio_only {
        println!("Video:");
        if qualities.video_qualities.is_empty() {
            println!("  No video formats reported by yt-dlp");
        } else {
            for quality in &qualities.video_qualities {
                let fps = quality
                    .fps
                    .map(|value| format!("{value:.0} fps"))
                    .unwrap_or_else(|| "source".to_string());
                println!(
                    "  {:<10} {:>4}x{:<4} {:<12} {}",
                    quality.resolution, quality.width, quality.height, quality.codec, fps
                );
            }
        }
    }
}

pub fn display_dependency_status(status: &DependencyStatus) {
    println!("Auth Strategy: {}", status.auth_strategy);
    print_tool("yt-dlp", &status.yt_dlp);
    print_tool("ffmpeg", &status.ffmpeg);
}

fn print_tool(name: &str, status: &rustify_core::ToolStatus) {
    if status.available {
        println!(
            "{}: OK{}{}",
            name,
            status
                .version
                .as_ref()
                .map(|version| format!(" ({version})"))
                .unwrap_or_default(),
            status
                .command
                .as_ref()
                .map(|command| format!(" via {command}"))
                .unwrap_or_default()
        );
    } else {
        println!(
            "{}: Missing{}",
            name,
            status
                .message
                .as_ref()
                .map(|message| format!(" - {message}"))
                .unwrap_or_default()
        );
    }
}
