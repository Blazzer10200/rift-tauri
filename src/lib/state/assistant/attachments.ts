// M4 (per docs/design/assistant-svelte-split.md) — per-tab attachment ops
// lifted out of `src/lib/state/assistant.svelte.ts` as free fns operating on
// a tab ref. Per brief: TabState fields STAY on TabState; only the *logic*
// moves. Re-assignment to `tab.attachments` mutates the Svelte 5 $state field
// directly — reactivity is preserved.

type Attachment = {
  id: string;
  mime: string;
  dataBase64: string;
  sizeBytes: number;
};

// A staged text file: its contents are inlined into the prompt as a fenced
// block at send (send.ts), NOT routed through the binary `attachments` param
// (that's the vision API). For files the assistant can't otherwise read — a
// log/config/output outside the workspace; workspace files it reads via MCP.
export type TextAttachment = {
  id: string;
  name: string;       // basename, shown on the chip + as the fence header
  text: string;       // UTF-8 contents (already truncated to the per-file cap)
  sizeBytes: number;  // original byte size (pre-truncation) for the chip label
  truncated: boolean; // contents were clipped at the per-file cap
};

/** Bag of attachment state. Shape-matched against TabState; no class import. */
type AttachmentHost = { attachments: Attachment[]; textAttachments: TextAttachment[] };

const CAP_BYTES = 20 * 1024 * 1024;
// Per-turn ceiling on total inlined text. Well under the backend 2 MiB
// PROMPT_BYTES_CAP (turn.rs) so inlined files + the typed prompt + system
// context all fit the single stdin write.
const TEXT_CAP_BYTES = 1 * 1024 * 1024;

/** Stage a binary attachment on the given tab. 20 MiB cumulative cap mirrors
 *  the backend guard. Returns false on overflow. */
export function addAttachment(
  tab: AttachmentHost,
  att: { mime: string; dataBase64: string; sizeBytes: number },
): boolean {
  const current = tab.attachments.reduce((s, a) => s + a.sizeBytes, 0);
  if (current + att.sizeBytes > CAP_BYTES) return false;
  tab.attachments = [...tab.attachments, { id: crypto.randomUUID(), ...att }];
  return true;
}

export function removeAttachment(tab: AttachmentHost, id: string): void {
  tab.attachments = tab.attachments.filter((a) => a.id !== id);
}

export function clearAttachments(tab: AttachmentHost): void {
  tab.attachments = [];
}

/** Stage a text-file attachment on the given tab. 1 MiB cumulative cap on the
 *  inlined text (measured on `text.length`, which is char-count ≈ bytes for the
 *  ASCII-heavy logs/configs this targets). Returns false on overflow. */
export function addTextAttachment(
  tab: AttachmentHost,
  att: { name: string; text: string; sizeBytes: number; truncated: boolean },
): boolean {
  const current = tab.textAttachments.reduce((s, a) => s + a.text.length, 0);
  if (current + att.text.length > TEXT_CAP_BYTES) return false;
  tab.textAttachments = [...tab.textAttachments, { id: crypto.randomUUID(), ...att }];
  return true;
}

export function removeTextAttachment(tab: AttachmentHost, id: string): void {
  tab.textAttachments = tab.textAttachments.filter((a) => a.id !== id);
}

export function clearTextAttachments(tab: AttachmentHost): void {
  tab.textAttachments = [];
}
