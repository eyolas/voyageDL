/// Download management commands.
///
/// Handles downloading tracks using yt-dlp and emitting progress updates to the frontend.

use crate::commands::{DownloadSummary, TrackInfo};
use crate::utils::sidecar::{find_sidecar, run_sidecar_command_async};
use std::path::Path;
use tauri::{command, AppHandle, Emitter};

/// Download progress event that is emitted to the frontend.
#[derive(Debug, Clone, serde::Serialize)]
struct DownloadProgress {
    current: usize,
    total: usize,
    track_title: String,
    status: String,
}

/// Downloads a collection of tracks to the specified output directory.
///
/// This command:
/// 1. Validates the output directory
/// 2. For each track, runs yt-dlp to download the audio as MP3
/// 3. Emits progress events to the frontend via the "download-progress" event
/// 4. Collects any errors encountered
/// 5. Returns a summary of the download operation
///
/// # Arguments
/// * `app` - Tauri app handle (used for emitting events)
/// * `tracks` - Vector of TrackInfo objects to download
/// * `output_dir` - Directory where MP3 files will be saved
///
/// # Returns
/// Returns a DownloadSummary with the results of the operation.
#[command]
pub async fn download_tracks(
    app: AppHandle,
    tracks: Vec<TrackInfo>,
    output_dir: String,
) -> Result<DownloadSummary, String> {
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

        // Emit progress event
        emit_progress(
            &app,
            DownloadProgress {
                current,
                total: total_tracks,
                track_title: track_title.clone(),
                status: "downloading".to_string(),
            },
        );

        // Build yt-dlp arguments
        // Format: -x = extract audio, --audio-format mp3 = save as MP3
        // --audio-quality 0 = best quality
        // -o = output template
        let output_template = format!(
            "{}/%(title)s.%(ext)s",
            output_dir.replace("\\", "/") // Normalize path separators
        );

        let args = vec![
            "-x".to_string(),
            "--audio-format".to_string(),
            "mp3".to_string(),
            "--audio-quality".to_string(),
            "0".to_string(),
            "-o".to_string(),
            output_template,
            track.url.clone(),
        ];

        // Run yt-dlp
        match run_sidecar_command_async(&yt_dlp_path, &args).await {
            Ok(_) => {
                summary.successful += 1;

                // Emit success event
                emit_progress(
                    &app,
                    DownloadProgress {
                        current,
                        total: total_tracks,
                        track_title: track_title.clone(),
                        status: "completed".to_string(),
                    },
                );
            }
            Err(e) => {
                summary.failed += 1;
                let error_msg = format!("Failed to download '{}': {}", track_title, e);
                summary.errors.push(error_msg.clone());

                // Emit error event
                emit_progress(
                    &app,
                    DownloadProgress {
                        current,
                        total: total_tracks,
                        track_title: track_title.clone(),
                        status: "error".to_string(),
                    },
                );
            }
        }
    }

    Ok(summary)
}

/// Helper function to emit a download progress event to the frontend.
fn emit_progress(app: &AppHandle, progress: DownloadProgress) {
    // Emit the event; if it fails, we just log it and continue
    if let Err(e) = app.emit("download-progress", &progress) {
        eprintln!("Failed to emit download-progress event: {}", e);
    }
}
