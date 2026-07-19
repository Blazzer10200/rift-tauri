// #98.1: slash commands the installed CLI itself reports — captured from the
// turn-opening `system/init` frame's `slash_commands[]` (streaming.ts). This
// is the runtime source the disk scan can't cover: CLI builtins + bundled
// plugin skills (`/code-review` etc.) ship inside the CLI install and follow
// its updates automatically. Persisted so the composer menu is populated from
// app start (last-known set, refreshed on every init frame).
const LS_KEY = "rift.cliSlashCommands.v1";

// Same shape skills_catalog.rs::valid_name accepts — a name that can't be
// typed as `/name` is garbage, drop it.
const NAME_RE = /^[a-zA-Z0-9][\w:.-]{0,63}$/;

function loadStored(): string[] {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (!raw) return [];
    const v: unknown = JSON.parse(raw);
    return Array.isArray(v) ? v.filter((s): s is string => typeof s === "string") : [];
  } catch {
    return []; // corrupt/blocked storage → empty menu section until next init
  }
}

/** Validate + normalize a raw init-frame array into sorted unique names. */
export function normalizeCliCommands(raw: unknown): string[] {
  if (!Array.isArray(raw)) return [];
  return [...new Set(
    raw
      .filter((s): s is string => typeof s === "string")
      .map((s) => (s.startsWith("/") ? s.slice(1) : s))
      .filter((s) => NAME_RE.test(s)),
  )].sort();
}

class CliCommands {
  names = $state<string[]>(loadStored());

  /** Feed the init frame's `slash_commands` field (unknown-shaped — older
   *  CLIs omit it entirely, in which case the stored set stands). */
  setFromInit(raw: unknown) {
    const cleaned = normalizeCliCommands(raw);
    if (cleaned.length === 0) return;
    if (cleaned.length === this.names.length && cleaned.every((n, i) => n === this.names[i])) return;
    this.names = cleaned;
    try {
      localStorage.setItem(LS_KEY, JSON.stringify(cleaned));
    } catch {
      // Storage full/blocked — menu still refreshes from the next init frame.
    }
  }
}

export const cliCommands = new CliCommands();
