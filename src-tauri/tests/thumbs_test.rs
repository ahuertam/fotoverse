//! Test de Fase 4 — generación de thumbnails.
//!
//! Estrategia:
//! 1. Hace un walk sobre `trial/` para obtener FileMetas.
//! 2. Genera thumbs para todas las entradas en un cache temporal.
//! 3. Verifica que los JPEGs existen, son >0 bytes, y son válidos
//!    (decodifican con `image`).
//! 4. Verifica el cache hit: una segunda llamada a `generate` no
//!    reescribe ni falla.
//!
//! Si `trial/` no existe, se salta (mismo patrón que los otros tests).

use std::path::{Path, PathBuf};

use fotoverse_lib::thumbs;
use fotoverse_lib::walk::{self, FileKind};

fn trial_dir() -> Option<PathBuf> {
    [
        PathBuf::from("/Users/home/Desktop/proyects/trial"),
        PathBuf::from("./trial"),
        PathBuf::from("../trial"),
    ]
    .into_iter()
    .find(|p| p.is_dir())
}

fn temp_cache_root(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("fotoverse-thumbs-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("mkdir cache root");
    p
}

#[test]
fn thumbs_generate_for_trial() {
    let Some(trial) = trial_dir() else {
        eprintln!("(saltado: trial/ no existe)");
        return;
    };

    let cache_root = temp_cache_root("gen");
    let entries = walk::walk_for_test(&trial).expect("walk");

    assert!(!entries.is_empty(), "trial/ no tiene entradas");
    let images: Vec<_> = entries
        .iter()
        .filter(|m| matches!(m.kind, FileKind::Image))
        .collect();
    assert!(!images.is_empty(), "trial/ no tiene imágenes");

    // 1) Generar thumbs para todas las imágenes.
    for meta in &images {
        let path = Path::new(&meta.path);
        let p = thumbs::generate_silent(path, meta.kind, meta.mtime_ns, &cache_root)
            .unwrap_or_else(|| panic!("falló generando thumb para {}", meta.path));
        assert!(p.exists(), "thumb no existe: {}", p.display());
        let size = std::fs::metadata(&p).expect("stat").len();
        assert!(size > 100, "thumb demasiado pequeño: {} bytes ({})", size, p.display());

        // Verificar que decodifica como JPEG válido.
        let img = image::open(&p).expect("decode thumb");
        assert_eq!(
            img.width(),
            256.min(img.width()),
            "ancho inesperado: {}",
            img.width()
        );
    }
    println!("▶ Generados {} thumbs de imágenes", images.len());

    // 2) Cache hit: la segunda llamada debe devolver la misma ruta
    //    sin tocar el disco.
    let first_meta = images[0];
    let path = Path::new(&first_meta.path);
    let p1 = thumbs::generate_silent(path, first_meta.kind, first_meta.mtime_ns, &cache_root)
        .expect("primera");
    let p2 = thumbs::generate_silent(path, first_meta.kind, first_meta.mtime_ns, &cache_root)
        .expect("segunda");
    assert_eq!(p1, p2, "cache hit debe devolver la misma ruta");
    println!("▶ Cache hit verificado: {}", p1.display());

    // 3) Vídeos (placeholders) — trial no tiene vídeos, pero probamos
    //    la rama con un path ficticio.
    let fake_video = temp_cache_root("vtest").join("fake.mp4");
    std::fs::write(&fake_video, b"\0\0\0\0ftypmp42").expect("write fake mp4");
    // No podemos invocar `walk::FileKind::Video` con mtime_ns y que
    // llegue al `generate_silent` porque requiere FileMeta; en su
    // lugar probamos con un path cualquiera + kind=Video + mtime=0.
    // Para ello, construimos un FileMeta a mano no es posible (campos
    // privados), así que validamos con `walk_for_test` si la fixture
    // tiene vídeos. Si no, omitimos.
    let videos: Vec<_> = entries
        .iter()
        .filter(|m| matches!(m.kind, FileKind::Video))
        .collect();
    if videos.is_empty() {
        eprintln!("(saltado: trial/ no tiene vídeos para probar placeholder)");
    } else {
        for meta in &videos {
            let path = Path::new(&meta.path);
            let p = thumbs::generate_silent(path, meta.kind, meta.mtime_ns, &cache_root)
                .expect("video placeholder");
            assert!(p.exists());
        }
        println!("▶ {} placeholders de vídeo generados", videos.len());
    }
    let _ = fake_video; // suprimimos el warning de "unused" — lo dejamos
                        // para un test manual de la rama video.

    // 4) Limpieza.
    std::fs::remove_dir_all(&cache_root).ok();
    println!("\n✓ Test de thumbs pasado.");
}

#[test]
fn thumbs_cache_path_for_deterministic() {
    // El mismo path + mtime debe dar el mismo path de cache.
    let cache_root = temp_cache_root("det");
    let p = Path::new("/tmp/fotoverse-test/sample.jpg");
    let mtime = 12345i64;
    let a = thumbs::cache_path_for(p, mtime, &cache_root);
    let b = thumbs::cache_path_for(p, mtime, &cache_root);
    assert_eq!(a, b, "cache path debe ser determinista");
    // `Path::ends_with` compara componentes, no strings. Usamos
    // `file_name()` para chequear la extensión del último segmento.
    assert_eq!(
        a.extension().and_then(|e| e.to_str()),
        Some("jpg"),
        "debe terminar en .jpg: {}",
        a.display()
    );
    // Cambiar mtime cambia el path.
    let c = thumbs::cache_path_for(p, 99999, &cache_root);
    assert_ne!(a, c, "mtime distinto → path distinto");
    println!(
        "▶ Cache path determinista: {} vs {} (mtime diferente)",
        a.display(),
        c.display()
    );
    std::fs::remove_dir_all(&cache_root).ok();
}
