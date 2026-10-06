/**
 * Step definitions: features/auth/device-consent-grant.feature
 *
 * Drives the SHIPPED device-consent grant at the API level. Where the grant lives:
 * the consent routes are served by the approving NODE's elohim-storage, not by the
 * doorway — the doorway routes no `/auth/consent/*` path, and backlog row 15
 * (arch-device-recognition-backlog.md) makes the identity/consent routes answer only
 * under the node's own names and treat a proxied loopback request as NOT local. So
 * every step here addresses a node's storage directly, as a legitimate caller on that
 * node's own machine would:
 *
 *   terminal (device node)  GET  {device}/auth/device/self        -> deviceKey, networkDna, contentDna
 *   portal page             POST {matthew}/auth/consent/view      -> label, fingerprints, askedActs
 *   portal approve          POST {matthew}/auth/consent/agree     -> returnTarget, expiresAt, consentCid
 *   terminal paste          POST {matthew}/auth/consent/redeem    -> SignedConsent (+ enrollment)
 *   terminal enroll         POST {device}/auth/device/enroll      -> 201 BindingReceipt
 *
 * LOOPBACK: on the household mesh every node is on localhost, which IS loopback, and
 * undici sends no Origin and no forwarding headers — so each call below is "this
 * machine" to the node it reaches, and the approving node resolves its single active
 * local session for Matthew (or the cookie of `POST /auth/login` when
 * E2E_MATTHEW_IDENTIFIER / E2E_MATTHEW_PASSWORD are set). A remote approval would need
 * the key-bound session of backlog row 9 (DPoP); no step here exercises that.
 *
 * Env (household defaults from hc-mesh.sh `http_port`: matthew 8090, jessica 8091):
 *   E2E_MATTHEW_NODE_URL   storage of the node that speaks for Matthew
 *   E2E_DEVICE_NODE_URL    storage of the "workspace" device node (REQUIRED — stage one
 *                          with `just mesh join-peer workspace` and point this at it)
 *   E2E_JESSICA_NODE_URL   storage of Jessica's node
 */
import { strict as assert } from 'node:assert';
import { createHash, generateKeyPairSync, randomBytes } from 'node:crypto';
import { createServer, type Server } from 'node:http';

import { Given, When, Then, Before, After } from '@cucumber/cucumber';

import { request } from 'undici';

import { noteSubstrateSkip } from '../../src/framework/fixtures/substrate-scope.js';

import type { E2EWorld } from '../../src/framework/world.js';
import type { AddressInfo } from 'node:net';

const GRANT_DOMAIN = 'elohim:consent-grant:v1';
const CLIENT_ID = 'epr-cli';
const CODE_TTL_SECONDS = 5 * 60;
const ENROLL = 'device.enroll';

interface Refusal {
  status: number;
  code: string;
}
interface Ask {
  verifier: string;
  state: string;
  request: Record<string, unknown>;
}
interface DeviceConsentState {
  matthewNode: string;
  deviceNode?: string;
  jessicaNode: string;
  identityRoot?: string;
  device?: { deviceKey: string; networkDna: string; contentDna: string };
  sessionCookie?: string;
  ask?: Ask;
  portalLink?: string;
  view?: Record<string, unknown>;
  agreed?: Record<string, unknown>;
  code?: string;
  delivered?: Record<string, unknown>;
  receipt?: Record<string, unknown>;
  refusal?: Refusal;
  listener?: { server: Server; port: number; received: Promise<URL> };
  codeTtlSeconds?: number;
}

const states = new WeakMap<E2EWorld, DeviceConsentState>();

function st(world: E2EWorld): DeviceConsentState {
  let s = states.get(world);
  if (!s) {
    s = {
      matthewNode: process.env.E2E_MATTHEW_NODE_URL ?? 'http://localhost:8090',
      deviceNode: process.env.E2E_DEVICE_NODE_URL,
      jessicaNode: process.env.E2E_JESSICA_NODE_URL ?? 'http://localhost:8091',
    };
    states.set(world, s);
  }
  return s;
}

function deviceNode(s: DeviceConsentState): string {
  if (!s.deviceNode) {
    throw new Error(
      'E2E_DEVICE_NODE_URL is unset: stage a device node with `just mesh join-peer workspace` ' +
        'and point E2E_DEVICE_NODE_URL at its storage (http://localhost:<its http port>)'
    );
  }
  return s.deviceNode;
}

async function call(
  method: 'GET' | 'POST',
  url: string,
  body?: unknown,
  cookie?: string
): Promise<{ status: number; json: Record<string, unknown>; setCookie?: string }> {
  const headers: Record<string, string> = {};
  // JSON content type: the agree/enroll routes refuse anything a cross-site form could send.
  if (body !== undefined) headers['content-type'] = 'application/json';
  if (cookie) headers.cookie = cookie;
  const res = await request(url, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.body.text();
  let json: Record<string, unknown> = {};
  try {
    json = text ? (JSON.parse(text) as Record<string, unknown>) : {};
  } catch {
    json = { raw: text };
  }
  const sc = res.headers['set-cookie'];
  const setCookie = Array.isArray(sc) ? sc.join('; ') : sc;
  return { status: res.statusCode, json, setCookie };
}

function refusalOf(r: { status: number; json: Record<string, unknown> }): Refusal {
  return { status: r.status, code: String(r.json.code ?? '') };
}

const BASE58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
function base58(bytes: Buffer): string {
  let n = BigInt(`0x${bytes.toString('hex')}`);
  let out = '';
  while (n > 0n) {
    out = BASE58[Number(n % 58n)] + out;
    n /= 58n;
  }
  for (const b of bytes) {
    if (b !== 0) break;
    out = `1${out}`;
  }
  return out;
}

/** A fresh ed25519 did:key, standing in for the device's root key (kept outside any project). */
function freshDeviceRootKey(): string {
  const { publicKey } = generateKeyPairSync('ed25519');
  const raw = publicKey.export({ format: 'der', type: 'spki' }).subarray(-32);
  return `did:key:z${base58(Buffer.concat([Buffer.from([0xed, 0x01]), raw]))}`;
}

async function readDeviceSelf(
  s: DeviceConsentState
): Promise<NonNullable<DeviceConsentState['device']>> {
  if (s.device) return s.device;
  const r = await call('GET', `${deviceNode(s)}/auth/device/self`);
  assert.equal(
    r.status,
    200,
    `device node /auth/device/self: ${r.status} ${JSON.stringify(r.json)}`
  );
  s.device = {
    deviceKey: String(r.json.deviceKey),
    networkDna: String(r.json.networkDna),
    contentDna: String(r.json.contentDna),
  };
  return s.device;
}

/**
 * The terminal asks. It composes a GrantRequest from its own node's self, keeps the
 * PKCE verifier, and prints a portal link only for a request the node admits — checked
 * with the same `POST /auth/consent/view` the portal page reads.
 */
async function terminalAsks(
  world: E2EWorld,
  acts: string[],
  returnPath: Record<string, unknown>,
  withRoot: boolean
): Promise<void> {
  const s = st(world);
  const self = await readDeviceSelf(s);
  const verifier = randomBytes(32).toString('base64url');
  const state = randomBytes(24).toString('base64url');
  const req: Record<string, unknown> = {
    domain: GRANT_DOMAIN,
    clientId: CLIENT_ID,
    deviceKey: self.deviceKey,
    label: 'workspace',
    networkDna: self.networkDna,
    contentDna: self.contentDna,
    acts,
    codeChallenge: createHash('sha256').update(verifier).digest('base64url'),
    state,
    returnPath,
  };
  if (withRoot) req.deviceRootKey = freshDeviceRootKey();
  s.ask = { verifier, state, request: req };
  const r = await call('POST', `${s.matthewNode}/auth/consent/view`, req);
  if (r.status === 200) {
    s.portalLink = `${s.matthewNode}/auth/consent/view`;
    s.refusal = undefined;
  } else {
    s.portalLink = undefined;
    s.refusal = refusalOf(r);
  }
}

async function signIn(s: DeviceConsentState): Promise<void> {
  const identifier = process.env.E2E_MATTHEW_IDENTIFIER;
  const password = process.env.E2E_MATTHEW_PASSWORD;
  if (!identifier || !password) return; // loopback: the node's active local session speaks for Matthew
  const r = await call('POST', `${s.matthewNode}/auth/login`, { identifier, password });
  assert.ok(r.status < 300, `POST /auth/login: ${r.status} ${JSON.stringify(r.json)}`);
  const m = r.setCookie?.match(/elohim_session=[^;]+/);
  s.sessionCookie = m?.[0];
}

async function openLink(s: DeviceConsentState): Promise<void> {
  assert.ok(s.ask && s.portalLink, 'the terminal printed no portal link to open');
  const r = await call('POST', `${s.matthewNode}/auth/consent/view`, s.ask.request);
  assert.equal(r.status, 200, `consent view: ${r.status} ${JSON.stringify(r.json)}`);
  s.view = r.json;
}

async function approve(s: DeviceConsentState, agreedActs?: string[]): Promise<void> {
  assert.ok(s.ask, 'no request to approve');
  const r = await call(
    'POST',
    `${s.matthewNode}/auth/consent/agree`,
    { request: s.ask.request, agreedActs: agreedActs ?? s.ask.request.acts },
    s.sessionCookie
  );
  assert.equal(
    r.status,
    200,
    `consent agree: ${r.status} ${JSON.stringify(r.json)}` +
      (r.json.code === 'consent_not_signed_in'
        ? ' — the node has no active local session; set E2E_MATTHEW_IDENTIFIER/E2E_MATTHEW_PASSWORD'
        : '')
  );
  s.agreed = r.json;
  const target = r.json.returnTarget as { kind: string; value?: string } | undefined;
  if (target?.kind === 'display') {
    // The portal shows `code#state` (consent-grant return_path.rs). The terminal
    // splits it, matches the state to the ask it made, and redeems the code alone
    // (epr-cli device.rs `parse_pasted`).
    const shown = String(target.value ?? '');
    const at = shown.lastIndexOf('#');
    assert.ok(at > 0, `the displayed value is not code#state: ${shown}`);
    assert.equal(shown.slice(at + 1), s.ask?.state, 'the displayed state does not match the ask');
    s.code = shown.slice(0, at);
  }
  const expiresAt = Number(r.json.expiresAt);
  // expiresAt's unit is not declared on the wire; normalise micros/millis/seconds.
  let secs = expiresAt;
  if (expiresAt > 1e14) secs = expiresAt / 1e6;
  else if (expiresAt > 1e11) secs = expiresAt / 1e3;
  const left = secs - Date.now() / 1000;
  const ttl = s.codeTtlSeconds ?? CODE_TTL_SECONDS;
  assert.ok(left > 0 && left <= ttl + 30, `code window ${left.toFixed(0)}s is not within ${ttl}s`);
}

async function redeem(s: DeviceConsentState, verifier: string, code: string) {
  const self = await readDeviceSelf(s);
  return call('POST', `${s.matthewNode}/auth/consent/redeem`, {
    code,
    codeVerifier: verifier,
    clientId: CLIENT_ID,
    deviceKey: self.deviceKey,
  });
}

/** The terminal redeems the code it holds, then hands the signed consent to its own node. */
async function terminalCompletes(s: DeviceConsentState): Promise<void> {
  assert.ok(s.ask && s.code, 'the terminal holds no code');
  const r = await redeem(s, s.ask.verifier, s.code);
  assert.equal(r.status, 200, `redeem: ${r.status} ${JSON.stringify(r.json)}`);
  s.delivered = r.json;
  const e = await call('POST', `${deviceNode(s)}/auth/device/enroll`, {
    request: s.ask.request,
    delivered: s.delivered,
  });
  assert.equal(e.status, 201, `device enroll: ${e.status} ${JSON.stringify(e.json)}`);
  s.receipt = e.json;
}

async function startListener(): Promise<NonNullable<DeviceConsentState['listener']>> {
  let resolve!: (u: URL) => void;
  const received = new Promise<URL>(r => (resolve = r));
  const server = createServer((req, res) => {
    resolve(new URL(req.url ?? '/', 'http://127.0.0.1'));
    res.end('ok');
  });
  await new Promise<void>(r => server.listen(0, '127.0.0.1', () => r()));
  return { server, port: (server.address() as AddressInfo).port, received };
}

// `device-node` is not a cluster-state capability, so the shared @requires gate fails open
// on it; the precondition is an env var, and this hook holds the scenario when it is unset.
Before({ tags: '@requires:device-node' }, function (this: E2EWorld, scenario) {
  if (process.env.E2E_DEVICE_NODE_URL) return undefined;
  noteSubstrateSkip(scenario.pickle.name, ['device-node'], scenario.pickle.uri);
  // eslint-disable-next-line no-console
  console.log(
    `  ⏭️  HELD (device-node): "${scenario.pickle.name}" — E2E_DEVICE_NODE_URL unset; skipped, not failed.`
  );
  return 'skipped';
});

After({ tags: '@device-consent-grant' }, function (this: E2EWorld) {
  st(this).listener?.server.close();
});

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

Given(
  'Matthew has an account that doorway {string} hosts for him',
  async function (this: E2EWorld, doorwayId: string) {
    this.getDoorway(doorwayId); // the doorway is registered; the grant itself is the node's (row 15)
    const s = st(this);
    let r = await call('GET', `${s.matthewNode}/auth/identity/standing`);
    if (r.json.code === 'consent_identity_unbootstrapped') {
      await call('POST', `${s.matthewNode}/auth/identity/bootstrap`, {});
      r = await call('GET', `${s.matthewNode}/auth/identity/standing`);
    }
    assert.equal(r.status, 200, `identity standing: ${r.status} ${JSON.stringify(r.json)}`);
    s.identityRoot = String(r.json.identityRoot);
  }
);

Given(
  'a device {string} with its own node that is not yet enrolled',
  async function (this: E2EWorld, _name: string) {
    const self = await readDeviceSelf(st(this));
    assert.ok(self.deviceKey.startsWith('u'), `device key ${self.deviceKey} is not an agent key`);
  }
);

Given(
  'Jessica, another person on the network, runs her own node, which doorway {string} does not host',
  async function (this: E2EWorld, _doorwayId: string) {
    const s = st(this);
    const r = await call('GET', `${s.jessicaNode}/auth/device/self`);
    assert.equal(r.status, 200, `Jessica's node: ${r.status} ${JSON.stringify(r.json)}`);
  }
);

Given('a one-time code is good for five minutes', function (this: E2EWorld) {
  st(this).codeTtlSeconds = CODE_TTL_SECONDS; // checked against each agreement's expiresAt
});

// ---------------------------------------------------------------------------
// The terminal asks
// ---------------------------------------------------------------------------

When(
  'the terminal on device {string} asks doorway {string} to enroll the device, returning the code by paste',
  async function (this: E2EWorld, _d: string, _dw: string) {
    await terminalAsks(this, [ENROLL], { kind: 'paste' }, false);
  }
);

When(
  "the terminal on device {string} asks doorway {string} to enroll the device, returning the code to the terminal's own listener",
  async function (this: E2EWorld, _d: string, _dw: string) {
    const s = st(this);
    s.listener = await startListener();
    await terminalAsks(this, [ENROLL], { kind: 'loopback', port: s.listener.port }, false);
  }
);

When(
  'the terminal on device {string} asks doorway {string} to bind its root key without enrolling the device',
  async function (this: E2EWorld, _d: string, _dw: string) {
    await terminalAsks(this, ['device.bind-root'], { kind: 'paste' }, true);
  }
);

When(
  'the terminal on device {string} asks doorway {string} for {string}',
  async function (this: E2EWorld, _d: string, _dw: string, act: string) {
    await terminalAsks(this, [act], { kind: 'paste' }, false);
  }
);

Then('the terminal prints a portal link and waits for a code', function (this: E2EWorld) {
  const s = st(this);
  assert.ok(s.portalLink, `no portal link: refused ${JSON.stringify(s.refusal)}`);
  assert.equal(s.code, undefined);
});

Then('the terminal prints no portal link', function (this: E2EWorld) {
  assert.equal(st(this).portalLink, undefined);
});

Then('the doorway refuses with code {string}', function (this: E2EWorld, code: string) {
  const s = st(this);
  assert.ok(s.refusal, 'nothing was refused');
  assert.equal(s.refusal.code, code, `refused ${JSON.stringify(s.refusal)}`);
});

// ---------------------------------------------------------------------------
// Matthew in the portal (API reads of what the portal page shows)
// ---------------------------------------------------------------------------

When('Matthew opens the link and signs in', async function (this: E2EWorld) {
  const s = st(this);
  await signIn(s);
  await openLink(s);
});

When(
  'Matthew opens the link, signs in, and approves the device {string}',
  async function (this: E2EWorld, _d: string) {
    const s = st(this);
    await signIn(s);
    await openLink(s);
    await approve(s);
  }
);

// The portal renders ConsentView verbatim; these read that response, not rendered UI.
Then(
  "the portal shows the name {string} and a short form of the device's key",
  async function (this: E2EWorld, name: string) {
    const s = st(this);
    const self = await readDeviceSelf(s);
    assert.equal(s.view?.label, name);
    const fp = String(s.view?.deviceFingerprint ?? '');
    assert.ok(fp.length > 0 && fp.length < self.deviceKey.length, `fingerprint "${fp}"`);
    const [head, tail] = fp.split('…');
    assert.ok(self.deviceKey.startsWith(head) && self.deviceKey.endsWith(tail ?? ''), fp);
  }
);

Then(
  'the portal lists {string} as the only thing being asked',
  function (this: E2EWorld, _label: string) {
    // "enroll this device" is the portal's words for the act `device.enroll`.
    assert.deepEqual(st(this).view?.askedActs, [ENROLL]);
  }
);

// Reads the agree response's returnTarget, which the portal renders verbatim.
Then('the portal shows Matthew a code to paste', function (this: E2EWorld) {
  const s = st(this);
  assert.equal((s.agreed?.returnTarget as { kind?: string })?.kind, 'display');
  assert.ok(s.code, 'no code displayed');
});

Then('the portal shows no code to paste', function (this: E2EWorld) {
  const s = st(this);
  assert.equal((s.agreed?.returnTarget as { kind?: string })?.kind, 'redirect');
  assert.equal(s.code, undefined);
});

// The step follows the redirect exactly as the browser would; the listener is the terminal's.
Then('the browser hands the code to the terminal', async function (this: E2EWorld) {
  const s = st(this);
  const url = String((s.agreed?.returnTarget as { url?: string })?.url ?? '');
  await request(url, { method: 'GET' }).then(async r => r.body.dump());
  const got = await s.listener!.received;
  assert.equal(got.searchParams.get('state'), s.ask?.state, 'state does not match the ask');
  s.code = got.searchParams.get('code') ?? undefined;
  assert.ok(s.code, 'the listener received no code');
});

// ---------------------------------------------------------------------------
// The code
// ---------------------------------------------------------------------------

When('Matthew pastes the code into the terminal', async function (this: E2EWorld) {
  await terminalCompletes(st(this));
});

Then(
  "the device {string} is enrolled under Matthew's identity",
  async function (this: E2EWorld, _d: string) {
    const s = st(this);
    if (!s.receipt) await terminalCompletes(s); // the loopback flow: terminal finishes on its own
    assert.ok(s.receipt?.bindingAction, `no binding receipt: ${JSON.stringify(s.receipt)}`);
    const record = s.delivered?.record as { identityRoot?: string } | undefined;
    assert.equal(record?.identityRoot, s.identityRoot, 'consent names another identity');
  }
);

Given(
  'Matthew has approved the device {string} and the portal has shown a code',
  async function (this: E2EWorld, _d: string) {
    const s = st(this);
    await terminalAsks(this, [ENROLL], { kind: 'paste' }, false);
    await signIn(s);
    await openLink(s);
    await approve(s);
    assert.ok(s.code, 'the portal showed no code');
  }
);

Given(
  'the device {string} has been enrolled with a code',
  async function (this: E2EWorld, _d: string) {
    const s = st(this);
    await terminalAsks(this, [ENROLL], { kind: 'paste' }, false);
    await signIn(s);
    await openLink(s);
    await approve(s);
    await terminalCompletes(s);
  }
);

When(
  'a terminal on another machine, which did not ask, presents that code to doorway {string}',
  async function (this: E2EWorld, _dw: string) {
    const s = st(this);
    // It saw the code and knows the (public) device key, but not the asking terminal's verifier.
    const r = await redeem(s, randomBytes(32).toString('base64url'), s.code!);
    s.refusal = r.status === 200 ? undefined : refusalOf(r);
    assert.notEqual(r.status, 200, 'a terminal that did not ask redeemed the code');
  }
);

Then(
  'the doorway refuses that code from then on, even from the terminal that asked',
  async function (this: E2EWorld) {
    const s = st(this);
    const r = await redeem(s, s.ask!.verifier, s.code!);
    assert.notEqual(r.status, 200, 'the burned code was redeemed');
    assert.match(String(r.json.code ?? ''), /^redemption_/);
  }
);

Then('the device {string} is not enrolled', function (this: E2EWorld, _d: string) {
  // Enrolling needs the delivered consent; the asking terminal never obtained one, so
  // it had nothing to hand its node. (No route reads a device's binding — see row 7.)
  const s = st(this);
  assert.equal(s.delivered, undefined);
  assert.equal(s.receipt, undefined);
});

When(
  'the terminal on device {string} presents that code to doorway {string} again',
  async function (this: E2EWorld, _d: string, _dw: string) {
    const s = st(this);
    const r = await redeem(s, s.ask!.verifier, s.code!);
    s.refusal = r.status === 200 ? undefined : refusalOf(r);
    assert.notEqual(r.status, 200, 'the code was redeemed twice');
  }
);
