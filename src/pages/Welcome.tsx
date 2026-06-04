/**
 * Página de bienvenida (placeholder en Fase 1).
 *
 * En la Fase 2 añadiremos:
 * - Botón "Elegir carpeta…" → invoca el comando Tauri `pick_folder`.
 * - Botón "Elegir disco…" → muestra los volúmenes del sistema.
 * - Lista de escaneos anteriores para reanudar.
 *
 * Por ahora sólo muestra el título y un mensaje de estado del backend.
 */
function Welcome() {
  return (
    <main className="welcome">
      <h1>fotoverse</h1>
      <p className="tagline">
        Encuentra duplicados en tu biblioteca de fotos y vídeos.
      </p>
      <p className="status">
        🟢 Backend Rust inicializado. Esperando integración con la Fase 2.
      </p>
    </main>
  );
}

export default Welcome;
