// ── IPv6 — strict validation + private/reserved detection ────────────────────
// §3.2: Format standard.  We use URL constructor for validation (spec §3.2 refs
// std::net::Ipv6Addr::from_str in Rust; in TS the URL API is the equivalent
// strict parser without regex complexity).

/**
 * Return true iff `s` is a syntactically valid IPv6 address.
 * Uses the WHATWG URL parser — the most authoritative JS IPv6 validator.
 * Pure function — no trimming.
 */
export function isValidIPv6(s: string): boolean {
  if (!s.includes(':')) return false;
  try {
    // Wrap in brackets as required by RFC 2732 / URL standard.
    const url = new URL(`http://[${s}]/`);
    // If hostname equals the bracketed address the parse succeeded.
    return url.hostname === `[${s.toLowerCase()}]` ||
           url.hostname === `[${s}]`;
  } catch {
    return false;
  }
}

/**
 * Return true iff `ip` is a private, loopback, or link-local IPv6 address.
 * Precondition: `isValidIPv6(ip)` is true.
 */
export function isPrivateIPv6(ip: string): boolean {
  const low = ip.toLowerCase();
  return (
    low === '::1'                 ||  // Loopback
    low === '::'                  ||  // Unspecified
    low.startsWith('fc')         ||  // ULA fc00::/7
    low.startsWith('fd')         ||  // ULA fd00::/7
    low.startsWith('fe80')       ||  // Link-local
    low.startsWith('::ffff:')    ||  // IPv4-mapped
    low.startsWith('2001:db8')   ||  // Documentation (RFC 3849)
    low.startsWith('100::')      ||  // Discard (RFC 6666)
    low === '::ffff:0:0'             // IPv4-translated
  );
}

/**
 * Extract all syntactically valid IPv6 addresses embedded in `text`.
 * §7.1 — free-text extraction.
 *
 * Strategy: find all colon-containing tokens and validate with isValidIPv6.
 * We look for patterns that contain at least one colon and hexadecimal digits.
 */
export function extractIPv6(text: string): string[] {
  // Match bracket-wrapped [::1] forms and bare forms.
  // The regex captures sequences of hex digits, colons, and optionally a
  // trailing IPv4 section (::ffff:1.2.3.4 style).
  const RE = /(?:\[([0-9a-fA-F:]+(?::[0-9a-fA-F]{1,4}|:\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})*)\])|([0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{0,4}){2,7}(?::\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})?)/g;
  const results: string[] = [];
  const seen = new Set<string>();
  for (const m of text.matchAll(RE)) {
    const candidate = (m[1] ?? m[2]).toLowerCase();
    if (!seen.has(candidate) && isValidIPv6(candidate)) {
      seen.add(candidate);
      results.push(candidate);
    }
  }
  return results;
}
