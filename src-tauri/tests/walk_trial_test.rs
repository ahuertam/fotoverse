//! Test de integración que ejercita el pipeline real sobre el directorio
//! `~/Desktop/proyects/trial/` (fixture sintética del usuario).
//!
//! Si la carpeta no existe (por ejemplo, en CI), el test se salta
//! silenciosamente con un mensaje informativo.
//!
//! Ejecutar: `cargo test --test walk_trial_test -- --nocapture`

use std::path::PathBuf;
use std::time::Instant;

use fotoverse_lib::db;
use fotoverse_lib::walk::{self, FileKind};

fn trial_dir() -> Option<PathBuf> {
    // Buscamos trial/ en varias ubicaciones probables.
    let candidates = [
        PathBuf::from("/Users/home/Desktop/proyects/trial"),
        PathBuf::from("./trial"),
        PathBuf::from("../trial"),
    ];
    candidates.into_iter().find(|p| p.is_dir())
}

#[test]
fn scan_trial_directory() {
    let Some(trial) = trial_dir() else {
        eprintln!("(saltado: ~/Desktop/proyects/trial/ no existe)");
        return;
    };
    println!("\n▶ Escaneando {}", trial.display());

    // 1. Walk: contar lo descubierto.
    let started = Instant::now();
    let entries = walk::walk_for_test(&trial).expect("walk");
    let walk_ms = started.elapsed().as_millis();

    let n_jpg = entries.iter().filter(|e| e.ext == "jpg").count();
    let n_png = entries.iter().filter(|e| e.ext == "png").count();
    let n_webp = entries.iter().filter(|e| e.ext == "webp").count();
    let n_img = entries.iter().filter(|e| matches!(e.kind, FileKind::Image)).count();
    let n_vid = entries.iter().filter(|e| matches!(e.kind, FileKind::Video)).count();

    println!("▶ Walk: {} entradas en {} ms", entries.len(), walk_ms);
    println!("  JPG:  {}", n_jpg);
    println!("  PNG:  {}", n_png);
    println!("  WEBP: {}", n_webp);
    println!("  Imágenes: {}", n_img);
    println!("  Vídeos:   {}", n_vid);

    // La fixture (regenerada por `tests/fixture.rs`) tiene:
    //   20 foto_N.jpg únicos + 4 hard links (foto_1×3, foto_2×2, foto_3×2 = 7 paths)
    //   = 24 JPGs + 3 WebPs en sub2/ = 27 imágenes, 0 vídeos.
    assert_eq!(n_img, 27, "debe descubrir 27 imágenes (20 + 4 + 3)");
    assert_eq!(n_vid, 0, "no debe haber vídeos en la fixture");

    // 2. Persistir en una DB temporal.
    let db_path = trial.parent().unwrap().join("trial-scan.db");
    let _ = std::fs::remove_file(&db_path);
    let mut conn = db::open_at(&db_path).expect("open db");
    conn.execute(
        "INSERT INTO scan_sessions (root_path, modes, status, started_at) \
         VALUES (?1, '[\"exact\"]', 'running', '2026-06-04T00:00:00Z')",
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

    // 3. Contar filas en `files`.
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM files WHERE session_id = ?1",
            [session_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, entries.len() as i64, "todas las entradas en DB");
    println!("▶ DB: {} filas en files (session_id={})", count, session_id);

    // 4. Preview de Modo 1: GROUP BY (filename, ext, ctime, size) HAVING n>1.
    //    Esto es lo que la Fase 3 implementará como comando.
    let mut dup_stmt = conn
        .prepare(
            "SELECT filename, ext, ctime_ns, size_bytes, COUNT(*) AS n \
             FROM files WHERE session_id = ?1 \
             GROUP BY filename, ext, ctime_ns, size_bytes \
             HAVING n > 1",
        )
        .unwrap();
    let dup_groups: Vec<(String, String, i64, i64, i64)> = dup_stmt
        .query_map([session_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    println!("▶ Grupos de duplicados exactos (preview Modo 1): {}", dup_groups.len());
    for g in &dup_groups {
        println!(
            "  · {} ({}), ctime_ns={}, size={} B → {} miembros",
            g.0, g.1, g.2, g.3, g.4
        );
    }

    // La fixture tiene 3 grupos pre-armados: foto_1 (×3), foto_2 (×2), foto_3 (×2).
    assert_eq!(dup_groups.len(), 3, "esperamos 3 grupos de duplicados");

    // 5. Verificar que los miembros de cada grupo son correctos.
    for g in &dup_groups {
        let mut m_stmt = conn
            .prepare("SELECT path FROM files WHERE session_id = ?1 AND filename = ?2 AND ext = ?3 AND ctime_ns = ?4 AND size_bytes = ?5")
            .unwrap();
        let members: Vec<String> = m_stmt
            .query_map(rusqlite::params![session_id, g.0, g.1, g.2, g.3], |r| r.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(members.len() as i64, g.4);
        println!("    miembros: {:?}", members);
    }

    // Limpieza.
    std::fs::remove_file(&db_path).ok();

    println!("\n✓ Smoke test sobre trial/ pasado.");
    println!("  Walk:  {} ms", walk_ms);
    println!("  Files: {}", entries.len());
    println!("  Dups:  {} grupos (preview Modo 1)", dup_groups.len());
}
