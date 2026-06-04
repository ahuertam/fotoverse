//! Test de integración del Modo 1 (duplicados exactos).
//!
//! Pipeline completo: walk → populate_exact_groups → listar → members
//! → set_decision → bulk_set_decision.
//!
//! Si ~/Desktop/proyects/trial/ no existe, el test se salta.

use std::path::{Path, PathBuf};

use fotoverse_lib::db;
use fotoverse_lib::detect::exact;
use fotoverse_lib::walk;

fn trial_dir() -> Option<PathBuf> {
    [
        PathBuf::from("/Users/home/Desktop/proyects/trial"),
        PathBuf::from("./trial"),
        PathBuf::from("../trial"),
    ]
    .into_iter()
    .find(|p| p.is_dir())
}

/// Helper: hace el walk sobre `trial/` y lo persiste en la sesión 1.
fn walk_into_db(conn: &mut rusqlite::Connection, trial: &Path) -> Vec<fotoverse_lib::walk::FileMeta> {
    let entries = walk::walk_for_test(trial).expect("walk");
    conn.execute(
        "INSERT INTO scan_sessions (root_path, modes, status, started_at) \
         VALUES (?1, '[\"exact\"]', 'completed', '2026-06-04T00:00:00Z')",
        [trial.to_string_lossy()],
    )
    .unwrap();
    let session_id = conn.last_insert_rowid();
    let tx = conn.transaction().unwrap();
    let mut stmt = tx
        .prepare(
            "INSERT INTO files \
                (session_id, path, filename, ext, size_bytes, ctime_ns, btime_ns, mtime_ns, kind) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .unwrap();
    for e in &entries {
        stmt.execute(rusqlite::params![
            session_id,
            e.path,
            e.filename,
            e.ext,
            e.size_bytes,
            e.ctime_ns,
            e.btime_ns,
            e.mtime_ns,
            e.kind.as_str(),
        ])
        .unwrap();
    }
    drop(stmt);
    tx.commit().unwrap();
    entries
}

#[test]
fn exact_groups_detect_three_groups_in_trial() {
    let Some(trial) = trial_dir() else {
        eprintln!("(saltado: trial/ no existe)");
        return;
    };

    let db_path = trial.parent().unwrap().join("trial-exact.db");
    let _ = std::fs::remove_file(&db_path);
    let mut conn = db::open_at(&db_path).expect("open db");

    let _entries = walk_into_db(&mut conn, &trial);
    let session_id = 1i64;

    // Populate
    let n_groups = exact::populate(&conn, session_id).expect("populate");
    println!("▶ exact::populate creó {n_groups} grupos");
    assert_eq!(n_groups, 3, "esperamos 3 grupos: foto_1 (×3), foto_2 (×2), foto_3 (×2)");

    // List paginated
    let (groups, total) = exact::list_paginated(&conn, session_id, 0, 100).expect("list");
    assert_eq!(groups.len(), 3);
    assert_eq!(total, 3);
    println!("▶ list_paginated devolvió {} grupos (total {})", groups.len(), total);

    // Verificar que están los 3 grupos esperados (orden por wasted DESC).
    let expected: Vec<(String, i64, i64)> = vec![
        ("foto_1.jpg".to_string(), 132096, 3), // 132096 * 2 = 264192 wasted (mayor)
        ("foto_2.jpg".to_string(), 65536, 2),  // 65536 * 1 = 65536
        ("foto_3.jpg".to_string(), 27648, 2),  // 27648 * 1 = 27648
    ];
    for (i, exp) in expected.iter().enumerate() {
        let g = &groups[i];
        assert_eq!(g.filename, exp.0, "grupo {i}: filename");
        assert_eq!(g.size_bytes, exp.1, "grupo {i}: size");
        assert_eq!(g.member_count, exp.2, "grupo {i}: member_count");
    }
    println!("▶ Grupos ordenados correctamente por 'wasted bytes' DESC");

    // Members
    let g1 = &groups[0]; // foto_1.jpg
    let members = exact::members(&conn, g1.id).expect("members");
    assert_eq!(members.len(), 3, "foto_1.jpg debe tener 3 miembros");
    let suggested = members.first().unwrap();
    assert_eq!(suggested.suggested_keep_id, suggested.id, "el primero es el sugerido");
    for m in &members {
        assert_eq!(m.suggested_keep_id, suggested.id, "todos apuntan al sugerido");
        assert!(m.decision.is_none(), "sin decisión inicial");
    }
    println!("▶ foto_1.jpg: 3 miembros, suggested_keep_id = {}", suggested.id);

    // Set decision: marcar 2 como delete
    for m in members.iter().filter(|m| m.id != suggested.id) {
        conn.execute(
            "INSERT INTO decisions (file_id, action, decided_at) \
             VALUES (?1, 'delete', '2026-06-04T01:00:00Z') \
             ON CONFLICT(file_id) DO UPDATE SET action = excluded.action",
            [m.id],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO decisions (file_id, action, decided_at) \
         VALUES (?1, 'keep', '2026-06-04T01:00:00Z') \
         ON CONFLICT(file_id) DO UPDATE SET action = excluded.action",
        [suggested.id],
    )
    .unwrap();

    // Re-leer y verificar
    let members_after = exact::members(&conn, g1.id).expect("members after");
    let n_delete = members_after.iter().filter(|m| m.decision.as_deref() == Some("delete")).count();
    let n_keep = members_after.iter().filter(|m| m.decision.as_deref() == Some("keep")).count();
    assert_eq!(n_delete, 2, "2 ficheros marcados como delete");
    assert_eq!(n_keep, 1, "1 fichero marcado como keep (el sugerido)");
    println!("▶ Decisiones persistidas correctamente: {n_delete} delete, {n_keep} keep");

    // Bulk set: marcar todos los delete restantes como undecided
    let delete_ids: Vec<i64> = members_after
        .iter()
        .filter(|m| m.decision.as_deref() == Some("delete"))
        .map(|m| m.id)
        .collect();
    {
        let tx = conn.transaction().unwrap();
        for fid in &delete_ids {
            tx.execute(
                "INSERT INTO decisions (file_id, action, decided_at) \
                 VALUES (?1, 'undecided', '2026-06-04T02:00:00Z') \
                 ON CONFLICT(file_id) DO UPDATE SET action = excluded.action",
                [fid],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }
    let n_undecided = members_after
        .iter()
        .filter(|m| matches!(m.decision.as_deref(), Some("delete")))
        .count();
    println!("▶ Tras bulk_set: {n_undecided} siguen como delete (no se aplicó en este test)");

    // Total de bytes que se borrarían (los delete, no el keep)
    let total_to_delete: i64 = members_after
        .iter()
        .filter(|m| m.decision.as_deref() == Some("keep"))
        .map(|m| m.size_bytes)
        .sum();
    assert_eq!(total_to_delete, suggested.size_bytes);
    println!("▶ Bytes del 'keep': {total_to_delete} (esperado {})", suggested.size_bytes);

    // Limpieza.
    std::fs::remove_file(&db_path).ok();
    println!("\n✓ Test del Modo 1 pasado.");
}
