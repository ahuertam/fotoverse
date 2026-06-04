//! Comandos Tauri (interfaz Rust ↔ TypeScript).
//!
//! Convenciones (ver docs/CONTRIBUTING.md#añadir-un-comando-tauri):
//! - Todo comando retorna `Result<T, AppError>`.
//! - El argumento siempre es un struct con `#[derive(Deserialize)]`.
//! - Si emite eventos, declara el payload en `walk/progress.rs` o donde
//!   corresponda y el wrapper TS en `src/lib/tauri.ts`.
//! - NUNCA añadir lógica de negocio aquí: este módulo es el "controller",
//!   la lógica vive en `walk/`, `hash/`, `detect/`, `review/`.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tracing::{info, warn};

use crate::cancel::Cancel;
use crate::error::{AppError, AppResult};
use crate::state::{AppState, ScanHandle};
use crate::walk;

// ---------------------------------------------------------------------------
// pick_folder
// ---------------------------------------------------------------------------

/// Abre el diálogo nativo de selección de carpeta. Devuelve `None` si el
/// usuario cancela.
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> AppResult<Option<String>> {
    // `blocking_pick_folder` se debe invocar desde un task bloqueante
    // porque internamente espera un callback del dialog.
    let folder = tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().blocking_pick_folder()
    })
    .await
    .map_err(|e| AppError::Other(format!("join dialog task: {e}")))?;

    Ok(folder.map(|p| p.to_string()))
}

// ---------------------------------------------------------------------------
// start_scan
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct StartScanArgs {
    pub root: String,
    /// Lista de modos a ejecutar. Por ahora sólo se persiste; las
    /// Fases 3-7 implementan la lógica. Siempre debe contener al
    /// menos un elemento.
    pub modes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ScanSession {
    pub id: i64,
    pub root_path: String,
    pub modes: String,
    pub status: String,
    pub total_files: i64,
    pub hashed_files: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
}

impl ScanSession {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            root_path: row.get(1)?,
            modes: row.get(2)?,
            status: row.get(3)?,
            total_files: row.get(4)?,
            hashed_files: row.get(5)?,
            started_at: row.get(6)?,
            finished_at: row.get(7)?,
        })
    }
}

/// Lanza un escaneo en background. Devuelve el `session_id` recién creado.
#[tauri::command]
pub async fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    args: StartScanArgs,
) -> AppResult<ScanSession> {
    // 1. Verificar que no hay otro escaneo en curso.
    {
        let current = state.current.lock().await;
        if let Some(handle) = current.as_ref() {
            return Err(AppError::AlreadyRunning(handle.session_id));
        }
    }

    // 2. Validar la ruta.
    let root = PathBuf::from(&args.root);
    if !root.is_dir() {
        return Err(AppError::InvalidPath(args.root.clone()));
    }
    if args.modes.is_empty() {
        return Err(AppError::Other("modes no puede estar vacío".into()));
    }

    // 3. Crear la sesión en la DB.
    let modes_json = serde_json::to_string(&args.modes)?;
    let started_at = Utc::now().to_rfc3339();
    let session_id = {
        let conn = state.db.lock().await;
        conn.execute(
            "INSERT INTO scan_sessions (root_path, modes, status, started_at) \
             VALUES (?1, ?2, 'running', ?3)",
            rusqlite::params![args.root, modes_json, started_at],
        )?;
        conn.last_insert_rowid()
    };
    info!(session_id, root = %args.root, "sesión creada");

    // 4. Crear el handle y guardarlo en AppState.
    let cancel = Cancel::new();
    {
        let mut current = state.current.lock().await;
        *current = Some(ScanHandle { session_id, cancel: cancel.clone() });
    }

    // 5. Spawn del walk en background. Usamos `tauri::async_runtime::spawn`
    //    (que internamente es tokio).
    let db = Arc::clone(&state.db);
    let app_for_task = app.clone();
    let cancel_for_task = cancel.clone();
    let root_for_task = root.clone();

    tauri::async_runtime::spawn(async move {
        let result = walk::run(
            app_for_task.clone(),
            db.clone(),
            session_id,
            root_for_task,
            cancel_for_task.clone(),
        )
        .await;

        // Al terminar (éxito, error o cancelación), liberar el handle.
        drop(db);

        // Limpiar AppState.current si todavía apunta a nosotros.
        // (Lo hacemos leyendo el state desde la app.)
        if let Some(state) = app_for_task.try_state::<AppState>() {
            let mut current = state.current.lock().await;
            if current.as_ref().map(|h| h.session_id) == Some(session_id) {
                *current = None;
            }
        }

        match result {
            Ok(summary) => {
                // Marcar la sesión como completed.
                let state = app_for_task.state::<AppState>();
                let conn = state.db.lock().await;
                let _ = conn.execute(
                    "UPDATE scan_sessions \
                     SET status = 'completed', total_files = ?1, finished_at = ?2 \
                     WHERE id = ?3",
                    rusqlite::params![summary.total_files as i64, Utc::now().to_rfc3339(), session_id],
                );
            }
            Err(AppError::Cancelled) => {
                warn!(session_id, "escaneo cancelado");
                let state = app_for_task.state::<AppState>();
                let conn = state.db.lock().await;
                let _ = conn.execute(
                    "UPDATE scan_sessions SET status = 'cancelled', finished_at = ?1 WHERE id = ?2",
                    rusqlite::params![Utc::now().to_rfc3339(), session_id],
                );
            }
            Err(e) => {
                warn!(session_id, error = %e, "escaneo falló");
                let state = app_for_task.state::<AppState>();
                let conn = state.db.lock().await;
                let _ = conn.execute(
                    "UPDATE scan_sessions SET status = 'failed', finished_at = ?1 WHERE id = ?2",
                    rusqlite::params![Utc::now().to_rfc3339(), session_id],
                );
                // Emitir evento de error.
                let _ = app_for_task.emit(
                    "scan_error",
                    serde_json::json!({ "session_id": session_id, "message": e.to_string() }),
                );
            }
        }
    });

    // 6. Devolver la sesión recién creada.
    let conn = state.db.lock().await;
    let session = conn.query_row(
        "SELECT id, root_path, modes, status, total_files, hashed_files, started_at, finished_at \
         FROM scan_sessions WHERE id = ?1",
        [session_id],
        ScanSession::from_row,
    )?;
    Ok(session)
}

// ---------------------------------------------------------------------------
// cancel_scan
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CancelScanArgs {
    pub session_id: i64,
}

#[tauri::command]
pub async fn cancel_scan(
    state: State<'_, AppState>,
    args: CancelScanArgs,
) -> AppResult<()> {
    let current = state.current.lock().await;
    let handle = current
        .as_ref()
        .ok_or(AppError::SessionNotFound(args.session_id))?;
    if handle.session_id != args.session_id {
        return Err(AppError::SessionNotFound(args.session_id));
    }
    handle.cancel.cancel();
    Ok(())
}

// ---------------------------------------------------------------------------
// get_scan_status
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GetScanStatusArgs {
    pub session_id: i64,
}

#[derive(Debug, Serialize)]
pub struct ScanStatus {
    pub session: ScanSession,
    pub cancel_requested: bool,
}

#[tauri::command]
pub async fn get_scan_status(
    state: State<'_, AppState>,
    args: GetScanStatusArgs,
) -> AppResult<ScanStatus> {
    let conn = state.db.lock().await;
    let session = conn
        .query_row(
            "SELECT id, root_path, modes, status, total_files, hashed_files, started_at, finished_at \
             FROM scan_sessions WHERE id = ?1",
            [args.session_id],
            ScanSession::from_row,
        )
        .map_err(|_| AppError::SessionNotFound(args.session_id))?;

    let cancel_requested = {
        let current = state.current.lock().await;
        current
            .as_ref()
            .map(|h| h.session_id == args.session_id && h.cancel.is_cancelled())
            .unwrap_or(false)
    };

    Ok(ScanStatus { session, cancel_requested })
}

// ---------------------------------------------------------------------------
// list_sessions (placeholder útil para Fase 2; UI lo consume en Fase 9)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> AppResult<Vec<ScanSession>> {
    let conn = state.db.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, root_path, modes, status, total_files, hashed_files, started_at, finished_at \
         FROM scan_sessions ORDER BY id DESC LIMIT 100",
    )?;
    let rows = stmt
        .query_map([], ScanSession::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// delete_session (placeholder para Fase 9)
// ---------------------------------------------------------------------------

#[tauri::command]
#[allow(dead_code)]
pub async fn delete_session(
    _state: State<'_, AppState>,
    _args: CancelScanArgs, // re-uso del struct por simplicidad
) -> AppResult<()> {
    // Implementado en Fase 9 cuando se borren los thumbs asociados.
    Ok(())
}
