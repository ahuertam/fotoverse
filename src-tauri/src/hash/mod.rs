//! Hashing perceptual de imágenes y vídeos (Fase 5 y 6).
//!
//! Estrategia:
//! - Imágenes: pHash via `img_hash::ImageHash::hash(.., 8, HashType::DCT)`
//!   → 8 bytes (64 bits) por imagen.
//! - Vídeos: pendiente Fase 6 (ffmpeg-sidecar).

pub mod image_hash;
