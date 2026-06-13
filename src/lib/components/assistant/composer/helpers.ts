// Pure helpers extracted from Composer.svelte (#20 C1 — composer-split.md).
// Zero DOM-state/store deps beyond the args passed in; unit-tested in helpers.test.ts.

export function fmtClock(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

// Light fuzzy match with three tiers: literal substring in basename,
// fuzzy match in basename, fuzzy match anywhere. `Comp` against
// `lib/foo/Composer.svelte` should beat `compress.lua` because the match
// starts at the basename.
export function fuzzyScore(path: string, query: string): number | null {
  if (query.length === 0) return 0;
  const p = path.toLowerCase();
  const q = query.toLowerCase();
  const basenameStart = p.lastIndexOf("/") + 1;
  const basename = p.slice(basenameStart);
  const subIdx = basename.indexOf(q);
  if (subIdx !== -1) return 1000 - subIdx;
  let pi = 0;
  let firstHit = -1;
  for (const ch of q) {
    const found = basename.indexOf(ch, pi);
    if (found === -1) { firstHit = -1; pi = -1; break; }
    if (firstHit === -1) firstHit = found;
    pi = found + 1;
  }
  if (pi !== -1 && firstHit >= 0) return 500 - firstHit;
  pi = 0; firstHit = -1;
  for (const ch of q) {
    const found = p.indexOf(ch, pi);
    if (found === -1) return null;
    if (firstHit === -1) firstHit = found;
    pi = found + 1;
  }
  return -firstHit;
}

// Pointer-drag slider math: map clientX → nearest stop index on a track rect.
export function effortIdxFromX(
  clientX: number,
  track: { left: number; width: number },
  stopCount: number,
): number {
  const frac = Math.min(1, Math.max(0, (clientX - track.left) / track.width));
  return Math.round(frac * Math.max(0, stopCount - 1));
}

export function bytesToBase64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf);
  let bin = "";
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) {
    bin += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(bin);
}

export function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

export function isFileDrag(e: DragEvent): boolean {
  const types = e.dataTransfer?.types;
  if (!types) return false;
  for (let i = 0; i < types.length; i++) if (types[i] === "Files") return true;
  return false;
}

const ATTACH_MAX_BYTES = 20 * 1024 * 1024;

export type AttachResult = {
  attached: number;
  nonImage: string[]; // dropped/pasted but not an image
  tooLarge: string[]; // image over the 20 MB cap
  failed: string[]; // couldn't be read
  limitHit: boolean; // per-turn total-size cap reached
};

/** Stage image files as attachments via the supplied `add` callback (kept as a
 *  param so this stays store-free + unit-testable). Non-image / oversized /
 *  unreadable files are collected, never silently dropped. Shared by the
 *  composer's paste/drop/file-pick paths and the window-level drop guard. */
export async function attachImageFiles(
  files: Iterable<File>,
  add: (att: { mime: string; dataBase64: string; sizeBytes: number }) => boolean,
): Promise<AttachResult> {
  const r: AttachResult = { attached: 0, nonImage: [], tooLarge: [], failed: [], limitHit: false };
  for (const file of Array.from(files)) {
    if (!file.type.startsWith("image/")) { r.nonImage.push(file.name || "file"); continue; }
    if (file.size > ATTACH_MAX_BYTES) { r.tooLarge.push(file.name || "image"); continue; }
    try {
      const dataBase64 = bytesToBase64(await file.arrayBuffer());
      if (add({ mime: file.type || "image/png", dataBase64, sizeBytes: file.size })) r.attached++;
      else r.limitHit = true;
    } catch {
      r.failed.push(file.name || "file");
    }
  }
  return r;
}

/** One-line summary of what an attach attempt rejected — null when everything
 *  attached cleanly. Drives a single warn toast instead of a swallowed skip. */
export function summarizeAttach(r: AttachResult): string | null {
  const parts: string[] = [];
  if (r.nonImage.length) {
    parts.push(r.nonImage.length === 1
      ? `${r.nonImage[0]} isn't an image — only images attach (the assistant can read workspace files directly)`
      : `${r.nonImage.length} non-image files skipped — only images attach`);
  }
  if (r.tooLarge.length) parts.push(`${r.tooLarge.length === 1 ? r.tooLarge[0] : `${r.tooLarge.length} images`} over the 20 MB cap`);
  if (r.failed.length) parts.push(`${r.failed.length} file(s) couldn't be read`);
  if (r.limitHit) parts.push("attachment limit reached (20 MB per turn)");
  return parts.length ? parts.join(" · ") : null;
}
