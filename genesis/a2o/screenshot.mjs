import { chromium } from 'playwright';
import { mkdir } from 'node:fs/promises';
const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
const ctx = await browser.newContext({ viewport: { width: 1024, height: 768 } });
const page = await ctx.newPage();
await page.goto('http://localhost:8081/login', { waitUntil: 'networkidle', timeout: 20000 });
await page
  .locator('[data-testid="threshold-identifier"]')
  .waitFor({ state: 'visible', timeout: 15000 });
await mkdir('reports/screenshots/threshold-login-domain-scoping', { recursive: true });
await page.screenshot({
  path: 'reports/screenshots/threshold-login-domain-scoping/scenario-1-form-shown.png',
  fullPage: true,
});
await page.locator('[data-testid="threshold-identifier"]').fill('matthew.dowell@alpha.elohim.host');
await page.waitForTimeout(300);
await page.screenshot({
  path: 'reports/screenshots/threshold-login-domain-scoping/scenario-2-after-paste.png',
  fullPage: true,
});
const inputVal = await page.locator('[data-testid="threshold-identifier"]').inputValue();
const suffix = await page.locator('[data-testid="threshold-domain-suffix"]').textContent();
const hintLink = await page
  .locator('[data-testid="threshold-different-doorway-hint"]')
  .textContent();
const hintHref = await page
  .locator('[data-testid="threshold-different-doorway-hint"]')
  .getAttribute('href');
console.log('input.value:', JSON.stringify(inputVal));
console.log('suffix.textContent:', JSON.stringify(suffix?.trim()));
console.log('hint.link:', JSON.stringify(hintLink?.trim()), '→', JSON.stringify(hintHref));
await browser.close();
