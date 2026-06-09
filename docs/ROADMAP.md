# Roadmap

> Plan de implementación en **9 fases**. Cada fase termina en un demo
> shippable. **No se avanza de fase hasta que todos los criterios de
> "hecho" estén marcados** y el usuario los haya aprobado.
>
> Para el diseño técnico completo de cada fase, ver
> [`ARCHITECTURE.md`](ARCHITECTURE.md). Para el flujo de PR y
> convenciones, ver [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Tabla de contenidos

- [Resumen](#resumen)
- [Leyenda de estado](#leyenda-de-estado)
- [Fase 0 — Documentación](#fase-0--documentación)
- [Fase 1 — Scaffolding](#fase-1--scaffolding)
- [Fase 2 — Picker + DB + walk](#fase-2--picker--db--walk)
- [Fase 3 — Modo 1 (exacto)](#fase-3--modo-1-exacto)
- [Fase 4 — Thumbnails](#fase-4--thumbnails)
- [Fase 5 — pHash de imágenes (Modo 2)](#fase-5--phash-de-imágenes-modo-2)
- [Fase 6 — Hash de vídeos (Modo 2)](#fase-6--hash-de-vídeos-modo-2)
- [Fase 7 — Modo 3 (visual)](#fase-7--modo-3-visual)
- [Fase 8 — Final review + papelera](#fase-8--final-review--papelera)
- [Fase 9 — Polish + release](#fase-9--polish--release)

---

## Resumen

| # | Título                            | Duración   | Estado       |
|---|-----------------------------------|------------|--------------|
| 0 | Documentación                     | 0.5 d      | 🚧 en curso |
| 1 | Scaffolding                       | 1 d        | ⬜ pendiente |
| 2 | Picker + DB + walk                | 1–2 d      | ⬜ pendiente |
| 3 | Modo 1 (exacto)                   | 1 d        | ⬜ pendiente |
| 4 | Thumbnails                        | 2 d        | ⬜ pendiente |
| 5 | pHash de imágenes (Modo 2)        | 2–3 d      | ⬜ pendiente |
| 6 | Hash de vídeos (Modo 2)           | 2–3 d      | ⬜ pendiente |
| 7 | Modo 3 (visual)                   | 1–2 d      | ⬜ pendiente |
| 8 | Final review + papelera           | 1–2 d      | ⬜ pendiente |
| 9 | Polish + release                  | 2–3 d      | ⬜ pendiente |

**Total estimado**: 12.5–18 días para un dev solo con biblioteca real de
2 TB como banco de pruebas.

---

## Leyenda de estado

- ⬜ **pendiente** — no empezada.
- 🚧 **en curso** — PR abierto o trabajo en una rama.
- ✅ **hecha** — todos los criterios de "hecho" marcados, mergeado a
  `main`, demo verificada.
- ⚠️ **bloqueada** — esperando input externo (diseño, decisión, asset).

---

## Fase 0 — Documentación

**Objetivo**: dejar la documentación fundacional del proyecto commiteada y
revisada antes de escribir una sola línea de código de producto.

**Criterios de "hecho"**:

- [ ] `README.md` existe en la raíz con tagline, qué es, características,
      formatos soportados, requisitos, dev/build, licencia TBD, enlaces a
      los 3 docs de `docs/`.
- [ ] `docs/ARCHITECTURE.md` existe y cubre: layout del repo, stack Rust y
      frontend, esquema SQLite literal, pipeline de 3 etapas, algoritmo de
      los 3 modos, comandos Tauri, thumbnails, seguridad, edge cases, y
      **al menos 6 ADRs** (Tauri vs Electron, jwalk vs walkdir, img_hash
      vs perceptual-hash-rs, ffmpeg-sidecar vs ffmpeg-next, rusqlite vs
      sqlx, trash vs send2trash, rayon vs tokio, zustand vs RTK,
      react-virtuoso vs react-window).
- [ ] `docs/CONTRIBUTING.md` existe y cubre: setup, troubleshooting,
      estructura del repo, convenciones Rust/TS, commits, tests, añadir
      un crate, añadir un comando Tauri (toques `commands.rs` Y
      `lib/tauri.ts` en el mismo PR), flujo de PR, seguridad.
- [ ] `docs/ROADMAP.md` (este archivo) lista las 9 fases con objetivos,
      criterios de "hecho" en formato `[ ]`, no-objetivos, duración, y
      enlaces a la sección correspondiente de `ARCHITECTURE.md` cuando
      aplique.
- [ ] Los 4 documentos están en **español**.
- [ ] `README.md` enlaza a los 3 documentos de `docs/`.
- [ ] `LICENSE` está creado (aunque sea con texto TBD).
- [ ] El usuario ha revisado y aprobado los 4 documentos.

**No-objetivos**:

- No se escribe código de producto (Rust/TS).
- No se inicializa `pnpm create tauri-app`.
- No se añaden GitHub Actions todavía.

**Entregable**: 4 archivos `.md` + `LICENSE` commiteados en `main` (o PR
mergeado), usuario aprueba explícitamente.

**Referencia**: [`ARCHITECTURE.md`](ARCHITECTURE.md).

---

## Fase 1 — Scaffolding

**Objetivo**: tener la app Tauri 2 arrancando en dev con la ventana
"fotoverse" en el título, sin lógica de negocio.

**Criterios de "hecho"**:

- [ ] `pnpm create tauri-app` ejecutado con plantilla React + TypeScript
      v2; estructura de `src/` y `src-tauri/` coincide con la del
      `ARCHITECTURE.md`.
- [ ] `tauri.conf.json` configurado: `productName = "fotoverse"`,
      `identifier` único, `window.title = "fotoverse"`,
      `window.width/height` razonables (1280×800), español como locale.
- [ ] `tracing` + `tracing-subscriber` inicializados en `lib.rs`; logs
      visibles en `pnpm tauri dev`.
- [ ] `zustand` y `react-router-dom` instalados; `<App />` con router
      mínimo (sólo la ruta `/` muestra "fotoverse" como placeholder).
- [ ] `tsconfig.json` con `strict: true`.
- [ ] ESLint configurado (config mínima: `@typescript-eslint`).
- [ ] `.gitignore` cubre `target/`, `node_modules/`, `dist/`,
      `*.app`, `*.dmg`, `*.exe`, `*.msi`, `fotoverse.db*`.
- [ ] `.editorconfig` con `LF`, `2 spaces`, `UTF-8`.
- [ ] CI (`.github/workflows/ci.yml`) corre en cada push: `cargo check`,
      `cargo clippy -- -D warnings`, `pnpm tsc`, `pnpm lint`.
- [ ] `pnpm tauri dev` abre una ventana con el título "fotoverse".

**No-objetivos**:

- Sin lógica de scan, DB, ni FS todavía.
- Sin release pipeline (eso es Fase 9).
- Sin tests aún (se añaden por fase).

**Entregable**: ventana "fotoverse" arrancando en dev + CI verde.

**Referencia**: [`ARCHITECTURE.md#stack-tecnológico`](ARCHITECTURE.md#stack-tecnológico).

---

## Fase 2 — Picker + DB + walk

**Objetivo**: el usuario puede elegir una carpeta, la app la recorre, y
persiste todos los ficheros en SQLite con eventos de progreso al
frontend.

**Criterios de "hecho"**:

- [ ] Comando `pick_folder` abre el diálogo nativo y devuelve la ruta.
- [ ] `Cargo.toml` incluye `jwalk`, `rusqlite`, `rayon`, `tokio-util`,
      `chrono`, `normpath`, `tracing`, `thiserror`, `anyhow`.
- [ ] Migración inicial en `src-tauri/src/db/migrations.rs` crea las
      tablas de `docs/ARCHITECTURE.md` (sin `phash`/`vhash` todavía; esos
      se añaden en Fases 5–6).
- [ ] DB en `app_data_dir()/fotoverse.db` con `PRAGMA journal_mode =
      WAL`, `synchronous = NORMAL`.
- [ ] `start_scan({root, modes})` lanza el escaneo en background y
      retorna inmediatamente el `session_id`.
- [ ] `AppState` mantiene `Arc<Mutex<Option<ScanHandle>>>` con
      `CancellationToken`.
- [ ] Walk con `jwalk` (`follow_links(false)`), filtro por extensiones
      soportadas, `stat()` en paralelo con `par_bridge()`.
- [ ] Inserción en lotes de 1000 filas por transacción.
- [ ] Evento `scan_progress` con `{phase: "walking", files_seen}` emitido
      throttled a 200 ms.
- [ ] Evento `scan_completed` con resumen al terminar.
- [ ] Comando `cancel_scan({session_id})` cancela limpiamente (joins el
      handle con timeout 2 s).
- [ ] `ScanProgress.tsx` muestra contador live, fase actual, botón
      Cancelar funcional.
- [ ] Smoke test con carpeta de 1k archivos: la app termina el walk en
      <10 s en Mac M1 y los 1000 archivos están en `files`.
- [ ] CI pasa: `cargo test`, `cargo clippy -- -D warnings`.

**No-objetivos**:

- Sin hashing todavía (sólo metadatos: path, size, ctime, mtime, btime,
  ext, kind).
- Sin thumbnails.
- Sin modos de detección.
- Sin UI de revisión.

**Entregable**: app que escanea una carpeta y muestra progreso live; DB
poblada; cancelación funcional.

**Referencia**: [`ARCHITECTURE.md#pipeline-de-escaneo`](ARCHITECTURE.md#pipeline-de-escaneo),
[`#esquema-de-la-base-de-datos`](ARCHITECTURE.md#esquema-de-la-base-de-datos),
[`#comandos-tauri`](ARCHITECTURE.md#comandos-tauri).

---

## Fase 3 — Modo 1 (exacto)

**Objetivo**: detectar duplicados exactos por (filename, ext, ctime, size)
y permitir al usuario revisar los grupos y tomar decisiones.

**Criterios de "hecho"**:

- [ ] Query `GROUP BY` en `detect/exact.rs` que puebla
      `exact_groups` con `member_count > 1`.
- [ ] Comandos `get_exact_groups({session_id, offset, limit})` y
      `get_group_members({group_id, mode: "exact"})` paginados.
- [ ] Comando `set_decision({file_id, action})` persiste en `decisions`.
- [ ] Comando `bulk_set_decision({file_ids, action})` para el workflow
      "borrar todos menos el primero".
- [ ] Página `Mode1Review.tsx` con:
      - [ ] Lista virtualizada de grupos (`react-virtuoso`).
      - [ ] Cada fila muestra filename, tamaño, member count, strip de
            2–4 placeholders grises (los thumbs vienen en Fase 4).
      - [ ] Expandir una fila revela checkboxes por archivo.
      - [ ] Default: mantener el archivo con `btime_ns` más antiguo,
            marcar el resto como `delete`.
      - [ ] Botón "Marcar para borrar el resto" en cada grupo.
- [ ] `Welcome.tsx` actualizado: tras completar un escaneo, navega
      automáticamente a `/review/:id/mode1`.
- [ ] Test unitario en Rust: la query devuelve los grupos esperados
      sobre una fixture sintética.
- [ ] Smoke test: una carpeta con varios `IMG_1234.jpg` con mismo ctime
      produce los grupos esperados en `Mode1Review`.

**No-objetivos**:

- Sin thumbnails todavía.
- Sin detección perceptual.
- Sin modo visual.
- Sin borrado real.

**Entregable**: usuario puede ver grupos del Modo 1 y marcar decisiones;
éstas persisten entre recargas.

**Referencia**: [`ARCHITECTURE.md#modo-1--exacto`](ARCHITECTURE.md#modo-1--exacto).

---

## Fase 4 — Thumbnails

**Objetivo**: generar y cachear thumbnails para imágenes y vídeos, y
reemplazar los placeholders grises de la Fase 3.

**Criterios de "hecho"**:

- [ ] Módulo `src-tauri/src/thumbs/` con:
      - [ ] `mod.rs`: trait `Thumbnailer` con impls para imagen y vídeo.
      - [ ] `cache.rs`: resolución de ruta bajo
            `app_cache_dir()/thumbs/<sha256(path)>.jpg` con sharding.
- [ ] Generación durante la Etapa B (hashing) en `image_hash.rs` y
      `video_hash.rs` (aunque el hash se implemente en Fases 5–6, la
      generación de thumbs se puede hacer ya).
- [ ] Imágenes: 256 px lado mayor, JPEG q=80, desde el `DynamicImage`
      en memoria.
- [ ] Vídeos: usar el keyframe ffmpeg extraído, downscale a 256 px.
- [ ] `fingerprint` con `mtime_ns` en sidecar `.mtime`; si coincide, se
      reusa el thumb sin re-decodificar.
- [ ] Comando `get_thumbnail({file_id, size})` devuelve la ruta absoluta.
- [ ] Frontend usa `convertFileSrc(path)` en el `<img src>`.
- [ ] `react-virtuoso` montado en `Mode1Review` para virtualizar la
      lista de grupos y los strips de thumbnails.
- [ ] Smoke test: 1k imágenes → thumbs se generan en <60 s y se
      renderizan en <2 s en `Mode1Review`.

**No-objetivos**:

- Sin detección perceptual (sólo generación de thumbs).
- Sin tamaños variables por dispositivo (sólo 256 px por ahora).

**Entregable**: thumbnails reales en `Mode1Review`; sin regresiones
visuales.

**Referencia**: [`ARCHITECTURE.md#thumbnails`](ARCHITECTURE.md#thumbnails).

---

## Fase 5 — pHash de imágenes (Modo 2)

**Objetivo**: calcular pHash de las imágenes, agrupar por similitud
perceptual con BK-tree, y mostrar los clusters en `Mode2Review`.

**Criterios de "hecho"**:

- [ ] `Cargo.toml` añade `img_hash` e `image`.
- [ ] `src-tauri/src/hash/image_hash.rs` implementa el algoritmo
      documentado en
      [`ARCHITECTURE.md#hash-de-imágenes-img_hash`](ARCHITECTURE.md#hash-de-imágenes-img_hash).
- [ ] Pool rayon acotado a `min(8, ncpu)` para imágenes.
- [ ] UPDATE de la fila `files` con `phash` y `hash_status='ok'` o
      `'error'` por imagen.
- [ ] `src-tauri/src/detect/perceptual.rs` implementa BK-tree con
      Hamming ≤ 10.
- [ ] `perceptual_groups` se puebla con representante + miembros.
- [ ] Ficheros ya revisados en Modo 1 **no** se re-surfacean.
- [ ] Comandos `get_perceptual_groups(...)` y
      `get_group_members({group_id, mode: "perceptual"})` paginados.
- [ ] `Mode2Review.tsx` con:
      - [ ] Lista virtualizada de clusters.
      - [ ] Cada fila con badge "distancia Hamming: N".
      - [ ] Botón "Mantener el original, borrar las copias".
- [ ] Progreso visible: nueva fase "Calculando hashes (imágenes)" en
      `ScanProgress.tsx`.
- [ ] Test: una carpeta con la misma imagen rotada y re-comprimida a
      distinto tamaño se agrupa correctamente.
- [ ] Smoke test: 1k imágenes mixtas (algunas duplicados visuales) →
      grupos coherentes.

**No-objetivos**:

- Sin vídeos todavía.
- Sin BK-tree con Hamming variable (fijo en 10).

**Entregable**: usuario ve clusters perceptuales de imágenes y puede
marcar decisiones.

**Referencia**: [`ARCHITECTURE.md#modo-2--perceptual`](ARCHITECTURE.md#modo-2--perceptual),
[`#decisiones-arquitectónicas-adrs`](ARCHITECTURE.md#decisiones-arquitectónicas-adrs)
(ADR-003 sobre `img_hash`).

---

## Fase 6 — Hash de vídeos (Modo 2)

**Objetivo**: extraer keyframes de vídeos, hashearlos, y agrupar vídeos
duplicados con tolerancia a ruido.

**Criterios de "hecho"**:

- [ ] `Cargo.toml` añade `ffmpeg-sidecar`.
- [ ] `src-tauri/src/hash/video_hash.rs` implementa:
      - [ ] `ffmpeg -i <file>` → duración.
      - [ ] Si `<1s`: 1 frame en `t=0`.
      - [ ] Si `≥1s`: 3 frames en `t = 0.1·D, 0.5·D, 0.9·D`.
      - [ ] pHash de cada frame.
      - [ ] `vhash` (8 bytes) = primer frame; `vhash_extra` (16 bytes) =
            2º + 3er.
- [ ] Pool rayon de 4 hilos con `Semaphore` para ffmpeg.
- [ ] Si ffmpeg falla: `hash_status='error'`, `hash_error=<msg>`, sigue.
- [ ] `perceptual_groups` clustering actualizado: vídeos coinciden si
      `vhash` ≤ 12 **o** `vhash_extra` ≤ 12.
- [ ] Validado en **macOS** y **Windows** (binarios ffmpeg distintos).
- [ ] Progreso visible: nueva fase "Calculando hashes (vídeos)" en
      `ScanProgress.tsx`.
- [ ] Smoke test: dos MP4 cortos con el mismo contenido pero bitrate
      distinto se agrupan.
- [ ] Bundle de release aumenta ~80 MB por el ffmpeg sidecar (verificar
      en CI que el tamaño es razonable).

**No-objetivos**:

- Sin streaming de vídeo en la UI.
- Sin selección de keyframe manual (siempre 3 automáticos).

**Entregable**: vídeos duplicados se detectan y agrupan correctamente
en ambas plataformas.

**Referencia**: [`ARCHITECTURE.md#hash-de-vídeos-ffmpeg-sidecar`](ARCHITECTURE.md#hash-de-vídeos-ffmpeg-sidecar),
ADR-004 sobre `ffmpeg-sidecar`.

---

## Fase 7 — Modo 3 (visual)

**Objetivo**: comparativa lado a lado para casos donde los algoritmos no
son concluyentes.

**Criterios de "hecho"**:

- [ ] `src-tauri/src/detect/manual.rs` puebla `manual_pairs` con todos
      los pares no decididos de los clusters de Modo 2.
- [ ] Comandos `get_manual_pairs({session_id, offset, limit, filter})`
      con `filter: "pending" | "all"`.
- [ ] `Mode3Review.tsx` con:
      - [ ] `<SideBySide>` 600×600 izda/dcha, con thumbs y metadata
            (path, size, dimensions, fecha) bajo cada uno.
      - [ ] 4 botones: "Borrar izquierda", "Borrar derecha",
            "Borrar ambas", "Mantener ambas".
      - [ ] Botón "Saltar" para posponer la decisión.
      - [ ] Atajos de teclado: `←` borrar izda, `→` borrar dcha,
            `↓` saltar, `space` mantener ambas.
      - [ ] Barra de progreso "127 / 412 pares".
- [ ] Pares con ambos miembros ya decididos (`keep`/`delete`) **no** se
      muestran.
- [ ] Test E2E: usuario puede tachar 100 pares en <1 min con el
      teclado.
- [ ] Smoke test: 50 pares sintéticos, decisiones se persisten
      correctamente.

**No-objetivos**:

- Sin zoom/pan en imágenes.
- Sin comparación de vídeo (sólo imágenes en este modo).

**Entregable**: usuario resuelve manualmente los casos dudosos con
atajos de teclado.

**Referencia**: [`ARCHITECTURE.md#modo-3--visual-manual`](ARCHITECTURE.md#modo-3--visual-manual).

---

## Fase 8 — Final review + papelera

**Objetivo**: vista unificada de todos los ficheros marcados para borrar,
confirmación explícita, y envío a la papelera.

**Criterios de "hecho"**:

- [ ] `Cargo.toml` añade `trash`.
- [ ] Comando `get_final_review({session_id})` devuelve todos los
      ficheros actualmente `delete`.
- [ ] Comando `execute_deletions({session_id})` invoca
      `trash::delete_all` y reporta `(path, error)` por fallo.
- [ ] Comando `open_path({path})` revela en Finder/Explorer.
- [ ] `FinalReview.tsx` con:
      - [ ] Lista única ordenada por path/grupo.
      - [ ] Cada fila: thumbnail, path, tamaño, contexto de grupo,
            **checkbox** (para desmarcar en el último momento).
      - [ ] Header con totales: "1 247 archivos · 8.4 GB".
      - [ ] Botón "Enviar a la papelera" abre modal de confirmación
            tipada si > 1000 archivos o > 5 GB (escribir "ELIMINAR").
      - [ ] Tras ejecutar: reporte de éxito + fallos.
- [ ] Página `Done.tsx` con resumen, espacio liberado, "Ver en
      Finder/Explorador", y "Empezar otro escaneo".
- [ ] Verificación: `grep -rn 'remove_file\|remove_dir' src-tauri/src/`
      devuelve **sólo** fixtures de tests (CI lo enforça).
- [ ] Smoke test con biblioteca real de 2 TB del usuario: 20k archivos
      → set final coherente → ejecución → todos los archivos en la
      papelera (Finder / Recycle Bin), **cero** borrados permanentes.
- [ ] Test que simula fallo de `trash` para un archivo y verifica el
      reporte de error.

**No-objetivos**:

- Sin opción de "borrar permanentemente" (decisión de producto: nunca).
- Sin deshacer (la papelera del sistema ya lo permite).

**Entregable**: usuario envía a la papelera con confianza, ve reporte
claro, y puede deshacer desde Finder/Explorer.

**Referencia**: [`ARCHITECTURE.md#seguridad`](ARCHITECTURE.md#seguridad),
[`#páginas-de-la-ui`](ARCHITECTURE.md#páginas-de-la-ui).

---

## Fase 9 — Polish + release

**Objetivo**: endurecer la app, empaquetar para Mac + Windows con firma y
auto-update, y dejar la pipeline de release lista.

**Criterios de "hecho"**:

- [ ] Cancelación robusta probada en todas las fases (walk, hash,
      cluster).
- [ ] Estados de carga (skeletons) en todas las páginas.
- [ ] Mensajes de error user-friendly en español.
- [ ] Manejo de DB corrupta: `PRAGMA integrity_check` al arranque;
      ofrecer descartar.
- [ ] Manejo de sesión > 1 GB: prompt para archivar; `VACUUM` al
      borrar sesión.
- [ ] Settings (toggle "incluir archivos ocultos", última carpeta
      usada) persistidos con `tauri-plugin-store`.
- [ ] `.github/workflows/release.yml` en tags `v*`:
      - [ ] **macOS**: build universal (aarch64 + x86_64), firma con
            Developer ID, notarización con `xcrun notarytool`, staple.
            Sube `.dmg` y `.app.zip` a GitHub Release.
      - [ ] **Windows**: build NSIS, firma con Azure Trusted Signing.
            Sube `.exe` y `.msi`.
- [ ] Code signing configurado: secrets de GitHub para Apple Developer
      ID, App Store Connect API key, Azure Trusted Signing.
- [ ] `tauri-plugin-updater` wired: `latest.json` generado por
      `tauri-action`; Welcome detecta y muestra "Hay una nueva versión
      disponible".
- [ ] Tag `v0.1.0` → release automático con binarios firmados en GitHub
      Releases.
- [ ] Smoke test corre en CI en `macos-latest` y `windows-latest` en
      cada push.
- [ ] README actualizado con badges de CI y release.

**No-objetivos**:

- Sin publicación en App Store / Microsoft Store.
- Sin telemetría.
- Sin planes / pricing (es OSS).

**Entregable**: `v0.1.0` publicado en GitHub Releases con binarios
firmados y auto-update funcional.

**Referencia**: [`ARCHITECTURE.md#stack-tecnológico`](ARCHITECTURE.md#stack-tecnológico)
(sección plugins), [`CONTRIBUTING.md`](CONTRIBUTING.md) (troubleshooting
de Windows y macOS).

---

## Cómo usar este roadmap

1. **Empieza una fase** creando la rama (`feat/fase-N-…`).
2. **Avanza los criterios de "hecho"** marcándolos con `[x]` en este
   mismo archivo (en el PR).
3. **Cuando todos estén marcados**, abre un PR de "Fase N completa" con
   capturas / GIFs de la UI.
4. **Pide review**. Si el revisor aprueba, se mergea a `main`, se
   actualiza la tabla de resumen y se pasa a la siguiente fase.

Si una fase descubre trabajo no contemplado, **se actualiza este roadmap
en el mismo PR** (no se hace work silencioso fuera de scope).

## Releases

- **v0.1.0** (2026-06-09): primera release jugable. Tag `v0.1.0`.
  4 modos, thumbnails, pHash, papelera. Sin code signing.
- **v0.2.0** (en planning): ver [`ROADMAP_v0.2.md`](ROADMAP_v0.2.md).
  Fases A–I: code signing, ffmpeg-sidecar, tests JS, toggle
  ocultos, easter egg, tema claro, capturas reales, plan B de
  vídeos, mejoras de UX. Estimado: 9–14 días.
