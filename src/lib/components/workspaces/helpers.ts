// Shared display helpers for the workspace pages (Harness + Cost).

/** Per-model hue (app convention: sonnet=blue, opus=purple, haiku=teal). */
export function modelHue(name: string): number {
  if (name.includes("haiku")) return 175;
  if (name.includes("opus")) return 280;
  if (name.includes("sonnet")) return 225;
  return 163;
}

/** "claude-opus-4-8" → "opus 4.8" (lowercase, pill-style). */
export function shortModel(id: string): string {
  return id.replace(/^claude-/, "").replace(/-(\d)-(\d)$/, " $1.$2");
}
