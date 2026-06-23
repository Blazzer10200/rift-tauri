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
  voice_commands: boolean;
  auto_stop_secs: number;
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
    voice_commands: true,
    auto_stop_secs: 0,
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
  /** Voice command "send it" landed — the composer watches this and fires. */
  sendRequested = $state(false);
  /** The tab whose composer owns the active dictation. Bound at start() so
   *  recognized text lands in the pane the mic was clicked in — NOT the
   *  focused-pane shim, which may point at a different pane (the mic's onclick
   *  fires before the pane-focus onclick bubbles). Composers gate their
   *  sendRequested handler on this so "send it" only fires the right pane. */
  targetTabId = $state<string | null>(null);
  /** True while the Haiku polish of a finished dictation is in flight. */
  polishing = $state(false);
  /** Restore-point after a Haiku polish: full draft before/after, so the user
   *  can flip back to the raw transcript. Cleared on typing or timeout. */
  polishUndo = $state<{ committed: string; original: string } | null>(null);

  // Whisper-specific reactive state.
  models = $state<ModelInfo[]>([]);
  modelDownloads = $state<Record<string, DownloadProgress | undefined>>({});
  inputDevices = $state<string[]>([]);
  /** True once stt_start_recording has been invoked but before recording=true arrives via event. */
  whisperStartInvoked = $state(false);

  // --- Private fields ---
  /** Per-utterance final segments (Web Speech) — "scratch that" pops the tail. */
  private segments: string[] = [];
  private pendingSend = false;
  private lastSpeechAt = 0;
  private silenceTimer: ReturnType<typeof setInterval> | null = null;
  private polishUndoTimer: ReturnType<typeof setTimeout> | null = null;
  // Monotonic token guarding the in-flight Haiku polish. Bumped to invalidate a
  // polish whose result/shimmer should no longer land (user typed, or the
  // visual cap elapsed) — see polishWebSpeechFinal / cancelPolish (#40a/#40b).
  private polishGuard = 0;
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
  private stopInFlight = false;
  private restartToken = 0;
  private unlisten: UnlistenFn[] = [];

  destroy() {
    for (const fn of this.unlisten) fn();
    this.unlisten = [];
    this.initStarted = false;
    // RR10: clear every armed timer so an HMR module swap doesn't leak a live
    // interval/timeout firing into stale $state.
    this.clearSilenceWatch();
    this.clearTranscribeTimer();
    if (this.polishUndoTimer) {
      clearTimeout(this.polishUndoTimer);
      this.polishUndoTimer = null;
    }
    // Invalidate any in-flight Haiku polish so its result/shimmer can't land in
    // the post-HMR store (capTimer is a local timeout; the guard bump stops the
    // awaited swap).
    this.polishGuard++;
    this.polishing = false;
  }

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
    // Per-channel guard: one channel failing must not skip the rest, and a
    // failure surfaces via lastError instead of silently degrading.
    const sub = async (channel: string, fn: () => Promise<UnlistenFn>) => {
      try {
        this.unlisten.push(await fn());
      } catch (e) {
        this.lastError = `STT channel ${channel} unavailable: ${String(e)}`;
      }
    };
    {
      await sub("stt://partial", () =>
        listen<{ text: string }>("stt://partial", (ev) => this.onBackendPartial(ev.payload.text)),
      );
      await sub("stt://final", () =>
        listen<{ text: string; raw: string; cleaned: boolean }>("stt://final", (ev) =>
          this.onBackendFinal(ev.payload.text, ev.payload.raw, ev.payload.cleaned),
        ),
      );
      await sub("stt://state", () =>
        listen<{ state: SttState; message: string | null }>("stt://state", (ev) => {
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
      await sub("stt://error", () =>
        listen<{ code: string; message: string }>("stt://error", (ev) => {
          this.lastError = ev.payload.message;
        }),
      );
      await sub("stt://download_progress", () =>
        listen<DownloadProgress>("stt://download_progress", (ev) => {
          if (!ev.payload?.model) return;
          this.modelDownloads = { ...this.modelDownloads, [ev.payload.model]: ev.payload };
          if (ev.payload.phase === "done") {
            void this.refreshModels();
          }
        }),
      );
      // Non-fatal — any failed channel is recorded above; Web Speech path stays functional.
    }

    // Best-effort initial loads for the Settings panel — failures are non-fatal.
    void this.refreshModels();
    void this.refreshInputDevices();
  }

  async setConfig(patch: Partial<SttConfig>) {
    const prev = this.config;
    const prevEngine = this.config.engine;
    const next: SttConfig = { ...this.config, ...patch };
    this.config = next;
    let succeeded = true;
    try {
      await invoke("stt_set_config", { config: next });
    } catch (e) {
      succeeded = false;
      this.config = prev; // backend rejected — don't leave UI ahead of persisted state
      this.lastError = `Save settings failed: ${e}`;
    }

    // RR10: only act on the engine switch if the backend ACCEPTED it. A rejected
    // save rolls config back to prev (same engine) — cancelling the live
    // recording then would destroy the user's dictation for a failed settings save.
    if (!succeeded) return;

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
      // abort() fires onEnd → a polish for the old segment; cancel it so the
      // new session doesn't inherit polishing=true and block all future polish.
      this.cancelPolish();
      this.recognition.abort();
      this.recording = false;
      setTimeout(() => {
        if (token === this.restartToken) void this.start();
      }, 120);
    }
  }

  /** Read/write the dictation target's draft. Routes to the tab the mic was
   *  started in (`targetTabId`), falling back to the focused-pane shim when no
   *  target is bound (e.g. legacy single-pane callers). */
  private readDraft(): string {
    const t = assistant.tabFor(this.targetTabId);
    return t ? t.draft : assistant.composerDraft;
  }
  private writeDraft(v: string) {
    const t = assistant.tabFor(this.targetTabId);
    if (t) t.draft = v;
    else assistant.composerDraft = v;
  }

  /** Composer-side hook: the current draft was just sent / cleared. */
  consume() {
    this.cancelRequested = true;
    this.cancelPolish(); // stop any shimmer + invalidate an in-flight polish
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
    this.segments = [];
    this.pendingSend = false;
    this.polishUndo = null;
    this.consumed = true;
    this.lastTranscript = "";
    this.recording = false;
    this.transcribing = false;
    this.recognition = null;
    this.clearTranscribeTimer();
    this.clearSilenceWatch();
  }

  /** Begin live recognition. Returns false if unavailable / disabled.
   *  `tabId` binds the dictation to a specific pane's tab; omit for the
   *  focused tab (the legacy single-pane behaviour). */
  async start(tabId?: string | null): Promise<boolean> {
    if (!this.config.enabled) {
      this.lastError = "Speech-to-text is disabled. Enable it in Settings → Speech.";
      return false;
    }
    if (this.recording) return true;
    // Bind BEFORE reading baseDraft so we capture the target pane's draft.
    this.targetTabId = tabId ?? assistant.currentConvoId;
    this.lastError = null;
    this.baseDraft = this.config.append_to_draft ? this.readDraft() : "";
    this.finalText = "";
    this.segments = [];
    this.pendingSend = false;
    this.sendRequested = false;
    this.polishUndo = null;
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
        this.startSilenceWatch();
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
      this.startSilenceWatch();
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
    // RR10: a second concurrent stop() would race past the recording-state guard
    // before the first flips it, double-invoking stt_stop_recording (the second
    // gets "no stt session active" surfaced as a spurious lastError).
    if (this.stopInFlight) return this.lastTranscript;
    this.stopInFlight = true;
    try {
      return await this.stopInner();
    } finally {
      this.stopInFlight = false;
    }
  }

  private async stopInner(): Promise<string> {
    this.clearSilenceWatch();
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
    this.clearSilenceWatch();
    this.segments = [];
    this.pendingSend = false;
    if (this.config.engine === "whisper") {
      if (this.recording || this.transcribing) {
        try {
          await invoke("stt_stop_recording");
        } catch {
          /* may already be stopped */
        }
      }
      if (!this.consumed) {
        this.writeDraft(this.baseDraft);
      }
      this.finalText = "";
      this.recording = false;
      this.transcribing = false;
      this.whisperStartInvoked = false;
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
      this.writeDraft(this.baseDraft);
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
    // Speech activity signal for the silence watch — fires even when interim
    // display is off (the backend only emits partials when VAD hears speech).
    this.lastSpeechAt = Date.now();
    if (this.consumed || this.cancelRequested) return;
    if (!this.config.show_interim) return;
    const t = decensor(text);
    this.lastTranscript = t;
    this.writeDraft(this.composeDraft(t, ""));
  }

  private onBackendFinal(text: string, raw?: string, cleaned?: boolean) {
    this.clearSilenceWatch();
    if (this.consumed || this.cancelRequested) return;
    let t = decensor(text);
    let send = false;
    if (this.config.voice_commands) {
      t = applyInlineCommands(t).trim();
      if (SEND_CMD_RE.test(t)) {
        t = t.replace(SEND_CMD_RE, "").trim();
        // Send even when the utterance was ONLY the command — the intent is to
        // ship whatever is already in the draft (baseDraft / prior segments).
        send = true;
      }
    }
    this.finalText = t;
    this.lastTranscript = t;
    const committed = this.composeDraft(t, "");
    this.writeDraft(committed);
    // The Whisper path polishes backend-side — arm the raw-transcript undo.
    if (cleaned && raw && raw.trim() !== text.trim()) {
      this.setPolishUndo(committed, this.composeDraft(decensor(raw.trim()), ""));
    }
    this.recording = false;
    this.transcribing = false;
    if (send) this.sendRequested = true;
  }

  // ---- Event handlers (Web Speech) ----------------------------------------

  private clearTranscribeTimer() {
    if (this.transcribeTimer) {
      clearTimeout(this.transcribeTimer);
      this.transcribeTimer = null;
    }
  }

  // ---- Auto-stop on silence ------------------------------------------------

  /** Watch the speech-event stream and end the recording after
   *  `auto_stop_secs` of silence. Skipped when the engine produces no events
   *  to watch (Web Speech with interim results off). */
  private startSilenceWatch() {
    this.clearSilenceWatch();
    const secs = this.config.auto_stop_secs;
    if (!secs) return;
    if (this.config.engine === "web_speech" && !this.config.show_interim) return;
    this.lastSpeechAt = Date.now();
    this.silenceTimer = setInterval(() => {
      if (!this.recording) return;
      if (Date.now() - this.lastSpeechAt >= secs * 1000) {
        this.clearSilenceWatch();
        void this.stop();
      }
    }, 500);
  }

  private clearSilenceWatch() {
    if (this.silenceTimer) {
      clearInterval(this.silenceTimer);
      this.silenceTimer = null;
    }
  }

  // ---- Haiku-polish undo -----------------------------------------------------

  /** Arm the raw-transcript restore point. Auto-expires. */
  private setPolishUndo(committed: string, original: string) {
    this.polishUndo = { committed, original };
    if (this.polishUndoTimer) clearTimeout(this.polishUndoTimer);
    this.polishUndoTimer = setTimeout(() => (this.polishUndo = null), 15000);
  }

  /** Flip the draft back to the raw (pre-Haiku) transcript. */
  revertPolish() {
    const u = this.polishUndo;
    this.polishUndo = null;
    if (u && this.readDraft() === u.committed) {
      this.writeDraft(u.original);
      this.finalText = "";
    }
  }

  dismissPolishUndo() {
    this.polishUndo = null;
  }

  /** Stop the polish shimmer immediately (e.g. the user started typing). The
   *  raw transcript is already committed + editable, so we only need to drop
   *  the visual and invalidate the late swap — #40b. */
  cancelPolish() {
    if (!this.polishing) return;
    this.polishGuard++;
    this.polishing = false;
  }

  private onResult(e: SpeechRecognitionEvent) {
    if (this.consumed) return;
    this.lastSpeechAt = Date.now();
    let interim = "";
    for (let i = e.resultIndex; i < e.results.length; i++) {
      const res = e.results[i];
      const txt = decensor(pickBestAlternate(res));
      if (res.isFinal) {
        this.commitSegment(txt);
      } else if (this.config.show_interim) {
        interim += txt;
      }
    }
    const composed = this.composeDraft(this.finalText, interim);
    this.writeDraft(composed);
    this.lastTranscript = this.finalText;
    // "send it" — draft is committed above; the composer's effect fires it.
    if (this.pendingSend) {
      this.pendingSend = false;
      if (this.config.cleanup_enabled && /\*{2,}/.test(this.finalText)) {
        // Engine-masked profanity that decensor() couldn't resolve (fully
        // masked, no leading letter) only gets restored by the Haiku pass —
        // polish first so "send it" doesn't ship asterisks.
        void this.polishWebSpeechFinal().then(() => (this.sendRequested = true));
      } else {
        this.sendRequested = true;
      }
    }
  }

  /** Fold one final Web Speech segment into the transcript, interpreting
   *  voice commands when enabled. */
  private commitSegment(raw: string) {
    let seg = raw.trim();
    if (!seg) return;
    if (this.config.voice_commands) {
      seg = applyInlineCommands(seg);
      if (SCRATCH_CMD_RE.test(seg)) {
        // "scratch that" deletes the last thing said. Pop the previous committed
        // segment, then push back any words spoken AFTER the command in the same
        // utterance ("scratch that, actually …") so they aren't lost.
        const rest = seg.replace(SCRATCH_CMD_RE, "").trim();
        this.segments.pop();
        if (rest) this.segments.push(rest);
        this.refreshFinal();
        return;
      }
      if (SEND_CMD_RE.test(seg)) {
        const rest = seg.replace(SEND_CMD_RE, "").trim();
        if (rest) this.segments.push(rest);
        this.refreshFinal();
        this.pendingSend = true;
        return;
      }
    }
    this.segments.push(seg);
    // Continuous-mode safety: collapse the backlog into a single string so an
    // hours-long session doesn't grow segments[] (and its per-commit join)
    // without bound. "scratch that" then only pops the most-recent utterance,
    // which is the practical case.
    if (this.segments.length > 500) {
      this.segments = [this.segments.join(" ")];
    }
    this.refreshFinal();
  }

  /** Rebuild finalText from segments — collapses runs of spaces but keeps the
   *  newlines voice commands insert. */
  private refreshFinal() {
    this.finalText = this.segments
      .join(" ")
      .replace(/[ \t]{2,}/g, " ")
      .replace(/[ \t]*\n[ \t]*/g, "\n")
      .trim();
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
    const commit = !this.cancelRequested && !this.consumed;
    if (commit) {
      // #40d: onResult already streamed the final text into the draft — only
      // rewrite if there's an actual delta, so the textarea doesn't flash the
      // whole phrase back in at stop.
      const composed = this.composeDraft(this.finalText, "");
      if (this.readDraft() !== composed) this.writeDraft(composed);
    }
    this.recording = false;
    this.transcribing = false;
    this.recognition = null;
    this.clearTranscribeTimer();
    this.clearSilenceWatch();
    // Web Speech finals never got the Haiku polish (Whisper-only until now) —
    // run it post-commit so punctuation lands and any leftover engine-masked
    // profanity ("******") gets restored from context. Skipped when a voice
    // command is about to send — the text is leaving now.
    if (commit && !this.sendRequested) void this.polishWebSpeechFinal();
  }

  /** Haiku cleanup for a finished Web Speech dictation. Replaces the dictated
   *  tail of the draft only if the user hasn't sent, cancelled, or typed over
   *  it while the polish was in flight. Any failure leaves the raw transcript
   *  — same never-lose-the-transcript contract as the Whisper path. */
  private async polishWebSpeechFinal() {
    if (this.polishing) return; // pre-send pass already in flight — don't double-spawn from onEnd
    const raw = this.finalText.trim();
    if (!this.config.cleanup_enabled || raw.split(/\s+/).length < 3) return;
    const committed = this.composeDraft(raw, "");
    // Non-blocking: the raw transcript is already committed to the draft and
    // is immediately editable/sendable. The Haiku pass is cosmetic, so it must
    // NOT raise `transcribing` (that disables the mic + blocks the composer,
    // which read as "loading forever"). `polishing` drives only the quiet
    // textarea shimmer; the result swaps in below only if the draft is
    // untouched.
    this.polishing = true;
    const token = ++this.polishGuard;
    // #40a: cap the *visual* well under the backend's 15s timeout — a slow
    // Haiku call shouldn't pulse the committed (already-usable) transcript for
    // that long. The swap below still lands if the result beats the cap.
    const SHIMMER_CAP_MS = 6000;
    const capTimer = setTimeout(() => {
      if (token === this.polishGuard) this.polishing = false;
    }, SHIMMER_CAP_MS);
    try {
      const cleaned = (await invoke<string>("stt_clean_transcript", { text: raw })).trim();
      if (
        token === this.polishGuard && // not cancelled by typing / superseded
        cleaned &&
        cleaned !== raw &&
        !this.consumed &&
        !this.cancelRequested &&
        this.readDraft() === committed
      ) {
        const polished = this.composeDraft(cleaned, "");
        this.writeDraft(polished);
        this.finalText = cleaned;
        this.lastTranscript = cleaned;
        this.setPolishUndo(polished, committed);
      }
    } catch (e) {
      console.warn("stt cleanup failed:", e);
    } finally {
      clearTimeout(capTimer);
      if (token === this.polishGuard) this.polishing = false;
    }
  }
}

// Azure/Google recognition masks profanity as a leading letter + asterisks
// ("f***", "b****") — the Web Speech API exposes no knob to turn that off, and
// Whisper occasionally emits the same masks from its training data. Restore the
// high-frequency unambiguous ones by (first letter, original word length);
// unknown masks pass through for the Haiku cleanup pass to resolve from context.
const DECENSOR_MAP: Record<string, string> = {
  f4: "fuck", f5: "fucks", f6: "fucker", f7: "fucking",
  s4: "shit", s5: "shits", s6: "shitty",
  b5: "bitch", b7: "bastard", b8: "bullshit",
  a3: "ass", a7: "asshole", a8: "assholes",
  d4: "damn", d6: "damnit", d7: "dammit",
  h4: "hell",
  g7: "goddamn", g9: "goddammit",
  p4: "piss", p5: "pussy", p6: "pissed", p7: "pissing",
  j7: "jackass",
  m12: "motherfucker", m13: "motherfucking",
};
// Spoken commands — recognized only when `voice_commands` is on. Trailing
// punctuation tolerated since both engines like to append periods.
const SEND_CMD_RE = /(?:^|\s)send (?:it|that|message)[.,!?]*\s*$/i;
const SCRATCH_CMD_RE = /(?:^|\s)(?:scratch|strike) that[.,!?]*\s*$/i;
export function applyInlineCommands(t: string): string {
  return t
    .replace(/(?:^|\s)new paragraph[.,]?(?=\s|$)/gi, "\n\n")
    .replace(/(?:^|\s)new line[.,]?(?=\s|$)/gi, "\n");
}

export function decensor(text: string): string {
  if (!text.includes("*")) return text;
  return text.replace(/\b([a-zA-Z])(\*{2,})([a-zA-Z]*)/g, (match, first: string, stars: string, tail: string) => {
    const key = `${first.toLowerCase()}${1 + stars.length + tail.length}`;
    const word = DECENSOR_MAP[key];
    if (!word) return match;
    return first === first.toUpperCase() ? word[0].toUpperCase() + word.slice(1) : word;
  });
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

// RR10: HMR teardown — clear listeners + timers on module dispose so a dev
// hot-reload starts clean (mirrors assistant.svelte.ts).
if (typeof import.meta !== "undefined" && (import.meta as { hot?: { dispose: (cb: () => void) => void } }).hot) {
  (import.meta as { hot: { dispose: (cb: () => void) => void } }).hot.dispose(() => stt.destroy());
}
