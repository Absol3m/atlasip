const LS_KEY = 'atlasip.history';
const MAX_ENTRIES = 100;

export interface HistoryEntry {
  id: string;
  targets: string[];
  timestamp: number;
  resultCount: number;
}

function isValidEntry(e: unknown): e is HistoryEntry {
  return (
    typeof e === 'object' && e !== null &&
    typeof (e as HistoryEntry).id === 'string' &&
    Array.isArray((e as HistoryEntry).targets) &&
    typeof (e as HistoryEntry).timestamp === 'number' &&
    typeof (e as HistoryEntry).resultCount === 'number'
  );
}

function load(): HistoryEntry[] {
  try {
    const raw = JSON.parse(localStorage.getItem(LS_KEY) ?? '[]');
    if (!Array.isArray(raw)) return [];
    return raw.filter(isValidEntry);
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

  removeMany(ids: string[]) {
    const set = new Set(ids);
    this.entries = this.entries.filter(e => !set.has(e.id));
    save(this.entries);
  }

  clear() {
    this.entries = [];
    save([]);
  }
}

export const historyStore = new HistoryStore();
