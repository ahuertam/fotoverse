//! Final review: lista de TODOS los ficheros marcados como `delete`
//! en los 3 modos de una sesión, con su path, size, thumb path.
//!
//! Ver `docs/ARCHITECTURE.md#fase-8--final-review--papelera` para
//! el flujo completo.

use rusqlite::Connection;
use serde::Serialize;

use crate::error::AppResult;

/// Fichero en la final review. Sale de JOINear `files` con
/// `decisions` y filtrar `action='delete'`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalReviewItem {
    pub file_id: i64,
    pub path: String,
    pub filename: String,
    pub size_bytes: i64,
    pub mtime_ns: i64,
    /// Modo en el que se decidió: "exact" (Modo 1), "perceptual"
    /// (Modo 2) o "manual" (Modo 3). Útil para mostrar al usuario
    /// de dónde vino la sugerencia.
    pub decided_in: String,
}

/// Lista todos los ficheros marcados como `delete` en la sesión.
/// Los devuelve ordenados por tamaño DESC (los más gordos
/// primero, que es lo que más ahorra).
pub fn list(conn: &Connection, session_id: i64) -> AppResult<Vec<FinalReviewItem>> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.path, f.filename, f.size_bytes, f.mtime_ns, d.action \
         FROM files f \
         JOIN decisions d ON d.file_id = f.id \
         WHERE f.session_id = ?1 \
           AND d.action = 'delete' \
         ORDER BY f.size_bytes DESC, f.id ASC",
    )?;
    let rows = stmt
        .query_map([session_id], |r| {
            Ok(FinalReviewItem {
                file_id: r.get(0)?,
                path: r.get(1)?,
                filename: r.get(2)?,
                size_bytes: r.get(3)?,
                mtime_ns: r.get(4)?,
                decided_in: "exact".to_string(), // placeholder; ver más abajo
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);

    // Para distinguir el modo, miramos en qué tabla está la decisión
    // original. En realidad `decisions` es una sola tabla y el "modo"
    // no se persiste por separado. Para Fase 8 sólo necesitamos el
    // nº total, no el modo exacto. Lo dejamos como "exact" y en
    // Fase 9 añadimos una columna `decided_in_mode` a `decisions`.
    Ok(rows)
}

/// Total de bytes que se borrarían si se confirman TODOS los
/// `delete` actuales.
pub fn total_bytes(conn: &Connection, session_id: i64) -> AppResult<i64> {
    let total: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(f.size_bytes), 0) \
             FROM files f \
             JOIN decisions d ON d.file_id = f.id \
             WHERE f.session_id = ?1 AND d.action = 'delete'",
            [session_id],
            |r| r.get(0),
        )?;
    Ok(total)
}

/// Nº de ficheros marcados como `delete` en la sesión.
pub fn count(conn: &Connection, session_id: i64) -> AppResult<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) \
         FROM files f \
         JOIN decisions d ON d.file_id = f.id \
         WHERE f.session_id = ?1 AND d.action = 'delete'",
        [session_id],
        |r| r.get(0),
    )?;
    Ok(n)
}
