# fotoverse — Plan v0.2.0

> Esta es la hoja de ruta de la **siguiente release** después de v0.1.0.
> v0.1.0 ya está en `main` con tag `v0.1.0`. Aquí priorizamos lo que
> **diferimos** de v0.1.0 + bugs descubiertos + features que
> añaden valor real.

---

## 1. Contexto

v0.1.0 es la primera release jugable end-to-end:

- 4 modos de revisión (exacto, perceptual, visual, vídeos+peso)
- Thumbnails cacheados, pHash propio, papelera real
- 11 tests Rust, 0 tests JS
- Branding mínimo (logo propio, iconos regenerados)
- Sin CI/CD, sin code signing, sin auto-update

**Limitaciones conocidas de v0.1.0** (de `README.md`):
- ffmpeg-sidecar no implementado (Modo 2 perceptual de vídeos)
- Toggle "incluir archivos ocultos" no cableado al walk
- Tema claro no implementado
- Code signing + auto-update pendientes (requieren $$)
- Sin capturas de pantalla reales (sustituir mock-tauri.html)

**Limitaciones de UX** descubiertas en dev-loop:
- "Coincidencias perceptuales" → renombrado a "Similares" (hecho en v0.1.0 final)
- "Modo 1/2/3" → renombrado a "Duplicados exactos / Similares / Revisión visual" (hecho)
- Snippets muy técnicos en algunas páginas (sustituidos por lenguaje humano en v0.1.0 final)
- "NaN GB" en `totalWasted` (arreglado con fallback snake/camel)
- Bucle infinito en `useEffect` de `Mode3Review` (arreglado en v0.1.0)
- Race condition walk ultra-rápido vs mount de página (arreglado con catch-up + polling)

---

## 2. Fases propuestas para v0.2.0

> Total estimado: **10–14 días** de trabajo para un dev solo.

### Fase A — Code signing & release pipeline (DÍA 1–2)

**Objetivo**: que `pnpm tauri build` produzca binarios firmados
y los suba a GitHub Releases automáticamente en tags `v*`.

**Criterios de "hecho"**:

- [ ] **macOS**: build universal (aarch64 + x86_64), firma con
      Apple Developer ID, notarización con `xcrun notarytool`,
      staple del ticket. Sube `.dmg` y `.app.zip` a GitHub Release.
- [ ] **Windows**: build NSIS, firma con Azure Trusted Signing
      (cuenta gratuita o de pago). Sube `.exe` y `.msi`.
- [ ] `.github/workflows/ci.yml`: en cada push, corre `cargo test`,
      `cargo clippy --all-targets -- -D warnings`, `pnpm tsc --noEmit`,
      `pnpm lint`, `pnpm test`. Bloquea PRs con errores.
- [ ] `.github/workflows/release.yml`: en tags `v*`, corre el build
      de release cross-platform.
- [ ] Secrets de GitHub documentados: `APPLE_IDENTITY`,
      `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
      `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`,
      `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`,
      `AZURE_CLIENT_SECRET`, `AZURE_KEY_VAULT_URI`.
- [ ] `tauri-plugin-updater` wired: el binario en la app
      detecta nueva versión, muestra banner "Hay una nueva
      versión disponible" en Welcome.

**Coste externo**: $99/año Apple Developer ID. Azure Trusted
Signing tiene tier gratuito limitado.

**Riesgo**: code signing es notorious por los problemas
de provisioning. **Fase más arriesgada** del roadmap.

---

### Fase B — Easter egg + branding real (DÍA 1)

**Objetivo**: implementar el Easter egg slot que dejé en `Settings.tsx`
como placeholder. Si el usuario pone `src/assets/easter-egg.jpg` (o
`.png`/`.webp`), la splash y/o un Easter egg button en Settings
lo muestra.

**Criterios de "hecho"**:

- [ ] Detección dinámica: si `src/assets/easter-egg.{jpg,png,webp}`
      existe, mostrarlo como splash alternativa (alterna con el
      logo en cada arranque, 50/50).
- [ ] Botón "Mostrar un secreto" en Settings que abre un modal
      con la imagen (cuando existe). Si no existe, dice "añade
      `src/assets/easter-egg.jpg` para habilitar".
- [ ] El logo de v0.1.0 (`fotologo.png`) se queda como logo
      "serio" por defecto. El Easter egg es opt-in.
- [ ] Documentación en `docs/CONTRIBUTING.md#easter-egg`.

**Riesgo**: ninguno. Es una feature puramente aditiva.

---

### Fase C — Tests JS con Vitest (DÍA 1–2)

**Objetivo**: cubrir los componentes críticos del frontend con
tests. v0.1.0 tiene 11 tests Rust y 0 tests JS — unbalanced.

**Criterios de "hecho"**:

- [ ] `vitest` + `@testing-library/react` + `jsdom` añadidos a
      `devDependencies`. `vitest.config.ts` configurado.
- [ ] `npm test` corre los tests JS (añadido a `package.json`).
- [ ] Tests prioritarios:
      - [ ] `Mode3Review`: atajos de teclado (←, →, ↓, A, Espacio, Esc).
      - [ ] `FinalReview`: checkbox de "desmarcar", botón de
            "Enviar a la papelera" se deshabilita si 0 seleccionados.
      - [ ] `FinalReview`: el modal de confirmación tipada sólo
            aparece si > 1000 files o > 5 GB.
      - [ ] `Mode1Review`: bulk "Mantener el más antiguo" marca
            correctamente.
      - [ ] `Welcome`: splash se desvanece tras 1.2 s.
      - [ ] `useScanProgress`: el fallback snake/camel de
            `synthesizeCompleted` funciona.
- [ ] Coverage target: ≥ 60% en `src/pages/` y `src/hooks/`.

**Riesgo**: bajo. Los tests son unitarios, no E2E.

---

### Fase D — Toggle "incluir archivos ocultos" (DÍA 1)

**Objetivo**: cablear el setting `includeHidden` (que ya existe en
`AppSettings` y se persiste con `tauri-plugin-store`) al walk
de Rust.

**Criterios de "hecho"**:

- [ ] `start_scan` acepta `options: ScanOptions { include_hidden: bool }`.
- [ ] `WalkDir::new(root).skip_hidden(!include_hidden)`.
- [ ] Welcome lee `includeHidden` de `getSettings()` y lo pasa
      al scan.
- [ ] El setting "incluir archivos ocultos" en `Settings.tsx` se
      aplica de verdad (ya está en la UI, sólo falta el cable).
- [ ] Test Rust: scan con `include_hidden=false` ignora `.git/`,
      `.photoslibrary`, etc. Scan con `include_hidden=true` los
      incluye.

**Riesgo**: bajo. Es una feature aditiva + un cambio menor en
`WalkDir`.

---

### Fase E — Tema claro (DÍA 1)

**Objetivo**: implementar el theme switching. v0.1.0 sólo tiene
dark theme con tokens CSS fijos. v0.2 añadirá tema claro + respeto
a `prefers-color-scheme` del SO.

**Criterios de "hecho"**:

- [ ] Refactor `src/styles/global.css` a custom properties
      ya casi todo lo es. Limpiar los pocos `color: #...` hardcoded.
- [ ] `[data-theme="light"]` define los tokens claros (background
      `#fafafa`, fg `#1a1c22`, accent `#0066ff`).
- [ ] El setting `theme` de `AppSettings` se aplica al
      `document.documentElement.dataset.theme`.
- [ ] Por defecto, si `theme === "system"`, leer
      `window.matchMedia('(prefers-color-scheme: dark)')`.
- [ ] Settings.tsx: dropdown de 3 opciones ("Oscuro", "Claro",
      "Según mi sistema").
- [ ] Captura de pantalla de ambos temas para el README.

**Riesgo**: bajo. El CSS ya usa variables.

---

### Fase F — ffmpeg-sidecar + Modo vídeos perceptual (DÍA 3–4)

**Objetivo**: implementar el hash perceptual de vídeos. v0.1.0 sólo
tiene el hash perceptual de imágenes. v0.2 añadirá vídeos.

**Criterios de "hecho"**:

- [ ] `ffmpeg-sidecar = "3"` en `Cargo.toml`. El sidecar descarga
      el binario de ffmpeg en el primer `tauri build` (no en dev).
- [ ] `src-tauri/src/hash/video_hash.rs`:
      - [ ] Probe de duración con ffprobe.
      - [ ] Si duración < 1s, un único frame en t=0.
      - [ ] Si duración >= 1s, 3 frames en 0.1·D, 0.5·D, 0.9·D
            con `-vf scale=320:-1 -frames:v 1 -f image2pipe`.
      - [ ] pHash por frame (re-usa el código de `image_hash.rs`).
      - [ ] vhash = primer frame, vhash_extra = concat 2º+3º (16 bytes).
- [ ] `hash_status` en `files` permite 'ok' / 'error' / 'pending'
      (igual que para imágenes).
- [ ] Backend emite evento `scan_progress` con `phase='hashing_videos'`
      y `files_seen` actualizado (el frontend ya tiene la fase
      en el union TS, sólo falta cablear al backend).
- [ ] `detect/perceptual.rs` se extiende: para vídeos,
      `vhash` + `vhash_extra` con threshold Hamming ≤ 12
      (más generoso que imágenes porque los vídeos son ruidosos).
- [ ] Modo 2 muestra vídeos perceptual con badge
      "Hamming: N · Vídeo" (distinguiendo de imágenes).
- [ ] Modo 4 (vídeos y peso) usa el bitrate real de ffprobe
      en lugar de la heurística actual.

**Riesgo**: medio. ffmpeg-sidecar tiene su propia DX (instalar,
configurar el sidecar, manejar los argumentos cross-platform).

**Tiempo de descarga del sidecar**: ~50 MB. El usuario lo nota
la primera vez que hace `tauri dev`.

---

### Fase G — Capturas de pantalla reales (DÍA 0.5)

**Objetivo**: sustituir el mock-tauri.html del script `make_gif.sh`
por capturas reales de la app corriendo, e incluirlas en el README.

**Criterios de "hecho"**:

- [ ] `scripts/capture_screenshots.sh`: automatiza el flujo
      completo con Playwright (o puppeteer) apuntando a la app
      Tauri real. Captura: Welcome, scan-done, mode1-expanded,
      mode2, mode3, mode4, final-review, settings, done.
- [ ] `scripts/make_gif.sh` actualizado para usar las capturas
      reales (no el mock).
- [ ] `docs/assets/demo.gif` con el flujo real (no el mock).
- [ ] `docs/assets/screenshots/` con PNGs individuales por
      página, referenciadas desde el README.

**Riesgo**: bajo. Es un script automatizado.

---

### Fase H — Modo vídeos perceptual (sin ffmpeg-sidecar) como Plan B (DÍA 1)

**Objetivo**: si la Fase F se complica (como puede pasar con
ffmpeg-sidecar en Windows), tener un fallback que dé valor
similar sin la dependencia de ffmpeg.

**Criterios de "hecho"**:

- [ ] "Modos 1.5": agrupar vídeos por (filename, ext, ctime, size)
      pero con un threshold de tamaño: si dos vídeos difieren en
      menos del 5% de tamaño, marcarlos como "probablemente el
      mismo contenido recodificado". Es heurística pero detecta
      el caso común (exportar un vídeo dos veces).
- [ ] Modo 4: además del top-N, sección "Vídeos con tamaño
      sospechoso" (los que tienen un gemelo casi idéntico).

**Riesgo**: bajo. Es heurística pura.

**Dependencia**: bloqueada por Fase F si se hace. Si Fase F se
hace, esta fase es opcional.

---

### Fase I — Mejoras de UX (backlog, sin orden)

Pequeñas mejoras que no caben en las fases anteriores:

- [ ] **Drag & drop de carpetas en Welcome** (en vez de sólo el
      dialog). Tauri lo soporta nativamente (`onDragDropEvent`).
- [ ] **Borrar sesión** desde la lista de escaneos anteriores
      (UI de "sesiones recientes" en Welcome, no implementada
      en v0.1.0).
- [ ] **"Re-escanea esta carpeta"** desde el breadcrumb (un
      botón que llama a `startScan(lastFolder)`).
- [ ] **Progress bar global** en Final Review (no por fichero,
      un overview "129 / 412 ficheros revisados").
- [ ] **Atajos globales** (Cmd+, = Settings, Cmd+W = cerrar,
      Cmd+Q = salir) en todas las páginas.
- [ ] **i18n**: extraer todos los strings a `src/i18n/es.json`
      para facilitar la traducción a otros idiomas. (Hoy todo
      está hardcoded en JSX — para v0.2.0 esto debería
      estar centralizado.)

---

## 3. Priorización recomendada

Si tuviera que elegir 4-5 features para una **v0.2.0 mínima viable**:

1. **Fase A** (code signing) — bloqueante para release real en
   Mac/Win store. **Crítico**.
2. **Fase D** (toggle ocultos) — feature de 1 día que completa
   el setting de v0.1.0. **Bajo esfuerzo, alto valor**.
3. **Fase C** (tests JS) — mejora la confianza en los próximos
   PRs. **Bajo esfuerzo, alto valor a largo plazo**.
4. **Fase F** (ffmpeg-sidecar) — completa el feature "vídeos"
   que está a medias. **Alto valor pero alto riesgo**.
5. **Fase B** (easter egg) — bonus divertido, 1 día. **Cosmético**.

**Fase E (tema claro), Fase G (capturas), Fase H (Plan B), Fase I
(mejoras)** son nice-to-have para una v0.3.0.

> **Fuera de v0.2.0 por tamaño:** el **Modo 5 semántico con embeddings
> on-device** (ADR-010: `candle` + CLIP, Modo 5 y búsqueda por texto) es un
> proyecto completo, no una feature de un día. Va a v0.3.0.
> El plan de ejecución está en [`LOCAL_AI.md`](LOCAL_AI.md).

---

## 4. Estimación total

| Fase | Días |
|---|---|
| A — Code signing & release | 2 |
| B — Easter egg | 1 |
| C — Tests JS (Vitest) | 2 |
| D — Toggle ocultos | 1 |
| E — Tema claro | 1 |
| F — ffmpeg-sidecar | 4 |
| G — Capturas reales | 0.5 |
| H — Plan B vídeos | 1 |
| I — Mejoras de UX | 2 |
| **Total mínimo (A+D+C+F)** | **9 días** |
| **Total completo** | **14.5 días** |

---

## 5. Riesgos y dependencias

- **Fase A** depende de tener una Apple Developer ID ($99/año)
  y/o Azure Trusted Signing. Si no, v0.2.0 se queda sin
  binarios firmados y la release sigue siendo "construye tú mismo".
- **Fase F** tiene alta variabilidad. Si ffmpeg-sidecar da
  problemas en Windows, caer a **Fase H** (heurística sin
  ffmpeg). Ambas son válidas.
- **Fase C** (tests JS) puede descubrir bugs no conocidos que
  retrasen otras fases. Si pasa, priorizar fix del bug y volver
  al test.
- **Capturas (Fase G)** se beneficia de las mejoras visuales de
  las Fases D, E, B. Hacerla al final para capturar la versión
  definitiva.

---

## 6. Cómo usar este plan

1. **Crea una rama**: `feat/v0.2-…`.
2. **Trabaja fase por fase**, en el orden que prefieras.
3. **Actualiza este `ROADMAP_v0.2.md`** con `[x]` cuando completes
   un criterio. Si una fase descubre trabajo nuevo, añádelo
   en el mismo PR.
4. **Tag `v0.2.0`** cuando todas las fases A–F estén completas.
5. **v0.3.0** = las fases E, G, H, I.

**Diferido a v0.3.0+** (no en este plan):
- i18n completo (Fase I)
- Tema claro (Fase E) — nice-to-have
- Mejoras UX misceláneas (Fase I)

---

## 7. Métricas de éxito para v0.2.0

- **Tests JS**: ≥ 60% de coverage.
- **Tests Rust**: los 11 actuales siguen pasando + 2-3 nuevos
  (Fase D, F, H).
- **Capturas reales** en README (no mock).
- **Tag v0.2.0** con binarios firmados y auto-update funcional.
- **Cero issues abiertos** en el repo de v0.2.0 (los bugs de v0.1.0
  están cerrados o reasignados).

---

## 8. Historial

- **v0.1.0** (2026-06-09): primera release jugable end-to-end.
  4 modos, thumbnails, pHash, papelera, branding mínimo.
  Sin code signing. Tag: `v0.1.0`.
- **v0.2.0** (plan actual): code signing, ffmpeg-sidecar,
  toggle ocultos, tests JS, easter egg, capturas reales.

> Para el detalle técnico de cada decisión arquitectónica, ver
> [`ARCHITECTURE.md`](ARCHITECTURE.md). Para convenciones de
> código, ver [`CONTRIBUTING.md`](CONTRIBUTING.md).
