//! Modo 1 — Detección de duplicados exactos.
//!
//! Regla: dos archivos son duplicados si tienen el MISMO basename
//! (case-folded), la MISMA extensión, el MISMO ctime (al segundo)
//! y el MISMO tamaño en bytes.
//!
//! La detección se hace con un único `GROUP BY` sobre la tabla `files`
//! de la sesión; los grupos resultantes se persisten en `exact_groups`
//! (cabecera) y se reconstruyen los miembros por JOIN en tiempo de
//! consulta (las cardinalidades son bajas).
//!
//! Ver `docs/ARCHITECTURE.md#modo-1--exacto`.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::error::AppResult;

/// Cabecera de un grupo de duplicados exactos (espejo TS: `ExactGroup`).
#[derive(Debug, Clone, Serialize)]
pub struct ExactGroup {
    pub id: i64,
    pub session_id: i64,
    pub filename: String,
    pub ext: String,
    pub ctime_ns: i64,
    pub size_bytes: i64,
    pub member_count: i64,
    pub reviewed: bool,
}

/// Miembro de un grupo (espejo TS: `ExactGroupMember`).
#[derive(Debug, Clone, Serialize)]
pub struct ExactGroupMember {
    pub id: i64,            // files.id
    pub path: String,
    pub size_bytes: i64,
    pub ctime_ns: Option<i64>,
    pub btime_ns: Option<i64>,
    pub mtime_ns: i64,
    /// Decisión actual del usuario (None si todavía no hay).
    pub decision: Option<String>,
    /// Sugerencia: el id del miembro con btime_ns más antiguo
    /// (None si todos tienen btime_ns NULL, en cuyo caso se usa mtime).
    pub suggested_keep_id: i64,
}

/// Puebla `exact_groups` para una sesión: ejecuta el GROUP BY y crea
/// una fila por grupo con `member_count > 1`.
///
/// Devuelve el número de grupos creados.
pub fn populate(conn: &Connection, session_id: i64) -> AppResult<usize> {
    // Limpia grupos previos de esta sesión (idempotencia).
    conn.execute(
        "DELETE FROM exact_groups WHERE session_id = ?1",
        [session_id],
    )?;

    // Inserta un grupo por cada combinación (filename, ext, ctime, size)
    // con más de un miembro.
    let n = conn.execute(
        "INSERT INTO exact_groups \
            (session_id, filename, ext, ctime_ns, size_bytes, member_count) \
         SELECT session_id, filename, ext, ctime_ns, size_bytes, COUNT(*) \
         FROM files \
         WHERE session_id = ?1 \
         GROUP BY filename, ext, ctime_ns, size_bytes \
         HAVING COUNT(*) > 1",
        [session_id],
    )?;
    Ok(n)
}

/// Lista paginada de grupos, ordenados por tamaño desperdiciado
/// (mayor primero) — así el usuario ve primero lo que más le conviene
/// limpiar.
pub fn list_paginated(
    conn: &Connection,
    session_id: i64,
    offset: i64,
    limit: i64,
) -> AppResult<(Vec<ExactGroup>, i64)> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM exact_groups WHERE session_id = ?1",
        [session_id],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, session_id, filename, ext, ctime_ns, size_bytes, member_count, reviewed \
         FROM exact_groups \
         WHERE session_id = ?1 \
         ORDER BY (size_bytes * (member_count - 1)) DESC, id ASC \
         LIMIT ?2 OFFSET ?3",
    )?;
    let groups = stmt
        .query_map(params![session_id, limit, offset], row_to_group)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok((groups, total))
}

fn row_to_group(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExactGroup> {
    Ok(ExactGroup {
        id: row.get(0)?,
        session_id: row.get(1)?,
        filename: row.get(2)?,
        ext: row.get(3)?,
        ctime_ns: row.get(4)?,
        size_bytes: row.get(5)?,
        member_count: row.get(6)?,
        reviewed: row.get::<_, i64>(7)? != 0,
    })
}

/// Devuelve los miembros de un grupo, ordenados por `btime_ns` ASC
/// (el más antiguo primero, que es el candidato a "mantener"). Incluye
/// la decisión actual de cada uno y el `suggested_keep_id`.
pub fn members(conn: &Connection, group_id: i64) -> AppResult<Vec<ExactGroupMember>> {
    // Recupera los datos básicos del grupo (necesitamos la key).
    let group: Option<ExactGroup> = conn
        .query_row(
            "SELECT id, session_id, filename, ext, ctime_ns, size_bytes, member_count, reviewed \
             FROM exact_groups WHERE id = ?1",
            [group_id],
            row_to_group,
        )
        .optional()?;
    let Some(g) = group else {
        return Ok(Vec::new());
    };

    let mut stmt = conn.prepare(
        "SELECT f.id, f.path, f.size_bytes, f.ctime_ns, f.btime_ns, f.mtime_ns, d.action \
         FROM files f \
         LEFT JOIN decisions d ON d.file_id = f.id \
         WHERE f.session_id = ?1 \
           AND f.filename = ?2 \
           AND f.ext = ?3 \
           AND f.ctime_ns = ?4 \
           AND f.size_bytes = ?5 \
         ORDER BY \
           CASE WHEN f.btime_ns IS NULL THEN 1 ELSE 0 END, \
           f.btime_ns ASC, \
           f.mtime_ns ASC, \
           f.id ASC",
    )?;

    let members: Vec<ExactGroupMember> = stmt
        .query_map(
            params![g.session_id, g.filename, g.ext, g.ctime_ns, g.size_bytes],
            |row| {
                Ok(ExactGroupMember {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    size_bytes: row.get(2)?,
                    ctime_ns: row.get(3)?,
                    btime_ns: row.get(4)?,
                    mtime_ns: row.get(5)?,
                    decision: row.get(6)?,
                    suggested_keep_id: 0, // se rellena abajo
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // El primero (tras ORDER BY) es el sugerido a mantener.
    let suggested_keep_id = members.first().map(|m| m.id).unwrap_or(0);
    Ok(members
        .into_iter()
        .map(|mut m| {
            m.suggested_keep_id = suggested_keep_id;
            m
        })
        .collect())
}
