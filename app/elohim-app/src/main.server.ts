import { bootstrapApplication, type BootstrapContext } from '@angular/platform-browser';
import { renderApplication as _platformRenderApplication } from '@angular/platform-server';

import { AppComponent } from './app/app.component';
import { config } from './app/app.config.server';

// Minimal HTML document template for SSR.
// @angular/platform-server's renderApplication needs a document with the
// app root selector present so it has a DOM node to render into.
// The elohim-render AngularRenderer driver calls renderApplication(bootstrap, { url })
// without supplying a document — this default template ensures it is always available.
const SSR_DOCUMENT = `<!DOCTYPE html><html><head><meta charset="utf-8"></head><body><app-root></app-root></body></html>`;

// The bootstrap function receives a BootstrapContext from @angular/platform-server's
// renderApplication(). The context carries the platformRef that @angular/platform-server
// creates via createServerPlatform(options). We must forward it to bootstrapApplication()
// as the third argument — omitting it triggers NG0401 ("Missing Platform").
//
// BootstrapContext became public API in Angular 22 (exported from
// '@angular/platform-browser', alongside bootstrapApplication), so the previous
// `any` cast + eslint-disable are no longer needed. renderApplication's bootstrap
// parameter is typed `(context: BootstrapContext) => Promise<ApplicationRef>`.
const bootstrap = (ctx: BootstrapContext) => bootstrapApplication(AppComponent, config, ctx);

// renderApplication wrapper exported for elohim-render's AngularRenderer driver:
//   `mod.renderApplication(mod.default, { url })`
// Injects a default document if the caller omits it — the driver only supplies { url }.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function renderApplication(
  bootstrapFn: any,
  options: Record<string, any>
): Promise<string> {
  return _platformRenderApplication(bootstrapFn, {
    document: SSR_DOCUMENT,
    ...options,
  });
}

export default bootstrap;
