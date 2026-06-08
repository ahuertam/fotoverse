//! Borrado seguro vía la papelera del sistema operativo.
//!
//! Usa la crate `trash` (cross-platform):
//! - macOS: Finder (mueve a ~/.Trash)
//! - Windows: Recycle Bin
//! - Linux: `gio trash` o `~/.local/share/Trash/files/`
//!
//! **NUNCA** borra permanentemente. Si el SO no tiene papelera
//! (raro en 2026), `trash` cae en un error y NO borra.

use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;
use tracing::{info, warn};

use crate::error::AppResult;

/// Resultado de la ejecución de borrado.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionReport {
    /// Nº de ficheros enviados a la papelera OK.
    pub deleted: usize,
    /// Nº de ficheros que fallaron (con su error).
    pub failed: usize,
    /// Lista de (path, error) para reportar al usuario.
    pub failures: Vec<(String, String)>,
    /// Bytes totales liberados (suma de size_bytes de los OK).
    pub bytes_freed: i64,
}

/// Envía a la papelera los ficheros cuyos file_ids están en
/// `file_ids`. Actualiza la DB para reflejar el borrado.
pub fn execute(
    conn: &Connection,
    file_ids: &[i64],
) -> AppResult<DeletionReport> {
    if file_ids.is_empty() {
        return Ok(DeletionReport {
            deleted: 0,
            failed: 0,
            failures: Vec::new(),
            bytes_freed: 0,
        });
    }

    // 1. Cargar paths y sizes de los file_ids solicitados.
    let placeholders = std::iter::repeat("?")
        .take(file_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT id, path, size_bytes FROM files WHERE id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> = file_ids
        .iter()
        .map(|i| i as &dyn rusqlite::ToSql)
        .collect();
    let rows: Vec<(i64, String, i64)> = stmt
        .query_map(params.as_slice(), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    info!(
        "execute_deletions: {} ficheros solicitados",
        rows.len()
    );

    // 2. Enviar a la papelera uno a uno para reportar fallos
    //    individuales.
    let mut deleted = 0usize;
    let mut failed = 0usize;
    let mut failures = Vec::new();
    let mut bytes_freed = 0i64;

    for (id, path, size) in &rows {
        let p = Path::new(path);
        match trash::delete_all(&[p]) {
            Ok(()) => {
                deleted += 1;
                bytes_freed += size;
            }
            Err(e) => {
                // Si el fichero ya no existe (porque ya está
                // en la papelera de un intento anterior), lo
                // contamos como éxito.
                if !p.exists() {
                    deleted += 1;
                    bytes_freed += size;
                } else {
                    failed += 1;
                    failures.push((path.clone(), e.to_string()));
                    warn!(file_id = id, error = %e, "no se pudo enviar a la papelera");
                }
            }
        }
    }

    info!(
        deleted, failed, bytes_freed,
        "execute_deletions: borrado completado"
    );

    // 3. Actualizar la DB: borrar decisiones y files. Cascade
    //    borra manual_pairs y perceptual_groups.
    let tx = conn.unchecked_transaction()?;
    {
        let del_ph = std::iter::repeat("?")
            .take(file_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let params: Vec<&dyn rusqlite::ToSql> = file_ids
            .iter()
            .map(|i| i as &dyn rusqlite::ToSql)
            .collect();
        let sql = format!("DELETE FROM decisions WHERE file_id IN ({del_ph})");
        tx.execute(&sql, params.as_slice())?;
        let sql = format!("DELETE FROM files WHERE id IN ({del_ph})");
        tx.execute(&sql, params.as_slice())?;
    }
    tx.commit()?;

    Ok(DeletionReport {
        deleted,
        failed,
        failures,
        bytes_freed,
    })
}

/// Re-export del ejecutor (compatibilidad con el módulo superior).
pub use execute as run;

/// Umbral por encima del cual exigimos confirmación tipada.
/// 1000 ficheros o 5 GB → modal con "ELIMINAR" que el usuario
/// debe escribir.
pub const THRESHOLD_FILES: usize = 1000;
pub const THRESHOLD_BYTES: i64 = 5 * 1024 * 1024 * 1024;

/// Devuelve `true` si el set de borrado supera el umbral y
/// requiere confirmación tipada.
#[allow(dead_code)]
pub fn requires_typed_confirmation(n_files: usize, n_bytes: i64) -> bool {
    n_files > THRESHOLD_FILES || n_bytes > THRESHOLD_BYTES
}
