//! Algoritmo de los 3 modos de detección.
//!
//! - [`exact`]: Modo 1 (GROUP BY filename, ext, ctime, size).
//! - [`perceptual`]: Modo 2 (pHash + BK-tree).
//! - [`manual`]: Modo 3 (pares a revisar manualmente).

pub mod exact;
pub mod manual;
pub mod perceptual;
