//! Runner de migraciones.
//!
//! Estrategia actual: ejecutar `schema.sql` en cada arranque. Las sentencias
//! son idempotentes (`CREATE TABLE IF NOT EXISTS`). Si en el futuro
//! necesitamos migraciones destructivas, introducimos una tabla
//! `schema_migrations` con un `version` entero y aplicamos en orden.

use rusqlite::Connection;
use tracing::info;

use crate::error::AppResult;

/// SQL embebido en tiempo de compilación.
const SCHEMA_SQL: &str = include_str!("schema.sql");

/// Aplica el esquema actual a la conexión. Idempotente.
pub fn run(conn: &Connection) -> AppResult<()> {
    info!("aplicando esquema de base de datos");
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(())
}
