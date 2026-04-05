import { parseInput } from '$lib/utils/parser';
import type { ParseResult } from '$lib/utils/parser';

// ── Types ─────────────────────────────────────────────────────────────────────

export type AnalysisStatus = 'idle' | 'loading' | 'success' | 'error';
export type ToastType = 'info' | 'success' | 'warning' | 'error';

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
}

// ── Store ─────────────────────────────────────────────────────────────────────

class AnalysisStore {
  /** Raw text from the input textarea — preserved across navigation. */
  rawInput = $state('');

  /** Structured parse result from the last parse run. */
  parseResult = $state<ParseResult | null>(null);

  /** Workflow status of the analysis request. */
  status = $state<AnalysisStatus>('idle');

  /** Active toast notifications. */
  toasts = $state<Toast[]>([]);

  private _parseTimer: ReturnType<typeof setTimeout> | null = null;

  // ── Input & parsing ────────────────────────────────────────────────────────

  /** Update raw input and schedule a debounced parse (80 ms). */
  scheduleparse(raw: string) {
    this.rawInput = raw;
    if (this._parseTimer !== null) clearTimeout(this._parseTimer);
    this._parseTimer = setTimeout(() => {
      this.parseResult = parseInput(raw);
      this._parseTimer = null;
    }, 80);
  }

  /** Update raw input and parse synchronously (use on paste events). */
  parseImmediate(raw: string) {
    this.rawInput = raw;
    if (this._parseTimer !== null) {
      clearTimeout(this._parseTimer);
      this._parseTimer = null;
    }
    this.parseResult = parseInput(raw);
  }

  /** Replace the raw input (e.g. when the table edits an IP row). */
  setInput(raw: string) {
    this.rawInput = raw;
    this.parseResult = parseInput(raw);
  }

  /** Clear the input and reset analysis state. */
  clear() {
    this.rawInput = '';
    this.parseResult = null;
    this.status = 'idle';
    if (this._parseTimer !== null) {
      clearTimeout(this._parseTimer);
      this._parseTimer = null;
    }
  }

  // ── Status ─────────────────────────────────────────────────────────────────

  setStatus(s: AnalysisStatus) {
    this.status = s;
  }

  // ── Toasts ─────────────────────────────────────────────────────────────────

  /** Show a toast. Returns the generated id. Duration 0 = persistent. */
  addToast(type: ToastType, message: string, duration = 4500): string {
    const id = crypto.randomUUID();
    this.toasts = [...this.toasts, { id, type, message }];
    if (duration > 0) {
      setTimeout(() => this.dismissToast(id), duration);
    }
    return id;
  }

  dismissToast(id: string) {
    this.toasts = this.toasts.filter(t => t.id !== id);
  }

  // ── Convenience getters ────────────────────────────────────────────────────

  get hasValidTargets(): boolean {
    return (this.parseResult?.validTargets.length ?? 0) > 0;
  }

  get validTargetCount(): number {
    return this.parseResult?.validTargets.length ?? 0;
  }

  /** true if any entry failed validation (P0-UX-001 — blocks the Analyze button). */
  get hasInvalidEntries(): boolean {
    return (this.parseResult?.invalidEntries.length ?? 0) > 0;
  }

  /** true when §5.1 blocking is active (explicit invalid token). */
  get isBlocked(): boolean {
    return this.parseResult?.blocked ?? false;
  }

  /** Count of IP entries (ipv4 + ipv6) in the valid target list (P1-UX-004). */
  get ipCount(): number {
    return this.parseResult?.entries.filter(
      e => e.type === 'ipv4' || e.type === 'ipv6',
    ).length ?? 0;
  }

  /** Count of hostname + URL entries in the valid target list (P1-UX-004). */
  get hostnameCount(): number {
    return this.parseResult?.entries.filter(
      e => e.type === 'hostname' || e.type === 'url',
    ).length ?? 0;
  }
}

export const analysisStore = new AnalysisStore();
