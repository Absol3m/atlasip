// ── Parser v2 — Public API ────────────────────────────────────────────────────
// This file is the stable public import surface for the rest of the application.
// All logic lives in ./parser/ modules.

export {
  parseInput,
  isValidIPv4,
  isPrivateIPv4,
  isValidIPv6,
  isPrivateIPv6,
  isValidHostname,
  isValidURL,
  parseURL,
  isEmailHeader,
} from './parser/index';

export type {
  ParseResult,
  InputMode,
  ValidEntry,
  InvalidEntry,
  InvalidReason,
  EntryType,
} from './parser/index';
