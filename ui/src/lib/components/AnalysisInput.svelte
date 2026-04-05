<script lang="ts">
  import { Search, X, TriangleAlert } from 'lucide-svelte';
  import { analysisStore } from '$lib/stores/analysis.svelte';
  import { i18n } from '$lib/services/i18n.svelte';
  import type { LocaleKey } from '$lib/locales/en';

  interface Props {
    onanalyze: () => void;
    /** Forwarded from parent so the page can call focus() after analysis. */
    textareaRef?: HTMLTextAreaElement | undefined;
  }

  let { onanalyze, textareaRef = $bindable() }: Props = $props();

  // ── Textarea auto-height (P1-INPUT-001 — max 2 lines) ────────────────────
  // 2 lines × (13 px × 1.75 line-height) + 14 px × 2 padding ≈ 73 px.
  // Using 80 px as a comfortable upper bound.
  const MAX_TEXTAREA_HEIGHT = 80;

  let textareaEl = $state<HTMLTextAreaElement | undefined>(undefined);

  // Expose the textarea element to the parent (P2-UX-007 — focus restore).
  $effect(() => {
    textareaRef = textareaEl;
  });

  function autoResize() {
    if (!textareaEl) return;
    textareaEl.style.height = 'auto';
    textareaEl.style.height = Math.min(textareaEl.scrollHeight, MAX_TEXTAREA_HEIGHT) + 'px';
  }

  // ── Event handlers ────────────────────────────────────────────────────────
  function handleInput(e: Event) {
    const value = (e.target as HTMLTextAreaElement).value;
    analysisStore.scheduleparse(value);
    autoResize();
  }

  function handlePaste(_e: ClipboardEvent) {
    setTimeout(() => {
      if (!textareaEl) return;
      analysisStore.parseImmediate(textareaEl.value);
      autoResize();
    }, 0);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (canAnalyze) onanalyze();
    }
    if (e.key === 'Escape') {
      analysisStore.clear();
    }
  }

  function handleClear() {
    analysisStore.clear();
    textareaEl?.focus();
    // Reset textarea height
    if (textareaEl) textareaEl.style.height = '';
  }

  // ── Derived state ─────────────────────────────────────────────────────────
  let parseResult = $derived(analysisStore.parseResult);
  let status      = $derived(analysisStore.status);

  // P0-UX-001: button disabled whenever there are invalid entries or no valid targets.
  let canAnalyze = $derived(
    analysisStore.hasValidTargets &&
    !analysisStore.hasInvalidEntries &&
    status !== 'loading',
  );

  // P0-UX-003 / P1-UX-005: proper badge label for every mode.
  const MODE_BADGE_KEY: Record<string, LocaleKey> = {
    ipv4:     'parse.mode.ipv4',
    ipv6:     'parse.mode.ipv6',
    hostname: 'parse.mode.hostname',
    mixed:    'parse.mode.mixed',
    invalid:  'parse.mode.invalid',
  };

  let badgeKey = $derived.by<LocaleKey | null>(() => {
    const mode = parseResult?.mode;
    if (!mode || mode === 'empty') return null;
    return MODE_BADGE_KEY[mode] ?? null;
  });

  // Badge variant drives colour: success (ipv4/ipv6/hostname), warning (mixed), error (invalid).
  let badgeVariant = $derived.by<'success' | 'warning' | 'error' | null>(() => {
    const mode = parseResult?.mode;
    if (!mode || mode === 'empty') return null;
    if (mode === 'invalid') return 'error';
    if (mode === 'mixed')   return 'warning';
    return 'success';
  });

  // P1-UX-004: type-aware counter ("2 IPs, 1 hostname" instead of "3 valid IPs").
  let summaryParts = $derived.by<string[]>(() => {
    if (!parseResult) return [];
    const parts: string[] = [];

    const ips       = analysisStore.ipCount;
    const hostnames = analysisStore.hostnameCount;
    const inv       = parseResult.invalidEntries.length;
    const filtered  = parseResult.filteredCount;

    if (ips > 0)       parts.push(i18n.tn('parse.count.ip.one',       'parse.count.ip.many',       ips));
    if (hostnames > 0) parts.push(i18n.tn('parse.count.hostname.one', 'parse.count.hostname.many', hostnames));
    if (filtered > 0)  parts.push(i18n.tn('parse.filtered.one',       'parse.filtered.many',       filtered));
    if (inv > 0)       parts.push(i18n.tn('parse.invalid.one',         'parse.invalid.many',        inv));

    return parts;
  });

  // P0-UX-002: global error message when analysis is blocked.
  let globalErrorKey = $derived.by<LocaleKey | null>(() => {
    if (!parseResult) return null;
    // Blocked by §5.1: explicit invalid token.
    if (parseResult.blocked) return 'error.fix_invalid_entries';
    // Has invalid entries but not fully blocked (e.g. text-extraction mode).
    if (parseResult.invalidEntries.length > 0 && parseResult.validTargets.length === 0)
      return 'error.fix_invalid_entries';
    // Has invalid entries even with some valid targets: button is disabled, show message.
    if (parseResult.invalidEntries.length > 0) return 'error.fix_invalid_entries';
    return null;
  });

  // P2-UX-006: dynamic placeholder key from parse result.
  let placeholderKey = $derived.by<LocaleKey>(() => {
    const key = parseResult?.placeholder;
    if (key) return key as LocaleKey;
    return 'analysis.placeholder.default';
  });

  let analyzeLabel = $derived(
    status === 'loading'
      ? i18n.t('analysis.btn.analyzing')
      : analysisStore.validTargetCount > 0
        ? `${i18n.t('analysis.btn.analyze')} (${analysisStore.validTargetCount})`
        : i18n.t('analysis.btn.analyze'),
  );
</script>

<div class="input-root">
  <!-- Textarea wrapper -->
  <div class="textarea-wrap" class:has-error={parseResult && parseResult.invalidEntries.length > 0}>
    <textarea
      bind:this={textareaEl}
      class="ip-textarea"
      value={analysisStore.rawInput}
      placeholder={i18n.t(placeholderKey)}
      oninput={handleInput}
      onpaste={handlePaste}
      onkeydown={handleKeydown}
      disabled={status === 'loading'}
      rows={2}
      spellcheck={false}
      autocomplete="off"
      aria-label="IP addresses input"
      aria-invalid={analysisStore.hasInvalidEntries || undefined}
    ></textarea>

    <!-- Clear button -->
    {#if analysisStore.rawInput}
      <button
        class="clear-btn"
        type="button"
        onclick={handleClear}
        aria-label={i18n.t('analysis.btn.clear')}
        title={i18n.t('analysis.btn.clear')}
      >
        <X size={14} />
      </button>
    {/if}
  </div>

  <!-- Footer: badge + summary + analyze button -->
  <div class="input-footer">
    <div class="feedback">
      <!-- P0-UX-003 / P1-UX-005: correct badge per detected mode -->
      {#if badgeKey}
        <span class="mode-badge mode-badge--{badgeVariant}">
          {i18n.t(badgeKey)}
        </span>
      {/if}

      {#if summaryParts.length > 0}
        <span class="summary">{summaryParts.join(' · ')}</span>
      {:else if !analysisStore.rawInput}
        <span class="hint">{i18n.t('analysis.hint')}</span>
      {/if}
    </div>

    <!-- P3-UX-009: more visible Analyze button -->
    <button
      class="btn-analyze"
      class:btn-loading={status === 'loading'}
      class:btn-success={status === 'success'}
      class:btn-error={status === 'error'}
      type="button"
      disabled={!canAnalyze}
      onclick={onanalyze}
      title={!canAnalyze && analysisStore.hasInvalidEntries ? i18n.t('error.fix_invalid_entries') : undefined}
    >
      {#if status === 'loading'}
        <span class="spinner" aria-hidden="true"></span>
      {:else}
        <Search size={14} aria-hidden="true" />
      {/if}
      {analyzeLabel}
    </button>
  </div>

  <!-- P0-UX-002: global error banner (blocked / has invalid entries) -->
  {#if globalErrorKey}
    <div class="global-error" role="alert" aria-live="polite">
      <TriangleAlert size={13} aria-hidden="true" />
      <span>{i18n.t(globalErrorKey)}</span>
    </div>
  {/if}

  <!-- Invalid entries list (contextual, per entry) -->
  {#if parseResult && parseResult.invalidEntries.length > 0}
    <ul class="invalid-list" aria-label="Invalid entries">
      {#each parseResult.invalidEntries as entry}
        <li class="invalid-entry">
          <TriangleAlert size={12} aria-hidden="true" />
          <code class="invalid-value">{entry.value}</code>
          <span class="invalid-reason">
            — {i18n.t(entry.reason === 'private_ip' ? 'error.private_ip' : 'error.invalid_ip')}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .input-root {
    display: flex;
    flex-direction: column;
    gap: 0;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--shadow);
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .input-root:focus-within {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent) 15%, transparent), var(--shadow);
  }

  /* ── Textarea (P1-INPUT-001 — max 2 lines) ── */
  .textarea-wrap {
    position: relative;
  }

  .textarea-wrap.has-error {
    border-bottom: 1px solid color-mix(in srgb, var(--color-error) 40%, transparent);
  }

  .ip-textarea {
    display: block;
    width: 100%;
    /* Min = 1 line + padding, max = 2 lines + padding (P1-INPUT-001) */
    min-height: 56px;
    max-height: 80px;
    padding: 14px 40px 14px 16px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--color-text);
    font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
    font-size: 13px;
    line-height: 1.75;
    resize: none;
    overflow-y: auto;
  }

  .ip-textarea::placeholder {
    color: var(--color-text-muted);
    opacity: 0.55;
    font-family: inherit;
  }

  .ip-textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* ── Clear button ── */
  .clear-btn {
    position: absolute;
    top: 10px;
    right: 10px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-muted);
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s, background-color 0.15s;
    line-height: 0;
  }

  .clear-btn:hover {
    color: var(--color-error);
    background: color-mix(in srgb, var(--color-error) 10%, transparent);
  }

  /* ── Footer ── */
  .input-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px 6px 12px;
    border-top: 1px solid var(--color-border);
    background: var(--color-bg);
    gap: 8px;
    min-height: 42px;
  }

  .feedback {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    overflow: hidden;
    min-width: 0;
  }

  /* ── Mode badge (P0-UX-003 — per-mode colour variants) ── */
  .mode-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    font-size: 10.5px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    border-radius: 4px;
    flex-shrink: 0;
    white-space: nowrap;
    line-height: 1.6;
  }

  /* success: ipv4 / ipv6 / hostname */
  .mode-badge--success {
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
    color: var(--color-success);
    border: 1px solid color-mix(in srgb, var(--color-success) 25%, transparent);
  }

  /* warning: mixed mode */
  .mode-badge--warning {
    background: color-mix(in srgb, #f59e0b 12%, transparent);
    color: #d97706;
    border: 1px solid color-mix(in srgb, #f59e0b 30%, transparent);
  }

  :global([data-theme='dark']) .mode-badge--warning {
    color: #fbbf24;
    background: color-mix(in srgb, #f59e0b 15%, transparent);
    border-color: color-mix(in srgb, #f59e0b 25%, transparent);
  }

  /* error: invalid */
  .mode-badge--error {
    background: color-mix(in srgb, var(--color-error) 12%, transparent);
    color: var(--color-error);
    border: 1px solid color-mix(in srgb, var(--color-error) 25%, transparent);
  }

  .summary {
    font-size: 12.5px;
    color: var(--color-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint {
    font-size: 12px;
    color: var(--color-text-muted);
    opacity: 0.7;
  }

  /* ── Analyze button (P3-UX-009 — more visible) ── */
  .btn-analyze {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 20px;
    background: var(--color-accent);
    color: #fff;
    border: none;
    border-radius: 7px;
    font-size: 13.5px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
    transition: background-color 0.2s, opacity 0.2s, box-shadow 0.2s, transform 0.1s;
    box-shadow: 0 2px 6px color-mix(in srgb, var(--color-accent) 30%, transparent);
    line-height: 1;
  }

  .btn-analyze:hover:not(:disabled) {
    background: var(--color-accent-hover);
    box-shadow: 0 4px 12px color-mix(in srgb, var(--color-accent) 40%, transparent);
    transform: translateY(-1px);
  }

  .btn-analyze:active:not(:disabled) {
    transform: translateY(0);
    box-shadow: 0 1px 4px color-mix(in srgb, var(--color-accent) 25%, transparent);
  }

  .btn-analyze:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    box-shadow: none;
    transform: none;
  }

  .btn-analyze.btn-loading {
    background: var(--color-accent);
    opacity: 0.85;
  }

  .btn-analyze.btn-success {
    background: var(--color-success);
    box-shadow: 0 2px 6px color-mix(in srgb, var(--color-success) 30%, transparent);
  }

  .btn-analyze.btn-error {
    background: var(--color-error);
    box-shadow: 0 2px 6px color-mix(in srgb, var(--color-error) 30%, transparent);
  }

  /* ── Spinner ── */
  .spinner {
    width: 13px;
    height: 13px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.65s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ── P0-UX-002: Global error banner ── */
  .global-error {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 14px;
    background: color-mix(in srgb, var(--color-error) 8%, var(--color-surface));
    border-top: 1px solid color-mix(in srgb, var(--color-error) 25%, transparent);
    font-size: 12.5px;
    font-weight: 500;
    color: var(--color-error);
    line-height: 0;
  }

  .global-error span {
    line-height: 1.5;
  }

  /* ── Invalid entries list ── */
  .invalid-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0;
    border-top: 1px solid color-mix(in srgb, var(--color-error) 20%, transparent);
    background: color-mix(in srgb, var(--color-error) 4%, var(--color-surface));
    padding: 4px 14px 6px;
  }

  .invalid-entry {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 0;
    font-size: 12px;
    color: var(--color-error);
  }

  .invalid-value {
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
    background: color-mix(in srgb, var(--color-error) 12%, transparent);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .invalid-reason {
    color: var(--color-text-muted);
    font-size: 11.5px;
  }
</style>
