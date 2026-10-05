// The URLs below are the refusals under test, never fetched.
/* eslint-disable sonarjs/no-clear-text-protocols, sonarjs/code-eval */
import { expect } from '@open-wc/testing';

import {
  KEY_HOLDER_SIGN_STEP,
  MAX_REASON_LENGTH,
  MAX_REQUEST_BYTES,
  NODE_CODE,
  REFUSAL,
  decodeConsentRequest,
  failureFor,
  isShowableView,
  isTerminalReturnUrl,
  keyHolderStep,
  nothingWasSigned,
  outcomeForAgreement,
  reasonOf,
  standingFor,
  trailAfterAgreement,
} from './logic.js';
import { approveCommand } from '../terminal.js';

import { approvalWords } from './words.js';
import type { ConsentAgreeResponse } from './wire.js';

/** base64url, unpadded — how the terminal prints the request. */
function encode(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  bytes.forEach(b => (binary += String.fromCodePoint(b)));
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
}

const REQUEST = {
  domain: 'elohim.device-consent.v1',
  clientId: 'epr-cli',
  label: 'workspace — Ünïcode',
  acts: ['device.enroll'],
  returnPath: { kind: 'display' },
};

const unreadable = { ok: false, code: REFUSAL.requestUnreadable };

describe('decodeConsentRequest — reading the link', () => {
  it('decodes base64url JSON into the request object, unchanged', () => {
    expect(decodeConsentRequest(encode(JSON.stringify(REQUEST)))).to.deep.equal({
      ok: true,
      request: REQUEST,
    });
  });

  it('accepts padded base64url too', () => {
    const padded = btoa('{"a":1}').replace(/\+/g, '-').replace(/\//g, '_');
    expect(padded.endsWith('=')).to.equal(true);
    expect(decodeConsentRequest(padded)).to.deep.equal({ ok: true, request: { a: 1 } });
  });

  for (const [label, param] of [
    ['absent', null],
    ['undefined', undefined],
    ['empty', ''],
    ['standard base64 characters', '+/' + encode('{}')],
    ['spaces', 'eyJ hIjoxfQ'],
    ['a length no base64 can have', encode('{"a":1}') + 'AAA'],
    ['not JSON', encode('just words')],
    ['truncated JSON', encode('{"clientId": "epr-cli"')],
    ['a JSON array', encode('[1,2,3]')],
    ['a JSON string', encode('"workspace"')],
    ['JSON null', encode('null')],
    ['very long', 'A'.repeat(100_000)],
  ] as const) {
    it(`refuses a parameter that is ${label}`, () => {
      expect(decodeConsentRequest(param)).to.deep.equal(unreadable);
    });
  }

  it('refuses bytes that are not UTF-8', () => {
    const binary = String.fromCodePoint(0x7b, 0xff, 0xfe, 0x7d);
    const param = btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
    expect(decodeConsentRequest(param)).to.deep.equal(unreadable);
  });

  it('accepts a request of exactly 4 KiB and refuses one byte more', () => {
    const fill = (size: number) =>
      JSON.stringify({ pad: 'x'.repeat(size - JSON.stringify({ pad: '' }).length) });
    expect(fill(MAX_REQUEST_BYTES)).to.have.length(MAX_REQUEST_BYTES);
    expect(decodeConsentRequest(encode(fill(MAX_REQUEST_BYTES))).ok).to.equal(true);
    expect(decodeConsentRequest(encode(fill(MAX_REQUEST_BYTES + 1)))).to.deep.equal(unreadable);
  });
});

describe('isTerminalReturnUrl — where the browser may hand the code', () => {
  for (const url of [
    'http://127.0.0.1:53682/callback?code=abc&state=xyz',
    'http://localhost:8400/',
    'http://localhost:1/x',
    'http://127.0.0.1:65535/done',
  ]) {
    it(`follows the terminal listener on this machine: ${url}`, () => {
      expect(isTerminalReturnUrl(url)).to.equal(true);
    });
  }

  for (const [label, url] of [
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
    ['a backslash', String.raw`http://localhost:8400/\evil.example`],
    ['whitespace', 'http://localhost:8400/ x'],
  ]) {
    it(`refuses ${label}`, () => {
      expect(isTerminalReturnUrl(url)).to.equal(false);
    });
  }

  it('refuses anything that is not a string', () => {
    expect(isTerminalReturnUrl(undefined)).to.equal(false);
    expect(isTerminalReturnUrl({ url: 'http://localhost:1/' })).to.equal(false);
  });
});

describe('failureFor — what a failed call means for the page', () => {
  const failed = (status: number, body: unknown = null) => ({ ok: false as const, status, body });

  it('shows the node’s own refusal code from a 4xx body', () => {
    expect(
      failureFor(failed(400, { error: 'cannot be shown', code: 'request_acts_incoherent' }))
    ).to.deep.equal({ kind: 'refused', code: 'request_acts_incoherent' });
  });

  it('treats a 4xx without a code as an unreadable request', () => {
    expect(failureFor(failed(400, { error: 'Invalid JSON' }))).to.deep.equal({
      kind: 'refused',
      code: REFUSAL.requestUnreadable,
    });
  });

  for (const [label, status] of [
    ['404 (the host does not mount the endpoint)', 404],
    ['501', 501],
    ['no answer at all', 0],
    ['a server error', 502],
  ] as const) {
    it(`says approvals are unavailable on ${label}`, () => {
      expect(failureFor(failed(status, { code: 'should_not_show' }))).to.deep.equal({
        kind: 'refused',
        code: REFUSAL.consentUnavailable,
      });
    });
  }

  it('sends the person to sign in on a 401, or when the node says no one is signed in', () => {
    expect(failureFor(failed(401))).to.deep.equal({ kind: 'sign-in' });
    expect(failureFor(failed(403, { error: 'x', code: NODE_CODE.notSignedIn }))).to.deep.equal({
      kind: 'sign-in',
    });
  });

  it('passes the two nothing-signed codes through as themselves', () => {
    for (const code of [NODE_CODE.identityUnbootstrapped, NODE_CODE.signingUnavailable]) {
      expect(failureFor(failed(409, { error: 'x', code }))).to.deep.equal({
        kind: 'refused',
        code,
      });
      expect(nothingWasSigned(code)).to.equal(true);
    }
    expect(nothingWasSigned('act_unknown')).to.equal(false);
  });

  it('reads a 401 asking to sign in again as its own step, not a sign-in redirect', () => {
    expect(
      failureFor(failed(401, { error: 'x', code: NODE_CODE.reauthenticationAsked, reason: 'r' }))
    ).to.deep.equal({ kind: 'refused', code: NODE_CODE.reauthenticationAsked });
    expect(nothingWasSigned(NODE_CODE.reauthenticationAsked)).to.equal(true);
  });

  it('shows the node’s reason as given, trimmed, at most 280 characters', () => {
    expect(reasonOf({ reason: '  plain words  ' })).to.equal('plain words');
    expect(reasonOf({ reason: 'x'.repeat(400) })).to.have.length(MAX_REASON_LENGTH);
    expect(reasonOf({ reason: '   ' })).to.equal(undefined);
    expect(reasonOf({ reason: 7 })).to.equal(undefined);
    expect(reasonOf(null)).to.equal(undefined);
  });

  it('keeps a waiting signer a wait even when it comes as a server error', () => {
    expect(
      failureFor(failed(503, { error: 'x', code: NODE_CODE.signingUnavailable }))
    ).to.deep.equal({
      kind: 'refused',
      code: NODE_CODE.signingUnavailable,
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
    expect(isShowableView(view)).to.equal(true);
    expect(isShowableView({ ...view, askedActs: [] })).to.equal(false);
    expect(isShowableView({ ...view, label: undefined })).to.equal(false);
    expect(isShowableView(null)).to.equal(false);
  });
});

const agreement = (partial: Partial<ConsentAgreeResponse>): ConsentAgreeResponse => ({
  returnTarget: { kind: 'display', value: 'K7QF-2MXD' },
  expiresAt: 1_791_000_300_000,
  consentCid: 'bafyreiconsent',
  controllers: { required: 1, signed: 1 },
  witnesses: [],
  ...partial,
});

describe('outcomeForAgreement — where an approval leaves the page', () => {
  it('shows a code to paste for a display target', () => {
    expect(outcomeForAgreement(agreement({}))).to.deep.equal({
      phase: 'code',
      code: 'K7QF-2MXD',
      expiresAt: 1_791_000_300_000,
    });
  });

  it('hands the code back to a terminal on this machine', () => {
    const url = 'http://127.0.0.1:53682/callback?code=c0de&state=s';
    expect(
      outcomeForAgreement(agreement({ returnTarget: { kind: 'redirect', url } }))
    ).to.deep.equal({ phase: 'handed-back', url });
  });

  it('refuses to follow a redirect anywhere else, and never yields the URL', () => {
    const outcome = outcomeForAgreement(
      agreement({ returnTarget: { kind: 'redirect', url: 'https://evil.example/steal' } })
    );
    expect(outcome).to.deep.equal({ phase: 'refused', code: REFUSAL.returnPathRefused });
  });

  it('treats a malformed answer as unavailable rather than guessing', () => {
    expect(outcomeForAgreement(null)).to.deep.equal({
      phase: 'refused',
      code: REFUSAL.consentUnavailable,
    });
    expect(
      outcomeForAgreement(agreement({ returnTarget: { kind: 'display', value: '' } }))
    ).to.deep.equal({ phase: 'refused', code: REFUSAL.consentUnavailable });
  });
});

describe('the witness trail — who secured the approval', () => {
  const doorway = { relation: 'your-doorway', label: 'alpha.elohim.host' } as const;
  const own = { relation: 'this-device' } as const;

  it('names the key holder as the host does, with no label when it has none', () => {
    expect(keyHolderStep(doorway, 'working')).to.deep.equal({
      id: KEY_HOLDER_SIGN_STEP,
      act: 'signed',
      relation: 'your-doorway',
      label: 'alpha.elohim.host',
      state: 'working',
    });
    expect(keyHolderStep(own, 'working')).to.deep.equal({
      id: KEY_HOLDER_SIGN_STEP,
      act: 'signed',
      relation: 'this-device',
      state: 'working',
    });
  });

  it('shows the reported witnesses instead of its own step, unchanged and in order', () => {
    const witnesses = [
      { id: 'b', act: 'recorded', relation: 'others', count: 2, state: 'done', extra: 'kept' },
      { id: 'a', act: 'signed', relation: 'this-device', state: 'done' },
    ];
    const trail = trailAfterAgreement(own, witnesses);
    expect(trail).to.deep.equal(witnesses);
    expect(trail[0]).to.equal(witnesses[0]);
  });

  it('falls back to the key holder alone when nothing was reported', () => {
    for (const witnesses of [undefined, [], { not: 'a list' }, [null, 'x', { id: 1 }]]) {
      expect(trailAfterAgreement(own, witnesses)).to.deep.equal([keyHolderStep(own, 'done')]);
    }
  });
});

describe('standingFor — what the approval rests on', () => {
  it('reads one of one as a complete approval on one node', () => {
    expect(standingFor({ required: 1, signed: 1 })).to.deep.equal({ kind: 'single' });
  });

  it('reads a quorum still short of its count, and how many more', () => {
    expect(standingFor({ required: 3, signed: 1 })).to.deep.equal({
      kind: 'short',
      more: 2,
      required: 3,
      signed: 1,
    });
  });

  it('reads a met quorum', () => {
    expect(standingFor({ required: 2, signed: 2 })).to.deep.equal({
      kind: 'enough',
      required: 2,
      signed: 2,
    });
  });

  it('says nothing when the node gave no usable count', () => {
    for (const c of [
      undefined,
      null,
      {},
      { required: 0, signed: 0 },
      { required: '1', signed: 1 },
    ]) {
      expect(standingFor(c)).to.equal(null);
    }
  });
});

describe('approvalWords — the host names the key holder', () => {
  const device = approvalWords({ name: 'This device', inSentence: 'this device' });
  const doorway = approvalWords({ name: 'Your doorway', inSentence: 'your doorway' });

  it('says one device is enough as a fact, not as a shortfall', () => {
    const line = device.standing({ kind: 'single' })!;
    expect(line).to.include(
      'This device signed as you, and that is enough: this approval is complete.'
    );
    expect(line).to.include('can affirm it later');
    expect(line).not.to.match(/not enough|must|waiting|pending/i);
  });

  it('asks for more only for a quorum the person set up', () => {
    const line = device.standing({ kind: 'short', more: 1, required: 2, signed: 1 })!;
    expect(line).to.include('You asked that 2 of the devices that speak for you agree');
    expect(line).to.include('1 more of them must agree');
  });

  it('says nothing when there is no count', () => {
    expect(device.standing(null)).to.equal(undefined);
  });

  it('says the doorway holds the key and signs as the person', () => {
    const line = (doorway.card.signerHosted as (host?: string) => string)('alpha.elohim.host');
    expect(line).to.equal(
      'Your doorway (alpha.elohim.host) holds your key, and will sign this as you when you approve.'
    );
  });

  it('never names a doorway on the native host', () => {
    const all = JSON.stringify(device.card.refusal) + device.standing({ kind: 'single' });
    const signer = (device.card.signerOwnNode as (host?: string) => string)();
    expect(all + signer).not.to.match(/doorway/i);
    expect(signer).to.equal(
      'This device holds your key, and will sign this as you when you approve.'
    );
  });

  it('says a page on another machine is not a retry, and lets a host word it its own way', () => {
    expect(device.card.refusal?.['consent_caller_not_local']).to.include(
      'open on a different machine from the one that holds your key'
    );
    expect(device.card.refusalHeading?.['consent_caller_not_local']).to.equal(
      'Approve on the machine that holds your key'
    );
    expect(nothingWasSigned('consent_caller_not_local')).to.equal(true);
    const own = approvalWords({
      name: 'Your doorway',
      inSentence: 'your doorway',
      callerNotLocal: 'Ours.',
    });
    expect(own.card.refusal?.['consent_caller_not_local']).to.equal('Ours.');
  });

  it('builds the terminal command with the link as one word', () => {
    expect(approveCommand('http://localhost:8090/auth/portal/consent/device?request=e30')).to.equal(
      "epr device approve 'http://localhost:8090/auth/portal/consent/device?request=e30'"
    );
  });

  it('turns off the trail’s own guess at what it rests on', () => {
    expect(device.trail).to.include({ doorwayAlone: '', thisDeviceAlone: '' });
  });

  it('says an unnamed witness attends the person, never that someone who knows them does', () => {
    const signed = device.trail.sentences?.['vouches-for-you']?.signed;
    expect(signed?.({})).to.equal('A witness attending you signed it too');
    expect(device.trail.party?.['vouches-for-you']?.({})).to.equal('a witness attending you');
    expect(signed?.({ name: 'Elohim' })).to.equal('Elohim signed it as a witness');
  });

  it('asks to sign in again in words a host may replace', () => {
    expect(device.card.refusal?.['consent_reauthentication_asked']).to.include(
      'Nothing was signed, and no code was issued.'
    );
    expect(device.card.refusalHeading?.['consent_reauthentication_asked']).to.equal(
      'Sign in again to go ahead'
    );
    const own = approvalWords({ name: 'X', inSentence: 'x', reauthentication: 'Ours.' });
    expect(own.card.refusal?.['consent_reauthentication_asked']).to.equal('Ours.');
  });
});
