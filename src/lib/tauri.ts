/**
 * Wrappers tipados de `invoke()` y `listen()`.
 *
 * Regla: **todos** los componentes importan de aquí, nunca llaman a
 * `invoke` directamente. Si añades un comando Rust, su wrapper vive aquí.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

import type {
  ExactGroup,
  ExactGroupMember,
  GroupMode,
  ScanCompleted,
  ScanError,
  ScanMode,
  ScanProgress,
  ScanSession,
  ScanStatus,
} from "./types";

// ---------------------------------------------------------------------------
// Comandos
// ---------------------------------------------------------------------------

/** Abre el diálogo nativo de selección de carpeta. */
export async function pickFolder(): Promise<string | null> {
  // El plugin-dialog del frontend tiene una API propia, pero la
  // delegación al backend (vía el comando `pick_folder`) nos da
  // control total sobre permisos. Aquí usamos el plugin directamente
  // porque ya tenemos el permiso `dialog:default` en capabilities.
  const result = await openDialog({
    directory: true,
    multiple: false,
    title: "Elegir carpeta para escanear",
  });
  if (Array.isArray(result)) return result[0] ?? null;
  return (result as string | null) ?? null;
}

/** Lanza un escaneo en background. Devuelve la sesión recién creada. */
export async function startScan(
  root: string,
  modes: ScanMode[],
): Promise<ScanSession> {
  return invoke<ScanSession>("start_scan", { args: { root, modes } });
}

/** Solicita la cancelación de un escaneo en curso. */
export async function cancelScan(sessionId: number): Promise<void> {
  return invoke<void>("cancel_scan", { args: { sessionId } });
}

/** Sondea el estado actual de un escaneo. */
export async function getScanStatus(sessionId: number): Promise<ScanStatus> {
  return invoke<ScanStatus>("get_scan_status", { args: { sessionId } });
}

/** Lista las últimas 100 sesiones (de más reciente a más antigua). */
export async function listSessions(): Promise<ScanSession[]> {
  return invoke<ScanSession[]>("list_sessions");
}

// ---------------------------------------------------------------------------
// Fase 4 — Thumbnails
// ---------------------------------------------------------------------------

/** Devuelve la ruta absoluta al JPEG cacheado del thumbnail de un file_id. */
export async function getThumbnail(fileId: number, size = 0): Promise<string> {
  return invoke<string>("get_thumbnail", { args: { fileId, size } });
}

// ---------------------------------------------------------------------------
// Modo 1 — Duplicados exactos
// ---------------------------------------------------------------------------

/** Lista paginada de grupos de duplicados exactos. */
export async function getExactGroups(
  sessionId: number,
  offset: number,
  limit: number,
): Promise<{ groups: ExactGroup[]; total: number }> {
  return invoke<{ groups: ExactGroup[]; total: number }>("get_exact_groups", {
    args: { sessionId, offset, limit },
  });
}

/** Miembros de un grupo con su decisión actual y el sugerido a mantener. */
export async function getGroupMembers(
  groupId: number,
  mode: GroupMode = "exact",
): Promise<{ members: ExactGroupMember[] }> {
  return invoke<{ members: ExactGroupMember[] }>("get_group_members", {
    args: { groupId, mode },
  });
}

/** Persiste la decisión del usuario sobre un fichero. */
export async function setDecision(
  fileId: number,
  action: "delete" | "keep" | "undecided",
): Promise<void> {
  return invoke<void>("set_decision", {
    args: { fileId, action },
  });
}

/** Persiste decisiones en lote (workflow "borrar el resto"). */
export async function bulkSetDecision(
  fileIds: number[],
  action: "delete" | "keep" | "undecided",
): Promise<void> {
  return invoke<void>("bulk_set_decision", {
    args: { fileIds, action },
  });
}

// ---------------------------------------------------------------------------
// Eventos
// ---------------------------------------------------------------------------

export function onScanProgress(
  handler: (p: ScanProgress) => void,
): Promise<UnlistenFn> {
  return listen<ScanProgress>("scan_progress", (e) => handler(e.payload));
}

export function onScanCompleted(
  handler: (p: ScanCompleted) => void,
): Promise<UnlistenFn> {
  return listen<ScanCompleted>("scan_completed", (e) => handler(e.payload));
}

export function onScanError(
  handler: (p: ScanError) => void,
): Promise<UnlistenFn> {
  return listen<ScanError>("scan_error", (e) => handler(e.payload));
}
