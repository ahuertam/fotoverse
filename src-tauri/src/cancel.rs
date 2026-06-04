//! Helpers de cancelación.
//!
//! Encapsula [`tokio_util::sync::CancellationToken`] para que el código de
//! negocio (walk, hash, cluster) use un tipo neutro en lugar de depender
//! directamente de `tokio_util`. Si en el futuro cambiamos de mecanismo de
//! cancelación, este es el único módulo a tocar.

pub use tokio_util::sync::CancellationToken as Cancel;

/// Crea un par (token, guard) donde el guard al hacer `drop` cancela el
/// token. Útil para RAII: si el futuro se cancela, se cancela el token.
#[allow(dead_code)]
pub fn cancellable() -> (Cancel, CancelGuard) {
    let token = Cancel::new();
    let guard = CancelGuard(token.clone());
    (token, guard)
}

/// RAII guard: al hacer `drop` cancela el token asociado.
#[allow(dead_code)]
pub struct CancelGuard(pub Cancel);

impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
