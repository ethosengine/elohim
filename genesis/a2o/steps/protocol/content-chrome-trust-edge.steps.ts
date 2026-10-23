/** Browser evidence for the content/chrome boundary; fixtures and entry visits reuse Station 5. */
import { strict as assert } from 'node:assert';
import { appendFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { join } from 'node:path';

import { Before, Given, Then, When } from '@cucumber/cucumber';

import {
  buildAppsWorker,
  routeAppsWorker,
  shellVisitSessions,
  visitShellInBrowser,
  chromeEntryUrl,
  content,
  island,
  type Content,
  openInfo,
  card,
  checkLink,
} from '../../scripts/lib/chrome-shell.js';
import {
  omni,
  chip,
  mark,
  panel,
  frameSelector,
  appFrameSelector,
} from '../../scripts/lib/chrome-testids.js';
import { resolvePeerUrl } from '../../src/framework/dataplane/surfaces.js';
import { BrowserDevice } from '../../src/framework/devices/browser-device.js';
import {
  loadHouseholdMeshFixture,
  requireFixtureStoragePeer,
} from '../../src/framework/fixtures/household-mesh.js';
import { getFixture } from '../../src/framework/fixtures/humans.js';
import { E2EWorld } from '../../src/framework/world.js';
import {
  pollUntil,
  stageBundle,
  stageBundleThroughStorage,
} from '../dataplane/epr-app-deliverability.helpers.js';
import {
  app,
  authorOwnedEprRecord,
  buildNextFixture,
  contentAppWorlds,
} from '../dataplane/epr-app-deliverability.steps.js';

import type { Frame, Page } from 'playwright';

interface State {
  page?: Page;
  pages: Page[];
  requests: Map<Page, number[]>;
  holder?: { text: string; dataHolders: string | null; islandCount?: number };
  access?: string;
  seeded?: string;
  outside?: string;
  raw?: string;
  auth?: { origin: string; token: string };
  contents: Content[];
  ownOrigin?: string;
  recordDoorway?: string;
  reached: string[];
  blocked: string[];
  reports: { url: string; body: string; status: number }[];
}
const states = new WeakMap<E2EWorld, State>();
const witnessAttribute = 'data-witness';
async function peerWitness(url: string, timeoutMs: number): Promise<string> {
  try {
    const response = await fetch(url, { signal: AbortSignal.timeout(timeoutMs) });
    if (response.status === 404) return '404 (not yet held)';
    if (response.ok) {
      const facing = (await response.json()) as { witness?: string };
      return facing.witness ?? 'missing witness';
    }
    return `HTTP ${response.status}`;
  } catch (error) {
    return String(error);
  }
}
function state(world: E2EWorld): State {
  const s = states.get(world);
  assert.ok(s);
  return s;
}
function page(world: E2EWorld): Page {
  const p = state(world).page;
  assert.ok(p, 'No retained entry visit');
  return p;
}
async function holderReading(p: Page): Promise<NonNullable<State['holder']>> {
  const row = p.locator('[data-page-info-row="holders"]');
  return {
    text: await row.innerText(),
    dataHolders: await row.getAttribute('data-holders'),
    islandCount: (await content(p)).holders?.holders,
  };
}
async function frame(p: Page): Promise<Frame> {
  const handle = await p.locator(frameSelector).elementHandle();
  const f = await handle?.contentFrame();
  assert.ok(f, 'Content frame did not load');
  return f;
}
Before({ tags: '@concern:content-chrome-trust-edge' }, async function (this: E2EWorld) {
  const appsWorker = await buildAppsWorker();
  // These bundles are content apps, served at /apps/<slug>/ on a content origin: relative base.
  contentAppWorlds.add(this);
  const s: State = {
    pages: [],
    requests: new Map(),
    contents: [],
    reached: [],
    blocked: [],
    reports: [],
  };
  states.set(this, s);
  shellVisitSessions.set(this, {
    requireOriginPair: true,
    retain: (browser, p) => {
      s.page = p;
      s.pages.push(p);
      this.onCleanup(async () => {
        await browser.close();
      });
    },
    prepare: async p => {
      await routeAppsWorker(p.context(), appsWorker);
      if (s.ownOrigin) {
        const port = new URL(s.ownOrigin).port;
        const allowed = new Set([`http://localhost:${port}`, `http://127.0.0.1:${port}`]);
        await p.route('**/*', async route => {
          const url = route.request().url();
          if (allowed.has(new URL(url).origin)) await route.fallback();
          else {
            s.blocked.push(url);
            await route.abort('blockedbyclient');
          }
        });
        p.on('response', response => s.reached.push(response.url()));
      }
      p.on('response', response => {
        if (
          new URL(response.url()).pathname.startsWith('/_elohim/csp-report/') &&
          response.request().method() === 'POST'
        )
          s.reports.push({
            url: response.url(),
            body: response.request().postData() ?? '',
            status: response.status(),
          });
      });
      const requests: number[] = [];
      s.requests.set(p, requests);
      p.on('request', r => {
        if (r.isNavigationRequest() && r.frame() !== p.mainFrame()) requests.push(Date.now());
      });
      await p.addInitScript(() => {
        const times: { omni?: number; frame?: number } = {};
        Object.assign(globalThis, { __chromeTimes: times });
        new MutationObserver(records => {
          for (const record of records)
            for (const node of record.addedNodes) {
              if (!(node instanceof Element)) continue;
              if (node.matches('#elohim-omni') || node.querySelector('#elohim-omni'))
                times.omni ??= performance.timeOrigin + performance.now();
              if (
                node.matches('iframe[data-testid="content-app-frame"]') ||
                node.querySelector('iframe[data-testid="content-app-frame"]')
              )
                times.frame ??= performance.timeOrigin + performance.now();
            }
        }).observe(document, { childList: true, subtree: true });
      });
      if (s.auth) {
        await p.context().addCookies([
          {
            name: 'elohim_session',
            value: s.auth.token,
            url: s.auth.origin,
            httpOnly: true,
            sameSite: 'Lax',
          },
        ]);
        await p.addInitScript(({ origin, token }) => {
          if (location.origin === origin) localStorage.setItem('elohim-auth-token', token);
        }, s.auth);
      }
    },
  });
});

function variant(world: E2EWorld, kind: 'popup' | 'parent', fresh = false): void {
  buildNextFixture.call(world, false, fresh);
  const bundle = app(world).bundle;
  assert.ok(bundle);
  const script =
    kind === 'popup'
      ? `const popup = window.open('about:blank'); root.dataset.popupResult = popup ? 'opened' : 'refused';`
      : `try { window.parent.document.querySelector('[data-page-info-row="holders"]').textContent = 'forged'; root.dataset.parentResult = 'reachable'; } catch (error) { root.dataset.parentResult = error.name; }
        const chromeOrigin = location.ancestorOrigins?.[0] || new URL(document.referrer).origin;
        const chromeUrl = chromeOrigin + '/api/v1/epr/' + ${JSON.stringify(encodeURIComponent(app(world).slug))} + '/chrome';
        void fetch(chromeUrl, { credentials: 'include' }).catch(() => {});
        const probe = document.createElement('img');
        probe.src = chromeUrl;
        document.body.appendChild(probe);`;
  appendFileSync(join(bundle.dir, bundle.entryScript), `\n${script}\n`);
}
Given(
  'a coherent EPR app bundle this run just built, which tries to open a new window when it starts',
  function (this: E2EWorld) {
    variant(this, 'popup');
  }
);
Given(
  'a second coherent EPR app bundle this run just built, which also tries to open a new window when it starts',
  function (this: E2EWorld) {
    variant(this, 'popup', true);
  }
);
Given(
  'a coherent EPR app bundle this run just built, which reaches for the page around it when it starts',
  function (this: E2EWorld) {
    variant(this, 'parent');
  }
);
Given(
  'an EPR record this run owns for it, declaring that it needs to open windows',
  { timeout: 720_000 },
  async function (this: E2EWorld) {
    await authorOwnedEprRecord(this, 'html5-app', ['popups']);
  }
);
Given(
  'an EPR record this run owns for it, declaring nothing it needs',
  { timeout: 720_000 },
  async function (this: E2EWorld) {
    await authorOwnedEprRecord(this, 'html5-app', []);
  }
);
Given(
  /^doorway "([^"]+)" holds (?:the|that) bundle and knows its record as a content app \(format html5-app\)$/,
  { timeout: 360_000 },
  async function (this: E2EWorld, peer: string) {
    const record = app(this);
    assert.ok(record.bundle);
    const result = await stageBundle({
      bundle: record.bundle,
      slug: record.slug,
      doorwayUrl: resolvePeerUrl(peer),
      declare: record.declaredThrough.length === 0,
      stepTimeoutMs: 300_000,
    });
    assert.equal(result.code, 0, result.output);
    record.blobHash = result.blobHash;
    record.declaredThrough.push(peer);
    assert.notEqual(
      await pollUntil(async () => {
        const response = await fetch(`${resolvePeerUrl(peer)}/db/content/${record.slug}`);
        if (!response.ok) return false;
        const row = (await response.json()) as { contentFormat?: string };
        return row.contentFormat === 'html5-app';
      }, 45_000),
      null,
      `${peer} has not projected the html5-app record`
    );
  }
);
async function visit(world: E2EWorld, url: string): Promise<void> {
  const result = await visitShellInBrowser(url, 30_000, undefined, shellVisitSessions.get(world));
  assert.equal(result.mainStatus, 200, result.pageErrors.join('\n'));
  assert.ok(result.frameBootstrapReady, result.pageErrors.join('\n'));
}
When(
  "a visitor opens that app's entry address on federated peer {string} as a page in a browser",
  { timeout: 120_000 },
  async function (this: E2EWorld, peer: string) {
    await visit(this, chromeEntryUrl(app(this).slug, peer));
  }
);
Then(
  "the chip is on the page before the content's frame begins to load",
  async function (this: E2EWorld) {
    const p = page(this);
    const times = await p.evaluate(
      () =>
        (globalThis as unknown as { __chromeTimes: { omni?: number; frame?: number } })
          .__chromeTimes
    );
    assert.ok(times.omni && times.frame, JSON.stringify(times));
    assert.ok(times.omni <= times.frame, JSON.stringify(times));
    const requests = state(this).requests.get(p) ?? [];
    assert.ok(requests.length, 'No frame request observed');
    assert.ok(times.omni <= requests[0] + 1, 'Frame request preceded omni mount');
    assert.ok(await p.locator(chip).isVisible());
  }
);
Then(
  String.raw`the content runs in a frame whose files come from doorway {string}'s apps origin \(apps. in front of its public name)`,
  async function (this: E2EWorld, _peer: string) {
    const p = page(this);
    const c = await content(p);
    const origin = new URL(c.frame.src, p.url());
    assert.equal(origin.hostname, `apps.${new URL(p.url()).hostname}`);
    assert.equal(new URL((await frame(p)).url()).origin, origin.origin);
  }
);
When('the visitor opens the chip', async function (this: E2EWorld) {
  await page(this).locator(chip).click();
});
Then(
  'the omnibar is open across the top of the page, with the trust mark on it',
  async function (this: E2EWorld) {
    const p = page(this);
    assert.equal(await p.locator(omni).getAttribute('data-omni-state'), 'expanded');
    assert.ok(await p.locator(mark).isVisible());
    const box = await p.locator('.omni-toolbar').boundingBox();
    assert.ok(box && box.y <= 1 && box.x <= 1);
  }
);
When('the visitor opens the trust mark', async function (this: E2EWorld) {
  await page(this).locator(mark).click();
});
When(
  'the visitor opens the chip, then the trust mark on the omnibar it opens',
  async function (this: E2EWorld) {
    await openInfo(page(this));
  }
);
Then(/^page info lists the rows (.+)$/, async function (this: E2EWorld, text: string) {
  const expected = [...text.matchAll(/"([^"]+)"/g)].map(m => m[1].toLowerCase());
  const actual = await page(this)
    .locator('[data-page-info-row]')
    .evaluateAll(els => els.map(el => el.getAttribute('data-page-info-row')));
  actual.sort((a, b) => (a ?? '').localeCompare(b ?? ''));
  expected.sort((a, b) => a.localeCompare(b));
  assert.deepEqual(actual, expected);
});
Then(
  'page info does not list "Yours", because the visitor is not signed in',
  async function (this: E2EWorld) {
    assert.equal(await page(this).locator('[data-page-info-row="yours"]').count(), 0);
  }
);
Then(
  "each row opens a card that ends in a link to this app's EPR page",
  async function (this: E2EWorld) {
    for (const row of ['source', 'delivery', 'holders', 'steward', 'claims', 'permissions'])
      await checkLink(page(this), row);
  }
);
Then('the "Yours" card ends in a link to this app\'s EPR page', async function (this: E2EWorld) {
  await checkLink(page(this), 'yours');
});
When('the visitor presses Escape', async function (this: E2EWorld) {
  await page(this).keyboard.press('Escape');
});
Then(
  'page info closes and the trust mark has the keyboard focus again',
  async function (this: E2EWorld) {
    assert.equal(await page(this).locator(panel).isVisible(), false);
    assert.equal(
      await page(this).evaluate(() =>
        document.activeElement?.matches('[data-omni-action="page-info"]')
      ),
      true
    );
  }
);
When(
  'the visitor opens page info and reads the holder count in its Holders row',
  async function (this: E2EWorld) {
    await openInfo(page(this));
    state(this).holder = await holderReading(page(this));
    assert.ok(state(this).holder!.text.trim(), 'Holders row has no reading');
  }
);
When(
  'the content in the frame tries to read the page around it and to change the holder count the chrome shows',
  async function (this: E2EWorld) {
    state(this).access = await (
      await frame(page(this))
    ).evaluate(() => {
      try {
        const parent = window.parent.document;
        const row = parent.querySelector('[data-page-info-row="holders"]');
        if (row) row.textContent = 'forged';
        return 'reachable';
      } catch (error) {
        return (error as Error).name;
      }
    });
  }
);
Then('the browser refuses the content access to the page around it', function (this: E2EWorld) {
  assert.equal(state(this).access, 'SecurityError');
});
Then(
  "page info's Holders row still shows the holder count the visitor read before",
  async function (this: E2EWorld) {
    assert.ok(state(this).holder, 'No baseline Holders reading');
    assert.deepEqual(await holderReading(page(this)), state(this).holder);
  }
);
Then(
  "the content could not read the sign-in kept by doorway {string}'s own origin",
  async function (this: E2EWorld, _peer: string) {
    const result = await (
      await frame(page(this))
    ).evaluate(() => ({
      token: localStorage.getItem('elohim-auth-token'),
      cookie: document.cookie,
    }));
    assert.equal(result.token, null);
    assert.ok(!result.cookie.includes('elohim_session'));
  }
);
Then(
  'the content was shown without waiting for its record to be witnessed',
  async function (this: E2EWorld) {
    assert.equal(
      await (await frame(page(this))).locator('app-root').getAttribute('data-app-ready'),
      'true'
    );
  }
);
Then(
  'the chip agrees with the network: green only if a peer other than the author already holds the record, and otherwise amber, turning green within 90 seconds once one does',
  { timeout: 110_000 },
  async function (this: E2EWorld) {
    const p = page(this);
    const first = await content(p);
    const firstMark = await p.locator(mark).getAttribute(witnessAttribute);
    assert.ok(first.witness === 'amber' || first.witness === 'green');
    this.attach(`witness-first-look: ${first.witness}`, 'text/plain');
    assert.equal(firstMark, first.witness, 'Island and trust mark disagree at first look');

    const jessica = requireFixtureStoragePeer(loadHouseholdMeshFixture(), 'jessica');
    const url = `${jessica.url}/api/v1/epr/${encodeURIComponent(first.recordId)}/chrome`;
    const deadline = Date.now() + (first.witness === 'green' ? 10_000 : 90_000);
    let chromeReading = `island=${first.witness}, mark=${firstMark}`;
    let jessicaReading = 'not read';
    while (Date.now() < deadline) {
      let chromeGreen = first.witness === 'green';
      if (first.witness === 'amber') {
        // Reopening refreshes the facing once the chrome and doorway memos expire.
        const sourceText = await card(p, 'source');
        const sourceWitness = await p
          .locator('[data-page-info-card="source"] [data-witness]')
          .getAttribute(witnessAttribute);
        const markWitness = await p.locator(mark).getAttribute(witnessAttribute);
        chromeReading = `source=${sourceWitness}, mark=${markWitness}`;
        assert.ok(sourceWitness === 'amber' || sourceWitness === 'green', chromeReading);
        assert.ok(markWitness === 'amber' || markWitness === 'green', chromeReading);
        if (sourceWitness === 'amber' || markWitness === 'amber')
          assert.match(sourceText, /await|not yet|pending/i);
        chromeGreen = sourceWitness === 'green' && markWitness === 'green';
      }
      const remaining = deadline - Date.now();
      if (remaining <= 0) break;
      jessicaReading = await peerWitness(url, Math.min(5_000, remaining));
      if (chromeGreen && jessicaReading === 'green' && Date.now() < deadline) return;
      await new Promise(resolve =>
        setTimeout(resolve, Math.min(1_000, Math.max(0, deadline - Date.now())))
      );
    }
    assert.fail(`Witness agreement timed out: chrome ${chromeReading}; jessica ${jessicaReading}`);
  }
);
Then(
  /^the chip shows (amber|green), and page info's Source card says the record (awaits the network's witness|is witnessed)$/,
  async function (this: E2EWorld, witness: string, _description: string) {
    const p = page(this);
    assert.equal((await content(p)).witness, witness);
    assert.equal(await p.locator(mark).getAttribute(witnessAttribute), witness);
    assert.match(
      await card(p, 'source'),
      witness === 'amber' ? /await|not yet|pending/i : /witnessed/i
    );
  }
);
Given(
  'a content app the household seeded when it was set up, whose record the network has since witnessed',
  async function (this: E2EWorld) {
    const id = process.env['E2E_WITNESSED_CONTENT_APP'] ?? 'evolution-of-trust';
    const response = await fetch(`${resolvePeerUrl('alpha-A')}/db/content/${id}`);
    assert.ok(response.ok, `Seeded content app ${id} missing; run household prologue`);
    const row = (await response.json()) as { contentFormat: string; dhtAnchorHash?: string };
    assert.equal(row.contentFormat, 'html5-app');
    assert.ok(row.dhtAnchorHash, `${id} has no DHT witness`);
    state(this).seeded = id;
  }
);
When(
  "a visitor opens that seeded app's entry address on federated peer {string} as a page in a browser",
  { timeout: 120_000 },
  async function (this: E2EWorld, peer: string) {
    const id = state(this).seeded;
    assert.ok(id);
    const result = await visitShellInBrowser(
      chromeEntryUrl(id, peer),
      30_000,
      undefined,
      shellVisitSessions.get(this)
    );
    assert.equal(result.mainStatus, 200);
    assert.equal(result.content?.isolation.boundary, 'origin-pair', result.pageErrors.join('\n'));
    assert.ok(await (await frame(page(this))).locator('body').isVisible());
  }
);
Then(
  /^the app inside the frame (opened its window|was refused its window)$/,
  async function (this: E2EWorld, result: string) {
    assert.equal(
      await (await frame(page(this))).locator('app-root').getAttribute('data-popup-result'),
      result.startsWith('opened') ? 'opened' : 'refused'
    );
    assert.equal(page(this).context().pages().length, result.startsWith('opened') ? 2 : 1);
  }
);
Then(
  /^page info's Permissions card says the app (may open windows, as its record declares|may not open windows, because its record does not declare it)$/,
  async function (this: E2EWorld, wording: string) {
    const allowed = !wording.startsWith('may not');
    const p = page(this);
    assert.equal((await content(p)).capabilities.includes('popups'), allowed);
    const sandbox = await p.locator(frameSelector).getAttribute('sandbox');
    assert.equal(sandbox?.split(' ').includes('allow-popups'), allowed);
    await card(p, 'permissions');
    const text = await p.locator('[data-page-info-cap="popups"]').innerText();
    assert.match(text, /Open windows/i);
    assert.match(text, allowed ? /Allowed/i : /Not declared|Not allowed|Blocked/i);
  }
);
Given(
  'the visitor is signed in as household persona {string} on doorway {string}',
  async function (this: E2EWorld, name: string, peer: string) {
    const fixture = getFixture(name);
    const origin = resolvePeerUrl(peer);
    const device = new BrowserDevice(`${name}-chrome`, origin);
    const auth = await device.login(fixture.credentials);
    assert.ok(auth.token);
    state(this).auth = {
      origin: new URL(chromeEntryUrl(app(this).slug, peer)).origin,
      token: auth.token,
    };
  }
);
Given(
  "a page on another website, served from an address outside the household's public names, that frames the raw entry file's address",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    await visit(this, chromeEntryUrl(app(this).slug, 'alpha-A'));
    const c = await content(page(this));
    const src = new URL(c.frame.src, page(this).url());
    const raw = new URL(`/apps/${c.recordId}/index.html`, src.origin).href;
    state(this).raw = raw;
    const server = createServer((_req, res) => {
      res.setHeader('Content-Type', 'text/html');
      res.end(
        `<html><body><iframe title="Third-party embed" src="${raw.replaceAll('&', '&amp;').replaceAll('"', '&quot;')}"></iframe></body></html>`
      );
    });
    await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
    this.onCleanup(async () => {
      server.closeAllConnections();
      await new Promise<void>((resolve, reject) =>
        server.close(error => (error ? reject(error) : resolve()))
      );
    });
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    state(this).outside = `http://127.0.0.1:${address.port}`;
    assert.notEqual(new URL(state(this).outside!).origin, new URL(page(this).url()).origin);
  }
);
Given(
  "the protocol's own app on doorway {string}, whose own record declares that it presents content",
  async function (this: E2EWorld, peer: string) {
    // The wrapper declaration is read through the content-chrome facing, the same reading the
    // chrome and the doorway act on, rather than by re-parsing the record body.
    const response = await fetch(`${resolvePeerUrl(peer)}/api/v1/epr/elohim-host-landing/chrome`);
    assert.ok(
      response.ok,
      `Landing app's content-chrome facing is unavailable (HTTP ${response.status})`
    );
    const facing = (await response.json()) as { format?: string; app?: { presents?: boolean } };
    assert.equal(
      facing.app?.presents,
      true,
      `Landing app's record (format ${facing.format ?? 'unknown'}) does not declare that it presents content`
    );
  }
);
When(
  "a visitor opens the raw entry file's address on doorway {string}'s apps origin as a page",
  async function (this: E2EWorld, _peer: string) {
    assert.ok(state(this).raw);
    await page(this).goto(state(this).raw!, { waitUntil: 'load' });
  }
);
Then(
  "the visitor lands on this app's entry address on doorway {string}, inside the chrome",
  async function (this: E2EWorld, peer: string) {
    assert.equal(page(this).url(), chromeEntryUrl(app(this).slug, peer));
    assert.ok(await page(this).locator(chip).isVisible());
    assert.ok(await page(this).locator(frameSelector).isVisible());
  }
);
When('a visitor opens that page on the other website', async function (this: E2EWorld) {
  assert.ok(state(this).outside);
  await page(this).goto(state(this).outside!, { waitUntil: 'load' });
});
Then(
  'that frame shows the chrome with the app inside it, not the app alone',
  async function (this: E2EWorld) {
    const outer = page(this).frameLocator('iframe');
    await outer.locator(chip).waitFor({ state: 'visible' });
    await outer.locator(frameSelector).waitFor({ state: 'visible' });
    await outer
      .frameLocator(frameSelector)
      .locator('[data-app-ready="true"]')
      .waitFor({ state: 'visible' });
  }
);
When(
  "a visitor opens this app's record page on doorway {string}",
  async function (this: E2EWorld, peer: string) {
    state(this).recordDoorway = resolvePeerUrl(peer);
    await page(this).goto(
      new URL(`/epr/${app(this).slug}`, chromeEntryUrl(app(this).slug, peer)).href,
      {
        waitUntil: 'load',
      }
    );
    await page(this).locator('[data-testid="epr-home"]').waitFor({ state: 'visible' });
  }
);
Then(
  "the app appears inside the record page's own frame, served from the apps origin, with no second chrome around it",
  async function (this: E2EWorld) {
    const p = page(this);
    const origin = new URL(p.url());
    // APIRequestContext uses Node DNS; preserve the browser's paired Host on loopback.
    const doorway = state(this).recordDoorway;
    assert.ok(doorway, 'No record-page doorway visit recorded');
    const discoveryUrl = new URL('/.well-known/elohim-auth', doorway);
    if (discoveryUrl.hostname === 'localhost') discoveryUrl.hostname = '127.0.0.1';
    const discovery = await p.request.get(discoveryUrl.href, { headers: { Host: origin.host } });
    assert.ok(discovery.ok(), `Auth discovery unavailable (HTTP ${discovery.status()})`);
    assert.equal(((await discovery.json()) as { apps?: boolean }).apps, true);
    origin.hostname = `apps.${origin.hostname}`;
    const appFrame = p.locator('[data-testid="epr-home-focal"]').locator(appFrameSelector);
    await appFrame.waitFor({ state: 'visible' });
    const f = appFrame.contentFrame();
    await f.locator('[data-app-ready="true"]').waitFor({ state: 'visible' });
    assert.equal(await appFrame.getAttribute('data-frame-origin'), origin.origin);
    assert.equal(await f.locator(omni).count(), 0);
  }
);
Then(
  "the record page shows, around the frame, the app's title, who holds it, how it is governed and where it came from",
  async function (this: E2EWorld) {
    const p = page(this);
    const title = p.locator('[data-testid="epr-home-title"]');
    await title.waitFor({ state: 'visible' });
    assert.ok(
      (await title.innerText()).includes(`EPR app deliverability fixture (${app(this).slug})`),
      'Record page does not show the fixture record title'
    );
    for (const leg of ['holds', 'governed', 'from']) {
      await p.locator(`[data-testid="epr-home-leg-${leg}"]`).waitFor({ state: 'visible' });
    }
  }
);
When(
  "another visitor opens this app's entry address on federated peer {string} as a page in a browser",
  { timeout: 120_000 },
  async function (this: E2EWorld, peer: string) {
    state(this).contents.push(await content(page(this)));
    await visit(this, chromeEntryUrl(app(this).slug, peer));
    state(this).contents.push(await content(page(this)));
  }
);
Then(
  'both chromes name the same record, the same content address and the same witness state',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const [a, b] = state(this).contents;
    assert.ok(a && b);
    const invariant = (c: Content) => ({
      recordId: c.recordId,
      contentAddress: c.contentAddress,
    });
    assert.deepEqual(invariant(a), invariant(b));
    // A record this run just published reaches each doorway's peer on its own schedule, so the
    // two islands can be taken on either side of the anchor arriving. The chrome re-reads the
    // witness whenever page info opens (its facing memo lasts 30 s); the readings must agree
    // once both have re-read, and a lasting difference still fails.
    const [pa, pb] = state(this).pages.slice(-2);
    assert.ok(pa && pb);
    const witnessOf = async (p: Page) => {
      await card(p, 'source');
      return p
        .locator('[data-page-info-card="source"] [data-witness]')
        .first()
        .getAttribute(witnessAttribute);
    };
    const deadline = Date.now() + 90_000;
    let seen: [string | null, string | null] = [a.witness, b.witness];
    while (seen[0] !== seen[1] && Date.now() < deadline) {
      await pa.waitForTimeout(5_000);
      seen = [await witnessOf(pa), await witnessOf(pb)];
    }
    assert.equal(
      seen[0],
      seen[1],
      `witness differs between doorways after 90 s: ${seen.join(' vs ')}`
    );
  }
);
Then("both chromes' Holders and Claims cards read the same", async function (this: E2EWorld) {
  const [a, b] = state(this).pages.slice(-2);
  assert.ok(a && b);
  for (const row of ['holders', 'claims']) assert.equal(await card(a, row), await card(b, row));
});
Then(
  "each chrome's Delivery card names the doorway the visitor came through and the peer behind it, and the two name different ones",
  async function (this: E2EWorld) {
    const [a, b] = state(this).contents;
    assert.ok(a && b);
    assert.notEqual(
      a.delivery.path.find(h => h.role === 'doorway')?.id,
      b.delivery.path.find(h => h.role === 'doorway')?.id
    );
    assert.notEqual(
      a.delivery.path.find(h => h.role === 'holder')?.peer,
      b.delivery.path.find(h => h.role === 'holder')?.peer
    );
    for (const p of state(this).pages.slice(-2)) {
      const c = await content(p);
      const text = await card(p, 'delivery');
      const door = c.delivery.path.find(h => h.role === 'doorway');
      const holder = c.delivery.path.find(h => h.role === 'holder');
      assert.ok(door && holder, 'Delivery must name both hops');
      assert.ok(text.includes(door.id));
      assert.ok(text.includes(holder.peer));
    }
  }
);
Then(
  "within 30 seconds at least one of the household's storage peers that holds the app has recorded a signal against its content address naming what it reached for, relayed by doorway {string}",
  { timeout: 40_000 },
  async function (this: E2EWorld, peer: string) {
    const c = await content(page(this));
    const chromeOrigin = new URL(page(this).url()).origin;
    const contentOrigin = new URL((await frame(page(this))).url()).origin;
    const coherence = await fetch(`${resolvePeerUrl(peer)}/api/v1/federation/coherence`, {
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(coherence.status, 200, `${peer}: federation coherence unavailable`);
    const { doorwayId } = (await coherence.json()) as { doorwayId?: string };
    assert.ok(doorwayId, `${peer}: federation coherence exposes no doorwayId`);
    const peers = Object.values(loadHouseholdMeshFixture().storagePeers ?? {})
      .map(p => p.url)
      .filter((url): url is string => !!url);
    assert.ok(peers.length, 'No household storage peers declared');
    let last = '';
    const deadline = Date.now() + 30_000;
    const found = await pollUntil(async () => {
      for (const url of peers) {
        if (Date.now() >= deadline) return false;
        try {
          const held = await fetch(`${url}/blob/${encodeURIComponent(c.contentAddress)}`, {
            headers: { Range: 'bytes=0-0' },
            signal: AbortSignal.timeout(Math.max(1, Math.min(1500, deadline - Date.now()))),
          });
          await held.body?.cancel();
          if (!held.ok) {
            last = `${url}: bundle not held (${held.status})`;
            continue;
          }
          const response = await fetch(
            `${url}/api/v1/observations/by-subject?subjectCid=${encodeURIComponent(c.contentAddress)}&kind=elohim:undeclared-behaviour`,
            { signal: AbortSignal.timeout(Math.max(1, Math.min(1500, deadline - Date.now()))) }
          );
          last = `${url}: ${response.status}`;
          if (!response.ok) continue;
          const signals = (await response.json()) as {
            subjectCid: string;
            observationKind: string;
            payloadJson: string;
          }[];
          const relayed = state(this).reports.some(report => {
            const origin = new URL(report.url);
            const doorway = new URL(page(this).url());
            return (
              [doorway.hostname, `apps.${doorway.hostname}`].includes(origin.hostname) &&
              origin.port === doorway.port &&
              report.status >= 200 &&
              report.status < 300
            );
          });
          if (
            relayed &&
            signals.some(signal => {
              const payload = JSON.parse(signal.payloadJson) as Record<string, unknown>;
              return (
                signal.subjectCid === c.contentAddress &&
                signal.observationKind === 'elohim:undeclared-behaviour' &&
                payload['content_address'] === c.contentAddress &&
                payload['signal_kind'] === 'undeclared-behaviour' &&
                typeof payload['directive'] === 'string' &&
                payload['reported_via'] === 'doorway' &&
                payload['relayed_by'] === doorwayId &&
                payload['observed_origin'] === contentOrigin &&
                payload['document_match'] === true &&
                typeof payload['blocked_uri'] === 'string' &&
                URL.canParse(payload['blocked_uri']) &&
                new URL(payload['blocked_uri']).origin === chromeOrigin
              );
            })
          )
            return true;
        } catch (error) {
          last = String(error);
        }
      }
      return false;
    }, 30_000);
    assert.notEqual(
      found,
      null,
      `${peer}: missing CSP/report-to feedback relay or no signal against ${c.contentAddress}; GET /api/v1/observations/by-subject last: ${last}. A parent DOM SecurityError alone does not guarantee a CSP report; require an observed browser POST to /_elohim/csp-report and a matching storage observation. Reports: ${JSON.stringify(state(this).reports)}`
    );
  }
);

Then(
  "each chrome's EPR links lead to the same record, on the doorway the visitor came through, naming that delivery",
  async function (this: E2EWorld) {
    for (const p of state(this).pages.slice(-2)) {
      for (const row of ['source', 'delivery', 'holders', 'steward', 'claims', 'permissions'])
        await checkLink(p, row);
    }
  }
);

Given(
  /^the household storage peer "([^"]+)" holds the bundle and knows its record as a content app \(format html5-app\)$/,
  { timeout: 360_000 },
  async function (this: E2EWorld, name: string) {
    const peer = requireFixtureStoragePeer(loadHouseholdMeshFixture(), name);
    const record = app(this);
    assert.ok(record.bundle);
    const staged = await stageBundleThroughStorage({
      bundle: record.bundle,
      slug: record.slug,
      storageUrl: peer.url,
      stepTimeoutMs: 300_000,
    });
    assert.equal(staged.code, 0, staged.output);
    record.blobHash = staged.blobHash;
    assert.notEqual(
      await pollUntil(async () => {
        const response = await fetch(`${peer.url}/db/content/${record.slug}`, {
          signal: AbortSignal.timeout(2000),
        });
        if (!response.ok) return false;
        const row = (await response.json()) as { contentFormat?: string };
        return row.contentFormat === 'html5-app';
      }, 30_000),
      null,
      `${name} has no html5-app record projection`
    );
  }
);
Given(
  'the visitor is matthew, on his own device and signed in as himself, and every doorway is unreachable from it',
  function (this: E2EWorld) {
    const peer = requireFixtureStoragePeer(loadHouseholdMeshFixture(), 'matthew');
    assert.ok(
      peer.agentPubKey,
      'Matthew fixture has no conductor key; refresh the household prologue'
    );
    const url = new URL(peer.url);
    assert.ok(
      ['localhost', '127.0.0.1'].includes(url.hostname),
      'Own-device scenario requires a local household peer'
    );
    state(this).ownOrigin = url.origin;
    const session = shellVisitSessions.get(this);
    assert.ok(session);
    session.blockServiceWorkers = true;
    session.requireOriginPair = false;
  }
);
When(
  "the visitor opens this app's entry address on the household storage peer {string} as a page in a browser",
  { timeout: 120_000 },
  async function (this: E2EWorld, name: string) {
    const peer = requireFixtureStoragePeer(loadHouseholdMeshFixture(), name);
    assert.equal(new URL(peer.url).origin, state(this).ownOrigin);
    const raw = new URL(`/apps/${app(this).slug}/index.html`, peer.url);
    raw.hostname = '127.0.0.1';
    const chrome = new URL(raw);
    chrome.hostname = 'localhost';
    const result = await visitShellInBrowser(
      raw.href,
      30_000,
      undefined,
      shellVisitSessions.get(this),
      chrome.origin
    );
    assert.equal(result.mainStatus, 200, result.pageErrors.join('\n'));
    assert.ok(result.frameBootstrapReady, result.pageErrors.join('\n'));
    assert.equal(page(this).url(), `${chrome.origin}/_elohim/shell${raw.pathname}`);
    const ctx = await island(page(this));
    assert.equal(ctx.viewer.kind, 'device-agent');
    assert.equal(ctx.medium, 'peer-native');
    const c = await content(page(this));
    assert.equal(c.isolation.boundary, 'origin-pair');
    assert.equal(c.isolation.contentOrigin, raw.origin);
    assert.equal(new URL((await frame(page(this))).url()).origin, raw.origin);
  }
);
Then(
  "page info's Delivery card says the content came from the visitor's own peer and names no doorway",
  async function (this: E2EWorld) {
    assert.deepEqual((await content(page(this))).delivery.path, [
      { role: 'self', reportedBy: 'self' },
    ]);
    const text = await card(page(this), 'delivery');
    assert.match(text, /from your own peer/i);
    assert.doesNotMatch(text, /doorway/i);
  }
);
Then(
  "page info's Source card states the record's witness as this device knows it",
  async function (this: E2EWorld) {
    const c = await content(page(this));
    const response = await fetch(`${state(this).ownOrigin}/db/content/${app(this).slug}`);
    assert.ok(response.ok);
    const row = (await response.json()) as { dhtAnchorHash?: string };
    assert.equal(c.witness, row.dhtAnchorHash ? 'green' : 'amber');
    assert.match(
      await card(page(this), 'source'),
      c.witness === 'green' ? /witnessed/i : /await|not yet|pending/i
    );
  }
);
Then(
  "page info's Holders card says how many households hold the app as far as this device knows, and when it last knew",
  async function (this: E2EWorld) {
    const c = await content(page(this));
    assert.ok(c.holders, 'Own peer did not provide a holders reading');
    assert.ok(
      c.holders.asOf && Number.isFinite(Date.parse(c.holders.asOf)),
      'Own peer omitted the reading timestamp'
    );
    const text = await card(page(this), 'holders');
    assert.ok(
      text.includes(
        c.holders.headline ??
          (c.holders.holders === 0
            ? 'Not yet held by any household'
            : `Held by ${c.holders.holders} of ${c.holders.target} households`)
      )
    );
    assert.match(text, /as of /i);
  }
);
Then('no request the browser made reached a doorway', function (this: E2EWorld) {
  const s = state(this);
  assert.ok(s.ownOrigin && s.reached.length);
  const port = new URL(s.ownOrigin).port;
  for (const url of s.reached)
    assert.ok(
      [`http://localhost:${port}`, `http://127.0.0.1:${port}`].includes(new URL(url).origin),
      `Reached outside own peer: ${url}`
    );
});
