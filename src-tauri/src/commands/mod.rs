/// Command handlers for Tauri events.
///
/// This module organizes all Tauri commands into submodules for better maintainability.

pub mod analyze;
pub mod cache;
pub mod config;
pub mod youtube;
pub mod deezer;
pub mod download;

use serde::{Deserialize, Serialize};

/// Represents a track to be downloaded.
///
/// This structure contains all information needed to identify and download a single track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub url: String,
    pub thumbnail_url: String,
    pub duration_seconds: u32,

    /// Album name (from Deezer)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,

    /// Album cover URL (from Deezer)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_cover_url: Option<String>,

    /// Track number in the album (from Deezer)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_number: Option<u32>,

    /// Release year (from Deezer)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
}

/// Response structure for download operations.
///
/// Contains a summary of the download operation results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSummary {
    /// Number of successfully downloaded tracks
    pub successful: usize,

    /// Number of tracks that failed to download
    pub failed: usize,

    /// List of error messages encountered
    pub errors: Vec<String>,
}

/// Configuration structure for the application.
///
/// Stores user preferences and credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Directory where downloads are saved
    pub download_dir: String,

    /// Audio format: "mp3" or "m4a"
    #[serde(default = "default_audio_format")]
    pub audio_format: String,
}

fn default_audio_format() -> String {
    "mp3".to_string()
}
