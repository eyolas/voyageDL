/// Deezer playlist fetching commands.
///
/// Handles extracting track information from Deezer playlists via the public API
/// and searching for those tracks on YouTube.

use crate::commands::cache::FetchCache;
use crate::commands::TrackInfo;
use crate::utils::sidecar::{find_sidecar, run_sidecar_command_async};
use reqwest::Client;
use serde::Deserialize;
use tauri::{command, AppHandle, Emitter, Manager, State};

/// Progress event emitted during Deezer playlist analysis.
#[derive(Debug, Clone, serde::Serialize)]
struct AnalyzeProgress {
    current: usize,
    total: usize,
    track_title: String,
    artist: String,
    status: String,
}

/// Dev-only logging macro. Compiles to nothing in release builds.
macro_rules! dev_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!("[deezer] {}", format!($($arg)*));
    };
}

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
    #[allow(dead_code)]
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
pub async fn fetch_deezer_playlist(
    app: AppHandle,
    cache: State<'_, FetchCache>,
    url: String,
) -> Result<Vec<TrackInfo>, String> {
    // Check cache first
    if let Some(cached) = cache.get(&url) {
        dev_log!("Cache hit pour {} ({} pistes)", url, cached.len());
        return Ok(cached);
    }
    dev_log!("Cache miss, demarrage du fetch pour {}", url);

    let client = Client::new();

    let playlist_id = extract_playlist_id(&url)?;
    dev_log!("Extracted playlist ID: {}", playlist_id);

    // Fetch first page (embedded in playlist response)
    let playlist_url = format!("https://api.deezer.com/playlist/{}", playlist_id);
    dev_log!("Calling Deezer API: {}", playlist_url);

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

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Deezer response body: {}", e))?;

    dev_log!("API response received ({} bytes)", body.len());

    let playlist: DeezerPlaylistResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Deezer playlist response: {} (at byte {})", e, e.column()))?;

    let mut deezer_tracks = playlist.tracks.data;
    let mut next_url = playlist.tracks.next;
    dev_log!("First page: {} tracks loaded", deezer_tracks.len());

    // Handle pagination
    let mut page_num = 1;
    while let Some(ref url) = next_url {
        page_num += 1;
        dev_log!("Pagination: loading page {}...", page_num);

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch next page of tracks: {}", e))?;

        let page_body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read tracks page body: {}", e))?;

        let page: DeezerTracksPage = serde_json::from_str(&page_body)
            .map_err(|e| format!("Failed to parse tracks page: {}", e))?;

        dev_log!("Page {}: {} additional tracks", page_num, page.data.len());
        deezer_tracks.extend(page.data);
        next_url = page.next;
    }

    if deezer_tracks.is_empty() {
        return Err("Playlist is empty".to_string());
    }

    dev_log!("Deezer total: {} tracks. Starting YouTube search...", deezer_tracks.len());

    // Find yt-dlp for YouTube search
    let yt_dlp_path = find_sidecar("yt-dlp").map_err(|e| {
        format!(
            "yt-dlp not found. Make sure it's installed or bundled with the app: {}",
            e
        )
    })?;

    let mut tracks = Vec::new();
    let total = deezer_tracks.len();

    for (idx, deezer_track) in deezer_tracks.iter().enumerate() {
        let search_query = format!(
            "ytsearch1:{} {}",
            deezer_track.artist.name, deezer_track.title
        );

        dev_log!(
            "[{}/{}] YT search: {} - {}",
            idx + 1, total, deezer_track.artist.name, deezer_track.title
        );

        // Emit progress to frontend via the webview window
        let progress = AnalyzeProgress {
            current: idx + 1,
            total,
            track_title: deezer_track.title.clone(),
            artist: deezer_track.artist.name.clone(),
            status: "searching".to_string(),
        };
        if let Some(window) = app.get_webview_window("main") {
            if let Err(e) = window.emit("analyze-progress", &progress) {
                dev_log!("ERREUR emit analyze-progress: {}", e);
            }
        } else {
            dev_log!("ERREUR: fenetre 'main' introuvable");
        }

        // Yield to let the event loop deliver the event to the webview
        tokio::task::yield_now().await;

        let yt_args = vec!["--dump-json".to_string(), search_query];

        match run_sidecar_command_async(&yt_dlp_path, &yt_args).await {
            Ok(yt_json_str) => {
                if let Ok(yt_json) = serde_json::from_str::<serde_json::Value>(&yt_json_str) {
                    if let Some(yt_id) = yt_json.get("id").and_then(|v| v.as_str()) {
                        dev_log!("[{}/{}] Found: https://youtube.com/watch?v={}", idx + 1, total, yt_id);
                        let track_info = TrackInfo {
                            id: yt_id.to_string(),
                            title: deezer_track.title.clone(),
                            artist: deezer_track.artist.name.clone(),
                            url: format!("https://www.youtube.com/watch?v={}", yt_id),
                            thumbnail_url: yt_json
                                .get("thumbnail")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            duration_seconds: deezer_track.duration,
                        };
                        tracks.push(track_info);
                    } else {
                        dev_log!("[{}/{}] No YouTube ID in response", idx + 1, total);
                    }
                } else {
                    dev_log!("[{}/{}] YouTube JSON parsing error", idx + 1, total);
                }
            }
            Err(e) => {
                dev_log!("[{}/{}] ERREUR recherche: {}", idx + 1, total, e);
            }
        }
    }

    dev_log!("Search done: {}/{} tracks found on YouTube", tracks.len(), total);

    if tracks.is_empty() {
        return Err(
            "No tracks found or could not search YouTube for any of them".to_string(),
        );
    }

    // Store in cache
    cache.set(url, tracks.clone());
    dev_log!("Resultats sauvegardes dans le cache");

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
