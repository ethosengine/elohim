// Pins the chrome's pure frame-attribute computation by loading the element in a
// bare vm sandbox (no DOM) and calling the test hook it exposes.
import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// The element carries a placeholder the crate's build.rs splices; do the same here.
const vocab = JSON.stringify(
  JSON.parse(readFileSync(fileURLToPath(new URL('../src/generated/chrome-vocabulary.json', import.meta.url)), 'utf8'))
);
const src = readFileSync(fileURLToPath(new URL('../src/omni-element.js', import.meta.url)), 'utf8').replace(
  '/*__CHROME_VOCABULARY__*/null',
  vocab
);
const window = { location: { pathname: '/', hostname: 'localhost' } };
const document = { readyState: 'complete', getElementById: () => null };
vm.runInNewContext(src, { window, document });
const {
  frameAttributes, holdersLine, safeHref, safeEncode, buildEprHref, sourceReading, deliveryReading,
  hopLine, relativeTime, witnessFromAnchor, contentFromRecord, isSignedIn, readingUrl, readGrants,
  writeGrant, changeGrant, contentFromFacing, dialStateFromHolders, proofClassWord, legibleAccent, parseColor, isFramed, adoptServedBy, noteServedBy, model, checkSession, pageInfoRows
} = window.__elohimOmni;

const PAIR = { boundary: 'origin-pair' };
const T = (caps, iso = PAIR) => ({ mode: 'trustful', capabilities: caps, isolation: iso });

const DENY_ALL =
  "fullscreen 'none'; clipboard-write 'none'; camera 'none'; microphone 'none'; geolocation 'none'";

test('trustful with popups declared, on an origin pair', () => {
  const out = frameAttributes(T(['popups']), {});
  assert.equal(out.sandbox, 'allow-scripts allow-same-origin allow-popups');
  assert.equal(out.allow, DENY_ALL);
});

test('trustful declares everything, in fixed order', () => {
  const out = frameAttributes(T(['clipboard-write', 'downloads', 'fullscreen', 'popups', 'forms']), {});
  assert.equal(out.sandbox, 'allow-scripts allow-same-origin allow-forms allow-popups allow-downloads');
  assert.equal(out.allow, "fullscreen; clipboard-write; camera 'none'; microphone 'none'; geolocation 'none'");
});

test('undeclared is never given', () => {
  assert.equal(frameAttributes(T([]), { popups: true }).sandbox, 'allow-scripts allow-same-origin');
});

test('the person narrows a declared capability', () => {
  const out = frameAttributes(T(['popups', 'forms']), { popups: false });
  assert.equal(out.sandbox, 'allow-scripts allow-same-origin allow-forms');
});

test('device capabilities need the explicit grant', () => {
  const c = T(['camera']);
  assert.match(frameAttributes(c, {}).allow, /camera 'none'/);
  assert.match(frameAttributes(c, { camera: true }).allow, /(^|; )camera(;|$)/);
});

test('no origin separation, no allow-same-origin (none, opaque, missing)', () => {
  for (const iso of [{ boundary: 'none' }, { boundary: 'opaque-sandbox' }, null, {}]) {
    const out = frameAttributes(T(['forms'], iso), {});
    assert.equal(out.sandbox, 'allow-scripts allow-forms', JSON.stringify(iso));
    assert.equal(out.ownOrigin, false);
  }
});

test('origin-separating boundaries keep the content its own origin', () => {
  for (const boundary of ['origin-pair', 'scheme-per-record', 'separate-webview']) {
    assert.match(frameAttributes(T([], { boundary }), {}).sandbox, /allow-same-origin/);
  }
});

test('an explicit isolation argument wins over the content’s own', () => {
  const out = frameAttributes(T([], PAIR), {}, { boundary: 'none' });
  assert.equal(out.sandbox, 'allow-scripts');
});

test('trustless (and anything not exactly trustful) is script-only', () => {
  for (const mode of ['trustless', undefined, 'banana']) {
    const out = frameAttributes({ mode, capabilities: ['popups', 'forms'], isolation: PAIR }, { popups: true });
    assert.equal(out.sandbox, 'allow-scripts');
    assert.equal(out.allow, DENY_ALL);
    assert.equal(JSON.stringify(out.granted), '[]');
  }
});

test('never a top-navigation or sandbox-escaping token', () => {
  const out = frameAttributes(T(['forms', 'popups', 'downloads', 'fullscreen', 'camera']), { camera: true });
  assert.ok(!/top-navigation|popups-to-escape/.test(out.sandbox + out.allow));
});

test('every capability not given is explicitly denied in allow', () => {
  assert.equal(frameAttributes(T([]), { camera: true }).allow, DENY_ALL);
});

test('holders: headline verbatim, counts as fallback, unknown only when null', () => {
  assert.equal(holdersLine({ holders: 3, target: 3, headline: 'Safe in 3 homes' }), 'Safe in 3 homes');
  assert.equal(holdersLine({ holders: 3, target: 3 }), 'Held by 3 of 3 households');
  assert.equal(holdersLine({ holders: 0, target: 3 }), 'Not yet held by any household');
  assert.equal(holdersLine(null), 'Held by households · see where it lives');
});

test('relative time', () => {
  const now = Date.parse('2026-10-15T12:00:00Z');
  assert.equal(relativeTime('2026-10-15T11:55:00Z', now), '5 minutes ago');
  assert.equal(relativeTime('2026-10-15T09:00:00Z', now), '3 hours ago');
  assert.equal(relativeTime('nonsense', now), '');
});

test('a lone surrogate never throws; the value is omitted or degraded', () => {
  assert.equal(safeEncode('\ud800'), null);
  assert.equal(safeEncode('ok id'), 'ok%20id');
  const d = { path: [{ role: 'doorway', name: 'n', id: '\ud800' }, { role: 'holder', peer: 'p 1' }] };
  assert.equal(buildEprHref('rec', d, 'source'), '/epr/rec?peer=p%201#source');
  assert.equal(buildEprHref('\ud800', null, 'source'), '/');
});

test('the EPR link carries the path (via, peer)', () => {
  const d = { path: [{ role: 'doorway', name: 'a', id: 'alpha-A' }, { role: 'holder', peer: 'abc' }] };
  assert.equal(buildEprHref('evo', d, 'delivery'), '/epr/evo?via=alpha-A&peer=abc#delivery');
  assert.equal(buildEprHref('evo', { path: [{ role: 'self' }] }, 'source'), '/epr/evo#source');
});

test('delivery readings per hop', () => {
  assert.equal(hopLine({ role: 'self' }), 'From your own peer');
  assert.equal(hopLine({ role: 'holder', peer: 'uhCAkpeerkey', transport: 'iroh' }), 'From peer uhCAkpeerkey over iroh');
  assert.equal(hopLine({ role: 'doorway', name: 'alpha.elohim.host', id: 'alpha-A' }), 'Through alpha.elohim.host (doorway alpha-A)');
  assert.equal(deliveryReading({ path: [{ role: 'self' }] }), 'From your own peer');
  assert.equal(
    deliveryReading({ path: [{ role: 'doorway', name: 'alpha.elohim.host', id: 'a' }, { role: 'holder', peer: 'abcdef' }] }),
    'Reached you through alpha.elohim.host from peer abcdef'
  );
  assert.equal(deliveryReading(null), 'How this reached you');
});

test('safeHref is same-origin-relative or http(s) only', () => {
  assert.equal(safeHref('/epr/x', null), '/epr/x');
  assert.equal(safeHref('https://a.example/x', null), 'https://a.example/x');
  for (const bad of ['//evil.example', '/\\evil.example', '/\t/evil.example', '/a\nb', 'javascript:alert(1)', 'data:text/html,x']) {
    assert.equal(safeHref(bad, null), null, JSON.stringify(bad));
  }
});

test('with no content the Source reading claims neither verification nor witness', () => {
  assert.equal(sourceReading(null, 'elohim-host-landing'), 'Record · elohim-host-landing');
  assert.ok(!/Verified|Witness/i.test(sourceReading({}, 'x')));
  assert.equal(sourceReading({ contentAddress: 'sha256-abc', witness: 'amber' }, 'x'), 'Awaiting witness · sha256-abc');
});

test('witness is derived from the anchor', () => {
  assert.equal(witnessFromAnchor('uhCkkabc'), 'green');
  for (const a of ['', '  ', null, undefined]) assert.equal(witnessFromAnchor(a), 'amber');
});

test('content is built from a record when the island had none', () => {
  const c = contentFromRecord({
    id: 'evo', blobHash: 'sha256-abc', contentFormat: 'html5-app', reach: 'commons', dhtAnchorHash: null,
    contentBody: JSON.stringify({ entryPoint: 'app.html', capabilities: ['popups', 'teleport'] })
  });
  assert.equal(c.recordId, 'evo');
  assert.equal(c.witness, 'amber');
  assert.equal(c.entry, 'app.html');
  assert.equal(JSON.stringify(c.capabilities), '["popups"]');
  assert.equal(c.frame, undefined);
  assert.equal(contentFromRecord({ id: 'x' }), null);
  assert.equal('witness' in contentFromRecord({ id: 'x', blobHash: 'h', contentFormat: 'markdown' }), false);
});

test('viewer.kind supersedes the deprecated boolean', () => {
  assert.equal(isSignedIn({ viewer: { kind: 'visitor' }, authenticated: true }), false);
  assert.equal(isSignedIn({ viewer: { kind: 'hosted-session' } }), true);
  assert.equal(isSignedIn({ viewer: { kind: 'device-agent' } }), true);
  assert.equal(isSignedIn({ authenticated: true }), true);
  assert.equal(isSignedIn({}), false);
});

test('readingsBase prefixes every projection fetch', () => {
  assert.equal(readingUrl({ readingsBase: 'http://127.0.0.1:8090/' }, '/api/v1/x'), 'http://127.0.0.1:8090/api/v1/x');
  assert.equal(readingUrl({}, '/api/v1/x'), '/api/v1/x');
  assert.equal(readingUrl({ readingsBase: 'javascript:alert(1)' }, '/api/v1/x'), '/api/v1/x');
});

test('grants are keyed by content address; the recordId key migrates once', () => {
  const store = new Map();
  window.localStorage = {
    getItem: (k) => (store.has(k) ? store.get(k) : null),
    setItem: (k, v) => store.set(k, v),
    removeItem: (k) => store.delete(k)
  };
  store.set('elohim-grants:evo', JSON.stringify({ popups: false }));
  const content = { contentAddress: 'sha256-abc' };
  assert.equal(readGrants(content, 'evo').popups, false);
  assert.equal(store.has('elohim-grants:evo'), false, 'old key dropped');
  assert.equal(store.has('elohim-grants:sha256-abc'), true, 'new key written');
  writeGrant(content, 'evo', 'camera', true);
  assert.equal(JSON.parse(store.get('elohim-grants:sha256-abc')).camera, true);
  // A new bundle (new address) starts from the declaration again.
  assert.equal(Object.keys(readGrants({ contentAddress: 'sha256-new' }, 'evo')).length, 0);
});

function storeWith(opts = {}) {
  window.top = window; // top-level unless a test says otherwise
  const store = new Map();
  window.localStorage = {
    getItem: (k) => (opts.hide ? null : store.has(k) ? store.get(k) : null),
    setItem: (k, v) => {
      if (opts.throwOnSet) throw new Error('QuotaExceededError');
      store.set(k, v);
    },
    removeItem: (k) => store.delete(k)
  };
  return store;
}

test('a grant that cannot be persisted leaves the state unchanged and says so', () => {
  const content = { contentAddress: 'sha256-q' };
  storeWith({ throwOnSet: true });
  const r = changeGrant(content, 'evo', 'popups');
  assert.equal(r.ok, false);
  assert.equal(r.granted, true, 'popups stays allowed: the narrowing was not kept');
  const dev = changeGrant(content, 'evo', 'camera');
  assert.equal(dev.ok, false);
  assert.equal(dev.granted, false, 'a device grant that was not kept is not shown as allowed');
});

test('storage that accepts a write but returns nothing is not a kept choice', () => {
  storeWith({ hide: true });
  assert.equal(changeGrant({ contentAddress: 'sha256-h' }, 'evo', 'camera').ok, false);
});

test('a persisted grant change reports the new state', () => {
  storeWith();
  const content = { contentAddress: 'sha256-ok' };
  const r = changeGrant(content, 'evo', 'popups');
  assert.equal(r.ok, true);
  assert.equal(r.granted, false);
  assert.equal(changeGrant(content, 'evo', 'popups').granted, true);
});

test('no storage at all is a failed write', () => {
  window.top = window;
  window.localStorage = undefined;
  assert.equal(writeGrant({ contentAddress: 'a' }, 'evo', 'popups', false), false);
});

test('content from a facing: trustful protocol record, declared capabilities only', () => {
  const c = contentFromFacing({
    recordId: 'evo', title: 't', format: 'html5-app', mode: 'trustful', reach: 'commons', witness: 'green',
    computedAt: 'x', contentAddress: 'bafyabc', app: { entry: 'app.html', capabilities: ['forms', 'teleport'], presents: false }
  });
  assert.equal(c.witness, 'green');
  assert.equal(c.entry, 'app.html');
  assert.equal(JSON.stringify(c.capabilities), '["forms"]');
  assert.equal(c.mode, 'trustful');
  assert.equal(contentFromFacing({ recordId: 'x', witness: 'amber' }), null, 'no content address, no content');
  assert.equal(contentFromFacing({ recordId: 'x', contentAddress: 'a', mode: 'other' }).mode, 'trustless');
});

test('the dial is drawn from the holders lens', () => {
  const d = dialStateFromHolders({ holders: 5, target: 3, reassurance: 'protected' });
  assert.equal(d.lit, 3);
  assert.equal(d.floorMet, true);
  assert.equal(d.measured, true);
  assert.equal(dialStateFromHolders({ holders: 0, target: 3, reassurance: 'not-yet-seen' }).measured, false);
});

test('proof classes read in plain words', () => {
  assert.equal(proofClassWord('self-attest'), 'self-attested');
  assert.equal(proofClassWord('witness'), 'witnessed');
  assert.equal(proofClassWord(undefined), 'not stated');
});

test('theme colours: unparseable are ignored, illegible are dropped, legible are kept', () => {
  assert.equal(parseColor('not a colour'), null);
  assert.equal(parseColor('url(x)'), null);
  assert.equal(legibleAccent('javascript:alert(1)', 'auto'), '');
  assert.equal(legibleAccent('#ffff00', 'light'), '', 'yellow on white is illegible');
  assert.equal(legibleAccent('#1a4fd6', 'light'), 'rgb(26, 79, 214)');
  assert.equal(legibleAccent('#1a4fd6', 'auto'), '', 'must read on both light and dark chips');
  assert.equal(legibleAccent('#7fb0ff', 'dark'), 'rgb(127, 176, 255)');
  assert.equal(legibleAccent('rgb(300, 0, 0)', 'light'), '');
});

test('framed: grants are read-only, nothing is written', () => {
  const store = storeWith();
  window.top = {}; // another page owns the top
  const content = { contentAddress: 'sha256-fr' };
  const r = changeGrant(content, 'evo', 'popups');
  assert.equal(r.ok, false);
  assert.equal(r.framed, true);
  assert.equal(r.granted, true, 'effective state is unchanged');
  assert.equal(store.size, 0, 'nothing written');
  assert.equal(readGrants(content, 'evo').popups, undefined);
});

test('a top access that throws counts as framed', () => {
  storeWith();
  Object.defineProperty(window, 'top', { configurable: true, get() { throw new Error('cross-origin'); } });
  assert.equal(isFramed(), true);
  assert.equal(changeGrant({ contentAddress: 'x' }, 'evo', 'popups').ok, false);
  Object.defineProperty(window, 'top', { configurable: true, writable: true, value: window });
});

test('top-level: grant changes persist as before', () => {
  storeWith();
  assert.equal(isFramed(), false);
  const r = changeGrant({ contentAddress: 'sha256-top' }, 'evo', 'popups');
  assert.equal(r.ok, true);
  assert.equal(r.granted, false);
});

const hdr = (v) => ({ headers: { get: (k) => (k.toLowerCase() === 'x-elohim-served-by' && v != null ? v : null) } });
const islandWith = () => ({
  delivery: {
    path: [
      { role: 'doorway', name: 'alpha.elohim.host', id: 'alpha-A', did: 'did:web:a', reportedBy: 'doorway' },
      { role: 'holder', peer: 'k', reportedBy: 'doorway' }
    ]
  }
});

test('served-by differs: the doorway hop moves and the cue plays once', () => {
  model.content = islandWith();
  model.cuePlays = 0;
  noteServedBy(hdr('alpha-B'));
  const hop = model.content.delivery.path[0];
  assert.equal(hop.id, 'alpha-B');
  assert.equal(hop.reportedBy, 'doorway');
  assert.equal('did' in hop, false, 'the old doorway DID is dropped');
  assert.equal(model.cuePlays, 1);
  assert.equal(model.content.delivery.path[1].peer, 'k', 'holder hop untouched');
  // The EPR link follows the new doorway.
  assert.match(buildEprHref('evo', model.content.delivery, 'delivery'), /via=alpha-B/);
  // The same answer again is not a change.
  noteServedBy(hdr('alpha-B'));
  assert.equal(model.cuePlays, 1);
});

test('served-by same, absent, or with no doorway hop: nothing happens', () => {
  model.content = islandWith();
  model.cuePlays = 0;
  noteServedBy(hdr('alpha-A'));
  noteServedBy(hdr(null));
  noteServedBy(hdr('  '));
  noteServedBy(null);
  assert.equal(model.cuePlays, 0);
  assert.equal(model.content.delivery.path[0].id, 'alpha-A');
  const selfOnly = { delivery: { path: [{ role: 'self', reportedBy: 'self' }] } };
  assert.equal(adoptServedBy(selfOnly, 'alpha-B'), false);
  model.content = null;
  noteServedBy(hdr('alpha-B'));
  assert.equal(model.cuePlays, 0);
});

function sessionSetup({ token, meStatus }) {
  const calls = [];
  window.localStorage = { getItem: (k) => (k === 'elohim-auth-token' ? token ?? null : null), setItem() {}, removeItem() {} };
  window.fetch = (url, opts) => {
    calls.push({ url, opts });
    if (url === '/.well-known/elohim-auth') return Promise.resolve({ ok: true, status: 200, json: () => Promise.resolve({ endpoints: { me: '/auth/me' } }) });
    return Promise.resolve({ ok: meStatus === 200, status: meStatus, json: () => Promise.resolve({}) });
  };
  model.ctx = { slug: 'evo', viewer: { kind: 'visitor' }, medium: 'web2-doorway' };
  model.content = null;
  model.sessionChecked = false;
  model.sessionViewer = false;
  return calls;
}

test('stored token and a 200 from me: the Yours row appears, the island is untouched', async () => {
  const calls = sessionSetup({ token: 'jwt-abc', meStatus: 200 });
  assert.equal(await checkSession(), true);
  assert.ok(pageInfoRows().includes('yours'));
  assert.equal(calls[1].url, '/auth/me');
  assert.equal(calls[1].opts.headers.authorization, 'Bearer jwt-abc');
  assert.equal(model.ctx.viewer.kind, 'visitor', 'the island is not mutated');
});

test('stored token and a 401: no Yours row, token left alone', async () => {
  sessionSetup({ token: 'jwt-abc', meStatus: 401 });
  assert.equal(await checkSession(), false);
  assert.ok(!pageInfoRows().includes('yours'));
  assert.equal(window.localStorage.getItem('elohim-auth-token'), 'jwt-abc');
});

test('no token: no request at all', async () => {
  const calls = sessionSetup({ token: null, meStatus: 200 });
  assert.equal(await checkSession(), false);
  assert.equal(calls.length, 0);
});

test('the check runs once, and only for a web2 visitor', async () => {
  const calls = sessionSetup({ token: 't', meStatus: 401 });
  await checkSession();
  await checkSession();
  assert.equal(calls.length, 2, 'discovery + me, once');
  const peer = sessionSetup({ token: 't', meStatus: 200 });
  model.ctx.medium = 'peer-native';
  await checkSession();
  assert.equal(peer.length, 0);
  const dev = sessionSetup({ token: 't', meStatus: 200 });
  model.ctx.viewer = { kind: 'device-agent' };
  await checkSession();
  assert.equal(dev.length, 0);
});
