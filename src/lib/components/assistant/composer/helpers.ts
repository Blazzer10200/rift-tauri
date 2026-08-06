// Pure helpers extracted from Composer.svelte; see docs/ARCHITECTURE.md#frontend-map.
// Zero DOM-state/store deps beyond the args passed in; unit-tested in helpers.test.ts.

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

// Slash-menu ranking — three tiers so `/de` puts `design-sync` (prefix) above
// `model` (subsequence via …odel? no — null) and `co` puts `copy`/`cost`
// (prefix) above `openincli` (substring/subsequence). Prefix > substring >
// subsequence; ties break toward shorter names / earlier hits. null = no match.
export function slashScore(name: string, query: string): number | null {
  if (query.length === 0) return 0;
  const n = name.toLowerCase();
  const q = query.toLowerCase();
  if (n.startsWith(q)) return 3000 - n.length;
  const idx = n.indexOf(q);
  if (idx !== -1) return 2000 - idx * 10 - n.length;
  let pi = 0;
  let firstHit = -1;
  for (const ch of q) {
    const found = n.indexOf(ch, pi);
    if (found === -1) return null;
    if (firstHit === -1) firstHit = found;
    pi = found + 1;
  }
  // Penalize late starts and spread-out matches so tight clusters win.
  const spread = pi - firstHit - q.length;
  return 1000 - firstHit * 10 - spread * 2 - n.length;
}

/** Palette search keeps command names dominant, but also finds a command by
 * its purpose, argument hint, or source. This prevents large skill catalogs
 * from feeling opaque when users remember the job rather than the exact name. */
export function slashCatalogScore(
  name: string,
  description: string,
  metadata: string,
  query: string,
): number | null {
  const nameScore = slashScore(name, query);
  if (nameScore !== null) return 4000 + nameScore;
  const q = query.toLowerCase();
  const descIndex = description.toLowerCase().indexOf(q);
  if (descIndex !== -1) return 1500 - descIndex;
  const metaIndex = metadata.toLowerCase().indexOf(q);
  if (metaIndex !== -1) return 700 - metaIndex;
  return null;
}

// Which chars of `name` the query hit — drives per-char highlight in the menu.
// Mirrors slashScore's tiers (contiguous run when substring-matched, else
// greedy left-to-right subsequence); empty query or no match → no highlights.
export function slashMatchSegments(
  name: string,
  query: string,
): { text: string; hit: boolean }[] {
  const whole = [{ text: name, hit: false }];
  if (query.length === 0) return whole;
  const n = name.toLowerCase();
  const q = query.toLowerCase();
  const hits = new Array<boolean>(name.length).fill(false);
  const idx = n.indexOf(q);
  if (idx !== -1) {
    for (let i = idx; i < idx + q.length; i++) hits[i] = true;
  } else {
    let pi = 0;
    for (const ch of q) {
      const found = n.indexOf(ch, pi);
      if (found === -1) return whole;
      hits[found] = true;
      pi = found + 1;
    }
  }
  const segs: { text: string; hit: boolean }[] = [];
  for (let i = 0; i < name.length; i++) {
    const last = segs[segs.length - 1];
    if (last && last.hit === hits[i]) last.text += name[i];
    else segs.push({ text: name[i], hit: hits[i] });
  }
  return segs;
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
// Per-file cap on inlined text. ~256 KB ≈ 64K tokens worst case — generous for
// a log/config/output, and the 1 MiB per-turn cap (attachments.ts) keeps the
// total well under the backend 2 MiB prompt ceiling. Larger files truncate
// with a marker rather than being rejected.
const TEXT_FILE_MAX_BYTES = 256 * 1024;

export type AttachResult = {
  attached: number;
  nonImage: string[]; // dropped/pasted but not an image (image-only path)
  tooLarge: string[]; // image over the 20 MB cap
  failed: string[]; // couldn't be read
  limitHit: boolean; // per-turn total-size cap reached
};

// Files we won't inline as text — a non-image binary the assistant can't use as
// prose. Detected by extension since File.type is unreliable for code/config.
const BINARY_EXT = new Set([
  "exe", "dll", "so", "dylib", "bin", "o", "a", "lib", "obj",
  "zip", "tar", "gz", "7z", "rar", "bz2", "xz",
  "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
  "mp3", "mp4", "wav", "mov", "avi", "mkv", "webm", "ogg", "flac",
  "ttf", "otf", "woff", "woff2", "eot",
  "wasm", "class", "pyc", "jar", "node",
]);

function looksBinary(name: string, type: string): boolean {
  if (type && !type.startsWith("text/") && !TEXTUAL_MIME.has(type)) {
    // an explicit non-textual MIME (e.g. application/octet-stream) → binary
    if (type.startsWith("image/") || type.startsWith("audio/") || type.startsWith("video/")) return true;
  }
  const ext = name.includes(".") ? name.slice(name.lastIndexOf(".") + 1).toLowerCase() : "";
  return BINARY_EXT.has(ext);
}

// MIMEs that are textual despite not starting with "text/".
const TEXTUAL_MIME = new Set([
  "application/json", "application/xml", "application/javascript",
  "application/x-yaml", "application/toml", "application/x-sh",
]);

export type TextAttachResult = {
  attached: number;
  binary: string[]; // skipped — a non-text binary
  truncated: string[]; // inlined but clipped at the per-file cap
  failed: string[]; // couldn't be read as text
  limitHit: boolean; // per-turn text cap reached
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

/** Stage text files as inline-prompt attachments via the supplied `add`
 *  callback (store-free + unit-testable, like attachImageFiles). Reads each as
 *  UTF-8, truncates at the per-file cap with a marker, and skips non-text
 *  binaries. Caller routes images here AFTER attachImageFiles has claimed them;
 *  this path handles everything that isn't an image. */
export async function attachTextFiles(
  files: Iterable<File>,
  add: (att: { name: string; text: string; sizeBytes: number; truncated: boolean }) => boolean,
): Promise<TextAttachResult> {
  const r: TextAttachResult = { attached: 0, binary: [], truncated: [], failed: [], limitHit: false };
  for (const file of Array.from(files)) {
    const name = file.name || "file";
    if (file.type.startsWith("image/")) continue; // images go through attachImageFiles
    if (looksBinary(name, file.type)) { r.binary.push(name); continue; }
    try {
      const raw = await file.text();
      const truncated = raw.length > TEXT_FILE_MAX_BYTES;
      const text = truncated
        ? raw.slice(0, TEXT_FILE_MAX_BYTES) + `\n\n…[truncated — ${fmtSize(file.size)} file clipped at ${fmtSize(TEXT_FILE_MAX_BYTES)}]`
        : raw;
      if (add({ name, text, sizeBytes: file.size, truncated })) {
        r.attached++;
        if (truncated) r.truncated.push(name);
      } else {
        r.limitHit = true;
      }
    } catch {
      r.failed.push(name);
    }
  }
  return r;
}

/** One-line summary of what an image-attach attempt rejected — null when
 *  everything attached cleanly. nonImage is no longer flagged here: non-image
 *  files route to attachTextFiles, summarized separately. */
export function summarizeAttach(r: AttachResult): string | null {
  const parts: string[] = [];
  if (r.tooLarge.length) parts.push(`${r.tooLarge.length === 1 ? r.tooLarge[0] : `${r.tooLarge.length} images`} over the 20 MB cap`);
  if (r.failed.length) parts.push(`${r.failed.length} file(s) couldn't be read`);
  if (r.limitHit) parts.push("attachment limit reached (20 MB per turn)");
  return parts.length ? parts.join(" · ") : null;
}

/** Display text for a queued-message chip: the message text, else an
 *  attachment marker ("2 images · 1 file") so an attachment-only item never
 *  renders as a blank chip. */
export function queueChipLabel(q: { text: string; images?: unknown[]; textFiles?: unknown[] }): string {
  if (q.text) return q.text;
  const parts: string[] = [];
  const img = q.images?.length ?? 0;
  const txt = q.textFiles?.length ?? 0;
  if (img) parts.push(`${img} image${img === 1 ? "" : "s"}`);
  if (txt) parts.push(`${txt} file${txt === 1 ? "" : "s"}`);
  return parts.join(" · ") || "(empty)";
}

/** One-line summary of a text-attach attempt — null when all clean. */
export function summarizeTextAttach(r: TextAttachResult): string | null {
  const parts: string[] = [];
  if (r.binary.length) {
    parts.push(r.binary.length === 1
      ? `${r.binary[0]} is a binary file — can't attach as text`
      : `${r.binary.length} binary files skipped — only text/images attach`);
  }
  if (r.truncated.length) parts.push(`${r.truncated.length === 1 ? r.truncated[0] : `${r.truncated.length} files`} truncated to fit`);
  if (r.failed.length) parts.push(`${r.failed.length} file(s) couldn't be read as text`);
  if (r.limitHit) parts.push("text attachment limit reached (1 MB per turn)");
  return parts.length ? parts.join(" · ") : null;
}
