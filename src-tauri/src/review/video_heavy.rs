//! Modo Vídeo y Peso (v0.2 bonus).
//!
//! Dos secciones en una sola query para la página ModeVideoAndHeavy:
//!
//! 1. **Vídeos duplicados** (mismo algoritmo que Modo 1 pero
//!    filtrado a `kind='video'`): GROUP BY (filename, ext,
//!    ctime_ns, size_bytes) HAVING n>1. Excluye los que ya tienen
//!    decisión.
//!
//! 2. **Top-N más pesados**: lista los N vídeos de la sesión
//!    ordenados por `size_bytes DESC`. Para cada uno, mostramos
//!    el "ahorro estimado" al comprimir al 70% (`size * 0.30`),
//!    que es un factor conservador típico para H.264 sin
//!    re-encoding perceptualmente distinto. **No ejecuta ffmpeg**:
//!    es sólo una estimación.
//!
//! Fase 6 (ffmpeg-sidecar) permitiría calcular el ahorro real
//! ejecutando una pasada de prueba; por ahora es una heurística.

use rusqlite::Connection;
use serde::Serialize;

use crate::detect::exact;
use crate::error::AppResult;

/// Factor de compresión estimado para H.264 1-pass "medium".
/// 0.30 = ahorra 30% del tamaño (es decir, el vídeo final ocupa
/// el 70% del original). Es un valor conservador.
pub const COMPRESSION_FACTOR: f64 = 0.30;

/// Cabecera de un grupo de vídeos duplicados por nombre.
pub type VideoDuplicateGroup = exact::ExactGroup;

/// Cabecera de un vídeo en el top-N pesados.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeavyVideo {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub size_bytes: i64,
    pub mtime_ns: i64,
    /// Ahorro estimado al comprimir al 70%: size * COMPRESSION_FACTOR.
    pub estimated_savings: i64,
}

/// Resultado combinado para la página ModeVideoAndHeavy.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoAndHeavyResult {
    pub duplicates: Vec<VideoDuplicateGroup>,
    pub heavy: Vec<HeavyVideo>,
    pub total_duplicate_bytes: i64,
    pub total_potential_savings: i64,
    pub compression_factor: f64,
}

/// Ejecuta la consulta combinada. Por ahora no persiste nada
/// en la DB (los `decisions` que el usuario tome en la UI pasan
/// por los comandos `set_decision` / `bulk_set_decision` que ya
/// existían).
pub fn run(conn: &Connection, session_id: i64, top_n: i64) -> AppResult<VideoAndHeavyResult> {
    // 1. Vídeos duplicados por nombre (mismo algoritmo que
    //    Modo 1 pero filtrado a kind='video' y excluyendo ya
    //    decididos).
    let mut stmt = conn.prepare(
        "SELECT filename, ext, ctime_ns, size_bytes, COUNT(*) AS n \
         FROM files \
         WHERE session_id = ?1 \
           AND kind = 'video' \
           AND id NOT IN (SELECT file_id FROM decisions WHERE action != 'undecided') \
         GROUP BY filename, ext, ctime_ns, size_bytes \
         HAVING n > 1",
    )?;
    let dup_rows: Vec<(String, String, i64, i64, i64)> = stmt
        .query_map([session_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    // Para los duplicados, en vez de crear una fila en
    // `exact_groups` (que es para el Modo 1), los sintetizamos
    // aquí en memoria. El frontend los trata como Modo 1 pero
    // filtrados por vídeo. Si el usuario decide borrarlos, usa
    // set_decision igual.
    let duplicates: Vec<VideoDuplicateGroup> = dup_rows
        .into_iter()
        .map(|(filename, ext, ctime_ns, size_bytes, n)| VideoDuplicateGroup {
            // id: 0 → el frontend sabe que es "virtual" y no
            // intentará pedir members al backend. Si decide, usa
            // set_decision por file_id.
            id: 0,
            session_id,
            filename,
            ext,
            ctime_ns,
            size_bytes,
            member_count: n,
            reviewed: false,
        })
        .collect();
    let total_duplicate_bytes: i64 = duplicates
        .iter()
        .map(|g| g.size_bytes * (g.member_count - 1))
        .sum();

    // 2. Top-N más pesados, excluyendo ya decididos.
    let mut stmt = conn.prepare(
        "SELECT id, path, filename, size_bytes, mtime_ns \
         FROM files \
         WHERE session_id = ?1 \
           AND kind = 'video' \
           AND id NOT IN (SELECT file_id FROM decisions WHERE action != 'undecided') \
         ORDER BY size_bytes DESC, id ASC \
         LIMIT ?2",
    )?;
    let heavy: Vec<HeavyVideo> = stmt
        .query_map(rusqlite::params![session_id, top_n], |r| {
            let size: i64 = r.get(3)?;
            Ok(HeavyVideo {
                id: r.get(0)?,
                path: r.get(1)?,
                filename: r.get(2)?,
                size_bytes: size,
                mtime_ns: r.get(4)?,
                estimated_savings: (size as f64 * COMPRESSION_FACTOR) as i64,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    let total_potential_savings: i64 = heavy.iter().map(|h| h.estimated_savings).sum();

    Ok(VideoAndHeavyResult {
        duplicates,
        heavy,
        total_duplicate_bytes,
        total_potential_savings,
        compression_factor: COMPRESSION_FACTOR,
    })
}
