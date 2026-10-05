import { HttpErrorResponse } from '@angular/common/http';
import { describe, expect, it } from 'vitest';

import {
  MAX_REQUEST_BYTES,
  REFUSAL,
  decodeConsentRequest,
  failureFor,
  isShowableView,
  isTerminalReturnUrl,
  outcomeForAgreement,
  signingTrail,
} from './device-consent.logic';

/** base64url, unpadded — how the terminal prints the request. */
function encode(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  bytes.forEach(b => (binary += String.fromCharCode(b)));
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

const REQUEST = {
  domain: 'elohim.device-consent.v1',
  clientId: 'epr-cli',
  label: 'workspace — Ünïcode',
  acts: ['device.enroll'],
  returnPath: { kind: 'display' },
};

describe('decodeConsentRequest — reading the link', () => {
  it('decodes base64url JSON into the request object, unchanged', () => {
    const decoded = decodeConsentRequest(encode(JSON.stringify(REQUEST)));
    expect(decoded).toEqual({ ok: true, request: REQUEST });
  });

  it('accepts padded base64url too', () => {
    const padded = btoa('{"a":1}').replace(/\+/g, '-').replace(/\//g, '_');
    expect(padded.endsWith('=')).toBe(true);
    expect(decodeConsentRequest(padded)).toEqual({ ok: true, request: { a: 1 } });
  });

  const unreadable = { ok: false, code: REFUSAL.requestUnreadable };

  it.each([
    ['absent', null],
    ['undefined', undefined],
    ['empty', ''],
  ])('refuses an %s parameter', (_label, param) => {
    expect(decodeConsentRequest(param)).toEqual(unreadable);
  });

  it.each([
    ['standard base64 characters', '+/' + encode('{}')],
    ['spaces', 'eyJ hIjoxfQ'],
    ['a length no base64 can have', encode('{"a":1}') + 'A'.repeat(3)],
  ])('refuses text that is not base64url (%s)', (_label, param) => {
    expect(decodeConsentRequest(param)).toEqual(unreadable);
  });

  it.each([
    ['not JSON', 'just words'],
    ['truncated JSON', '{"clientId": "epr-cli"'],
    ['a JSON array', '[1,2,3]'],
    ['a JSON string', '"workspace"'],
    ['JSON null', 'null'],
  ])('refuses a payload that is %s', (_label, text) => {
    expect(decodeConsentRequest(encode(text))).toEqual(unreadable);
  });

  it('refuses bytes that are not UTF-8', () => {
    const binary = String.fromCharCode(0x7b, 0xff, 0xfe, 0x7d);
    const param = btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
    expect(decodeConsentRequest(param)).toEqual(unreadable);
  });

  it('accepts a request of exactly 4 KiB and refuses one byte more', () => {
    const fill = (size: number) =>
      JSON.stringify({ pad: 'x'.repeat(size - JSON.stringify({ pad: '' }).length) });
    expect(fill(MAX_REQUEST_BYTES)).toHaveLength(MAX_REQUEST_BYTES);
    expect(decodeConsentRequest(encode(fill(MAX_REQUEST_BYTES))).ok).toBe(true);
    expect(decodeConsentRequest(encode(fill(MAX_REQUEST_BYTES + 1)))).toEqual(unreadable);
  });

  it('refuses a very long parameter without decoding it', () => {
    expect(decodeConsentRequest('A'.repeat(100_000))).toEqual(unreadable);
  });
});

describe('isTerminalReturnUrl — where the browser may hand the code', () => {
  it.each([
    'http://127.0.0.1:53682/callback?code=abc&state=xyz',
    'http://localhost:8400/',
    'http://localhost:1/x',
    'http://127.0.0.1:65535/done',
  ])('follows the terminal listener on this machine: %s', url => {
    expect(isTerminalReturnUrl(url)).toBe(true);
  });

  it.each([
    ['https to loopback', 'https://127.0.0.1:53682/callback'],
    ['no port', 'http://127.0.0.1/callback'],
    ['no path', 'http://localhost:8400'],
    ['port zero', 'http://localhost:0/'],
    ['port out of range', 'http://localhost:70000/'],
    ['another loopback spelling', 'http://127.1:8400/'],
    ['IPv6 loopback', 'http://[::1]:8400/'],
    ['localhost subdomain', 'http://localhost.evil.example:8400/'],
    ['credentials', 'http://user@localhost:8400/'],
    ['upper-case host', 'http://LOCALHOST:8400/'],
    ['a remote host', 'http://evil.example:8400/'],
    ['a javascript URL', 'javascript:alert(1)'],
    ['a backslash', 'http://localhost:8400/\\evil.example'],
    ['whitespace', 'http://localhost:8400/ x'],
  ])('refuses %s', (_label, url) => {
    expect(isTerminalReturnUrl(url)).toBe(false);
  });

  it('refuses anything that is not a string', () => {
    expect(isTerminalReturnUrl(undefined)).toBe(false);
    expect(isTerminalReturnUrl({ url: 'http://localhost:1/' })).toBe(false);
  });
});

describe('failureFor — what a failed call means for the page', () => {
  const http = (status: number, error: unknown = null) =>
    new HttpErrorResponse({ status, error, url: '/auth/consent/view' });

  it('shows the runtime’s own refusal code from a 4xx body', () => {
    expect(
      failureFor(http(400, { error: 'cannot be shown', code: 'request_acts_incoherent' }))
    ).toEqual({ kind: 'refused', code: 'request_acts_incoherent' });
  });

  it('treats a 4xx without a code as an unreadable request', () => {
    expect(failureFor(http(400, { error: 'Invalid JSON' }))).toEqual({
      kind: 'refused',
      code: REFUSAL.requestUnreadable,
    });
  });

  it.each([
    ['404 (doorway does not mount the endpoint)', 404],
    ['501', 501],
    ['a network failure', 0],
    ['a server error', 502],
  ])('says this doorway cannot approve devices on %s', (_label, status) => {
    expect(failureFor(http(status, { code: 'should_not_show' }))).toEqual({
      kind: 'refused',
      code: REFUSAL.consentUnavailable,
    });
  });

  it('asks the person to sign in again when the session ran out', () => {
    expect(failureFor(http(401))).toEqual({ kind: 'sign-in' });
  });

  it('treats anything that is not an HTTP error as unavailable', () => {
    expect(failureFor(new Error('boom'))).toEqual({
      kind: 'refused',
      code: REFUSAL.consentUnavailable,
    });
  });
});

describe('isShowableView', () => {
  it('needs a label, a fingerprint and at least one asked act', () => {
    const view = {
      clientId: 'epr-cli',
      label: 'workspace',
      deviceFingerprint: 'uhCAk…',
      askedActs: ['device.enroll'],
    };
    expect(isShowableView(view)).toBe(true);
    expect(isShowableView({ ...view, askedActs: [] })).toBe(false);
    expect(isShowableView({ ...view, label: undefined })).toBe(false);
    expect(isShowableView(null)).toBe(false);
  });
});

describe('outcomeForAgreement — where an approval leaves the page', () => {
  it('shows a code to paste for a display target', () => {
    expect(
      outcomeForAgreement({
        returnTarget: { kind: 'display', value: 'K7QF-2MXD' },
        expiresAt: 1_791_000_300_000,
      })
    ).toEqual({ phase: 'code', code: 'K7QF-2MXD', expiresAt: 1_791_000_300_000 });
  });

  it('hands the code back to a terminal on this machine', () => {
    const url = 'http://127.0.0.1:53682/callback?code=c0de&state=s';
    expect(outcomeForAgreement({ returnTarget: { kind: 'redirect', url }, expiresAt: 1 })).toEqual({
      phase: 'handed-back',
      url,
    });
  });

  it('refuses to follow a redirect anywhere else, and never yields the URL', () => {
    const outcome = outcomeForAgreement({
      returnTarget: { kind: 'redirect', url: 'https://evil.example/steal' },
      expiresAt: 1,
    });
    expect(outcome).toEqual({ phase: 'refused', code: REFUSAL.returnPathRefused });
  });

  it('treats a malformed answer as unavailable rather than guessing', () => {
    expect(outcomeForAgreement(null)).toEqual({
      phase: 'refused',
      code: REFUSAL.consentUnavailable,
    });
    expect(
      outcomeForAgreement({
        returnTarget: { kind: 'display', value: '' },
        expiresAt: 1,
      })
    ).toEqual({ phase: 'refused', code: REFUSAL.consentUnavailable });
  });
});

describe('signingTrail — who secured the approval', () => {
  const doorway = (state: string) => ({
    id: 'doorway-sign',
    act: 'signed',
    relation: 'your-doorway',
    label: 'alpha.elohim.host',
    state,
  });

  it.each(['working', 'done', 'failed'] as const)('is the doorway step alone while %s', state => {
    expect(signingTrail('alpha.elohim.host', state)).toEqual([doorway(state)]);
  });

  it('passes reported witnesses through after the doorway step, unchanged and in order', () => {
    const witnesses = [
      { id: 'b', act: 'recorded', relation: 'others', count: 2, state: 'done', extra: 'kept' },
      { id: 'a', act: 'signed', relation: 'your-device', label: 'workspace', state: 'done' },
    ];
    const trail = signingTrail('alpha.elohim.host', 'done', witnesses);
    expect(trail).toEqual([doorway('done'), ...witnesses]);
    expect(trail[1]).toBe(witnesses[0]);
  });

  it('ignores a witnesses field that is not a list', () => {
    expect(signingTrail('alpha.elohim.host', 'done', { not: 'a list' })).toEqual([doorway('done')]);
  });
});
