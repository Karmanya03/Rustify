// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::{AppState, *};
use tauri::Manager;

fn main() {
    // Setup logging
    tracing_subscriber::fmt::init();
    
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_video_info,
            get_quality_options,
            convert_video,
            convert_playlist,
            get_conversion_progress,
            cancel_conversion,
            select_output_directory,
            validate_youtube_url
        ])
        .setup(|app| {
            // Initialize application state
            app.manage(AppState::new());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
