-- ============================================================================
-- fotoverse — esquema inicial de la base de datos
-- ============================================================================
-- Esta migración se ejecuta en cada arranque (CREATE TABLE IF NOT EXISTS).
-- En el futuro introduciremos un sistema de versiones con tabla `schema_migrations`
-- si necesitamos migraciones destructivas. Por ahora idempotencia basta.
-- ============================================================================

-- ---------------------------------------------------------------------------
-- Sesiones de escaneo
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS scan_sessions (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  root_path     TEXT    NOT NULL,
  modes         TEXT    NOT NULL,           -- JSON: ["exact","perceptual","manual"]
  status        TEXT    NOT NULL,           -- running | completed | cancelled | failed
  total_files   INTEGER NOT NULL DEFAULT 0,
  hashed_files  INTEGER NOT NULL DEFAULT 0,
  started_at    TEXT    NOT NULL,
  finished_at   TEXT
);

-- ---------------------------------------------------------------------------
-- Ficheros descubiertos en el walk
-- ---------------------------------------------------------------------------
-- Se inserta una fila por cada fichero de imagen o vídeo encontrado.
-- Los hashes se rellenan en Fases 5-6 (pHash para imágenes, keyframes hash
-- para vídeos). La columna `kind` permite filtrar por tipo.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS files (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id    INTEGER NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
  path          TEXT    NOT NULL,
  filename      TEXT    NOT NULL,           -- basename case-folded
  ext           TEXT    NOT NULL,           -- lowercase sin punto
  size_bytes    INTEGER NOT NULL,
  ctime_ns      INTEGER,                    -- Windows: creation; Unix: inode change
  btime_ns      INTEGER,                    -- birth time (best-effort)
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

CREATE INDEX IF NOT EXISTS idx_files_session ON files(session_id);
CREATE INDEX IF NOT EXISTS idx_files_exact   ON files(session_id, filename, ext, ctime_ns, size_bytes);
CREATE INDEX IF NOT EXISTS idx_files_phash   ON files(session_id, phash)    WHERE phash    IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_files_vhash   ON files(session_id, vhash)    WHERE vhash    IS NOT NULL;

-- ---------------------------------------------------------------------------
-- Decisiones del usuario (placeholder; lo usa Fase 3+)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS decisions (
  file_id    INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
  action     TEXT    NOT NULL,             -- 'delete' | 'keep' | 'undecided'
  decided_at TEXT    NOT NULL
);
