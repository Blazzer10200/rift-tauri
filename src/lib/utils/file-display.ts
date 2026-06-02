import { Folder, FileCode, File } from "lucide-svelte";

/** Shared file-row formatters/helpers extracted from LocalPane + RemotePane.
 *  Pure functions only — entity-specific logic (rowStatus, parentOf,
 *  drag/drop handlers) stays in each pane since paths differ (Windows
 *  backslash vs POSIX forward-slash) and conflict matching differs
 *  (local_path vs remote_path). */

/** Shared filter-chip mode for the Files browser panes. */
export type FileFilter = "all" | "lua" | "conflicts" | "modified";

export function fmtSize(n: number, isDir: boolean): string {
  if (isDir) return "";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

const CODE_EXT_RE = /\.(lua|ts|js|py|rs|cs|go|rb|json|yml|yaml|toml)$/i;

export function pickIcon(name: string, isDir: boolean) {
  if (isDir) return Folder;
  if (CODE_EXT_RE.test(name)) return FileCode;
  return File;
}

export function isEditableTarget(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable);
}

/** Clamp a context-menu origin so the menu doesn't overflow the viewport.
 *  Used by both LocalPane and RemotePane with identical constants. */
export function clampMenuPos(clientX: number, clientY: number): { x: number; y: number } {
  const MENU_W = 240;
  const MENU_H = 320;
  const PAD = 8;
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  let x = clientX;
  let y = clientY;
  if (x + MENU_W > vw) x = Math.max(PAD, vw - MENU_W - PAD);
  if (y + MENU_H > vh) y = Math.max(PAD, vh - MENU_H - PAD);
  return { x, y };
}
