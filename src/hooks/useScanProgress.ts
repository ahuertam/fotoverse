/**
 * Hook que se suscribe a los eventos del pipeline de escaneo para un
 * `session_id` dado.
 *
 * Comportamiento:
 * 1. Al montar, **catch-up**: consulta `getScanStatus` para detectar
 *    escaneos ya terminados (race condition: el walk puede ser más
 *    rápido que el tiempo de mount de la página, así que el evento
 *    `scan_completed` puede dispararse ANTES de que nos suscribamos).
 * 2. Suscribe a `scan_progress` y `scan_completed`.
 * 3. Limpia los listeners al desmontar.
 * 4. Si el `sessionId` cambia, re-suscribe.
 */

import { useEffect, useState } from "react";

import { getScanStatus, onScanCompleted, onScanProgress } from "../lib/tauri";
import type { ScanCompleted, ScanProgress, ScanStatus } from "../lib/types";

export interface UseScanProgressResult {
  progress: ScanProgress | null;
  completed: ScanCompleted | null;
  /** Estado inicial cargado al montar (también útil para el header). */
  initialStatus: ScanStatus | null;
}

function synthesizeCompleted(s: ScanStatus): ScanCompleted {
  // Narrowing: s.session.status es uno de los tres terminales
  // (lo garantiza la guard del caller). El cast es seguro.
  const status = s.session.status as "completed" | "cancelled" | "failed";
  return {
    session_id: s.session.id,
    total_files: s.session.total_files,
    // No tenemos duration_ms exacto en la DB; el caller puede
    // mostrar el total_files como dato principal.
    duration_ms: 0,
    status,
    error: null,
  };
}

export function useScanProgress(sessionId: number | null): UseScanProgressResult {
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [completed, setCompleted] = useState<ScanCompleted | null>(null);
  const [initialStatus, setInitialStatus] = useState<ScanStatus | null>(null);

  useEffect(() => {
    if (sessionId == null) return;

    let cancelled = false;
    const unlistens: Array<Promise<() => void>> = [];

    // 1) Catch-up: si la página se monta después de que el walk terminó,
    //    getScanStatus nos lo dice y sintetizamos el evento.
    void (async () => {
      try {
        const status = await getScanStatus(sessionId);
        if (cancelled) return;
        setInitialStatus(status);
        if (
          status.session.status === "completed" ||
          status.session.status === "cancelled" ||
          status.session.status === "failed"
        ) {
          setCompleted(synthesizeCompleted(status));
        }
      } catch {
        if (!cancelled) setInitialStatus(null);
      }
    })();

    // 2) Suscripción a los eventos en vivo.
    unlistens.push(
      onScanProgress((p) => {
        if (p.session_id === sessionId) setProgress(p);
      }),
    );
    unlistens.push(
      onScanCompleted((c) => {
        if (c.session_id === sessionId) setCompleted(c);
      }),
    );

    return () => {
      cancelled = true;
      void Promise.all(unlistens).then((fns) => fns.forEach((f) => f()));
    };
  }, [sessionId]);

  return { progress, completed, initialStatus };
}
