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
    /// Unique identifier for the track (e.g., YouTube video ID)
    pub id: String,

    /// Title of the track
    pub title: String,

    /// Artist name (uploader for YouTube, artist for Spotify)
    pub artist: String,

    /// URL to the source (YouTube URL)
    pub url: String,

    /// Thumbnail URL for the track
    pub thumbnail_url: String,

    /// Duration in seconds
    pub duration_seconds: u32,
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

}
