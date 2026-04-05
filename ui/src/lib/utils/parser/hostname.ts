// ── Hostname — strict validation ──────────────────────────────────────────────
// §3.3: Strict regex, no double dot, no forbidden characters.
// §12: Pure, deterministic.

/**
 * RFC-1123-compliant hostname regex.
 *
 * Rules:
 *  - Each label: starts and ends with alphanumeric, may contain hyphens.
 *  - Label length: 1–63 characters.
 *  - TLD: 2–63 alpha-only characters.
 *  - No consecutive dots (double dot).
 *  - Total length ≤ 253 characters (enforced separately).
 *  - No trailing dot (we do not support FQDN with trailing dot as explicit input).
 */
const HOSTNAME_RE =
  /^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,63}$/;

/**
 * Return true iff `s` is a syntactically valid hostname.
 * Pure function — no trimming.
 */
export function isValidHostname(s: string): boolean {
  if (s.length > 253) return false;
  if (s.includes('..')) return false; // §3.3: no double dot
  return HOSTNAME_RE.test(s);
}

/**
 * Extract all valid hostnames embedded in `text`.
 * §7.2: google.fr, sub.domain.co.uk, etc.
 *
 * Hostnames must have at least one dot; tokens without a dot are not hostnames.
 * IPv4 addresses that match the hostname pattern are excluded.
 */
export function extractHostnames(text: string): string[] {
  // Split on whitespace and common separators, then filter
  const tokens = text.split(/[\s,;|<>()\[\]{}"'`\t\r\n]+/);
  const results: string[] = [];
  const seen = new Set<string>();
  for (const token of tokens) {
    const clean = token.replace(/^[^a-zA-Z0-9]+|[^a-zA-Z0-9]+$/g, '');
    if (clean.length === 0) continue;
    if (seen.has(clean.toLowerCase())) continue;
    if (isValidHostname(clean)) {
      seen.add(clean.toLowerCase());
      results.push(clean);
    }
  }
  return results;
}
