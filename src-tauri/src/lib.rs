//! Punto de entrada del backend Rust de fotoverse.
//!
//! Ver [ARCHITECTURE.md](../../docs/ARCHITECTURE.md) para el diseño completo.

use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Comando de ejemplo que el scaffold de Tauri incluye. Se eliminará
/// en la Fase 2 cuando se sustituyan por `pick_folder`, `start_scan`, etc.
#[tauri::command]
fn greet(name: &str) -> String {
    format!("¡Hola, {name}! Te saluda Rust desde fotoverse.")
}

/// Inicializa el logger estructurado (`tracing`).
///
/// - En `dev` (cargo run, `tauri dev`): nivel `debug` por defecto, configurable
///   con la env var `RUST_LOG`.
/// - En `release` (`tauri build`): nivel `info` por defecto.
///
/// Idempotente: si ya hay un subscriber global, no lo reemplaza.
fn init_tracing() {
    let default_level = if cfg!(debug_assertions) { "debug" } else { "info" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_level(true))
        .try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    info!("fotoverse arrancando (v{})", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error fatal al ejecutar la aplicación fotoverse");
}
