/**
 * One-off interactive look: open the protocol-omni resilience hypercard panel
 * at a mobile viewport and screenshot closed + open states.
 *
 * Usage: npx tsx scripts/look-resilience-panel.ts [url] [WxH]
 */

import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { chromium } from 'playwright';

const url = process.argv[2] ?? 'https://doorway-alpha.elohim.host/';
const vpArg = /^(\d+)x(\d+)$/.exec(process.argv[3] ?? '390x844');
const viewport = vpArg
  ? { width: Number(vpArg[1]), height: Number(vpArg[2]) }
  : { width: 390, height: 844 };

const outDir = resolve('reports/look/resilience-panel-mobile');

async function main(): Promise<void> {
  await mkdir(outDir, { recursive: true });
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });
  const page = await browser.newPage({ viewport });
  const consoleLogs: { type: string; text: string }[] = [];
  const pageErrors: string[] = [];
  const httpErrors: { url: string; status: number }[] = [];
  page.on('console', m => consoleLogs.push({ type: m.type(), text: m.text() }));
  page.on('pageerror', e => pageErrors.push(e.message));
  page.on('response', r => {
    if (r.status() >= 400) httpErrors.push({ url: r.url(), status: r.status() });
  });

  const notes: string[] = [];
  try {
    await page.goto(url, { waitUntil: 'networkidle', timeout: 45_000 }).catch(() => {
      notes.push('goto: networkidle timeout — continuing with what rendered');
    });

    // 1. Expand the omni toolbar if collapsed to the chip.
    const chip = page.locator('[data-testid="protocol-omni-chip"]');
    if (await chip.isVisible().catch(() => false)) {
      await chip.click();
      await page.waitForTimeout(400);
    }
    await page.screenshot({ path: join(outDir, '1-toolbar.png'), fullPage: false });

    // 2. Click the resilience icon (or report the neutral glyph).
    const icon = page.locator('[data-testid="resilience-icon"]');
    const iconVisible = await icon.isVisible().catch(() => false);
    if (!iconVisible) {
      notes.push(
        'resilience-icon NOT present — snapshot likely unresolved (neutral glyph fallback)'
      );
    } else {
      const iconBox = await icon.boundingBox();
      notes.push(`icon boundingBox: ${JSON.stringify(iconBox)}`);
      await icon.click();
      await page.waitForTimeout(500);

      const panel = page.locator('[data-testid="resilience-hypercard"]');
      const panelBox = await panel.boundingBox();
      notes.push(`panel boundingBox: ${JSON.stringify(panelBox)} (viewport ${viewport.width}x${viewport.height})`);
      if (panelBox) {
        const offRight = panelBox.x + panelBox.width - viewport.width;
        const offLeft = -panelBox.x;
        notes.push(
          `overflow: right=${offRight.toFixed(1)}px left=${offLeft.toFixed(1)}px ` +
            `(positive = pixels off-screen)`
        );
      }
    }
    await page.screenshot({ path: join(outDir, '2-panel-open.png'), fullPage: false });

    // 3. Flip to full density via the panel's action row (shadow DOM pierced).
    const viewFull = page.locator('elohim-hypercard-panel button', {
      hasText: 'View full resilience',
    });
    if (await viewFull.isVisible().catch(() => false)) {
      await viewFull.click();
      await page.waitForTimeout(400);
      const panel = page.locator('[data-testid="resilience-hypercard"]');
      const fullBox = await panel.boundingBox();
      notes.push(`full-density panel boundingBox: ${JSON.stringify(fullBox)}`);
      await page.screenshot({ path: join(outDir, '3-panel-full.png'), fullPage: false });
    } else {
      notes.push('view-full action not found — skipped full-density shot');
    }

    // Tap-target audit of the trigger.
    const iconEl = page.locator('[data-testid="resilience-icon"]');
    if (await iconEl.isVisible().catch(() => false)) {
      const m = await iconEl.evaluate(el => {
        const r = el.getBoundingClientRect();
        const cs = getComputedStyle(el);
        return { w: r.width, h: r.height, padding: cs.padding, fontSize: cs.fontSize };
      });
      notes.push(`trigger tap target: ${JSON.stringify(m)} (WCAG 2.5.8 minimum 24x24)`);
    }
  } finally {
    await writeFile(
      join(outDir, 'capture.json'),
      JSON.stringify(
        { url, viewport, notes, console: consoleLogs, pageErrors, httpErrors },
        null,
        2
      )
    );
    await browser.close();
  }
  console.log(outDir);
  for (const n of notes) console.log('NOTE:', n);
}

main().catch(e => {
  console.error(e instanceof Error ? e.message : String(e));
  process.exit(2);
});
