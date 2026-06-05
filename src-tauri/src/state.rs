//! Estado global de la aplicación.
//!
//! - [`AppState`] se inyecta a cada comando Tauri vía `tauri::State`.
//! - Contiene la conexión SQLite (compartida), el handle del escaneo
//!   en curso (si lo hay) y la ruta raíz del cache de thumbnails.
//! - Es `Send + Sync` para poderla capturar en `tokio::spawn`.

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::cancel::Cancel;

/// Handle del escaneo actualmente en curso (si lo hay).
#[derive(Clone)]
pub struct ScanHandle {
    pub session_id: i64,
    pub cancel: Cancel,
}

/// Estado global de la app. Se construye en `lib::run` y se registra con
/// `tauri::Builder::manage`.
pub struct AppState {
    /// Conexión SQLite (WAL, synchronous=NORMAL). Se serializa con
    /// `Mutex`; para Fase 2 basta porque la carga de I/O es
    /// secuencial por sesión. Si en el futuro hay muchos lectores
    /// concurrentes, considerar `r2d2_sqlite` o conexiones por hilo.
    pub db: Arc<Mutex<Connection>>,

    /// Handle del escaneo en curso. `None` si no hay ninguno.
    pub current: Arc<Mutex<Option<ScanHandle>>>,

    /// Raíz del cache de thumbnails: `<app_cache_dir>/thumbs`.
    /// Lo pasamos a las funciones de `thumbs::*` para que no
    /// tengan que conocer la convención de Tauri.
    pub cache_root: PathBuf,
}

impl AppState {
    pub fn new(db: Connection, cache_root: PathBuf) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            current: Arc::new(Mutex::new(None)),
            cache_root,
        }
    }
}

/// Crea un `CancellationToken` "huérfano" (nadie lo va a cancelar desde
/// fuera). Útil para tests o llamadas internas que no necesitan un handle
/// de cancelación real.
#[allow(dead_code)]
pub fn orphan_cancel() -> Cancel {
    CancellationToken::new()
}
