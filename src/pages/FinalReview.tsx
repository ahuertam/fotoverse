/**
 * Página de Final Review (Fase 8).
 *
 * Lista TODOS los ficheros marcados como `delete` en los 3 modos.
 * El usuario puede desmarcar los que no quiere con un checkbox.
 * El botón "Enviar a la papelera" ejecuta el borrado.
 *
 * Si el nº de ficheros o bytes supera el umbral (1000 / 5 GB),
 * el botón abre un modal de confirmación tipada: el usuario debe
 * escribir "ELIMINAR" para confirmar.
 *
 * Tras el borrado, navega a `/done` con el reporte.
 */

import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Virtuoso } from "react-virtuoso";

import { Thumbnail } from "../components/Thumbnail";
import {
  executeDeletions,
  getFinalReview,
} from "../lib/tauri";
import type { DeletionReport, FinalReviewItem } from "../lib/types";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024)
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function basename(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i >= 0 ? path.slice(i + 1) : path;
}

const THRESHOLD_FILES = 1000;
const THRESHOLD_BYTES = 5 * 1024 * 1024 * 1024;

function FinalReview() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const sessionId = id != null ? Number(id) : null;

  const [items, setItems] = useState<FinalReviewItem[]>([]);
  const [totalBytes, setTotalBytes] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  /** Set de file_ids que el usuario HA desmarcado (no se borran). */
  const [unchecked, setUnchecked] = useState<Set<number>>(new Set());

  const [confirmOpen, setConfirmOpen] = useState(false);
  const [typedConfirm, setTypedConfirm] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (sessionId == null) return;
    let cancelled = false;
    void (async () => {
      try {
        setLoading(true);
        const result = await getFinalReview(sessionId);
        if (!cancelled) {
          setItems(result.items);
          setTotalBytes(result.totalBytes);
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  /** Items que se borrarán (los que el usuario NO desmarcó). */
  const toDelete = useMemo(
    () => items.filter((it) => !unchecked.has(it.fileId)),
    [items, unchecked],
  );
  const toDeleteBytes = useMemo(
    () => toDelete.reduce((acc, it) => acc + it.sizeBytes, 0),
    [toDelete],
  );
  const toDeleteCount = toDelete.length;
  const needsTypedConfirm =
    toDeleteCount > THRESHOLD_FILES || toDeleteBytes > THRESHOLD_BYTES;

  function toggleUnchecked(fileId: number) {
    setUnchecked((prev) => {
      const next = new Set(prev);
      if (next.has(fileId)) {
        next.delete(fileId);
      } else {
        next.add(fileId);
      }
      return next;
    });
  }

  function handleConfirmClick() {
    if (toDeleteCount === 0) return;
    if (needsTypedConfirm) {
      setTypedConfirm("");
      setConfirmOpen(true);
    } else {
      void doDelete();
    }
  }

  async function doDelete() {
    if (sessionId == null) return;
    setBusy(true);
    try {
      const fileIds = toDelete.map((it) => it.fileId);
      const { report } = await executeDeletions(sessionId, fileIds);
      navigate(`/done/${sessionId}`, { state: { report } as { report: DeletionReport } });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setConfirmOpen(false);
    }
  }

  if (sessionId == null) {
    return (
      <main className="mode1-review">
        <p>ID de sesión inválido.</p>
        <button type="button" onClick={() => navigate("/")}>
          Volver al inicio
        </button>
      </main>
    );
  }

  if (loading) {
    return (
      <main className="mode1-review">
        <p>Cargando revisión final…</p>
      </main>
    );
  }

  if (error) {
    return (
      <main className="mode1-review">
        <p className="error">⚠️ {error}</p>
        <button type="button" onClick={() => navigate("/")}>
          Volver al inicio
        </button>
      </main>
    );
  }

  if (items.length === 0) {
    return (
      <main className="mode1-review">
        <nav className="breadcrumb">
          <button type="button" onClick={() => navigate("/")}>
            Inicio
          </button>
          <span className="bc-sep">›</span>
          <button
            type="button"
            onClick={() => navigate(`/scan/${sessionId}`)}
          >
            Escaneo #{sessionId}
          </button>
          <span className="bc-sep">›</span>
          <span className="bc-current">🗑️ Papelera</span>
        </nav>
        <header>
          <h1>🗑️ Papelera</h1>
          <p className="muted">
            🎉 No has marcado ningún fichero para borrar. Vuelve
            a los modos anteriores y selecciona los duplicados
            que quieras enviar a la papelera.
          </p>
        </header>
        <button type="button" onClick={() => navigate(`/scan/${sessionId}`)}>
          ← Resumen del escaneo
        </button>
      </main>
    );
  }

  return (
    <main className="mode1-review">
      <nav className="breadcrumb">
        <button type="button" onClick={() => navigate("/")}>
          Inicio
        </button>
        <span className="bc-sep">›</span>
        <button
          type="button"
          onClick={() => navigate(`/scan/${sessionId}`)}
        >
          Escaneo #{sessionId}
        </button>
        <span className="bc-sep">›</span>
        <span className="bc-current">🗑️ Papelera</span>
      </nav>

      <header>
        <h1>🗑️ Papelera</h1>
        <p className="muted small">
          Aquí ves todo lo que marcaste para borrar en los 3
          modos. Los ficheros que NO quieres, desmárcalos. Los
          demás se enviarán a la papelera de tu sistema (Finder
          en Mac, Recycle Bin en Windows) — nunca se borran
          permanentemente.
        </p>
        <p className="summary">
          <strong>{items.length}</strong>{" "}
          {items.length === 1 ? "fichero" : "ficheros"} ·{" "}
          <strong>{formatBytes(totalBytes)}</strong> totales
        </p>
        <p className="summary">
          Se enviarán a la papelera:{" "}
          <strong>{toDeleteCount}</strong>{" "}
          {toDeleteCount === 1 ? "fichero" : "ficheros"} ·{" "}
          <strong>{formatBytes(toDeleteBytes)}</strong>
          {needsTypedConfirm && (
            <span style={{ color: "var(--color-warn)" }}>
              {" "}
              ⚠️ borrado grande — pedirá confirmación extra
            </span>
          )}
        </p>
      </header>

      <Virtuoso
        style={{ height: "60vh" }}
        data={items}
        itemContent={(_index, item) => {
          const isUnchecked = unchecked.has(item.fileId);
          return (
            <div
              className={`group ${isUnchecked ? "expanded unchecked" : ""}`}
            >
              <label
                className="group-summary"
                style={{ cursor: "pointer" }}
              >
                <input
                  type="checkbox"
                  checked={!isUnchecked}
                  onChange={() => toggleUnchecked(item.fileId)}
                  style={{ marginRight: "0.75rem" }}
                />
                <div className="group-info">
                  <Thumbnail
                    fileId={item.fileId}
                    title={item.path}
                    className="thumb-img"
                  />
                  <strong className="filename">
                    {basename(item.path)}
                  </strong>
                  <span className="muted small">
                    {item.path}
                  </span>
                </div>
                <div className="group-wasted">
                  {formatBytes(item.sizeBytes)}
                </div>
              </label>
            </div>
          );
        }}
      />

      <footer className="mode1-footer">
        <button type="button" onClick={() => navigate(`/review/${sessionId}/mode1`)}>
          ← Volver a Modo 1
        </button>
        <button
          type="button"
          className="primary"
          onClick={handleConfirmClick}
          disabled={busy || toDeleteCount === 0}
        >
          {busy
            ? "Enviando a la papelera…"
            : `🗑️ Enviar ${toDeleteCount} a la papelera (${formatBytes(toDeleteBytes)})`}
        </button>
      </footer>

      {confirmOpen && (
        <div
          className="modal-backdrop"
          onClick={() => !busy && setConfirmOpen(false)}
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.6)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 10000,
          }}
        >
          <div
            className="modal"
            onClick={(e) => e.stopPropagation()}
            style={{
              background: "var(--color-bg-elevated)",
              borderRadius: "var(--radius-lg)",
              padding: "var(--space-5)",
              maxWidth: "32rem",
              width: "90%",
            }}
          >
            <h2 style={{ color: "var(--color-warn)" }}>
              ⚠️ Vas a borrar mucho
            </h2>
            <p>
              Vas a enviar <strong>{toDeleteCount}</strong>{" "}
              {toDeleteCount === 1 ? "fichero" : "ficheros"} (
              <strong>{formatBytes(toDeleteBytes)}</strong>) a la
              papelera de tu sistema.
            </p>
            <p
              className="muted small"
              style={{
                background: "rgba(0,0,0,0.3)",
                padding: "0.75rem",
                borderRadius: "4px",
                marginTop: "0.75rem",
              }}
            >
              ℹ️ No se borran permanentemente: van a la papelera
              de tu sistema y puedes recuperarlos desde ahí.
            </p>
            <p style={{ marginTop: "1rem" }}>
              Para confirmar, escribe{" "}
              <code style={{ background: "#000", padding: "2px 6px" }}>
                enviar a la papelera
              </code>{" "}
              en el campo de abajo:
            </p>
            <input
              type="text"
              value={typedConfirm}
              onChange={(e) => setTypedConfirm(e.target.value)}
              autoFocus
              disabled={busy}
              style={{
                width: "100%",
                padding: "0.5rem",
                fontSize: "1rem",
                background: "#000",
                color: "#fff",
                border: "1px solid #555",
                borderRadius: "4px",
                marginTop: "0.5rem",
              }}
            />
            <div
              style={{
                display: "flex",
                gap: "0.5rem",
                marginTop: "1rem",
                justifyContent: "flex-end",
              }}
            >
              <button
                type="button"
                onClick={() => setConfirmOpen(false)}
                disabled={busy}
              >
                Cancelar
              </button>
              <button
                type="button"
                className="primary"
                disabled={
                  busy || typedConfirm.toLowerCase() !== "enviar a la papelera"
                }
                onClick={() => void doDelete()}
              >
                {busy ? "Enviando…" : "🗑️ Sí, enviar a la papelera"}
              </button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}

export default FinalReview;
