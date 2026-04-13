/// Configuration management commands.
///
/// Handles loading, saving, and managing application settings.

use crate::commands::Config;
use serde_json;
use std::fs;
use tauri::{command, AppHandle};

/// Loads the application configuration from disk.
///
/// # Returns
/// Returns the configuration if it exists, or a default configuration if not found.
#[command]
pub async fn get_config() -> Result<Config, String> {
    let config_path = get_config_path()?;

    // If the config file doesn't exist, return a default config
    if !config_path.exists() {
        return Ok(Config {
            download_dir: dirs::download_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "./downloads".to_string()),
        });
    }

    // Read and parse the config file
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: Config = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(config)
}

/// Saves the application configuration to disk.
///
/// # Arguments
/// * `config` - The configuration object to save
///
/// # Returns
/// Returns OK if successful, or an error message if the save fails.
#[command]
pub async fn save_config(config: Config) -> Result<(), String> {
    let config_path = get_config_path()?;

    // Create the config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Serialize and write the config to file
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, config_json)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Opens a native folder picker dialog and returns the selected directory path.
///
/// Uses the Tauri dialog plugin to show a native file browser.
///
/// # Returns
/// Returns the selected directory path as a string, or an error if the dialog fails.
#[command]
pub async fn select_download_dir(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .pick_folder(move |path| {
            let _ = tx.send(path);
        });
    let path = rx
        .await
        .map_err(|e| format!("Failed to open folder picker: {}", e))?
        .ok_or_else(|| "No folder selected".to_string())?;

    let path_buf = path.into_path().map_err(|e| format!("Invalid path: {}", e))?;
    Ok(path_buf.to_string_lossy().to_string())
}

/// Helper function to get the configuration file path.
///
/// The configuration is stored at: `~/.config/voyage-dl/config.json` (Unix-like)
/// or the equivalent Windows config directory.
fn get_config_path() -> Result<std::path::PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Failed to determine config directory".to_string())?;

    Ok(config_dir.join("voyage-dl").join("config.json"))
}
