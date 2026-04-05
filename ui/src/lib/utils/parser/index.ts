// ── Parser v2 — Main entry point ──────────────────────────────────────────────
// §2: Strict, coherent, predictable, testable, OSINT-friendly.
// §5.1: One invalid explicit entry → everything is blocked.
// §6: Mode detection table.
// §12: Pure, no side effects, no silent trimming, no implicit fallback.

import { isValidIPv4, isPrivateIPv4 } from './ipv4';
import { isValidIPv6, isPrivateIPv6 } from './ipv6';
import { isValidHostname } from './hostname';
import { isValidURL, parseURL } from './url';
import { isEmailHeader, extractFromText } from './extractors';
import type { ParseResult, ValidEntry, InvalidEntry, InputMode, EntryType } from './types';

// ── Re-exports for consumers ──────────────────────────────────────────────────
export type { ParseResult, ValidEntry, InvalidEntry, InputMode, EntryType };
export type { InvalidReason } from './types';
export { isValidIPv4, isPrivateIPv4 } from './ipv4';
export { isValidIPv6, isPrivateIPv6 } from './ipv6';
export { isValidHostname } from './hostname';
export { isValidURL, parseURL } from './url';
export { isEmailHeader } from './extractors';

// ── Placeholder keys ──────────────────────────────────────────────────────────

const PLACEHOLDER: Record<InputMode, string> = {
  empty:    'analysis.placeholder.default',
  ipv4:     'analysis.placeholder.default',
  ipv6:     'analysis.placeholder.default',
  hostname: 'analysis.placeholder.hostname',
  mixed:    'analysis.placeholder.default',
  invalid:  'analysis.placeholder.default',
};

// ── Mode derivation from a set of entry types (§6) ────────────────────────────

function deriveMode(types: Set<EntryType>): InputMode {
  if (types.size === 0) return 'invalid';

  const hasIPv4     = types.has('ipv4');
  const hasIPv6     = types.has('ipv6');
  const hasHostname = types.has('hostname');
  const hasURL      = types.has('url');

  // More than one distinct kind → MIXED MODE (§6)
  const kindCount = [hasIPv4, hasIPv6, hasHostname, hasURL].filter(Boolean).length;
  if (kindCount > 1) return 'mixed';

  if (hasIPv4)     return 'ipv4';
  if (hasIPv6)     return 'ipv6';
  if (hasHostname || hasURL) return 'hostname';
  return 'invalid';
}

// ── Token classification ──────────────────────────────────────────────────────

interface TokenResult {
  entry: ValidEntry | null;
  invalid: InvalidEntry | null;
  filtered: boolean;
}

/**
 * Classify a single explicit token (one line from the user's input).
 * Order of checks: IPv4 → IPv6 → URL → hostname → unknown.
 * Returns entry=null + invalid != null when the token is unrecognised.
 */
function classifyToken(raw: string): TokenResult {
  // ── IPv4 ──────────────────────────────────────────────────────────────────
  // If it looks like a dotted quad, treat it as an IPv4 attempt.
  if (/^\d{1,3}(?:\.\d{1,3}){3}$/.test(raw)) {
    if (!isValidIPv4(raw)) {
      return { entry: null, invalid: { value: raw, reason: 'invalid_ipv4' }, filtered: false };
    }
    if (isPrivateIPv4(raw)) {
      return { entry: null, invalid: null, filtered: true };
    }
    return { entry: { raw, type: 'ipv4', target: raw }, invalid: null, filtered: false };
  }

  // ── IPv6 ──────────────────────────────────────────────────────────────────
  if (raw.includes(':') && !raw.includes('://')) {
    // Strip optional brackets [::1]
    const stripped = raw.startsWith('[') && raw.endsWith(']') ? raw.slice(1, -1) : raw;
    if (!isValidIPv6(stripped)) {
      return { entry: null, invalid: { value: raw, reason: 'invalid_ipv6' }, filtered: false };
    }
    if (isPrivateIPv6(stripped)) {
      return { entry: null, invalid: null, filtered: true };
    }
    return { entry: { raw, type: 'ipv6', target: stripped }, invalid: null, filtered: false };
  }

  // ── URL ───────────────────────────────────────────────────────────────────
  if (raw.includes('://')) {
    const parsed = parseURL(raw);
    if (!parsed) {
      return { entry: null, invalid: { value: raw, reason: 'invalid_url' }, filtered: false };
    }
    // Resolve hostname from URL
    const { hostname, port, protocol } = parsed;
    // If hostname is an IP, validate it
    if (isValidIPv4(hostname)) {
      if (isPrivateIPv4(hostname)) return { entry: null, invalid: null, filtered: true };
      return { entry: { raw, type: 'url', target: hostname, port, protocol }, invalid: null, filtered: false };
    }
    if (isValidIPv6(hostname)) {
      if (isPrivateIPv6(hostname)) return { entry: null, invalid: null, filtered: true };
      return { entry: { raw, type: 'url', target: hostname, port, protocol }, invalid: null, filtered: false };
    }
    if (!isValidHostname(hostname)) {
      return { entry: null, invalid: { value: raw, reason: 'invalid_url' }, filtered: false };
    }
    return { entry: { raw, type: 'url', target: hostname, port, protocol }, invalid: null, filtered: false };
  }

  // ── Hostname ──────────────────────────────────────────────────────────────
  if (isValidHostname(raw)) {
    return { entry: { raw, type: 'hostname', target: raw }, invalid: null, filtered: false };
  }

  // ── Unknown ── §5.1: one unknown token → block everything ─────────────────
  return { entry: null, invalid: { value: raw, reason: 'unknown_entry' }, filtered: false };
}

// ── Main parse function ───────────────────────────────────────────────────────

/**
 * Parse raw user input and return a structured ParseResult.
 *
 * Supports:
 *  - Single or multiple IPv4/IPv6 addresses (§3.1, §3.2)
 *  - Hostnames (§3.3)
 *  - URLs (§3.4)
 *  - Free text with embedded IPs/hostnames/URLs (§3.5)
 *  - Email headers (§7.4)
 *
 * §5.1: If any single-word explicit entry is unrecognised, the whole result is
 *        blocked (validTargets = [], blocked = true).
 *
 * §12: No trimming of individual tokens, no heuristic fallback.
 */
export function parseInput(raw: string): ParseResult {
  // ── Empty input ────────────────────────────────────────────────────────────
  // We do allow trimming the overall input to detect emptiness (UX requirement),
  // but individual tokens are never silently trimmed (§12).
  if (raw.trim() === '') {
    return {
      mode: 'empty',
      validTargets: [],
      invalidEntries: [],
      filteredCount: 0,
      blocked: false,
      placeholder: PLACEHOLDER.empty,
      globalError: null,
      entries: [],
    };
  }

  // ── Free text / header mode ────────────────────────────────────────────────
  // Detect email headers first (§7.4), then fall back to general text extraction.
  // In extraction mode §5.1 does NOT apply (words are context, not entries).
  const looksLikeHeader = isEmailHeader(raw);

  // Heuristic for "free text": the input is multi-word on at least one line
  // (contains a space within a non-empty line), AND is not pure IP/hostname/URL list.
  // We detect this after attempting explicit line-by-line parsing.

  // ── Split input into explicit lines ───────────────────────────────────────
  // Lines separated by newline. Each non-empty line is a candidate.
  const lines = raw.split('\n');
  const tokens: string[] = [];
  const multiWordLines: string[] = [];

  for (const line of lines) {
    const trimmedLine = line.trim();
    if (trimmedLine === '') continue;
    // A line is "multi-word" if it contains a space after trimming
    if (/\s/.test(trimmedLine)) {
      multiWordLines.push(trimmedLine);
    } else {
      tokens.push(trimmedLine);
    }
  }

  // If we have email headers OR multi-word lines → text extraction mode
  if (looksLikeHeader || (multiWordLines.length > 0 && tokens.length === 0)) {
    // Pure text extraction — §5.1 does not apply (§7.5: normal words ignored)
    const { validEntries, filteredCount, invalidEntries: extractInvalid } = extractFromText(raw);

    const typeSet = new Set<EntryType>(validEntries.map(e => e.type));
    const mode = validEntries.length === 0 ? 'invalid' : deriveMode(typeSet);
    const validTargets = dedupe(validEntries.map(e => e.target));

    return {
      mode,
      validTargets,
      invalidEntries: extractInvalid,
      filteredCount,
      blocked: false,
      placeholder: PLACEHOLDER[looksLikeHeader ? 'ipv4' : mode],
      globalError: null,
      entries: validEntries,
    };
  }

  // ── Mixed: some explicit tokens + some multi-word lines ──────────────────
  // Extract from multi-word lines as text (words are context, not entries).
  // Apply §5.1 only to explicit tokens.
  const validEntries: ValidEntry[] = [];
  const invalidEntries: InvalidEntry[] = [];
  let filteredCount = 0;
  let blocked = false;
  const seen = new Set<string>();

  // Process explicit single-word tokens
  for (const token of tokens) {
    const { entry, invalid, filtered } = classifyToken(token);
    if (filtered) { filteredCount++; continue; }
    if (invalid) {
      invalidEntries.push(invalid);
      blocked = true; // §5.1
      continue;
    }
    if (entry) {
      const key = entry.target.toLowerCase();
      if (!seen.has(key)) {
        seen.add(key);
        validEntries.push(entry);
      }
    }
  }

  // Extract from multi-word lines (text context — no blocking)
  if (multiWordLines.length > 0) {
    const textContent = multiWordLines.join('\n');
    const { validEntries: textEntries, filteredCount: textFiltered } = extractFromText(textContent);
    filteredCount += textFiltered;
    for (const e of textEntries) {
      const key = e.target.toLowerCase();
      if (!seen.has(key)) {
        seen.add(key);
        validEntries.push(e);
      }
    }
  }

  // §5.1: if blocked, clear valid targets
  const effectiveEntries = blocked ? [] : validEntries;
  const validTargets = dedupe(effectiveEntries.map(e => e.target));

  const typeSet = new Set<EntryType>(effectiveEntries.map(e => e.type));
  let mode: InputMode;
  if (blocked || (validTargets.length === 0 && invalidEntries.length > 0)) {
    mode = 'invalid';
  } else if (validTargets.length === 0 && filteredCount > 0) {
    mode = 'invalid'; // everything was filtered
  } else {
    mode = deriveMode(typeSet);
  }

  return {
    mode,
    validTargets,
    invalidEntries,
    filteredCount,
    blocked,
    placeholder: PLACEHOLDER[mode],
    globalError: blocked ? 'error.no_valid_targets' : null,
    entries: effectiveEntries,
  };
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function dedupe(arr: string[]): string[] {
  const seen = new Set<string>();
  return arr.filter(s => {
    const k = s.toLowerCase();
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });
}
