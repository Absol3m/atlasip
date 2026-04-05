<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    Search, Columns3, Download, Copy, MapPin, Eye,
    ChevronUp, ChevronDown, ChevronsUpDown, X, Braces,
    FileText, Table2, Maximize2,
  } from 'lucide-svelte';
  import { fade, fly } from 'svelte/transition';
  import { resultsStore } from '$lib/stores/results.svelte';
  import { ALL_COLUMNS } from '$lib/types/ip';
  import type { IpRecord, ColumnDef, SortState } from '$lib/types/ip';
  import ResultRow from './ResultRow.svelte';

  // ── Derived: visible + ordered columns ──────────────────────────────────────
  let visibleCols = $derived.by<ColumnDef[]>(() => {
    const order = resultsStore.columnOrder;
    const visible = new Set(resultsStore.visibleColumns);
    return ALL_COLUMNS
      .slice()
      .sort((a, b) => order.indexOf(a.key) - order.indexOf(b.key))
      .filter(c => visible.has(c.key));
  });

  // ── Derived: filtered rows ───────────────────────────────────────────────────
  let filtered = $derived.by<IpRecord[]>(() => {
    const q = resultsStore.searchQuery.toLowerCase().trim();
    if (!q) return resultsStore.results;
    return resultsStore.results.filter(r =>
      visibleCols.some(col => col.getValue(r).toLowerCase().includes(q)),
    );
  });

  // ── Derived: sorted rows ─────────────────────────────────────────────────────
  let sorted = $derived.by<IpRecord[]>(() => {
    const states = resultsStore.sortStates;
    if (states.length === 0) return filtered;
    return [...filtered].sort((a, b) => {
      for (const { key, dir } of states) {
        const col = ALL_COLUMNS.find(c => c.key === key);
        if (!col) continue;
        const cmp = col.getValue(a).localeCompare(col.getValue(b), undefined, { numeric: true });
        if (cmp !== 0) return dir === 'asc' ? cmp : -cmp;
      }
      return 0;
    });
  });

  // ── Derived: paginated rows ──────────────────────────────────────────────────
  let paginated = $derived.by<IpRecord[]>(() => {
    const start = (resultsStore.currentPage - 1) * resultsStore.pageSize;
    return sorted.slice(start, start + resultsStore.pageSize);
  });

  let totalPages = $derived(Math.max(1, Math.ceil(filtered.length / resultsStore.pageSize)));
  let filteredIds = $derived(filtered.map(r => r.id));
  let allSelected = $derived(
    paginated.length > 0 && paginated.every(r => resultsStore.selected.has(r.id)),
  );

  // ── Column widths ─────────────────────────────────────────────────────────────
  const LS_KEY = 'atlasip.columnWidths';
  let columnWidths = $state<Record<string, number>>({});
  // Non-reactive flag: prevents drag-to-reorder from firing during a resize drag.
  let _resizing = false;
  // Reactive key of the column currently being resized (drives visual feedback).
  let resizingKey = $state<string | null>(null);

  function loadFromStorage(): Record<string, number> {
    try {
      return JSON.parse(localStorage.getItem(LS_KEY) ?? '{}');
    } catch {
      return {};
    }
  }

  function saveToStorage() {
    try {
      // Spread into a plain object to avoid serialising Svelte proxy internals
      localStorage.setItem(LS_KEY, JSON.stringify({ ...columnWidths }));
    } catch { /* localStorage unavailable (e.g. private browsing) */ }
  }

  function getColWidth(key: string): number {
    return columnWidths[key] ?? ALL_COLUMNS.find(c => c.key === key)?.minWidth ?? 80;
  }

  // ── Canvas text measurement ──────────────────────────────────────────────────
  let _canvas: HTMLCanvasElement | null = null;

  function measureText(text: string, font: string): number {
    if (typeof document === 'undefined') return 0;
    if (!_canvas) _canvas = document.createElement('canvas');
    const ctx = _canvas.getContext('2d');
    if (!ctx) return 0;
    ctx.font = font;
    return ctx.measureText(text).width;
  }

  // Match the actual CSS: header = 600 12px uppercase, cells = 13px
  const HEADER_FONT = '600 12px Inter, system-ui, sans-serif';
  const CELL_FONT   = '13px Inter, system-ui, sans-serif';
  const HEADER_PAD  = 48; // 12 + 12 padding + ~24 sort-icon space
  const CELL_PAD    = 24; // 12 + 12 padding

  function computeAutoWidth(col: ColumnDef, rows: IpRecord[]): number {
    const headerW = measureText(col.label.toUpperCase(), HEADER_FONT) + HEADER_PAD;
    let maxW = headerW;
    for (const row of rows) {
      const w = measureText(col.getValue(row), CELL_FONT) + CELL_PAD;
      if (w > maxW) maxW = w;
    }
    return Math.max(col.minWidth, Math.min(maxW, 520));
  }

  // Auto-size a single column (on double-click of resize handle or explicit call)
  function autoSizeColumn(key: string) {
    const col = ALL_COLUMNS.find(c => c.key === key);
    if (!col) return;
    columnWidths[key] = computeAutoWidth(col, resultsStore.results);
    saveToStorage();
  }

  // Auto-size every visible column
  function autoSizeAllColumns() {
    for (const col of visibleCols) {
      columnWidths[col.key] = computeAutoWidth(col, resultsStore.results);
    }
    saveToStorage();
  }

  // Load persisted widths on mount (before first paint)
  onMount(() => {
    const saved = loadFromStorage();
    Object.assign(columnWidths, saved);
  });

  // When results arrive, auto-size any column that has no stored/manual width
  $effect(() => {
    const rows = resultsStore.results;
    if (rows.length === 0) return;
    untrack(() => {
      let changed = false;
      for (const col of ALL_COLUMNS) {
        if (columnWidths[col.key] === undefined) {
          columnWidths[col.key] = computeAutoWidth(col, rows);
          changed = true;
        }
      }
      if (changed) saveToStorage();
    });
  });

  // ── Manual column resize (drag on right border of <th>) ──────────────────────
  function startResize(e: MouseEvent, key: string) {
    e.preventDefault();   // prevent text selection & native drag
    e.stopPropagation();  // don't bubble to <th> (sort/drag-reorder)

    _resizing  = true;
    resizingKey = key;

    const startX = e.clientX;
    const startW = getColWidth(key);
    const minW   = ALL_COLUMNS.find(c => c.key === key)?.minWidth ?? 60;

    // Override body cursor for the entire drag so it stays col-resize even
    // when the mouse leaves the handle area (prevents flickering).
    const prevCursor    = document.body.style.cursor;
    const prevUserSelect = document.body.style.userSelect;
    document.body.style.cursor     = 'col-resize';
    document.body.style.userSelect = 'none';

    function onMove(ev: MouseEvent) {
      columnWidths[key] = Math.max(minW, startW + ev.clientX - startX);
    }

    function onUp() {
      // Restore body styles before clearing _resizing so any queued dragstart
      // that fires in this tick still sees _resizing = true.
      document.body.style.cursor     = prevCursor;
      document.body.style.userSelect = prevUserSelect;
      resizingKey = null;
      // Small timeout so ondragstart (if it somehow fires) sees _resizing = true.
      setTimeout(() => { _resizing = false; }, 0);
      saveToStorage();
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup',   onUp);
    }

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup',   onUp);
  }

  // ── UI state ─────────────────────────────────────────────────────────────────
  let showDrawer  = $state(false);
  let dragFromKey = $state<string | null>(null);
  let dragOverKey = $state<string | null>(null);
  let lastSelectedId = $state<string | null>(null);

  interface CtxMenu { x: number; y: number; record: IpRecord; colKey: string | null }
  let contextMenu = $state<CtxMenu | null>(null);

  // Close context menu on any outside click
  $effect(() => {
    if (!contextMenu) return;
    function close() { contextMenu = null; }
    document.addEventListener('click', close, { capture: true, once: true });
    return () => document.removeEventListener('click', close, true);
  });

  // ── Sort helpers ─────────────────────────────────────────────────────────────
  function getSortState(key: string): SortState | undefined {
    return resultsStore.sortStates.find(s => s.key === key);
  }

  function getSortPriority(key: string): number {
    const idx = resultsStore.sortStates.findIndex(s => s.key === key);
    return idx >= 0 ? idx + 1 : 0;
  }

  // ── Drag-and-drop column reorder ─────────────────────────────────────────────
  function onDragStart(e: DragEvent, key: string) {
    if (_resizing) { e.preventDefault(); return; }
    dragFromKey = key;
  }

  function onDragOver(e: DragEvent, key: string) {
    e.preventDefault();
    dragOverKey = key;
  }

  function onDrop(key: string) {
    if (!dragFromKey || dragFromKey === key) {
      dragFromKey = null;
      dragOverKey = null;
      return;
    }
    const order = resultsStore.columnOrder;
    const fromIdx = order.indexOf(dragFromKey);
    const toIdx   = order.indexOf(key);
    if (fromIdx !== -1 && toIdx !== -1) {
      resultsStore.reorderColumns(fromIdx, toIdx);
    }
    dragFromKey = null;
    dragOverKey = null;
  }

  // ── Row interactions ─────────────────────────────────────────────────────────
  function handleRowClick(record: IpRecord, e: MouseEvent) {
    resultsStore.toggleSelect(record.id, e.shiftKey, e.ctrlKey || e.metaKey, filteredIds, lastSelectedId);
    lastSelectedId = record.id;
  }

  function handleRowDblClick(record: IpRecord) {
    goto(`/ip/${record.ip}`);
  }

  function handleContextMenu(record: IpRecord, e: MouseEvent) {
    const td = (e.target as HTMLElement).closest('td');
    const colKey = td?.dataset.colkey ?? null;
    contextMenu = { x: e.clientX, y: e.clientY, record, colKey };
  }

  function handleCheckbox(record: IpRecord, e: MouseEvent) {
    resultsStore.toggleSelect(record.id, e.shiftKey, true, filteredIds, lastSelectedId);
    lastSelectedId = record.id;
  }

  function handleHeaderCheckbox() {
    if (allSelected) {
      resultsStore.clearSelection();
    } else {
      resultsStore.selectAll(paginated.map(r => r.id));
    }
  }

  // ── Export helpers ───────────────────────────────────────────────────────────
  function selectedRecords(): IpRecord[] {
    return resultsStore.results.filter(r => resultsStore.selected.has(r.id));
  }

  function downloadFile(content: string, filename: string, mime: string) {
    const blob = new Blob([content], { type: mime });
    const url  = URL.createObjectURL(blob);
    const a    = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  function exportCSV() {
    const rows    = selectedRecords();
    const headers = visibleCols.map(c => `"${c.label}"`).join(',');
    const lines   = rows.map(r =>
      visibleCols.map(c => `"${c.getValue(r).replace(/"/g, '""')}"`).join(','),
    );
    downloadFile([headers, ...lines].join('\n'), 'atlasip_export.csv', 'text/csv');
  }

  function exportTSV() {
    const rows    = selectedRecords();
    const headers = visibleCols.map(c => c.label).join('\t');
    const lines   = rows.map(r => visibleCols.map(c => c.getValue(r)).join('\t'));
    downloadFile([headers, ...lines].join('\n'), 'atlasip_export.tsv', 'text/tab-separated-values');
  }

  function exportJSON() {
    downloadFile(JSON.stringify(selectedRecords(), null, 2), 'atlasip_export.json', 'application/json');
  }

  async function copySelection() {
    await navigator.clipboard.writeText(selectedRecords().map(r => r.ip).join('\n'));
  }

  async function openGoogleMaps(record: IpRecord) {
    const query = [record.address, record.country].filter(Boolean).join(', ');
    if (!query) return;
    await openUrl(`https://maps.google.com/?q=${encodeURIComponent(query)}`);
  }

  function hideColumn(key: string) {
    if (key === 'ip') return;
    resultsStore.setVisibleColumns(resultsStore.visibleColumns.filter(k => k !== key));
  }

  // ── Pagination ───────────────────────────────────────────────────────────────
  const pageSizeOptions: (20 | 50 | 100)[] = [20, 50, 100];

  // ── Drawer derived ───────────────────────────────────────────────────────────
  let visibleCount = $derived(resultsStore.visibleColumns.length);
  let totalCount   = $derived(ALL_COLUMNS.length);
</script>

<div class="grid-root">

  <!-- ── P0-DATAGRID-001: Results header ── -->
  <div class="grid-header">
    <span class="grid-title">
      Results
      <span class="grid-count">({resultsStore.results.length})</span>
    </span>
  </div>

  <!-- ── Toolbar ── -->
  <div class="toolbar">
    <div class="search-wrap">
      <Search size={15} class="search-icon" />
      <input
        class="search-input"
        type="search"
        placeholder="Rechercher…"
        value={resultsStore.searchQuery}
        oninput={(e) => resultsStore.setSearchQuery((e.target as HTMLInputElement).value)}
      />
    </div>
    <span class="row-count">{filtered.length} résultat{filtered.length !== 1 ? 's' : ''}</span>
    <button
      class="btn-icon"
      title="Auto-size toutes les colonnes"
      onclick={autoSizeAllColumns}
    >
      <Maximize2 size={15} />
    </button>
    <button class="btn-icon" title="Colonnes visibles" onclick={() => (showDrawer = true)}>
      <Columns3 size={16} />
    </button>
  </div>

  <!-- ── Action bar (shown when rows are selected) ── -->
  {#if resultsStore.selected.size > 0}
    <div class="action-bar" transition:fly={{ y: -8, duration: 180 }}>
      <span class="selection-count">
        {resultsStore.selected.size} sélectionné{resultsStore.selected.size > 1 ? 's' : ''}
      </span>
      <div class="action-sep"></div>
      <button class="action-btn" title="Copier les IPs" onclick={copySelection}>
        <Copy size={14} /> Copier
      </button>
      <button class="action-btn" title="Export CSV" onclick={exportCSV}>
        <Table2 size={14} /> CSV
      </button>
      <button class="action-btn" title="Export TSV" onclick={exportTSV}>
        <FileText size={14} /> TSV
      </button>
      <button class="action-btn" title="Export JSON" onclick={exportJSON}>
        <Braces size={14} /> JSON
      </button>
      <button class="action-btn" title="Ouvrir dans Google Maps"
        onclick={() => { const r = selectedRecords()[0]; if (r) openGoogleMaps(r); }}
      >
        <MapPin size={14} /> Maps
      </button>
      <button class="action-btn" title="Voir détails"
        onclick={() => { const r = selectedRecords()[0]; if (r) goto(`/ip/${r.ip}`); }}
      >
        <Eye size={14} /> Détails
      </button>
      <button class="action-btn-ghost" title="Désélectionner" onclick={() => resultsStore.clearSelection()}>
        <X size={14} />
      </button>
    </div>
  {/if}

  <!-- ── Data table ── -->
  <div class="table-wrapper">
    <table class="data-grid">
      <thead>
        <tr>
          <!-- Checkbox column (sticky left) -->
          <th class="col-check">
            <input
              type="checkbox"
              checked={allSelected}
              indeterminate={resultsStore.selected.size > 0 && !allSelected}
              onclick={handleHeaderCheckbox}
              title="Tout sélectionner"
            />
          </th>

          {#each visibleCols as col (col.key)}
            {@const sortState    = getSortState(col.key)}
            {@const sortPriority = getSortPriority(col.key)}
            <th
              class:sticky-ip={col.key === 'ip'}
              class:dragging-over={dragOverKey === col.key}
              class:is-resizing={resizingKey === col.key}
              style="width: {getColWidth(col.key)}px; min-width: {col.minWidth}px;"
              draggable="true"
              ondragstart={(e) => onDragStart(e, col.key)}
              ondragover={(e) => onDragOver(e, col.key)}
              ondrop={() => onDrop(col.key)}
              ondragend={() => { dragFromKey = null; dragOverKey = null; }}
              onclick={(e) => resultsStore.toggleSort(col.key, e.shiftKey)}
              title="Clic: trier · Shift+clic: tri multiple · Glisser: réorganiser"
            >
              <span class="th-content">
                <span class="th-label">{col.label}</span>
                {#if sortState}
                  <span class="sort-indicator">
                    {#if sortState.dir === 'asc'}
                      <ChevronUp size={13} />
                    {:else}
                      <ChevronDown size={13} />
                    {/if}
                    {#if resultsStore.sortStates.length > 1}
                      <sup class="sort-priority">{sortPriority}</sup>
                    {/if}
                  </span>
                {:else}
                  <span class="sort-unsorted"><ChevronsUpDown size={12} /></span>
                {/if}
              </span>
              <!-- Resize handle: drag to resize, double-click to auto-size -->
              <button
                class="resize-handle"
                type="button"
                aria-label="Redimensionner la colonne {col.label}"
                title="Drag: redimensionner · Double-clic: auto-size"
                onmousedown={(e) => startResize(e, col.key)}
                ondblclick={(e) => { e.stopPropagation(); autoSizeColumn(col.key); }}
                onclick={(e) => e.stopPropagation()}
              ></button>
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each paginated as record, i (record.id)}
          <ResultRow
            {record}
            columns={visibleCols}
            isSelected={resultsStore.selected.has(record.id)}
            isEven={i % 2 === 1}
            onrowclick={handleRowClick}
            onrowdblclick={handleRowDblClick}
            onrowcontextmenu={handleContextMenu}
            oncheckboxclick={handleCheckbox}
          />
        {:else}
          <tr>
            <td class="empty-state" colspan={visibleCols.length + 1}>
              {resultsStore.results.length === 0
                ? 'Aucune donnée'
                : 'Aucun résultat pour cette recherche'}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <!-- ── Pagination ── -->
  <div class="pagination">
    <div class="page-size">
      <span>Lignes&nbsp;:</span>
      {#each pageSizeOptions as size}
        <button
          class="page-size-btn"
          class:active={resultsStore.pageSize === size}
          onclick={() => resultsStore.setPageSize(size)}
        >{size}</button>
      {/each}
    </div>
    <div class="page-nav">
      <button
        class="page-btn"
        disabled={resultsStore.currentPage <= 1}
        onclick={() => resultsStore.setPage(resultsStore.currentPage - 1)}
      >‹</button>
      <span class="page-info">Page {resultsStore.currentPage} / {totalPages}</span>
      <button
        class="page-btn"
        disabled={resultsStore.currentPage >= totalPages}
        onclick={() => resultsStore.setPage(resultsStore.currentPage + 1)}
      >›</button>
    </div>
  </div>
</div>

<!-- ── Context menu (row right-click) ── -->
{#if contextMenu}
  {@const cm = contextMenu}
  <div
    class="context-menu"
    style="left: {cm.x}px; top: {cm.y}px"
    transition:fade={{ duration: 100 }}
    role="menu"
  >
    <button class="ctx-item" role="menuitem"
      onclick={() => { goto(`/ip/${cm.record.ip}`); contextMenu = null; }}
    >
      <Eye size={14} /> Voir détails
    </button>
    <button class="ctx-item" role="menuitem"
      onclick={async () => { await navigator.clipboard.writeText(cm.record.ip); contextMenu = null; }}
    >
      <Copy size={14} /> Copier IP
    </button>
    <button class="ctx-item" role="menuitem"
      onclick={() => { openGoogleMaps(cm.record); contextMenu = null; }}
    >
      <MapPin size={14} /> Google Maps
    </button>
    <div class="ctx-sep"></div>
    <button class="ctx-item" role="menuitem"
      onclick={() => {
        const headers = visibleCols.map(c => `"${c.label}"`).join(',');
        const line    = visibleCols.map(c => `"${c.getValue(cm.record).replace(/"/g, '""')}"`).join(',');
        downloadFile([headers, line].join('\n'), `${cm.record.ip}.csv`, 'text/csv');
        contextMenu = null;
      }}
    >
      <Download size={14} /> Exporter cette ligne
    </button>
    {#if cm.colKey && cm.colKey !== 'ip'}
      <div class="ctx-sep"></div>
      <button class="ctx-item ctx-muted" role="menuitem"
        onclick={() => { hideColumn(cm.colKey!); contextMenu = null; }}
      >
        Masquer "{visibleCols.find(c => c.key === cm.colKey)?.label ?? cm.colKey}"
      </button>
    {/if}
  </div>
{/if}

<!-- ── Column visibility drawer ── -->
{#if showDrawer}
  <div
    class="drawer-overlay"
    transition:fade={{ duration: 150 }}
    onclick={() => (showDrawer = false)}
    role="presentation"
  ></div>
  <div class="drawer" transition:fly={{ x: 320, duration: 200 }}>
    <div class="drawer-header">
      <div>
        <span class="drawer-title">Colonnes</span>
        <span class="drawer-count">{visibleCount} / {totalCount} affichées</span>
      </div>
      <button class="btn-icon" onclick={() => (showDrawer = false)}><X size={16} /></button>
    </div>

    <div class="drawer-body">
      {#each ALL_COLUMNS.slice().sort((a, b) =>
        resultsStore.columnOrder.indexOf(a.key) - resultsStore.columnOrder.indexOf(b.key)
      ) as col (col.key)}
        <label class="drawer-item" class:disabled={col.key === 'ip'}>
          <input
            type="checkbox"
            checked={resultsStore.visibleColumns.includes(col.key)}
            disabled={col.key === 'ip'}
            onchange={(e) => {
              const checked = (e.target as HTMLInputElement).checked;
              if (checked) {
                resultsStore.setVisibleColumns([...resultsStore.visibleColumns, col.key]);
              } else {
                hideColumn(col.key);
              }
            }}
          />
          <span class="drawer-label">{col.label}</span>
        </label>
      {/each}
    </div>

    <div class="drawer-footer">
      <button class="btn-small"
        onclick={() => resultsStore.setVisibleColumns(ALL_COLUMNS.map(c => c.key))}
      >
        Tout afficher
      </button>
      <button class="btn-small"
        onclick={() => resultsStore.setVisibleColumns(['ip'])}
      >
        Tout masquer
      </button>
      <button class="btn-small"
        onclick={autoSizeAllColumns}
        title="Recalculer toutes les largeurs"
      >
        <Maximize2 size={12} /> Auto-size
      </button>
    </div>
  </div>
{/if}

<style>
  /* ── Root (P2-LAYOUT-001 — flex:1 so pagination stays at bottom) ── */
  .grid-root {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    background: var(--color-surface);
    box-shadow: var(--shadow);
  }

  /* ── P0-DATAGRID-001: Results header ── */
  .grid-header {
    display: flex;
    align-items: center;
    padding: 10px 14px 8px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
    flex-shrink: 0;
  }

  .grid-title {
    font-size: 13.5px;
    font-weight: 700;
    color: var(--color-text);
    letter-spacing: 0.01em;
  }

  .grid-count {
    font-weight: 400;
    color: var(--color-text-muted);
    margin-left: 4px;
  }

  /* ── Toolbar ── */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    max-width: 320px;
  }

  :global(.search-wrap .search-icon) {
    position: absolute;
    left: 8px;
    color: var(--color-text-muted);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 5px 10px 5px 30px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-bg);
    color: var(--color-text);
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s;
  }

  .search-input:focus {
    border-color: var(--color-accent);
  }

  .row-count {
    font-size: 12px;
    color: var(--color-text-muted);
    margin-left: auto;
    white-space: nowrap;
  }

  .btn-icon {
    background: none;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    padding: 5px 7px;
    cursor: pointer;
    color: var(--color-text-muted);
    display: flex;
    align-items: center;
    transition: background-color 0.15s, color 0.15s;
    flex-shrink: 0;
  }

  .btn-icon:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  /* ── Action bar ── */
  .action-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 12px;
    background: color-mix(in srgb, var(--color-accent) 10%, var(--color-surface));
    border-bottom: 1px solid color-mix(in srgb, var(--color-accent) 30%, transparent);
    font-size: 13px;
    flex-wrap: wrap;
  }

  .selection-count {
    font-weight: 600;
    color: var(--color-accent);
    min-width: 90px;
  }

  .action-sep {
    width: 1px;
    height: 20px;
    background: var(--color-border);
    margin: 0 2px;
    flex-shrink: 0;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 5px;
    font-size: 12.5px;
    cursor: pointer;
    color: var(--color-text);
    white-space: nowrap;
    transition: background-color 0.15s;
  }

  .action-btn:hover {
    background: var(--color-hover);
  }

  .action-btn-ghost {
    margin-left: auto;
    background: none;
    border: none;
    padding: 4px 6px;
    cursor: pointer;
    color: var(--color-text-muted);
    border-radius: 4px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
  }

  .action-btn-ghost:hover {
    color: var(--color-error);
  }

  /* ── Table wrapper (P2-LAYOUT-001 — fills available height; pagination stays visible) ── */
  .table-wrapper {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  /* ── Data grid ── */
  .data-grid {
    border-collapse: collapse;
    table-layout: fixed;
    /* Width = sum of th widths; wrapper scrolls for overflow */
  }

  /* ── Header ── */
  thead tr {
    background: var(--color-header-bg);
  }

  th {
    position: sticky;
    top: 0;
    z-index: 2;
    padding: 0 12px;
    height: 38px;
    text-align: left;
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: var(--color-header-bg);
    border-bottom: 2px solid var(--color-border);
    white-space: nowrap;
    overflow: hidden;
    cursor: pointer;
    user-select: none;
    /* position:relative needed for the resize handle */
    position: sticky;
    transition: background-color 0.15s;
  }

  th:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  th.col-check {
    width: 40px;
    min-width: 40px;
    max-width: 40px;
    text-align: center;
    padding: 0;
    cursor: default;
    position: sticky;
    left: 0;
    z-index: 3;
  }

  th.sticky-ip {
    position: sticky;
    left: 40px;
    z-index: 3;
  }

  th.dragging-over {
    background: color-mix(in srgb, var(--color-accent) 20%, var(--color-header-bg));
    border-left: 2px solid var(--color-accent);
  }

  .th-content {
    display: flex;
    align-items: center;
    gap: 4px;
    /* leave 6px on the right for the resize handle */
    padding-right: 6px;
    overflow: hidden;
  }

  .th-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sort-indicator {
    display: flex;
    align-items: center;
    gap: 1px;
    color: var(--color-accent);
    flex-shrink: 0;
  }

  .sort-unsorted {
    opacity: 0.3;
    flex-shrink: 0;
  }

  .sort-priority {
    font-size: 9px;
    line-height: 1;
  }

  /* ── Resize handle ── */
  .resize-handle {
    position: absolute;
    right: 0;
    top: 0;
    /* Wider hit area (12 px) for easier grabbing; visually centred on the border. */
    width: 12px;
    height: 100%;
    cursor: col-resize;
    z-index: 4;
    display: flex;
    align-items: center;
    justify-content: center;
    /* Extend slightly outside the th so the grab area straddles the border. */
    transform: translateX(4px);
  }

  .resize-handle::after {
    content: '';
    display: block;
    width: 2px;
    height: 60%;
    border-radius: 2px;
    background: var(--color-border);
    transition: background-color 0.12s, opacity 0.12s;
    /* Always show a subtle indicator; brighter on hover / during resize. */
    opacity: 0.45;
  }

  th:hover .resize-handle::after {
    opacity: 0.9;
  }

  .resize-handle:hover::after,
  th.is-resizing .resize-handle::after {
    background: var(--color-accent);
    opacity: 1;
  }

  /* Highlight header cell while being resized. */
  th.is-resizing {
    background: color-mix(in srgb, var(--color-accent) 8%, var(--color-header-bg)) !important;
  }

  /* ── Empty state ── */
  .empty-state {
    text-align: center;
    padding: 3rem;
    color: var(--color-text-muted);
    font-size: 14px;
    border-bottom: none;
  }

  /* ── Pagination (P2-LAYOUT-001 — sticky footer, never squeezed) ── */
  .pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 14px;
    border-top: 1px solid var(--color-border);
    background: var(--color-surface);
    font-size: 13px;
    color: var(--color-text-muted);
    flex-shrink: 0;
  }

  .page-size {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .page-size-btn {
    padding: 3px 9px;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    background: none;
    cursor: pointer;
    font-size: 12px;
    color: var(--color-text-muted);
    transition: background-color 0.15s, color 0.15s;
  }

  .page-size-btn.active {
    background: var(--color-accent);
    color: #fff;
    border-color: var(--color-accent);
  }

  .page-size-btn:not(.active):hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .page-nav {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .page-info {
    font-size: 12.5px;
  }

  .page-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: none;
    cursor: pointer;
    font-size: 16px;
    color: var(--color-text);
    transition: background-color 0.15s;
  }

  .page-btn:hover:not(:disabled) {
    background: var(--color-hover);
  }

  .page-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  /* ── Context menu ── */
  .context-menu {
    position: fixed;
    z-index: 1000;
    min-width: 200px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
    padding: 4px 0;
  }

  .ctx-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 14px;
    border: none;
    background: none;
    font-size: 13px;
    color: var(--color-text);
    cursor: pointer;
    text-align: left;
    transition: background-color 0.1s;
  }

  .ctx-item:hover {
    background: var(--color-hover);
  }

  .ctx-item.ctx-muted {
    color: var(--color-text-muted);
    font-size: 12.5px;
  }

  .ctx-sep {
    height: 1px;
    background: var(--color-border);
    margin: 3px 0;
  }

  /* ── Column drawer ── */
  .drawer-overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: rgba(0, 0, 0, 0.25);
  }

  .drawer {
    position: fixed;
    right: 0;
    top: 0;
    bottom: 0;
    z-index: 101;
    width: 280px;
    background: var(--color-surface);
    border-left: 1px solid var(--color-border);
    box-shadow: -4px 0 20px rgba(0, 0, 0, 0.12);
    display: flex;
    flex-direction: column;
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--color-border);
    gap: 8px;
  }

  .drawer-title {
    font-weight: 700;
    font-size: 14px;
    display: block;
  }

  .drawer-count {
    font-size: 11.5px;
    color: var(--color-text-muted);
    display: block;
    margin-top: 1px;
  }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 6px 0;
  }

  .drawer-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 16px;
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .drawer-item:hover {
    background: var(--color-hover);
  }

  .drawer-item.disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .drawer-item input {
    accent-color: var(--color-accent);
    cursor: pointer;
    width: 15px;
    height: 15px;
    flex-shrink: 0;
  }

  .drawer-label {
    font-size: 13.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .drawer-footer {
    padding: 10px 16px;
    border-top: 1px solid var(--color-border);
    display: flex;
    gap: 6px;
  }

  .btn-small {
    flex: 1;
    padding: 6px 8px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: none;
    font-size: 12px;
    cursor: pointer;
    color: var(--color-text);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    transition: background-color 0.15s;
  }

  .btn-small:hover {
    background: var(--color-hover);
  }

  /* ── Checkbox header ── */
  th.col-check input {
    width: 15px;
    height: 15px;
    cursor: pointer;
    accent-color: var(--color-accent);
    display: block;
    margin: auto;
  }
</style>
