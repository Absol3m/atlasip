// ── Extractors — free text and email headers ──────────────────────────────────
// §7: Extraction from free text, headers, logs, URLs.
// §7.5: Normal words, punctuation, and separators are ignored.
// §12: Pure, no side effects.

import { isValidIPv4, isPrivateIPv4, extractIPv4 } from './ipv4';
import { isValidIPv6, isPrivateIPv6, extractIPv6 } from './ipv6';
import { isValidHostname, extractHostnames } from './hostname';
import { extractURLs } from './url';
import type { ValidEntry, InvalidEntry } from './types';

// ── Email header markers (§7.4) ───────────────────────────────────────────────

const HEADER_MARKERS_RE =
  /^(?:Received|X-Originating-IP|X-Forwarded-For|X-Real-IP|Return-Path|Delivered-To):/im;

/**
 * Return true iff `text` contains recognizable email header lines.
 * §7.4 detection.
 */
export function isEmailHeader(text: string): boolean {
  return HEADER_MARKERS_RE.test(text);
}

// ── Extraction result ─────────────────────────────────────────────────────────

export interface ExtractionResult {
  validEntries: ValidEntry[];
  filteredCount: number;
  invalidEntries: InvalidEntry[];
}

/**
 * Extract all IPs, hostnames, and URLs from arbitrary `text`.
 * Normal words and punctuation are silently ignored (§7.5 / §8.5 fix).
 * Private IPs are counted but not added as valid targets.
 */
export function extractFromText(text: string): ExtractionResult {
  const validEntries: ValidEntry[] = [];
  const invalidEntries: InvalidEntry[] = [];
  let filteredCount = 0;
  const seen = new Set<string>();

  function addIP(ip: string, type: 'ipv4' | 'ipv6') {
    const key = ip.toLowerCase();
    if (seen.has(key)) return;
    seen.add(key);
    const priv = type === 'ipv4' ? isPrivateIPv4(ip) : isPrivateIPv6(ip);
    if (priv) { filteredCount++; return; }
    validEntries.push({ raw: ip, type, target: ip });
  }

  // 1. Extract IPv4
  for (const ip of extractIPv4(text)) addIP(ip, 'ipv4');

  // 2. Extract IPv6 (skip tokens already seen as IPv4)
  for (const ip of extractIPv6(text)) addIP(ip, 'ipv6');

  // 3. Extract URLs → get their hostnames
  for (const parsed of extractURLs(text)) {
    const key = parsed.hostname.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    // If the URL hostname is an IP, validate it
    if (isValidIPv4(parsed.hostname)) {
      if (isPrivateIPv4(parsed.hostname)) { filteredCount++; continue; }
      validEntries.push({ raw: parsed.raw, type: 'url', target: parsed.hostname, port: parsed.port, protocol: parsed.protocol });
    } else if (parsed.hostname.includes(':') && isValidIPv6(parsed.hostname)) {
      if (isPrivateIPv6(parsed.hostname)) { filteredCount++; continue; }
      validEntries.push({ raw: parsed.raw, type: 'url', target: parsed.hostname, port: parsed.port, protocol: parsed.protocol });
    } else if (isValidHostname(parsed.hostname)) {
      validEntries.push({ raw: parsed.raw, type: 'url', target: parsed.hostname, port: parsed.port, protocol: parsed.protocol });
    }
    // Else: hostname is not a valid IP or hostname — skip silently (text context)
  }

  // 4. Extract hostnames (skip anything already seen)
  for (const h of extractHostnames(text)) {
    const key = h.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    validEntries.push({ raw: h, type: 'hostname', target: h });
  }

  return { validEntries, filteredCount, invalidEntries };
}
