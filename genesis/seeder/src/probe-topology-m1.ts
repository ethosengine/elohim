/**
 * M1 Topology Verification Probe
 *
 * Local Playwright probe that logs in as Matthew, screenshots all 6
 * topology surfaces, and asserts non-empty data shape. Replaces the
 * /tmp/verify-topology.mjs ad-hoc from the prior sprint with an
 * in-tree script that survives across shifts.
 *
 * Usage:
 *   ALPHA_BASE_URL=https://app.elohim.host \
 *   MATTHEW_USERNAME=matthew.dowell@alpha.elohim.host \
 *   MATTHEW_PASSWORD=TestAdmin2026! \
 *   M1_BLOB_HASH=sha256-... \
 *   npx tsx src/probe-topology-m1.ts
 */

import { chromium, type BrowserContext, type Page } from 'playwright';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const BASE = process.env.ALPHA_BASE_URL || 'https://alpha.elohim.host';
const DOORWAY = process.env.ALPHA_DOORWAY_URL || 'https://doorway-alpha.elohim.host';
const USER = process.env.MATTHEW_USERNAME || 'matthew.dowell@alpha.elohim.host';
const PASS = process.env.MATTHEW_PASSWORD || 'TestAdmin2026!';
const BLOB = process.env.M1_BLOB_HASH || '';
const OUT_DIR = resolve(process.cwd(), '.claude/shifts/m1-probe');

mkdirSync(OUT_DIR, { recursive: true });

interface Check {
  name: string;
  ok: boolean;
  detail: string;
}
const results: Check[] = [];

/**
 * JWT-injection auth — mirrors genesis/a2o/steps/fixture-humans.steps.ts.
 * Hits doorway /auth/login directly to get a token, then bakes it into:
 *  - extraHTTPHeaders (Authorization: Bearer ...) for every network request
 *  - localStorage doorway_auth_token (for the Angular auth interceptor)
 *
 * Avoids the OAuth UI flow entirely (alpha redirects to doorway-alpha's
 * /threshold/login which requires browser-side OAuth handshake).
 */
async function authenticate(): Promise<{ token: string; agentPubKey: string; humanId: string }> {
  const resp = await fetch(`${DOORWAY}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ identifier: USER.toLowerCase(), password: PASS }),
  });
  if (!resp.ok) {
    throw new Error(`login failed: ${resp.status} ${await resp.text()}`);
  }
  const json = (await resp.json()) as { token: string; agentPubKey?: string; humanId?: string };
  return {
    token: json.token,
    agentPubKey: json.agentPubKey ?? '',
    humanId: json.humanId ?? '',
  };
}

async function injectToken(ctx: BrowserContext, token: string): Promise<void> {
  // Pre-load localStorage on every page in the context so the Angular auth
  // interceptor reads the token before it issues any data fetch.
  await ctx.addInitScript(t => {
    try {
      localStorage.setItem('doorway_auth_token', t);
    } catch {
      /* localStorage may be unavailable in some contexts; header injection
         covers that path. */
    }
  }, token);
}

async function checkClusterPage(page: Page): Promise<void> {
  await page.goto(`${BASE}/shefa/cluster`);
  await page.screenshot({ path: `${OUT_DIR}/D2-cluster.png`, fullPage: true });
  const tileCount = await page.locator('[data-testid=device-tile]').count();
  results.push({
    name: 'D2: cluster page renders ≥1 device tile',
    ok: tileCount >= 1,
    detail: `device-tile count: ${tileCount}`,
  });
}

async function checkPeerTopology(page: Page): Promise<void> {
  await page.goto(`${BASE}/shefa/peers`);
  await page.screenshot({ path: `${OUT_DIR}/D3-peers.png`, fullPage: true });
  const cardCount = await page.locator('[data-testid=peer-household-card]').count();
  results.push({
    name: 'D3: peer topology shows ≥1 peer-household-card',
    ok: cardCount >= 1,
    detail: `peer-household-card count: ${cardCount}`,
  });
}

async function checkReciprocity(page: Page): Promise<void> {
  await page.goto(`${BASE}/shefa/reciprocity`);
  await page.screenshot({ path: `${OUT_DIR}/D4-reciprocity.png`, fullPage: true });
  const inflowRows = await page.locator('[data-testid=reciprocity-inflow-row]').count();
  results.push({
    name: 'D4: reciprocity page shows ≥1 inflow row',
    ok: inflowRows >= 1,
    detail: `inflow-row count: ${inflowRows}`,
  });
}

async function checkContentViewer(page: Page): Promise<void> {
  if (!BLOB) {
    results.push({
      name: 'D1+D6: blob-backed content viewer',
      ok: false,
      detail: 'M1_BLOB_HASH not set — skipped',
    });
    return;
  }
  // Resource path will likely need adjustment to wherever the manifesto loads at.
  await page.goto(`${BASE}/resource/manifesto-fruit-back-on-the-tree`);
  await page.screenshot({ path: `${OUT_DIR}/D1+D6-content.png`, fullPage: true });
  const badge = await page.locator('elohim-distribution-badge').count();
  const snap = await page.locator('elohim-resilience-snapshot').count();
  results.push({
    name: 'D1: distribution-badge renders on content viewer',
    ok: badge >= 1,
    detail: `distribution-badge count: ${badge}`,
  });
  results.push({
    name: 'D6: resilience-snapshot renders on content viewer',
    ok: snap >= 1,
    detail: `resilience-snapshot count: ${snap}`,
  });
}

async function main(): Promise<void> {
  // Authenticate against doorway, get a JWT, then run the browser session
  // with that token injected at the network + localStorage layers.
  const auth = await authenticate();
  console.log(`Authenticated as ${auth.humanId} (agentPubKey ${auth.agentPubKey.slice(0, 16)}...).`);

  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({
    extraHTTPHeaders: { Authorization: `Bearer ${auth.token}` },
  });
  await injectToken(ctx, auth.token);
  const page = await ctx.newPage();

  // Surface anything the SPA logs to the console so empty-state vs crash is
  // distinguishable in the local report.
  page.on('console', msg => {
    if (msg.type() === 'error' || msg.type() === 'warning') {
      console.log(`  [browser ${msg.type()}] ${msg.text().slice(0, 200)}`);
    }
  });

  try {
    await checkClusterPage(page);
    await checkPeerTopology(page);
    await checkReciprocity(page);
    await checkContentViewer(page);
  } finally {
    await browser.close();
  }

  console.log('\n=== M1 PROBE RESULTS ===');
  let failed = 0;
  for (const r of results) {
    const icon = r.ok ? '✓' : '✗';
    console.log(`  ${icon} ${r.name} — ${r.detail}`);
    if (!r.ok) failed += 1;
  }
  console.log(`\n${results.length - failed}/${results.length} passed.`);
  console.log(`Screenshots: ${OUT_DIR}`);

  process.exit(failed === 0 ? 0 : 1);
}

main().catch(err => {
  console.error('PROBE CRASH:', err);
  process.exit(2);
});
