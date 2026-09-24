/**
 * Attention witnessed privately — step definitions.
 *
 * Drives genesis/a2o/features/lms/attention-witnessed-privately.feature
 * (@concern:attention-witnessed-privately): a person's reading of a page is
 * witnessed as an agent-private `lamad:content-viewed` observation on their
 * own storage peer, shown back to them on `/lamad/me/stream` with the recipe
 * CID printed, invisible to another person's stream, and never announced to
 * other peers.
 *
 * Surfaces this file reads (each is the thing the story claims, not a proxy):
 *   - the content viewer at `/resource/:id` (elohim-app shell) — it opens the
 *     witnessed view on load, tracks the window's deepest scroll, and POSTs
 *     `/api/v1/observations` when the viewer is destroyed or moves to another
 *     node. This lane exercises the IN-APP leave: the "moves on … without
 *     leaving the app" step routes inside the shell and waits for the POST's
 *     answer, which is the leave whose ack a scenario can read. A HARD leave
 *     (page.goto, tab close, cross-bundle navigation) is now witnessed too —
 *     the emitter flushes every open view on `pagehide` with `sendBeacon` or a
 *     keepalive fetch (ruling R-A8, covered by
 *     `app/elohim-library/projects/elohim-rea-runtime/src/lib/observation-emitter.service.spec.ts`
 *     "hard leave") — so the in-app step is kept for the readable ack, not
 *     because it is the only path that witnesses.
 *   - `/lamad/me/stream` (the lamad bundle) — testids `stream-provenance`
 *     (recipe name + `<code>` CID), `stream-entry` (an `<a href="/epr/{id}">`,
 *     dwell as "3.4 s" / "1 min 5 s", depth as "N%").
 *   - `GET /api/v1/observations/stream` through the doorway with the person's
 *     own bearer token (the doorway injects `X-Agent-Cid` from the JWT).
 *   - the storage peer's own `/metrics`:
 *     `elohim_observation_cursor_suppressed_total{kind}` and
 *     `elohim_observation_cursor_announced_total{kind}` (counter vecs — a
 *     label set that never incremented is absent, which reads as 0).
 *
 * Browser-tier: every page step returns 'pending' when the human has no
 * PlaywrightDevice (HTTP-only lanes), mirroring epr-link-navigation.steps.ts.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { getRaw } from '../../src/framework/dataplane/surfaces.js';
import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import {
  loadHouseholdMeshFixture,
  requireFixturePrimaryStorageUrl,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants and wire shapes
// ---------------------------------------------------------------------------

const PENDING = 'pending';

const SUPPRESSED_SERIES = 'elohim_observation_cursor_suppressed_total';
const ANNOUNCED_SERIES = 'elohim_observation_cursor_announced_total';

const OBSERVATION_WRITE_PATH = '/api/v1/observations';
const OBSERVATION_STREAM_PATH = '/api/v1/observations/stream';

const STREAM_TESTID = {
  PROVENANCE: 'stream-provenance',
  ENTRY: 'stream-entry',
} as const;

/** Minimal slice of `ObservationAcceptedView` this file reads. */
interface ObservationAck {
  observerCid?: string;
  observerCidNamespace?: string;
  logOffset?: number;
  seq?: number;
  signed?: string;
}

/** Minimal slice of `ObservationStreamView` this file reads. */
interface ObservationStream {
  recipe: { name: string; cid: string };
  entries: { subjectCid: string | null; dwellMs: number; scrollDepthPct: number }[];
}

/** Minimal slice of a Playwright Response (the device's PWPage stub has no type for it). */
interface PWResponseLike {
  url(): string;
  status(): number;
  request(): { method(): string };
  text(): Promise<string>;
}

interface WitnessedPost {
  status: number;
  body: string;
}

/** Per-scenario state, keyed by the world so parallel workers never share it. */
interface AttentionState {
  metricsBase?: { storageUrl: string; kind: string; suppressed: number; announced: number };
  posts: WitnessedPost[];
  listening: boolean;
  openedPage?: string;
  ack?: ObservationAck;
}

const STATE = new WeakMap<E2EWorld, AttentionState>();

function state(world: E2EWorld): AttentionState {
  let s = STATE.get(world);
  if (!s) {
    s = { posts: [], listening: false };
    STATE.set(world, s);
  }
  return s;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** The named human's PlaywrightDevice, or null in an HTTP-only lane. */
function deviceOf(world: E2EWorld, name: string): PlaywrightDevice | null {
  const human = world.humans.get(name);
  if (!human) return null;
  for (const device of human.devices) {
    if (device instanceof PlaywrightDevice) return device;
  }
  return null;
}

/** The doorway the human signed in on and the bearer token it issued. */
function signedIn(world: E2EWorld, name: string): { doorwayUrl: string; token: string } {
  const human = world.getHuman(name);
  const first = [...human.tokens.entries()][0];
  assert.ok(first, `${name} holds no doorway token — the Background sign-in did not complete`);
  const [doorwayId, token] = first;
  let doorwayUrl = world.getDoorway(doorwayId).url;
  while (doorwayUrl.endsWith('/')) doorwayUrl = doorwayUrl.slice(0, -1);
  return { doorwayUrl, token };
}

/** GET the person's own stream through the doorway with their own sign-in. */
async function fetchStream(world: E2EWorld, name: string): Promise<ObservationStream> {
  const { doorwayUrl, token } = signedIn(world, name);
  const url = `${doorwayUrl}${OBSERVATION_STREAM_PATH}`;
  const res = await getRaw(url, { headers: { Authorization: `Bearer ${token}` } });
  assert.equal(
    res.status,
    200,
    `GET ${url} as ${name} returned ${res.status}: ${res.text.slice(0, 300)} — the person's own ` +
      'stream must answer under their own sign-in'
  );
  return JSON.parse(res.text) as ObservationStream;
}

/**
 * Sum one counter's samples for a single `kind` label value. A label set that
 * never incremented is absent from the exposition — that is 0, not unknown.
 */
function counterForKind(text: string, series: string, kind: string): number {
  let total = 0;
  const needle = `kind="${kind}"`;
  for (const raw of text.split('\n')) {
    const line = raw.trim();
    if (line.startsWith('#') || !line.startsWith(`${series}{`)) continue;
    const close = line.indexOf('}');
    if (close === -1 || !line.slice(series.length, close).includes(needle)) continue;
    const value = Number(
      line
        .slice(close + 1)
        .trim()
        .split(/\s+/)[0]
    );
    if (!Number.isNaN(value)) total += value;
  }
  return total;
}

async function scrapeStorage(storageUrl: string): Promise<string> {
  const res = await getRaw(`${storageUrl}/metrics`, { timeoutMs: 10_000 });
  assert.equal(
    res.status,
    200,
    `GET ${storageUrl}/metrics returned ${res.status} — the storage peer's Prometheus surface must answer`
  );
  return res.text;
}

/**
 * Read a stream entry's words: dwell as the page writes it ("3.4 s", "42 s",
 * "1 min", "1 min 5 s") in ms, and depth ("N%") in percent. Token-based — the
 * entry's text is short and its shape is fixed by the page's formatter.
 */
function readEntryMeasures(text: string): { dwellMs: number | null; depthPct: number | null } {
  const tokens = text.split(/\s+/).filter(t => t.length > 0);
  let dwellMs: number | null = null;
  let depthPct: number | null = null;
  tokens.forEach((token, i) => {
    const unit = tokens[i + 1];
    const value = Number(token);
    if (!Number.isNaN(value) && (unit === 'min' || unit === 's')) {
      dwellMs = (dwellMs ?? 0) + value * (unit === 'min' ? 60_000 : 1000);
    } else if (token.endsWith('%') && !Number.isNaN(Number(token.slice(0, -1)))) {
      depthPct = Number(token.slice(0, -1));
    }
  });
  return { dwellMs, depthPct };
}

async function sleep(ms: number): Promise<void> {
  await new Promise<void>(resolve => setTimeout(resolve, ms));
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

/**
 * Write down the storage peer's current counts for one observation kind, so the
 * final step reads a DELTA — the counters are cumulative since the peer booted
 * and a second run must not pass on the first run's increment.
 *
 * Example:
 *   Given before Jessica reads, the storage peer behind doorway "alpha" is asked how many "lamad:content-viewed" notes it has withheld from other peers so far
 */
Given(
  'before {word} reads, the storage peer behind doorway {string} is asked how many {string} notes it has withheld from other peers so far',
  async function (this: E2EWorld, _reader: string, doorwayId: string, kind: string) {
    const storageUrl = requireFixturePrimaryStorageUrl(loadHouseholdMeshFixture(), doorwayId);
    const text = await scrapeStorage(storageUrl);
    state(this).metricsBase = {
      storageUrl,
      kind,
      suppressed: counterForKind(text, SUPPRESSED_SERIES, kind),
      announced: counterForKind(text, ANNOUNCED_SERIES, kind),
    };
  }
);

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

/**
 * Open a seeded content node in the shell's content viewer (`/resource/:id`)
 * and wait until its body has rendered taller than the window, so "scroll to
 * the bottom" means something. Starts listening for the observation POST.
 *
 * Example: When Jessica opens the "manifesto" page
 */
When('{word} opens the {string} page', async function (this: E2EWorld, name: string, id: string) {
  const device = deviceOf(this, name);
  if (!device) return PENDING;
  const s = state(this);
  if (!s.listening) {
    device.page.on('response', (...args: unknown[]) => {
      const res = args[0] as PWResponseLike;
      let path = '';
      try {
        path = new URL(res.url()).pathname;
      } catch {
        return;
      }
      if (path !== OBSERVATION_WRITE_PATH || res.request().method() !== 'POST') return;
      void res
        .text()
        .catch(() => '')
        .then(body => s.posts.push({ status: res.status(), body }));
    });
    s.listening = true;
  }
  s.openedPage = id;
  await device.navigate(`/resource/${encodeURIComponent(id)}`);
  await device.page.waitForFunction(
    () => document.documentElement.scrollHeight > window.innerHeight * 1.5,
    undefined,
    { timeout: 30_000 }
  );
});

/**
 * Stay on the page. Half a second of margin above the stated floor: the stream
 * page rounds dwell to a tenth of a second, and the floor must hold exactly.
 *
 * Example: And Jessica stays on the page for at least 3 seconds
 */
When(
  '{word} stays on the page for at least {int} seconds',
  async function (this: E2EWorld, name: string, seconds: number) {
    const device = deviceOf(this, name);
    if (!device) return PENDING;
    await device.page.waitForTimeout(seconds * 1000 + 500);
  }
);

/**
 * Scroll the window to the end of the page. The viewer measures depth from the
 * WINDOW's scroll events; if the window did not move (the page scrolls inside
 * an inner container instead), the viewer cannot see this reading and the step
 * says so rather than letting the depth assertion fail later without a cause.
 *
 * Example: And Jessica scrolls to the bottom of the page
 */
When('{word} scrolls to the bottom of the page', async function (this: E2EWorld, name: string) {
  const device = deviceOf(this, name);
  if (!device) return PENDING;
  // Two hops so a lazily-growing page is still reached at its final bottom.
  for (let hop = 0; hop < 2; hop += 1) {
    await device.page.evaluate(() =>
      window.scrollTo({ top: document.documentElement.scrollHeight, behavior: 'instant' })
    );
    await device.page.waitForTimeout(400);
  }
  const scrolled = (await device.page.evaluate(() => window.scrollY)) as number;
  assert.ok(
    scrolled > 0,
    'the window did not scroll (scrollY is 0) although the page is taller than the window — the ' +
      "page scrolls inside an inner container, so the content viewer's window scroll listener " +
      'cannot witness how far the reader got'
  );
});

/**
 * Leave the content node by an in-app route change to the shell's home page
 * (`/`). Angular's router follows a popstate to the current history entry, so
 * the viewer is destroyed inside the living document and its POST is sent.
 * A page.goto would now also witness (the emitter's `pagehide` flush, R-A8),
 * but its request outlives the document with no response this lane can await —
 * the in-app leave is what gives the next step an ack to read.
 *
 * Example: And Jessica moves on from the page to the home page without leaving the app
 */
When(
  '{word} moves on from the page to the home page without leaving the app',
  async function (this: E2EWorld, name: string) {
    const device = deviceOf(this, name);
    if (!device) return PENDING;
    await device.page.evaluate(() => {
      globalThis.history.pushState({}, '', '/');
      globalThis.dispatchEvent(new PopStateEvent('popstate', { state: {} }));
    });
    await device.page.waitForURL(url => url.pathname === '/', { timeout: 15_000 });
  }
);

/**
 * Wait for the storage peer's answer to the observation POST the leave sent,
 * and check it names this person as the observer (as the doorway asserted
 * them — the browser path carries the JWT's human id; the agent key is the
 * other namespace, so either is accepted and the ack's namespace is recorded).
 *
 * Example: And Jessica waits until her storage peer replies to her app that it has kept her note on the "manifesto" page
 */
When(
  '{word} waits until her storage peer replies to her app that it has kept her note on the {string} page',
  async function (this: E2EWorld, name: string, id: string) {
    if (!deviceOf(this, name)) return PENDING;
    const s = state(this);
    assert.equal(s.openedPage, id, `the scenario opened "${s.openedPage}", not "${id}"`);
    const deadline = Date.now() + 15_000;
    while (s.posts.length === 0 && Date.now() < deadline) await sleep(200);
    assert.ok(
      s.posts.length > 0,
      `no POST ${OBSERVATION_WRITE_PATH} was answered within 15 s of ${name} leaving "${id}" — ` +
        'the view was never witnessed (the content viewer posts on an in-app leave only)'
    );
    const post = s.posts.at(-1);
    assert.ok(post, 'no observation answer was captured');
    assert.ok(
      post.status >= 200 && post.status < 300,
      `POST ${OBSERVATION_WRITE_PATH} answered ${post.status}: ${post.body.slice(0, 300)}`
    );
    const ack = JSON.parse(post.body) as ObservationAck;
    const human = this.getHuman(name);
    const selves = [human.humanId, human.agentPubKey].filter(Boolean);
    assert.ok(
      ack.observerCid !== undefined && selves.includes(ack.observerCid),
      `the storage peer recorded observer "${ack.observerCid}" (namespace ` +
        `${ack.observerCidNamespace}); ${name} is ${selves.join(' / ')}`
    );
    s.ack = ack;
  }
);

/**
 * Open the person's own stream page in the lamad bundle and wait for its
 * provenance line — the page prints it whether or not there are entries.
 *
 * Example: And Jessica opens her stream
 */
When('{word} opens her stream', async function (this: E2EWorld, name: string) {
  const device = deviceOf(this, name);
  if (!device) return PENDING;
  await device.navigate('/lamad/me/stream');
  await device.page
    .locator(`[data-testid="${STREAM_TESTID.PROVENANCE}"]`)
    .waitFor({ state: 'visible', timeout: 30_000 });
});

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

/**
 * The newest stream entry for the page shows at least the dwell and depth the
 * reader spent. Read twice: as the page words it (what the person sees) and as
 * the node returns it (the exact numbers the page rounds).
 *
 * Example: Then Jessica's stream lists the "manifesto" page with at least 3000 ms of reading and at least 50 percent read
 */
Then(
  "{word}'s stream lists the {string} page with at least {int} ms of reading and at least {int} percent read",
  async function (this: E2EWorld, name: string, id: string, minDwellMs: number, minDepth: number) {
    const device = deviceOf(this, name);
    if (!device) return PENDING;

    const href = `/epr/${encodeURIComponent(id)}`;
    const entry = device.page
      .locator(`[data-testid="${STREAM_TESTID.ENTRY}"]:has(a[href="${href}"])`)
      .first();
    await entry.waitFor({ state: 'visible', timeout: 15_000 });
    const text = await entry.innerText();
    const shown = readEntryMeasures(text);
    assert.ok(
      shown.dwellMs !== null && shown.dwellMs >= minDwellMs,
      `the stream entry for "${id}" shows dwell ${shown.dwellMs} ms (text "${text}"); expected >= ${minDwellMs}`
    );
    assert.ok(
      shown.depthPct !== null && shown.depthPct >= minDepth,
      `the stream entry for "${id}" shows depth ${shown.depthPct}% (text "${text}"); expected >= ${minDepth}%`
    );

    // The node's exact numbers for the same (newest) entry.
    const stream = await fetchStream(this, name);
    const own = stream.entries.find(e => e.subjectCid === id);
    assert.ok(own, `GET ${OBSERVATION_STREAM_PATH} as ${name} has no entry for "${id}"`);
    assert.ok(
      own.dwellMs >= minDwellMs && own.scrollDepthPct >= minDepth,
      `the node's newest entry for "${id}" reads dwell ${own.dwellMs} ms, depth ` +
        `${own.scrollDepthPct}%; expected >= ${minDwellMs} ms and >= ${minDepth}%`
    );
  }
);

/**
 * The recipe CID the page prints is the CID the node reports for this
 * person's stream — the printed provenance is the real one, not a label.
 *
 * Example: And the recipe address printed on Jessica's stream page matches the one her storage peer returns when she asks it for her stream through its web API, independently of the page
 */
Then(
  "the recipe address printed on {word}'s stream page matches the one her storage peer returns when she asks it for her stream through its web API, independently of the page",
  async function (this: E2EWorld, name: string) {
    const device = deviceOf(this, name);
    if (!device) return PENDING;
    const printed = (
      (await device.page
        .locator(`[data-testid="${STREAM_TESTID.PROVENANCE}"] code`)
        .first()
        .textContent()) ?? ''
    ).trim();
    assert.ok(printed.length > 0, 'the stream page printed no recipe address');
    const stream = await fetchStream(this, name);
    assert.equal(
      printed,
      stream.recipe.cid,
      `the page prints recipe "${printed}" but the node renders ${name}'s stream through ` +
        `"${stream.recipe.cid}" (${stream.recipe.name})`
    );
  }
);

/**
 * Another person's stream, under their own sign-in, carries no entry for the
 * page — one storage peer holding both accounts never mixes them.
 *
 * Example: And James's stream, fetched with his own sign-in, has no entry for the "manifesto" page
 */
Then(
  "{word}'s stream, fetched with his own sign-in, has no entry for the {string} page",
  async function (this: E2EWorld, name: string, id: string) {
    const stream = await fetchStream(this, name);
    const leaked = stream.entries.filter(e => e.subjectCid === id);
    assert.equal(
      leaked.length,
      0,
      `${name}'s own stream carries ${leaked.length} entr(ies) for "${id}" that ${name} never read`
    );
  }
);

/**
 * The storage peer counted the note as held back (suppressed) at least `n`
 * more times since the baseline, and has announced none of this kind — ever,
 * not just since the baseline.
 *
 * Example: And the storage peer behind doorway "alpha" has withheld at least 1 more "lamad:content-viewed" note from other peers and told other peers about none
 */
Then(
  'the storage peer behind doorway {string} has withheld at least {int} more {string} note(s) from other peers and told other peers about none',
  async function (this: E2EWorld, doorwayId: string, n: number, kind: string) {
    const base = state(this).metricsBase;
    assert.ok(base, 'no baseline was written down before the page was opened');
    assert.equal(base.kind, kind, `the baseline was taken for "${base.kind}", not "${kind}"`);
    const storageUrl = requireFixturePrimaryStorageUrl(loadHouseholdMeshFixture(), doorwayId);
    assert.equal(
      storageUrl,
      base.storageUrl,
      'the baseline was read from a different storage peer'
    );
    const text = await scrapeStorage(storageUrl);
    const suppressed = counterForKind(text, SUPPRESSED_SERIES, kind);
    const announced = counterForKind(text, ANNOUNCED_SERIES, kind);
    assert.ok(
      suppressed - base.suppressed >= n,
      `${SUPPRESSED_SERIES}{kind="${kind}"} at ${storageUrl} went ${base.suppressed} -> ` +
        `${suppressed}; expected a rise of at least ${n}`
    );
    assert.equal(
      announced,
      0,
      `${ANNOUNCED_SERIES}{kind="${kind}"} at ${storageUrl} is ${announced} — an agent-private ` +
        'note was announced to other peers'
    );
  }
);
