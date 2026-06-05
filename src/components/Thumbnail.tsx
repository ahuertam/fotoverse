/**
 * Componente `<Thumbnail fileId={id} />` (Fase 4).
 *
 * - Llama a `getThumbnail(fileId)` para obtener la ruta absoluta
 *   del JPEG cacheado.
 * - La convierte a un `src` que la WebView de Tauri puede cargar
 *   directamente desde disco mediante `convertFileSrc`.
 * - Mientras carga (o si falla), muestra un placeholder gris.
 * - Cachea la respuesta en `useState` local: si el componente se
 *   re-monta con el mismo fileId, no vuelve a llamar al backend.
 */

import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";

import { getThumbnail } from "../lib/tauri";

export interface ThumbnailProps {
  fileId: number;
  /** Texto del tooltip. Si no se da, sólo muestra el alt. */
  title?: string;
  /** Clase extra para el <img>. */
  className?: string;
}

export function Thumbnail({ fileId, title, className }: ThumbnailProps) {
  const [src, setSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setSrc(null);
    setError(null);
    void (async () => {
      try {
        const absPath = await getThumbnail(fileId, 0);
        if (cancelled) return;
        setSrc(convertFileSrc(absPath));
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [fileId]);

  if (src != null) {
    return (
      <img
        src={src}
        alt=""
        title={title}
        className={className}
        draggable={false}
      />
    );
  }

  // Placeholder: gradiente gris con un icono de "cargando".
  return (
    <div className={`thumb-placeholder ${className ?? ""}`} title={title ?? (error ?? "cargando…")}>
      {error ? "⚠" : "…"}
    </div>
  );
}

export default Thumbnail;
