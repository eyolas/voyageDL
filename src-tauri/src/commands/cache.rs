/// Persistent disk cache for URL fetch results.
///
/// Two levels of caching:
/// - URL-level: caches complete results for a URL (YouTube videos/playlists)
/// - Track-level: caches individual YT search results by search query (for Deezer)
///
/// Survives app restarts and crashes.

use crate::commands::TrackInfo;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{command, State};

pub struct FetchCache {
    /// In-memory layer (avoids repeated disk reads within the same session).
    memory: Mutex<HashMap<String, Vec<TrackInfo>>>,
    /// Directory where cache files are stored.
    cache_dir: PathBuf,
    /// Directory for per-track YT search results.
    tracks_dir: PathBuf,
}

impl FetchCache {
    pub fn new() -> Self {
        let base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("voyage-dl")
            .join("cache");

        let tracks_dir = base.join("tracks");

        let _ = fs::create_dir_all(&base);
        let _ = fs::create_dir_all(&tracks_dir);

        Self {
            memory: Mutex::new(HashMap::new()),
            cache_dir: base,
            tracks_dir,
        }
    }

    /// Returns cached tracks for the given URL, if any.
    /// Checks memory first, then disk.
    pub fn get(&self, url: &str) -> Option<Vec<TrackInfo>> {
        if let Some(tracks) = self.memory.lock().unwrap().get(url) {
            return Some(tracks.clone());
        }

        let path = self.cache_path(url);
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(tracks) = serde_json::from_str::<Vec<TrackInfo>>(&contents) {
                    self.memory.lock().unwrap().insert(url.to_string(), tracks.clone());
                    return Some(tracks);
                }
            }
        }

        None
    }

    /// Stores tracks for the given URL (memory + disk).
    pub fn set(&self, url: String, tracks: Vec<TrackInfo>) {
        let path = self.cache_path(&url);
        if let Ok(json) = serde_json::to_string(&tracks) {
            let _ = fs::write(&path, json);
        }
        self.memory.lock().unwrap().insert(url, tracks);
    }

    /// Returns a cached single-track YT search result by search query.
    pub fn get_track(&self, search_query: &str) -> Option<TrackInfo> {
        let path = self.track_cache_path(search_query);
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(track) = serde_json::from_str::<TrackInfo>(&contents) {
                    return Some(track);
                }
            }
        }
        None
    }

    /// Stores a single-track YT search result by search query.
    pub fn set_track(&self, search_query: &str, track: &TrackInfo) {
        let path = self.track_cache_path(search_query);
        if let Ok(json) = serde_json::to_string(track) {
            let _ = fs::write(&path, json);
        }
    }

    fn cache_path(&self, url: &str) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();
        self.cache_dir.join(format!("{:x}.json", hash))
    }

    /// Clears the URL-level cache (YouTube playlists/videos).
    pub fn clear_url_cache(&self) -> usize {
        self.memory.lock().unwrap().clear();
        clear_json_files(&self.cache_dir)
    }

    /// Clears the per-track cache (Deezer YT search results).
    pub fn clear_track_cache(&self) -> usize {
        clear_json_files(&self.tracks_dir)
    }

    fn track_cache_path(&self, query: &str) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        let hash = hasher.finish();
        self.tracks_dir.join(format!("{:x}.json", hash))
    }
}

/// Deletes all .json files in a directory. Returns the number of files deleted.
fn clear_json_files(dir: &PathBuf) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if fs::remove_file(&path).is_ok() {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Clears the YouTube cache (URL-level).
#[command]
pub async fn clear_youtube_cache(cache: State<'_, FetchCache>) -> Result<usize, String> {
    let count = cache.clear_url_cache();
    Ok(count)
}

/// Clears the Deezer cache (per-track YT search results).
#[command]
pub async fn clear_deezer_cache(cache: State<'_, FetchCache>) -> Result<usize, String> {
    let count = cache.clear_track_cache();
    Ok(count)
}
