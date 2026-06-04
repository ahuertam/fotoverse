//! Emisión throttled de eventos de progreso al frontend.
//!
//! Tauri ejecuta comandos en un thread pool; emitir un evento por cada
//! fichero ahogaría el IPC. Throttleamos a 200 ms entre emisiones.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::error::AppError;

/// Intervalo mínimo entre emisiones de un mismo tipo de evento.
const THROTTLE: Duration = Duration::from_millis(200);

/// Payload de `scan_progress` (Rust → frontend).
#[derive(Clone, Debug, Serialize)]
pub struct ScanProgress {
    pub session_id: i64,
    pub phase: &'static str, // "walking" | "hashing_images" | "hashing_videos" | "clustering" | "done"
    pub files_seen: usize,
    pub total_estimate: Option<usize>,
    pub current_path: Option<String>,
}

/// Payload de `scan_completed` (Rust → frontend).
#[derive(Clone, Debug, Serialize)]
pub struct ScanCompleted {
    pub session_id: i64,
    pub total_files: usize,
    pub duration_ms: u64,
    pub status: &'static str, // "completed" | "cancelled" | "failed"
    pub error: Option<String>,
}

/// Emisor throttled de eventos de progreso.
pub struct ProgressEmitter {
    last: Mutex<Instant>,
}

impl Default for ProgressEmitter {
    fn default() -> Self {
        Self {
            last: Mutex::new(Instant::now() - THROTTLE),
        }
    }
}

impl ProgressEmitter {
    /// Emite `scan_progress` si han pasado al menos 200 ms desde la
    /// última emisión. Si `force` es `true`, emite siempre (útil para
    /// el último update de una fase).
    pub fn emit_progress(
        &self,
        app: &AppHandle,
        payload: &ScanProgress,
        force: bool,
    ) -> Result<(), AppError> {
        let mut last = self.last.lock().map_err(|e| {
            AppError::Other(format!("mutex envenenado en ProgressEmitter: {e}"))
        })?;
        if force || last.elapsed() >= THROTTLE {
            app.emit("scan_progress", payload)
                .map_err(|e| AppError::Other(format!("emit scan_progress: {e}")))?;
            *last = Instant::now();
        }
        Ok(())
    }

    /// Emite `scan_completed` (sin throttle, es un evento terminal).
    pub fn emit_completed(
        &self,
        app: &AppHandle,
        payload: &ScanCompleted,
    ) -> Result<(), AppError> {
        app.emit("scan_completed", payload)
            .map_err(|e| AppError::Other(format!("emit scan_completed: {e}")))
    }
}
