// M4 (per docs/design/assistant-svelte-split.md) — per-tab attachment ops
// lifted out of `src/lib/state/assistant.svelte.ts` as free fns operating on
// a tab ref. Per brief: TabState fields STAY on TabState; only the *logic*
// moves. Re-assignment to `tab.attachments` mutates the Svelte 5 $state field
// directly — reactivity is preserved.

export type Attachment = {
  id: string;
  mime: string;
  dataBase64: string;
  sizeBytes: number;
};

/** Bag of attachment state. Shape-matched against TabState; no class import. */
type AttachmentHost = { attachments: Attachment[] };

const CAP_BYTES = 20 * 1024 * 1024;

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
