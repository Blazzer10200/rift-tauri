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
