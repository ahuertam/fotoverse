//! Tipos de error de la aplicación.
//!
//! [`AppError`] es la única error que se serializa al frontend. Cualquier
//! `Result<T, AppError>` que retorne un `#[tauri::command]` se convierte
//! automáticamente en una `String` con el mensaje legible en español.

use serde::{Serialize, Serializer};

/// Error de aplicación que se serializa a `String` para el frontend.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Error de base de datos: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Error de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error recorriendo el sistema de archivos: {0}")]
    Walk(#[from] jwalk::Error),

    #[error("No se pudo serializar: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("El escaneo fue cancelado por el usuario")]
    Cancelled,

    #[error("Ya hay un escaneo en curso (session_id={0})")]
    AlreadyRunning(i64),

    #[error("La sesión {0} no existe")]
    SessionNotFound(i64),

    #[error("Ruta inválida: {0}")]
    InvalidPath(String),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// Crea un error "otro" con un mensaje personalizado.
    #[allow(dead_code)]
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}

/// Serializa el error como un string legible (en español) para el frontend.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// Tipo de resultado estándar de la aplicación.
pub type AppResult<T> = Result<T, AppError>;
