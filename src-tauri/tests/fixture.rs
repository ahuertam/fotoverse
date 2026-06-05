//! Regenera la fixture `trial/` con imágenes VÁLIDAS (JPEGs/WebPs
//! reales) para que los tests de Fase 4 (thumbs) y Fase 5 (pHash)
//! tengan algo que morder.
//!
//! Estrategia:
//! 1. Borra los ficheros previos.
//! 2. Genera 20 JPEGs distintos (256x192) con un gradiente
//!    pseudo-aleatorio por nº.
//! 3. Crea hard links para los 3 grupos que Modo 1 espera:
//!    - `foto_1.jpg` × 3 (raíz + sub2 + ?)
//!    - `foto_2.jpg` × 2
//!    - `foto_3.jpg` × 2
//! 4. Genera 3 WebPs en `sub2/` (placeholders de "vídeos" hasta
//!    que Fase 6 introduzca ffmpeg).
//!
//! Si `trial/` no existe, lo crea.
//!
//! Se ejecuta **una vez** al inicio de la sesión de tests; es
//! idempotente. Para saltarlo: `SKIP_FIXTURE=1 cargo test`.

use std::path::PathBuf;

fn trial_dir() -> PathBuf {
    let p = std::path::PathBuf::from("/Users/home/Desktop/proyects/trial");
    if !p.exists() {
        std::fs::create_dir_all(&p).expect("mkdir trial");
    }
    std::fs::create_dir_all(p.join("sub2")).expect("mkdir sub2");
    p
}

/// Genera un JPEG de 256x192 con un gradiente basado en `seed`.
fn make_jpeg(path: &std::path::Path, seed: u32) {
    let w = 256u32;
    let h = 192u32;
    let mut img = image::RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            // Gradiente diagonal con tonalidad según seed.
            let t = ((x as f32 / w as f32) + (y as f32 / h as f32)) * 0.5;
            let r = ((seed as f32 * 7.0).sin().abs() * 200.0 + 30.0) as u8;
            let g = ((seed as f32 * 11.0 + t * 255.0).sin().abs() * 200.0 + 30.0) as u8;
            let b = ((seed as f32 * 13.0 + t * 255.0).cos().abs() * 200.0 + 30.0) as u8;
            img.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    img.save_with_format(path, image::ImageFormat::Jpeg)
        .expect("save jpeg");
}

fn make_webp(path: &std::path::Path, seed: u32) {
    // Mismo tamaño 16:9, más pequeño.
    let w = 320u32;
    let h = 180u32;
    let mut img = image::RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let t = x as f32 / w as f32;
            let r = ((seed as f32 * 17.0).cos().abs() * 180.0 + 40.0) as u8;
            let g = ((seed as f32 * 19.0 + t * 255.0).sin().abs() * 180.0 + 40.0) as u8;
            let b = ((seed as f32 * 23.0).sin().abs() * 180.0 + 40.0) as u8;
            img.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    img.save_with_format(path, image::ImageFormat::WebP)
        .expect("save webp");
}

#[test]
fn regenerate_trial_fixture() {
    if std::env::var("SKIP_FIXTURE").is_ok() {
        eprintln!("(saltado: SKIP_FIXTURE=1)");
        return;
    }
    let trial = trial_dir();

    // 1. Limpia cualquier fichero de imagen/vídeo previo (recursivo).
    fn clean_recursive(dir: &std::path::Path) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    clean_recursive(&p);
                } else if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "gif" | "mp4" | "mov") {
                            let _ = std::fs::remove_file(&p);
                        }
                    }
                }
            }
        }
    }
    clean_recursive(&trial);
    // Elimina subdirectorios vacíos que hayan quedado (no `sub2`).
    if let Ok(entries) = std::fs::read_dir(&trial) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "sub2" {
                    let _ = std::fs::remove_dir(&p);
                }
            }
        }
    }

    // 2. Genera 20 JPEGs.
    for n in 1..=20u32 {
        let p = trial.join(format!("foto_{n}.jpg"));
        make_jpeg(&p, n);
    }

    // 3. Hard links para los 3 grupos de Modo 1.
    //    Grupo 1: foto_1.jpg × 3 (raíz + sub2 + sub3).
    std::fs::hard_link(trial.join("foto_1.jpg"), trial.join("sub2/foto_1.jpg"))
        .expect("hard link foto_1 → sub2");
    std::fs::create_dir_all(trial.join("sub3")).ok();
    std::fs::hard_link(trial.join("foto_1.jpg"), trial.join("sub3/foto_1.jpg"))
        .expect("hard link foto_1 → sub3");
    //    Grupo 2: foto_2.jpg × 2 (raíz + sub2).
    std::fs::hard_link(trial.join("foto_2.jpg"), trial.join("sub2/foto_2.jpg"))
        .expect("hard link foto_2");
    //    Grupo 3: foto_3.jpg × 2 (raíz + sub2).
    std::fs::hard_link(trial.join("foto_3.jpg"), trial.join("sub2/foto_3.jpg"))
        .expect("hard link foto_3");

    // 4. Genera 3 WebPs en sub2/ como placeholders de vídeos.
    for n in 1..=3u32 {
        let p = trial.join(format!("sub2/clip_{n}.webp"));
        make_webp(&p, n);
    }

    // 5. Smoke check: existen, son JPEGs/WebPs válidos, foto_1.jpg
    //    tiene 3 hard links (raíz + sub2 + sub3).
    assert!(trial.join("foto_1.jpg").exists());
    assert!(trial.join("sub2/foto_1.jpg").exists());
    assert!(trial.join("sub3/foto_1.jpg").exists());
    let meta_root = std::fs::metadata(trial.join("foto_1.jpg")).unwrap();
    let meta_sub = std::fs::metadata(trial.join("sub2/foto_1.jpg")).unwrap();
    assert_eq!(meta_root.len(), meta_sub.len(), "hard link debe tener mismo size");
    // En macOS, el nº de hard links está en `links()`.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(meta_root.nlink(), 3, "foto_1.jpg debe tener 3 hard links (raíz + sub2 + sub3)");
    }

    // 6. Verificar que decodifican como JPEGs.
    let img = image::open(trial.join("foto_1.jpg")).expect("decode foto_1");
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 192);

    let webp = image::open(trial.join("sub2/clip_1.webp")).expect("decode clip_1");
    assert_eq!(webp.width(), 320);
    assert_eq!(webp.height(), 180);

    println!("\n✓ Fixture regenerada:");
    println!("  20 JPEGs en trial/ (256x192)");
    println!("  foto_1 × 3 (raíz + sub2 + sub3), foto_2 × 2, foto_3 × 2");
    println!("  3 WebPs en sub2/ (320x180, placeholder de vídeos)");
    println!("  Total: ~28 archivos, ~150 KB");
}
