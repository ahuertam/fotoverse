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
