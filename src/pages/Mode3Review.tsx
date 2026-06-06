/**
 * Página de revisión del Modo 3 (visual, side-by-side).
 *
 * Layout:
 * - Header: "N / M pares restantes" + barra de progreso.
 * - Par actual: dos thumbnails grandes lado a lado (600×600 cada uno)
 *   con metadata debajo (path, size, fecha).
 * - 4 botones: "Borrar izda" | "Borrar dcha" | "Borrar ambas" |
 *   "Mantener ambas".
 * - Botón "Saltar" (no decide, sólo marca como revisado).
 * - Atajos: ← izda, → dcha, ↓ saltar, space mantener ambas.
 *
 * Las decisiones usan `set_decision` por cada file_id (el backend
 * `bulk_set_decision` ya lo implementa). Al decidir, se marca el par
 * como `reviewed` para que no vuelva a salir.
 */

import { useCallback, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { Thumbnail } from "../components/Thumbnail";
import {
  bulkSetDecision,
  getFilesMeta,
  getManualPairs,
  setDecision,
  skipManualPair,
} from "../lib/tauri";

interface FileMeta {
  id: number;
  path: string;
  sizeBytes: number;
  mtimeNs: number;
}

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

function formatDate(ns: number): string {
  try {
    const ms = Math.floor(ns / 1_000_000);
    return new Date(ms).toLocaleString("es-ES");
  } catch {
    return "—";
  }
}

interface CurrentPair {
  pairId: number;
  fileA: FileMeta;
  fileB: FileMeta;
}

function Mode3Review() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const sessionId = id != null ? Number(id) : null;

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [total, setTotal] = useState(0);
  const [current, setCurrent] = useState<CurrentPair | null>(null);
  const [done, setDone] = useState(false);

  // Carga inicial: primer par pendiente.
  const loadNext = useCallback(
    async (offset: number) => {
      if (sessionId == null) return;
      try {
        const { pairs, total: t } = await getManualPairs(sessionId, offset, 1, "pending");
        setTotal(t);
        if (pairs.length === 0) {
          setCurrent(null);
          setDone(true);
          return;
        }
        const pair = pairs[0];
        // Metadata de los dos ficheros.
        const { files } = await getFilesMeta([pair.fileA, pair.fileB]);
        const byId = new Map(files.map((f) => [f.id, f]));
        const fileA = byId.get(pair.fileA);
        const fileB = byId.get(pair.fileB);
        if (!fileA || !fileB) {
          setError("Ficheros del par ya no existen en la DB");
          return;
        }
        setCurrent({
          pairId: pair.id,
          fileA: {
            id: fileA.id,
            path: fileA.path,
            sizeBytes: fileA.sizeBytes,
            mtimeNs: fileA.mtimeNs,
          },
          fileB: {
            id: fileB.id,
            path: fileB.path,
            sizeBytes: fileB.sizeBytes,
            mtimeNs: fileB.mtimeNs,
          },
        });
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    },
    [sessionId],
  );

  // Carga inicial: sólo al mount, NO cada vez que loadNext cambia
  // (eso causaría un bucle infinito porque loadNext se re-crea
  // cuando cambian los states que toca). Usamos sessionId como
  // dependencia única.
  useEffect(() => {
    if (sessionId == null) return;
    void loadNext(0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  async function decideAndAdvance(
    decision: "delete-a" | "delete-b" | "delete-both" | "keep-both" | "skip",
  ) {
    if (!current) return;
    try {
      if (decision === "skip") {
        await skipManualPair(current.pairId);
      } else if (decision === "delete-a") {
        await setDecision(current.fileA.id, "delete");
        await skipManualPair(current.pairId);
      } else if (decision === "delete-b") {
        await setDecision(current.fileB.id, "delete");
        await skipManualPair(current.pairId);
      } else if (decision === "delete-both") {
        await bulkSetDecision([current.fileA.id, current.fileB.id], "delete");
        await skipManualPair(current.pairId);
      } else if (decision === "keep-both") {
        await bulkSetDecision([current.fileA.id, current.fileB.id], "keep");
        await skipManualPair(current.pairId);
      }
      // Avanza al siguiente par.
      void loadNext(0);
    } catch (e) {
      setError(String(e));
    }
  }

  // Atajos de teclado.
  useEffect(() => {
    if (!current) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        void decideAndAdvance("delete-a");
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        void decideAndAdvance("delete-b");
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        void decideAndAdvance("skip");
      } else if (e.key === " ") {
        e.preventDefault();
        void decideAndAdvance("keep-both");
      } else if (e.key.toLowerCase() === "a") {
        e.preventDefault();
        void decideAndAdvance("delete-both");
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [current]); // eslint-disable-line react-hooks/exhaustive-deps

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
        <p>Cargando pares…</p>
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

  if (done || !current) {
    return (
      <main className="mode1-review">
        <header>
          <h1>Revisión visual</h1>
          <p className="muted">
            🎉 No quedan pares pendientes de revisar. Has terminado
            Modo 3.
          </p>
        </header>
        <button type="button" onClick={() => navigate("/")}>
          Volver al inicio
        </button>
      </main>
    );
  }

  return (
    <main className="mode1-review">
      <header>
        <h1>Revisión visual</h1>
        <p className="summary">
          {total} {total === 1 ? "par" : "pares"} restantes
        </p>
        <p className="muted small">
          ← Borrar izda · → Borrar dcha · A Borrar ambas · ↓ Saltar ·
          Espacio Mantener ambas
        </p>
      </header>

      <section className="side-by-side">
        <div className="side">
          <Thumbnail
            fileId={current.fileA.id}
            title={current.fileA.path}
            className="side-img"
          />
          <p className="path" title={current.fileA.path}>
            {basename(current.fileA.path)}
          </p>
          <p className="muted small">
            {formatBytes(current.fileA.sizeBytes)} ·{" "}
            {formatDate(current.fileA.mtimeNs)}
          </p>
        </div>
        <div className="side">
          <Thumbnail
            fileId={current.fileB.id}
            title={current.fileB.path}
            className="side-img"
          />
          <p className="path" title={current.fileB.path}>
            {basename(current.fileB.path)}
          </p>
          <p className="muted small">
            {formatBytes(current.fileB.sizeBytes)} ·{" "}
            {formatDate(current.fileB.mtimeNs)}
          </p>
        </div>
      </section>

      <section className="actions side-actions">
        <button type="button" onClick={() => void decideAndAdvance("delete-a")}>
          ← Borrar izda
        </button>
        <button type="button" onClick={() => void decideAndAdvance("delete-b")}>
          Borrar dcha →
        </button>
        <button type="button" onClick={() => void decideAndAdvance("delete-both")}>
          Borrar ambas
        </button>
        <button
          type="button"
          className="primary"
          onClick={() => void decideAndAdvance("keep-both")}
        >
          Mantener ambas (Espacio)
        </button>
        <button type="button" onClick={() => void decideAndAdvance("skip")}>
          ↓ Saltar
        </button>
      </section>
    </main>
  );
}

export default Mode3Review;
