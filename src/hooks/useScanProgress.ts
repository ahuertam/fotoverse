/**
 * Hook que se suscribe a los eventos del pipeline de escaneo para un
 * `session_id` dado.
 *
 * Comportamiento (defensa en profundidad contra la race condition del
 * walk ultra-rápido):
 * 1. Al montar, **catch-up**: consulta `getScanStatus` para detectar
 *    escaneos ya terminados (el walk puede ser más rápido que el
 *    tiempo de mount de la página, así que el evento `scan_completed`
 *    puede dispararse ANTES de que nos suscribamos).
 * 2. Suscribe a `scan_progress` y `scan_completed`.
 * 3. **Polling de respaldo** cada 500 ms: aunque el evento en vivo se
 *    pierda (por ejemplo, una navegación rápida), el polling acabará
 *    viendo el estado terminal y sintetizará el `ScanCompleted`.
 * 4. Limpia listeners e interval al desmontar.
 * 5. Si el `sessionId` cambia, re-suscribe.
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

// Tipo "dual" para tolerar la transición snake/camel del
// backend. El backend serializa con rename_all = "camelCase"
// desde el commit df5ccaa; las interfaces de TS mantienen
// nombres snake_case por consistencia con Rust. Por eso leemos
// de ambas keys.
type SessionWithCamel = ScanStatus["session"] & {
  totalFiles?: number;
  id?: number;
};

function synthesizeCompleted(s: ScanStatus): ScanCompleted {
  // Narrowing: s.session.status es uno de los tres terminales
  // (lo garantiza la guard del caller). El cast es seguro.
  const status = s.session.status as "completed" | "cancelled" | "failed";
  const session = (s.session ?? {}) as SessionWithCamel;
  const totalFiles = session.totalFiles ?? session.total_files ?? 0;
  const sessionId = session.id ?? 0;
  return {
    session_id: sessionId,
    total_files: totalFiles,
    // No tenemos duration_ms exacto en la DB; el caller puede
    // mostrar el total_files como dato principal.
    duration_ms: 0,
    status,
    error: null,
  };
}

function isTerminal(status: string): boolean {
  return (
    status === "completed" || status === "cancelled" || status === "failed"
  );
}

export function useScanProgress(sessionId: number | null): UseScanProgressResult {
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [completed, setCompleted] = useState<ScanCompleted | null>(null);
  const [initialStatus, setInitialStatus] = useState<ScanStatus | null>(null);

  useEffect(() => {
    if (sessionId == null) return;

    let cancelled = false;
    const unlistens: Array<Promise<() => void>> = [];

    // 1) Catch-up + 3) Polling de respaldo.
    //    Mientras `completed` sea null, sondeamos la sesión cada 500 ms.
    //    Si encontramos estado terminal, sintetizamos el evento y paramos.
    const pollId = window.setInterval(async () => {
      if (cancelled) return;
      try {
        const status = await getScanStatus(sessionId);
        if (cancelled) return;
        setInitialStatus(status);
        if (isTerminal(status.session.status)) {
          setCompleted(synthesizeCompleted(status));
        }
      } catch {
        // Ignoramos errores puntuales; el siguiente tick reintentará.
      }
    }, 500);

    // 2) Suscripción a los eventos en vivo (vía rápida; el polling es
    //    la red de seguridad).
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
      window.clearInterval(pollId);
      void Promise.all(unlistens).then((fns) => fns.forEach((f) => f()));
    };
  }, [sessionId]);

  return { progress, completed, initialStatus };
}
