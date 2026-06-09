/**
 * Exploration-sidebar step definitions.
 *
 * Drives genesis/a2o/features/content/exploration-sidebar.feature — the native
 * content-graph seam slice. The SAME shared ExplorationSidebarComponent renders
 * beside content on BOTH the standalone /epr/{id} content-viewer AND the
 * path-step lesson-view, and its related-concepts panel (sourced from the
 * resolver graph endpoint with computed=true) surfaces two honestly-distinguished
 * kinds of neighbor:
 *   - AUTHORED related concepts (explicit relatedNodeIds) → related-concept-card
 *   - DISCOVERED tag neighbors (computed=true)            → discovered-concept-card
 *     carrying an inference-source attribute that is NOT "explicit".
 *
 * STORY-FIRST: the resolver (computed=true) + the shared sidebar are not yet on a
 * live stack (alpha runs the old storage). These steps go green once the branch
 * is built and deployed (C2 seed + verify, then the integrator's edge build).
 *
 * Transport: an anonymous Playwright visitor (mirrors deep-link-delivery.steps),
 * navigating cold to the universal EPR address as a recipient of a shared link
 * would. Render-verified discipline: every assertion reads a RENDERED data-testid
 * (or the markdown content body), never a raw response shell. In non-Playwright
 * (API) mode the browser steps degrade to 'pending'.
 *
 * Background steps (seeded as commons markdown EPR nodes / authored relationships
 * / computed discovered neighbor) are PRECONDITION declarations: they document the
 * witness contract the C2 seed-and-verify establishes and assert it lightly via
 * the doorway HTTP surface where cheaply observable, so a mis-seeded stack fails
 * with a clear message rather than a confusing render assertion downstream.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { Human } from '../../src/framework/human.js';
import { EprContentPage } from '../../src/framework/pages/index.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

const PENDING = 'pending';
const VISITOR_NAME = '_exploration-sidebar-visitor';

/**
 * Resolve (or lazily create) the anonymous visitor's Playwright device. Carries
 * no auth — opens the public commons markdown nodes cold. Returns null in
 * non-Playwright mode so the @browser-only steps degrade to 'pending' on API
 * runs (mirrors deep-link-delivery.steps#ensureVisitor).
 */
async function ensureVisitor(world: E2EWorld): Promise<PlaywrightDevice | null> {
  if (world.deviceMode !== 'playwright') {
    return null;
  }

  let human: Human;
  try {
    human = world.getHuman(VISITOR_NAME);
  } catch {
    human = new Human(VISITOR_NAME, { identifier: '', password: '', displayName: VISITOR_NAME });
    world.addHuman(VISITOR_NAME, human);
  }

  const existing = human.devices.find(d => d.type === 'playwright');
  if (existing) return existing as PlaywrightDevice;

  const [, doorway] = [...world.doorways][0] ?? [];
  assert.ok(doorway, 'No doorway registered. Did the Background step run?');

  const appUrl = doorwayToAppUrl(doorway.url);
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const device = new PlaywrightDevice(`${VISITOR_NAME}-pw`, appUrl, doorway.url, browser);
  await device.init();
  human.addDevice(device);
  world.onCleanup(async () => device.close());
  return device;
}

/** The page object bound to the visitor's current page. */
async function eprPage(world: E2EWorld): Promise<EprContentPage | null> {
  const device = await ensureVisitor(world);
  if (!device) return null;
  return new EprContentPage(device.page);
}

// ---------------------------------------------------------------------------
// Background — witness preconditions (documented contract; lightly HTTP-verified)
// ---------------------------------------------------------------------------

/**
 * "Given the doctrinal docs {string} are seeded as commons markdown EPR nodes" —
 * the witness corpus exists at the doorway as public, ungated, markdown content.
 * Asserts each id serves over HTTP when the doorway is reachable; tolerated as a
 * documented precondition when not (the render steps surface a clearer failure).
 */
Given(
  'the doctrinal docs {string},{string},{string},{string} are seeded as commons markdown EPR nodes',
  async function (this: E2EWorld, a: string, b: string, c: string, d: string) {
    const ids = [a, b, c, d];
    let doorwayUrl: string | undefined;
    try {
      doorwayUrl = this.getDoorway('alpha').url;
    } catch {
      return PENDING;
    }
    for (const id of ids) {
      const resp = await fetch(`${doorwayUrl}/db/content/${id}`).catch(() => null);
      assert.ok(
        resp?.ok,
        `Witness node "${id}" must serve as a seeded commons markdown EPR node ` +
          `(GET /db/content/${id} → ${resp ? resp.status : 'no response'}). ` +
          `Run the C2 seed (--content-only --ids=constitution,confession,theology) first.`
      );
    }
  }
);

/**
 * "Given {string} has authored relationships to {string}" — the focal node's
 * explicit relatedNodeIds resolve to RELATES_TO edges with inferenceSource
 * "explicit" (NOT computed). Verified via the resolver graph endpoint with
 * computed=false where reachable.
 */
Given(
  '{string} has authored relationships to {string},{string},{string}',
  async function (this: E2EWorld, focalId: string, a: string, b: string, c: string) {
    const expected = [a, b, c];
    let doorwayUrl: string | undefined;
    try {
      doorwayUrl = this.getDoorway('alpha').url;
    } catch {
      return PENDING;
    }
    const resp = await fetch(
      `${doorwayUrl}/db/relationships/graph/${focalId}?computed=false`
    ).catch(() => null);
    if (!resp?.ok) return PENDING; // resolver not yet deployed — story-first
    const graph = (await resp.json()) as { related?: { contentId: string }[] };
    const got = new Set((graph.related ?? []).map(r => r.contentId));
    for (const id of expected) {
      assert.ok(
        got.has(id),
        `"${focalId}" must have an authored relationship to "${id}" ` +
          `(explicit edge in /db/relationships/graph/${focalId}?computed=false). Got: ${[...got].join(', ')}`
      );
    }
  }
);

/**
 * "Given the backend computes at least one discovered (tag) neighbor for {string}
 * not in its relatedNodeIds" — with computed=true the resolver yields ≥1
 * inferenceSource:"tag" neighbor that is NOT among the focal node's authored
 * relatedNodeIds (the discovery the sidebar surfaces). Verified where reachable;
 * tolerated as a precondition when the resolver is not yet deployed (story-first).
 */
Given(
  String.raw`the backend computes at least one discovered \(tag) neighbor for {string} not in its relatedNodeIds`,
  async function (this: E2EWorld, focalId: string) {
    let doorwayUrl: string | undefined;
    try {
      doorwayUrl = this.getDoorway('alpha').url;
    } catch {
      return PENDING;
    }
    const resp = await fetch(`${doorwayUrl}/db/relationships/graph/${focalId}?computed=true`).catch(
      () => null
    );
    if (!resp?.ok) return PENDING; // resolver not yet deployed — story-first
    const graph = (await resp.json()) as {
      related?: { contentId: string; inferenceSource?: string }[];
    };
    const discovered = (graph.related ?? []).filter(r => r.inferenceSource === 'tag');
    assert.ok(
      discovered.length >= 1,
      `Expected ≥1 computed (tag) neighbor for "${focalId}" with computed=true; ` +
        `found ${discovered.length}. Check the C1 tag overlap (≥2 shared tags) / minSharedTags.`
    );
  }
);

// ---------------------------------------------------------------------------
// When / Given — open the surface
// ---------------------------------------------------------------------------

/**
 * "When the learner opens the EPR address for {string}" — the visitor navigates
 * cold to the universal /epr/{id} standalone address, where the content-viewer
 * renders the focal markdown AND the shared exploration sidebar (pinned rail).
 */
When('the learner opens the EPR address for {string}', async function (this: E2EWorld, id: string) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  await device.navigate(`/epr/${id}`);
});

/**
 * "Given the learner is on a path step rendering {string}" — the visitor lands on
 * a path-step lesson-view (/lamad/path/{pathId}/step/{n}) whose step renders the
 * named markdown node, exercising the SAME shared sidebar in its path-coupled
 * layout.
 *
 * The scenario targets "manifesto" — the markdown node a seeded path already
 * steps through (love-map-matthew-jessica step resourceId) — so the runtime
 * lookup resolves. The doctrinal trio stays deliberately un-path-attached
 * (slice plan C1 Step 3); the lookup scans steps[].resourceId only, not
 * chapter conceptIds.
 */
Given(
  'the learner is on a path step rendering {string}',
  async function (this: E2EWorld, contentId: string) {
    const device = await ensureVisitor(this);
    if (!device) {
      return PENDING;
    }
    const located = await locatePathStepFor(this, contentId);
    assert.ok(
      located,
      `No seeded path steps through "${contentId}". The witness corpus leaves the ` +
        `doctrinal nodes un-path-attached (slice plan C1 Step 3) — attach one to a ` +
        `path step (or retarget this scenario) to light up the path-coupled sidebar.`
    );
    await device.navigate(`/lamad/path/${located.pathId}/step/${located.stepIndex}`);
  }
);

/**
 * Find a seeded path whose step renders the given content id, returning the
 * pathId + stepIndex to deep-link to. Returns null when none exists.
 *
 * Paths are ContentNodes (`contentType: 'path'`) — there is no `/db/paths`
 * listing route on the current wire (the doorway's /db registry mirrors
 * storage's manifest, which doesn't declare one). The canonical body shape is
 * `sections[].items[]` with `ref: "epr:{id}"` (role: "step"); the lesson-view
 * URL's stepIndex is the 0-based GLOBAL item index across sections
 * (path-navigator.component.ts: "Global concept index across all sections").
 * A legacy `steps[].resourceId` shape is tolerated for older seeds.
 */
async function locatePathStepFor(
  world: E2EWorld,
  contentId: string
): Promise<{ pathId: string; stepIndex: number } | null> {
  let doorwayUrl: string | undefined;
  try {
    doorwayUrl = world.getDoorway('alpha').url;
  } catch {
    return null;
  }
  const resp = await fetch(`${doorwayUrl}/db/content?contentType=path&limit=100`).catch(() => null);
  if (!resp?.ok) return null;
  const list = (await resp.json().catch(() => null)) as { items?: PathContentRow[] } | null;
  for (const row of list?.items ?? []) {
    const body = parseBody(row.contentBody ?? row.content);
    if (!body) continue;
    // Canonical shape: sections[].items[].ref ("epr:{id}" or bare id), flat-indexed.
    let flat = 0;
    for (const section of body.sections ?? []) {
      for (const item of section.items ?? []) {
        const ref = (item.ref ?? '').replace(/^epr:/, '');
        if (ref === contentId) return { pathId: row.id, stepIndex: flat };
        flat++;
      }
    }
    // Legacy shape: top-level steps[].resourceId.
    const idx = (body.steps ?? []).findIndex(s => s.resourceId === contentId);
    if (idx >= 0) return { pathId: row.id, stepIndex: idx };
  }
  return null;
}

interface PathContentRow {
  id: string;
  contentBody?: unknown;
  content?: unknown;
}

interface PathBody {
  sections?: { items?: { ref?: string }[] }[];
  steps?: { resourceId?: string }[];
}

/** Path content bodies arrive as parsed JSON or a JSON string depending on the route. */
function parseBody(raw: unknown): PathBody | null {
  if (raw && typeof raw === 'object') return raw as PathBody;
  if (typeof raw === 'string') {
    try {
      return JSON.parse(raw) as PathBody;
    } catch {
      return null;
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Then — sidebar + neighbor assertions (render-verified)
// ---------------------------------------------------------------------------

/** "Then the exploration sidebar is visible" — the shared sidebar mounted. */
Then('the exploration sidebar is visible', async function (this: E2EWorld) {
  const page = await eprPage(this);
  if (!page) {
    return PENDING;
  }
  assert.ok(
    await page.isSidebarVisible(),
    'Expected the shared exploration sidebar (data-testid="exploration-sidebar") to be visible'
  );
});

/**
 * "Then it lists {string}, {string} and {string} as authored related concepts" —
 * the three explicit relatedNodeIds surface as authored related-concept cards.
 * Asserts by title/id substring so it survives title wording (the card renders
 * `concept.title || concept.id`).
 */
Then(
  'it lists {string}, {string} and {string} as authored related concepts',
  async function (this: E2EWorld, a: string, b: string, c: string) {
    const page = await eprPage(this);
    if (!page) {
      return PENDING;
    }
    const texts = (await page.getAuthoredConceptTexts()).map(t => t.toLowerCase());
    for (const expected of [a, b, c]) {
      const needle = expected.toLowerCase();
      assert.ok(
        texts.some(t => t.includes(needle)),
        `Expected an authored related-concept card for "${expected}"; ` +
          `authored cards were: ${JSON.stringify(texts)}`
      );
    }
  }
);

/** "Then it shows at least one discovered concept card" — ≥1 computed neighbor. */
Then('it shows at least one discovered concept card', async function (this: E2EWorld) {
  const page = await eprPage(this);
  if (!page) {
    return PENDING;
  }
  const count = await page.getDiscoveredConceptCount();
  assert.ok(
    count >= 1,
    `Expected ≥1 discovered concept card (data-testid="discovered-concept-card"); found ${count}`
  );
});

/**
 * "Then the discovered card's inference source is not {string}" — the discovery
 * is honestly marked: its inference-source attribute is NOT the authored marker.
 */
Then(
  "the discovered card's inference source is not {string}",
  async function (this: E2EWorld, forbidden: string) {
    const page = await eprPage(this);
    if (!page) {
      return PENDING;
    }
    const source = await page.getFirstDiscoveredInferenceSource();
    assert.ok(source !== null, 'Expected a discovered concept card with an inference-source attr');
    assert.notEqual(
      source,
      forbidden,
      `Discovered card must not be marked "${forbidden}" (it is computed, not authored); got "${source}"`
    );
  }
);

/**
 * "Then the exploration sidebar is visible with authored and discovered neighbors"
 * — composite assertion for the path-step flow: the SAME sidebar mounted and
 * carries both authored cards and ≥1 discovered card.
 */
Then(
  'the exploration sidebar is visible with authored and discovered neighbors',
  async function (this: E2EWorld) {
    const page = await eprPage(this);
    if (!page) {
      return PENDING;
    }
    // The lesson-view sidebar is collapsible and starts CLOSED — perform the
    // learner's "Explore" tap first (no-op on the pinned standalone viewer).
    assert.ok(
      await page.ensureSidebarOpen(),
      'Expected the shared exploration sidebar to open (Explore toggle) in the path-step lesson view'
    );
    const authored = await page.getAuthoredConceptTexts();
    assert.ok(authored.length >= 1, 'Expected at least one authored related-concept card');
    const discovered = await page.getDiscoveredConceptCount();
    assert.ok(discovered >= 1, 'Expected at least one discovered concept card');
  }
);

// ---------------------------------------------------------------------------
// Then — markdown body (independent of the epr-composite / PathViewer keystone)
// ---------------------------------------------------------------------------

/**
 * "Then the markdown content body is rendered" — the doctrinal markdown rendered
 * via the markdown renderer, with no dependency on the absent epr-composite /
 * PathViewer keystone.
 */
Then('the markdown content body is rendered', async function (this: E2EWorld) {
  const page = await eprPage(this);
  if (!page) {
    return PENDING;
  }
  assert.ok(
    await page.isMarkdownBodyRendered(),
    'Expected the markdown content body (.markdown-content) to be rendered'
  );
});

/** "Then the exploration sidebar is populated" — sidebar mounted with ≥1 card. */
Then('the exploration sidebar is populated', async function (this: E2EWorld) {
  const page = await eprPage(this);
  if (!page) {
    return PENDING;
  }
  assert.ok(await page.isSidebarVisible(), 'Expected the shared exploration sidebar to be visible');
  const authored = await page.getAuthoredConceptTexts();
  const discovered = await page.getDiscoveredConceptCount();
  assert.ok(
    authored.length + discovered >= 1,
    'Expected the exploration sidebar to be populated with at least one neighbor card'
  );
});
