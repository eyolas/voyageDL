#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use voyage_dl::commands::{
    config::*, youtube::*, deezer::*, download::*
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // Config commands
            get_config,
            save_config,
            select_download_dir,

            // YouTube commands
            fetch_youtube_info,

            // Deezer commands
            fetch_deezer_playlist,

            // Download commands
            download_tracks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
