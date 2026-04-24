/// Download management commands.
///
/// Handles downloading tracks using yt-dlp and emitting progress updates to the frontend.
/// Supports cancellation of all downloads or individual tracks.

use crate::commands::{DownloadSummary, TrackInfo};
use crate::utils::sidecar::{find_sidecar, kill_process, spawn_sidecar};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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
    audio_format: String,
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

    // Find sidecars
    let yt_dlp_path = find_sidecar("yt-dlp").map_err(|e| {
        format!(
            "yt-dlp not found. Make sure it's installed or bundled with the app: {}",
            e
        )
    })?;
    let ffmpeg_path = find_sidecar("ffmpeg").map_err(|e| {
        format!(
            "ffmpeg not found. Make sure it's installed or bundled with the app: {}",
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
        let is_m4a = audio_format == "m4a";
        let fmt = if is_m4a { "m4a" } else { "mp3" };
        let has_deezer_cover = track.album_cover_url.is_some();

        let output_template = format!(
            "{}/%(title)s.%(ext)s",
            output_dir.replace("\\", "/")
        );

        let mut args = vec![
            "-x".to_string(),
            "--audio-format".to_string(),
            fmt.to_string(),
            "--audio-quality".to_string(),
            "0".to_string(),
        ];

        args.push("-o".to_string());
        args.push(output_template);

        // Build ffmpeg metadata args
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

        let pp_suffix = if is_m4a { "" } else { " -id3v2_version 3" };
        args.push("--postprocessor-args".to_string());
        args.push(format!("ffmpeg:{}{}", meta_flags.join(" "), pp_suffix));

        // Capture the final filepath produced by yt-dlp: Deezer titles and the
        // YouTube filenames diverge often (e.g. remaster 2015 vs 1998), so we have
        // yt-dlp emit the exact path instead of guessing from title sanitization.
        let filepath_marker = std::env::temp_dir()
            .join(format!("voyagedl_{}_{}.path", std::process::id(), track_id));
        let _ = std::fs::remove_file(&filepath_marker);
        args.push("--print-to-file".to_string());
        args.push("after_move:filepath".to_string());
        args.push(filepath_marker.to_string_lossy().to_string());

        args.push(track.url.clone());

        // Run yt-dlp with cancellation support
        let run_result = run_cancellable(&yt_dlp_path, &args, &state).await;
        let audio_path = std::fs::read_to_string(&filepath_marker)
            .ok()
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| p.exists());
        let _ = std::fs::remove_file(&filepath_marker);

        match run_result {
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

                // Embed Deezer album cover if available
                if has_deezer_cover {
                    if let Some(ref cover_url) = track.album_cover_url {
                        match audio_path.as_deref() {
                            Some(path) => {
                                if let Err(e) = embed_cover(
                                    path, fmt, cover_url, &ffmpeg_path
                                ).await {
                                    eprintln!(
                                        "[cover] embed failed for '{}': {}",
                                        track.title, e
                                    );
                                }
                            }
                            None => eprintln!(
                                "[cover] skip '{}': audio file path not available",
                                track.title
                            ),
                        }
                    }
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

/// Downloads a cover image from Deezer and embeds it into the audio file using ffmpeg.
async fn embed_cover(
    audio_path: &Path,
    fmt: &str,
    cover_url: &str,
    ffmpeg_path: &Path,
) -> Result<(), String> {
    eprintln!(
        "[cover] start embed file='{}' fmt={} url={}",
        audio_path.display(), fmt, cover_url
    );

    // Download cover image
    let cover_bytes = reqwest::get(cover_url)
        .await
        .map_err(|e| format!("Failed to download cover: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read cover bytes: {}", e))?;

    let output_dir = audio_path.parent().ok_or_else(|| {
        format!("Invalid audio path (no parent): {}", audio_path.display())
    })?;
    let cover_path = output_dir.join("_cover_tmp.jpg");
    std::fs::write(&cover_path, &cover_bytes)
        .map_err(|e| format!("Failed to write cover: {}", e))?;

    let tmp_path = audio_path.with_extension(format!("tmp.{}", fmt));

    // Use ffmpeg to embed cover
    let mut args = vec![
        "-i".to_string(),
        audio_path.to_string_lossy().to_string(),
        "-i".to_string(),
        cover_path.to_string_lossy().to_string(),
        "-map".to_string(), "0:a".to_string(),
        "-map".to_string(), "1:0".to_string(),
        "-c".to_string(), "copy".to_string(),
    ];

    if fmt == "mp3" {
        args.extend([
            "-id3v2_version".to_string(), "3".to_string(),
            "-metadata:s:v".to_string(), "title=Album cover".to_string(),
            "-metadata:s:v".to_string(), "comment=Cover (front)".to_string(),
        ]);
    }
    args.extend([
        "-disposition:v:0".to_string(), "attached_pic".to_string(),
    ]);

    args.push("-y".to_string());
    args.push(tmp_path.to_string_lossy().to_string());

    let child = spawn_sidecar(ffmpeg_path, &args)?;
    let output = child.wait_with_output().await
        .map_err(|e| format!("ffmpeg cover embed failed: {}", e))?;

    // Clean up cover temp file
    let _ = std::fs::remove_file(&cover_path);

    if output.status.success() {
        let _ = std::fs::remove_file(audio_path);
        std::fs::rename(&tmp_path, audio_path)
            .map_err(|e| format!("Failed to replace audio file after cover embed: {}", e))?;
        Ok(())
    } else {
        let _ = std::fs::remove_file(&tmp_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "ffmpeg exited with status {}: {}",
            output.status,
            stderr.trim()
        ))
    }
}

