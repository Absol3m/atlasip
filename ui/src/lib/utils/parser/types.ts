// ── Input Mode (§6) ──────────────────────────────────────────────────────────
// Reflects what was detected, not how the user entered it.

export type InputMode =
  | 'empty'     // No input
  | 'ipv4'      // IPv4 DETECTED
  | 'ipv6'      // IPv6 DETECTED
  | 'hostname'  // HOSTNAME DETECTED
  | 'mixed'     // MIXED MODE (IPv4+IPv6, IP+hostname, IP+URL, hostname+URL, …)
  | 'invalid';  // INVALID (§5.1 triggered, or no valid entries found)

// ── Entry types ───────────────────────────────────────────────────────────────

export type EntryType = 'ipv4' | 'ipv6' | 'hostname' | 'url';

// ── Invalid reasons ───────────────────────────────────────────────────────────

export type InvalidReason =
  | 'invalid_ip'       // Syntactically invalid IPv4 or IPv6 (backward compat alias)
  | 'invalid_ipv4'     // Octet value out of range, wrong segment count, etc.
  | 'invalid_ipv6'     // Not a valid IPv6 string
  | 'invalid_hostname' // Fails hostname regex
  | 'invalid_url'      // URL constructor throws
  | 'private_ip'       // Valid format but private/reserved range (filtered, not blocked)
  | 'unknown_entry';   // Single-word token that matches no valid type → blocks (§5.1)

// ── Valid entry ───────────────────────────────────────────────────────────────

export interface ValidEntry {
  /** Original value as written by the user. */
  raw: string;
  type: EntryType;
  /** Normalised target to send to the API. */
  target: string;
  port?: number;
  protocol?: string;
}

// ── Invalid entry ─────────────────────────────────────────────────────────────

export interface InvalidEntry {
  value: string;
  reason: InvalidReason;
}

// ── Parse result ──────────────────────────────────────────────────────────────

export interface ParseResult {
  mode: InputMode;

  /** Deduplicated, resolved targets (IPs / hostnames) ready for the API. */
  validTargets: string[];

  /** Entries that failed validation and caused blocking or were flagged. */
  invalidEntries: InvalidEntry[];

  /** Count of valid-format but private/reserved IPs silently dropped (not blocking). */
  filteredCount: number;

  /**
   * §5.1: If any explicit entry (single-word token) is invalid,
   * the whole analysis is blocked: validTargets = [], blocked = true.
   * Does NOT apply to words found inside free-text extraction context.
   */
  blocked: boolean;

  /** i18n key for the placeholder shown in the textarea. */
  placeholder: string;

  /** i18n key for the global error message (set when blocked). */
  globalError: string | null;

  /** Full validated entry details. */
  entries: ValidEntry[];
}
