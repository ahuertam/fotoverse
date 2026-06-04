/**
 * Hook que se suscribe al evento `scan_progress` para un session_id dado.
 *
 * - Devuelve el último `ScanProgress` recibido (o `null` si todavía nada).
 * - Limpia el listener al desmontar.
 * - Si el `sessionId` cambia, re-suscribe.
 */

import { useEffect, useState } from "react";

import { onScanCompleted, onScanProgress } from "../lib/tauri";
import type { ScanCompleted, ScanProgress } from "../lib/types";

export interface UseScanProgressResult {
  progress: ScanProgress | null;
  completed: ScanCompleted | null;
}

export function useScanProgress(sessionId: number | null): UseScanProgressResult {
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [completed, setCompleted] = useState<ScanCompleted | null>(null);

  useEffect(() => {
    if (sessionId == null) return;

    const unlistens: Array<Promise<() => void>> = [];

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
      void Promise.all(unlistens).then((fns) => fns.forEach((f) => f()));
    };
  }, [sessionId]);

  return { progress, completed };
}
