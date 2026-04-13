/// Deezer playlist fetching commands.
///
/// Handles extracting track information from Deezer playlists via the public API
/// and searching for those tracks on YouTube.

use crate::commands::analyze::AnalyzeState;
use crate::commands::cache::FetchCache;
use crate::commands::TrackInfo;
use crate::utils::sidecar::{find_sidecar, spawn_sidecar};
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
    #[serde(default)]
    album: Option<DeezerAlbum>,
}

#[derive(Debug, Deserialize)]
struct DeezerArtist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct DeezerAlbum {
    title: String,
    #[serde(default)]
    cover_big: Option<String>,
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
    analyze_state: State<'_, AnalyzeState>,
    cache: State<'_, FetchCache>,
    url: String,
) -> Result<Vec<TrackInfo>, String> {
    analyze_state.reset();
    dev_log!("Starting fetch for {}", url);

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
    #[allow(unused_variables, unused_mut, unused_assignments)]
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
        // Check cancel
        if analyze_state.is_cancelled() {
            dev_log!("Analysis cancelled at track {}/{}", idx + 1, total);
            break;
        }

        // Wait while paused
        while analyze_state.is_paused() {
            if analyze_state.is_cancelled() {
                break;
            }
            let progress = AnalyzeProgress {
                current: idx + 1,
                total,
                track_title: deezer_track.title.clone(),
                artist: deezer_track.artist.name.clone(),
                status: "paused".to_string(),
            };
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("analyze-progress", &progress);
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        let search_query = format!(
            "ytsearch1:{} {}",
            deezer_track.artist.name, deezer_track.title
        );

        // Emit progress to frontend
        let progress = AnalyzeProgress {
            current: idx + 1,
            total,
            track_title: deezer_track.title.clone(),
            artist: deezer_track.artist.name.clone(),
            status: "searching".to_string(),
        };
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("analyze-progress", &progress);
        }
        tokio::task::yield_now().await;

        // Check per-track cache first
        if let Some(cached_track) = cache.get_track(&search_query) {
            dev_log!("[{}/{}] Cache hit: {} - {}", idx + 1, total, deezer_track.artist.name, deezer_track.title);
            tracks.push(cached_track);
            continue;
        }

        dev_log!(
            "[{}/{}] YT search: {} - {}",
            idx + 1, total, deezer_track.artist.name, deezer_track.title
        );

        let yt_args = vec!["--dump-json".to_string(), search_query.clone()];

        // Spawn yt-dlp with PID tracking for cancellation
        let child = spawn_sidecar(&yt_dlp_path, &yt_args)?;

        if let Some(pid) = child.id() {
            *analyze_state.current_pid.lock().unwrap() = Some(pid);
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| format!("yt-dlp process error: {}", e))?;

        *analyze_state.current_pid.lock().unwrap() = None;

        // Check if cancelled during yt-dlp
        if analyze_state.is_cancelled() {
            dev_log!("Analysis cancelled during search for track {}/{}", idx + 1, total);
            break;
        }

        if output.status.success() {
            let yt_json_str = String::from_utf8_lossy(&output.stdout);
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
                        album: deezer_track.album.as_ref().map(|a| a.title.clone()),
                        album_cover_url: deezer_track.album.as_ref().and_then(|a| a.cover_big.clone()),
                        track_number: Some((idx + 1) as u32),
                        year: None,
                    };

                    cache.set_track(&search_query, &track_info);
                    tracks.push(track_info);
                } else {
                    dev_log!("[{}/{}] No YouTube ID in response", idx + 1, total);
                }
            } else {
                dev_log!("[{}/{}] YouTube JSON parsing error", idx + 1, total);
            }
        } else {
            #[allow(unused_variables)]
            let stderr = String::from_utf8_lossy(&output.stderr);
            dev_log!("[{}/{}] yt-dlp ERROR: {}", idx + 1, total, stderr);
        }
    }

    dev_log!("Search done: {}/{} tracks found on YouTube", tracks.len(), total);

    if tracks.is_empty() {
        return Err(
            "No tracks found or could not search YouTube for any of them".to_string(),
        );
    }

    Ok(tracks)
}

/// Fetches a single track from Deezer and searches for it on YouTube.
#[command]
pub async fn fetch_deezer_track(
    app: AppHandle,
    analyze_state: State<'_, AnalyzeState>,
    cache: State<'_, FetchCache>,
    url: String,
) -> Result<Vec<TrackInfo>, String> {
    analyze_state.reset();
    dev_log!("Starting track fetch for {}", url);

    let track_id = extract_track_id(&url)?;

    let client = Client::new();
    let api_url = format!("https://api.deezer.com/track/{}", track_id);
    dev_log!("Calling Deezer API: {}", api_url);

    let response = client
        .get(&api_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Deezer track: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch track (status {})", response.status()));
    }

    let body = response.text().await
        .map_err(|e| format!("Failed to read Deezer response: {}", e))?;

    #[derive(Deserialize)]
    struct DeezerSingleTrack {
        title: String,
        #[serde(default)]
        duration: u32,
        artist: DeezerArtist,
        #[serde(default)]
        album: Option<DeezerAlbum>,
    }

    let deezer_track: DeezerSingleTrack = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Deezer track: {}", e))?;

    let search_query = format!("ytsearch1:{} {}", deezer_track.artist.name, deezer_track.title);

    // Check per-track cache
    if let Some(cached) = cache.get_track(&search_query) {
        dev_log!("Cache hit: {} - {}", deezer_track.artist.name, deezer_track.title);
        return Ok(vec![cached]);
    }

    dev_log!("YT search: {} - {}", deezer_track.artist.name, deezer_track.title);

    // Emit progress
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("analyze-progress", &AnalyzeProgress {
            current: 1,
            total: 1,
            track_title: deezer_track.title.clone(),
            artist: deezer_track.artist.name.clone(),
            status: "searching".to_string(),
        });
    }

    let yt_dlp_path = find_sidecar("yt-dlp").map_err(|e| format!("yt-dlp not found: {}", e))?;
    let yt_args = vec!["--dump-json".to_string(), search_query.clone()];

    let child = spawn_sidecar(&yt_dlp_path, &yt_args)?;
    if let Some(pid) = child.id() {
        *analyze_state.current_pid.lock().unwrap() = Some(pid);
    }

    let output = child.wait_with_output().await
        .map_err(|e| format!("yt-dlp process error: {}", e))?;
    *analyze_state.current_pid.lock().unwrap() = None;

    if analyze_state.is_cancelled() {
        return Ok(vec![]);
    }

    if !output.status.success() {
        return Err(format!("Failed to find track on YouTube"));
    }

    let yt_json_str = String::from_utf8_lossy(&output.stdout);
    let yt_json: serde_json::Value = serde_json::from_str(&yt_json_str)
        .map_err(|_| "Failed to parse YouTube response".to_string())?;

    let yt_id = yt_json.get("id").and_then(|v| v.as_str())
        .ok_or_else(|| "No YouTube video found".to_string())?;

    let track_info = TrackInfo {
        id: yt_id.to_string(),
        title: deezer_track.title,
        artist: deezer_track.artist.name,
        url: format!("https://www.youtube.com/watch?v={}", yt_id),
        thumbnail_url: yt_json.get("thumbnail").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        duration_seconds: deezer_track.duration,
        album: deezer_track.album.as_ref().map(|a| a.title.clone()),
        album_cover_url: deezer_track.album.as_ref().and_then(|a| a.cover_big.clone()),
        track_number: Some(1),
        year: None,
    };

    cache.set_track(&search_query, &track_info);
    Ok(vec![track_info])
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

/// Extracts the track ID from a Deezer track URL.
fn extract_track_id(url: &str) -> Result<String, String> {
    if let Some(track_id) = url
        .split("track/")
        .nth(1)
        .and_then(|s| s.split('?').next())
        .and_then(|s| s.split('#').next())
    {
        if !track_id.is_empty() {
            return Ok(track_id.to_string());
        }
    }

    Err("Could not extract track ID from URL. Make sure it's a valid Deezer track URL.".to_string())
}
