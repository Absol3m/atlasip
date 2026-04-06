import { ALL_COLUMNS, DEFAULT_VISIBLE_COLUMNS } from '$lib/types/ip';
import type { IpRecord, SortState } from '$lib/types/ip';

class ResultsStore {
  results = $state<IpRecord[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);
  selected = $state(new Set<string>());
  visibleColumns = $state<string[]>([...DEFAULT_VISIBLE_COLUMNS]);
  columnOrder = $state<string[]>(ALL_COLUMNS.map(c => c.key));
  searchQuery = $state('');
  sortStates = $state<SortState[]>([]);
  pageSize = $state<20 | 50 | 100>(20);
  currentPage = $state(1);
  scrollTop = 0;
  scrollLeft = 0;

  setResults(records: IpRecord[]) {
    this.results = records.map((r, i) => ({ ...r, order: i + 1 }));
    this.selected = new Set();
    this.currentPage = 1;
  }

  toggleSelect(
    id: string,
    shiftKey: boolean,
    ctrlKey: boolean,
    filteredIds: string[],
    lastSelectedId: string | null,
  ) {
    const next = new Set(this.selected);
    if (ctrlKey) {
      if (next.has(id)) next.delete(id);
      else next.add(id);
    } else if (shiftKey && lastSelectedId && filteredIds.length > 0) {
      const from = filteredIds.indexOf(lastSelectedId);
      const to = filteredIds.indexOf(id);
      if (from !== -1 && to !== -1) {
        const [lo, hi] = from < to ? [from, to] : [to, from];
        for (let i = lo; i <= hi; i++) next.add(filteredIds[i]);
      } else {
        next.add(id);
      }
    } else {
      next.clear();
      next.add(id);
    }
    this.selected = next;
  }

  selectAll(ids: string[]) {
    this.selected = new Set(ids);
  }

  clearSelection() {
    this.selected = new Set();
  }

  setVisibleColumns(cols: string[]) {
    // ip column is always visible
    this.visibleColumns = cols.includes('ip') ? cols : ['ip', ...cols];
  }

  reorderColumns(fromIndex: number, toIndex: number) {
    const order = [...this.columnOrder];
    const [col] = order.splice(fromIndex, 1);
    order.splice(toIndex, 0, col);
    this.columnOrder = order;
  }

  setSearchQuery(q: string) {
    this.searchQuery = q;
    this.currentPage = 1;
  }

  toggleSort(key: string, multi: boolean) {
    const states = [...this.sortStates];
    const idx = states.findIndex(s => s.key === key);

    if (multi) {
      if (idx >= 0) {
        if (states[idx].dir === 'asc') {
          states[idx] = { key, dir: 'desc' };
        } else {
          states.splice(idx, 1);
        }
      } else {
        states.push({ key, dir: 'asc' });
      }
    } else {
      if (idx >= 0 && states.length === 1) {
        states[0] = { key, dir: states[0].dir === 'asc' ? 'desc' : 'asc' };
      } else {
        const prevDir = idx >= 0 ? states[idx].dir : null;
        states.length = 0;
        states.push({ key, dir: prevDir === 'asc' ? 'desc' : 'asc' });
      }
    }
    this.sortStates = states;
  }

  setPageSize(size: 20 | 50 | 100) {
    this.pageSize = size;
    this.currentPage = 1;
  }

  setPage(page: number) {
    this.currentPage = page;
  }
}

export const resultsStore = new ResultsStore();
