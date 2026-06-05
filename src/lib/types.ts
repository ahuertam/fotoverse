/**
 * Tipos espejo de los structs de Rust en `src-tauri/src/`.
 *
 * Regla (ver CONTRIBUTING.md#añadir-un-comando-tauri):
 * cada vez que añades un struct en Rust expuesto al frontend, lo
 * declaras también aquí. **Nunca** `any` en componentes.
 */

// ---------------------------------------------------------------------------
// ScanSession / ScanStatus
// ---------------------------------------------------------------------------

/** Estado de una sesión de escaneo (espejo de `commands::ScanSession`). */
export interface ScanSession {
  id: number;
  root_path: string;
  /** JSON string serializado desde el backend, p.ej. `["exact","perceptual"]`. */
  modes: string;
  status: "running" | "completed" | "cancelled" | "failed";
  total_files: number;
  hashed_files: number;
  started_at: string; // ISO 8601
  finished_at: string | null;
}

export interface ScanStatus {
  session: ScanSession;
  cancel_requested: boolean;
}

// ---------------------------------------------------------------------------
// Eventos backend → frontend
// ---------------------------------------------------------------------------

/** Payload de `scan_progress` (throttled 200 ms). */
export interface ScanProgress {
  session_id: number;
  phase: "walking" | "hashing_images" | "hashing_videos" | "clustering" | "done";
  files_seen: number;
  total_estimate: number | null;
  current_path: string | null;
}

/** Payload de `scan_completed` (evento terminal). */
export interface ScanCompleted {
  session_id: number;
  total_files: number;
  duration_ms: number;
  status: "completed" | "cancelled" | "failed";
  error: string | null;
}

/** Payload de `scan_error` (evento de error irrecuperable). */
export interface ScanError {
  session_id: number;
  message: string;
}

// ---------------------------------------------------------------------------
// Tipos de modo de detección
// ---------------------------------------------------------------------------

/** Modos de detección que el usuario puede activar. */
export type ScanMode = "exact" | "perceptual" | "manual";

// ---------------------------------------------------------------------------
// Modo 1 — Duplicados exactos
// ---------------------------------------------------------------------------

/** Cabecera de un grupo (espejo de `detect::exact::ExactGroup`). */
export interface ExactGroup {
  id: number;
  session_id: number;
  filename: string;
  ext: string;
  ctime_ns: number;
  size_bytes: number;
  member_count: number;
  reviewed: boolean;
}

/** Miembro de un grupo (espejo de `detect::exact::ExactGroupMember`). */
export interface ExactGroupMember {
  id: number;
  path: string;
  size_bytes: number;
  ctime_ns: number | null;
  btime_ns: number | null;
  mtime_ns: number;
  /** "delete" | "keep" | "undecided" | null (sin decisión aún). */
  decision: string | null;
  /** id del miembro con btime_ns más antiguo; es la sugerencia a mantener. */
  suggested_keep_id: number;
}

/** Decisión que el usuario puede tomar sobre un fichero. */
export type DecisionAction = "delete" | "keep" | "undecided";

/** Modo del que se piden los miembros de un grupo. */
export type GroupMode = "exact" | "perceptual" | "manual";

// ---------------------------------------------------------------------------
// Modo 2 — Duplicados perceptuales (Fase 5)
// ---------------------------------------------------------------------------

/** Cabecera de un grupo perceptual (espejo de `detect::perceptual::PerceptualGroup`). */
export interface PerceptualGroup {
  id: number;
  sessionId: number;
  representative: number;
  hammingToRep: number;
  memberCount: number;
  reviewed: boolean;
}

/** Miembro de un grupo perceptual (espejo de `detect::perceptual::PerceptualGroupMember`). */
export interface PerceptualGroupMember {
  id: number;
  path: string;
  sizeBytes: number;
  ctimeNs: number | null;
  btimeNs: number | null;
  mtimeNs: number;
  /** "delete" | "keep" | "undecided" | null */
  decision: string | null;
  hammingToRep: number;
  /** id del representante del cluster; sugerencia a mantener. */
  suggestedKeepId: number;
}
