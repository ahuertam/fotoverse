/**
 * Pantalla final tras el borrado (Fase 8).
 *
 * Muestra el reporte: nº de ficheros enviados a la papelera,
 * bytes liberados, fallos (si los hay). Ofrece:
 * - "Empezar otro escaneo" → vuelve a Welcome
 * - "Ver el resumen de la sesión" → vuelve a la página inicial
 *   de la sesión (scan/N) con los modos.
 */

import { useLocation, useNavigate, useParams } from "react-router-dom";

import type { DeletionReport } from "../lib/types";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024)
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function Done() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const location = useLocation();
  const state = location.state as { report?: DeletionReport } | null;
  const report = state?.report ?? null;

  return (
    <main className="welcome">
      <h1>🎉 ¡Listo!</h1>
      <p className="tagline">
        {report && report.deleted > 0
          ? `Has liberado ${formatBytes(report.bytesFreed)}`
          : "Borrado completado."}
      </p>

      {report && (
        <div
          className="status"
          style={{ textAlign: "left", maxWidth: "32rem" }}
        >
          <p>
            <strong>✅ {report.deleted}</strong>{" "}
            {report.deleted === 1 ? "fichero enviado" : "ficheros enviados"} a
            la papelera ({formatBytes(report.bytesFreed)})
          </p>
          {report.failed > 0 && (
            <p className="error">
              ⚠️ <strong>{report.failed}</strong>{" "}
              {report.failed === 1 ? "fallo" : "fallos"}. Revisa la papelera
              y los permisos.
            </p>
          )}
          {report.failures.length > 0 && (
            <details>
              <summary>Ver detalles de los fallos</summary>
              <ul style={{ fontSize: "0.75rem", marginTop: "0.5rem" }}>
                {report.failures.map((failure: [string, string], i: number) => (
                  <li key={i}>
                    <code>{failure[0]}</code>: {failure[1]}
                  </li>
                ))}
              </ul>
            </details>
          )}
        </div>
      )}

      <button
        type="button"
        className="primary"
        onClick={() => navigate("/")}
        style={{ marginTop: "1.5rem" }}
      >
        Empezar otro escaneo
      </button>
      {id && (
        <button
          type="button"
          onClick={() => navigate(`/scan/${id}`)}
          style={{ marginTop: "0.5rem" }}
        >
          Volver al resumen de la sesión
        </button>
      )}
    </main>
  );
}

export default Done;
