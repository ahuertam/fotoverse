//! Generación de pares para revisión manual (Modo 3) — Fase 7.
//!
//! Estrategia: para cada sesión, generamos **todos los pares posibles**
//! de archivos no decididos (imágenes y vídeos), excluyendo los
//! que ya están en el mismo grupo de Modo 1 (esos se deciden en
//! la página Modo 1, no aquí).
//!
//! Para 50 000 archivos, el nº de pares es ~1.25B — completamente
//! inviable. Fase 7 sólo va a usar el sample del usuario para
//! probar la UI. Fase 8+ lo sustituirá por pares derivados de los
//! clusters de Modo 2 (los "borderline" cerca del threshold).
//!
//! Reglas de filtrado por par (ambos deben cumplirse para incluirlo):
//! - Ambos archivos existen en la sesión
//! - Ninguno tiene decisión (`keep`/`delete`); `undecided` sí cuenta
//! - No están en el mismo grupo de Modo 1 (`exact_groups`)
//! - No son el mismo archivo (i ≠ j)
//! - `file_a.id < file_b.id` para evitar duplicados `(a,b)` y `(b,a)`

use rusqlite::Connection;
use serde::Serialize;
use tracing::{debug, info, warn};

use crate::error::AppResult;

/// Cabecera de un par (espejo del struct TS).
#[derive(Debug, Clone, Serialize)]
pub struct ManualPair {
    pub id: i64,
    pub session_id: i64,
    pub file_a: i64,
    pub file_b: i64,
    pub reviewed: bool,
}

/// Puebla `manual_pairs` para una sesión. Borra los pares previos
/// (idempotente).
pub fn populate(conn: &Connection, session_id: i64) -> AppResult<usize> {
    // 1. Listar file_ids de la sesión sin decisión terminal.
    let mut stmt = conn.prepare(
        "SELECT id FROM files \
         WHERE session_id = ?1 \
           AND id NOT IN (SELECT file_id FROM decisions WHERE action != 'undecided')",
    )?;
    let ids: Vec<i64> = stmt
        .query_map([session_id], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    if ids.len() < 2 {
        debug!(session_id, n = ids.len(), "pocos archivos, no generamos pares");
        return Ok(0);
    }
    if ids.len() > 200 {
        warn!(
            session_id, n = ids.len(),
            "manual::populate generará O(n²) pares; Fase 8 lo limitará a los cluster borderlines de Modo 2"
        );
    }

    // 2. Set de file_ids en el mismo grupo de Modo 1 (para excluirlos).
    //    Como Modo 1 los agrupa por (filename, ext, ctime_ns, size), los
    //    hard links (mismo path semántico) caen juntos. Aquí no nos
    //    importa el nombre, sólo el "están en el mismo grupo".
    let mut stmt = conn.prepare(
        "SELECT id, filename, ext, ctime_ns, size_bytes \
         FROM files WHERE session_id = ?1",
    )?;
    let rows: Vec<(i64, String, String, i64, i64)> = stmt
        .query_map([session_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    use std::collections::HashMap;
    let mut by_group: HashMap<(String, String, i64, i64), Vec<i64>> = HashMap::new();
    for (id, fn_, ext, ctime, size) in rows {
        by_group
            .entry((fn_, ext, ctime, size))
            .or_default()
            .push(id);
    }
    let mut in_same_exact_group: std::collections::HashSet<(i64, i64)> =
        std::collections::HashSet::new();
    for (_k, members) in by_group {
        if members.len() < 2 {
            continue;
        }
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let (a, b) = if members[i] < members[j] {
                    (members[i], members[j])
                } else {
                    (members[j], members[i])
                };
                in_same_exact_group.insert((a, b));
            }
        }
    }

    // 3. Generar todos los pares (i, j) con i < j.
    let mut pairs: Vec<(i64, i64)> = Vec::new();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let a = ids[i].min(ids[j]);
            let b = ids[i].max(ids[j]);
            if in_same_exact_group.contains(&(a, b)) {
                continue;
            }
            pairs.push((a, b));
        }
    }
    let n_pairs = pairs.len();
    info!(
        session_id, files = ids.len(), pairs = n_pairs,
        "manual::populate generando pares"
    );

    // 4. INSERT en `manual_pairs`. Borramos los previos primero
    //    para que populate sea idempotente.
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM manual_pairs WHERE session_id = ?1",
        [session_id],
    )?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO manual_pairs (session_id, file_a, file_b, reviewed) \
             VALUES (?1, ?2, ?3, 0)",
        )?;
        for (a, b) in &pairs {
            stmt.execute(rusqlite::params![session_id, a, b])?;
        }
    }
    tx.commit()?;

    Ok(n_pairs)
}

/// Lista paginada de pares, filtrando por estado.
pub fn list_paginated(
    conn: &Connection,
    session_id: i64,
    filter: PairFilter,
    offset: i64,
    limit: i64,
) -> AppResult<(Vec<ManualPair>, i64)> {
    let where_extra = match filter {
        PairFilter::Pending => " AND reviewed = 0",
        PairFilter::All => "",
    };
    let total: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM manual_pairs WHERE session_id = ?1{}",
            where_extra
        ),
        [session_id],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(&format!(
        "SELECT id, session_id, file_a, file_b, reviewed \
         FROM manual_pairs \
         WHERE session_id = ?1{} \
         ORDER BY id ASC \
         LIMIT ?2 OFFSET ?3",
        where_extra
    ))?;
    let rows = stmt
        .query_map(rusqlite::params![session_id, limit, offset], |r| {
            Ok(ManualPair {
                id: r.get(0)?,
                session_id: r.get(1)?,
                file_a: r.get(2)?,
                file_b: r.get(3)?,
                reviewed: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok((rows, total))
}

/// Filtro para la lista de pares.
#[derive(Debug, Clone, Copy)]
pub enum PairFilter {
    Pending,
    All,
}

impl std::str::FromStr for PairFilter {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "all" => Self::All,
            _ => Self::Pending,
        })
    }
}

/// Marca un par como revisado (saltar o decidir).
pub fn mark_reviewed(conn: &Connection, pair_id: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE manual_pairs SET reviewed = 1 WHERE id = ?1",
        [pair_id],
    )?;
    Ok(())
}
