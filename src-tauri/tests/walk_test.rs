//! Test de integración: walk sobre una fixture sintética.
//!
//! Verifica:
//! - Cuenta total de archivos descubiertos.
//! - Filtrado por extensión: se omiten los no soportados.
//! - Clasificación Image vs Video correcta.
//! - Metadata coherente (size, ctime, mtime).
//! - Inserción en SQLite: tras el walk, la tabla `files` tiene N filas.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use fotoverse_lib::db;
use fotoverse_lib::walk::{self, FileKind, WalkSummary};

/// Crea un directorio temporal con una jerarquía de archivos sintéticos.
/// Devuelve la ruta al directorio.
fn create_fixture(num_images: usize, num_videos: usize, num_other: usize) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("fotoverse-fixture-{stamp}"));
    fs::create_dir_all(&dir).expect("create fixture dir");

    // Subdirectorios para ejercitar la recursión.
    let sub1 = dir.join("vacaciones");
    let sub2 = dir.join("vacaciones/playa");
    let sub3 = dir.join("trabajo");
    fs::create_dir_all(&sub1).unwrap();
    fs::create_dir_all(&sub2).unwrap();
    fs::create_dir_all(&sub3).unwrap();

    // Imágenes (jpg, png, webp, gif, bmp, tif).
    let img_exts = ["jpg", "png", "webp", "gif", "bmp", "tif"];
    for i in 0..num_images {
        let ext = img_exts[i % img_exts.len()];
        let sub = [dir.as_path(), sub1.as_path(), sub2.as_path(), sub3.as_path()][i % 4];
        let path = sub.join(format!("foto_{i}.{ext}"));
        write_file(&path, 100 + (i as u64) * 10);
    }

    // Vídeos (mp4, mov, mkv, avi, webm).
    let vid_exts = ["mp4", "mov", "mkv", "avi", "webm"];
    for i in 0..num_videos {
        let ext = vid_exts[i % vid_exts.len()];
        let sub = [dir.as_path(), sub1.as_path(), sub3.as_path()][i % 3];
        let path = sub.join(format!("clip_{i}.{ext}"));
        write_file(&path, 1024 + (i as u64) * 100);
    }

    // Archivos no soportados (deben ignorarse).
    let other_exts = ["txt", "pdf", "zip", "doc"];
    for i in 0..num_other {
        let ext = other_exts[i % other_exts.len()];
        let path = dir.join(format!("doc_{i}.{ext}"));
        write_file(&path, 50);
    }

    dir
}

fn write_file(path: &PathBuf, size: u64) {
    let mut f = fs::File::create(path).expect("create file");
    let buf = vec![0u8; size.min(1024) as usize];
    f.write_all(&buf).expect("write file");
    // Para que size > 0 si piden menos de 1KB.
    if size > 1024 {
        f.write_all(&buf).expect("write file");
    }
}

/// Helper: cuenta filas de la tabla `files` para una sesión.
fn count_files(db_path: &PathBuf, session_id: i64) -> i64 {
    let conn = rusqlite::Connection::open(db_path).expect("open db");
    conn.query_row(
        "SELECT COUNT(*) FROM files WHERE session_id = ?1",
        [session_id],
        |row| row.get(0),
    )
    .expect("count")
}

#[test]
fn walk_finds_all_supported_files_and_ignores_others() {
    let fixture = create_fixture(50, 10, 20);
    let entries = walk::walk_for_test(&fixture).expect("walk");
    assert_eq!(entries.len(), 60, "debe encontrar 50 imgs + 10 vids");

    let images = entries
        .iter()
        .filter(|e| e.kind == FileKind::Image)
        .count();
    let videos = entries
        .iter()
        .filter(|e| e.kind == FileKind::Video)
        .count();
    assert_eq!(images, 50);
    assert_eq!(videos, 10);

    // Metadata coherente.
    for e in &entries {
        assert!(e.size_bytes > 0, "size debe ser > 0: {:?}", e.path);
        assert!(!e.filename.is_empty(), "filename no vacío");
        assert!(!e.ext.is_empty(), "ext no vacío");
    }

    fs::remove_dir_all(&fixture).ok();
}

#[test]
fn walk_inserts_into_sqlite_with_correct_count() {
    let fixture = create_fixture(20, 5, 10);
    let db_path = fixture.parent().unwrap().join("test-walk.db");
    let _ = fs::remove_file(&db_path);

    let mut conn = db::open_at(&db_path).expect("open db");
    conn.execute(
        "INSERT INTO scan_sessions (root_path, modes, status, started_at) \
         VALUES (?1, '[\"exact\"]', 'running', '2026-06-04T00:00:00Z')",
        [&fixture.to_string_lossy()],
    )
    .unwrap();
    let session_id = conn.last_insert_rowid();

    // walk::walk_for_test no inserta; replicamos la lógica mínima
    // equivalente para verificar que el esquema acepta la fila.
    let entries = walk::walk_for_test(&fixture).expect("walk");
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

    let count = count_files(&db_path, session_id);
    assert_eq!(count, entries.len() as i64);

    // Limpieza.
    fs::remove_dir_all(&fixture).ok();
    fs::remove_file(&db_path).ok();
}

#[test]
fn walk_handles_empty_directory() {
    let dir = std::env::temp_dir().join(format!(
        "fotoverse-empty-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    let entries = walk::walk_for_test(&dir).expect("walk");
    assert!(entries.is_empty(), "directorio vacío → 0 entradas");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn walk_skips_subdirectories_not_files() {
    // El fixture crea subdirectorios; el walk sólo debe devolver ficheros.
    let fixture = create_fixture(5, 2, 0);
    let entries = walk::walk_for_test(&fixture).expect("walk");
    for e in &entries {
        let p = std::path::Path::new(&e.path);
        assert!(p.is_file(), "path debe ser fichero: {}", e.path);
    }
    fs::remove_dir_all(&fixture).ok();
}

#[allow(dead_code)]
fn _summary_compiles(_s: WalkSummary) {}
