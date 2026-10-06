import { expect } from '@open-wc/testing';

import { sameOriginJson } from '../same-origin.js';

import { SESSION_PROOF_CODE, isSessionProofRefusal } from './index.js';
import { base64url, makeProof, sha256Base64url } from './proof.js';
import { sessionKeysOver } from './session.js';
import type { PublicJwk, SessionKeyStore, SessionSigner } from './signer.js';
import { webCryptoSessionKeys } from './webcrypto.js';

const DECODER = new TextDecoder();
const ENCODER = new TextEncoder();

const fromB64url = (text: string): Uint8Array => {
  const b64 = text.replaceAll('-', '+').replaceAll('_', '/');
  const binary = atob(b64 + '='.repeat((4 - (b64.length % 4)) % 4));
  return Uint8Array.from(binary, ch => ch.codePointAt(0) ?? 0);
};
const decodeJson = (segment: string): Record<string, unknown> =>
  JSON.parse(DECODER.decode(fromB64url(segment))) as Record<string, unknown>;

/** Verify a compact JWS with WebCrypto's own verify, from the key its header names. */
async function verifies(jws: string): Promise<boolean> {
  const [h, c, s] = jws.split('.');
  const jwk = decodeJson(h!)['jwk'] as JsonWebKey;
  const key = await crypto.subtle.importKey(
    'jwk',
    jwk,
    { name: 'ECDSA', namedCurve: 'P-256' },
    false,
    ['verify']
  );
  return crypto.subtle.verify(
    { name: 'ECDSA', hash: 'SHA-256' },
    key,
    fromB64url(s!) as BufferSource,
    ENCODER.encode(`${h}.${c}`) as BufferSource
  );
}

describe('session key — WebCrypto, kept in IndexedDB', () => {
  const store = webCryptoSessionKeys();

  afterEach(async () => store.clear());

  it('is available in a secure context', () => {
    expect(store.available()).to.equal(true);
  });

  it('makes a key whose private half cannot be exported', async () => {
    const signer = (await store.make()) as SessionSigner & { pair: CryptoKeyPair };
    expect(signer.pair.privateKey.extractable).to.equal(false);
    let exported = true;
    await crypto.subtle.exportKey('jwk', signer.pair.privateKey).catch(() => (exported = false));
    expect(exported).to.equal(false);
  });

  it('shows only the members that name the public key', async () => {
    const jwk = await (await store.make()).publicJwk();
    expect(Object.keys(jwk)).to.deep.equal(['kty', 'crv', 'x', 'y']);
    expect(jwk).to.include({ kty: 'EC', crv: 'P-256' });
    expect(fromB64url(jwk.x)).to.have.length(32);
  });

  it('keeps the key between loads, and forgets it', async () => {
    const signer = await store.make();
    await store.keep(signer);
    const again = await store.load();
    expect(await again!.publicJwk()).to.deep.equal(await signer.publicJwk());
    await store.clear();
    expect(await store.load()).to.equal(null);
  });
});

describe('the proof — a DPoP JWS over the exact body bytes', () => {
  it('signs a header and claims WebCrypto’s own verify accepts', async () => {
    const signer = await webCryptoSessionKeys().make();
    const text = JSON.stringify({
      request: { clientId: 'epr-cli' },
      agreedActs: ['device.enroll'],
    });
    const jws = await makeProof(signer, {
      htm: 'POST',
      htu: 'https://node.local/auth/consent/agree',
      body: ENCODER.encode(text),
      jti: 'proof-1',
      iat: 1_791_000_000,
    });
    const [h, c, s] = jws.split('.');
    expect(decodeJson(h!)).to.deep.equal({
      typ: 'dpop+jwt',
      alg: 'ES256',
      jwk: await signer.publicJwk(),
    });
    expect(decodeJson(c!)).to.deep.equal({
      jti: 'proof-1',
      htm: 'POST',
      htu: 'https://node.local/auth/consent/agree',
      iat: 1_791_000_000,
      bsh: await sha256Base64url(ENCODER.encode(text)),
    });
    expect(fromB64url(s!)).to.have.length(64);
    expect(jws).not.to.include('=');
    expect(await verifies(jws)).to.equal(true);
  });

  it('does not verify once a byte of the signed part changes', async () => {
    const signer = await webCryptoSessionKeys().make();
    const jws = await makeProof(signer, {
      htm: 'POST',
      htu: 'https://n/x',
      body: new Uint8Array(),
    });
    const [h, c, s] = jws.split('.');
    const claims = decodeJson(c!);
    const forged = base64url(ENCODER.encode(JSON.stringify({ ...claims, htm: 'GET' })));
    expect(await verifies(`${h}.${forged}.${s}`)).to.equal(false);
  });

  it('hashes the empty body as the hash of no bytes', async () => {
    expect(await sha256Base64url(new Uint8Array())).to.equal(
      '47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU'
    );
  });
});

describe('the session’s key across sign-in and sign-out', () => {
  function memoryStore(): SessionKeyStore & { kept: SessionSigner | null } {
    const real = webCryptoSessionKeys();
    const s = {
      kept: null as SessionSigner | null,
      available: () => true,
      make: () => real.make(),
      keep: async (signer: SessionSigner) => {
        s.kept = signer;
      },
      load: async () => s.kept,
      clear: async () => {
        s.kept = null;
      },
    };
    return s;
  }

  afterEach(() => localStorage.clear());

  it('keeps a key only when the node bound the session to it', async () => {
    const store = memoryStore();
    const keys = sessionKeysOver(store);
    const signer = await keys.forSignIn();
    await keys.signedIn(signer, false);
    expect(store.kept).to.equal(null);
    await keys.signedIn(signer, true);
    expect(store.kept).to.equal(signer);
    expect(await keys.current()).to.equal(signer);
  });

  it('reads a bound session whose key is gone as lost', async () => {
    const store = memoryStore();
    const keys = sessionKeysOver(store);
    await keys.signedIn(await keys.forSignIn(), true);
    expect(await keys.lost()).to.equal(false);
    // Site data cleared under the page: the key is gone, the marker is not.
    const fresh = sessionKeysOver({ ...store, load: async () => null });
    expect(await fresh.lost()).to.equal(true);
    await keys.forget();
    expect(await sessionKeysOver({ ...store, load: async () => null }).lost()).to.equal(false);
  });
});

describe('a POST that makes the node sign carries the proof', () => {
  interface Sent {
    url: string;
    init: RequestInit;
  }

  function fetchAnswering(...answers: { status: number; body: unknown }[]) {
    const sent: Sent[] = [];
    const fetchFn = (async (url: string, init: RequestInit) => {
      sent.push({ url, init });
      const a = answers[Math.min(sent.length - 1, answers.length - 1)]!;
      return new Response(JSON.stringify(a.body), { status: a.status });
    }) as unknown as typeof fetch;
    return { sent, fetchFn };
  }

  const dpop = (s: Sent) => (s.init.headers as Record<string, string>)['DPoP'];

  it('hashes and sends the same serialised body, and names this request', async () => {
    const signer = await webCryptoSessionKeys().make();
    const { sent, fetchFn } = fetchAnswering({ status: 200, body: { ok: 1 } });
    const body = { request: { label: 'workspace — Ünïcode' }, agreedActs: ['device.enroll'] };
    await sameOriginJson(
      { fetch: fetchFn, prove: async () => signer },
      'POST',
      '/auth/consent/agree',
      body
    );

    const proof = dpop(sent[0]!)!;
    const claims = decodeJson(proof.split('.')[1]!);
    expect(sent[0]!.init.body).to.equal(JSON.stringify(body));
    expect(claims['bsh']).to.equal(
      await sha256Base64url(ENCODER.encode(sent[0]!.init.body as string))
    );
    expect(claims['htm']).to.equal('POST');
    expect(claims['htu']).to.equal(new URL('/auth/consent/agree', location.href).href);
    expect(await verifies(proof)).to.equal(true);
  });

  it('sends a fresh proof once when the node calls it stale, and then stops', async () => {
    const signer = await webCryptoSessionKeys().make();
    const stale = { status: 401, body: { error: 'x', code: SESSION_PROOF_CODE.stale } };
    const { sent, fetchFn } = fetchAnswering(stale, stale, stale);
    const result = await sameOriginJson(
      { fetch: fetchFn, prove: async () => signer },
      'POST',
      '/p',
      {}
    );
    expect(sent).to.have.length(2);
    const ids = sent.map(s => decodeJson(dpop(s)!.split('.')[1]!)['jti']);
    expect(ids[0]).not.to.equal(ids[1]);
    expect(result).to.deep.include({ ok: false, status: 401 });
  });

  it('never retries any other refusal', async () => {
    const signer = await webCryptoSessionKeys().make();
    for (const code of [
      SESSION_PROOF_CODE.missing,
      SESSION_PROOF_CODE.invalid,
      SESSION_PROOF_CODE.replayed,
    ]) {
      const { sent, fetchFn } = fetchAnswering({ status: 401, body: { code } });
      await sameOriginJson({ fetch: fetchFn, prove: async () => signer }, 'POST', '/p', {});
      expect(sent).to.have.length(1);
      expect(isSessionProofRefusal(code)).to.equal(true);
    }
  });

  it('sends no proof without a key, and none for a read', async () => {
    const { sent, fetchFn } = fetchAnswering({ status: 200, body: {} });
    await sameOriginJson({ fetch: fetchFn, prove: async () => null }, 'POST', '/p', {});
    await sameOriginJson({ fetch: fetchFn }, 'GET', '/r');
    expect(dpop(sent[0]!)).to.equal(undefined);
    expect(dpop(sent[1]!)).to.equal(undefined);
  });

  it('accepts any signer, not only WebCrypto', async () => {
    const real = await webCryptoSessionKeys().make();
    const other: SessionSigner = {
      alg: 'ES256',
      publicJwk: async (): Promise<PublicJwk> => real.publicJwk(),
      sign: async bytes => real.sign(bytes),
    };
    const jws = await makeProof(other, { htm: 'POST', htu: 'https://n/x', body: new Uint8Array() });
    expect(await verifies(jws)).to.equal(true);
  });
});
