/// Deezer playlist fetching commands.
///
/// Handles extracting track information from Deezer playlists via the public API
/// and searching for those tracks on YouTube.

use crate::commands::TrackInfo;
use crate::utils::sidecar::{find_sidecar, run_sidecar_command};
use reqwest::Client;
use serde::Deserialize;
use tauri::command;

#[derive(Debug, Deserialize)]
struct DeezerPlaylistResponse {
    tracks: DeezerTracksData,
}

#[derive(Debug, Deserialize)]
struct DeezerTracksData {
    data: Vec<DeezerTrack>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeezerTracksPage {
    data: Vec<DeezerTrack>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeezerTrack {
    id: u64,
    title: String,
    #[serde(default)]
    duration: u32,
    artist: DeezerArtist,
}

#[derive(Debug, Deserialize)]
struct DeezerArtist {
    name: String,
}

/// Fetches all tracks from a Deezer playlist and searches for them on YouTube.
///
/// This command:
/// 1. Extracts the playlist ID from the URL
/// 2. Fetches all tracks from the playlist via the public Deezer API (handles pagination)
/// 3. For each track, searches YouTube and retrieves the video URL
/// 4. Returns a list of TrackInfo objects ready for download
#[command]
pub async fn fetch_deezer_playlist(url: String) -> Result<Vec<TrackInfo>, String> {
    let client = Client::new();

    let playlist_id = extract_playlist_id(&url)?;

    // Fetch first page (embedded in playlist response)
    let playlist_url = format!("https://api.deezer.com/playlist/{}", playlist_id);
    let response = client
        .get(&playlist_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Deezer playlist: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch playlist (status {}): Make sure the playlist ID is correct",
            response.status()
        ));
    }

    let playlist: DeezerPlaylistResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Deezer playlist response: {}", e))?;

    let mut deezer_tracks = playlist.tracks.data;
    let mut next_url = playlist.tracks.next;

    // Handle pagination
    while let Some(ref url) = next_url {
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch next page of tracks: {}", e))?;

        let page: DeezerTracksPage = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse tracks page: {}", e))?;

        deezer_tracks.extend(page.data);
        next_url = page.next;
    }

    if deezer_tracks.is_empty() {
        return Err("Playlist is empty".to_string());
    }

    // Find yt-dlp for YouTube search
    let yt_dlp_path = find_sidecar("yt-dlp").map_err(|e| {
        format!(
            "yt-dlp not found. Make sure it's installed or bundled with the app: {}",
            e
        )
    })?;

    let mut tracks = Vec::new();

    for deezer_track in deezer_tracks {
        let search_query = format!(
            "ytsearch1:{} {}",
            deezer_track.artist.name, deezer_track.title
        );

        let yt_args = vec!["--dump-json".to_string(), search_query];

        match run_sidecar_command(&yt_dlp_path, &yt_args) {
            Ok(yt_json_str) => {
                if let Ok(yt_json) = serde_json::from_str::<serde_json::Value>(&yt_json_str) {
                    if let Some(yt_id) = yt_json.get("id").and_then(|v| v.as_str()) {
                        let track_info = TrackInfo {
                            id: yt_id.to_string(),
                            title: deezer_track.title,
                            artist: deezer_track.artist.name,
                            url: format!("https://www.youtube.com/watch?v={}", yt_id),
                            thumbnail_url: yt_json
                                .get("thumbnail")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            duration_seconds: deezer_track.duration,
                        };
                        tracks.push(track_info);
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to search for track '{}': {}",
                    deezer_track.title, e
                );
            }
        }
    }

    if tracks.is_empty() {
        return Err(
            "No tracks found or could not search YouTube for any of them".to_string(),
        );
    }

    Ok(tracks)
}

/// Extracts the playlist ID from a Deezer playlist URL.
///
/// Supports formats like:
/// - https://www.deezer.com/playlist/1234567890
/// - https://www.deezer.com/fr/playlist/1234567890
/// - https://deezer.com/playlist/1234567890
fn extract_playlist_id(url: &str) -> Result<String, String> {
    if let Some(playlist_id) = url
        .split("playlist/")
        .nth(1)
        .and_then(|s| s.split('?').next())
        .and_then(|s| s.split('#').next())
    {
        if !playlist_id.is_empty() {
            return Ok(playlist_id.to_string());
        }
    }

    Err("Could not extract playlist ID from URL. Make sure it's a valid Deezer playlist URL."
        .to_string())
}
