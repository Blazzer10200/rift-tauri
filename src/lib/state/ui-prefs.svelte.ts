type Density = "compact" | "regular" | "comfy";
type Presence = "calm" | "bold";
type CodePrefs = { fontSize: number; tabWidth: number; ligatures: boolean };

const STORAGE_KEY = "rift.ui.density.v1";
const RAIL_PINNED_KEY = "rift.ui.rail-pinned.v1";
const ACCENT_KEY = "rift.ui.accent.v1";
const PRESENCE_KEY = "rift.ui.presence.v1";
const CODE_KEY = "rift.ui.code.v1";
const LAUNCH_AT_LOGIN_KEY = "rift.ui.launch-at-login.v1";
const RESTORE_SESSION_KEY = "rift.ui.restore-session.v1";
const CONFIRM_ON_QUIT_KEY = "rift.ui.confirm-on-quit.v1";
const RCON_AUTO_RECONNECT_KEY = "rift.ui.rcon-auto-reconnect.v1";

// 8 curated accent hues — one hue drives the whole accent ramp via --accent-h.
export type AccentSwatch = { id: string; label: string; hue: number };
export const ACCENTS: AccentSwatch[] = [
  { id: "emerald", label: "Emerald", hue: 163 },
  { id: "teal", label: "Teal", hue: 195 },
  { id: "sky", label: "Sky", hue: 230 },
  { id: "violet", label: "Violet", hue: 275 },
  { id: "magenta", label: "Magenta", hue: 328 },
  { id: "rose", label: "Rose", hue: 12 },
  { id: "amber", label: "Amber", hue: 70 },
  { id: "lime", label: "Lime", hue: 130 },
];

const DEFAULT_CODE: CodePrefs = { fontSize: 13, tabWidth: 2, ligatures: false };

class UiPrefs {
  density = $state<Density>("compact");
  railPinned = $state(false);
  accentHue = $state(163);
  presence = $state<Presence>("calm");
  code = $state<CodePrefs>({ ...DEFAULT_CODE });
  // On-machine intent flags — no OS-level enforcement; stored as user intent.
  launchAtLogin = $state(false);
  restoreSession = $state(false);
  confirmOnQuit = $state(false);
  rconAutoReconnect = $state(false);

  init() {
    if (typeof window === "undefined") return;
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "compact" || raw === "regular" || raw === "comfy") {
      this.density = raw;
    }
    this.railPinned = localStorage.getItem(RAIL_PINNED_KEY) === "1";

    const accentRaw = localStorage.getItem(ACCENT_KEY);
    if (accentRaw !== null) {
      const hue = Number(accentRaw);
      if (Number.isFinite(hue) && hue >= 0 && hue <= 360) this.accentHue = hue;
    }

    const pres = localStorage.getItem(PRESENCE_KEY);
    if (pres === "calm" || pres === "bold") this.presence = pres;

    try {
      const c = JSON.parse(localStorage.getItem(CODE_KEY) ?? "null");
      if (c && typeof c === "object") this.code = { ...DEFAULT_CODE, ...c };
    } catch {
      /* malformed code prefs — fall back to defaults */
    }

    this.launchAtLogin = localStorage.getItem(LAUNCH_AT_LOGIN_KEY) === "1";
    this.restoreSession = localStorage.getItem(RESTORE_SESSION_KEY) === "1";
    this.confirmOnQuit = localStorage.getItem(CONFIRM_ON_QUIT_KEY) === "1";
    this.rconAutoReconnect = localStorage.getItem(RCON_AUTO_RECONNECT_KEY) === "1";

    this.apply();
  }

  setDensity(d: Density) {
    this.density = d;
    localStorage.setItem(STORAGE_KEY, d);
    this.apply();
  }

  toggleRailPinned() {
    this.railPinned = !this.railPinned;
    localStorage.setItem(RAIL_PINNED_KEY, this.railPinned ? "1" : "0");
    this.applyRail();
  }

  setAccentHue(h: number) {
    this.accentHue = h;
    localStorage.setItem(ACCENT_KEY, String(h));
    this.applyAccent();
  }

  setPresence(p: Presence) {
    this.presence = p;
    localStorage.setItem(PRESENCE_KEY, p);
    this.applyPresence();
  }

  setCode(patch: Partial<CodePrefs>) {
    this.code = { ...this.code, ...patch };
    localStorage.setItem(CODE_KEY, JSON.stringify(this.code));
    this.applyCode();
  }

  toggleLaunchAtLogin() {
    this.launchAtLogin = !this.launchAtLogin;
    localStorage.setItem(LAUNCH_AT_LOGIN_KEY, this.launchAtLogin ? "1" : "0");
  }

  toggleRestoreSession() {
    this.restoreSession = !this.restoreSession;
    localStorage.setItem(RESTORE_SESSION_KEY, this.restoreSession ? "1" : "0");
  }

  toggleConfirmOnQuit() {
    this.confirmOnQuit = !this.confirmOnQuit;
    localStorage.setItem(CONFIRM_ON_QUIT_KEY, this.confirmOnQuit ? "1" : "0");
  }

  toggleRconAutoReconnect() {
    this.rconAutoReconnect = !this.rconAutoReconnect;
    localStorage.setItem(RCON_AUTO_RECONNECT_KEY, this.rconAutoReconnect ? "1" : "0");
  }

  private apply() {
    if (typeof document === "undefined") return;
    document.documentElement.dataset.density = this.density;
    this.applyRail();
    this.applyAccent();
    this.applyPresence();
    this.applyCode();
  }

  private applyRail() {
    if (typeof document !== "undefined") {
      document.documentElement.style.setProperty("--rail-w", this.railPinned ? "220px" : "48px");
    }
  }

  private applyAccent() {
    if (typeof document !== "undefined") {
      document.documentElement.style.setProperty("--accent-h", String(this.accentHue));
    }
  }

  private applyPresence() {
    if (typeof document !== "undefined") {
      document.documentElement.dataset.presence = this.presence;
    }
  }

  private applyCode() {
    if (typeof document === "undefined") return;
    const r = document.documentElement;
    r.style.setProperty("--code-fs", `${this.code.fontSize}px`);
    r.style.setProperty("--code-tab", String(this.code.tabWidth));
    r.dataset.ligatures = this.code.ligatures ? "on" : "off";
  }
}

export const uiPrefs = new UiPrefs();
