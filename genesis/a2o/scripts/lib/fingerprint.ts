import { createHash } from 'node:crypto';

const UUID_RE = /\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/gi;
const ISO_RE  = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g;
const HEX_HASH_RE = /\b[0-9a-f]{16,}\b/gi;
const URL_PORT_RE = /(https?:\/\/[^\s/:]+):\d+/g;
const WS_RE = /\s+/g;

export function normalizeMessage(raw: string): string {
  return raw
    .replace(UUID_RE, '<uuid>')
    .replace(ISO_RE, '<ts>')
    .replace(URL_PORT_RE, '$1')
    .replace(HEX_HASH_RE, '<hash>')
    .replace(WS_RE, ' ')
    .trim();
}

export function fingerprint(raw: string): string {
  const normalized = normalizeMessage(raw);
  return createHash('sha256').update(normalized).digest('hex').slice(0, 12);
}
