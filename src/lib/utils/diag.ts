import { invoke } from "@tauri-apps/api/core";

/**
 * #248: report a frontend-side failure to the backend DiagBus so it shows
 * up in the Diagnostics panel + activity feed. Always call alongside the
 * local `console.error` — devtools logging still helps; this just makes the
 * failure visible without them.
 *
 * `label` is a short slug ("connect", "auto-reconnect", "wire") used as the
 * stage prefix in the emitted message. Caller should pass the same string
 * shape as the existing `console.error("X failed", e)` arg.
 *
 * Fire-and-forget — if the IPC itself fails (Tauri runtime gone) we silently
 * drop. The local console.error is the durable record.
 */
export function reportFrontendError(label: string, err: unknown): void {
  const msg = err instanceof Error ? err.message : String(err);
  void invoke("diag_log_frontend_error", { label, message: msg }).catch(() => {});
}
