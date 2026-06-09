/**
 * Wrappers tipados de `invoke()` y `listen()`.
 *
 * Regla: **todos** los componentes importan de aquí, nunca llaman a
 * `invoke` directamente. Si añades un comando Rust, su wrapper vive aquí.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { load, Store } from "@tauri-apps/plugin-store";

import type {
  ExactGroup,
  ExactGroupMember,
  GroupMode,
  PerceptualGroup,
  PerceptualGroupMember,
  ScanCompleted,
  ScanError,
  ScanMode,
  ScanProgress,
  ScanSession,
  ScanStatus,
  VideoAndHeavyResult,
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
// Fase 7 — Modo 3 (revisión visual manual)
// ---------------------------------------------------------------------------

/** Par para revisión side-by-side. */
export interface ManualPair {
  id: number;
  sessionId: number;
  fileA: number;
  fileB: number;
  reviewed: boolean;
}

export async function getManualPairs(
  sessionId: number,
  offset: number,
  limit: number,
  filter: "pending" | "all" = "pending",
): Promise<{ pairs: ManualPair[]; total: number }> {
  return invoke<{ pairs: ManualPair[]; total: number }>(
    "get_manual_pairs",
    { args: { sessionId, offset, limit, filter } },
  );
}

/** Marca un par como revisado (saltar sin decidir). */
export async function skipManualPair(pairId: number): Promise<void> {
  return invoke<void>("skip_manual_pair", { args: { pairId } });
}

/** Metadata de uno o varios ficheros (path, size, mtime). */
export async function getFilesMeta(
  fileIds: number[],
): Promise<{ files: { id: number; path: string; sizeBytes: number; mtimeNs: number }[] }> {
  return invoke<{
    files: { id: number; path: string; sizeBytes: number; mtimeNs: number }[];
  }>("get_files_meta", { args: { fileIds } });
}

// ---------------------------------------------------------------------------
// Fase 5 — Modo 2 (duplicados perceptuales)
// ---------------------------------------------------------------------------

export async function getPerceptualGroups(
  sessionId: number,
  offset: number,
  limit: number,
): Promise<{ groups: PerceptualGroup[]; total: number }> {
  return invoke<{ groups: PerceptualGroup[]; total: number }>(
    "get_perceptual_groups",
    { args: { sessionId, offset, limit } },
  );
}

// ---------------------------------------------------------------------------
// Fase 8 — Final review + papelera
// ---------------------------------------------------------------------------

/** Fichero en la final review. */
export interface FinalReviewItem {
  fileId: number;
  path: string;
  filename: string;
  sizeBytes: number;
  mtimeNs: number;
  decidedIn: string;
}

export interface GetFinalReviewResult {
  items: FinalReviewItem[];
  count: number;
  totalBytes: number;
}

export async function getFinalReview(
  sessionId: number,
): Promise<GetFinalReviewResult> {
  return invoke<GetFinalReviewResult>("get_final_review", {
    args: { sessionId },
  });
}

/** Reporte de ejecución de borrado a la papelera. */
export interface DeletionReport {
  deleted: number;
  failed: number;
  failures: [string, string][]; // [path, error]
  bytesFreed: number;
}

export async function executeDeletions(
  sessionId: number,
  fileIds: number[],
): Promise<{ report: DeletionReport }> {
  return invoke<{ report: DeletionReport }>("execute_deletions", {
    args: { sessionId, fileIds },
  });
}

// ---------------------------------------------------------------------------
// Bonus v0.2 — Modo Vídeo y Peso
// ---------------------------------------------------------------------------

export async function getVideoAndHeavy(
  sessionId: number,
  topN = 20,
): Promise<VideoAndHeavyResult> {
  return invoke<VideoAndHeavyResult>("get_video_and_heavy", {
    args: { sessionId, topN },
  });
}

// ---------------------------------------------------------------------------
// Fase 9 — Settings (tauri-plugin-store)
// ---------------------------------------------------------------------------

const SETTINGS_FILE = "settings.json";

/** Settings persistentes. Almacenados en app_data_dir. */
export interface AppSettings {
  /** Incluir archivos/carpetas ocultos (`.jpg`, `.photoslibrary`).
   * Default: true (los oculta son comunes en Macs). */
  includeHidden: boolean;
  /** Última carpeta seleccionada (para "Re-escanea aquí"). */
  lastFolder: string | null;
  /** Tema visual. Default: "dark". */
  theme: "dark" | "light";
}

const DEFAULT_SETTINGS: AppSettings = {
  includeHidden: true,
  lastFolder: null,
  theme: "dark",
};

let cachedStore: Store | null = null;

async function getStore(): Promise<Store> {
  if (cachedStore) return cachedStore;
  cachedStore = await load(SETTINGS_FILE, {
    autoSave: true,
    defaults: DEFAULT_SETTINGS as unknown as Record<string, unknown>,
  });
  return cachedStore;
}

export async function getSettings(): Promise<AppSettings> {
  try {
    const s = await getStore();
    return {
      includeHidden: (await s.get<boolean>("includeHidden")) ?? DEFAULT_SETTINGS.includeHidden,
      lastFolder: (await s.get<string | null>("lastFolder")) ?? DEFAULT_SETTINGS.lastFolder,
      theme: (await s.get<"dark" | "light">("theme")) ?? DEFAULT_SETTINGS.theme,
    };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export async function setSetting<K extends keyof AppSettings>(
  key: K,
  value: AppSettings[K],
): Promise<void> {
  try {
    const s = await getStore();
    await s.set(key, value);
    await s.save();
  } catch (e) {
    console.error("[settings] no se pudo guardar:", e);
  }
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
): Promise<{ members: ExactGroupMember[] | PerceptualGroupMember[] }> {
  return invoke<{ members: ExactGroupMember[] | PerceptualGroupMember[] }>(
    "get_group_members",
    { args: { groupId, mode } },
  );
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
