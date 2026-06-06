//! Runner de migraciones.
//!
//! Estrategia:
//! 1. Ejecutar `schema.sql` (idempotente vía `CREATE TABLE IF NOT EXISTS`).
//! 2. Aplicar migraciones "delta" para columnas añadidas a tablas
//!    existentes (SQLite no tiene `ADD COLUMN IF NOT EXISTS`).
//!
//! Si en el futuro necesitamos migraciones destructivas, introducimos
//! una tabla `schema_migrations` con un `version` entero y aplicamos
//! en orden. Por ahora `schema.sql` + deltas cubren el flujo.

use rusqlite::Connection;
use tracing::{info, warn};

use crate::error::AppResult;

/// SQL embebido en tiempo de compilación.
const SCHEMA_SQL: &str = include_str!("schema.sql");

/// Aplica el esquema actual + deltas a la conexión. Idempotente.
pub fn run(conn: &Connection) -> AppResult<()> {
    info!("aplicando esquema de base de datos");
    conn.execute_batch(SCHEMA_SQL)?;

    // Migraciones "delta": columnas que se añadieron a tablas
    // existentes después del schema inicial. SQLite < 3.35 no soporta
    // `ADD COLUMN IF NOT EXISTS`, así que comprobamos con
    // `PRAGMA table_info` antes de hacer `ALTER TABLE`.
    ensure_column(conn, "perceptual_groups", "member_count", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "perceptual_groups", "hamming_to_rep", "INTEGER NOT NULL DEFAULT 0")?;

    Ok(())
}

/// Helper: añade una columna a una tabla si no existe. Comprueba
/// `PRAGMA table_info` para no fallar en reaperturas.
fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> AppResult<()> {
    // `PRAGMA table_info` no soporta placeholders, así que interpolamos.
    // Es seguro porque `table` y `column` son literales controlados.
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma)?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?; // columna "name" en table_info
        if name == column {
            return Ok(());
        }
    }
    // No existe → ALTER TABLE.
    let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
    warn!(table, column, "aplicando delta: {sql}");
    conn.execute_batch(&sql)?;
    Ok(())
}
