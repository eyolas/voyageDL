/// YouTube video/playlist fetching commands.
///
/// Handles extracting information from YouTube videos and playlists using yt-dlp.

use crate::commands::cache::FetchCache;
use crate::commands::TrackInfo;
use crate::utils::sidecar::{find_sidecar, run_sidecar_command};
use serde_json;
use tauri::{command, State};
use url::Url;

/// Fetches information about a YouTube video or playlist.
/// Returns cached results if the URL has been fetched before.
#[command]
pub async fn fetch_youtube_info(
    cache: State<'_, FetchCache>,
    url: String,
) -> Result<Vec<TrackInfo>, String> {
    // Check cache first
    if let Some(cached) = cache.get(&url) {
        return Ok(cached);
    }

    // Validate the URL
    Url::parse(&url).map_err(|e| format!("Invalid URL: {}", e))?;

    // Determine if this is a playlist or single video
    let is_playlist = url.contains("list=");

    // Prepare yt-dlp arguments
    let args = if is_playlist {
        vec![
            "--flat-playlist".to_string(),
            "--dump-json".to_string(),
            url.clone(),
        ]
    } else {
        vec!["--dump-json".to_string(), url.clone()]
    };

    // Find and run yt-dlp sidecar
    let yt_dlp_path = find_sidecar("yt-dlp").map_err(|e| {
        format!(
            "yt-dlp not found. Make sure it's installed or bundled with the app: {}",
            e
        )
    })?;

    let output = run_sidecar_command(&yt_dlp_path, &args)
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

    // Parse the JSON output
    let tracks = if is_playlist {
        parse_playlist_json(&output)?
    } else {
        parse_video_json(&output)?
    };

    // Store in cache
    cache.set(url, tracks.clone());

    Ok(tracks)
}

/// Parses a single video's JSON output from yt-dlp.
fn parse_video_json(json_str: &str) -> Result<Vec<TrackInfo>, String> {
    let json: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let track = TrackInfo {
        id: json.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
        title: json.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string(),
        artist: json.get("uploader").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
        url: json.get("webpage_url").and_then(|v| v.as_str())
            .or_else(|| json.get("url").and_then(|v| v.as_str()))
            .unwrap_or("").to_string(),
        thumbnail_url: json.get("thumbnail").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        duration_seconds: json.get("duration").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        album: json.get("album").and_then(|v| v.as_str()).map(|s| s.to_string()),
        album_cover_url: None,
        track_number: None,
        year: json.get("upload_date").and_then(|v| v.as_str()).map(|s| s[..4].to_string()),
    };

    Ok(vec![track])
}

/// Parses a playlist's JSON output from yt-dlp.
///
/// When using --flat-playlist, yt-dlp returns newline-delimited JSON objects.
fn parse_playlist_json(json_str: &str) -> Result<Vec<TrackInfo>, String> {
    let mut tracks = Vec::new();

    for line in json_str.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let json: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| format!("Failed to parse playlist JSON line: {}", e))?;

        let track = TrackInfo {
            id: json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            title: json
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string(),
            artist: json.get("uploader").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            url: format!(
                "https://www.youtube.com/watch?v={}",
                json.get("id").and_then(|v| v.as_str()).unwrap_or("unknown")
            ),
            thumbnail_url: json.get("thumbnail").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            duration_seconds: json.get("duration").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            album: None,
            album_cover_url: None,
            track_number: None,
            year: None,
        };

        tracks.push(track);
    }

    if tracks.is_empty() {
        return Err("No tracks found in playlist".to_string());
    }

    Ok(tracks)
}
