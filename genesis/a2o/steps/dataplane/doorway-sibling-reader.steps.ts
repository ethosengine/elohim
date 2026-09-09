import { strict as assert } from 'node:assert';
import { execFile, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { readFile, readlink } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { After, Given, When, Then } from '@cucumber/cucumber';

import { chromium, type Browser, type Page, type Response } from 'playwright';

import { resolvePeerUrl } from '../../src/framework/dataplane/surfaces.js';
import { loadHouseholdMeshFixture } from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

const run = promisify(execFile);
const meshScript = fileURLToPath(
  new URL('../../../../app/elohim-app/scripts/hc-mesh.sh', import.meta.url)
);

interface Reader {
  browser: Browser;
  page: Page;
  primary: string;
  sibling: string;
  pid: number;
  ticks: string;
  executable: string;
  paused: boolean;
  documents: number;
  requests: string[];
  errors: string[];
  blobUrls: string[];
  blobDigest: string;
  blobResponses: Response[];
  manifesto?: Response;
  recovered?: Response;
}

const readers = new WeakMap<E2EWorld, Reader>();
const leases = new WeakMap<E2EWorld, ChildProcessWithoutNullStreams>();
const browsers = new WeakMap<E2EWorld, Browser>();

async function acquireLease(world: E2EWorld): Promise<void> {
  const child = spawn(
    '/usr/bin/flock',
    [
      '-n',
      // eslint-disable-next-line sonarjs/publicly-writable-directories -- The owned mesh's shared lock coordinates every fault and staging run.
      `${process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh'}/a2o.lock`,
      '/bin/bash',
      '-c',
      'echo locked; read -r _',
    ],
    { stdio: 'pipe' }
  );
  leases.set(world, child);
  const granted = await Promise.race([
    once(child.stdout, 'data').then(([chunk]) => String(chunk).includes('locked')),
    once(child, 'exit').then(() => false),
  ]);
  assert.ok(granted, 'household is already in use; no fault is allowed');
}

function reader(world: E2EWorld): Reader {
  const value = readers.get(world);
  assert.ok(value, 'open the household reader first');
  return value;
}

async function startTicks(pid: number): Promise<string> {
  const stat = await readFile(`/proc/${pid}/stat`, 'utf8');
  return stat.slice(stat.lastIndexOf(')') + 2).split(' ')[19];
}

async function signalOwnedDoorway(state: Reader, signal: 'SIGSTOP' | 'SIGCONT'): Promise<void> {
  assert.equal(await startTicks(state.pid), state.ticks, 'refuse a recycled doorway PID');
  assert.equal(
    await readlink(`/proc/${state.pid}/exe`),
    state.executable,
    'doorway executable changed'
  );
  process.kill(state.pid, signal);
  state.paused = signal === 'SIGSTOP';
}

async function visibleTitle(page: Page, response: Response, id: string): Promise<void> {
  assert.equal(response.status(), 200, `${id} must be delivered`);
  const row = (await response.json()) as { id?: string; title?: string };
  assert.equal(row.id, id, 'application received the wrong content');
  assert.ok(row.title, 'content must have a title');
  await page.waitForFunction(
    title =>
      document.querySelector('[data-testid="epr-home-title"]')?.textContent?.trim() === title,
    row.title,
    { timeout: 20_000 }
  );
}

Given(
  'an anonymous reader is viewing the landing atom through doorway {string} and has discovered doorway {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, first: string, second: string) {
    assert.equal(
      loadHouseholdMeshFixture().processControl,
      true,
      'owned local process control is required'
    );
    const primary = new URL(resolvePeerUrl(first)).origin;
    const sibling = new URL(resolvePeerUrl(second)).origin;
    assert.notEqual(primary, sibling, 'the sibling must be another entrance');
    await acquireLease(this);
    // Fixture witness only: the later browser must independently fetch these bytes.
    const fixture = await fetch(`${primary}/db/content/manifesto`, {
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(fixture.status, 200);
    const fixtureRow = (await fixture.json()) as { blobCid?: string };
    assert.ok(fixtureRow.blobCid, 'prepared manifesto must carry a content address');
    const blobPath = `/blob/${encodeURIComponent(fixtureRow.blobCid)}`;
    const fixtureBlob = await fetch(`${primary}${blobPath}`, { signal: AbortSignal.timeout(5000) });
    assert.equal(fixtureBlob.status, 200);
    const blobDigest = createHash('sha256')
      .update(Buffer.from(await fixtureBlob.arrayBuffer()))
      .digest('hex');
    // The mesh command owns the lease; use its recorded start-tick guard, not pgrep.
    const { stdout } = await run('bash', [
      '-c',
      'source "$1"; live_recorded_pid doorway a',
      'reader-fault',
      meshScript,
    ]);
    const pid = Number(stdout.trim());
    assert.ok(
      Number.isSafeInteger(pid) && pid > 1 && pid !== process.pid,
      'safe owned doorway PID required'
    );
    const env = (await readFile(`/proc/${pid}/environ`, 'utf8')).split('\0');
    assert.ok(
      env.includes(`DOORWAY_URL=${primary}`),
      'owned doorway must match the visited entrance'
    );
    const browser = await chromium.launch({
      headless: true,
      args: ['--no-sandbox', '--disable-dev-shm-usage'],
    });
    browsers.set(this, browser);
    const page = await browser.newPage();
    const state: Reader = {
      browser,
      page,
      primary,
      sibling,
      pid,
      ticks: await startTicks(pid),
      executable: await readlink(`/proc/${pid}/exe`),
      paused: false,
      documents: 0,
      requests: [],
      errors: [],
      // The prepared contentBody still uses the supported legacy SHA256 wire
      // alias; its metadata carries the equivalent raw CID. Both name these bytes.
      blobUrls: [`${sibling}${blobPath}`, `${sibling}/blob/sha256-${blobDigest}`],
      blobDigest,
      blobResponses: [],
    };
    readers.set(this, state);
    page.on('request', request => {
      state.requests.push(request.url());
      if (request.isNavigationRequest() && request.frame() === page.mainFrame())
        state.documents += 1;
    });
    page.on('pageerror', error => state.errors.push(error.message));
    page.on('response', response => {
      if (state.blobUrls.includes(response.url())) state.blobResponses.push(response);
    });
    const discovery = page.waitForResponse(
      response =>
        response.url() === `${primary}/api/v1/federation/doorways` && response.status() === 200,
      { timeout: 30_000 }
    );
    const landing = page.waitForResponse(`${primary}/db/content/elohim-host-landing`, {
      timeout: 30_000,
    });
    // Settle both observers on failure so teardown never leaves unhandled promises.
    const observed = Promise.all([discovery, landing]);
    observed.catch(() => undefined);
    await page.goto(`${primary}/epr/elohim-host-landing`, { waitUntil: 'load', timeout: 30_000 });
    const [discovered, loaded] = await observed;
    const body = (await discovered.json()) as { doorways?: { url?: string }[] };
    assert.ok(
      body.doorways?.some(peer => peer.url && new URL(peer.url).origin === sibling),
      'the app must discover its real sibling'
    );
    await visibleTitle(page, loaded, 'elohim-host-landing');
    await page.locator('[data-related-id="manifesto"]').waitFor({ state: 'visible' });
    assert.equal(
      state.requests.some(url => new URL(url).pathname === '/db/content/manifesto'),
      false,
      'manifesto must not already be read'
    );
    assert.equal(state.documents, 1, 'one running app document');
  }
);

When("that reader's doorway stops answering", { timeout: 10_000 }, async function (this: E2EWorld) {
  const state = reader(this);
  await signalOwnedDoorway(state, 'SIGSTOP');
  await assert.rejects(
    fetch(`${state.primary}/health`, { signal: AbortSignal.timeout(1500) }),
    'paused doorway must actually stop answering'
  );
});

When(
  "the reader follows the landing's manifesto link without leaving the running app",
  { timeout: 60_000 },
  async function (this: E2EWorld) {
    const state = reader(this);
    const response = state.page.waitForResponse(`${state.sibling}/db/content/manifesto`, {
      timeout: 45_000,
    });
    response.catch(() => undefined);
    await state.page.locator('[data-related-id="manifesto"]').click();
    state.manifesto = await response;
    assert.equal(
      new URL(state.page.url()).origin,
      state.primary,
      'the browser remains in its running app'
    );
    assert.equal(state.documents, 1, 'related link must not reload the dead entrance');
  }
);

Then(
  'the app receives the manifesto from the discovered sibling and displays its title and body',
  { timeout: 45_000 },
  async function (this: E2EWorld) {
    const state = reader(this);
    assert.ok(state.manifesto, 'application sibling response required');
    await visibleTitle(state.page, state.manifesto, 'manifesto');
    await state.page.waitForFunction(
      () => {
        const text = document.querySelector('[data-testid="epr-home-focal"]')?.textContent ?? '';
        return text.includes('Love-Centered') && text.length > 1000;
      },
      undefined,
      { timeout: 35_000 }
    );
    assert.deepEqual(state.errors, [], 'no uncaught browser errors');
    const blob = state.blobResponses.find(response => response.status() === 200);
    assert.ok(blob, 'the running app must fetch the manifesto bytes through the sibling');
    assert.equal(
      createHash('sha256')
        .update(await blob.body())
        .digest('hex'),
      state.blobDigest,
      'sibling bytes must equal the prepared manifesto'
    );
  }
);

Then('the sibling content request carries no session credentials', async function (this: E2EWorld) {
  const state = reader(this);
  assert.ok(state.manifesto);
  for (const response of [state.manifesto, ...state.blobResponses]) {
    const headers = await response.request().allHeaders();
    for (const name of [
      'authorization',
      'proxy-authorization',
      'cookie',
      'x-api-key',
      'x-session-token',
    ]) {
      assert.equal(headers[name], undefined, `${name} must not cross to the discovered sibling`);
    }
  }
});

When(
  "the original doorway recovers and the app's primary retry interval has elapsed",
  { timeout: 45_000 },
  async function (this: E2EWorld) {
    const state = reader(this);
    await signalOwnedDoorway(state, 'SIGCONT');
    const response = await fetch(`${state.primary}/health`, { signal: AbortSignal.timeout(5000) });
    assert.equal(response.status, 200, 'restored doorway must serve');
    await state.page.waitForTimeout(31_000);
  }
);

When(
  'the reader returns to the landing atom within the running app',
  { timeout: 35_000 },
  async function (this: E2EWorld) {
    const state = reader(this);
    const response = state.page.waitForResponse(`${state.primary}/db/content/elohim-host-landing`, {
      timeout: 30_000,
    });
    response.catch(() => undefined);
    await state.page.locator('[data-testid="epr-home-arrival"]').click();
    state.recovered = await response;
    assert.equal(state.documents, 1, 'recovery must stay inside the running app');
  }
);

Then(
  'the app receives the landing atom through the recovered original doorway and displays its title',
  { timeout: 25_000 },
  async function (this: E2EWorld) {
    const state = reader(this);
    assert.ok(state.recovered);
    await visibleTitle(state.page, state.recovered, 'elohim-host-landing');
  }
);

After({ tags: '@concern:doorway-failover', timeout: 20_000 }, async function (this: E2EWorld) {
  const state = readers.get(this);
  try {
    if (!state) return;
    // Recovery has priority over screenshots and reporting, including assertion failures.
    if (state.paused) await signalOwnedDoorway(state, 'SIGCONT');
    const health = await fetch(`${state.primary}/health`, { signal: AbortSignal.timeout(5000) });
    assert.equal(health.status, 200, 'fault teardown must restore the original doorway');
    this.attach(
      JSON.stringify({
        requests: state.requests,
        errors: state.errors,
        documents: state.documents,
        restored: true,
      }),
      'application/json'
    );
    this.attach(await state.page.screenshot(), 'image/png');
  } finally {
    readers.delete(this);
    leases.get(this)?.stdin.end();
    leases.delete(this);
    await browsers.get(this)?.close();
    browsers.delete(this);
  }
});
