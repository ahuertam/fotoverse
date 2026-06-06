//! Detección de duplicados perceptuales (Modo 2) — Fase 5.
//!
//! Estrategia:
//! 1. Cargar todas las imágenes con `phash` no-NULL y `hash_status='ok'`,
//!    **excluyendo** los file_ids que ya tienen decisión (no se
//!    re-surfacean en Modo 2).
//! 2. Union-Find para clustering: para cada par (i, j) con Hamming ≤ 10,
//!    unirlos en el mismo cluster. Representante = menor file_id.
//! 3. INSERT en `perceptual_groups` (representante, hamming=0 por
//!    construcción) y los miembros se reconstruyen por JOIN en tiempo
//!    de query (más simple que almacenarlos).
//!
//! Naïve O(n²) en nº de imágenes. Para 50k imágenes con Hamming ≤ 10
//! en hashes de 64 bits, es manejable (~1.25B comparaciones, pero
//! con poda por early-exit cuando la distancia parcial supera 10).
//!
//! Si el nº de imágenes > 5 000, loggeamos un warning y Fase 8
//! sustituirá este loop por un BK-tree (ver ARCHITECTURE.md §5.2).

use rusqlite::Connection;
use serde::Serialize;
use tracing::{debug, info, warn};

use crate::error::AppResult;
use crate::hash::image_hash::{hamming_distance, HAMMING_THRESHOLD, HASH_BYTES};

/// Cabecera de un grupo perceptual (espejo del struct TS).
#[derive(Debug, Clone, Serialize)]
pub struct PerceptualGroup {
    pub id: i64,
    pub session_id: i64,
    pub representative: i64,
    pub hamming_to_rep: i64,
    pub member_count: i64,
    pub reviewed: bool,
}

/// Miembro de un grupo perceptual. `hamming_to_rep` es 0 para el
/// representante, y la distancia Hamming al representante para el
/// resto. `suggested_keep_id` siempre es el representante.
#[derive(Debug, Clone, Serialize)]
pub struct PerceptualGroupMember {
    pub id: i64,
    pub path: String,
    pub size_bytes: i64,
    pub ctime_ns: Option<i64>,
    pub btime_ns: Option<i64>,
    pub mtime_ns: i64,
    pub decision: Option<String>,
    pub hamming_to_rep: i64,
    pub suggested_keep_id: i64,
}

/// Pobla `perceptual_groups` para una sesión. Borra los grupos
/// previos de esa sesión y los regenera.
pub fn populate(conn: &Connection, session_id: i64) -> AppResult<usize> {
    // 1. Listar imágenes hasheadas, EXCLUYENDO las que ya tienen
    //    decisión (no se re-surfacean en Modo 2).
    let mut stmt = conn.prepare(
        "SELECT id, phash FROM files \
         WHERE session_id = ?1 \
           AND kind = 'image' \
           AND hash_status = 'ok' \
           AND phash IS NOT NULL \
           AND id NOT IN (SELECT file_id FROM decisions)",
    )?;
    let rows: Vec<(i64, Vec<u8>)> = stmt
        .query_map([session_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    let n = rows.len();
    if n < 2 {
        debug!(session_id, n, "perceptual::populate: pocas imágenes, nada que agrupar");
        return Ok(0);
    }
    if n > 5000 {
        warn!(
            session_id, n,
            "perceptual::populate O(n²) sobre >5k imágenes; Fase 8 lo optimizará con BK-tree"
        );
    }

    // 2. Convertir a [u8; 8] para hamming_distance.
    let mut items: Vec<(i64, [u8; HASH_BYTES])> = Vec::with_capacity(n);
    for (id, bytes) in rows {
        if bytes.len() != HASH_BYTES {
            warn!(file_id = id, "phash con longitud inesperada, saltando");
            continue;
        }
        let mut h = [0u8; HASH_BYTES];
        h.copy_from_slice(&bytes);
        items.push((id, h));
    }

    // 3. Union-Find ingenuo. parent[i] = i inicialmente. find con
    //    path compression parcial. Unión siempre al menor file_id (será
    //    el representante).
    let mut parent: Vec<usize> = (0..items.len()).collect();

    // 4. O(n²) con poda temprana: para cada par, si los primeros
    //    `partial_k` bits XORados ya superan el threshold, skip.
    let mut comparisons = 0u64;
    let mut unions = 0u32;
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            // Poda: si el primer byte difiere en más de 10 bits,
            // imposible que estén dentro del threshold. En la práctica
            // para pHash de 64 bits, sólo ahorramos ~1%.
            if (items[i].1[0] ^ items[j].1[0]).count_ones() > HAMMING_THRESHOLD {
                continue;
            }
            let d = hamming_distance(&items[i].1, &items[j].1);
            comparisons += 1;
            if d <= HAMMING_THRESHOLD {
                let ri = find(&mut parent, i);
                let rj = find(&mut parent, j);
                if ri != rj {
                    // Unir eligiendo el de menor file_id como raíz.
                    if items[i].0 < items[j].0 {
                        parent[rj] = ri;
                    } else {
                        parent[ri] = rj;
                    }
                    unions += 1;
                }
            }
        }
    }
    debug!(
        session_id, total = n, comparisons, unions,
        "clustering perceptual completado"
    );

    // 5. Recolectar clusters y elegir representante (menor file_id).
    let mut cluster_map: std::collections::HashMap<usize, i64> =
        std::collections::HashMap::new();
    for (idx, (id, _)) in items.iter().enumerate() {
        let root = find(&mut parent, idx);
        cluster_map
            .entry(root)
            .and_modify(|rep| {
                if *id < *rep {
                    *rep = *id;
                }
            })
            .or_insert(*id);
    }
    // Filtrar clusters con un solo miembro (no son duplicados).
    let mut cluster_members: std::collections::HashMap<i64, Vec<i64>> =
        std::collections::HashMap::new();
    for (idx, (id, _)) in items.iter().enumerate() {
        let root = find(&mut parent, idx);
        if let Some(rep) = cluster_map.get(&root) {
            cluster_members.entry(*rep).or_default().push(*id);
        }
    }
    let clusters: Vec<(i64, Vec<i64>)> = cluster_members
        .into_iter()
        .filter(|(_, members)| members.len() > 1)
        .collect();

    // 6. INSERT en `perceptual_groups`. Borramos los previos primero
    //    para que populate sea idempotente.
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM perceptual_groups WHERE session_id = ?1",
        [session_id],
    )?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO perceptual_groups \
                (session_id, representative, hamming_to_rep, member_count, reviewed) \
             VALUES (?1, ?2, 0, ?3, 0)",
        )?;
        for (rep, members) in &clusters {
            stmt.execute(rusqlite::params![session_id, rep, members.len() as i64])?;
        }
    }
    tx.commit()?;

    info!(
        session_id, clusters = clusters.len(),
        "perceptual::populate: {} grupos creados", clusters.len()
    );
    Ok(clusters.len())
}

/// Lista paginada de grupos (ordenados por nº de miembros DESC).
pub fn list_paginated(
    conn: &Connection,
    session_id: i64,
    offset: i64,
    limit: i64,
) -> AppResult<(Vec<PerceptualGroup>, i64)> {
    info!(
        session_id, offset, limit,
        "perceptual::list_paginado invocado"
    );
    // Total
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM perceptual_groups WHERE session_id = ?1",
        [session_id],
        |r| r.get(0),
    )?;

    // Lista
    let mut stmt = conn.prepare(
        "SELECT id, session_id, representative, hamming_to_rep, member_count, reviewed \
         FROM perceptual_groups \
         WHERE session_id = ?1 \
         ORDER BY member_count DESC, id ASC \
         LIMIT ?2 OFFSET ?3",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![session_id, limit, offset], |r| {
            Ok(PerceptualGroup {
                id: r.get(0)?,
                session_id: r.get(1)?,
                representative: r.get(2)?,
                hamming_to_rep: r.get(3)?,
                member_count: r.get(4)?,
                reviewed: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok((rows, total))
}

/// Miembros de un grupo perceptual: el representante más todos los
/// files de la misma sesión con phash a distancia ≤ threshold.
pub fn members(conn: &Connection, group_id: i64) -> AppResult<Vec<PerceptualGroupMember>> {
    // 1. Obtener rep + phash del representante.
    let (rep_id, rep_phash): (i64, Vec<u8>) = conn.query_row(
        "SELECT pg.representative, f.phash \
         FROM perceptual_groups pg \
         JOIN files f ON f.id = pg.representative \
         WHERE pg.id = ?1",
        [group_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if rep_phash.len() != HASH_BYTES {
        return Ok(Vec::new());
    }
    let mut rep_h = [0u8; HASH_BYTES];
    rep_h.copy_from_slice(&rep_phash);

    // 2. Todos los files de la sesión con phash válido, no decididos,
    //    cuya distancia al rep sea ≤ threshold.
    let mut stmt = conn.prepare(
        "SELECT f.id, f.path, f.size_bytes, f.ctime_ns, f.btime_ns, f.mtime_ns, \
                f.phash, d.action \
         FROM files f \
         LEFT JOIN decisions d ON d.file_id = f.id \
         WHERE f.session_id = (SELECT session_id FROM perceptual_groups WHERE id = ?1) \
           AND f.kind = 'image' \
           AND f.hash_status = 'ok' \
           AND f.phash IS NOT NULL \
           AND f.id NOT IN (SELECT file_id FROM decisions WHERE action != 'undecided')",
    )?;
    let candidates: Vec<(i64, String, i64, Option<i64>, Option<i64>, i64, Vec<u8>, Option<String>)> =
        stmt.query_map([group_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    // 3. Filtrar por Hamming y construir la lista.
    let mut out = Vec::new();
    for (id, path, size, ctime, btime, mtime, phash, decision) in candidates {
        if phash.len() != HASH_BYTES {
            continue;
        }
        let mut h = [0u8; HASH_BYTES];
        h.copy_from_slice(&phash);
        let d = hamming_distance(&h, &rep_h);
        if d > HAMMING_THRESHOLD {
            continue;
        }
        out.push(PerceptualGroupMember {
            id,
            path,
            size_bytes: size,
            ctime_ns: ctime,
            btime_ns: btime,
            mtime_ns: mtime,
            decision,
            hamming_to_rep: d as i64,
            suggested_keep_id: rep_id,
        });
    }

    // 4. Ordenar: representante primero, luego por Hamming ASC.
    out.sort_by_key(|m| (m.id != rep_id, m.hamming_to_rep));
    Ok(out)
}

/// Find con path compression parcial.
fn find(parent: &mut Vec<usize>, mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}
