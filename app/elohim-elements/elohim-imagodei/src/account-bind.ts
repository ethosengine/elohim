import { sameOriginJson, type SameOriginOptions } from './same-origin.js';

export interface AccountBindChallenge {
  audience: string;
  identifier: string;
  human: string;
  from: string;
  nonce: string;
  expires: string;
}

export interface AccountBindRefusal {
  refused: 'NOT_A_BIND_CHALLENGE' | 'CHALLENGE_EXPIRED';
}

export interface SignedAccountBind {
  agentPubKey: string;
  /** Standard base64 from the node. */
  signature: string;
}

/** Validate without changing any of the bytes the node will sign. */
export function parseAccountBindChallenge(
  text: string,
  now: () => number = Date.now
): AccountBindChallenge | AccountBindRefusal {
  const invalid = { refused: 'NOT_A_BIND_CHALLENGE' } as const;
  if (new TextEncoder().encode(text).length > 1024 || text.includes('\r')) return invalid;
  const lines = text.split('\n');
  const fields = ['audience', 'identifier', 'human', 'from', 'nonce', 'expires'] as const;
  if (lines.length !== 7 || lines[0] !== 'elohim:doorway-account-bind:v1') return invalid;
  const values: AccountBindChallenge = {
    audience: '',
    identifier: '',
    human: '',
    from: '',
    nonce: '',
    expires: '',
  };
  for (const [index, field] of fields.entries()) {
    const line = lines[index + 1] ?? '';
    if (!line.startsWith(`${field}=`) || line.length === field.length + 1) return invalid;
    values[field] = line.slice(field.length + 1);
  }
  const { audience, identifier, human, from, nonce, expires } = values;
  if (!/^[a-f0-9]{64}$/.test(nonce)) return invalid;
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(expires)) return invalid;
  const expiry = Date.parse(expires);
  if (
    !Number.isFinite(expiry) ||
    new Date(expiry).toISOString() !== expires.replace('Z', '.000Z')
  ) {
    return invalid;
  }
  if (expiry <= now()) return { refused: 'CHALLENGE_EXPIRED' };
  return { audience, identifier, human, from, nonce, expires };
}

export async function signAccountBind(options: SameOriginOptions, challenge: string) {
  return sameOriginJson<SignedAccountBind>(options, 'POST', '/api/v1/identity/account-bind/sign', {
    challenge,
  });
}

interface AccountBindReturn extends SignedAccountBind {
  challengeId: string;
  refused?: never;
}

/** Only a fragment carries the signature back; it must never enter a query string. */
export function returnFragment(result: AccountBindReturn | { refused: string }): string {
  const params = new URLSearchParams();
  if (result.refused === undefined) {
    params.set('challengeId', result.challengeId);
    params.set('agentPubKey', result.agentPubKey);
    params.set(
      'signature',
      result.signature.replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '')
    );
  } else {
    params.set('refused', result.refused);
  }
  return `#${params.toString()}`;
}
