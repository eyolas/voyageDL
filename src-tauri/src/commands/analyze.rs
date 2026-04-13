/// Analyze state management.
///
/// Shared state for cancelling and pausing URL analysis (Deezer YT search loop).

use crate::utils::sidecar::kill_process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{command, State};

pub struct AnalyzeState {
    pub cancel_flag: AtomicBool,
    pub pause_flag: AtomicBool,
    pub current_pid: Mutex<Option<u32>>,
}

impl AnalyzeState {
    pub fn new() -> Self {
        Self {
            cancel_flag: AtomicBool::new(false),
            pause_flag: AtomicBool::new(false),
            current_pid: Mutex::new(None),
        }
    }

    /// Resets flags for a new analysis.
    pub fn reset(&self) {
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.pause_flag.store(false, Ordering::Relaxed);
        *self.current_pid.lock().unwrap() = None;
    }

    /// Checks if cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    /// Checks if paused.
    pub fn is_paused(&self) -> bool {
        self.pause_flag.load(Ordering::Relaxed)
    }
}

/// Cancels the current analysis.
#[command]
pub async fn cancel_analyze(state: State<'_, AnalyzeState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::Relaxed);

    if let Some(pid) = *state.current_pid.lock().unwrap() {
        kill_process(pid);
    }

    Ok(())
}

/// Toggles pause on the current analysis.
#[command]
pub async fn toggle_pause_analyze(state: State<'_, AnalyzeState>) -> Result<bool, String> {
    let new_state = !state.pause_flag.load(Ordering::Relaxed);
    state.pause_flag.store(new_state, Ordering::Relaxed);
    Ok(new_state)
}
