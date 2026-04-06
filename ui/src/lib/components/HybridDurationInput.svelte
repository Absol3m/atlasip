<script lang="ts">
  interface Props {
    label: string;
    description: string;
    value: number; // ms
    min?: number;  // ms
    max?: number;  // ms
  }

  let {
    label,
    description,
    value = $bindable(),
    min = 0,
    max = 600_000,
  }: Props = $props();

  // ── Logarithmic slider ────────────────────────────────────────────────────
  const LOG_MIN = Math.log10(Math.max(10, min || 10));
  const LOG_MAX = Math.log10(max);

  function msToSlider(ms: number): number {
    const clamped = Math.max(Math.max(10, min), Math.min(max, ms));
    return ((Math.log10(clamped) - LOG_MIN) / (LOG_MAX - LOG_MIN)) * 100;
  }

  function sliderToMs(pos: number): number {
    const log = LOG_MIN + (pos / 100) * (LOG_MAX - LOG_MIN);
    return Math.round(10 ** log);
  }

  let sliderPos = $derived(msToSlider(value));

  function onSliderInput(e: Event) {
    const pos = parseFloat((e.target as HTMLInputElement).value);
    value = sliderToMs(pos);
    textVal = formatDisplay(value);
    error = '';
  }

  // ── Text input parsing ────────────────────────────────────────────────────
  type Unit = 'ms' | 's' | 'min';

  function parseDuration(raw: string): { ms: number; unit: Unit } | null {
    const t = raw.trim().toLowerCase();
    if (!t) return null;
    const m = t.match(/^([0-9]+(?:\.[0-9]*)?)(?:\s*(ms|s|min|m))?$/);
    if (!m) return null;
    const num  = parseFloat(m[1]);
    const unit = (m[2] === 'm' ? 'min' : m[2] || 'ms') as Unit;
    const ms = unit === 'ms' ? num : unit === 's' ? num * 1_000 : num * 60_000;
    return { ms: Math.round(ms), unit };
  }

  function formatDisplay(ms: number): string {
    if (ms >= 60_000 && ms % 60_000 === 0) return `${ms / 60_000}min`;
    if (ms >= 1_000  && ms % 1_000  === 0) return `${ms / 1_000}s`;
    return `${ms}ms`;
  }

  function naturalPhrase(ms: number): string {
    if (ms < 1_000)  return `The server will wait up to ${ms} ms before giving up.`;
    if (ms < 60_000) return `The server will wait up to ${(ms / 1_000).toFixed(ms % 1_000 === 0 ? 0 : 1)} seconds before giving up.`;
    return `The server will wait up to ${(ms / 60_000).toFixed(ms % 60_000 === 0 ? 0 : 1)} minutes before giving up.`;
  }

  let textVal = $state(formatDisplay(value));
  let detectedUnit = $state<Unit>('ms');
  let error = $state('');

  function onTextInput(e: Event) {
    const raw = (e.target as HTMLInputElement).value;
    textVal = raw;
    const parsed = parseDuration(raw);
    if (!parsed) {
      error = 'Enter a value like 500, 5s, or 1.5min';
      return;
    }
    if (parsed.ms < min) {
      error = `Minimum is ${formatDisplay(min)}`;
      return;
    }
    if (parsed.ms > max) {
      error = `Maximum is ${formatDisplay(max)}`;
      return;
    }
    error = '';
    detectedUnit = parsed.unit;
    value = parsed.ms;
  }

  function onTextBlur() {
    if (!error) textVal = formatDisplay(value);
  }

  // ── Color feedback ────────────────────────────────────────────────────────
  let colorClass = $derived(
    value <= 300  ? 'good'
    : value <= 2_000 ? 'warn'
    : 'bad'
  );
</script>

<div class="hdi">
  <div class="hdi-header">
    <span class="hdi-label">{label}</span>
    <span class="hdi-badge badge-{colorClass}">{formatDisplay(value)}</span>
  </div>

  <div class="hdi-controls">
    <div class="hdi-input-wrap">
      <input
        class="hdi-input"
        class:invalid={!!error}
        type="text"
        value={textVal}
        oninput={onTextInput}
        onblur={onTextBlur}
        aria-label={label}
      />
      {#if !error}
        <span class="hdi-unit">{detectedUnit}</span>
      {/if}
    </div>
    <input
      class="hdi-slider slider-{colorClass}"
      type="range"
      min="0"
      max="100"
      step="0.1"
      value={sliderPos}
      oninput={onSliderInput}
      aria-label="{label} slider"
    />
  </div>

  {#if error}
    <p class="hdi-error">{error}</p>
  {:else}
    <p class="hdi-desc">{naturalPhrase(value)}</p>
  {/if}

  <p class="hdi-subdesc">{description}</p>
</div>

<style>
  .hdi {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .hdi-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .hdi-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--color-text);
  }

  .hdi-badge {
    font-size: 11.5px;
    font-weight: 600;
    font-family: var(--font-mono);
    padding: 2px 8px;
    border-radius: var(--radius-full);
  }

  .badge-good  { background: color-mix(in srgb, var(--color-success) 15%, transparent); color: var(--color-success); }
  .badge-warn  { background: color-mix(in srgb, var(--color-warning) 15%, transparent); color: var(--color-warning-text); }
  .badge-bad   { background: color-mix(in srgb, var(--color-error)   15%, transparent); color: var(--color-error); }

  .hdi-controls {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .hdi-input-wrap {
    position: relative;
  }

  .hdi-input {
    width: 100%;
    padding: 7px 42px 7px 10px;
    font-size: 13px;
    font-family: var(--font-mono);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    color: var(--color-text);
    outline: none;
    transition: border-color var(--transition-fast);
  }

  .hdi-input:focus { border-color: var(--color-accent); }
  .hdi-input.invalid { border-color: var(--color-error); }

  .hdi-unit {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11px;
    color: var(--color-text-muted);
    pointer-events: none;
  }

  .hdi-slider {
    width: 100%;
    height: 4px;
    appearance: none;
    -webkit-appearance: none;
    border-radius: 999px;
    outline: none;
    cursor: pointer;
    background: var(--color-border);
  }

  .hdi-slider.slider-good::-webkit-slider-thumb  { background: var(--color-success); }
  .hdi-slider.slider-warn::-webkit-slider-thumb  { background: var(--color-warning); }
  .hdi-slider.slider-bad::-webkit-slider-thumb   { background: var(--color-error); }

  .hdi-slider::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--color-surface);
    box-shadow: 0 0 0 1px var(--color-border);
    transition: transform 0.1s;
  }

  .hdi-slider:active::-webkit-slider-thumb { transform: scale(1.2); }

  .hdi-error {
    font-size: 11.5px;
    color: var(--color-error);
  }

  .hdi-desc {
    font-size: 12px;
    color: var(--color-text-muted);
    font-style: italic;
  }

  .hdi-subdesc {
    font-size: 11.5px;
    color: var(--color-text-muted);
    opacity: 0.7;
  }
</style>
