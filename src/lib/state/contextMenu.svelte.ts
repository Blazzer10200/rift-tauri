// App-wide right-click menu. Components open bespoke menus via
// contextMenu.open(); handleGlobalContextMenu() (wired in +layout.svelte)
// suppresses the stock WebView2 menu everywhere and builds default items
// for edit fields, text selections, links, and code blocks.
import {
  BookPlus,
  ClipboardPaste,
  Copy,
  ExternalLink,
  Link as LinkIcon,
  Scissors,
  SpellCheck,
  SquareCode,
  TextSelect,
} from "@lucide/svelte";
import { addPersonalWord, correctText, correctWord, warmAutocorrectDictionary } from "$lib/utils/autocorrect";

type CtxIcon = typeof Copy;

export type CtxMenuItem =
  | {
      kind?: "item";
      label: string;
      icon?: CtxIcon;
      disabled?: boolean;
      action: () => void | Promise<void>;
    }
  | { kind: "divider" };

class ContextMenuState {
  current = $state<{ x: number; y: number; items: CtxMenuItem[] } | null>(null);

  open(x: number, y: number, items: CtxMenuItem[]) {
    this.current = { x, y, items };
  }
  close() {
    this.current = null;
  }
}

export const contextMenu = new ContextMenuState();

export async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch (e) {
    console.warn("copy failed", e);
  }
}

type EditField = HTMLInputElement | HTMLTextAreaElement;

function editableFrom(target: Element | null): EditField | null {
  const el = target?.closest?.("input, textarea") as EditField | null;
  if (!el) return null;
  if (el instanceof HTMLInputElement) {
    // Only text-like inputs have a selectable value.
    const t = el.type;
    if (!["text", "search", "url", "tel", "password", "email", "number"].includes(t)) return null;
  }
  return el;
}

function fieldSelection(el: EditField): string {
  const s = el.selectionStart ?? 0;
  const e = el.selectionEnd ?? 0;
  return el.value.slice(s, e);
}

/** Replace the field's current selection and fire `input` so bind:value sees it. */
function replaceFieldSelection(el: EditField, text: string) {
  el.focus();
  const s = el.selectionStart ?? el.value.length;
  const e = el.selectionEnd ?? el.value.length;
  el.setRangeText(text, s, e, "end");
  el.dispatchEvent(new Event("input", { bubbles: true }));
}

async function cutField(el: EditField) {
  const sel = fieldSelection(el);
  if (!sel) return;
  await copyText(sel);
  replaceFieldSelection(el, "");
}

/** Correct the field's selection if one exists, else the whole value. Replaces
 *  via setRangeText so bind:value updates and the change stays undoable. */
async function autoCorrectField(el: EditField) {
  const selStart = el.selectionStart ?? 0;
  const selEnd = el.selectionEnd ?? 0;
  const hasSel = selStart !== selEnd;
  const s = hasSel ? selStart : 0;
  const e = hasSel ? selEnd : el.value.length;
  const src = el.value.slice(s, e);
  // An explicit whole-text pass can wait for the optional fuzzy dictionary;
  // typing-time exact corrections never wait on this chunk.
  await warmAutocorrectDictionary().catch(() => {});
  const fixed = correctText(src);
  if (fixed === src) return;
  el.focus();
  el.setRangeText(fixed, s, e, "end");
  el.dispatchEvent(new Event("input", { bubbles: true }));
}

async function pasteField(el: EditField) {
  let text = "";
  try {
    const { readText } = await import("@tauri-apps/plugin-clipboard-manager");
    text = (await readText()) ?? "";
  } catch (e) {
    console.warn("paste failed", e);
    return;
  }
  if (!text) return;
  replaceFieldSelection(el, text);
}

/** The word under the caret (or the selection, when it's exactly one word). */
function wordAtCaret(el: EditField): string | null {
  const sel = fieldSelection(el).trim();
  if (sel) return /^[A-Za-z][A-Za-z']*$/.test(sel) ? sel : null;
  const pos = el.selectionStart ?? 0;
  const left = /[A-Za-z']*$/.exec(el.value.slice(0, pos))?.[0] ?? "";
  const right = /^[A-Za-z']*/.exec(el.value.slice(pos))?.[0] ?? "";
  const w = left + right;
  return /^[A-Za-z][A-Za-z']*$/.test(w) ? w : null;
}

function editFieldItems(el: EditField): CtxMenuItem[] {
  const sel = fieldSelection(el);
  const ro = el.readOnly || el.disabled;
  // Offer "Add to dictionary" only for a word autocorrect would actually touch.
  const dictWord = wordAtCaret(el);
  const teachable = dictWord && dictWord.length >= 3 && correctWord(dictWord) !== null ? dictWord : null;
  return [
    { label: "Cut", icon: Scissors, disabled: !sel || ro, action: () => cutField(el) },
    { label: "Copy", icon: Copy, disabled: !sel, action: () => copyText(sel) },
    { label: "Paste", icon: ClipboardPaste, disabled: ro, action: () => pasteField(el) },
    { kind: "divider" },
    {
      label: "Select all",
      icon: TextSelect,
      disabled: !el.value,
      action: () => {
        el.focus();
        el.select();
      },
    },
    { kind: "divider" },
    {
      label: "Auto-correct",
      icon: SpellCheck,
      disabled: !el.value || ro,
      action: () => autoCorrectField(el),
    },
    ...(teachable
      ? [
          {
            label: `Add "${teachable}" to dictionary`,
            icon: BookPlus,
            action: () => addPersonalWord(teachable),
          } satisfies CtxMenuItem,
        ]
      : []),
  ];
}

/** Document-level fallback. Runs after component handlers — anything that
 *  preventDefault()ed already owns this right-click. */
export function handleGlobalContextMenu(e: MouseEvent) {
  if (e.defaultPrevented) return;
  // Dev escape hatch: Shift+right-click keeps the native menu (Inspect).
  if (import.meta.env.DEV && e.shiftKey) return;
  e.preventDefault();

  const target = e.target as Element | null;

  const field = editableFrom(target);
  if (field) {
    contextMenu.open(e.clientX, e.clientY, editFieldItems(field));
    return;
  }

  const items: CtxMenuItem[] = [];
  const sel = window.getSelection();
  const selText = sel && !sel.isCollapsed ? sel.toString() : "";
  if (selText) items.push({ label: "Copy", icon: Copy, action: () => copyText(selText) });

  const pre = target?.closest?.("pre");
  if (pre) {
    const code = (pre.querySelector("code")?.textContent ?? pre.textContent ?? "").trimEnd();
    if (code) items.push({ label: "Copy code", icon: SquareCode, action: () => copyText(code) });
  }

  const link = target?.closest?.("a[href]") as HTMLAnchorElement | null;
  if (link?.href && /^https?:/i.test(link.href)) {
    if (items.length) items.push({ kind: "divider" });
    items.push({
      label: "Open link in browser",
      icon: ExternalLink,
      action: async () => {
        const { openUrl } = await import("@tauri-apps/plugin-opener");
        await openUrl(link.href);
      },
    });
    items.push({ label: "Copy link address", icon: LinkIcon, action: () => copyText(link.href) });
  }

  if (items.length) contextMenu.open(e.clientX, e.clientY, items);
}
