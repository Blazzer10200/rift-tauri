// Beta-program acknowledgment gate. Shown once on first launch before testers
// use the app — distinct from onboarding (that gates on missing provider auth;
// this gates on a one-time "I understand" for the beta + AI-mistakes notice).
// Bump ACK_VERSION to re-prompt everyone after a material change to the terms.

const ACK_VERSION = 1;
const ACK_KEY = "rift.betaNotice.ack";

function loadAck(): boolean {
  try {
    return parseInt(localStorage.getItem(ACK_KEY) ?? "0", 10) >= ACK_VERSION;
  } catch {
    // localStorage unavailable (private mode / very early boot) → treat as not
    // acknowledged so the notice still shows.
    return false;
  }
}

class BetaNoticeStore {
  /** True once the user has acknowledged the current notice version. Persisted. */
  acknowledged = $state<boolean>(loadAck());

  acknowledge() {
    this.acknowledged = true;
    try {
      localStorage.setItem(ACK_KEY, String(ACK_VERSION));
    } catch {
      /* persistence is best-effort; in-memory flag still suppresses re-show */
    }
  }

  /** Clear the ack — for a "show beta notice again" action in Settings. */
  reset() {
    this.acknowledged = false;
    try {
      localStorage.removeItem(ACK_KEY);
    } catch {
      /* ignore */
    }
  }
}

export const betaNotice = new BetaNoticeStore();
