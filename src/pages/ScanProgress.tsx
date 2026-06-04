/**
 * Página de progreso de escaneo.
 *
 * - Hook `useScanProgress` (con catch-up) gestiona eventos + estado
 *   inicial de la sesión.
 * - Muestra fase actual, contador, path actual.
 * - Botón Cancelar invoca `cancelScan`.
 * - Cuando llega `scan_completed` (o se detecta en el catch-up),
 *   muestra resumen y botón "Revisar duplicados (Modo 1) →".
 */

import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useScanProgress } from "../hooks/useScanProgress";
import { cancelScan } from "../lib/tauri";

function phaseLabel(phase: string): string {
  switch (phase) {
    case "walking":
      return "Caminando";
    case "hashing_images":
      return "Calculando hashes (imágenes)";
    case "hashing_videos":
      return "Calculando hashes (vídeos)";
    case "clustering":
      return "Agrupando duplicados";
    case "done":
      return "Listo";
    default:
      return phase;
  }
}

function ScanProgressPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const sessionId = id != null ? Number(id) : null;

  const [cancelling, setCancelling] = useState(false);
  const { progress, completed, initialStatus } = useScanProgress(sessionId);

  const session = initialStatus?.session ?? null;

  async function handleCancel() {
    if (sessionId == null) return;
    setCancelling(true);
    try {
      await cancelScan(sessionId);
    } finally {
      setCancelling(false);
    }
  }

  // Render -----------------------------------------------------------------

  if (sessionId == null) {
    return (
      <main className="scan-progress">
        <p>ID de sesión inválido.</p>
        <button type="button" onClick={() => navigate("/")}>
          Volver al inicio
        </button>
      </main>
    );
  }

  const totalEstimate = progress?.total_estimate ?? null;
  const filesSeen = progress?.files_seen ?? 0;
  const pct = totalEstimate && totalEstimate > 0
    ? Math.min(100, Math.round((filesSeen / totalEstimate) * 100))
    : 0;
  const isDone = completed != null;

  return (
    <main className="scan-progress">
      <header>
        <h1>Escaneando…</h1>
        {session && (
          <p className="root-path" title={session.root_path}>
            📁 {session.root_path}
          </p>
        )}
      </header>

      <section className="phase">
        <h2>{phaseLabel(progress?.phase ?? "walking")}</h2>
        <div
          className="progress-bar"
          role="progressbar"
          aria-valuenow={pct}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <div className="progress-bar-fill" style={{ width: `${pct}%` }} />
        </div>
        <p className="counter">
          {filesSeen.toLocaleString("es-ES")}
          {totalEstimate != null
            ? ` / ${totalEstimate.toLocaleString("es-ES")} archivos`
            : " archivos"}
          {pct > 0 && ` · ${pct}%`}
        </p>
        {progress?.current_path && (
          <p className="current-path" title={progress.current_path}>
            {progress.current_path}
          </p>
        )}
      </section>

      {isDone ? (
        <section className="done">
          <h2>
            {completed?.status === "completed"
              ? "✅ Escaneo completado"
              : completed?.status === "cancelled"
              ? "⛔ Escaneo cancelado"
              : "❌ Escaneo falló"}
          </h2>
          {completed && (
            <p>
              {completed.total_files.toLocaleString("es-ES")} archivos
              encontrados en {(completed.duration_ms / 1000).toFixed(1)} s
            </p>
          )}
          {completed?.error && <p className="error">⚠️ {completed.error}</p>}
          <div className="done-actions">
            <button
              type="button"
              className="primary"
              onClick={() => navigate(`/review/${sessionId}/mode1`)}
              disabled={completed?.status !== "completed"}
            >
              Revisar duplicados (Modo 1) →
            </button>
            <button
              type="button"
              onClick={() => navigate("/")}
            >
              Volver al inicio
            </button>
          </div>
        </section>
      ) : (
        <section className="actions">
          <button
            type="button"
            onClick={handleCancel}
            disabled={cancelling}
          >
            {cancelling ? "Cancelando…" : "Cancelar"}
          </button>
        </section>
      )}
    </main>
  );
}

export default ScanProgressPage;
