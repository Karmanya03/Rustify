fn main() {
    // Skip icon embedding to avoid ICO requirement
    std::env::set_var("TAURI_SKIP_EMBEDDED_SERVER", "false");
    tauri_build::build()
}
