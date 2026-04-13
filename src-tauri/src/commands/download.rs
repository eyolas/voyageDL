/// Download management commands.
///
/// Handles downloading tracks using yt-dlp and emitting progress updates to the frontend.
/// Supports cancellation of all downloads or individual tracks.

use crate::commands::{DownloadSummary, TrackInfo};
use crate::utils::sidecar::{find_sidecar, kill_process, spawn_sidecar};
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{command, AppHandle, Emitter, State};

/// Shared state for download cancellation.
pub struct DownloadState {
    /// Flag to signal full cancellation of the download loop.
    pub cancel_flag: AtomicBool,
    /// PID of the currently running yt-dlp process (for killing on cancel).
    pub current_pid: Mutex<Option<u32>>,
    /// Track ID currently being downloaded.
    pub current_track_id: Mutex<Option<String>>,
    /// Set of track IDs to skip (for per-track cancellation).
    pub skip_set: Mutex<HashSet<String>>,
}

impl DownloadState {
    pub fn new() -> Self {
        Self {
            cancel_flag: AtomicBool::new(false),
            current_pid: Mutex::new(None),
            current_track_id: Mutex::new(None),
            skip_set: Mutex::new(HashSet::new()),
        }
    }
}

/// Download progress event that is emitted to the frontend.
#[derive(Debug, Clone, serde::Serialize)]
struct DownloadProgress {
    current: usize,
    total: usize,
    track_title: String,
    track_id: String,
    status: String,
}

/// Downloads a collection of tracks to the specified output directory.
#[command]
pub async fn download_tracks(
    app: AppHandle,
    state: State<'_, DownloadState>,
    tracks: Vec<TrackInfo>,
    output_dir: String,
) -> Result<DownloadSummary, String> {
    // Reset state at the start of a new download batch
    state.cancel_flag.store(false, Ordering::Relaxed);
    state.skip_set.lock().unwrap().clear();

    // Validate output directory
    let output_path = Path::new(&output_dir);
    if !output_path.exists() {
        std::fs::create_dir_all(output_path)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    // Find yt-dlp sidecar
    let yt_dlp_path = find_sidecar("yt-dlp").map_err(|e| {
        format!(
            "yt-dlp not found. Make sure it's installed or bundled with the app: {}",
            e
        )
    })?;

    let mut summary = DownloadSummary {
        successful: 0,
        failed: 0,
        errors: Vec::new(),
    };

    let total_tracks = tracks.len();

    for (index, track) in tracks.iter().enumerate() {
        let current = index + 1;
        let track_title = track.title.clone();
        let track_id = track.id.clone();

        // Check full cancellation
        if state.cancel_flag.load(Ordering::Relaxed) {
            emit_progress(
                &app,
                DownloadProgress {
                    current,
                    total: total_tracks,
                    track_title,
                    track_id,
                    status: "cancelled".to_string(),
                },
            );
            break;
        }

        // Check if this track was individually skipped
        if state.skip_set.lock().unwrap().contains(&track_id) {
            emit_progress(
                &app,
                DownloadProgress {
                    current,
                    total: total_tracks,
                    track_title,
                    track_id,
                    status: "cancelled".to_string(),
                },
            );
            continue;
        }

        // Set current track ID
        *state.current_track_id.lock().unwrap() = Some(track_id.clone());

        // Emit downloading event
        emit_progress(
            &app,
            DownloadProgress {
                current,
                total: total_tracks,
                track_title: track_title.clone(),
                track_id: track_id.clone(),
                status: "downloading".to_string(),
            },
        );

        // Build yt-dlp arguments
        let output_template = format!(
            "{}/%(title)s.%(ext)s",
            output_dir.replace("\\", "/")
        );

        let mut args = vec![
            "-x".to_string(),
            "--audio-format".to_string(),
            "mp3".to_string(),
            "--audio-quality".to_string(),
            "0".to_string(),
            "-o".to_string(),
            output_template,
        ];

        // Build ffmpeg metadata args for ID3 tags
        let mut meta_flags = Vec::new();
        meta_flags.push(format!("-metadata \"artist={}\"", escape_metadata(&track.artist)));
        meta_flags.push(format!("-metadata \"title={}\"", escape_metadata(&track.title)));
        if let Some(ref album) = track.album {
            meta_flags.push(format!("-metadata \"album={}\"", escape_metadata(album)));
        }
        if let Some(track_num) = track.track_number {
            meta_flags.push(format!("-metadata \"track={}\"", track_num));
        }
        if let Some(ref year) = track.year {
            meta_flags.push(format!("-metadata \"date={}\"", escape_metadata(year)));
        }

        args.push("--postprocessor-args".to_string());
        args.push(format!("ffmpeg:{} -id3v2_version 3", meta_flags.join(" ")));

        args.push(track.url.clone());

        // Run yt-dlp with cancellation support
        match run_cancellable(&yt_dlp_path, &args, &state).await {
            Ok(_) => {
                // Check if cancelled or skipped during download
                let was_skipped = state.skip_set.lock().unwrap().contains(&track_id);
                let was_cancelled = state.cancel_flag.load(Ordering::Relaxed);

                if was_skipped || was_cancelled {
                    emit_progress(
                        &app,
                        DownloadProgress {
                            current,
                            total: total_tracks,
                            track_title,
                            track_id,
                            status: "cancelled".to_string(),
                        },
                    );
                    if was_cancelled {
                        break;
                    }
                    continue;
                }

                summary.successful += 1;
                emit_progress(
                    &app,
                    DownloadProgress {
                        current,
                        total: total_tracks,
                        track_title,
                        track_id,
                        status: "completed".to_string(),
                    },
                );
            }
            Err(e) => {
                // Check if skipped or fully cancelled
                let was_skipped = state.skip_set.lock().unwrap().contains(&track_id);
                let was_cancelled = state.cancel_flag.load(Ordering::Relaxed);

                if was_skipped || was_cancelled {
                    emit_progress(
                        &app,
                        DownloadProgress {
                            current,
                            total: total_tracks,
                            track_title,
                            track_id,
                            status: "cancelled".to_string(),
                        },
                    );
                    if was_cancelled {
                        break;
                    }
                    continue;
                }

                summary.failed += 1;
                let error_msg = format!("Failed to download '{}': {}", track_title, e);
                summary.errors.push(error_msg);

                emit_progress(
                    &app,
                    DownloadProgress {
                        current,
                        total: total_tracks,
                        track_title,
                        track_id,
                        status: "error".to_string(),
                    },
                );
            }
        }
    }

    // Clear state
    *state.current_pid.lock().unwrap() = None;
    *state.current_track_id.lock().unwrap() = None;

    Ok(summary)
}

/// Cancels all remaining downloads.
#[command]
pub async fn cancel_downloads(state: State<'_, DownloadState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::Relaxed);
    kill_current_process(&state);
    Ok(())
}

/// Skips a single track by ID. If it's currently downloading, kills the process.
#[command]
pub async fn skip_track(state: State<'_, DownloadState>, track_id: String) -> Result<(), String> {
    state.skip_set.lock().unwrap().insert(track_id.clone());

    // If this track is currently downloading, kill the process
    let is_current = state
        .current_track_id
        .lock()
        .unwrap()
        .as_ref()
        .map_or(false, |id| id == &track_id);

    if is_current {
        kill_current_process(&state);
    }

    Ok(())
}

/// Kills the currently running yt-dlp process.
fn kill_current_process(state: &DownloadState) {
    if let Some(pid) = *state.current_pid.lock().unwrap() {
        kill_process(pid);
    }
}

/// Runs a sidecar command with cancellation support.
async fn run_cancellable(
    binary_path: &Path,
    args: &[String],
    state: &DownloadState,
) -> Result<String, String> {
    let child = spawn_sidecar(binary_path, args)?;

    // Store PID for cancellation
    if let Some(pid) = child.id() {
        *state.current_pid.lock().unwrap() = Some(pid);
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("Process error: {}", e))?;

    // Clear PID
    *state.current_pid.lock().unwrap() = None;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else {
            stdout.to_string()
        };
        return Err(format!(
            "Command failed with status {}: {}",
            output.status, error_msg
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to decode output: {}", e))?;
    Ok(stdout)
}

/// Helper function to emit a download progress event to the frontend.
fn emit_progress(app: &AppHandle, progress: DownloadProgress) {
    if let Err(e) = app.emit("download-progress", &progress) {
        eprintln!("Failed to emit download-progress event: {}", e);
    }
}

/// Escapes special characters in metadata values for ffmpeg.
fn escape_metadata(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
