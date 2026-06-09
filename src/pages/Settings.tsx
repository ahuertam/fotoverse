/**
 * Página de Settings (Fase 9).
 *
 * Permite al usuario ajustar:
 * - Incluir archivos ocultos en el walk (default: true)
 * - Tema visual (dark / light)
 *
 * La última carpeta usada se persiste automáticamente en
 * `pickFolder()` y se muestra aquí como info.
 */

import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";

import { getSettings, setSetting, type AppSettings } from "../lib/tauri";

function Settings() {
  const navigate = useNavigate();
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    void getSettings().then(setSettings);
  }, []);

  async function update<K extends keyof AppSettings>(
    key: K,
    value: AppSettings[K],
  ) {
    setSaving(true);
    setSettings((prev) => (prev ? { ...prev, [key]: value } : prev));
    try {
      await setSetting(key, value);
    } finally {
      setSaving(false);
    }
  }

  if (!settings) {
    return (
      <main className="welcome">
        <p>Cargando ajustes…</p>
      </main>
    );
  }

  return (
    <main className="welcome">
      <h1>Ajustes</h1>
      <p className="tagline">
        Preferencias guardadas en tu dispositivo (no se sincronizan).
      </p>

      <section
        className="status"
        style={{ textAlign: "left", maxWidth: "32rem" }}
      >
        <label
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.5rem",
            margin: "0.5rem 0",
          }}
        >
          <input
            type="checkbox"
            checked={settings.includeHidden}
            onChange={(e) => void update("includeHidden", e.target.checked)}
            disabled={saving}
          />
          Incluir archivos y carpetas ocultos
        </label>
        <p
          className="muted small"
          style={{ marginTop: 0, marginLeft: "1.5rem" }}
        >
          Por defecto activado: muchos sistemas guardan fotos en
          carpetas como <code>.photoslibrary</code> o
          <code> .config</code>. Desactívalo si quieres excluirlas.
          El cambio se aplicará en el próximo escaneo.
        </p>

        <label
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.5rem",
            margin: "0.5rem 0",
          }}
        >
          <span>Tema:</span>
          <select
            value={settings.theme}
            onChange={(e) =>
              void update("theme", e.target.value as "dark" | "light")
            }
            disabled={saving}
          >
            <option value="dark">Oscuro</option>
            <option value="light">Claro (próximamente)</option>
          </select>
        </label>
        <p
          className="muted small"
          style={{ marginTop: 0, marginLeft: "1.5rem" }}
        >
          Por ahora sólo está disponible el tema oscuro.
        </p>

        {settings.lastFolder && (
          <div style={{ marginTop: "1rem" }}>
            <p className="muted small" style={{ margin: 0 }}>
              Última carpeta escaneada:
            </p>
            <p
              className="path"
              style={{ margin: 0, fontFamily: "var(--font-mono)" }}
            >
              {settings.lastFolder}
            </p>
          </div>
        )}
      </section>

      <section
        className="status"
        style={{
          textAlign: "left",
          maxWidth: "32rem",
          marginTop: "1.5rem",
          border: "1px dashed #555",
        }}
      >
        <p
          className="muted small"
          style={{ margin: 0, fontFamily: "var(--font-mono)" }}
        >
          🕳 Easter egg slot
        </p>
        <p
          className="muted small"
          style={{ marginTop: "0.5rem" }}
        >
          ¿Quieres que tu splash muestre una imagen random en
          lugar del logo? Renombra tu imagen a{" "}
          <code>src/assets/easter-egg.{`{jpg,png,webp}`}</code> y
          aparecerá aquí (siguiente release, v0.2).
        </p>
      </section>

      <button
        type="button"
        onClick={() => navigate("/")}
        style={{ marginTop: "1rem" }}
      >
        Volver al inicio
      </button>
    </main>
  );
}

export default Settings;
