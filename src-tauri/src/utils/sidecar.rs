/// Sidecar binary management.
///
/// Handles finding and running external executables (yt-dlp, ffmpeg) as sidecars.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;

/// On Windows, prevent child processes from opening console windows.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Finds a sidecar binary by name.
///
/// This function attempts to locate a binary in the following order:
/// 1. In the app's resource directory (for bundled sidecars in production)
/// 2. In the system PATH (for development or system-installed binaries)
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
    let mut cmd = Command::new(binary_path);
    cmd.args(args);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output()
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
    let mut cmd = AsyncCommand::new(binary_path);
    cmd.args(args);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output()
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

/// Spawns a sidecar command asynchronously with hidden console window on Windows.
/// Returns the Child process for PID tracking / cancellation.
pub fn spawn_sidecar(binary_path: &Path, args: &[String]) -> Result<tokio::process::Child, String> {
    let mut cmd = AsyncCommand::new(binary_path);
    cmd.args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to execute {}: {}", binary_path.display(), e))
}

/// Kills a process by PID. Cross-platform (kill on unix, taskkill on windows).
pub fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .arg(pid.to_string())
            .output();
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string(), "/F"]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.output();
    }
}
