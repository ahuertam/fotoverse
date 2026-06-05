//! Hashing perceptual de imágenes — pHash (Fase 5).
//!
//! Implementación propia del algoritmo clásico:
//!   1. decode → grayscale 32×32
//!   2. DCT 2D 32×32
//!   3. top-left 8×8 (excluyendo DC = posición 0,0)
//!   4. threshold: bit i = 1 si value > median(64 values)
//!   5. pack 64 bits → 8 bytes
//!
//! Hamming ≤ 10 entre dos hashes se considera "perceptualmente
//! duplicado" (ver ARCHITECTURE.md § Modos de búsqueda).
//!
//! Por qué no usamos `img_hash`:
//! - v0.5: usa `#![feature(hash)]` y `push_all` (nightly).
//! - v3.x: atado a `image 0.21-0.23`, incompatible con nuestra 0.25.
//! Reimplementar son ~150 líneas y nos da control sobre el formato.

use std::path::Path;

use image::imageops::FilterType;
use rusqlite::Connection;
use tracing::{debug, warn};

use crate::error::{AppError, AppResult};
use crate::walk::FileKind;

/// Tamaño del hash (8 → 64 bits). Coincide con el `phash BLOB` del
/// esquema (8 bytes exactos).
pub const HASH_BYTES: usize = 8;

/// Lado del cuadrado al que reescalamos antes del DCT.
const DCT_SIDE: usize = 32;

/// Tamaño del bloque del que extraemos el hash (típicamente 8).
const HASH_SIDE: usize = 8;

/// Distancia Hamming máxima para considerar dos imágenes como
/// perceptualmente duplicadas. 10 está documentado en ARCHITECTURE.md.
pub const HAMMING_THRESHOLD: u32 = 10;

/// Calcula el pHash de un archivo de imagen. Devuelve los 8 bytes
/// del hash, o un error si el decode falla.
pub fn compute(path: &Path) -> AppResult<[u8; HASH_BYTES]> {
    let img = image::open(path).map_err(|e| {
        AppError::Other(format!("decode {}: {e}", path.display()))
    })?;
    Ok(phash_dct(&img))
}

/// Versión silenciosa: no aborta, devuelve `None` si falla.
pub fn compute_silent(path: &Path) -> Option<[u8; HASH_BYTES]> {
    compute(path).ok()
}

/// Distancia Hamming entre dos hashes de 8 bytes.
pub fn hamming_distance(a: &[u8; HASH_BYTES], b: &[u8; HASH_BYTES]) -> u32 {
    let mut dist = 0u32;
    for (x, y) in a.iter().zip(b.iter()) {
        dist += (x ^ y).count_ones();
    }
    dist
}

// ---------------------------------------------------------------------------
// Algoritmo pHash
// ---------------------------------------------------------------------------

/// Pipeline: grayscale → resize 32×32 → DCT 2D → top-left 8×8 → median
/// threshold → pack 64 bits.
fn phash_dct(img: &image::DynamicImage) -> [u8; HASH_BYTES] {
    // 1. Grayscale + resize.
    let gray = img.to_luma8();
    let resized = image::imageops::resize(&gray, DCT_SIDE as u32, DCT_SIDE as u32, FilterType::Triangle);
    let mut samples = [0f64; DCT_SIDE * DCT_SIDE];
    for y in 0..DCT_SIDE {
        for x in 0..DCT_SIDE {
            samples[y * DCT_SIDE + x] = resized.get_pixel(x as u32, y as u32).0[0] as f64;
        }
    }

    // 2. DCT 2D.
    let dct = dct_2d(&samples, DCT_SIDE, DCT_SIDE);

    // 3. Top-left 8×8 (excluyendo DC component en (0,0)).
    let mut block = [0f64; HASH_SIDE * HASH_SIDE];
    for y in 0..HASH_SIDE {
        for x in 0..HASH_SIDE {
            block[y * HASH_SIDE + x] = dct[y * DCT_SIDE + x];
        }
    }

    // 4. Mediana (excluyendo DC). El primer elemento (y=0,x=0) es
    //    la "energía total" de la imagen — no la usamos para threshold.
    let mut sorted: Vec<f64> = block[1..].to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];

    // 5. Pack bits: bit i = 1 si block[i] > median.
    let mut hash = [0u8; HASH_BYTES];
    for (i, &v) in block.iter().enumerate() {
        if v > median {
            // i va de 0 a 63; byte = i/8, bit = i%8 (LSB first).
            hash[i / 8] |= 1 << (i % 8);
        }
    }
    hash
}

/// DCT 2D separable: primero filas, luego columnas. Cosine transform
/// tipo-II sin normalización (no nos importa la escala exacta, sólo
/// las posiciones relativas para el threshold).
fn dct_2d(input: &[f64], w: usize, h: usize) -> Vec<f64> {
    // DCT por filas.
    let mut tmp = vec![0f64; w * h];
    for y in 0..h {
        let row = &input[y * w..(y + 1) * w];
        let transformed = dct_1d(row);
        tmp[y * w..(y + 1) * w].copy_from_slice(&transformed);
    }
    // DCT por columnas.
    let mut out = vec![0f64; w * h];
    for x in 0..w {
        let col: Vec<f64> = (0..h).map(|y| tmp[y * w + x]).collect();
        let transformed = dct_1d(&col);
        for (y, v) in transformed.into_iter().enumerate() {
            out[y * w + x] = v;
        }
    }
    out
}

/// DCT 1D tipo-II (sin normalización).
///   X_k = sum_{n=0..N-1} x_n * cos(π/N * (n + 0.5) * k)
fn dct_1d(input: &[f64]) -> Vec<f64> {
    let n = input.len();
    let mut out = vec![0f64; n];
    let scale = std::f64::consts::PI / n as f64;
    for k in 0..n {
        let mut sum = 0.0;
        for (i, &x) in input.iter().enumerate() {
            sum += x * (scale * (i as f64 + 0.5) * k as f64).cos();
        }
        out[k] = sum;
    }
    out
}

// ---------------------------------------------------------------------------
// DB helpers
// ---------------------------------------------------------------------------

/// Calcula pHash de todas las imágenes de la sesión que aún no lo
/// tengan (`hash_status IS NULL OR = 'pending' OR = 'error'`), y los
/// persiste en `files.phash`. Devuelve el nº de filas actualizadas.
pub fn populate_session(conn: &Connection, session_id: i64) -> AppResult<usize> {
    use rayon::prelude::*;

    // 1. Listar imágenes pendientes.
    let mut stmt = conn.prepare(
        "SELECT id, path, mtime_ns FROM files \
         WHERE session_id = ?1 AND kind = 'image' \
         AND (hash_status IS NULL OR hash_status = 'pending' OR hash_status = 'error')",
    )?;
    let rows: Vec<(i64, String, i64)> = stmt
        .query_map([session_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    if rows.is_empty() {
        debug!(session_id, "no hay imágenes pendientes de hashear");
        return Ok(0);
    }
    debug!(session_id, total = rows.len(), "hasheando imágenes");

    // 2. Calcular pHash en paralelo.
    let results: Vec<(i64, Result<[u8; HASH_BYTES], String>)> = rows
        .par_iter()
        .map(|(id, path, _mtime)| {
            let res: Result<[u8; HASH_BYTES], String> =
                compute(Path::new(path)).map_err(|e| e.to_string());
            (*id, res)
        })
        .collect();

    // 3. Persistir. Una sola transacción para no fragmentar el WAL.
    let mut updated = 0usize;
    let tx = conn.unchecked_transaction()?;
    {
        let mut ok_stmt = tx.prepare(
            "UPDATE files SET phash = ?1, hash_status = 'ok', hash_error = NULL \
             WHERE id = ?2",
        )?;
        let mut err_stmt = tx.prepare(
            "UPDATE files SET hash_status = 'error', hash_error = ?1 \
             WHERE id = ?2",
        )?;
        for (id, hash_res) in results {
            match hash_res {
                Ok(bytes) => {
                    ok_stmt.execute(rusqlite::params![&bytes, id])?;
                }
                Err(msg) => {
                    err_stmt.execute(rusqlite::params![msg, id])?;
                    warn!(file_id = id, "pHash falló: {msg}");
                }
            }
            updated += 1;
        }
    }
    tx.commit()?;

    debug!(session_id, updated, "pHash de imágenes persistido");
    Ok(updated)
}

/// Filtra: dado un conjunto de hashes candidatos y un hash de
/// referencia, devuelve los ids cuya distancia Hamming al de
/// referencia sea ≤ threshold.
#[allow(dead_code)]
pub fn find_matches(
    candidates: &[(i64, [u8; HASH_BYTES])],
    reference: &[u8; HASH_BYTES],
    threshold: u32,
) -> Vec<(i64, u32)> {
    candidates
        .iter()
        .filter_map(|(id, h)| {
            let d = hamming_distance(h, reference);
            if d <= threshold {
                Some((*id, d))
            } else {
                None
            }
        })
        .collect()
}

/// Símbolo para tests: indica que el módulo carga.
#[allow(dead_code)]
pub fn _smoke() {
    let _ = FileKind::Image;
}
