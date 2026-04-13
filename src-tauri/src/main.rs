#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use voyage_dl::commands::{
    analyze::*, cache::*, config::*, youtube::*, deezer::*, download::*
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AnalyzeState::new())
        .manage(DownloadState::new())
        .manage(FetchCache::new())
        .invoke_handler(tauri::generate_handler![
            // Config commands
            get_config,
            save_config,
            select_download_dir,

            // Analyze commands
            cancel_analyze,
            toggle_pause_analyze,

            // YouTube commands
            fetch_youtube_info,

            // Deezer commands
            fetch_deezer_playlist,

            // Cache commands
            clear_youtube_cache,
            clear_deezer_cache,

            // Download commands
            download_tracks,
            cancel_downloads,
            skip_track,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
