// eslint-disable-next-line import/no-extraneous-dependencies -- Vitest is supplied by the root workspace.
import { describe, expect, it, vi } from 'vitest';

import { parseAccountBindChallenge, returnFragment, signAccountBind } from './account-bind.js';

const nonce = 'ab'.repeat(32);
const text = [
  'elohim:doorway-account-bind:v1',
  'audience=doorway.example',
  'identifier=person@example.org',
  'human=human-1',
  'from=uhCAk-old',
  `nonce=${nonce}`,
  'expires=2026-10-16T20:15:00Z',
].join('\n');
const now = () => Date.parse('2026-10-16T20:14:00Z');

describe('account-bind challenge', () => {
  it('parses the exact fields with an injected clock', () => {
    expect(parseAccountBindChallenge(text, now)).toEqual({
      audience: 'doorway.example',
      identifier: 'person@example.org',
      human: 'human-1',
      from: 'uhCAk-old',
      nonce,
      expires: '2026-10-16T20:15:00Z',
    });
  });

  it.each([
    ['wrong order', text.replace('human=human-1\nfrom=uhCAk-old', 'from=uhCAk-old\nhuman=human-1')],
    ['extra line', text + '\n'],
    ['bad nonce', text.replace(nonce, 'AB'.repeat(32))],
    ['short nonce', text.replace(nonce, 'ab')],
    ['CR', text.replace(/\n/g, '\r\n')],
    ['oversize bytes', text.replace('human-1', 'é'.repeat(512))],
    ['wrong purpose', text.replace(':v1', ':v2')],
    ['empty field', text.replace('human=human-1', 'human=')],
    ['bad calendar date', text.replace('2026-10-16', '2026-02-30')],
    ['fractional seconds', text.replace('20:15:00Z', '20:15:00.000Z')],
    ['offset', text.replace('20:15:00Z', '20:15:00+00:00')],
  ])('refuses %s', (_name, challenge) => {
    expect(parseAccountBindChallenge(challenge, now)).toEqual({ refused: 'NOT_A_BIND_CHALLENGE' });
  });

  it('refuses expired challenges including the exact expiry', () => {
    for (const date of ['2026-10-16T20:15:00Z', '2026-10-16T20:16:00Z']) {
      expect(parseAccountBindChallenge(text, () => Date.parse(date))).toEqual({
        refused: 'CHALLENGE_EXPIRED',
      });
    }
  });
});

describe('account-bind return fragment', () => {
  it('encodes the key and converts standard base64 to unpadded base64url', () => {
    const fragment = returnFragment({
      challengeId: nonce,
      agentPubKey: 'uhCAk+/?',
      signature: '+/8=',
    });
    expect(fragment.startsWith('#challengeId=')).toBe(true);
    expect(Object.fromEntries(new URLSearchParams(fragment.slice(1)))).toEqual({
      challengeId: nonce,
      agentPubKey: 'uhCAk+/?',
      signature: '-_8',
    });
  });

  it('returns only the refusal', () => {
    expect(returnFragment({ refused: 'DECLINED' })).toBe('#refused=DECLINED');
  });
});

describe('account-bind signing', () => {
  it('posts unchanged text through the same-origin transport and invokes its prover', async () => {
    const fetch = vi.fn(
      async () => new Response(JSON.stringify({ agentPubKey: 'key', signature: '+/8=' }))
    );
    const prove = vi.fn(async () => null);
    expect(await signAccountBind({ fetch, prove }, text)).toEqual({
      ok: true,
      body: { agentPubKey: 'key', signature: '+/8=' },
    });
    expect(prove).toHaveBeenCalledOnce();
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/identity/account-bind/sign',
      expect.objectContaining({
        method: 'POST',
        credentials: 'same-origin',
        body: JSON.stringify({ challenge: text }),
      })
    );
  });

  it('preserves the node refusal code', async () => {
    const body = { error: 'Bind your session first', code: 'SESSION_NOT_BOUND' };
    const fetch = vi.fn(async () => new Response(JSON.stringify(body), { status: 403 }));
    expect(await signAccountBind({ fetch }, text)).toEqual({ ok: false, status: 403, body });
  });
});
