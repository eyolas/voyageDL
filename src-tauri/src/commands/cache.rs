/// Persistent disk cache for URL fetch results.
///
/// Stores TrackInfo results as JSON files keyed by URL hash.
/// Survives app restarts and crashes.

use crate::commands::TrackInfo;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::HashMap;

pub struct FetchCache {
    /// In-memory layer (avoids repeated disk reads within the same session).
    memory: Mutex<HashMap<String, Vec<TrackInfo>>>,
    /// Directory where cache files are stored.
    cache_dir: PathBuf,
}

impl FetchCache {
    pub fn new() -> Self {
        let cache_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("voyage-dl")
            .join("cache");

        // Create cache directory if needed
        let _ = fs::create_dir_all(&cache_dir);

        Self {
            memory: Mutex::new(HashMap::new()),
            cache_dir,
        }
    }

    /// Returns cached tracks for the given URL, if any.
    /// Checks memory first, then disk.
    pub fn get(&self, url: &str) -> Option<Vec<TrackInfo>> {
        // Check memory cache
        if let Some(tracks) = self.memory.lock().unwrap().get(url) {
            return Some(tracks.clone());
        }

        // Check disk cache
        let path = self.cache_path(url);
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(tracks) = serde_json::from_str::<Vec<TrackInfo>>(&contents) {
                    // Populate memory cache
                    self.memory.lock().unwrap().insert(url.to_string(), tracks.clone());
                    return Some(tracks);
                }
            }
        }

        None
    }

    /// Stores tracks for the given URL (memory + disk).
    pub fn set(&self, url: String, tracks: Vec<TrackInfo>) {
        // Write to disk (best effort)
        let path = self.cache_path(&url);
        if let Ok(json) = serde_json::to_string(&tracks) {
            let _ = fs::write(&path, json);
        }

        // Write to memory
        self.memory.lock().unwrap().insert(url, tracks);
    }

    /// Returns the file path for a given URL's cache entry.
    fn cache_path(&self, url: &str) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();
        self.cache_dir.join(format!("{:x}.json", hash))
    }
}
