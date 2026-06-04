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
