// Speech-to-text state.
//
// Uses the browser's Web Speech API (WebView2 ships Edge's Azure-backed
// recogniser). Live interim text streams into the composer as the user
// speaks; the final committed transcript replaces interim segments segment-
// by-segment. No Rust-side audio capture or model — only settings are
// persisted backend-side via `stt_get_config` / `stt_set_config`.

import { invoke } from "@tauri-apps/api/core";
import { assistant } from "./assistant.svelte";

export type SttConfig = {
  enabled: boolean;
  language: string;
  append_to_draft: boolean;
  continuous: boolean;
  show_interim: boolean;
};

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
  });
  configLoaded = $state(false);

  /** Whether the WebView exposes a SpeechRecognition implementation. */
  supported = $state(false);
  recording = $state(false);
  /** True while we've stopped but final results may still arrive. */
  transcribing = $state(false);
  lastError = $state<string | null>(null);
  lastTranscript = $state<string>("");

  private recognition: SpeechRecognitionInstance | null = null;
  private initStarted = false;
  // Composer draft snapshot at the moment recording started — interim +
  // final transcripts are concatenated onto this so a partway result doesn't
  // wipe what the user typed before talking.
  private baseDraft = "";
  // Accumulated final-only transcript across the current recording session.
  private finalText = "";
  // Set true when the composer has consumed the current draft (send / slash
  // fire). Late-arriving onResult / onEnd writes are dropped until the next
  // start() — otherwise queued finals re-paste what the user just sent.
  private consumed = false;
  // Watchdog timer for the post-stop "transcribing" window. Some WebView2
  // builds never fire onend after stop() if no finals are pending; without
  // this the mic button stays disabled forever.
  private transcribeTimer: ReturnType<typeof setTimeout> | null = null;
  // Debounce token for setConfig-driven restart — prevents double-start
  // when the user toggles multiple options in quick succession.
  private restartToken = 0;

  async init() {
    if (this.initStarted) return;
    this.initStarted = true;
    this.supported = getSRCtor() !== null;
    try {
      this.config = await invoke<SttConfig>("stt_get_config");
      this.configLoaded = true;
    } catch (e) {
      console.warn("[stt] load config failed:", e);
      this.configLoaded = true;
    }
  }

  async setConfig(patch: Partial<SttConfig>) {
    const next: SttConfig = { ...this.config, ...patch };
    this.config = next;
    try {
      await invoke("stt_set_config", { config: next });
    } catch (e) {
      this.lastError = `Save settings failed: ${e}`;
    }
    if (this.recording && this.recognition) {
      // Restart recognition so language/continuous/interim changes take
      // effect. Debounced via token so rapid toggles don't double-start.
      const token = ++this.restartToken;
      this.recognition.abort();
      this.recording = false;
      setTimeout(() => {
        if (token === this.restartToken) void this.start();
      }, 120);
    }
  }

  /** Composer-side hook: the current draft was just sent / cleared. Resets
   *  the transcript baseline so queued late finals don't re-paste it. */
  consume() {
    this.baseDraft = "";
    this.finalText = "";
    this.consumed = true;
    this.lastTranscript = "";
  }

  /** Begin live recognition. Returns false if unavailable / disabled. */
  async start(): Promise<boolean> {
    if (!this.supported) {
      this.lastError = "Speech recognition is not available in this WebView.";
      return false;
    }
    if (!this.config.enabled) {
      this.lastError = "Speech-to-text is disabled. Enable it in Settings → Speech.";
      return false;
    }
    if (this.recording) return true;
    const Ctor = getSRCtor();
    if (!Ctor) return false;
    this.lastError = null;
    this.baseDraft = this.config.append_to_draft ? assistant.composerDraft : "";
    this.finalText = "";
    this.consumed = false;
    this.clearTranscribeTimer();

    const r = new Ctor();
    r.lang = this.config.language || "en-US";
    r.continuous = this.config.continuous;
    r.interimResults = this.config.show_interim;
    // S91: request 3 alternates so onResult can pick the highest-confidence
    // variant. WebView2's Azure backend sometimes ranks a cleaner alternate
    // above the slurred primary; defaulting to alt[0] missed those wins.
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

  /** End live recognition. Final results that are already queued still fire
   *  before `onend`; we return the committed transcript via `lastTranscript`. */
  async stop(): Promise<string> {
    if (!this.recognition) return this.lastTranscript;
    try {
      this.recognition.stop();
      this.transcribing = true;
      // Watchdog: if onend doesn't fire within 4s (seen on some WebView2
      // builds when no finals are pending), force-clear the transcribing
      // flag so the mic button isn't stuck disabled.
      this.clearTranscribeTimer();
      this.transcribeTimer = setTimeout(() => {
        if (this.transcribing) {
          this.transcribing = false;
          this.recognition = null;
        }
      }, 4000);
    } catch (e) {
      console.warn("[stt] stop failed:", e);
    }
    return this.lastTranscript;
  }

  /** Hard-cancel — drop interim text, restore the original draft. */
  async cancel() {
    if (!this.recognition) return;
    try {
      this.recognition.abort();
    } catch {
      /* recogniser may already be stopped */
    }
    // Only restore the pre-recording draft if the user hasn't already sent.
    // Restoring after a send re-pastes the original text the user just shipped.
    if (!this.consumed) {
      assistant.composerDraft = this.baseDraft;
    }
    this.finalText = "";
    this.recording = false;
    this.transcribing = false;
    this.recognition = null;
    this.clearTranscribeTimer();
  }

  private clearTranscribeTimer() {
    if (this.transcribeTimer) {
      clearTimeout(this.transcribeTimer);
      this.transcribeTimer = null;
    }
  }

  private onResult(e: SpeechRecognitionEvent) {
    // After a send/clear, drop late-arriving results so they don't re-paste
    // the committed text. Caller (Composer) restarts via toggleMic if the
    // user wants to keep dictating.
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
    const friendly = errorMessage(e.error, e.message);
    this.lastError = friendly;
    // `no-speech` / `aborted` are routine; don't toast loudly.
    if (e.error !== "no-speech" && e.error !== "aborted") {
      console.warn("[stt] recognition error:", e.error, e.message);
    }
  }

  private onEnd() {
    // Commit any interim text by re-composing with empty interim — but only
    // if the draft hasn't been consumed (send), to avoid re-pasting.
    if (this.recognition && !this.consumed) {
      assistant.composerDraft = this.composeDraft(this.finalText, "");
    }
    this.recording = false;
    this.transcribing = false;
    this.recognition = null;
    this.clearTranscribeTimer();
  }
}

// Pick the alternate with the highest confidence. Falls back to alt[0]
// when confidence is uniformly 0 (some WebView2 builds return 0 for every
// alternate — the spec allows it). Tolerates slurred input by letting a
// lower-ranked-but-more-confident variant win when the engine bothers to
// score them. Empty / missing alternates -> empty string.
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
