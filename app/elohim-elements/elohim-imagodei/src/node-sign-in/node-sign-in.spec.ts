// Sign-in secrets below are fixtures for a fake node, never real secrets.
/* eslint-disable sonarjs/no-hardcoded-passwords */
import { expect } from '@open-wc/testing';

import { sessionKeysOver } from '../session-key/session.js';
import type { SessionKeyStore, SessionSigner } from '../session-key/signer.js';
import { webCryptoSessionKeys } from '../session-key/webcrypto.js';

import { SignInController } from './controller.js';
import { signInFailureFor } from './logic.js';
import { LOGIN_PATH, LOGOUT_PATH, createNodeSignInClient, type SignInResponse } from './wire.js';

const SIGNED_IN: SignInResponse = {
  humanId: 'h-4c1d',
  agentPubKey: 'uhCAkagent',
  identifier: 'matthew',
  displayName: 'Matthew',
  expiresAt: 1_791_086_400,
  isSteward: true,
  redirect: '/auth/portal/',
  sessionKeyBound: true,
};

function keysIn(available = true) {
  const real = webCryptoSessionKeys();
  const store: SessionKeyStore & { kept: SessionSigner | null } = {
    kept: null,
    available: () => available,
    make: () => real.make(),
    keep: async s => {
      store.kept = s;
    },
    load: async () => store.kept,
    clear: async () => {
      store.kept = null;
    },
  };
  return { store, keys: sessionKeysOver(store) };
}

function fetchAnswering(status: number, body: unknown) {
  const sent: { url: string; init: RequestInit }[] = [];
  const fetchFn = (async (url: string, init: RequestInit) => {
    sent.push({ url, init });
    return new Response(JSON.stringify(body), { status });
  }) as unknown as typeof fetch;
  return { sent, fetchFn };
}

const header = (init: RequestInit, name: string) => (init.headers as Record<string, string>)[name];

describe('signing in to this node', () => {
  afterEach(() => localStorage.clear());

  it('binds the session to a new key: its public half in the body, a proof made with it', async () => {
    const { store, keys } = keysIn();
    const { sent, fetchFn } = fetchAnswering(200, SIGNED_IN);
    const client = createNodeSignInClient({ fetch: fetchFn, keys });
    await client.signIn({ identifier: 'matthew', password: 'a-long-secret', remember: true });

    expect(sent[0]!.url).to.equal(LOGIN_PATH);
    const body = JSON.parse(sent[0]!.init.body as string);
    expect(body).to.deep.include({
      identifier: 'matthew',
      password: 'a-long-secret',
      remember: true,
    });
    expect(body.sessionKey.alg).to.equal('ES256');
    expect(Object.keys(body.sessionKey.jwk)).to.deep.equal(['kty', 'crv', 'x', 'y']);
    const proof = header(sent[0]!.init, 'DPoP')!;
    const named = JSON.parse(atob(proof.split('.')[0]!.replaceAll('-', '+').replaceAll('_', '/')));
    expect(named.jwk).to.deep.equal(body.sessionKey.jwk);
    expect(await store.kept!.publicJwk()).to.deep.equal(body.sessionKey.jwk);
  });

  it('keeps no key when the node did not bind the session to it', async () => {
    const { store, keys } = keysIn();
    const { fetchFn } = fetchAnswering(200, { ...SIGNED_IN, sessionKeyBound: false });
    await createNodeSignInClient({ fetch: fetchFn, keys }).signIn({
      identifier: 'm',
      password: 'p',
    });
    expect(store.kept).to.equal(null);
  });

  it('signs in without a key where the page cannot keep one, saying nothing', async () => {
    const { keys } = keysIn(false);
    const { sent, fetchFn } = fetchAnswering(200, { ...SIGNED_IN, sessionKeyBound: false });
    await createNodeSignInClient({ fetch: fetchFn, keys }).signIn({
      identifier: 'm',
      password: 'p',
    });
    expect(JSON.parse(sent[0]!.init.body as string).sessionKey).to.equal(undefined);
    expect(header(sent[0]!.init, 'DPoP')).to.equal(undefined);
  });

  it('keeps an earlier key when a sign-in is refused', async () => {
    const { store, keys } = keysIn();
    const earlier = await keys.forSignIn();
    await keys.signedIn(earlier, true);
    const { fetchFn } = fetchAnswering(401, {
      error: 'Invalid credentials',
      code: 'INVALID_CREDENTIALS',
    });
    await createNodeSignInClient({ fetch: fetchFn, keys }).signIn({
      identifier: 'm',
      password: 'p',
    });
    expect(store.kept).to.equal(earlier);
  });

  it('signs out with a proof, and forgets the key', async () => {
    const { store, keys } = keysIn();
    await keys.signedIn(await keys.forSignIn(), true);
    const { sent, fetchFn } = fetchAnswering(200, {});
    await createNodeSignInClient({ fetch: fetchFn, keys }).signOut();
    expect(sent[0]!.url).to.equal(LOGOUT_PATH);
    expect(header(sent[0]!.init, 'DPoP')).to.be.a('string');
    expect(store.kept).to.equal(null);
  });
});

describe('what a refused sign-in means', () => {
  const failed = (status: number, body: unknown) => ({ ok: false as const, status, body });

  it('names each of the node’s answers', () => {
    expect(signInFailureFor(failed(401, { code: 'INVALID_CREDENTIALS' }))).to.deep.equal({
      kind: 'invalid',
    });
    expect(signInFailureFor(failed(409, { code: 'signin_secret_unset' }))).to.deep.equal({
      kind: 'secret-unset',
    });
    expect(signInFailureFor(failed(429, { code: 'signin_slowed', retryAfter: 42 }))).to.deep.equal({
      kind: 'slowed',
      retryAfter: 42,
    });
    expect(
      signInFailureFor(failed(401, { code: 'signin_paused', reason: ' Unusual hour. ' }))
    ).to.deep.equal({
      kind: 'paused',
      reason: 'Unusual hour.',
    });
    expect(signInFailureFor(failed(401, { code: 'signin_paused' }))).to.deep.equal({
      kind: 'paused',
    });
    expect(signInFailureFor(failed(400, { code: 'signin_needs_session_key' }))).to.deep.equal({
      kind: 'needs-session-key',
    });
    expect(signInFailureFor(failed(400, { error: 'identifier is required' }))).to.deep.equal({
      kind: 'missing',
    });
    expect(signInFailureFor(failed(0, null))).to.deep.equal({ kind: 'unavailable' });
  });
});

describe('SignInController', () => {
  it('sends one sign-in at a time, and nothing for an empty form', async () => {
    let calls = 0;
    let release!: () => void;
    const controller = new SignInController({
      client: {
        signIn: () => {
          calls++;
          return new Promise(resolve => (release = () => resolve({ ok: true, body: SIGNED_IN })));
        },
        signOut: async () => ({ ok: true, body: {} }),
      },
      onChange: () => undefined,
    });
    expect(await controller.signIn({ identifier: ' ', password: 'x' })).to.equal(false);
    expect(controller.state.refusal).to.deep.equal({ kind: 'missing' });
    const first = controller.signIn({ identifier: 'matthew', password: 'secret-secret' });
    expect(controller.state.phase).to.equal('signing-in');
    expect(await controller.signIn({ identifier: 'matthew', password: 'secret-secret' })).to.equal(
      false
    );
    release();
    expect(await first).to.equal(true);
    expect(calls).to.equal(1);
    expect(controller.state.phase).to.equal('signed-in');
  });
});
