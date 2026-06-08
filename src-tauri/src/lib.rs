//! Punto de entrada del backend Rust de fotoverse.
//!
//! Ver [ARCHITECTURE.md](../../docs/ARCHITECTURE.md) para el diseño completo.

use tauri::Manager;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod cancel;
mod commands;
pub mod db;
pub mod detect;
mod error;
pub mod hash;
pub mod review;
mod state;
pub mod thumbs;
pub mod walk;

/// Comando de ejemplo del scaffold original. Se mantiene por compatibilidad
/// con el tutorial de Tauri; los comandos reales son los de `commands::*`.
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Abrir y migrar la base de datos antes de que la UI
            // pueda hacer nada.
            let conn = db::open(app.handle())
                .expect("no se pudo abrir la base de datos");
            // Raíz del cache de thumbnails (Fase 4). `app_cache_dir()`
            // es portable (Mac: ~/Library/Caches/...; Windows: %LOCALAPPDATA%\...).
            let cache_root = app
                .path()
                .app_cache_dir()
                .expect("no se pudo resolver app_cache_dir");
            let state = state::AppState::new(conn, cache_root);
            app.manage(state);
            info!("base de datos lista");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::pick_folder,
            commands::start_scan,
            commands::cancel_scan,
            commands::get_scan_status,
            commands::list_sessions,
            commands::delete_session,
            commands::get_exact_groups,
            commands::get_group_members,
            commands::set_decision,
            commands::bulk_set_decision,
            commands::get_thumbnail,
            commands::get_perceptual_groups,
            commands::get_manual_pairs,
            commands::skip_manual_pair,
            commands::get_files_meta,
            commands::get_final_review,
            commands::execute_deletions,
            commands::get_video_and_heavy,
        ])
        .run(tauri::generate_context!())
        .expect("error fatal al ejecutar la aplicación fotoverse");
}
