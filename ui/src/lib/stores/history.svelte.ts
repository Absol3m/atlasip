const LS_KEY = 'atlasip.history';
const MAX_ENTRIES = 100;

export interface HistoryEntry {
  id: string;
  targets: string[];
  timestamp: number;
  resultCount: number;
}

function load(): HistoryEntry[] {
  try {
    return JSON.parse(localStorage.getItem(LS_KEY) ?? '[]');
  } catch {
    return [];
  }
}

function save(entries: HistoryEntry[]) {
  try {
    localStorage.setItem(LS_KEY, JSON.stringify(entries));
  } catch { /* ignore */ }
}

class HistoryStore {
  entries = $state<HistoryEntry[]>([]);

  init() {
    this.entries = load();
  }

  add(targets: string[], resultCount: number) {
    const entry: HistoryEntry = {
      id: crypto.randomUUID(),
      targets,
      timestamp: Date.now(),
      resultCount,
    };
    this.entries = [entry, ...this.entries].slice(0, MAX_ENTRIES);
    save(this.entries);
  }

  remove(id: string) {
    this.entries = this.entries.filter(e => e.id !== id);
    save(this.entries);
  }

  clear() {
    this.entries = [];
    save([]);
  }
}

export const historyStore = new HistoryStore();
