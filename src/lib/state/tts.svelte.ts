// Text-to-speech state.
//
// Wraps the Rust `tts_*` commands and the `tts://audio` event stream. The
// assistant pipeline feeds streaming text through `feed(messageId, chunk)` —
// we buffer until a sentence terminator, dispatch each completed sentence as
// a separate synthesis request, and play the resulting MP3 chunks back-to-back
// via HTMLAudioElement. Per-message replay reuses the same pipeline.
//
// Cancellation: `cancel()` bumps the backend generation (drops any in-flight +
// queued synthesis) AND clears our local playback queue (anything already
// emitted but not yet played is dropped on the floor).

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type TtsConfig = {
  enabled: boolean;
  auto_speak: boolean;
  voice: string;
  rate: number;
  pitch: number;
  volume: number;
};

export type VoiceSummary = {
  name: string;
  short_name: string;
  locale: string;
  gender: string | null;
};

type QueueEntry = {
  requestId: string;
  messageId: string;
  text: string;
  url: string | null;
};

type AudioPayload = {
  request_id: string;
  audio_b64: string;
  mime: string;
};

type CancelledPayload = { request_id: string };
type ErrorPayload = { request_id: string; error: string };

const DEFAULT_VOICE = "Microsoft Server Speech Text to Speech Voice (en-US, AriaNeural)";

/** Split text into sentences. Returns `{ complete, rest }` where `rest` is the
 *  trailing incomplete fragment (no terminator + whitespace seen yet). */
export function splitSentences(text: string): { complete: string[]; rest: string } {
  const complete: string[] = [];
  let start = 0;
  // Match terminator + optional closing punctuation + whitespace.
  const re = /([.!?]+["')\]]?)(\s+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    const end = m.index + m[1].length;
    const piece = text.slice(start, end).trim();
    if (piece.length > 0) complete.push(piece);
    start = re.lastIndex;
  }
  return { complete, rest: text.slice(start) };
}

/** Split text into sentences for one-shot replay (no incomplete remainder). */
export function splitAllSentences(text: string): string[] {
  const { complete, rest } = splitSentences(text);
  const r = rest.trim();
  if (r.length > 0) complete.push(r);
  return complete;
}

function base64ToBlob(b64: string, mime: string): Blob {
  const bin = atob(b64);
  const len = bin.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) bytes[i] = bin.charCodeAt(i);
  return new Blob([bytes], { type: mime });
}

class TtsStore {
  config = $state<TtsConfig>({
    enabled: false,
    auto_speak: false,
    voice: "",
    rate: 0,
    pitch: 0,
    volume: 0,
  });
  configLoaded = $state(false);

  voices = $state<VoiceSummary[]>([]);
  voicesLoading = $state(false);
  voicesError = $state<string | null>(null);

  playing = $state(false);
  playingMessageId = $state<string | null>(null);
  lastError = $state<string | null>(null);

  // Per-message text buffer holding the trailing incomplete fragment.
  private buffers = new Map<string, string>();
  // Ordered queue of in-flight + ready-to-play chunks. Items arrive `url: null`
  // and are promoted once the backend emits the matching audio.
  private queue: QueueEntry[] = [];
  // Requests we've dispatched and are waiting on. Receiving an audio for an
  // id not in here = stale (post-cancel) and ignored.
  private pending = new Set<string>();
  private currentAudio: HTMLAudioElement | null = null;
  private currentUrl: string | null = null;
  private unlistens: UnlistenFn[] = [];
  private initStarted = false;

  async init() {
    if (this.initStarted) return;
    this.initStarted = true;
    try {
      const cfg = await invoke<TtsConfig>("tts_get_config");
      this.config = { ...this.config, ...cfg };
      this.configLoaded = true;
    } catch (e) {
      console.warn("[tts] load config failed:", e);
      this.configLoaded = true;
    }
    this.unlistens.push(
      await listen<AudioPayload>("tts://audio", (e) => this.onAudio(e.payload)),
      await listen<CancelledPayload>("tts://cancelled", (e) => this.onCancelled(e.payload)),
      await listen<ErrorPayload>("tts://error", (e) => this.onErrorEvent(e.payload)),
    );
  }

  async dispose() {
    for (const u of this.unlistens) u();
    this.unlistens = [];
    this.initStarted = false;
    await this.cancel();
  }

  /** Update settings (and persist). Mutating `enabled` off cancels in-flight. */
  async setConfig(patch: Partial<TtsConfig>) {
    const next: TtsConfig = { ...this.config, ...patch };
    this.config = next;
    try {
      await invoke("tts_set_config", { config: next });
    } catch (e) {
      this.lastError = `Save settings failed: ${e}`;
    }
    if (patch.enabled === false) {
      await this.cancel();
    }
  }

  /** Lazy-load the Edge voice list — first call fetches over HTTP. */
  async loadVoices(force = false) {
    if (this.voicesLoading) return;
    if (this.voices.length > 0 && !force) return;
    this.voicesLoading = true;
    this.voicesError = null;
    try {
      this.voices = await invoke<VoiceSummary[]>("tts_list_voices");
    } catch (e) {
      this.voicesError = String(e);
    } finally {
      this.voicesLoading = false;
    }
  }

  /** Streaming hook — call with every text chunk the assistant appends. */
  feed(messageId: string, chunk: string) {
    if (!this.config.enabled || !this.config.auto_speak) return;
    if (!chunk) return;
    const carried = this.buffers.get(messageId) ?? "";
    const merged = carried + chunk;
    const { complete, rest } = splitSentences(merged);
    this.buffers.set(messageId, rest);
    for (const s of complete) this.enqueue(messageId, s);
  }

  /** Flush the trailing fragment for `messageId` at end-of-turn. */
  flush(messageId: string) {
    if (!this.config.enabled || !this.config.auto_speak) return;
    const carried = this.buffers.get(messageId);
    this.buffers.delete(messageId);
    if (carried && carried.trim()) this.enqueue(messageId, carried.trim());
  }

  /** Replay full message text on-demand (speaker icon click). */
  async replay(messageId: string, text: string) {
    if (!this.config.enabled) {
      await this.setConfig({ enabled: true });
    }
    await this.cancel();
    const sentences = splitAllSentences(text);
    for (const s of sentences) this.enqueue(messageId, s);
  }

  /** Synthesise + play a one-shot blurb (e.g. settings "Test voice" button). */
  async testVoice(text = "This is a test of the current voice.") {
    if (!this.config.enabled) {
      await this.setConfig({ enabled: true });
    }
    await this.cancel();
    this.enqueue("__test__", text);
  }

  async cancel() {
    this.queue = [];
    this.pending.clear();
    this.buffers.clear();
    if (this.currentAudio) {
      try {
        this.currentAudio.pause();
        this.currentAudio.src = "";
      } catch {
        /* detached */
      }
    }
    if (this.currentUrl) {
      URL.revokeObjectURL(this.currentUrl);
      this.currentUrl = null;
    }
    this.currentAudio = null;
    this.playing = false;
    this.playingMessageId = null;
    try {
      await invoke("tts_cancel");
    } catch {
      /* worker may be gone — local cancel already applied */
    }
  }

  private enqueue(messageId: string, text: string) {
    if (!text || text.length === 0) return;
    const requestId =
      typeof crypto !== "undefined" && crypto.randomUUID
        ? crypto.randomUUID()
        : `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    this.pending.add(requestId);
    this.queue.push({ requestId, messageId, text, url: null });
    void invoke("tts_speak", { text, requestId }).catch((e) => {
      console.warn("[tts] tts_speak failed:", e);
      this.pending.delete(requestId);
      this.queue = this.queue.filter((q) => q.requestId !== requestId);
      this.drain();
    });
  }

  private onAudio(payload: AudioPayload) {
    if (!this.pending.has(payload.request_id)) return;
    this.pending.delete(payload.request_id);
    const slot = this.queue.find((q) => q.requestId === payload.request_id);
    if (!slot) return;
    const blob = base64ToBlob(payload.audio_b64, payload.mime || "audio/mpeg");
    slot.url = URL.createObjectURL(blob);
    this.drain();
  }

  private onCancelled(payload: CancelledPayload) {
    this.pending.delete(payload.request_id);
    const slot = this.queue.find((q) => q.requestId === payload.request_id);
    if (slot && slot.url) {
      URL.revokeObjectURL(slot.url);
    }
    this.queue = this.queue.filter((q) => q.requestId !== payload.request_id);
    this.drain();
  }

  private onErrorEvent(payload: ErrorPayload) {
    this.lastError = payload.error;
    this.pending.delete(payload.request_id);
    this.queue = this.queue.filter((q) => q.requestId !== payload.request_id);
    this.drain();
  }

  private drain() {
    if (this.currentAudio) return;
    const next = this.queue[0];
    if (!next) {
      this.playing = false;
      this.playingMessageId = null;
      return;
    }
    if (next.url === null) return; // waiting on backend
    this.queue.shift();
    this.currentUrl = next.url;
    this.playing = true;
    this.playingMessageId = next.messageId;
    const audio = new Audio(next.url);
    this.currentAudio = audio;
    const cleanup = () => {
      if (this.currentUrl === next.url) {
        URL.revokeObjectURL(next.url!);
        this.currentUrl = null;
      } else {
        URL.revokeObjectURL(next.url!);
      }
      this.currentAudio = null;
      this.drain();
    };
    audio.onended = cleanup;
    audio.onerror = () => {
      console.warn("[tts] audio playback error");
      cleanup();
    };
    audio.play().catch((e) => {
      console.warn("[tts] audio.play() rejected:", e);
      cleanup();
    });
  }
}

export const tts = new TtsStore();
export { DEFAULT_VOICE };
