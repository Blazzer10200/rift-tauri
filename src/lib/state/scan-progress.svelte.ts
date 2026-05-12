export type ScanPhase = "scanning" | "done";

export type ScanResult = {
  push: number;
  pull: number;
  conflicts: number;
  error?: string;
};

class ScanProgressStore {
  active = $state(false);
  phase = $state<ScanPhase>("scanning");
  current = $state(0);
  total = $state(0);
  resource = $state<string>("");
  result = $state<ScanResult | null>(null);
  private dismissTimer: ReturnType<typeof setTimeout> | null = null;

  start() {
    this.clearTimer();
    this.active = true;
    this.phase = "scanning";
    this.current = 0;
    this.total = 0;
    this.resource = "";
    this.result = null;
  }

  progress(current: number, total: number, resource: string) {
    if (!this.active) return;
    this.current = current;
    this.total = total;
    this.resource = resource;
  }

  finish(result: ScanResult) {
    if (!this.active) return;
    this.phase = "done";
    this.result = result;
    // Auto-dismiss after letting the user read the result
    this.dismissTimer = setTimeout(() => this.dismiss(), 4500);
  }

  dismiss() {
    this.clearTimer();
    this.active = false;
  }

  private clearTimer() {
    if (this.dismissTimer !== null) {
      clearTimeout(this.dismissTimer);
      this.dismissTimer = null;
    }
  }
}

export const scanProgress = new ScanProgressStore();
