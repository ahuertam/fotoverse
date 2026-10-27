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

- **Rendimiento de hashing**: el perceptual hash de imágenes es CPU-bound.
  Rust con `rayon` aprovecha todos los núcleos. Node/Electron sufriría mucho
  con decenas de miles de archivos.
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
│       └── ci.yml                  # type-check, lint, clippy, fmt y audit (3 SO)
├── docs/
│   ├── ARCHITECTURE.md             # este archivo
│   ├── CONTRIBUTING.md             # guía para devs
│   ├── ROADMAP.md                  # fases de v0.1 con criterios de "hecho"
│   └── ROADMAP_v0.2.md             # plan de v0.2
├── src-tauri/                      # Backend Rust
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── capabilities/
│   │   └── default.json            # permisos Tauri (core, opener, dialog, fs)
│   ├── icons/
│   ├── src/
│   │   ├── main.rs                 # entry point
│   │   ├── lib.rs                  # tauri::Builder + plugins + comandos
│   │   ├── state.rs                # AppState: handles de escaneo
│   │   ├── cancel.rs               # CancellationToken
│   │   ├── error.rs                # AppError / AppResult con thiserror
│   │   ├── commands.rs             # todos los #[tauri::command] (ver §7)
│   │   ├── db/
│   │   │   ├── mod.rs              # conexión, integrity_check, VACUUM, reset_db
│   │   │   ├── migrations.rs       # schema.sql + deltas de columnas
│   │   │   └── schema.sql          # el esquema de la §4
│   │   ├── walk/
│   │   │   ├── mod.rs              # jwalk + rayon + inserción por lotes
│   │   │   └── progress.rs         # emisión de scan_progress (throttle 200 ms)
│   │   ├── hash/
│   │   │   ├── mod.rs
│   │   │   └── image_hash.rs       # pHash propio: DCT 2D + Hamming
│   │   ├── detect/
│   │   │   ├── mod.rs
│   │   │   ├── exact.rs            # Modo 1 (SQL GROUP BY)
│   │   │   ├── perceptual.rs       # Modo 2 (pHash + Union-Find O(n²))
│   │   │   └── manual.rs           # Modo 3 (pares para revisión)
│   │   ├── review/
│   │   │   ├── mod.rs
│   │   │   ├── papelera.rs         # trash::delete_all + umbral de confirmación
│   │   │   ├── final_review.rs     # resumen previo al borrado
│   │   │   └── video_heavy.rs      # Modo 4: duplicados de vídeo + top-N
│   │   └── thumbs/
│   │       └── mod.rs              # generación y caché de thumbnails
│   └── tests/                      # tests de integración (fuera de src/)
│       ├── fixture.rs              # genera la fixture JPEG/WebP
│       ├── walk_test.rs
│       ├── walk_trial_test.rs
│       ├── detect_exact_test.rs
│       └── thumbs_test.rs
├── src/                            # Frontend React 19 + TypeScript
│   ├── main.tsx
│   ├── App.tsx                     # router
│   ├── pages/
│   │   ├── Welcome.tsx
│   │   ├── ScanProgress.tsx
│   │   ├── Mode1Review.tsx
│   │   ├── Mode2Review.tsx
│   │   ├── Mode3Review.tsx
│   │   ├── ModeVideoAndHeavy.tsx
│   │   ├── FinalReview.tsx
│   │   ├── Done.tsx
│   │   ├── Settings.tsx
│   │   └── NotFound.tsx
│   ├── components/
│   │   ├── Splash.tsx
│   │   └── Thumbnail.tsx
│   ├── hooks/
│   │   └── useScanProgress.ts
│   ├── lib/
│   │   ├── tauri.ts                # wrappers tipados de invoke/listen
│   │   └── types.ts                # DTOs espejo de Rust
│   ├── styles/global.css
│   └── assets/fotologo.png
├── scripts/
│   ├── smoke.sh                    # smoke test con fixture sintética
│   ├── make_gif.sh                 # genera el GIF del README
│   └── demo/                       # mock de Tauri para capturas sin backend
├── package.json
├── pnpm-lock.yaml
├── pnpm-workspace.yaml
├── vite.config.ts
├── tsconfig.json
├── eslint.config.js
├── .editorconfig
├── .gitignore
├── LICENSE
└── README.md
```

---

## Stack tecnológico

### Backend Rust (`src-tauri/Cargo.toml`)

Dependencias reales del crate. Lo que no aparece en esta tabla es que no se usa.

| Necesidad              | Crate                                                  | Por qué                                                                                            |
|------------------------|--------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| Runtime Tauri          | `tauri = "2"`, `tauri-build = "2"`                     | Tauri 2.x: modelo de plugins estable, binario pequeño.                                              |
| Recorrido de FS        | `jwalk`                                                | Walker paralelo, con `par_bridge()` a rayon.                                                       |
| pHash de imágenes      | **propio**, en `hash/image_hash.rs`                    | `img_hash` no es viable aquí — ver ADR-003. No es una dependencia.                                   |
| Decodificar imágenes   | `image = "0.25"`                                       | JPG/PNG/WebP/GIF/BMP/TIFF al `DynamicImage` que necesita el pHash.                                 |
| SQLite                 | `rusqlite = { version = "0.32", features = ["bundled"] }` | Síncrono, libsqlite embebido. Tauri ya corre sobre un blocking pool; async sería overhead.       |
| Mover a papelera       | `trash = "5"`                                          | Cross-platform, puro Rust, va a Finder/Recycle Bin.                                                |
| Paralelismo CPU        | `rayon = "1"`                                          | Work-stealing para el bucle de hashing y thumbnails.                                               |
| Cancelación            | `tokio-util` (CancellationToken)                       | Propagación anidada, integración con el runtime de Tauri.                                          |
| Normalización Unicode  | `normpath = "1"`                                       | NFC + canonicalización; evita trampas de HFS+.                                                     |
| Thumbnails             | `sha2`, `hex`                                          | Clave de caché = SHA-256 del path + mtime.                                                         |
| Errores                | `thiserror` (tipos propios), `anyhow` (contexto)       | `AppError` con `#[from]` para cada io::Error.                                                       |
| Logging                | `tracing`, `tracing-subscriber`                        | Logs estructurados con `session_id` en cada evento.                                                 |
| Tiempo                 | `chrono`                                               | `ctime` / `mtime` en nanosegundos.                                                                 |

### Dependencias que se descartaron

| Crate             | Por qué no está                                                                 |
|-------------------|---------------------------------------------------------------------------------|
| `img_hash`        | 0.5 exige nightly; 3.x está atada a `image 0.21-0.23`. Ver ADR-003.             |
| `ffmpeg-sidecar`  | Decidido en ADR-004 pero **no implementado**: no hay hash perceptual de vídeo.  |
| `kamadak-exif`    | No está. Las fechas se leen del `metadata()` del sistema de ficheros.            |

### Plugins Tauri

| Plugin              | Para qué                                                        |
|---------------------|-----------------------------------------------------------------|
| `tauri-plugin-dialog`| Selector nativo de carpeta (nunca una ruta escrita a mano).      |
| `tauri-plugin-opener`| Mostrar el contenido de una carpeta de resultados.               |
| `tauri-plugin-fs`   | Declarado en capabilities pero **el frontend no lo usa**: todo el acceso a disco ocurre en Rust. Pendiente de quitar. |
| `tauri-plugin-store`| Persistencia de ajustes (`lastFolder`, `theme`, `includeHidden`). |

No hay `tauri-plugin-http` ni `tauri-plugin-shell`. La app no puede hacer red.

### Frontend (`package.json`)

| Necesidad        | Paquete                          | Por qué                                          |
|------------------|----------------------------------|--------------------------------------------------|
| UI               | `react` 19, `react-dom`          | Lo que ya usa el equipo en el editor de Gamelearn. |
| Estado global    | `zustand`                        | Ver ADR-008.                                      |
| Routing          | `react-router-dom` 7             | Una ruta por pantalla de revisión.                |
| Listas virtuales | `react-virtuoso`                | Miles de thumbnails sin congelar la UI. Ver ADR-009. |
| Build            | `vite` 7 + TypeScript 5.8        | —                                                 |
| Lint             | `eslint` 9 + typescript-eslint   | Corre en CI en los 3 SO.                          |

**No hay librería de i18n.** Los strings están en español, en el JSX. Ver §
"Páginas de la UI → Internacionalización".

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
- **`phash` son 8 bytes** (64 bits) y es la única columna de hash que se
  escribe hoy. `vhash` (8 bytes) y `vhash_extra` (16 bytes) están declaradas
  para el hash perceptual de vídeo, que **no está implementado** — ningún
  código las rellena.
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
- **Vídeos**: **no implementado.** El plan era un pool de 4 hilos con
  `Semaphore` (ffmpeg es process-heavy; 50k ffmpegs en paralelo OOMearía la
  máquina), extrayendo 3 keyframes por vídeo y concatenándolas en `vhash` +
  `vhash_extra`. Ver ADR-004.

Cada worker hace `UPDATE` de su fila en SQLite al terminar (WAL serializa
bien los writers). El progreso se emite throttled a **cada 200 ms** para no
ahogar el IPC.

La cancelación se comprueba por fichero (`CancellationToken::is_cancelled`).

### Etapa C — Cluster (secuencial, O(n²) con poda)

- Compara todos los pares de hashes de la sesión: **O(n²)**.
- Dos imágenes unen si su distancia de **Hamming ≤ 10**.
- Poda temprana: si el XOR del primer byte ya supera el umbral, el par se
  descarta sin calcular la distancia completa.
- Union-Find agrupa las uniones. El representante de cada cluster es el miembro
  con menor `id`.
- Inserta los clusters en `perceptual_groups`.
- Por encima de **5.000 imágenes** se emite un `warn!` de coste cuadrático. El
  sustituto previsto es un BK-tree, **no implementado**.
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
                              │ C. Cluster   │  Union-Find O(n²)
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

#### Hash de imágenes — implementación propia (`src-tauri/src/hash/image_hash.rs`)

```
1. decode → DynamicImage
2. to_luma8()            (grayscale directo, sin pasar por RGB8)
3. resize a 32×32 con FilterType::Triangle
4. DCT 2D separable      (filas, luego columnas; sin normalizar)
5. bloque top-left 8×8    (la componente DC en (0,0) no entra en el threshold)
6. mediana de los 63 valores restantes
7. bit i = 1 iff valor > mediana
8. pack 64 bits → 8 bytes
```

**Por qué propio y no `img_hash`:** la 0.5 usa `#![feature(hash)]` y `push_all`
(requiere nightly); la 3.x está atada a `image 0.21-0.23` y el proyecto va a
`0.25`. Son ~150 líneas a cambio de controlar el formato del hash. Ver ADR-003.

#### Hash de vídeos — no implementado

El hash perceptual de vídeo (`vhash`, `vhash_extra`) **no existe**. Las columnas
están en el esquema, pero ningún código las escribe.

Lo que sí hay es el **Modo 4**, que es *por metadatos*, no perceptual:
duplicados de vídeo con el mismo algoritmo que el Modo 1 filtrado a
`kind='video'`, más un top-N de los vídeos más pesados con una **estimación** de
ahorro al 70% (`size * 0.30`). No ejecuta ffmpeg ni recomprime nada:
`review/video_heavy.rs` lo dice en su propia cabecera.

El diseño futuro (`ffmpeg-sidecar`, 3 keyframes por vídeo → pHash de cada una)
está en ADR-004, pero no está escrito. **Trátalo como pendiente.**

#### Clustering — Union-Find, no BK-tree

Implementado en `detect/perceptual.rs`:

- **Union-Find** con path compression parcial sobre los hashes de la sesión.
- Comparación **O(n²)** con poda temprana: si el XOR del primer byte ya supera
  el umbral, el par se descarta sin calcular la distancia completa. En pHash de
  64 bits ahorra poco (~1%); está por claridad del límite, no por rendimiento.
- **Umbral Hamming ≤ 10** (`HAMMING_THRESHOLD`, en `hash/image_hash.rs`).
- Representante = miembro con menor `id` del cluster.
- Ficheros con decisión previa **no se re-surfacean** en Modo 2 (excluidos con
  `NOT IN (SELECT file_id FROM decisions)`).
- Por encima de **5.000 imágenes se emite un `warn!`**: el coste cuadrático ya
  se nota, y el sustituto previsto es un **BK-tree**, O(log n) por consulta.

> El BK-tree **no está implementado**. Cualquier versión anterior de este
> documento que dijera que sí, era incorrecta.

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
| `get_manual_pairs`     | `{ session_id, offset, limit, filter? }`                    | `{ pairs, total }`                            | Pairs pendientes del Modo 3.                                             |
| `skip_manual_pair`     | `{ pair_id }`                                               | `()`                                          | Marca un par como revisado para no volver a mostrarlo.                   |
| `get_group_members`    | `{ group_id, mode }`                                        | `{ members, ... }`                            | Expande los miembros de un grupo.                                        |
| `set_decision`         | `{ file_id, action }`                                       | `()`                                          | Persiste `keep` / `delete` / `undecided` para un fichero.                |
| `bulk_set_decision`    | `{ file_ids, action }`                                      | `()`                                          | Marca muchas decisiones a la vez (workflow "borrar el resto").          |
| `get_thumbnail`        | `{ file_id, size }`                                         | `String` (ruta absoluta)                      | Ruta del thumbnail cacheado; lo genera on-demand si falta.               |
| `get_files_meta`       | `{ file_ids }`                                              | `FileMeta[]`                                  | Metadatos de varios ficheros a la vez.                                   |
| `get_video_and_heavy`  | `{ session_id, limit }`                                     | `VideoAndHeavyResult`                         | Modo 4: duplicados de vídeo por metadatos + top-N de los más pesados.    |
| `get_final_review`     | `{ session_id }`                                            | `File[]`                                      | Todos los ficheros actualmente marcados `delete`.                        |
| `execute_deletions`    | `{ file_ids }`                                              | `{ deleted, failed, failures, bytes_freed }` | Mueve a la papelera vía `trash::delete_all`, un archivo a uno.           |
| `check_db_integrity`   | —                                                           | `IntegrityReport`                             | `PRAGMA integrity_check` al arranque.                                    |
| `reset_db`             | —                                                           | `()`                                          | Borra el SQLite y sus `-wal`/`-shm`. Solo tras un integrity fallido.    |
| `db_size`              | —                                                           | `i64`                                         | Tamaño del fichero de BD en bytes.                                      |

> Una versión anterior de esta tabla listaba un comando `open_path` para
> "revelar en Finder". **No existe.** Si alguna vez se anunció, era incorrecto.

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
- **Vídeos**: **sin thumbnail.** Reutilizar el keyframe de ffmpeg era el plan,
  pero no hay ffmpeg (ver ADR-004). La página de Modo 4 los muestra sin miniatura.

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

Por ahora **sólo español** (decisión de producto). **No hay sistema de i18n**:
los strings están escritos directamente en el JSX y no existe `src/i18n/`.
Una versión anterior de este documento mencionaba un `es.json`; ese archivo
nunca llegó a existir.

Si algún día se añade inglés, la entrada sería `react-i18next` y un
`src/i18n/es.json` + `en.json`, no más de eso.

---

## Seguridad

La promesa de producto es **"nunca se borra nada permanentemente"**. Esto
está reforzado por código, tests y checklist de PR.

1. **Toda eliminación de ficheros del usuario va por `trash::delete_all`**
   (`review/papelera.rs`). No hay borrado permanente en ninguna ruta de código.
   La única excepción es `db/mod.rs`, que usa `remove_file` sobre el **propio
   fichero SQLite** y sus `-wal`/`-shm` cuando el usuario confirma un reset
   tras un `integrity_check` fallido. No toca fotos ni vídeos.

   Esto se verifica en CI con un grep que excluye ese módulo:
   ```bash
   ! grep -rn 'remove_file\|remove_dir' src-tauri/src/ | grep -v 'db/mod.rs'
   ```
2. **Final Review obligatorio** antes de ejecutar. Muestra resumen
   acumulativo de archivos y bytes a liberar.
3. **Modal de confirmación tipada** ("Escribe ELIMINAR para continuar") si
   el set excede 1000 archivos o 5 GB (`review/papelera.rs`,
   `THRESHOLD_FILES` / `THRESHOLD_BYTES`).
4. **`execute_deletions` reporta fallos por archivo** como tuplas
   `(path, error)`. El usuario decide reintentar los fallidos.
5. **Sin acceso a red**: no hay `tauri-plugin-http` ni `tauri-plugin-shell` en
   las dependencias ni en `capabilities/default.json`. La app no tiene con qué
   hacer red — no es una política activa, es que no existe el código.
6. **WAL en SQLite**: un corte de luz durante un escaneo deja la sesión
   anterior intacta; el escaneo parcial queda marcado como `failed` y puede
   borrarse.
7. **Path normalization (NFC)** evita que "foto.jpg" y "fotó.jpg" se traten
   como duplicados distintos. No es una defensa de path traversal — el path lo
   elige el usuario vía el diálogo nativo, no el frontend.
8. **`tauri-plugin-fs` está declarado en `capabilities/default.json` pero el
   frontend no lo usa**: todo el acceso a disco ocurre en el backend Rust, que
   no pasa por los permisos del plugin. El permiso sobrante puede quitarse en
   una pasada de limpieza; hasta entonces es superficie muerta, no un riesgo
   activo.

---

## Edge cases

| Caso                    | Tratamiento                                                                                                |
|-------------------------|-----------------------------------------------------------------------------------------------------------|
| **Symlinks**            | `follow_links(false)` en el walk. **No hay contador en la UI** — se omiten en silencio.                    |
| **Hidden files**        | Siempre **incluidos**: el walk usa `skip_hidden(false)` fijo. Hay un toggle `includeHidden` en `Settings.tsx` que persiste en el store pero **no llega al backend**: hoy no hace nada. Diferido a v0.2. |
| **Network drives**      | **No detectado.** Un escaneo sobre SMB/NFS funciona, pero puede tardar mucho y sin aviso.                  |
| **FS case-insensitive** | `filename` case-folded para `GROUP BY`; `path` original para mostrar.                                     |
| **Unicode (HFS+)**      | `normpath` NFC antes de almacenar. Evita "foto.jpg" vs "fotó.jpg" como duplicados distintos.         |
| **`ctime` vs `btime`**  | Se almacenan ambos; Modo 1 agrupa por `ctime_ns`. En Unix `ctime` es *inode change time*, no creación, así que puede agrupar dos ficheros que no salieron del mismo original. La UI enseña la fecha que tenga. |
| **Permiso denegado**    | `tracing::warn!` + `hash_status='error'` con el mensaje en `hash_error`. **No hay contador de errores en el Final Review**: solo se ve en los logs. |
| **0-byte files**        | **No se omiten en Modo 1**: no hay filtro de `size_bytes`, así que dos ficheros vacíos con el mismo nombre agrupan como duplicados. En Modo 2 el decode falla → `hash_status='error'`. Pendiente. |
| **Fichero modificado mid-scan** | Best-effort: el hash refleja el momento de lectura. Sin reintento si falla el thumbnail.                |
| **Imágenes 4K+**        | `DynamicImage` decodifica a RAM. 50 MP ≈ 200 MB en vuelo con 8 hilos; el pool acotado a `min(8, ncpu)` existe justo para acotar esa cifra. |
| **Vídeo**               | **Sin hash perceptual.** El Modo 4 agrupa por metadatos y estima el ahorro al 70% sin ejecutar ffmpeg. Ver ADR-004. |
| **DB corrupta**         | `PRAGMA integrity_check` al arranque (`check_db_integrity`). Si falla se ofrece reset, que borra el fichero SQLite y sus `-wal`/`-shm`. |
| **DB > 1 GB**           | `db_size` expone el tamaño; `VACUUM` se ejecuta al borrar sesión. **No hay prompt de archivado.**          |

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

### ADR-003 — pHash propio sobre `img_hash` y otras librerías

- **Estado**: aplicado en v0.1.0 (`src-tauri/src/hash/image_hash.rs`).
- **Contexto**: necesitamos calcular pHash de imágenes JPG/PNG/etc. en Rust
  puro, sin dependencias nativas que compliquen el cross-compile, y sobre el
  crate `image 0.25`.
- **Decisión**: **implementar pHash a mano** (~150 líneas: grayscale → resize
  32×32 → DCT 2D separable → bloque 8×8 → mediana → 64 bits).
- **Consecuencias**:
  - Control total del formato del hash: 8 bytes exactos, que es lo que espera
    la columna `files.phash BLOB`.
  - Sin `nightly`, sin FFI, sin conflicto de versiones.
  - Solo se obtiene pHash. Si alguna vez hiciera falta dHash o wHash, habría
    que añadirlos (o volver a mirar una librería).
- **Alternativas consideradas**:
  - **`img_hash` 0.5**: usa `#![feature(hash)]` y `push_all` → exige nightly.
    Descartada.
  - **`img_hash` 3.x**: pura y estable, pero atada a `image 0.21-0.23`.
    Incompatible con el `image 0.25` que usa el proyecto. Descartada.
  - **`perceptual-hash-rs`**: menos mantenido, API más pequeña (solo aHash).
    Descartada.
  - **libphash por FFI**: superficie de fallo y de cross-compile en Windows.
    Descartada.

> Una versión anterior de este ADR decidía `img_hash` v0.5. Esa decisión no
> sobrevivió al primer `cargo check`.

### ADR-004 — ffmpeg-sidecar sobre ffmpeg-next o system ffmpeg

- **Estado**: **decidido, no implementado.** No hay ffmpeg en `Cargo.toml` ni en
  el bundle. El Modo 4 (`review/video_heavy.rs`) es por metadatos y estima el
  ahorro sin ejecutar nada.
- **Contexto**: para extraer keyframes de vídeos MP4/MOV/MKV/etc. hay que
  invocar ffmpeg. La solución debe funcionar en macOS y Windows sin requerir
  que el usuario instale nada.
- **Decisión (prevista para v0.2)**: `ffmpeg-sidecar` v3, que empaqueta el
  binario de ffmpeg como asset de la app.
- **Consecuencias (esperadas)**:
  - Cero dolores de linker en Windows (no hay que linkear `libav*`).
  - El binario añade ~80 MB al bundle.
  - ffmpeg se invoca como subproceso, no como librería: más fácil de debuggear
    y de actualizar.
- **Alternativas consideradas**:
  - **`ffmpeg-next`**: bindings sobre `libav*`. Obliga a tener ffmpeg compilado
    por plataforma y linkearlo. Pesadilla en Windows.
  - **System ffmpeg (PATH)**: obliga al usuario a instalarlo. Rompe la promesa
    de "doble clic y funciona".

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
- **Decisión**: `rayon` con un pool acotado (`min(8, ncpu)`) para el hashing de
  imágenes y para la generación de thumbnails.
- **Consecuencias**:
  - Work-stealing saturando los núcleos de CPU.
  - El walk y el hashing corren en `spawn_blocking`, no en el reactor, para no
    bloquear el runtime de tokio.
  - Cancelación cooperativa vía `CancellationToken`, comprobada cada 1.000
    entradas durante el walk.
  - El pool está acotado a propósito: con 50k imágenes, saturar el SSD rinde
    peor que dejar algo de I/O en paralelo.
- **Pendiente (v0.2)**: el hashing de vídeo necesitará además un `Semaphore`
  que limite los ffmpeg concurrentes. No existe todavía — ver ADR-004.
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
