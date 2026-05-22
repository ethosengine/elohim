import { chromium } from 'playwright';
const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
const ctx = await browser.newContext();
const page = await ctx.newPage();
page.on('console', msg => console.log('[console]', msg.type(), msg.text()));
page.on('pageerror', err => console.log('[pageerror]', err.message));
await page.goto('http://localhost:8888/threshold/login', {
  waitUntil: 'networkidle',
  timeout: 30000,
});
await page.screenshot({ path: '/tmp/threshold-probe.png', fullPage: true });
const html = await page
  .locator('app-root')
  .innerHTML()
  .catch(() => '(empty app-root)');
console.log('---app-root.innerHTML preview---');
console.log(html.slice(0, 500));
console.log('---url---', page.url());
console.log(
  '---input present?---',
  await page.locator('[data-testid="threshold-identifier"]').count()
);
await browser.close();
