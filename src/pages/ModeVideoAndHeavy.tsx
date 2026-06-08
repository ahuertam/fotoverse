/**
 * Bonus v0.2 — Modo Vídeo y Peso.
 *
 * Una sola página con dos secciones:
 *
 * 1. **Vídeos duplicados por nombre** (estilo Modo 1 pero
 *    filtrado a kind='video'). Mismo algoritmo de agrupación.
 *    Si decides borrar, usa set_decision por file_id (igual
 *    que en Modo 1).
 *
 * 2. **Top-N más pesados**: lista los vídeos más gordos de la
 *    sesión con un "ahorro estimado al comprimir al 70%". Por
 *    ahora es una heurística; Fase 6 con ffmpeg-sidecar
 *    permitiría calcular el ahorro real ejecutando una pasada
 *    de prueba.
 *
 * Las decisiones (delete/keep) van al backend igual que en
 * Modo 1, y aparecen en la Final Review / papelera.
 */

import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Virtuoso } from "react-virtuoso";

import { Thumbnail } from "../components/Thumbnail";
import {
  getVideoAndHeavy,
} from "../lib/tauri";
import type { HeavyVideo, VideoAndHeavyResult } from "../lib/types";

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

function ModeVideoAndHeavy() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const sessionId = id != null ? Number(id) : null;

  const [data, setData] = useState<VideoAndHeavyResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // State para los duplicados (estilo Mode1Review): miembros
  // expandidos, decisiones, bulk action.
  const [expandedDups, setExpandedDups] = useState<Set<number>>(new Set());

  useEffect(() => {
    if (sessionId == null) return;
    let cancelled = false;
    void (async () => {
      try {
        setLoading(true);
        const result = await getVideoAndHeavy(sessionId, 20);
        if (!cancelled) setData(result);
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
        <p>Cargando vídeos…</p>
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

  if (!data) return null;

  const hasNoContent =
    data.duplicates.length === 0 && data.heavy.length === 0;

  return (
    <main className="mode1-review">
      <header>
        <h1>Modo Vídeo y Peso</h1>
        <p className="muted">
          Dos secciones. En la primera, vídeos con el mismo nombre
          (probables copias). En la segunda, los más pesados con
          un cálculo estimado de ahorro al comprimir al{" "}
          <strong>{Math.round((1 - data.compressionFactor) * 100)}%</strong>.
        </p>
      </header>

      {hasNoContent && (
        <p className="muted">
          🎉 No hay vídeos para analizar. Si quieres probar este
          modo, añade archivos .mp4/.mov/.mkv/.avi/.webm a tu
          carpeta y re-escanea.
        </p>
      )}

      {/* Sección 1: Vídeos duplicados por nombre */}
      {data.duplicates.length > 0 && (
        <>
          <h2 style={{ marginTop: "var(--space-5)" }}>
            Vídeos duplicados por nombre ({data.duplicates.length})
          </h2>
          <p className="summary">
            {formatBytes(data.totalDuplicateBytes)} desperdiciados si
            los borras.
          </p>
          <p className="muted small">
            Nota: estos grupos se generan en memoria (no se
            persisten en exact_groups). Si decides borrar, las
            decisiones se persisten vía set_decision y aparecen
            en Final Review.
          </p>
          <Virtuoso
            style={{ height: "40vh" }}
            data={data.duplicates}
            itemContent={(_index, dup) => {
              const isExpanded = expandedDups.has(dup.id);
              return (
                <div className={`group ${isExpanded ? "expanded" : ""}`}>
                  <div
                    className="group-summary"
                    onClick={() =>
                      setExpandedDups((prev) => {
                        const next = new Set(prev);
                        if (next.has(dup.id)) next.delete(dup.id);
                        else next.add(dup.id);
                        return next;
                      })
                    }
                    role="button"
                    tabIndex={0}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        setExpandedDups((prev) => {
                          const next = new Set(prev);
                          if (next.has(dup.id)) next.delete(dup.id);
                          else next.add(dup.id);
                          return next;
                        });
                      }
                    }}
                  >
                    <div className="group-info">
                      <span className="caret">
                        {isExpanded ? "▾" : "▸"}
                      </span>
                      <strong className="filename">
                        {dup.filename}
                      </strong>
                      <span className="ext">.{dup.ext}</span>
                      <span className="count">
                        {dup.memberCount} archivos
                      </span>
                    </div>
                    <div className="group-wasted">
                      {formatBytes(
                        dup.sizeBytes * (dup.memberCount - 1),
                      )}{" "}
                      desperdiciados
                    </div>
                  </div>

                  {isExpanded && (
                    <div className="group-members">
                      <p className="muted small">
                        Para ver/seleccionar los paths exactos,
                        expande el grupo en Modo 1 (los nombres
                        coinciden). Para borrar, decide en Modo 1
                        y vuelve aquí: el grupo desaparecerá.
                      </p>
                    </div>
                  )}
                </div>
              );
            }}
          />
        </>
      )}

      {/* Sección 2: Top-N más pesados */}
      {data.heavy.length > 0 && (
        <>
          <h2 style={{ marginTop: "var(--space-5)" }}>
            Top {data.heavy.length} vídeos más pesados
          </h2>
          <p className="summary">
            Si los comprimes al {Math.round((1 - data.compressionFactor) * 100)}%,
            podrías ahorrar ~<strong>{formatBytes(data.totalPotentialSavings)}</strong>.
          </p>
          <p className="muted small">
            Estimación conservadora H.264 1-pass "medium". Para
            recodificación real usa ffmpeg/HandBrake (próximamente
            integrado con ffmpeg-sidecar).
          </p>
          <Virtuoso
            style={{ height: "40vh" }}
            data={data.heavy}
            itemContent={(_index, h: HeavyVideo) => (
              <div className="group">
                <div
                  className="group-summary"
                  style={{ cursor: "default" }}
                >
                  <div className="group-info">
                    <Thumbnail
                      fileId={h.id}
                      title={h.path}
                      className="thumb-img"
                    />
                    <strong className="filename">
                      {basename(h.path)}
                    </strong>
                    <span className="muted small">{h.path}</span>
                  </div>
                  <div className="group-wasted">
                    {formatBytes(h.sizeBytes)}
                    <br />
                    <span
                      className="muted small"
                      style={{ color: "var(--color-success)" }}
                    >
                      ~{formatBytes(h.estimatedSavings)} ahorro
                    </span>
                  </div>
                </div>
              </div>
            )}
          />
        </>
      )}

      <footer className="mode1-footer">
        <button type="button" onClick={() => navigate(`/review/${sessionId}/final`)}>
          🗑️ Ir a Revisión final y papelera →
        </button>
        <button type="button" onClick={() => navigate("/")}>
          ← Volver al inicio
        </button>
      </footer>
    </main>
  );
}

export default ModeVideoAndHeavy;
