/// Sidecar binary management.
///
/// Handles finding and running external executables (yt-dlp, ffmpeg) as sidecars.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;

/// Finds a sidecar binary by name.
///
/// This function attempts to locate a binary in the following order:
/// 1. In the app's resource directory (for bundled sidecars in production)
/// 2. In the system PATH (for development or system-installed binaries)
///
/// # Arguments
/// * `binary_name` - Name of the binary to find (e.g., "yt-dlp", "ffmpeg")
///
/// # Returns
/// Returns the path to the binary if found, or an error if not found.
pub fn find_sidecar(binary_name: &str) -> Result<PathBuf, String> {
    // Try to find in app's resource directory first (production bundled)
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let bundled_binary = exe_dir
                .parent()
                .map(|p| p.join("Resources").join(binary_name))
                .filter(|p| p.exists());

            if let Some(path) = bundled_binary {
                return Ok(path);
            }

            // Also check in the same directory as the executable
            let same_dir_binary = exe_dir.join(binary_name);
            if same_dir_binary.exists() {
                return Ok(same_dir_binary);
            }
        }
    }

    // Try to find in system PATH
    if let Ok(path_var) = env::var("PATH") {
        for path_dir in env::split_paths(&path_var) {
            let binary_path = path_dir.join(binary_name);

            // On Windows, also check with .exe extension
            #[cfg(windows)]
            {
                let exe_path = path_dir.join(format!("{}.exe", binary_name));
                if exe_path.exists() {
                    return Ok(exe_path);
                }
            }

            if binary_path.exists() {
                return Ok(binary_path);
            }
        }
    }

    Err(format!(
        "Binary '{}' not found in PATH or app resources",
        binary_name
    ))
}

/// Runs a sidecar command synchronously and returns its output.
pub fn run_sidecar_command(binary_path: &Path, args: &[String]) -> Result<String, String> {
    let output = Command::new(binary_path)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", binary_path.display(), e))?;

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
        .map_err(|e| format!("Failed to decode command output: {}", e))?;

    Ok(stdout)
}

/// Runs a sidecar command asynchronously and returns its output.
/// This allows Tauri events to be emitted between calls without blocking.
pub async fn run_sidecar_command_async(binary_path: &Path, args: &[String]) -> Result<String, String> {
    let output = AsyncCommand::new(binary_path)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to execute {}: {}", binary_path.display(), e))?;

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
        .map_err(|e| format!("Failed to decode command output: {}", e))?;

    Ok(stdout)
}
