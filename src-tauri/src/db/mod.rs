//! Capa de base de datos.
//!
//! - [`open`] abre (o crea) el fichero SQLite en `app_data_dir()`,
//!   configura WAL y ejecuta las migraciones.
//! - [`check_integrity`] corre `PRAGMA integrity_check` (a usar al
//!   arranque en versiones futuras, ver Fase 9).

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};
use tracing::{info, warn};

use crate::error::{AppError, AppResult};

pub mod migrations;

/// Nombre del fichero SQLite.
const DB_FILENAME: &str = "fotoverse.db";

/// Abre la base de datos en `app_data_dir()/fotoverse.db` y la deja lista
/// para usar (WAL + synchronous=NORMAL + migraciones aplicadas).
pub fn open(app: &AppHandle) -> AppResult<Connection> {
    let dir = app_data_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|e| {
        AppError::Io(std::io::Error::new(
            e.kind(),
            format!("no se pudo crear {}: {e}", dir.display()),
        ))
    })?;
    let path = dir.join(DB_FILENAME);
    open_at(&path)
}

/// Abre (o crea) la base de datos en una ruta arbitraria. Útil para tests
/// con ficheros temporales.
pub fn open_at(path: &Path) -> AppResult<Connection> {
    info!("abriendo base de datos en {}", path.display());
    let conn = Connection::open(path)?;

    // Pragmas de rendimiento y durabilidad.
    // WAL: lecturas concurrentes, escritura no bloquea lecturas.
    // synchronous=NORMAL: commit puede sobrevivir a un crash del OS
    // (no a un power loss) — aceptable para nuestro caso de uso
    // (la sesión es recuperable re-escaneando).
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;

    migrations::run(&conn)?;
    Ok(conn)
}

/// Resuelve el directorio de datos de la app (OS-específico).
fn app_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::Other(format!("no se pudo resolver app_data_dir: {e}")))
}

/// Comprueba la integridad de la DB. Devuelve `Ok(true)` si está sana.
/// En Fase 9 se invocará al arranque y se le preguntará al usuario
/// qué hacer si está corrupta.
#[allow(dead_code)]
pub fn check_integrity(conn: &Connection) -> AppResult<bool> {
    let mut stmt = conn.prepare("PRAGMA integrity_check")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let result: String = row.get(0)?;
        if result == "ok" {
            Ok(true)
        } else {
            warn!("integrity_check devolvió: {result}");
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

/// Resultado de [`check_integrity_with_details`]: estado de la DB
/// y, si está corrupta, diagnóstico.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub ok: bool,
    /// Mensaje crudo de `PRAGMA integrity_check`. "ok" si todo
    /// bien, o una descripción del problema si está corrupta.
    pub details: String,
}

impl IntegrityReport {
    pub fn ok() -> Self {
        Self { ok: true, details: "ok".into() }
    }
}

/// Variante de `check_integrity` que devuelve el mensaje crudo
/// de `PRAGMA integrity_check` para diagnóstico.
pub fn check_integrity_with_details(conn: &Connection) -> AppResult<IntegrityReport> {
    let mut stmt = conn.prepare("PRAGMA integrity_check")?;
    let mut rows = stmt.query([])?;
    let details: String = if let Some(row) = rows.next()? {
        row.get(0)?
    } else {
        "no rows".into()
    };
    let ok = details.trim() == "ok";
    if !ok {
        warn!(details = %details, "DB corrupta detectada por integrity_check");
    }
    Ok(IntegrityReport { ok, details })
}

/// Elimina el fichero de DB y lo recrea vacío. **Destructivo**:
/// borra todas las sesiones. Usar sólo si `check_integrity`
/// falla y el usuario confirma.
pub fn reset_db(path: &Path) -> AppResult<()> {
    warn!(path = %path.display(), "reseteando base de datos");
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| {
            AppError::Io(std::io::Error::new(
                e.kind(),
                format!("no se pudo borrar {}: {e}", path.display()),
            ))
        })?;
    }
    // Borrar también los ficheros WAL/SHM si existen.
    for ext in ["-wal", "-shm", "-journal"] {
        let p = path.with_extension(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| format!("{e}{ext}"))
                .unwrap_or_else(|| ext.to_string()),
        );
        if p.exists() {
            let _ = std::fs::remove_file(&p);
        }
    }
    Ok(())
}
