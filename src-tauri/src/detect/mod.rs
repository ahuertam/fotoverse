//! Algoritmo de los 3 modos de detección.
//!
//! - [`exact`]: Modo 1 (GROUP BY filename, ext, ctime, size).
//! - `perceptual` (Fase 5-6): Modo 2 (pHash + BK-tree).
//! - `manual` (Fase 7): Modo 3 (pares a revisar manualmente).

pub mod exact;
