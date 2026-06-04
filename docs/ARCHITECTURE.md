# Arquitectura de fotoverse

> Diseño técnico del proyecto. Este documento es la versión permanente y
> narrativa del plan de implementación. Si algo aquí contradice el código,
> gana el código — abre un PR para actualizar este archivo.

## Tabla de contenidos

1. [Visión general](#visión-general)
2. [Layout del repositorio](#layout-del-repositorio)
3. [Stack tecnológico](#stack-tecnológico)
4. [Esquema de la base de datos](#esquema-de-la-base-de-datos)
5. [Pipeline de escaneo](#pipeline-de-escaneo)
6. [Algoritmo de los tres modos](#algoritmo-de-los-tres-modos)
7. [Comandos Tauri](#comandos-tauri)
8. [Thumbnails](#thumbnails)
9. [Páginas de la UI](#páginas-de-la-ui)
10. [Seguridad](#seguridad)
11. [Edge cases](#edge-cases)
12. [Decisiones arquitectónicas (ADRs)](#decisiones-arquitectónicas-adrs)

---

## Visión general

fotoverse es una aplicación de escritorio **multiplataforma (macOS + Windows)**
construida con **Tauri 2**: un binario nativo de **Rust** en el backend
(escaneo, hashing, base de datos) y un frontend web en **React + TypeScript**
(la UI visual de revisión). La comunicación entre ambos se hace por comandos
Tauri (`invoke`/`emit`).

La aplicación **no habla con la red**: todo el procesamiento es local y los
ficheros sólo se mueven a la Papelera del sistema.

### Por qué este enfoque

- **Rendimiento de hashing**: el perceptual hash de imágenes y la extracción
  de keyframes de vídeo son CPU-bound. Rust con `rayon` aprovecha todos los
  núcleos. Node/Electron sufriría mucho con decenas de miles de archivos.
- **Binarios pequeños y multiplataforma**: Tauri usa el webview del sistema
  (WebKit en macOS, WebView2 en Windows), así que el bundle final es de
  ~10 MB en lugar de los ~150 MB de Electron.
- **Borrado seguro multiplataforma**: el crate `trash` de Rust envía a la
  Papelera de macOS o al Recycle Bin de Windows con la misma API.

---

## Layout del repositorio

```
fotoverse/
├── .github/
│   └── workflows/
│       ├── ci.yml                  # lint + test en cada push
│       └── release.yml             # builds firmados para Mac + Windows
├── docs/
│   ├── ARCHITECTURE.md             # este archivo
│   ├── CONTRIBUTING.md             # guía para devs
│   └── ROADMAP.md                  # fases con criterios de "hecho"
├── src-tauri/                      # Backend Rust
│   ├── Cargo.toml                  # manifiesto de dependencias Rust
│   ├── tauri.conf.json             # ventana, capabilities, bundle
│   ├── build.rs
│   ├── icons/                      # iconos de la app
│   ├── capabilities/
│   │   └── default.json            # permisos Tauri (fs, dialog, shell…)
│   └── src/
│       ├── main.rs                 # entry point
│       ├── lib.rs                  # tauri::Builder + plugins + comandos
│       ├── state.rs                # AppState: handles de escaneo
│       ├── cancel.rs               # CancellationToken
│       ├── commands.rs             # #[tauri::command] (ver §7)
│       ├── db/
│       │   ├── mod.rs
│       │   ├── migrations.rs       # SQL embebido al arranque
│       │   └── schema.sql          # el esquema de la §4
│       ├── walk/
│       │   ├── mod.rs              # wrapper de jwalk
│       │   └── progress.rs         # emisión de scan_progress
│       ├── hash/
│       │   ├── mod.rs              # trait Hash + dispatch imagen/vídeo
│       │   ├── image_hash.rs       # pHash vía img_hash
│       │   └── video_hash.rs       # keyframes vía ffmpeg-sidecar
│       ├── detect/
│       │   ├── mod.rs
│       │   ├── exact.rs            # Modo 1 (SQL GROUP BY)
│       │   ├── perceptual.rs       # Modo 2 (BK-tree)
│       │   └── manual.rs           # Modo 3 (pares para revisión)
│       ├── review/
│       │   ├── mod.rs
│       │   └── trash.rs            # execute_deletions
│       └── thumbs/
│           ├── mod.rs              # generación de thumbnails
│           └── cache.rs            # resolución de rutas en disco
├── src/                            # Frontend React + TypeScript
│   ├── main.tsx
│   ├── App.tsx                     # router
│   ├── i18n/
│   │   └── es.json                 # todos los strings en español
│   ├── pages/
│   │   ├── Welcome.tsx
│   │   ├── ScanProgress.tsx
│   │   ├── Mode1Review.tsx
│   │   ├── Mode2Review.tsx
│   │   ├── Mode3Review.tsx
│   │   ├── FinalReview.tsx
│   │   └── Done.tsx
│   ├── components/
│   │   ├── Thumbnail.tsx
│   │   ├── DuplicateCard.tsx
│   │   ├── SideBySide.tsx
│   │   └── DecisionBar.tsx
│   ├── hooks/
│   │   ├── useScanProgress.ts
│   │   └── useDecisions.ts
│   ├── lib/
│   │   ├── tauri.ts                # wrappers tipados de invoke/listen
│   │   └── types.ts                # DTOs espejo de Rust
│   └── styles/
│       └── global.css
├── scripts/
│   └── smoke.sh                    # smoke test con fixture sintético
├── package.json
├── pnpm-lock.yaml
├── vite.config.ts
├── tsconfig.json
├── .gitignore
├── .editorconfig
├── LICENSE
└── README.md
```

---

## Stack tecnológico

### Backend Rust (`src-tauri/Cargo.toml`)

| Necesidad              | Crate                                                  | Por qué                                                                                            |
|------------------------|--------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| Runtime Tauri          | `tauri = "2"`, `tauri-build = "2"`                     | Tauri 2.x: modelo de plugins estable, mejor tooling.                                                |
| Recorrido de FS        | `jwalk`                                                | Walker paralelo, 5–10× más rápido que `walkdir` con 50k+ archivos en SSD.                          |
| pHash de imágenes      | `img_hash = "0.5"`                                     | pHash/dHash/aHash/MarrHildreth en Rust puro, sin dependencias nativas.                              |
| Lectura EXIF           | `kamadak-exif = "0.6"`                                 | Estándar de facto en Rust para EXIF.                                                               |
| Decodificar imágenes   | `image = "0.25"`                                       | Decodifica JPG/PNG/WebP/GIF/BMP/TIFF al `DynamicImage` necesario por `img_hash`.                   |
| Keyframes de vídeo     | `ffmpeg-sidecar = "3"`                                 | Empaqueta el binario ffmpeg; evita linker libav* en Windows.                                       |
| SQLite                 | `rusqlite = { version = "0.32", features = ["bundled"] }` | Síncrono, libsqlite embebido. Tauri ya corre en un blocking pool; async sería overhead.         |
| Mover a papelera       | `trash = "5"`                                          | Cross-platform, puro Rust, va a Finder/Recycle Bin.                                                |
| Paralelismo CPU        | `rayon = "1"`                                          | Work-stealing ideal para el bucle de hashing CPU-bound.                                            |
| Cancelación            | `tokio-util` con feature `sync` (CancellationToken)    | API limpia, propagación anidada, integración con Tauri runtime.                                     |
| Normalización Unicode  | `normpath = "1"`                                       | NFC + canonicalización; evita trampas de HFS+ en macOS.                                            |
| Tiempo                 | `chrono = "0.4"`                                       | Manejo de ctime/mtime consciente de timezones.                                                     |
| Logging                | `tracing`, `tracing-subscriber`                        | Logs estructurados, a stderr en dev, a fichero en producción.                                      |
| Errores                | `thiserror = "1"`, `anyhow = "1"`                      | Errores idiomáticos en Rust.                                                                       |
| MIME / magic bytes     | `infer = "0.16"`                                       | Sanity-check de extensiones por magic bytes.                                                       |

### Plugins Tauri 2

- **`tauri-plugin-dialog`** — selector nativo de carpetas.
- **`tauri-plugin-fs`** — acceso de lectura con scope a la carpeta elegida.
- **`tauri-plugin-shell`** — invocar el sidecar de ffmpeg.
- **`tauri-plugin-store`** — persistir ajustes (última carpeta usada, tema).
- **`tauri-plugin-updater`** — auto-update desde GitHub Releases.

### Frontend (`package.json`)

| Necesidad              | Paquete                                          | Por qué                                                                          |
|------------------------|--------------------------------------------------|----------------------------------------------------------------------------------|
| UI framework           | `react`, `react-dom`                             | Ecosistema maduro, fácil de contratar ayuda.                                     |
| Routing                | `react-router-dom`                               | Suficiente para el flujo lineal de pantallas.                                    |
| Bundler                | `vite`                                           | Rápido, HMR instantáneo, soporte nativo de TS.                                   |
| Lenguaje               | `typescript` (con `strict: true`)                | Tipado estricto.                                                                 |
| API de Tauri           | `@tauri-apps/api` v2 + plugins oficiales         | SDK oficial; sin librerías de terceros.                                          |
| Estado global          | `zustand`                                        | ~1 KB, sin boilerplate, suficiente para una app de 5–7 pantallas.                |
| Listas virtualizadas   | `react-virtuoso`                                 | Soporta ítems de altura variable (thumbnails), IntersectionObserver integrado.  |
| Utilidades CSS         | `clsx`                                           | Concatenación condicional de clases.                                             |

**Decisión explícita**: **sin** framework CSS (Tailwind, MUI, etc.). Se usa
CSS vanilla con variables CSS para tokens (colores, espaciados). Esto
minimiza el bundle y da control total sobre el look & feel.

---

## Esquema de la base de datos

La base de datos es un único fichero SQLite en `app_data_dir()/fotoverse.db`:

- macOS: `~/Library/Application Support/fotoverse/fotoverse.db`
- Windows: `%APPDATA%\fotoverse\fotoverse.db`

Configuración: `PRAGMA journal_mode = WAL`, `PRAGMA synchronous = NORMAL`.

### Tablas

```sql
-- Una fila por sesión de escaneo iniciada
CREATE TABLE scan_sessions (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  root_path     TEXT    NOT NULL,
  modes         TEXT    NOT NULL,           -- JSON: ["exact","perceptual","manual"]
  status        TEXT    NOT NULL,           -- running | completed | cancelled | failed
  total_files   INTEGER NOT NULL DEFAULT 0,
  hashed_files  INTEGER NOT NULL DEFAULT 0,
  started_at    TEXT    NOT NULL,
  finished_at   TEXT
);

-- Una fila por fichero descubierto
CREATE TABLE files (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id    INTEGER NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
  path          TEXT    NOT NULL,
  filename      TEXT    NOT NULL,           -- basename case-folded
  ext           TEXT    NOT NULL,           -- lowercase sin punto
  size_bytes    INTEGER NOT NULL,
  ctime_ns      INTEGER,                    -- Windows: creation; Unix: inode change
  btime_ns      INTEGER,                    -- birth time, best-effort
  mtime_ns      INTEGER NOT NULL,
  kind          TEXT    NOT NULL,           -- 'image' | 'video'
  width         INTEGER,
  height        INTEGER,
  duration_ms   INTEGER,
  phash         BLOB,                       -- 8 bytes pHash (imágenes)
  vhash         BLOB,                       -- 8 bytes pHash del keyframe dominante (vídeos)
  vhash_extra   BLOB,                       -- 16 bytes = 2 pHash de keyframes secundarios
  hash_status   TEXT    NOT NULL DEFAULT 'pending', -- pending | ok | error
  hash_error    TEXT
);

CREATE INDEX idx_files_session ON files(session_id);
CREATE INDEX idx_files_exact   ON files(session_id, filename, ext, ctime_ns, size_bytes);
CREATE INDEX idx_files_phash   ON files(session_id, phash) WHERE phash IS NOT NULL;
CREATE INDEX idx_files_vhash   ON files(session_id, vhash) WHERE vhash IS NOT NULL;

-- Modo 1: cabeceras de grupo por (filename, ext, ctime, size)
CREATE TABLE exact_groups (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id      INTEGER NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
  filename        TEXT    NOT NULL,
  ext             TEXT    NOT NULL,
  ctime_ns        INTEGER NOT NULL,
  size_bytes      INTEGER NOT NULL,
  member_count    INTEGER NOT NULL,
  reviewed        INTEGER NOT NULL DEFAULT 0
);

-- Modo 2: miembros de clusters perceptuales
CREATE TABLE perceptual_groups (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id      INTEGER NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
  representative  INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  hamming_to_rep  INTEGER NOT NULL,
  reviewed        INTEGER NOT NULL DEFAULT 0
);

-- Modo 3: pares a revisar manualmente
CREATE TABLE manual_pairs (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id      INTEGER NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
  file_a          INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  file_b          INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  reviewed        INTEGER NOT NULL DEFAULT 0
);

-- Decisión del usuario por fichero
CREATE TABLE decisions (
  file_id         INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
  action          TEXT    NOT NULL,         -- 'delete' | 'keep' | 'undecided'
  decided_at      TEXT    NOT NULL
);
```

### Notas sobre columnas no triviales

- **`filename` (case-folded)** se usa para agrupar en Modo 1. El `path` conserva
  la capitalización original para mostrar al usuario.
- **`ctime_ns` vs `btime_ns`**: `ctime_ns` es "creation time" en Windows y
  "inode change time" en Unix (semántica distinta). `btime_ns` es "birth time"
  (cuándo se creó el fichero) y se obtiene con `statx`/`GetFileAttributesEx`
  (best-effort; puede ser `NULL` en algunos FS). Modo 1 usa `ctime_ns`; la UI
  muestra `btime_ns` si está disponible, si no `ctime_ns`.
- **`phash` y `vhash` son 8 bytes** (64 bits). `vhash_extra` son 16 bytes
  (2 hashes adicionales para keyframes secundarios de vídeo).
- **`hash_status = 'error'`** indica que el hash no se pudo calcular
  (permiso denegado, formato corrupto, etc.). El fichero se incluye en la
  revisión final con un icono de aviso.

### Cómo se reconstruyen los grupos

La membresía completa de cada grupo se calcula por JOIN en tiempo de
consulta (las cardinalidades son bajas). Ejemplo para Modo 1:

```sql
SELECT f.* FROM files f
JOIN exact_groups g
  ON g.session_id = f.session_id
 AND g.filename   = f.filename
 AND g.ext        = f.ext
 AND g.ctime_ns   = f.ctime_ns
 AND g.size_bytes = f.size_bytes
WHERE g.id = ?;
```

---

## Pipeline de escaneo

El escaneo se ejecuta como **un único task en background** que emite eventos
de progreso al frontend. Tiene tres etapas secuenciales.

### Etapa A — Walk (jwalk, I/O paralelo)

```rust
let entries: Vec<FileMeta> = jwalk::WalkDir::new(root)
    .follow_links(false)
    .into_iter()
    .filter_map(Result::ok)
    .filter(|e| e.file_type().is_file())
    .filter(|e| is_supported_ext(&e.path()))
    .par_bridge()                    // rayon: stat() en paralelo
    .map(|e| FileMeta::from(&e))
    .collect();
```

- `follow_links(false)` evita ciclos en symlinks.
- Inserción en SQLite en lotes de **1000 filas** dentro de una transacción.
- Emite evento `scan_progress` con `{phase: "walking", files_seen}`.

### Etapa B — Hash (rayon, pool acotado)

Dos pools separados:

- **Imágenes**: pool de `min(8, ncpu)` hilos. Decode → resize 32×32 → DCT
  32×32 → top-left 8×8 → mediana → 64 bits. Coste típico: ~10 ms/imagen
  (CPU-bound).
- **Vídeos**: pool de **4 hilos** con `Semaphore` (ffmpeg es process-heavy;
  50k ffmpegs en paralelo OOMea la máquina). Extrae 3 keyframes, hashea
  cada uno, concatena en `vhash` + `vhash_extra`.

Cada worker hace `UPDATE` de su fila en SQLite al terminar (WAL serializa
bien los writers). El progreso se emite throttled a **cada 200 ms** para no
ahogar el IPC.

La cancelación se comprueba por fichero (`CancellationToken::is_cancelled`).

### Etapa C — Cluster (secuencial, sub-segundo para 50k)

- Construye un **BK-tree** sobre todos los hashes en O(n log n).
- Para cada hash, consulta vecinos dentro de Hamming ≤ 10 (imágenes) o ≤ 12
  (vídeos). Coste: O(log n) por consulta.
- Inserta los clusters en `perceptual_groups`. El representante de cada
  cluster es el miembro con menor `id`.
- Emite `phase: "clustering"`.

### Diagrama de flujo

```
[Welcome] --(pick_folder)--> [ScanProgress]
                                     |
                                     v
                              ┌──────────────┐
                              │  A. Walk     │  jwalk paralelo
                              │  (insertar)  │  batch 1000
                              └──────┬───────┘
                                     v
                              ┌──────────────┐
                              │  B. Hash     │  rayon (2 pools)
                              │  (imgs+vid)  │  UPDATE por fila
                              └──────┬───────┘
                                     v
                              ┌──────────────┐
                              │ C. Cluster   │  BK-tree O(n log n)
                              │ (insertar)   │  perceptual_groups
                              └──────┬───────┘
                                     v
                              [scan_completed]
                                     |
                                     v
                                [Mode1Review]
                                     |
                                     v
                                [Mode2Review]
                                     |
                                     v
                                [Mode3Review]
                                     |
                                     v
                                [FinalReview]
                                     |
                                     v
                                  [Done]
```

---

## Algoritmo de los tres modos

### Modo 1 — Exacto

```sql
SELECT filename, ext, ctime_ns, size_bytes, COUNT(*) AS n
FROM files
WHERE session_id = ? AND kind IN ('image','video')
GROUP BY filename, ext, ctime_ns, size_bytes
HAVING n > 1;
```

**Regla de agrupación**: basename (case-folded) + extensión (lowercased) +
ctimeNs (bucket a segundo) + tamaño en bytes. El tamaño es un **pre-filtro
barato** que evita colisiones por nombre coincidente.

**Por qué no comparar contenido byte a byte (MD5)?** En 2 TB el coste de I/O
es el mismo que leer el fichero para hacer pHash; y el Modo 2 ya cubre los
casos "parece la misma imagen pero los bytes difieren". El Modo 1 se queda
como **búsqueda instantánea por metadatos**.

### Modo 2 — Perceptual

#### Hash de imágenes (`img_hash`)

```
1. decode → RGB8
2. to_luma()
3. resize to 32×32 (bilinear)
4. DCT 32×32
5. top-left 8×8 (excl. DC component)
6. median of 64 values
7. bit i = 1 iff value > median
8. pack 64 bits → 8 bytes
```

#### Hash de vídeos (`ffmpeg-sidecar`)

```
1. ffmpeg -i <file>  →  duration_seconds
2. if duration < 1s:  extract 1 frame at t=0
   else:              extract 3 frames at t = 0.1·D, 0.5·D, 0.9·D
   command: ffmpeg -ss <t> -i <file> -frames:v 1 \
                   -vf scale=320:-1 -f image2pipe -c:v mjpeg -
3. decode each JPEG → pHash
4. first frame → vhash
   2nd + 3rd frames → vhash_extra (16 bytes)
5. if ffmpeg errors:  hash_status='error', hash_error=<msg>, continue
```

#### Clustering

- **BK-tree** sobre el set de hashes.
- **Umbral Hamming ≤ 10** para imágenes.
- **Umbral Hamming ≤ 12** para vídeos (más ruidosos).
- Vídeos: dos hashes coinciden si `vhash` ≤ 12 **o** `vhash_extra` ≤ 12
  (maneja "tráiler/créditos que cambian").
- Representante = miembro con menor `id` del cluster.
- Ficheros ya revisados en Modo 1 **no se re-surfacean** en Modo 2.

### Modo 3 — Visual (manual)

- Para cada cluster de Modo 2 con `member_count >= 2`, generar todos los
  pares no ordenados `(a, b)` y guardar en `manual_pairs`.
- El frontend los recorre **secuencialmente**, saltando los pares donde ambos
  miembros ya tienen decisión `keep` o `delete`.
- Atajos de teclado: `←` borrar izda, `→` borrar dcha, `↓` saltar,
  `space` mantener ambas.
- El usuario también puede inyectar pares ad-hoc ("comparar con…").

---

## Comandos Tauri

Estos son los `#[tauri::command]` expuestos al frontend. Cada uno retorna
`Result<T, AppError>` (serializado como string al frontend).

| Comando                | Argumentos                                                  | Retorno                                       | Propósito                                                                 |
|------------------------|-------------------------------------------------------------|-----------------------------------------------|---------------------------------------------------------------------------|
| `pick_folder`          | —                                                           | `Option<String>`                              | Abre el diálogo nativo de selección de carpeta.                          |
| `start_scan`           | `{ root, modes: ["exact","perceptual","manual"] }`          | `ScanSession`                                 | Lanza el escaneo en background; retorna inmediatamente.                  |
| `cancel_scan`          | `{ session_id }`                                            | `()`                                          | Marca la cancelación y espera al join con timeout 2 s.                    |
| `get_scan_status`      | `{ session_id }`                                            | `ScanStatus`                                  | Sondeo síncrono (fallback al evento).                                    |
| `list_sessions`        | —                                                           | `ScanSession[]`                               | Lista de escaneos anteriores (para reanudar en Welcome).                 |
| `delete_session`       | `{ session_id }`                                            | `()`                                          | Borra la sesión y sus thumbnails.                                        |
| `get_exact_groups`     | `{ session_id, offset, limit }`                            | `{ groups, total }`                           | Lista paginada de grupos del Modo 1.                                     |
| `get_perceptual_groups`| `{ session_id, offset, limit }`                            | `{ groups, total }`                           | Lista paginada de grupos del Modo 2.                                     |
| `get_manual_pairs`     | `{ session_id, offset, limit, filter }`                     | `{ pairs, total }`                            | Pares pendientes del Modo 3.                                             |
| `get_group_members`    | `{ group_id, mode }`                                        | `File[]`                                      | Expande los miembros de un grupo.                                        |
| `set_decision`         | `{ file_id, action }`                                       | `()`                                          | Persiste `keep` / `delete` / `undecided` para un fichero.                |
| `bulk_set_decision`    | `{ file_ids, action }`                                      | `()`                                          | Marca muchas decisiones a la vez (workflow "borrar el resto").          |
| `get_thumbnail`        | `{ file_id, size }`                                         | `String` (ruta absoluta)                      | Devuelve la ruta del thumbnail cacheado.                                 |
| `get_final_review`     | `{ session_id }`                                            | `File[]`                                      | Todos los ficheros actualmente marcados `delete`.                        |
| `execute_deletions`    | `{ session_id }`                                            | `{ trashed, failed: [(path, error)] }`        | Mueve a la papelera vía `trash::delete_all`.                             |
| `open_path`            | `{ path }`                                                  | `()`                                          | Revela en Finder/Explorer (vía `tauri-plugin-shell`).                    |

### Eventos backend → frontend

| Evento             | Payload                                       | Cuándo se emite                                       |
|--------------------|-----------------------------------------------|-------------------------------------------------------|
| `scan_progress`    | `{ session_id, phase, files_seen, current? }` | Throttled a 200 ms durante walk/hash/cluster.         |
| `scan_completed`   | `{ session_id, summary }`                     | Al terminar exitosamente.                             |
| `scan_cancelled`   | `{ session_id }`                              | Tras `cancel_scan` o al pulsar Cancelar en la UI.     |
| `scan_error`       | `{ session_id, message }`                     | Si una etapa falla de forma irrecuperable.            |

### Contrato TS

El frontend **nunca** invoca `invoke` directamente. Usa los wrappers tipados
en `src/lib/tauri.ts`, que son la única fuente de verdad para el contrato
Rust ↔ TypeScript. Si añades un comando Rust, **debes** añadir su wrapper
en el mismo PR.

---

## Thumbnails

### Generación

Se generan **durante la Etapa B** (hashing), aprovechando el fichero ya
abierto:

- **Imágenes**: desde el `DynamicImage` en memoria, downscale a 256 px lado
  mayor, JPEG calidad 80. Cero I/O adicional.
- **Vídeos**: reuso del keyframe ffmpeg ya extraído, downscale a 256 px.

### Cache

Path: `app_cache_dir()/thumbs/<sha256(path)>.jpg` con sharding por 2
primeros hex chars:

```
~/Library/Caches/fotoverse/thumbs/ab/abcd…ef.jpg
%LOCALAPPDATA%\fotoverse\thumbs\ab\abcd…ef.jpg
```

`sha256` se calcula sobre el path **absoluto, normalizado a NFC y
case-folded** (para manejar case-insensitive filesystems en macOS/Windows).

Se almacena `mtime_ns` en un sidecar (`.mtime` junto al `.jpg`) para
invalidación: si al regenerar el `mtime_ns` del fichero origen coincide, se
reusa el thumb sin re-decodificar.

La DB **no** almacena los bytes del thumbnail, sólo su path en disco. Esto
mantiene la DB compacta y los checkpoints de WAL rápidos.

### Carga en el frontend

- `get_thumbnail(file_id, size)` devuelve la ruta absoluta.
- `<img src={convertFileSrc(path)} />` (Tauri 2 sirve `file://` URLs al
  webview saltándose la política CORS).
- `react-virtuoso` virtualiza la lista: sólo monta los thumbs visibles.
- Placeholder gris mientras carga el thumb.

---

## Páginas de la UI

Todas las cadenas de texto viven en `src/i18n/es.json` (no hardcoded en
componentes).

| Página              | Ruta                | Función                                                                                              |
|---------------------|---------------------|------------------------------------------------------------------------------------------------------|
| `Welcome`           | `/`                 | Hero, "Elegir carpeta…", lista de escaneos anteriores para reanudar, settings.                       |
| `ScanProgress`      | `/scan/:id`         | Barra de progreso, fase actual, contador, path en monospace, botón Cancelar.                         |
| `Mode1Review`       | `/review/:id/mode1` | Lista de grupos con strip de thumbnails, expandible, default "mantener el más antiguo, borrar resto". |
| `Mode2Review`       | `/review/:id/mode2` | Igual que Modo 1, con badge "distancia Hamming: N", orden por tamaño.                                 |
| `Mode3Review`       | `/review/:id/mode3` | `<SideBySide>` 600×600 con metadata, 4 botones + atajos `← → ↓ space`.                                |
| `FinalReview`       | `/review/:id/final` | Lista única de marcados `delete` con checkboxes, header "N archivos · M GB", modal de confirmación. |
| `Done`              | `/done`             | Resumen de éxito, espacio liberado, "Ver en Finder/Explorador", "Empezar otro".                     |

### Internacionalización

Por ahora **sólo español** (decisión de producto). El sistema de i18n es
deliberadamente simple: un único `es.json` importado en runtime. Si en el
futuro se añade inglés, se introduce `react-i18next` en ese momento.

---

## Seguridad

La promesa de producto es **"nunca se borra nada permanentemente"**. Esto
está reforzado por código, tests y checklist de PR.

1. **Toda eliminación va por `trash::delete_all`**. No existe ningún
   `std::fs::remove_file` ni `remove_dir` en `src-tauri/src/` fuera de los
   fixtures de tests. Esto se verifica con un grep en CI:
   ```bash
   ! grep -rn 'remove_file\|remove_dir' src-tauri/src/ | grep -v test
   ```
2. **Final Review obligatorio** antes de ejecutar. Muestra resumen
   acumulativo de archivos y bytes a liberar.
3. **Modal de confirmación tipada** ("Escribe ELIMINAR para continuar") si
   el set excede 1000 archivos o 5 GB.
4. **`execute_deletions` reporta fallos por archivo** como tuplas
   `(path, error)`. El usuario decide reintentar los fallidos.
5. **Sin acceso a red**: el capability block de Tauri deshabilita `http` y
   shells salientes. La app es 100% offline.
6. **WAL en SQLite**: un corte de luz durante un escaneo deja la sesión
   anterior intacta; el escaneo parcial queda marcado como `failed` y puede
   borrarse.
7. **Path normalization (NFC)** evita ataques de path traversal vía
   caracteres Unicode confusables.
8. **Scope FS**: `tauri-plugin-fs` se configura con scope a la carpeta
   elegida por el dialog. No se puede leer fuera de ella aunque un path
   venga manipulado.

---

## Edge cases

| Caso                    | Tratamiento                                                                                                |
|-------------------------|-----------------------------------------------------------------------------------------------------------|
| **Symlinks**            | `follow_links(false)`. Contador "12 symlinks omitidos" visible en la UI.                                   |
| **Hidden files**        | Por defecto **incluidos** (algunos guardan fotos en `.photoslibrary`). Toggle en Welcome.                  |
| **Network drives**      | Detección al inicio del escaneo; aviso "Esto puede ser muy lento" + opción de continuar.                  |
| **FS case-insensitive** | `filename` case-folded para `GROUP BY`; `path` original para mostrar.                                     |
| **Unicode (HFS+)**      | `normpath` NFC antes de almacenar. Evita "foto.jpg" vs "fotó.jpg" como duplicados distintos.         |
| **`ctime` vs `btime`**  | Se almacenan ambos; Modo 1 usa `ctime_ns`; UI muestra `btime_ns` cuando está disponible.                  |
| **Permiso denegado**    | `tracing::warn!` + `hash_status='error'`. El Final Review muestra el contador de ficheros con error.      |
| **0-byte files**        | Omitidos en Modo 1. En Modo 2, el resize de 0×0 falla → `hash_status='error'`.                            |
| **Fichero modificado mid-scan** | Best-effort: el hash refleja el momento de lectura. Reintento único si el thumb falla.            |
| **Imágenes 4K+**        | `DynamicImage` decodifica a RAM. 50 MP ≈ 200 MB en flight con 8 hilos. Cap de 8 hilos y heap 1 GB.        |
| **ffmpeg no disponible**| Sidecar empaquetado con la app. Si falla, `hash_error` y continuar.                                       |
| **DB corrupta**         | `PRAGMA integrity_check` al arranque. Si falla, ofrecer descartar y re-escanear.                          |
| **DB > 1 GB**           | Prompt para archivar. `VACUUM` al borrar sesión.                                                          |

---

## Decisiones arquitectónicas (ADRs)

Las decisiones se documentan en formato ADR ligero (Architecture Decision
Record): **contexto**, **decisión**, **consecuencias**, **alternativas**.

### ADR-001 — Tauri 2 sobre Electron o Flutter desktop

- **Contexto**: necesitamos una app de escritorio multiplataforma
  (macOS + Windows) que haga un trabajo CPU-bound pesado (hashing de
  decenas de miles de imágenes y vídeos). Los binarios deben ser
  razonablemente pequeños para distribuir.
- **Decisión**: Tauri 2 con backend Rust.
- **Consecuencias**:
  - Binarios pequeños (~10 MB) gracias a usar el webview del sistema.
  - El trabajo pesado se hace en Rust nativo con `rayon`, aprovechando
    todos los núcleos.
  - El frontend es un webview estándar, lo que limita algunos APIs del
    navegador pero es suficiente para la UI de revisión.
  - El tooling de Tauri 2 está bien pero es menos maduro que el de
    Electron.
- **Alternativas consideradas**:
  - **Electron + Node**: ecosistema JS enorme (sharp, ffmpeg.wasm,
    exifr) pero binarios de ~150 MB y rendimiento pobre para hashing
    nativo en 50k+ archivos.
  - **Flutter desktop**: UI excelente, pero el ecosistema Dart para
    image/video hashing es débil; habría que usar platform channels
    nativos igualmente.

### ADR-002 — jwalk sobre walkdir o ignore

- **Contexto**: hay que recorrer un árbol de directorios con hasta 50 000
  ficheros. El walker es el cuello de botella de la Etapa A.
- **Decisión**: `jwalk`.
- **Consecuencias**:
  - 5–10× más rápido que `walkdir` en SSDs por paralelizar el `stat()`.
  - API ligeramente más verbosa que `walkdir`.
  - Compatible con `rayon` mediante `par_bridge()`.
- **Alternativas consideradas**:
  - **walkdir**: simple y estable, pero single-threaded. Con 50k archivos
    tarda 30+ s en un SSD rápido.
  - **ignore**: añade filtrado estilo `.gitignore` que no necesitamos;
    overhead extra por entrada.

### ADR-003 — img_hash sobre perceptual-hash-rs u otros

- **Contexto**: necesitamos calcular pHash (u otro perceptual hash) de
  imágenes JPG/PNG/etc. de forma fiable, en Rust puro, sin dependencias
  nativas que compliquen el cross-compile.
- **Decisión**: `img_hash` v0.5.
- **Consecuencias**:
  - Implementa pHash, dHash, aHash, MarrHildreth, wHash. Suficiente.
  - Puro Rust, sin FFI.
  - Combina bien con el crate `image` para decodificar.
- **Alternativas consideradas**:
  - **perceptual-hash-rs**: menos mantenido, API más pequeña (sólo aHash).
  - **Llamar a libphash por FFI**: mayor superficie de fallo en Windows.

### ADR-004 — ffmpeg-sidecar sobre ffmpeg-next o system ffmpeg

- **Contexto**: para extraer keyframes de vídeos MP4/MOV/MKV/etc. hay que
  invocar ffmpeg. La solución debe funcionar en macOS y Windows sin
  requerir que el usuario instale nada.
- **Decisión**: `ffmpeg-sidecar` v3, que descarga y empaqueta el binario
  ffmpeg como asset de la app Tauri.
- **Consecuencias**:
  - Cero dolores de linker en Windows (no hay que linkear libav*).
  - El binario ffmpeg añade ~80 MB al bundle, pero es asumible.
  - ffmpeg se invoca como subproceso, no como librería; más fácil de
    debuggear y de actualizar.
  - El crate gestiona la versión de ffmpeg y la plataforma objetivo.
- **Alternativas consideradas**:
  - **ffmpeg-next**: bindings Rust sobre libav*. Requiere tener ffmpeg
    compilado para cada plataforma y linkearlo. Pesadilla en Windows.
  - **System ffmpeg (PATH)**: obliga al usuario a instalar ffmpeg. Mala
    UX; rompe la promesa de "doble clic y funciona".

### ADR-005 — rusqlite sobre sqlx o diesel

- **Contexto**: almacenamiento local de sesiones, ficheros, hashes y
  decisiones. Una sola DB por usuario, sin red.
- **Decisión**: `rusqlite` con feature `bundled` (libsqlite embebido).
- **Consecuencias**:
  - API síncrona, sin runtime async. Los comandos Tauri ya corren en un
    blocking pool, así que async sería overhead puro.
  - Libsqlite embebido: no hay que instalar nada en el sistema del
    usuario.
  - Compilación rápida; sin DSL ni migraciones autogeneradas (que no
    necesitamos: el esquema es tiny y se ejecuta al arranque).
- **Alternativas consideradas**:
  - **sqlx**: excelente para servidores con Postgres/MySQL. Async-first;
    overkill para una DB local.
  - **diesel**: añade un DSL y un sistema de migraciones que no aporta
    valor aquí; curva de aprendizaje mayor.

### ADR-006 — trash crate sobre send2trash o borrado directo

- **Contexto**: la promesa del producto es no borrar nunca permanentemente.
  Hay que enviar a la Papelera de macOS / Recycle Bin de Windows.
- **Decisión**: `trash` v5.
- **Consecuencias**:
  - API unificada cross-platform.
  - Puro Rust, sin invocar Python u otro intérprete.
  - El crate mapea a `Finder Trash` en macOS y `SHFileOperation` en
    Windows.
- **Alternativas consideradas**:
  - **send2trash** (Python): requeriría un intérprete de Python embebido
    o un subproceso; frágil y lento.
  - **Borrado directo (`rm`, `RemoveFileW`)**: rompe la promesa de
    producto. Descartado por principios.

### ADR-007 — rayon sobre tokio::spawn para el hashing

- **Contexto**: el bucle de hashing es CPU-bound (DCT, decode de
  imágenes). Tauri usa tokio por debajo para los comandos.
- **Decisión**: `rayon` con un pool acotado (`min(8, ncpu)` para imágenes,
  4 para vídeos).
- **Consecuencias**:
  - Work-stealing saturando los núcleos de CPU.
  - Cancelación cooperativa vía `CancellationToken` por fichero.
  - Para vídeos se usa un `Semaphore` además para limitar ffmpegs
    concurrentes.
- **Alternativas consideradas**:
  - **`tokio::spawn` + `spawn_blocking`**: el scheduler de tokio está
    optimizado para I/O, no para CPU-bound. `rayon` rinde mejor en el
    bucle de DCT.

### ADR-008 — zustand sobre Redux Toolkit para el estado global

- **Contexto**: estado compartido entre 5–7 pantallas (sesión actual,
  escaneos anteriores, decisiones). Pocas actualizaciones concurrentes.
- **Decisión**: `zustand`.
- **Consecuencias**:
  - ~1 KB, sin boilerplate de actions/reducers.
  - Hooks de React ergonómicos.
  - Suficiente para el volumen de estado de esta app.
- **Alternativas consideradas**:
  - **Redux Toolkit**: más estructurado pero overkill para una app de
    este tamaño; el doble de código por la misma funcionalidad.

### ADR-009 — react-virtuoso sobre react-window

- **Contexto**: hay que renderizar listas de hasta 50 000 thumbnails
  (Modo 1/2/3 + Final Review) sin que la UI se congele.
- **Decisión**: `react-virtuoso`.
- **Consecuencias**:
  - Soporte nativo de ítems de altura variable (thumbnails pueden no
    ser cuadrados si el aspect ratio varía).
  - IntersectionObserver integrado para prefetch.
  - API más rica que `react-window` (incluye scroll to index, sticky
    headers, etc.).
- **Alternativas consideradas**:
  - **react-window**: minimalista y rápido, pero menos flexible con
    alturas variables.

---

## Referencias

- [Tauri 2 docs](https://tauri.app/v2/)
- [jwalk](https://crates.io/crates/jwalk)
- [img_hash](https://crates.io/crates/img_hash)
- [ffmpeg-sidecar](https://crates.io/crates/ffmpeg-sidecar)
- [rusqlite](https://crates.io/crates/rusqlite)
- [trash](https://crates.io/crates/trash)
- [rayon](https://crates.io/crates/rayon)
- [react-virtuoso](https://virtuoso.dev/)
- [zustand](https://zustand.docs.pmnd.rs/)
