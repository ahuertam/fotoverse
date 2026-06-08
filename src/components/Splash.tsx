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

import splashUrl from "../assets/splash-placeholder.jpg";

export interface SplashProps {
  onDone?: () => void;
  minDurationMs?: number;
}

export function Splash({ onDone, minDurationMs = 1200 }: SplashProps) {
  const [opacity, setOpacity] = useState(0);
  const [captionOpacity, setCaptionOpacity] = useState(0);

  useEffect(() => {
    // Fade-in de la imagen (300 ms) y del caption (200 ms después).
    const fadeIn = window.setTimeout(() => setOpacity(1), 30);
    const captionIn = window.setTimeout(() => setCaptionOpacity(0.85), 280);

    // Fade-out a los `minDurationMs` (1200 ms por defecto).
    const fadeOut = window.setTimeout(() => {
      setOpacity(0);
      setCaptionOpacity(0);
    }, minDurationMs);

    // Notificar al padre a los `minDurationMs + 600` ms (después
    // del fade-out de 600 ms).
    const done = window.setTimeout(() => {
      onDone?.();
    }, minDurationMs + 600);

    return () => {
      window.clearTimeout(fadeIn);
      window.clearTimeout(captionIn);
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
        alt="fotoverse splash placeholder"
        style={{
          maxWidth: "min(70vw, 640px)",
          maxHeight: "60vh",
          borderRadius: "var(--radius-lg)",
          boxShadow: "0 8px 32px rgba(0,0,0,0.5)",
        }}
      />
      <h1
        style={{
          marginTop: "1.5rem",
          fontSize: "2.5rem",
          color: "var(--color-fg)",
        }}
      >
        fotoverse
      </h1>
      <p
        style={{
          color: "var(--color-fg-muted)",
          fontSize: "1rem",
          marginTop: "0.25rem",
        }}
      >
        Encuentra duplicados en tu biblioteca
      </p>
      <p
        style={{
          color: "var(--color-fg-muted)",
          fontSize: "0.7rem",
          marginTop: "2rem",
          opacity: captionOpacity,
          transition: "opacity 400ms ease-in-out",
          fontFamily: "var(--font-mono)",
        }}
      >
        ⚠ placeholder · el branding real llegará en v0.2
      </p>
    </div>
  );
}

export default Splash;
