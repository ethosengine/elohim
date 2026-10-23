/**
 * `click-probe` — does every clickable-looking thing on a page DO something?
 *
 * `look` renders a page; this drives it. For one URL (optionally signed in as
 * a fixture human) it enumerates everything that invites a click — links,
 * buttons, ARIA widgets, and anything styled `cursor: pointer`, piercing open
 * shadow roots so Lit elements are included — then, from a fresh load each
 * time, clicks each one and classifies what happened:
 *
 *   navigated       the route changed
 *   in-page         same route, visible content changed (menu, panel, tab)
 *   popup           a new tab/window opened
 *   PIXELS-ONLY     text unchanged, pixels moved — an animation OR a dead click; read the shot
 *   NO-EFFECT       nothing visible changed — the dead-click class
 *   UNCLICKABLE     covered, detached or not actionable
 *   skipped         destructive or session-ending label; never clicked
 *
 * `cursorOnly: true` marks an element that invites a click ONLY through
 * `cursor: pointer` (no link/button/role) — an affordance with no semantics,
 * worth a look even when the click appeared to do something.
 *
 * It also records console errors / page errors / HTTP >=400 per click, the
 * page's landmark navigation (for duplicate-nav reading), and the status of
 * external links. Read-mostly: destructive labels are skipped, nothing is
 * typed, but a click is still a click against a live deployment.
 *
 *   pnpm probe <url> [--as <FixtureHuman>] [--out <slug>] [--max <n>] [--viewport WxH]
 *
 * Writes reports/probe/<slug>/{probe.json,page.png,click-<n>.jpg}.
 */

import { writeFileSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium, type ElementHandle, type Page } from 'playwright';

import { PlaywrightDevice } from '../src/framework/devices/playwright-device.js';
import { fixtureCredentials } from '../src/framework/fixtures/humans.js';

const USAGE =
  'Usage: probe <url> [--as <FixtureHuman>] [--out <slug>] [--max <n>] [--viewport WxH] [--budget <seconds>]';

/** Labels never clicked: they mutate shared state or end the session. */
const SKIP_WORDS = [
  'sign ?out',
  'log ?out',
  'delete',
  'remove',
  'revoke',
  'leave',
  'publish',
  'send',
  'submit',
  'confirm',
  'archive',
  'destroy',
  'reset',
  'follow',
  'unfollow',
  'block',
  'pay',
  'transfer',
  'approve',
  'reject',
  'accept',
  'decline',
  'vote',
  'join',
  'enroll',
  'invite',
  'request',
  'save',
  'create',
  'add',
  'like',
  'react',
  'share',
  'post',
  'upload',
  'install',
  'grant',
];
const SKIP_LABEL = new RegExp(String.raw`\b(${SKIP_WORDS.join('|')})\b`, 'i');

interface Candidate {
  i: number;
  tag: string;
  role: string | null;
  text: string;
  label: string | null;
  testid: string | null;
  href: string | null;
  target: string | null;
  cursorOnly: boolean;
  region: string | null;
  sig: string;
}

interface ClickResult extends Candidate {
  outcome:
    | 'navigated'
    | 'in-page'
    | 'PIXELS-ONLY'
    | 'popup'
    | 'NO-EFFECT'
    | 'UNCLICKABLE'
    | 'skipped';
  from: string;
  to: string | null;
  detail?: string;
  consoleErrors: string[];
  pageErrors: string[];
  httpErrors: { url: string; status: number }[];
  shot?: string;
}

const CLICK_SEL =
  'a[href],button,[role=button],[role=link],[role=tab],[role=menuitem],[role=option],[role=switch],[role=checkbox],summary,[onclick],[tabindex]:not([tabindex="-1"])';

// In-page code is passed as STRINGS: tsx's keepNames wraps named inner
// functions in a `__name` helper the browser lacks (see genesis/a2o/CLAUDE.md).

/** Every click-inviting, visible element, through open shadow roots, in document order. */
const COLLECT = `(() => {
  const SEL = ${JSON.stringify(CLICK_SEL)};
  const out = [], seen = new Set();
  const up = el => el.parentElement || (el.getRootNode() && el.getRootNode().host) || null;
  const walk = root => {
    for (const el of Array.from(root.querySelectorAll('*'))) {
      if (el.shadowRoot) walk(el.shadowRoot);
      if (seen.has(el)) continue;
      const m = el.matches(SEL);
      const cur = !m && getComputedStyle(el).cursor === 'pointer';
      if (cur) { const p = up(el); if (p && getComputedStyle(p).cursor === 'pointer') continue; }
      if (!m && !cur) continue;
      const r = el.getBoundingClientRect(), cs = getComputedStyle(el);
      if (r.width < 2 || r.height < 2 || cs.visibility === 'hidden' || cs.display === 'none') continue;
      // Rendered but not perceivable (faded-out flip face, inert/aria-hidden subtree) is not an invitation.
      if (el.checkVisibility && !el.checkVisibility({ opacityProperty: true, visibilityProperty: true })) continue;
      if (el.closest('[inert],[aria-hidden=true]')) continue;
      seen.add(el); out.push(el);
    }
  };
  walk(document);
  return out;
})()`;

/** Describe each collected element (signature lets a reload find it again). */
const ENUMERATE = String.raw`(() => {
  const SEL = ${JSON.stringify(CLICK_SEL)};
  const up = el => el.parentElement || (el.getRootNode() && el.getRootNode().host) || null;
  return ${COLLECT}.map((el, i) => {
    const text = (el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ');
    let region = null;
    for (let n = el; n; n = up(n)) {
      const t = n.tagName.toLowerCase(), role = n.getAttribute('role');
      if (['nav','header','footer','aside','main','dialog'].includes(t)) { region = t; break; }
      if (role && ['navigation','banner','contentinfo','complementary','dialog','menu'].includes(role)) { region = role; break; }
    }
    const href = el.getAttribute('href');
    const c = {
      i, tag: el.tagName.toLowerCase(), role: el.getAttribute('role'), text: text.slice(0, 80),
      label: el.getAttribute('aria-label'), testid: el.getAttribute('data-testid'),
      href: href ? new URL(href, location.href).href : null, target: el.getAttribute('target'),
      cursorOnly: !el.matches(SEL), region, sig: '',
    };
    c.sig = [c.tag, c.role, c.testid, c.label, c.text.slice(0, 40), href].join('|');
    return c;
  });
})()`;

/** A cheap fingerprint of what the person can see. */
const VISIBLE_STATE = `(() => {
  const t = document.body.innerText;
  const open = document.querySelectorAll('dialog[open],[role=dialog],[role=menu],[aria-expanded=true]').length;
  return [t.length, document.querySelectorAll('*').length, open, t.slice(0, 2000)].join('§');
})()`;

async function settle(page: Page, ms = 1500) {
  await page.waitForLoadState('networkidle', { timeout: 5000 }).catch(() => {});
  await page.waitForTimeout(ms);
}

async function load(page: Page, url: string) {
  await page.goto(url, { waitUntil: 'load', timeout: 30_000 }).catch(() => {});
  await settle(page, 2000);
}

export async function runProbe(argv: string[]) {
  const args = [...argv];
  const url = args.shift();
  if (!url || url.startsWith('--')) throw new Error(USAGE);
  let as: string | undefined;
  let slug = 'latest';
  let max = 40;
  let budget = 240;
  let viewport = { width: 1280, height: 800 };
  for (let i = 0; i < args.length; i++) {
    const v = args[i + 1];
    if (args[i] === '--as') {
      as = v;
      i++;
    } else if (args[i] === '--out') {
      slug = v;
      i++;
    } else if (args[i] === '--max') {
      max = Number(v);
      i++;
    } else if (args[i] === '--budget') {
      budget = Number(v);
      i++;
    } else if (args[i] === '--viewport') {
      const m = /^(\d+)x(\d+)$/.exec(v ?? '');
      if (!m) throw new Error(USAGE);
      viewport = { width: Number(m[1]), height: Number(m[2]) };
      i++;
    } else throw new Error(`Unknown flag: ${args[i]}\n${USAGE}`);
  }

  const outDir = resolve('reports/probe', slug);
  await mkdir(outDir, { recursive: true });
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });
  const device = new PlaywrightDevice(
    'probe',
    new URL(url).origin,
    process.env['E2E_DOORWAY_ALPHA'] ?? '',
    browser as unknown as ConstructorParameters<typeof PlaywrightDevice>[3]
  );
  const results: ClickResult[] = [];
  // An unattended walk must always end with a report. The loop stops clicking at the budget;
  // the watchdog covers a hang inside one step (login, a navigation that never settles) by
  // writing what it has and exiting 2 — a partial report, never a silent hang.
  const deadline = Date.now() + budget * 1000;
  let truncated: false | 'budget' = false;
  const watchdog = setTimeout(
    () => {
      writeFileSync(
        join(outDir, 'probe.json'),
        JSON.stringify(
          { url, as: as ?? null, truncated: 'watchdog', probed: results.length, results },
          null,
          2
        )
      );
      console.error(`${outDir}/probe.json  watchdog after ${budget + 90}s — partial report`);
      process.exit(2);
    },
    (budget + 90) * 1000
  );
  watchdog.unref();
  try {
    await device.init();
    const page = device.page as unknown as Page;
    await page.setViewportSize(viewport);
    let consoleErrors: string[] = [];
    let pageErrors: string[] = [];
    let httpErrors: { url: string; status: number }[] = [];
    page.on('console', m => {
      if (m.type() === 'error' || m.type() === 'warning')
        consoleErrors.push(`${m.type()}: ${m.text()}`.slice(0, 300));
    });
    page.on('pageerror', e => pageErrors.push(e.message.slice(0, 300)));
    page.on('response', r => {
      if (r.status() >= 400) httpErrors.push({ url: r.url(), status: r.status() });
    });
    const drain = () => {
      const d = { consoleErrors, pageErrors, httpErrors };
      consoleErrors = [];
      pageErrors = [];
      httpErrors = [];
      return d;
    };

    if (as) {
      const creds = fixtureCredentials(as);
      await device.login({ identifier: creds.identifier, password: creds.password });
    }
    await load(page, url);
    const landedAt = page.url();
    const onLoad = drain();
    await page.screenshot({ path: join(outDir, 'page.png'), fullPage: true });
    const candidates = await page.evaluate<Candidate[]>(ENUMERATE);

    const externalLinks: { href: string; status: number | string }[] = [];
    const origin = new URL(landedAt).origin;
    const ext = [
      ...new Set(
        candidates
          .map(c => c.href)
          .filter((h): h is string => !!h && /^https?:/.test(h) && new URL(h).origin !== origin)
      ),
    ];
    externalLinks.push(
      ...(await Promise.all(
        ext.slice(0, 30).map(async href => ({
          href,
          status: await page.request.get(href, { timeout: 10_000, maxRedirects: 5 }).then(
            r => r.status() as number | string,
            e => String((e as Error).message).slice(0, 80)
          ),
        }))
      ))
    );

    for (const c of candidates.slice(0, max)) {
      if (Date.now() > deadline) {
        truncated = 'budget';
        break;
      }
      const base = {
        ...c,
        from: landedAt,
        to: null as string | null,
        consoleErrors: [] as string[],
        pageErrors: [] as string[],
        httpErrors: [] as { url: string; status: number }[],
      };
      if (SKIP_LABEL.test(`${c.text} ${c.label ?? ''} ${c.testid ?? ''}`)) {
        results.push({ ...base, outcome: 'skipped' });
        continue;
      }
      if (c.href && new URL(c.href).origin !== origin) {
        results.push({
          ...base,
          outcome: 'navigated',
          to: c.href,
          detail: 'external link (status in externalLinks, not clicked)',
        });
        continue;
      }
      if (page.url() === landedAt) {
        await page.evaluate(() => window.scrollTo(0, 0));
      } else {
        await load(page, landedAt);
      }
      drain();
      const now = await page.evaluate<Candidate[]>(ENUMERATE);
      const idx = now.findIndex(n => n.sig === c.sig);
      if (idx < 0) {
        await load(page, landedAt);
        drain();
      }
      const fresh = idx < 0 ? await page.evaluate<Candidate[]>(ENUMERATE) : now;
      const at = fresh.findIndex(n => n.sig === c.sig);
      if (at < 0) {
        results.push({
          ...base,
          outcome: 'UNCLICKABLE',
          detail: 'not present after reload (unstable render)',
        });
        continue;
      }
      const handle = await page.evaluateHandle(`${COLLECT}[${at}]`);
      const el = handle.asElement() as ElementHandle<Element> | null;
      if (!el) {
        results.push({ ...base, outcome: 'UNCLICKABLE', detail: 'element handle lost' });
        continue;
      }
      // Hover first so hover styling is in both frames; only the click itself is measured.
      await el.hover({ timeout: 2000 }).catch(() => {});
      await page.waitForTimeout(400);
      const before = await page.evaluate(VISIBLE_STATE);
      const beforeShot = await page.screenshot({ type: 'jpeg', quality: 40 }).catch(() => null);
      const popup = page
        .context()
        .waitForEvent('page', { timeout: 2500 })
        .catch(() => null);
      try {
        await el.click({ timeout: 4000 });
      } catch (e) {
        results.push({
          ...base,
          outcome: 'UNCLICKABLE',
          detail: (e as Error).message.split('\n').slice(0, 3).join(' ').slice(0, 240),
          ...drain(),
        });
        continue;
      }
      await settle(page);
      const opened = await popup;
      const after = page.url();
      const shot = `click-${c.i}.jpg`;
      await page
        .screenshot({ path: join(outDir, shot), type: 'jpeg', quality: 60 })
        .catch(() => {});
      let outcome: ClickResult['outcome'];
      if (opened) {
        outcome = 'popup';
        await opened.close().catch(() => {});
      } else if (after.split('#')[0] === landedAt.split('#')[0]) {
        // NO-EFFECT only when neither the text state nor the pixels moved —
        // an opacity flip changes pixels but not innerText.
        const afterShot = await page.screenshot({ type: 'jpeg', quality: 40 }).catch(() => null);
        const pixelsMoved = !beforeShot || !afterShot || !beforeShot.equals(afterShot);
        const textMoved = (await page.evaluate(VISIBLE_STATE)) !== before;
        outcome = textMoved ? 'in-page' : pixelsMoved ? 'PIXELS-ONLY' : 'NO-EFFECT';
      } else {
        outcome = 'navigated';
      }
      results.push({ ...base, outcome, to: opened ? opened.url() : after, shot, ...drain() });
    }

    const nav = candidates.filter(c => c.region && c.region !== 'main' && c.href);
    const byHref = new Map<string, string[]>();
    for (const c of nav)
      byHref.set(c.href!, [
        ...(byHref.get(c.href!) ?? []),
        `${c.region}:${[c.text, c.label, c.testid].find(Boolean)}`,
      ]);
    const duplicateNav = [...byHref]
      .filter(([, v]) => v.length > 1)
      .map(([href, where]) => ({ href, where }));

    const summary = results.reduce<Record<string, number>>(
      (a, r) => ({ ...a, [r.outcome]: (a[r.outcome] ?? 0) + 1 }),
      {}
    );
    const report = {
      url,
      landedAt,
      as: as ?? null,
      viewport: `${viewport.width}x${viewport.height}`,
      candidates: candidates.length,
      probed: results.length,
      truncated,
      summary,
      onLoad,
      duplicateNav,
      externalLinks,
      links: [
        ...new Set(
          candidates.map(c => c.href).filter((h): h is string => !!h && h.startsWith(origin))
        ),
      ],
      results,
    };
    await writeFile(join(outDir, 'probe.json'), JSON.stringify(report, null, 2));
    console.log(
      `${outDir}/probe.json  ${JSON.stringify(summary)}  candidates=${candidates.length}`
    );
  } finally {
    clearTimeout(watchdog);
    await device.close().catch(() => {});
    await browser.close().catch(() => {});
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  runProbe(process.argv.slice(2)).catch(e => {
    console.error((e as Error).message);
    process.exit(1);
  });
}
