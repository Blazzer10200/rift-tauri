// #7 first-run onboarding gate state. Tracks whether the guided flow has been
// dismissed (completed or skipped) so it shows at most once per install. The
// actual "is this a fresh install" decision lives in AppShell, combining this
// flag with the assistant auth probe result (no API key + not logged in).

const DISMISS_KEY = "rift.onboarding.dismissed";

function loadDismissed(): boolean {
  try {
    return localStorage.getItem(DISMISS_KEY) === "1";
  } catch {
    // localStorage unavailable (private mode / very early boot) → treat as not
    // dismissed; the gate's other conditions still protect existing users.
    return false;
  }
}

class OnboardingStore {
  /** True once the user has finished or skipped onboarding. Persisted. */
  dismissed = $state<boolean>(loadDismissed());

  dismiss() {
    this.dismissed = true;
    try {
      localStorage.setItem(DISMISS_KEY, "1");
    } catch {
      /* persistence is best-effort; in-memory flag still suppresses re-show */
    }
  }

  /** Clear the dismissed flag — for a future "restart onboarding" action. */
  reset() {
    this.dismissed = false;
    try {
      localStorage.removeItem(DISMISS_KEY);
    } catch {
      /* ignore */
    }
  }
}

export const onboarding = new OnboardingStore();
