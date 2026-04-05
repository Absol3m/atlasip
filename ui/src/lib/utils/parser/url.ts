// ── URL — parsing and hostname extraction ─────────────────────────────────────
// §3.4: Parse via URL constructor, extract hostname, port (optional), protocol.
// §7.3: path is ignored; §8.3/8.11 fix.
// §12: Pure, deterministic.

export interface ParsedURL {
  /** Original raw URL string. */
  raw: string;
  /** Extracted hostname (without brackets for IPv6). */
  hostname: string;
  /** Extracted port, undefined if not present. */
  port?: number;
  /** Extracted protocol (with colon, e.g. "https:"). */
  protocol: string;
}

/**
 * Attempt to parse `raw` as a URL.
 * Returns a ParsedURL on success, null on failure.
 * Pure function — no trimming, no fallback.
 */
export function parseURL(raw: string): ParsedURL | null {
  // The URL constructor requires a protocol; bare "example.com/path" would fail.
  // We only parse tokens that already look like URLs (contain ://).
  if (!raw.includes('://')) return null;
  try {
    const u = new URL(raw);
    // Reject non-network schemes (data:, javascript:, blob:, file:)
    if (!['http:', 'https:', 'ftp:', 'ftps:'].includes(u.protocol)) return null;
    const portNum = u.port ? Number(u.port) : undefined;
    // u.hostname strips brackets from IPv6
    const hostname = u.hostname;
    if (!hostname) return null;
    return {
      raw,
      hostname,
      port: portNum,
      protocol: u.protocol,
    };
  } catch {
    return null;
  }
}

/**
 * Return true iff `s` is a parseable, valid URL with a network scheme.
 */
export function isValidURL(s: string): boolean {
  return parseURL(s) !== null;
}

/**
 * Extract all URLs from `text`, returning their parsed form.
 * §7.3: protocol, hostname, port extracted; path ignored.
 */
export function extractURLs(text: string): ParsedURL[] {
  // Split on whitespace and angle brackets / quotes
  const tokens = text.split(/[\s<>"'`\t\r\n]+/);
  const results: ParsedURL[] = [];
  const seen = new Set<string>();
  for (const token of tokens) {
    if (!token.includes('://')) continue;
    const parsed = parseURL(token);
    if (parsed && !seen.has(parsed.hostname.toLowerCase())) {
      seen.add(parsed.hostname.toLowerCase());
      results.push(parsed);
    }
  }
  return results;
}
