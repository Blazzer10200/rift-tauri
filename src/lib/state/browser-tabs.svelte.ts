export type BrowserTab = {
  id: string;
  name: string;
  localPath: string;
  remotePath: string;
};

const STORAGE_KEY = "rift.browser.tabs.v1";

type Persisted = {
  tabs: BrowserTab[];
  activeIdx: number;
};

function loadPersisted(): Persisted | null {
  if (typeof localStorage === "undefined") return null;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Persisted;
    if (!Array.isArray(parsed.tabs)) return null;
    return parsed;
  } catch {
    return null;
  }
}

function savePersisted(tabs: BrowserTab[], activeIdx: number) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ tabs, activeIdx }));
  } catch {}
}

class BrowserTabsStore {
  tabs = $state<BrowserTab[]>([]);
  activeIdx = $state(0);

  active = $derived(this.tabs[this.activeIdx] ?? null);

  hydrate(defaultLocal: string, defaultRemote: string) {
    const p = loadPersisted();
    if (p && p.tabs.length > 0) {
      this.tabs = p.tabs;
      this.activeIdx = Math.max(0, Math.min(p.activeIdx, p.tabs.length - 1));
      return;
    }
    this.tabs = [
      {
        id: crypto.randomUUID(),
        name: "default",
        localPath: defaultLocal,
        remotePath: defaultRemote,
      },
    ];
    this.activeIdx = 0;
    this.persist();
  }

  reset() {
    this.tabs = [];
    this.activeIdx = 0;
    if (typeof localStorage !== "undefined") {
      try { localStorage.removeItem(STORAGE_KEY); } catch {}
    }
  }

  open(name: string, localPath: string, remotePath: string) {
    const next: BrowserTab = {
      id: crypto.randomUUID(),
      name,
      localPath,
      remotePath,
    };
    this.tabs = [...this.tabs, next];
    this.activeIdx = this.tabs.length - 1;
    this.persist();
  }

  close(idx: number) {
    if (idx < 0 || idx >= this.tabs.length) return;
    if (this.tabs.length === 1) return;
    const next = this.tabs.filter((_, i) => i !== idx);
    this.tabs = next;
    if (this.activeIdx >= next.length) this.activeIdx = next.length - 1;
    else if (idx < this.activeIdx) this.activeIdx -= 1;
    this.persist();
  }

  setActive(idx: number) {
    if (idx >= 0 && idx < this.tabs.length) {
      this.activeIdx = idx;
      this.persist();
    }
  }

  updateLocalPath(idx: number, path: string) {
    if (idx < 0 || idx >= this.tabs.length) return;
    const next = [...this.tabs];
    next[idx] = { ...next[idx], localPath: path };
    this.tabs = next;
    this.persist();
  }

  updateRemotePath(idx: number, path: string) {
    if (idx < 0 || idx >= this.tabs.length) return;
    const next = [...this.tabs];
    next[idx] = { ...next[idx], remotePath: path };
    this.tabs = next;
    this.persist();
  }

  private persist() {
    savePersisted(this.tabs, this.activeIdx);
  }
}

export const browserTabs = new BrowserTabsStore();
