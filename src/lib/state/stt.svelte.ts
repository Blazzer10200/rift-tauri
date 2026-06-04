// Speech-to-text state.
//
// Two engines, same public surface (composer wires `recording` /
// `transcribing` / `lastError` / `start` / `stop` / `cancel` / `consume`
// without caring which engine is active):
//
//   • engine="web_speech" (default, legacy) — WebView2's built-in
//     SpeechRecognition (Azure-backed when online). 100% in-browser, no
//     backend round-trips. Settings persisted via `stt_get/set_config`.
//
//   • engine="whisper" — Rust-side cpal mic capture, webrtc-vad gating,
//     whisper.cpp inference, optional Claude Haiku polish. Driven over
//     Tauri IPC (`stt_start_recording` / `stt_stop_recording`); partial +
//     final text arrives via `stt://partial` / `stt://final` events. The
//     same `composeDraft` path feeds them into the composer so the live-
//     typing UX matches Web Speech's interim/final segment behaviour.
//
// Switching engines mid-session is allowed — `setConfig({engine})` aborts
// any in-flight recording before the new engine takes over.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { assistant } from "./assistant.svelte";

export type SttEngine = "web_speech" | "whisper";

export type SttConfig = {
  enabled: boolean;
  language: string;
  append_to_draft: boolean;
  continuous: boolean;
  show_interim: boolean;
  engine: SttEngine;
  whisper_model: string;
  input_device: string | null;
  initial_prompt: string;
  vocab_text: string;
  cleanup_enabled: boolean;
  beam_size: number | null;
};

export type ModelInfo = {
  id: string;
  display_name: string;
  filename: string;
  approx_size_bytes: number;
  on_disk_bytes: number | null;
  downloaded: boolean;
  path: string | null;
  sha256: string | null;
};

export type DownloadProgress = {
  model: string;
  downloaded: number;
  total: number;
  phase: "start" | "progress" | "done" | "error";
  message: string | null;
};

export type SttState =
  | "idle"
  | "loading_model"
  | "recording"
  | "transcribing";

type SpeechRecognitionAlternative = { transcript: string; confidence: number };
type SpeechRecognitionResult = {
  isFinal: boolean;
  length: number;
  [index: number]: SpeechRecognitionAlternative;
};
type SpeechRecognitionResultList = {
  length: number;
  [index: number]: SpeechRecognitionResult;
};
type SpeechRecognitionEvent = { resultIndex: number; results: SpeechRecognitionResultList };
type SpeechRecognitionErrorEvent = { error: string; message?: string };

type SpeechRecognitionInstance = {
  lang: string;
  continuous: boolean;
  interimResults: boolean;
  maxAlternatives: number;
  onresult: ((e: SpeechRecognitionEvent) => void) | null;
  onerror: ((e: SpeechRecognitionErrorEvent) => void) | null;
  onend: ((e: Event) => void) | null;
  onstart: ((e: Event) => void) | null;
  start: () => void;
  stop: () => void;
  abort: () => void;
};

type SpeechRecognitionCtor = new () => SpeechRecognitionInstance;

function getSRCtor(): SpeechRecognitionCtor | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as {
    SpeechRecognition?: SpeechRecognitionCtor;
    webkitSpeechRecognition?: SpeechRecognitionCtor;
  };
  return w.SpeechRecognition ?? w.webkitSpeechRecognition ?? null;
}

class SttStore {
  config = $state<SttConfig>({
    enabled: false,
    language: "en-US",
    append_to_draft: true,
    continuous: true,
    show_interim: true,
    engine: "web_speech",
    whisper_model: "large-v3-turbo-q5_0",
    input_device: null,
    initial_prompt: "",
    vocab_text: "",
    cleanup_enabled: true,
    beam_size: null,
  });
  configLoaded = $state(false);

  /** Web Speech API availability in the current WebView. */
  supported = $state(false);
  /** True if the Whisper Rust backend was compiled in (default: false until
   *  the user installs LLVM + rebuilds with `--features whisper-rs`). */
  backendAvailable = $state(false);
  recording = $state(false);
  /** True while we've stopped but final results may still arrive. */
  transcribing = $state(false);
  lastError = $state<string | null>(null);
  lastTranscript = $state<string>("");
  /** Backend lifecycle state, mirrored from `stt://state` events. */
  currentState = $state<SttState>("idle");

  // Whisper-specific reactive state.
  models = $state<ModelInfo[]>([]);
  modelDownloads = $state<Record<string, DownloadProgress>>({});
  inputDevices = $state<string[]>([]);
  /** True once stt_start_recording has been invoked but before recording=true arrives via event. */
  whisperStartInvoked = $state(false);

  // --- Private fields ---
  private recognition: SpeechRecognitionInstance | null = null;
  private initStarted = false;
  private baseDraft = "";
  private finalText = "";
  private consumed = false;
  // #175: explicit intent flag separates "session ended naturally" (commit
  // the finalText draft) from "user pressed Cancel" (drop the draft). Prior
  // code overloaded `this.recognition === null` for both roles, which made
  // the onEnd branch fragile to future edits that null the handle for
  // unrelated reasons.
  private cancelRequested = false;
  private transcribeTimer: ReturnType<typeof setTimeout> | null = null;
  private restartToken = 0;
  private unlisten: UnlistenFn[] = [];

  async init() {
    if (this.initStarted) return;
    this.initStarted = true;
    this.supported = getSRCtor() !== null;

    try {
      this.config = await invoke<SttConfig>("stt_get_config");
      this.configLoaded = true;
    } catch {
      // Non-fatal — keep default config; UI shows defaults until next session.
      this.configLoaded = true;
    }

    try {
      this.backendAvailable = await invoke<boolean>("stt_backend_available");
    } catch {
      this.backendAvailable = false;
    }

    // Subscribe to backend events. Always subscribe — engine can be flipped
    // at runtime so we want listeners ready before the first start().
    try {
      this.unlisten.push(
        await listen<{ text: string }>("stt://partial", (ev) => this.onBackendPartial(ev.payload.text)),
      );
      this.unlisten.push(
        await listen<{ text: string; raw: string; cleaned: boolean }>("stt://final", (ev) =>
          this.onBackendFinal(ev.payload.text),
        ),
      );
      this.unlisten.push(
        await listen<{ state: SttState; message: string | null }>("stt://state", (ev) => {
          this.currentState = ev.payload.state;
          if (ev.payload.state === "transcribing") this.transcribing = true;
          if (ev.payload.state === "recording") {
            this.recording = true;
            this.transcribing = false;
          }
          if (ev.payload.state === "idle") {
            this.recording = false;
            this.transcribing = false;
          }
        }),
      );
      this.unlisten.push(
        await listen<{ code: string; message: string }>("stt://error", (ev) => {
          this.lastError = ev.payload.message;
        }),
      );
      this.unlisten.push(
        await listen<DownloadProgress>("stt://download_progress", (ev) => {
          this.modelDownloads = { ...this.modelDownloads, [ev.payload.model]: ev.payload };
          if (ev.payload.phase === "done") {
            void this.refreshModels();
          }
        }),
      );
    } catch {
      // Non-fatal — backend events won't arrive, but Web Speech path stays functional.
    }

    // Best-effort initial loads for the Settings panel — failures are non-fatal.
    void this.refreshModels();
    void this.refreshInputDevices();
  }

  async setConfig(patch: Partial<SttConfig>) {
    const prevEngine = this.config.engine;
    const next: SttConfig = { ...this.config, ...patch };
    this.config = next;
    try {
      await invoke("stt_set_config", { config: next });
    } catch (e) {
      this.lastError = `Save settings failed: ${e}`;
    }

    // Engine switch mid-recording — hard-cancel the in-flight session under
    // the OLD engine before letting the new one take over.
    if (patch.engine && patch.engine !== prevEngine && this.recording) {
      await this.cancel();
      return;
    }

    // Web Speech only: restart recogniser so language/continuous/interim
    // changes take effect mid-session. Whisper picks up config on next start().
    if (next.engine === "web_speech" && this.recording && this.recognition) {
      const token = ++this.restartToken;
      this.recognition.abort();
      this.recording = false;
      setTimeout(() => {
        if (token === this.restartToken) void this.start();
      }, 120);
    }
  }

  /** Composer-side hook: the current draft was just sent / cleared. */
  consume() {
    this.cancelRequested = true;
    if (this.config.engine === "whisper") {
      // Fire-and-forget — the backend's drop-on-stop preserves any in-flight
      // partials but stops emitting new ones.
      void invoke("stt_stop_recording").catch(() => {});
    } else if (this.recognition) {
      try {
        this.recognition.abort();
      } catch {
        /* recogniser may already be stopped */
      }
    }
    this.baseDraft = "";
    this.finalText = "";
    this.consumed = true;
    this.lastTranscript = "";
    this.recording = false;
    this.transcribing = false;
    this.recognition = null;
    this.clearTranscribeTimer();
  }

  /** Begin live recognition. Returns false if unavailable / disabled. */
  async start(): Promise<boolean> {
    if (!this.config.enabled) {
      this.lastError = "Speech-to-text is disabled. Enable it in Settings → Speech.";
      return false;
    }
    if (this.recording) return true;
    this.lastError = null;
    this.baseDraft = this.config.append_to_draft ? assistant.composerDraft : "";
    this.finalText = "";
    this.consumed = false;
    this.cancelRequested = false;
    this.clearTranscribeTimer();

    if (this.config.engine === "whisper") {
      if (!this.backendAvailable) {
        this.lastError =
          "Whisper backend not built. Install LLVM + rebuild with --features whisper-rs (see Settings → Speech for details).";
        return false;
      }
      try {
        await invoke("stt_start_recording", { model: this.config.whisper_model });
        // `recording` flips to true once the `stt://state: recording` event arrives.
        this.whisperStartInvoked = true;
        return true;
      } catch (e) {
        this.lastError = `Could not start whisper recording: ${e}`;
        return false;
      }
    }

    // --- Web Speech path (unchanged) ---
    if (!this.supported) {
      this.lastError = "Speech recognition is not available in this WebView.";
      return false;
    }
    const Ctor = getSRCtor();
    if (!Ctor) return false;

    const r = new Ctor();
    r.lang = this.config.language || "en-US";
    r.continuous = this.config.continuous;
    r.interimResults = this.config.show_interim;
    r.maxAlternatives = 3;
    r.onstart = () => {
      this.recording = true;
      this.transcribing = false;
    };
    r.onresult = (e) => this.onResult(e);
    r.onerror = (e) => this.onError(e);
    r.onend = () => this.onEnd();
    this.recognition = r;
    try {
      r.start();
      return true;
    } catch (e) {
      this.lastError = `Could not start recognition: ${e}`;
      this.recognition = null;
      this.recording = false;
      return false;
    }
  }

  /** End live recognition. */
  async stop(): Promise<string> {
    if (this.config.engine === "whisper") {
      if (!this.recording && !this.transcribing && !this.whisperStartInvoked) return this.lastTranscript;
      this.whisperStartInvoked = false;
      try {
        this.transcribing = true;
        const final = await invoke<string>("stt_stop_recording");
        // The `stt://final` event also fires; we treat the invoke return as
        // the authoritative result and ignore the event if it lands first.
        return final;
      } catch (e) {
        this.lastError = `Stop whisper recording failed: ${e}`;
        this.recording = false;
        this.transcribing = false;
        return this.lastTranscript;
      }
    }

    if (!this.recognition) return this.lastTranscript;
    try {
      this.recognition.stop();
      this.transcribing = true;
      this.clearTranscribeTimer();
      this.transcribeTimer = setTimeout(() => {
        if (this.transcribing) {
          this.transcribing = false;
          this.recognition = null;
        }
      }, 4000);
    } catch {
      /* recogniser already stopped */
    }
    return this.lastTranscript;
  }

  /** Hard-cancel — drop interim text, restore the original draft. */
  async cancel() {
    this.cancelRequested = true;
    if (this.config.engine === "whisper") {
      if (this.recording || this.transcribing) {
        try {
          await invoke("stt_stop_recording");
        } catch {
          /* may already be stopped */
        }
      }
      if (!this.consumed) {
        assistant.composerDraft = this.baseDraft;
      }
      this.finalText = "";
      this.recording = false;
      this.transcribing = false;
      this.clearTranscribeTimer();
      return;
    }

    if (!this.recognition) return;
    try {
      this.recognition.abort();
    } catch {
      /* recogniser may already be stopped */
    }
    if (!this.consumed) {
      assistant.composerDraft = this.baseDraft;
    }
    this.finalText = "";
    this.recording = false;
    this.transcribing = false;
    this.recognition = null;
    this.clearTranscribeTimer();
  }

  // ---- Whisper-only management ops ----------------------------------------

  async refreshModels() {
    try {
      this.models = await invoke<ModelInfo[]>("stt_list_models");
    } catch {
      /* Whisper not built — keep prior models list (likely empty). */
    }
  }

  async refreshInputDevices() {
    try {
      this.inputDevices = await invoke<string[]>("stt_get_input_devices");
    } catch {
      this.inputDevices = [];
    }
  }

  async downloadModel(modelId: string) {
    this.lastError = null;
    try {
      await invoke("stt_download_model", { modelId });
    } catch (e) {
      this.lastError = `Download failed: ${e}`;
    }
  }

  async cancelDownload() {
    try {
      await invoke("stt_cancel_download");
    } catch {
      /* nothing in flight — no-op */
    }
  }

  async deleteModel(modelId: string) {
    try {
      await invoke("stt_delete_model", { modelId });
      await this.refreshModels();
    } catch (e) {
      this.lastError = `Delete failed: ${e}`;
    }
  }

  // ---- Event handlers (backend) -------------------------------------------

  private onBackendPartial(text: string) {
    if (this.consumed) return;
    if (!this.config.show_interim) return;
    this.lastTranscript = text;
    assistant.composerDraft = this.composeDraft(text, "");
  }

  private onBackendFinal(text: string) {
    if (this.consumed) return;
    this.finalText = text;
    this.lastTranscript = text;
    assistant.composerDraft = this.composeDraft(text, "");
    this.recording = false;
    this.transcribing = false;
  }

  // ---- Event handlers (Web Speech) ----------------------------------------

  private clearTranscribeTimer() {
    if (this.transcribeTimer) {
      clearTimeout(this.transcribeTimer);
      this.transcribeTimer = null;
    }
  }

  private onResult(e: SpeechRecognitionEvent) {
    if (this.consumed) return;
    let interim = "";
    for (let i = e.resultIndex; i < e.results.length; i++) {
      const res = e.results[i];
      const txt = pickBestAlternate(res);
      if (res.isFinal) {
        this.finalText = (this.finalText + " " + txt).replace(/\s+/g, " ").trim();
      } else if (this.config.show_interim) {
        interim += txt;
      }
    }
    const composed = this.composeDraft(this.finalText, interim);
    assistant.composerDraft = composed;
    this.lastTranscript = this.finalText;
  }

  private composeDraft(final: string, interim: string): string {
    const tail = [final, interim].filter((s) => s.length > 0).join(" ").trim();
    if (!this.baseDraft) return tail;
    const sep = /\s$/.test(this.baseDraft) ? "" : " ";
    return this.baseDraft + sep + tail;
  }

  private onError(e: SpeechRecognitionErrorEvent) {
    // Friendly message already surfaced via `lastError`; the raw code is
    // dropped to avoid the #22 / #250 console-noise regression.
    this.lastError = errorMessage(e.error, e.message);
  }

  private onEnd() {
    // #175: commit only if neither user-cancel nor composer-consume fired.
    if (!this.cancelRequested && !this.consumed) {
      assistant.composerDraft = this.composeDraft(this.finalText, "");
    }
    this.recording = false;
    this.transcribing = false;
    this.recognition = null;
    this.clearTranscribeTimer();
  }
}

function pickBestAlternate(res: SpeechRecognitionResult): string {
  if (res.length === 0) return "";
  let best = res[0];
  for (let i = 1; i < res.length; i++) {
    const alt = res[i];
    if (alt && (alt.confidence ?? 0) > (best?.confidence ?? 0)) {
      best = alt;
    }
  }
  return best?.transcript ?? "";
}

function errorMessage(code: string, msg?: string): string {
  switch (code) {
    case "not-allowed":
    case "service-not-allowed":
      return "Microphone permission denied. Allow access in Windows Settings → Privacy → Microphone.";
    case "no-speech":
      return "No speech detected.";
    case "audio-capture":
      return "No microphone found.";
    case "network":
      return "Network error — speech recognition needs an internet connection.";
    case "aborted":
      return "Recording cancelled.";
    default:
      return msg ? `${code}: ${msg}` : `Recognition error: ${code}`;
  }
}

export const stt = new SttStore();
