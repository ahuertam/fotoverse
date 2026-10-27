//! Algoritmo de los 3 modos de detección.
//!
//! - [`exact`]: Modo 1 (GROUP BY filename, ext, ctime, size).
//! - [`perceptual`]: Modo 2 (pHash + Union-Find, O(n²) con poda).
//! - [`manual`]: Modo 3 (pares a revisar manualmente).

pub mod exact;
pub mod manual;
pub mod perceptual;
