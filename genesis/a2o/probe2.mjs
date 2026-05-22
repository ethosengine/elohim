import { chromium } from 'playwright';
const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
const ctx = await browser.newContext();
const page = await ctx.newPage();
const failed = [];
page.on('requestfailed', req =>
  failed.push(`${req.method()} ${req.url()} :: ${req.failure()?.errorText}`)
);
page.on('response', resp => {
  if (resp.status() >= 400) failed.push(`${resp.status()} ${resp.url()}`);
});
await page.goto('http://localhost:8888/threshold/login', {
  waitUntil: 'networkidle',
  timeout: 30000,
});
console.log('---failed requests---');
console.log(failed.slice(0, 15).join('\n'));
console.log('---url---', page.url());
await browser.close();
