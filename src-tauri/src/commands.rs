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
use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tracing::{info, warn};

use crate::cancel::Cancel;
use crate::detect::exact::{self, ExactGroup, ExactGroupMember};
use crate::detect::manual;
use crate::detect::perceptual;
use crate::error::{AppError, AppResult};
use crate::hash;
use crate::state::{AppState, ScanHandle};
use crate::thumbs;
use crate::walk;

// ---------------------------------------------------------------------------
// pick_folder
// ---------------------------------------------------------------------------

/// Abre el diálogo nativo de selección de carpeta. Devuelve `None` si el
/// usuario cancela.
#[tauri::command(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct StartScanArgs {
    pub root: String,
    /// Lista de modos a ejecutar. Por ahora sólo se persiste; las
    /// Fases 3-7 implementan la lógica. Siempre debe contener al
    /// menos un elemento.
    pub modes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[tauri::command(rename_all = "camelCase")]
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
                // Poblar los grupos de duplicados exactos (Modo 1). Si
                // falla, lo loggeamos pero no fallamos el escaneo: el
                // usuario podría re-invocar `populate_exact_groups`
                // manualmente (Fase 8+).
                if let Err(e) = exact::populate(&conn, session_id) {
                    warn!(session_id, error = %e, "no se pudieron poblar exact_groups");
                }
                info!(session_id, "Modo 1 poblado");

                // Fase 5: calcular pHash de las imágenes y poblar
                // perceptual_groups. Bloquea el cierre del scan porque
                // queremos que el usuario pueda ir a Modo 2 directamente.
                if let Err(e) = hash::image_hash::populate_session(&conn, session_id) {
                    warn!(session_id, error = %e, "pHash de imágenes falló");
                } else {
                    info!(session_id, "pHash de imágenes calculado");
                }
                if let Err(e) = perceptual::populate(&conn, session_id) {
                    warn!(session_id, error = %e, "perceptual_groups falló");
                } else {
                    info!(session_id, "Modo 2 poblado");
                }

                // Fase 7: poblar `manual_pairs` (Modo 3) tras los modos 1 y 2.
                if let Err(e) = manual::populate(&conn, session_id) {
                    warn!(session_id, error = %e, "manual_pairs falló");
                } else {
                    info!(session_id, "Modo 3 poblado");
                }
                drop(conn);

                // Fase 4: generar thumbnails en background. No bloquea
                // el cierre del scan; el frontend puede pedirlos
                // on-demand si no están listos.
                walk::spawn_thumb_generation(
                    app_for_task.clone(),
                    session_id,
                    summary.entries,
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
#[serde(rename_all = "camelCase")]
pub struct CancelScanArgs {
    pub session_id: i64,
}

#[tauri::command(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct GetScanStatusArgs {
    pub session_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub session: ScanSession,
    pub cancel_requested: bool,
}

#[tauri::command(rename_all = "camelCase")]
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

#[tauri::command(rename_all = "camelCase")]
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

#[tauri::command(rename_all = "camelCase")]
#[allow(dead_code)]
pub async fn delete_session(
    _state: State<'_, AppState>,
    _args: CancelScanArgs, // re-uso del struct por simplicidad
) -> AppResult<()> {
    // Implementado en Fase 9 cuando se borren los thumbs asociados.
    Ok(())
}

// ===========================================================================
// Modo 1 — Duplicados exactos
// ===========================================================================

// --- get_exact_groups ---------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExactGroupsArgs {
    pub session_id: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExactGroupsResult {
    pub groups: Vec<ExactGroup>,
    pub total: i64,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_exact_groups(
    state: State<'_, AppState>,
    args: GetExactGroupsArgs,
) -> AppResult<GetExactGroupsResult> {
    let conn = state.db.lock().await;
    let (groups, total) = exact::list_paginated(&conn, args.session_id, args.offset, args.limit)?;
    Ok(GetExactGroupsResult { groups, total })
}

// --- get_group_members --------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGroupMembersArgs {
    pub group_id: i64,
    /// "exact" | "perceptual" | "manual". Por ahora sólo "exact".
    #[allow(dead_code)]
    pub mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGroupMembersResult {
    pub members: Vec<ExactGroupMember>,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_group_members(
    state: State<'_, AppState>,
    args: GetGroupMembersArgs,
) -> AppResult<GetGroupMembersResult> {
    let conn = state.db.lock().await;
    let members = exact::members(&conn, args.group_id)?;
    Ok(GetGroupMembersResult { members })
}

// --- set_decision -------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDecisionArgs {
    pub file_id: i64,
    /// "delete" | "keep" | "undecided".
    pub action: String,
}

fn validate_action(action: &str) -> AppResult<&str> {
    match action {
        "delete" | "keep" | "undecided" => Ok(action),
        other => Err(AppError::Other(format!(
            "acción de decisión inválida: {other} (esperaba delete|keep|undecided)"
        ))),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn set_decision(
    state: State<'_, AppState>,
    args: SetDecisionArgs,
) -> AppResult<()> {
    let action = validate_action(&args.action)?;
    let conn = state.db.lock().await;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO decisions (file_id, action, decided_at) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(file_id) DO UPDATE SET action = excluded.action, decided_at = excluded.decided_at",
        rusqlite::params![args.file_id, action, now],
    )?;
    Ok(())
}

// --- bulk_set_decision --------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkSetDecisionArgs {
    pub file_ids: Vec<i64>,
    pub action: String,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn bulk_set_decision(
    state: State<'_, AppState>,
    args: BulkSetDecisionArgs,
) -> AppResult<()> {
    let action = validate_action(&args.action)?;
    if args.file_ids.is_empty() {
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();
    let mut conn = state.db.lock().await;
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO decisions (file_id, action, decided_at) \
             VALUES (?1, ?2, ?3) \
             ON CONFLICT(file_id) DO UPDATE SET action = excluded.action, decided_at = excluded.decided_at",
        )?;
        for fid in &args.file_ids {
            stmt.execute(rusqlite::params![fid, action, now])?;
        }
    }
    tx.commit()?;
    Ok(())
}

// ===========================================================================
// Fase 4 — Thumbnails
// ===========================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetThumbnailArgs {
    pub file_id: i64,
    /// Tamaño preferido (lado mayor). 0 → default (256). Por ahora
    /// sólo soportamos el tamaño del cache (256); si piden otro,
    /// devolvemos el cacheado igualmente y el frontend escala con CSS.
    #[allow(dead_code)]
    pub size: u32,
}

/// Devuelve la ruta absoluta al thumbnail cacheado del fichero.
/// Si no existe (p.ej. la sesión era antigua, previa a Fase 4), lo
/// genera ahora a partir del path en la DB.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_thumbnail(
    state: State<'_, AppState>,
    args: GetThumbnailArgs,
) -> AppResult<String> {
    let conn = state.db.lock().await;
    let (path_str, kind_str, mtime_ns): (String, String, i64) = conn
        .query_row(
            "SELECT path, kind, mtime_ns FROM files WHERE id = ?1",
            [args.file_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| AppError::Other(format!("file_id {} no existe", args.file_id)))?;
    drop(conn);

    let kind = match kind_str.as_str() {
        "image" => walk::FileKind::Image,
        "video" => walk::FileKind::Video,
        other => {
            return Err(AppError::Other(format!(
                "kind desconocido '{other}' para file_id {}",
                args.file_id
            )));
        }
    };
    let path = std::path::PathBuf::from(&path_str);

    // `generate` es CPU/IO bound; lo movemos a un thread bloqueante
    // para no saturar el runtime de Tauri.
    let cache_root = state.cache_root.clone();
    let abs = tauri::async_runtime::spawn_blocking(move || {
        thumbs::generate(&path, kind, mtime_ns, &cache_root)
    })
    .await
    .map_err(|e| AppError::Other(format!("join thumb task: {e}")))??;

    Ok(abs.to_string_lossy().to_string())
}

// ===========================================================================
// Fase 5 — Modo 2 (duplicados perceptuales)
// ===========================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPerceptualGroupsArgs {
    pub session_id: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPerceptualGroupsResult {
    pub groups: Vec<perceptual::PerceptualGroup>,
    pub total: i64,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_perceptual_groups(
    state: State<'_, AppState>,
    args: GetPerceptualGroupsArgs,
) -> AppResult<GetPerceptualGroupsResult> {
    let conn = state.db.lock().await;
    let (groups, total) =
        perceptual::list_paginated(&conn, args.session_id, args.offset, args.limit)?;
    Ok(GetPerceptualGroupsResult { groups, total })
}

// ===========================================================================
// Fase 7 — Modo 3 (revisión visual manual)
// ===========================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetManualPairsArgs {
    pub session_id: i64,
    pub offset: i64,
    pub limit: i64,
    /// "pending" (default) | "all"
    pub filter: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetManualPairsResult {
    pub pairs: Vec<manual::ManualPair>,
    pub total: i64,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_manual_pairs(
    state: State<'_, AppState>,
    args: GetManualPairsArgs,
) -> AppResult<GetManualPairsResult> {
    let conn = state.db.lock().await;
    let filter = manual::PairFilter::from_str(args.filter.as_deref().unwrap_or("pending"))
        .unwrap_or(manual::PairFilter::Pending);
    let (pairs, total) =
        manual::list_paginated(&conn, args.session_id, filter, args.offset, args.limit)?;
    Ok(GetManualPairsResult { pairs, total })
}

/// Marca un par como revisado (saltar).
#[tauri::command(rename_all = "camelCase")]
pub async fn skip_manual_pair(
    state: State<'_, AppState>,
    args: SkipManualPairArgs,
) -> AppResult<()> {
    let conn = state.db.lock().await;
    manual::mark_reviewed(&conn, args.pair_id)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkipManualPairArgs {
    pub pair_id: i64,
}

/// Metadata de un fichero (usado por Modo 3 para mostrar el par).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetaDto {
    pub id: i64,
    pub path: String,
    pub size_bytes: i64,
    pub mtime_ns: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFilesMetaArgs {
    pub file_ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFilesMetaResult {
    pub files: Vec<FileMetaDto>,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_files_meta(
    state: State<'_, AppState>,
    args: GetFilesMetaArgs,
) -> AppResult<GetFilesMetaResult> {
    if args.file_ids.is_empty() {
        return Ok(GetFilesMetaResult { files: Vec::new() });
    }
    let conn = state.db.lock().await;
    // Construir placeholders para IN clause.
    let placeholders = std::iter::repeat("?")
        .take(args.file_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT id, path, size_bytes, mtime_ns FROM files WHERE id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> = args
        .file_ids
        .iter()
        .map(|i| i as &dyn rusqlite::ToSql)
        .collect();
    let rows = stmt
        .query_map(params.as_slice(), |r| {
            Ok(FileMetaDto {
                id: r.get(0)?,
                path: r.get(1)?,
                size_bytes: r.get(2)?,
                mtime_ns: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(GetFilesMetaResult { files: rows })
}
