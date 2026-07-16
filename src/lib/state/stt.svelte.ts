// Speech-to-text state.
//
// Three engines, same public surface (composer wires `recording` /
// `transcribing` / `lastError` / `start` / `stop` / `cancel` / `consume`
// without caring which engine is active):
//
//   • engine="web_speech" (default, legacy) — WebView2's built-in
//     SpeechRecognition (Azure-backed when online). 100% in-browser, no
//     backend round-trips. Settings persisted via `stt_get/set_config`.
//
//   • engine="whisper" / engine="parakeet" — Rust-side cpal mic capture,
//     webrtc-vad gating, local inference (whisper.cpp / Parakeet TDT via
//     ONNX Runtime). Driven over Tauri IPC (`stt_start_recording` /
//     `stt_stop_recording`). Event flow: `stt://partial` (interim ghost) →
//     `stt://segment` (sentence committed on a speech pause, mid-recording) →
//     `stt://final` (stop-time uncommitted tail only) → `stt://polish`
//     (background Claude cleanup, swapped in only if the draft is untouched).
//
// Interim speech renders as `ghostTail` (dim, display-only, in the composer
// overlay); only finalized text is committed into the draft — spoken words
// "turn white" when the transcript lands (per sentence for local engines,
// per final result for Web Speech).
//
// Switching engines mid-session is allowed — `setConfig({engine})` aborts
// any in-flight recording before the new engine takes over.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { assistant } from "./assistant.svelte";
import { notify } from "./toast.svelte";

export type SttEngine = "web_speech" | "whisper" | "parakeet";

/** Engines whose audio pipeline lives in Rust (cpal capture + local inference). */
export function isLocalEngine(e: SttEngine): boolean {
  return e === "whisper" || e === "parakeet";
}

export type SttConfig = {
  enabled: boolean;
  language: string;
  append_to_draft: boolean;
  continuous: boolean;
  show_interim: boolean;
  engine: SttEngine;
  whisper_model: string;
  parakeet_model: string;
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
  engine: "whisper" | "parakeet";
  filename: string;
  approx_size_bytes: number;
  on_disk_bytes: number | null;
  downloaded: boolean;
  path: string | null;
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

const DEFAULT_STT_CONFIG: SttConfig = {
  enabled: false,
  language: "en-US",
  append_to_draft: true,
  continuous: true,
  show_interim: true,
  engine: "web_speech",
  whisper_model: "large-v3-turbo-q5_0",
  parakeet_model: "parakeet-tdt-0.6b-v3-int8",
  input_device: null,
  initial_prompt: "",
  vocab_text: "",
  cleanup_enabled: true,
  beam_size: null,
  voice_commands: true,
  auto_stop_secs: 0,
};

class SttStore {
  config = $state<SttConfig>({ ...DEFAULT_STT_CONFIG });
  configLoaded = $state(false);

  /** Web Speech API availability in the current WebView. */
  supported = $state(false);
  /** Which local Rust backends were compiled in. Parakeet ships in release
   *  builds; Whisper needs an LLVM opt-in build (`--features whisper-rs`). */
  backends = $state<{ whisper: boolean; parakeet: boolean }>({ whisper: false, parakeet: false });
  recording = $state(false);
  /** True from start() until the engine confirms recording (or the start
   *  fails) — covers the capture-init window so UI keyed to "dictation is
   *  live here" (composer posture) doesn't flicker while the mic spins up. */
  starting = $state(false);
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
  /** Live mic input level, 0..1 normalized — drives the composer waveform meter.
   *  Whisper feeds it from `stt://level`; web_speech from a browser AnalyserNode. */
  level = $state(0);
  /** Fraction (1→0) of the auto-stop warning window remaining, surfaced only in
   *  the last few seconds of silence — drives the composer's depleting ring.
   *  null when auto-stop is off or speech is recent. */
  silenceFrac = $state<number | null>(null);
  /** True while the cleanup polish of a finished dictation is in flight. */
  polishing = $state(false);
  /** Uncommitted interim speech — rendered by the composer as ghost text after
   *  the solid draft, NOT written into it. Committed (turned solid) when a
   *  segment or the final lands; cleared on cancel/consume. All engines: only
   *  the in-flight sentence is ghost — Whisper/Parakeet commit on speech
   *  pauses (stt://segment), Web Speech on recognizer finals. */
  ghostTail = $state("");
  /** Restore-point after a cleanup polish: full draft before/after, so the user
   *  can flip back to the raw transcript. Cleared on typing or timeout. */
  polishUndo = $state<{ committed: string; original: string } | null>(null);

  // Whisper-specific reactive state.
  models = $state<ModelInfo[]>([]);
  modelDownloads = $state<Record<string, DownloadProgress | undefined>>({});
  inputDevices = $state<string[]>([]);
  /** True once stt_start_recording has been invoked but before recording=true arrives via event. */
  whisperStartInvoked = $state(false);
  /** The in-flight stt_start_recording invoke. Mic start takes real time
   *  (capture init; model load on first use) — a push-to-talk release inside
   *  that window used to hit stop()'s "nothing running" guard and no-op,
   *  leaving the mic recording after the key was let go and the spoken ghost
   *  words uncommitted. stop()/cancel() await this first so a release is
   *  ALWAYS a stop, however fast. */
  private startPending: Promise<unknown> | null = null;

  // --- Private fields ---
  /** Per-utterance final segments (Web Speech) — "scratch that" pops the tail. */
  private segments: string[] = [];
  private pendingSend = false;
  private lastSpeechAt = 0;
  private silenceTimer: ReturnType<typeof setInterval> | null = null;
  private polishUndoTimer: ReturnType<typeof setTimeout> | null = null;
  // Monotonic token guarding the in-flight cleanup polish. Bumped to invalidate a
  // polish whose result/shimmer should no longer land (user typed, or the
  // visual cap elapsed) — see polishWebSpeechFinal / cancelPolish (#40a/#40b).
  private polishGuard = 0;
  // Token armed at stt://final when a backend polish is pending; the
  // stt://polish handler only applies when it still matches polishGuard.
  private pendingPolishToken: number | null = null;
  // True once a voice command edited the committed text ("scratch that",
  // "new line", "send it"). The backend polishes ITS raw transcript, which no
  // longer matches — applying it would resurrect scratched words, so the
  // polish swap is skipped for the session.
  private commandsMutated = false;
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
  // Web-speech level meter: getUserMedia → AudioContext → AnalyserNode polled on
  // a rAF loop (web_speech has no Rust audio path, so the meter is browser-side).
  private meterCtx: AudioContext | null = null;
  private meterStream: MediaStream | null = null;
  private meterRaf: number | null = null;

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
    // Invalidate any in-flight cleanup polish so its result/shimmer can't land in
    // the post-HMR store (capTimer is a local timeout; the guard bump stops the
    // awaited swap).
    this.polishGuard++;
    this.polishing = false;
    this.ghostTail = "";
    this.stopWebMeter();
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
      this.backends = await invoke<{ whisper: boolean; parakeet: boolean }>(
        "stt_backend_available",
      );
    } catch {
      this.backends = { whisper: false, parakeet: false };
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
      await sub("stt://segment", () =>
        listen<{ text: string }>("stt://segment", (ev) => this.onBackendSegment(ev.payload.text)),
      );
      await sub("stt://final", () =>
        listen<{ text: string; polish_pending: boolean }>("stt://final", (ev) =>
          this.onBackendFinal(ev.payload.text, ev.payload.polish_pending),
        ),
      );
      await sub("stt://polish", () =>
        listen<{ text: string; raw: string }>("stt://polish", (ev) =>
          this.onBackendPolish(ev.payload.text, ev.payload.raw),
        ),
      );
      await sub("stt://state", () =>
        listen<{ state: SttState; message: string | null }>("stt://state", (ev) => {
          this.currentState = ev.payload.state;
          if (ev.payload.state === "transcribing") this.transcribing = true;
          if (ev.payload.state === "recording") {
            this.recording = true;
            this.starting = false;
            this.transcribing = false;
            // The session is live — any earlier transient error (e.g. a raced
            // double-start) is stale. Leaving it set kept the mic painted red
            // through a perfectly healthy recording.
            this.lastError = null;
          }
          if (ev.payload.state === "idle") {
            this.recording = false;
            this.starting = false;
            this.transcribing = false;
          }
        }),
      );
      await sub("stt://level", () =>
        listen<{ rms: number }>("stt://level", (ev) => {
          // Whisper-only channel — web_speech drives `level` from its own
          // AnalyserNode. Both feed through pushLevel for matched smoothing.
          this.pushLevel(ev.payload.rms);
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
          this.modelDownloads[ev.payload.model] = ev.payload;
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
        // Re-check recording at fire-time: if something restarted the mic in
        // the 120ms gap, start() again would double-start the recogniser.
        if (token === this.restartToken && !this.recording) void this.start();
      }, 120);
    }
  }

  /** Every voice pref → factory defaults. Downloaded Whisper model files stay on disk. */
  async resetConfig() {
    await this.setConfig({ ...DEFAULT_STT_CONFIG });
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
    if (isLocalEngine(this.config.engine)) {
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
    this.ghostTail = "";
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
      this.failToast();
      return false;
    }
    if (this.recording) return true;
    // A start is already in flight (model load / capture init) — treat the
    // second call as success instead of racing a duplicate into the backend
    // ("stt session already active" painted the mic red mid-recording).
    if (this.whisperStartInvoked) return true;
    // Bind BEFORE reading baseDraft so we capture the target pane's draft.
    this.targetTabId = tabId ?? assistant.currentConvoId;
    this.starting = true;
    this.lastError = null;
    this.ghostTail = "";
    this.baseDraft = this.config.append_to_draft ? this.readDraft() : "";
    // Replace mode: the old draft dies at the moment dictation starts — the
    // ghost tail must not render appended after text that's about to vanish.
    if (!this.config.append_to_draft) this.writeDraft("");
    this.finalText = "";
    this.segments = [];
    this.pendingSend = false;
    this.sendRequested = false;
    this.polishUndo = null;
    this.consumed = false;
    this.cancelRequested = false;
    this.commandsMutated = false;
    this.pendingPolishToken = null; // a polish from a prior session can't land here
    this.clearTranscribeTimer();

    if (isLocalEngine(this.config.engine)) {
      const isWhisper = this.config.engine === "whisper";
      if (isWhisper ? !this.backends.whisper : !this.backends.parakeet) {
        this.lastError = isWhisper
          ? "Whisper backend not built. Install LLVM + rebuild with --features whisper-rs (see Settings → Speech for details)."
          : "Parakeet backend not built into this binary. Rebuild with the default feature set (see Settings → Speech).";
        this.failToast();
        this.starting = false;
        return false;
      }
      try {
        const pending = invoke("stt_start_recording", {
          model: isWhisper ? this.config.whisper_model : this.config.parakeet_model,
        });
        this.startPending = pending;
        await pending;
        // `recording` flips to true once the `stt://state: recording` event arrives.
        this.whisperStartInvoked = true;
        this.startSilenceWatch();
        return true;
      } catch (e) {
        this.lastError = `Could not start whisper recording: ${e}`;
        this.failToast();
        this.starting = false;
        return false;
      } finally {
        this.startPending = null;
      }
    }

    // --- Web Speech path (unchanged) ---
    if (!this.supported) {
      this.lastError = "Speech recognition is not available in this WebView.";
      this.failToast();
      this.starting = false;
      return false;
    }
    const Ctor = getSRCtor();
    if (!Ctor) { this.starting = false; return false; }

    const r = new Ctor();
    r.lang = this.config.language || "en-US";
    r.continuous = this.config.continuous;
    r.interimResults = this.config.show_interim;
    r.maxAlternatives = 3;
    r.onstart = () => {
      this.recording = true;
      this.starting = false;
      this.transcribing = false;
      this.startSilenceWatch();
      void this.startWebMeter();
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
      this.failToast();
      this.recognition = null;
      this.recording = false;
      this.starting = false;
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
    this.starting = false;
    try {
      return await this.stopInner();
    } finally {
      this.stopInFlight = false;
    }
  }

  private async stopInner(): Promise<string> {
    this.clearSilenceWatch();
    if (isLocalEngine(this.config.engine)) {
      // A start is mid-flight (capture/model init) — wait it out, then stop.
      // A push-to-talk release inside that window must still end the session;
      // set whisperStartInvoked ourselves so the guard below can't no-op on
      // microtask ordering. (Start failure → nothing running → guard returns.)
      if (this.startPending) {
        try {
          await this.startPending;
          this.whisperStartInvoked = true;
        } catch { /* start failed — the guard below no-ops */ }
      }
      if (!this.recording && !this.transcribing && !this.whisperStartInvoked) return this.lastTranscript;
      this.whisperStartInvoked = false;
      try {
        this.transcribing = true;
        // Draft commits ride the events (`stt://segment` / `stt://final`);
        // the invoke return is the full raw transcript, for the caller only.
        return await invoke<string>("stt_stop_recording");
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
    this.starting = false;
    this.cancelPolish();
    if (this.polishUndoTimer) { clearTimeout(this.polishUndoTimer); this.polishUndoTimer = null; }
    this.clearSilenceWatch();
    this.segments = [];
    this.pendingSend = false;
    this.ghostTail = "";
    if (isLocalEngine(this.config.engine)) {
      // A start still mid-invoke must be waited out too (mirrors stopInner) —
      // cancelling during capture/model init used to leave the mic recording.
      if (this.startPending) {
        try {
          await this.startPending;
          this.whisperStartInvoked = true;
        } catch { /* start failed — nothing to cancel */ }
      }
      // Include whisperStartInvoked: cancel can fire in the gap between
      // stt_start_recording returning and the `recording` state event landing —
      // recording is still false then, so without this the backend keeps
      // recording after a cancel (mirrors the same guard in stop(), line ~455).
      if (this.recording || this.transcribing || this.whisperStartInvoked) {
        try {
          await invoke("stt_stop_recording");
        } catch {
          /* may already be stopped */
        }
      }
      this.whisperStartInvoked = false;
      if (!this.consumed) {
        this.writeDraft(this.baseDraft);
      }
      this.finalText = "";
      this.recording = false;
      this.transcribing = false;
      this.whisperStartInvoked = false;
      this.level = 0;
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
    this.stopWebMeter();
  }

  // ---- Level meter --------------------------------------------------------

  /** Normalize + smooth a raw RMS reading into `level` (0..1). Asymmetric
   *  smoothing — snap up on speech onset (attack), ease down on silence
   *  (release) — so the bars feel responsive but don't strobe. Shared by both
   *  the whisper event and the web_speech AnalyserNode so they behave alike. */
  private pushLevel(rawRms: number) {
    const target = Math.min(1, rawRms * 4);
    const prev = this.level;
    const k = target > prev ? 0.6 : 0.25; // fast attack, slow release
    this.level = prev + (target - prev) * k;
  }

  // ---- Web-speech level meter ---------------------------------------------

  /** Open a mic AnalyserNode and poll its RMS into `level` on a rAF loop.
   *  web_speech recognition holds its own mic capture; this is a separate
   *  getUserMedia stream purely for visualization. Torn down by stopWebMeter. */
  private async startWebMeter() {
    if (this.meterCtx) return; // already running
    try {
      const constraints: MediaStreamConstraints = {
        audio: this.config.input_device
          ? { deviceId: { ideal: this.config.input_device } }
          : true,
      };
      const stream = await navigator.mediaDevices.getUserMedia(constraints);
      // Recording may have ended while permission/getUserMedia was in flight.
      if (!this.recording) {
        stream.getTracks().forEach((t) => t.stop());
        return;
      }
      const ctx = new AudioContext();
      const src = ctx.createMediaStreamSource(stream);
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 512;
      src.connect(analyser);
      this.meterStream = stream;
      this.meterCtx = ctx;
      const buf = new Float32Array(analyser.fftSize);
      const tick = () => {
        if (!this.meterCtx) return;
        analyser.getFloatTimeDomainData(buf);
        let sumSq = 0;
        for (let i = 0; i < buf.length; i++) sumSq += buf[i] * buf[i];
        const rms = Math.sqrt(sumSq / buf.length);
        this.pushLevel(rms);
        this.meterRaf = requestAnimationFrame(tick);
      };
      this.meterRaf = requestAnimationFrame(tick);
    } catch (e) {
      // Non-fatal — recognition still works without the visual meter.
      console.warn("[stt] level meter unavailable:", e);
    }
  }

  private stopWebMeter() {
    if (this.meterRaf !== null) {
      cancelAnimationFrame(this.meterRaf);
      this.meterRaf = null;
    }
    if (this.meterStream) {
      this.meterStream.getTracks().forEach((t) => t.stop());
      this.meterStream = null;
    }
    if (this.meterCtx) {
      void this.meterCtx.close().catch(() => {});
      this.meterCtx = null;
    }
    this.level = 0;
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
    // Ghost only — the draft stays untouched until the final commits, so the
    // spoken words render dimmed and "turn white" when the transcript lands.
    this.ghostTail = t;
  }

  /** A sentence finalized mid-recording (the backend heard a speech pause).
   *  Folds through the same segment machinery as Web Speech finals — voice
   *  commands ("send it", "scratch that", "new line") work mid-dictation. */
  private onBackendSegment(text: string) {
    this.lastSpeechAt = Date.now();
    if (this.consumed || this.cancelRequested) return;
    this.ghostTail = "";
    this.commitSegment(decensor(text));
    const composed = this.composeDraft(this.finalText);
    if (this.readDraft() !== composed) this.writeDraft(composed);
    this.lastTranscript = this.finalText;
    if (this.pendingSend) {
      this.pendingSend = false;
      // Committed text is already in the draft; the composer's effect fires
      // the send, whose consume() stops the backend recording.
      this.sendRequested = true;
    }
  }

  /** Stop-time result — carries ONLY the uncommitted tail (segments already
   *  landed via stt://segment). The raw transcript is committed and sendable
   *  immediately; the cleanup polish arrives later via stt://polish. */
  private onBackendFinal(tailText: string, polishPending: boolean) {
    this.clearSilenceWatch();
    this.ghostTail = "";
    if (this.consumed || this.cancelRequested) return;
    this.commitSegment(decensor(tailText));
    const composed = this.composeDraft(this.finalText);
    if (this.readDraft() !== composed) this.writeDraft(composed);
    this.lastTranscript = this.finalText;
    this.recording = false;
    this.transcribing = false;
    // Same stale-error rule as the Web Speech onEnd: a non-empty final means
    // the session succeeded — clear any transient error so the mic doesn't
    // stay red.
    if (this.finalText) this.lastError = null;
    if (this.pendingSend) {
      this.pendingSend = false;
      this.sendRequested = true;
    }
    // Announce the background cleanup: quiet shimmer (visual cap well under
    // the backend's 15s timeout), never blocks editing or sending.
    if (polishPending && !this.sendRequested) {
      this.polishing = true;
      const token = ++this.polishGuard;
      this.pendingPolishToken = token;
      setTimeout(() => {
        if (token === this.polishGuard) this.polishing = false;
      }, 6000);
    }
  }

  /** Background cleanup landed. Swap the polished transcript into the draft
   *  only when nothing invalidated it: no typing since the final (draft still
   *  matches), no send/cancel, no voice-command edits (the backend polished
   *  its own raw join, which wouldn't include them). */
  private onBackendPolish(text: string, raw: string) {
    const token = this.pendingPolishToken;
    this.pendingPolishToken = null;
    if (token === null || token !== this.polishGuard) return; // cancelled or superseded
    this.polishing = false;
    if (this.consumed || this.cancelRequested || this.commandsMutated) return;
    const cleaned = decensor(text).trim();
    if (!cleaned || cleaned === raw.trim()) return;
    const committed = this.composeDraft(this.finalText);
    if (this.readDraft() !== committed) return;
    const polished = this.composeDraft(cleaned);
    this.writeDraft(polished);
    this.segments = [cleaned];
    this.finalText = cleaned;
    this.lastTranscript = cleaned;
    this.setPolishUndo(polished, committed);
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
    // Surface the ring only in the final stretch so it reads as a warning,
    // not a perpetual timer. ≤5s auto-stop → last 2s; longer → last 3s.
    const showMs = Math.min(secs <= 5 ? 2 : 3, secs) * 1000;
    this.silenceTimer = setInterval(() => {
      if (!this.recording) return;
      const elapsedMs = Date.now() - this.lastSpeechAt;
      const remainingMs = secs * 1000 - elapsedMs;
      if (remainingMs <= 0) {
        this.silenceFrac = null;
        this.clearSilenceWatch();
        void this.stop();
        return;
      }
      this.silenceFrac = remainingMs <= showMs ? remainingMs / showMs : null;
    }, 250);
  }

  private clearSilenceWatch() {
    if (this.silenceTimer) {
      clearInterval(this.silenceTimer);
      this.silenceTimer = null;
    }
    this.silenceFrac = null;
  }

  // ---- Cleanup-polish undo ---------------------------------------------------

  /** Arm the raw-transcript restore point. Auto-expires. */
  private setPolishUndo(committed: string, original: string) {
    this.polishUndo = { committed, original };
    if (this.polishUndoTimer) clearTimeout(this.polishUndoTimer);
    this.polishUndoTimer = setTimeout(() => (this.polishUndo = null), 15000);
  }

  /** Flip the draft back to the raw (pre-cleanup) transcript. */
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
    // Finals commit solid into the draft; the in-flight interim rides the
    // ghost tail (display-only) until the recogniser finalises it.
    const composed = this.composeDraft(this.finalText);
    if (this.readDraft() !== composed) this.writeDraft(composed);
    this.ghostTail = interim.trim();
    this.lastTranscript = this.finalText;
    // "send it" — draft is committed above; the composer's effect fires it.
    if (this.pendingSend) {
      this.pendingSend = false;
      if (this.config.cleanup_enabled && /\*{2,}/.test(this.finalText)) {
        // Engine-masked profanity that decensor() couldn't resolve (fully
        // masked, no leading letter) only gets restored by the cleanup pass —
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
      const applied = applyInlineCommands(seg);
      if (applied !== seg) this.commandsMutated = true;
      seg = applied;
      if (SCRATCH_CMD_RE.test(seg)) {
        this.commandsMutated = true;
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
        this.commandsMutated = true;
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

  /** Committed text only — interim speech renders via `ghostTail`, never here. */
  private composeDraft(text: string): string {
    const tail = text.trim();
    if (!this.baseDraft) return tail;
    const sep = /\s$/.test(this.baseDraft) ? "" : " ";
    return this.baseDraft + sep + tail;
  }

  private onError(e: SpeechRecognitionErrorEvent) {
    // "aborted" only ever results from our own abort() calls (consume on send,
    // cancel, engine-switch restart) — not a failure. Surfacing it left the mic
    // permanently red ("Recording cancelled.") after every dictated send, since
    // lastError is only cleared on the next start().
    if (e.error === "aborted") return;
    // Friendly message already surfaced via `lastError`; the raw code is
    // dropped to avoid the #22 / #250 console-noise regression.
    this.lastError = errorMessage(e.error, e.message);
    // #81: permission / no-mic are hard session failures — toast them.
    // Transients (no-speech, network blips) stay outline-only.
    if (e.error === "not-allowed" || e.error === "service-not-allowed" || e.error === "audio-capture") {
      this.failToast();
    }
  }

  /** #81: a session that fails to start (or dies on a hard error) only turned
   *  the mic red — easy to miss mid-conversation. Auto-expiring danger toast
   *  (8s default) carrying the same friendly message as the tooltip. */
  private failToast() {
    if (this.lastError) notify.danger("Voice input failed", { detail: this.lastError });
  }

  private onEnd() {
    this.ghostTail = "";
    this.starting = false;
    // #175: commit only if neither user-cancel nor composer-consume fired.
    const commit = !this.cancelRequested && !this.consumed;
    if (commit) {
      // #40d: onResult already streamed the final text into the draft — only
      // rewrite if there's an actual delta, so the textarea doesn't flash the
      // whole phrase back in at stop.
      const composed = this.composeDraft(this.finalText);
      if (this.readDraft() !== composed) this.writeDraft(composed);
      // Session ended normally with text committed → any mid-session transient
      // (e.g. a no-speech during a pause) is stale; don't leave the mic red
      // after a successful dictation. Hard failures (permission, no mic)
      // produce no committed text and keep their error.
      if (this.finalText.trim()) this.lastError = null;
    }
    this.recording = false;
    this.transcribing = false;
    this.recognition = null;
    this.clearTranscribeTimer();
    this.clearSilenceWatch();
    this.stopWebMeter();
    // Web Speech finals never got the cleanup polish (Whisper-only until now) —
    // run it post-commit so punctuation lands and any leftover engine-masked
    // profanity ("******") gets restored from context. Skipped when a voice
    // command is about to send — the text is leaving now.
    if (commit && !this.sendRequested) void this.polishWebSpeechFinal();
  }

  /** Claude cleanup for a finished Web Speech dictation. Replaces the dictated
   *  tail of the draft only if the user hasn't sent, cancelled, or typed over
   *  it while the polish was in flight. Any failure leaves the raw transcript
   *  — same never-lose-the-transcript contract as the Whisper path. */
  private async polishWebSpeechFinal() {
    if (this.polishing) return; // pre-send pass already in flight — don't double-spawn from onEnd
    const raw = this.finalText.trim();
    // The <3-word gate skips cleanup for short dictations — but a masked short
    // phrase ("f*** you", "****") is EXACTLY what needs de-censoring and is the
    // most common profanity case. So when the text carries an asterisk mask,
    // run cleanup regardless of length (the engine censored the speaker; the
    // cleanup pass restores it). Non-masked short phrases still skip — nothing
    // to fix there.
    const hasMask = raw.includes("*");
    if (!this.config.cleanup_enabled || (raw.split(/\s+/).length < 3 && !hasMask)) return;
    const committed = this.composeDraft(raw);
    // Non-blocking: the raw transcript is already committed to the draft and
    // is immediately editable/sendable. The cleanup pass is cosmetic, so it must
    // NOT raise `transcribing` (that disables the mic + blocks the composer,
    // which read as "loading forever"). `polishing` drives only the quiet
    // textarea shimmer; the result swaps in below only if the draft is
    // untouched.
    this.polishing = true;
    const token = ++this.polishGuard;
    // #40a: cap the *visual* well under the backend's 15s timeout — a slow
    // cleanup call shouldn't pulse the committed (already-usable) transcript for
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
        const polished = this.composeDraft(cleaned);
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
// unknown masks pass through for the Claude cleanup pass to resolve from context.
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
function applyInlineCommands(t: string): string {
  return t
    .replace(/(?:^|\s)new paragraph[.,]?(?=\s|$)/gi, "\n\n")
    .replace(/(?:^|\s)new line[.,]?(?=\s|$)/gi, "\n");
}

function decensor(text: string): string {
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
