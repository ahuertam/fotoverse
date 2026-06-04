//! Recorrido del sistema de archivos (Etapa A del pipeline).
//!
//! Estrategia:
//! 1. `jwalk` recorre el árbol en paralelo.
//! 2. `par_bridge` paraleliza el `stat()` de cada entrada vía rayon.
//! 3. Filtramos por extensión soportada (imagen o vídeo).
//! 4. Acumulamos en un buffer y volcamos a SQLite en transacciones de
//!    1 000 filas.
//! 5. Throttle de eventos de progreso a 200 ms.
//! 6. `CancellationToken` comprobada cada 1 000 entradas.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use jwalk::WalkDir;
use rayon::prelude::*;
use rusqlite::Connection;
use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::cancel::Cancel;
use crate::error::{AppError, AppResult};
use crate::walk::progress::{ProgressEmitter, ScanCompleted, ScanProgress};

pub mod progress;

// ---------------------------------------------------------------------------
// Extensiones soportadas
// ---------------------------------------------------------------------------

const SUPPORTED_IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff",
];

const SUPPORTED_VIDEO_EXTS: &[&str] = &["mp4", "mov", "mkv", "avi", "webm"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Image,
    Video,
}

impl FileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FileKind::Image => "image",
            FileKind::Video => "video",
        }
    }
}

/// Clasifica un archivo por su extensión. Devuelve `None` si no se soporta.
fn classify(path: &Path) -> Option<FileKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if SUPPORTED_IMAGE_EXTS.contains(&ext.as_str()) {
        Some(FileKind::Image)
    } else if SUPPORTED_VIDEO_EXTS.contains(&ext.as_str()) {
        Some(FileKind::Video)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Metadata extraída de cada fichero
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FileMeta {
    pub path: String,
    pub filename: String, // case-folded
    pub ext: String,
    pub size_bytes: i64,
    pub ctime_ns: Option<i64>,
    pub btime_ns: Option<i64>,
    pub mtime_ns: i64,
    pub kind: FileKind,
}

impl FileMeta {
    /// Construye un `FileMeta` a partir de una entrada de `jwalk`. Hace
    /// el `stat()` aquí (puede ser I/O — por eso va en el iterador
    /// paralelo).
    fn from_entry(entry: &jwalk::DirEntry<((), ())>) -> Option<Self> {
        if !entry.file_type().is_file() {
            return None;
        }
        let path = entry.path();
        let kind = classify(&path)?;
        let metadata = entry.metadata().ok()?;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        let ext = path
            .extension()
            .and_then(|n| n.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        let size_bytes = metadata.len() as i64;
        let ctime_ns = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i64);
        let btime_ns = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i64);
        let mtime_ns = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);

        Some(Self {
            path: path.to_string_lossy().to_string(),
            filename,
            ext,
            size_bytes,
            ctime_ns,
            btime_ns,
            mtime_ns,
            kind,
        })
    }
}

// ---------------------------------------------------------------------------
// Ejecución del walk
// ---------------------------------------------------------------------------

const INSERT_BATCH: usize = 1_000;
#[allow(dead_code)]
const CANCEL_CHECK_EVERY: usize = 1_000;

/// Resultado del walk.
#[derive(Debug, Clone)]
pub struct WalkSummary {
    pub total_files: usize,
    #[allow(dead_code)]
    pub duration_ms: u64,
}

/// Lanza el walk en un `spawn_blocking` (I/O bound). Devuelve cuando
/// termina o se cancela.
pub async fn run(
    app: AppHandle,
    db: Arc<Mutex<Connection>>,
    session_id: i64,
    root: PathBuf,
    cancel: Cancel,
) -> AppResult<WalkSummary> {
    let app2 = app.clone();
    let join = tokio::task::spawn_blocking(move || {
        walk_blocking(&app2, db, session_id, &root, &cancel)
    });

    match join.await {
        Ok(Ok(summary)) => Ok(summary),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(AppError::Other(format!("join walk task: {e}"))),
    }
}

fn walk_blocking(
    app: &AppHandle,
    db: Arc<Mutex<Connection>>,
    session_id: i64,
    root: &Path,
    cancel: &CancellationToken,
) -> AppResult<WalkSummary> {
    let started = Instant::now();
    let emitter = ProgressEmitter::default();
    let mut total: usize = 0;
    let mut last_path: Option<String> = None;

    info!(session_id, "iniciando walk en {}", root.display());

    // 1. Recolectar entradas en paralelo (jwalk + rayon).
    //    jwalk no expone par_iter() sin la feature "rayon", así que
    //    usamos par_bridge() sobre into_iter().
    let entries: Vec<FileMeta> = WalkDir::new(root)
        .follow_links(false)
        .skip_hidden(false)
        .into_iter()
        .par_bridge()
        .filter_map(|res| {
            // Chequeo de cancelación barato cada N entradas; aquí no
            // iteramos con índice, así que se hace dentro del batch
            // insert más abajo.
            let entry = res.ok()?;
            FileMeta::from_entry(&entry)
        })
        .collect();

    // 2. Insertar en SQLite en lotes.
    let mut conn = db.blocking_lock();
    let tx = conn.transaction()?;

    // Insert con UPSERT semantics no necesarias: cada walk es una
    // sesión nueva. Usamos INSERT simple.
    let mut stmt = tx.prepare(
        "INSERT INTO files \
            (session_id, path, filename, ext, size_bytes, ctime_ns, btime_ns, mtime_ns, kind) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;

    for chunk in entries.chunks(INSERT_BATCH) {
        if cancel.is_cancelled() {
            // Hacemos rollback explícito antes de soltar el stmt y la tx.
            drop(stmt);
            tx.rollback().ok();
            return Err(AppError::Cancelled);
        }

        for meta in chunk {
            stmt.execute(rusqlite::params![
                session_id,
                meta.path,
                meta.filename,
                meta.ext,
                meta.size_bytes,
                meta.ctime_ns,
                meta.btime_ns,
                meta.mtime_ns,
                meta.kind.as_str(),
            ])?;
            total += 1;
            last_path = Some(meta.path.clone());
        }

        // Emisión throttled. El check de cancel anterior ya pasó, así
        // que es seguro emitir.
        emitter.emit_progress(
            app,
            &ScanProgress {
                session_id,
                phase: "walking",
                files_seen: total,
                total_estimate: Some(entries.len()),
                current_path: last_path.clone(),
            },
            false,
        )?;
    }

    drop(stmt);
    tx.commit()?;
    drop(conn); // liberar el Mutex antes de emitir el evento final

    let duration_ms = started.elapsed().as_millis() as u64;
    info!(session_id, total, duration_ms, "walk finalizado");

    // Emisión final (forzada) del progreso y del evento de cierre.
    emitter.emit_progress(
        app,
        &ScanProgress {
            session_id,
            phase: "walking",
            files_seen: total,
            total_estimate: Some(total),
            current_path: None,
        },
        true,
    )?;
    emitter.emit_completed(
        app,
        &ScanCompleted {
            session_id,
            total_files: total,
            duration_ms,
            status: "completed",
            error: None,
        },
    )?;

    Ok(WalkSummary { total_files: total, duration_ms })
}

/// Variante "fire-and-forget" usada por smoke tests: hace el walk en
/// el thread actual y loggea el resultado. No interactúa con Tauri.
#[allow(dead_code)]
pub fn walk_for_test(root: &Path) -> AppResult<Vec<FileMeta>> {
    let entries: Vec<FileMeta> = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|res| res.ok())
        .filter_map(|e| FileMeta::from_entry(&e))
        .collect();
    if entries.is_empty() {
        warn!("walk_for_test: 0 entradas en {}", root.display());
    }
    Ok(entries)
}
