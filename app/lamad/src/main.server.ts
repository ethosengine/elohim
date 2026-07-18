import { bootstrapApplication } from '@angular/platform-browser';
import { renderApplication as _platformRenderApplication } from '@angular/platform-server';

import { AppComponent } from './app/app.component';
import { config } from './app/app.config.server';

// Minimal HTML document template for SSR.
// @angular/platform-server's renderApplication needs a document with the
// app root selector present so it has a DOM node to render into. The compose
// step (elohim-render compose.rs) derives this root tag from the rendered
// output — <lamad-root> here — and splices it into the browser shell, so the
// selector lives in exactly two places: app.component.ts and this template.
//
// NOTE: unlike src/main.ts, this entry does NOT import the custom-element
// register side effects (elohim-core/register, elohim-imagodei/register) —
// custom elements upgrade client-side after hydration; SSR renders them as
// inert unknown elements, which is correct and keeps browser-only code out of
// the V8 render path.
const SSR_DOCUMENT = `<!DOCTYPE html><html><head><meta charset="utf-8"></head><body><lamad-root></lamad-root></body></html>`;

// The bootstrap function receives a BootstrapContext from @angular/platform-server's
// renderApplication(). The context carries the platformRef that @angular/platform-server
// creates via createServerPlatform(options). We must forward it to bootstrapApplication()
// as the third argument — omitting it triggers NG0401 ("Missing Platform").
//
// Type cast: bootstrapApplication's third arg is BootstrapContext from '@angular/core',
// which matches the shape @angular/platform-server passes ({ platformRef }).
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const bootstrap = (ctx?: any) => bootstrapApplication(AppComponent, config, ctx);

// renderApplication wrapper exported for elohim-render's AngularRenderer driver:
//   `mod.renderApplication(mod.default, { url })`
// Injects a default document if the caller omits it — the driver only supplies { url }.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function renderApplication(bootstrapFn: any, options: Record<string, any>): Promise<string> {
  return _platformRenderApplication(bootstrapFn, {
    document: SSR_DOCUMENT,
    ...options,
  });
}

export default bootstrap;
