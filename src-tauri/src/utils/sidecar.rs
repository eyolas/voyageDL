/// Sidecar binary management.
///
/// Handles finding and running external executables (yt-dlp, ffmpeg) as sidecars.
/// Supports Tauri's naming convention with target triple suffixes.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;

/// On Windows, prevent child processes from opening console windows.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Returns all candidate filenames for a sidecar binary.
/// Tauri bundles sidecars with the target triple suffix.
fn sidecar_candidates(binary_name: &str) -> Vec<String> {
    let target = env!("TARGET");
    let mut candidates = vec![
        format!("{}-{}", binary_name, target),   // yt-dlp-aarch64-apple-darwin
        binary_name.to_string(),                  // yt-dlp
    ];

    #[cfg(windows)]
    {
        // On Windows, also try with .exe extension
        candidates.insert(0, format!("{}-{}.exe", binary_name, target));
        candidates.push(format!("{}.exe", binary_name));
    }

    candidates
}

/// Finds a sidecar binary by name.
///
/// Search order:
/// 1. App's resource directory with target triple suffix (macOS production)
/// 2. Same directory as executable with target triple suffix (Windows production)
/// 3. Plain name in resources/exe directory (dev fallback)
/// 4. System PATH
pub fn find_sidecar(binary_name: &str) -> Result<PathBuf, String> {
    let candidates = sidecar_candidates(binary_name);

    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Check Resources directory (macOS app bundle)
            if let Some(resources_dir) = exe_dir.parent().map(|p| p.join("Resources")) {
                for name in &candidates {
                    let path = resources_dir.join(name);
                    if path.exists() {
                        return Ok(path);
                    }
                }
            }

            // Check same directory as executable (Windows install)
            for name in &candidates {
                let path = exe_dir.join(name);
                if path.exists() {
                    return Ok(path);
                }
            }
        }
    }

    // Try to find in system PATH
    if let Ok(path_var) = env::var("PATH") {
        for path_dir in env::split_paths(&path_var) {
            for name in &candidates {
                let path = path_dir.join(name);
                if path.exists() {
                    return Ok(path);
                }
            }
        }
    }

    Err(format!(
        "Binary '{}' not found in PATH or app resources (tried: {})",
        binary_name,
        candidates.join(", ")
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
