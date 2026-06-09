/**
 * Componente `<Splash />` (Fase 9 polish).
 *
 * Pantalla de bienvenida inicial que se muestra durante ~1.2 s al
 * cargar la app. Incluye:
 *  - Una imagen placeholder (splash-placeholder.jpg) que se
 *    desvanece suavemente.
 *  - Un caption claramente etiquetado como "placeholder" para
 *    que nadie se confunda: el branding real aún no existe.
 *  - El nombre "fotoverse" + tagline fija.
 *
 * Props:
 *  - onDone: callback al terminar la animación.
 *  - minDurationMs: mínimo de tiempo en pantalla (default 1200 ms).
 */

import { useEffect, useState } from "react";

import splashUrl from "../assets/logo.jpeg";

export interface SplashProps {
  onDone?: () => void;
  minDurationMs?: number;
}

export function Splash({ onDone, minDurationMs = 1200 }: SplashProps) {
  const [opacity, setOpacity] = useState(0);

  useEffect(() => {
    // Fade-in (30 ms) → mantener (minDurationMs) → fade-out (600 ms)
    // → notificar al padre.
    const fadeIn = window.setTimeout(() => setOpacity(1), 30);
    const fadeOut = window.setTimeout(() => setOpacity(0), minDurationMs);
    const done = window.setTimeout(() => onDone?.(), minDurationMs + 600);

    return () => {
      window.clearTimeout(fadeIn);
      window.clearTimeout(fadeOut);
      window.clearTimeout(done);
    };
  }, [onDone, minDurationMs]);

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "var(--color-bg)",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9999,
        opacity: opacity,
        transition: "opacity 600ms ease-in-out",
        pointerEvents: opacity > 0 ? "auto" : "none",
      }}
    >
      <img
        src={splashUrl}
        alt="fotoverse"
        style={{
          width: "min(60vw, 360px)",
          height: "auto",
          borderRadius: "32px",
          boxShadow: "0 16px 64px rgba(0,0,0,0.5)",
        }}
      />
      <p
        style={{
          color: "var(--color-fg-muted)",
          fontSize: "1.1rem",
          marginTop: "1.5rem",
          maxWidth: "32rem",
          textAlign: "center",
        }}
      >
        Encuentra fotos y vídeos duplicados en tu biblioteca
      </p>
    </div>
  );
}

export default Splash;
