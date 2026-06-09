/**
 * Página de bienvenida.
 *
 * En Fase 2:
 * - Botón "Elegir carpeta…" que invoca el dialog nativo.
 * - Lanza el escaneo y navega a /scan/:id.
 *
 * En Fases futuras se añadirá:
 * - Botón "Elegir disco…".
 * - Lista de escaneos anteriores.
 * - Ajustes (cache, archivos ocultos).
 */

import { useState } from "react";
import { useNavigate } from "react-router-dom";

import { Splash } from "../components/Splash";
import logoUrl from "../assets/fotologo.png";
import { pickFolder, setSetting, startScan } from "../lib/tauri";
import type { ScanMode } from "../lib/types";

const DEFAULT_MODES: ScanMode[] = ["exact", "perceptual", "manual"];

function Welcome() {
  const navigate = useNavigate();
  const [splashDone, setSplashDone] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handlePickFolder() {
    setError(null);
    setBusy(true);
    try {
      const root = await pickFolder();
      if (!root) {
        // Usuario canceló el dialog.
        return;
      }
      // Persistir la última carpeta para la próxima sesión.
      void setSetting("lastFolder", root);
      const session = await startScan(root, DEFAULT_MODES);
      navigate(`/scan/${session.id}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      {!splashDone && <Splash onDone={() => setSplashDone(true)} />}
      <main className="welcome" style={{ opacity: splashDone ? 1 : 0, transition: "opacity 400ms ease-in-out" }}>
        <img
          src={logoUrl}
          alt="fotoverse"
          style={{
            width: "180px",
            height: "180px",
            borderRadius: "36px",
            marginBottom: "1rem",
            boxShadow: "0 8px 32px rgba(0,0,0,0.3)",
          }}
        />
        <h1>fotoverse</h1>
        <p className="tagline">
          Encuentra fotos y vídeos duplicados en tu biblioteca
        </p>

        <button
          type="button"
          className="primary"
          onClick={handlePickFolder}
          disabled={busy}
        >
          {busy ? "Iniciando escaneo…" : "Elegir carpeta…"}
        </button>

        <button
          type="button"
          onClick={() => navigate("/settings")}
          style={{ marginTop: "0.5rem" }}
        >
          Ajustes
        </button>

        {error && <p className="error">⚠️ {error}</p>}

        <p className="status">
          🟢 Backend Rust inicializado. SQLite en app_data_dir().
        </p>
      </main>
    </>
  );
}

export default Welcome;
