/**
 * Validates CoordinationEnvelopes emitted by simulation scripts.
 * Lightweight runtime validation — the TypeScript interfaces live in
 * elohim-app, but we validate the JSON shape here without importing them.
 */

const VALID_VERBS = [
  'invoke', 'sense', 'respond', 'aggregate', 'route',
  'delegate', 'escalate', 'ratify', 'recall', 'provision', 'federate',
  'settle',
] as const;

type CoordinationVerb = (typeof VALID_VERBS)[number];

interface Envelope {
  verb: string;
  scope?: { agents?: string[] };
  routing?: { urgency?: string; fallback?: string };
  payload?: Record<string, unknown>;
  sender?: { agentId?: string; delegationChain?: unknown[] };
  timestamp?: string;
}

export function validateEnvelope(obj: unknown): boolean {
  if (!obj || typeof obj !== 'object') return false;
  const e = obj as Envelope;
  if (!e.verb || !VALID_VERBS.includes(e.verb as CoordinationVerb)) return false;
  return true;
}

export function isProvisionEnvelope(obj: unknown): boolean {
  if (!validateEnvelope(obj)) return false;
  const e = obj as Envelope;
  return e.verb === 'provision' && !!e.payload?.serviceRequest;
}

export function isSettleEnvelope(obj: unknown): boolean {
  if (!validateEnvelope(obj)) return false;
  const e = obj as Envelope;
  return e.verb === 'settle' && !!e.payload?.economicEvent;
}

export function isSenseEnvelope(obj: unknown): boolean {
  if (!validateEnvelope(obj)) return false;
  const e = obj as Envelope;
  return e.verb === 'sense' && !!e.payload?.computeMetrics;
}
