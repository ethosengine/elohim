#!/usr/bin/env node
// SSR-entry conformance gate — the authoring-time rail for the EPR-app server
// bundle contract (runtime twin: elohim-render compose.rs typed refusals).
//
// An EPR app that declares an SSR build (angular.json `server`/`ssr` keys)
// must ship a server entry the peer runtime can actually drive:
//
//   1. `export default` bootstrap + `export … renderApplication` wrapper —
//      the elohim-render AngularRenderer driver calls
//      `mod.renderApplication(mod.default, { url })`.
//   2. An SSR document template whose root tag MATCHES the app component's
//      selector. A renamed selector with a stale template renders into a tag
//      the compose step can't find in the shell — the exact cross-app/selector
//      drift class that kept /lamad dark for two weeks (2026-07-18 RCA); catch
//      it here at authoring time, not in prod headers.
//   3. NO custom-element register side-effect imports (`…/register`) in the
//      server entry — element upgrade is client-side after hydration; browser
//      -only code must stay out of the V8 render path.
//
// Apps with NO server entry declared pass silently — the rail fires only when
// SSR is claimed. Framework note: this rail is Angular-shaped because the
// current clients are Angular; the CONTRACT it guards (server entry the
// runtime's RenderSpec can drive + root tag locatable in the shell) is
// framework-neutral — a react-ssr rail would check its own entry shape against
// the same principles.
//
// usage: lint-ssr-entry.mjs <appDir>   (the directory holding angular.json)

import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const appDir = process.argv[2];
if (!appDir) {
  console.error('usage: lint-ssr-entry.mjs <appDir>');
  process.exit(2);
}

const angularJsonPath = join(appDir, 'angular.json');
if (!existsSync(angularJsonPath)) {
  console.error(`lint-ssr-entry: no angular.json at ${angularJsonPath}`);
  process.exit(2);
}
const angularJson = JSON.parse(readFileSync(angularJsonPath, 'utf8'));

let failures = 0;
const fail = (msg) => {
  console.error(`ssr-entry: ${msg}`);
  failures += 1;
};

for (const [projName, proj] of Object.entries(angularJson.projects ?? {})) {
  const opts = proj?.architect?.build?.options ?? proj?.targets?.build?.options ?? {};
  const serverEntry = opts.ssr?.entry ?? opts.server;
  if (!serverEntry) continue; // no SSR declared — rail does not fire

  const entryPath = join(appDir, serverEntry);
  if (!existsSync(entryPath)) {
    fail(`${projName}: declared server entry ${serverEntry} does not exist`);
    continue;
  }
  const entry = readFileSync(entryPath, 'utf8');

  // (1) driver-callable exports
  if (!/export\s+default\s/.test(entry)) {
    fail(`${projName}: ${serverEntry} must \`export default\` the bootstrap fn (driver calls mod.default)`);
  }
  if (!/export\s+(async\s+)?(function|const)\s+renderApplication/.test(entry)) {
    fail(`${projName}: ${serverEntry} must export a renderApplication wrapper (driver calls mod.renderApplication)`);
  }

  // (2) document root tag matches the app component selector
  const componentPath = join(appDir, 'src/app/app.component.ts');
  if (!existsSync(componentPath)) {
    fail(`${projName}: expected src/app/app.component.ts (convention) to read the root selector`);
  } else {
    const selMatch = readFileSync(componentPath, 'utf8').match(/selector:\s*['"]([a-z][a-z0-9-]*)['"]/);
    if (!selMatch) {
      fail(`${projName}: could not find a selector in src/app/app.component.ts`);
    } else {
      const selector = selMatch[1];
      if (!entry.includes(`<${selector}>`)) {
        fail(
          `${projName}: ${serverEntry} SSR document must contain <${selector}> ` +
            `(app.component.ts selector) — a mismatched root tag renders into an element ` +
            `the shell compose step cannot locate`,
        );
      }
    }
  }

  // (3) no browser-only element-registration side effects in the server graph's entry
  for (const line of entry.split('\n')) {
    const trimmed = line.trimStart();
    if (/^(\*|\/\/|\/\*)/.test(trimmed)) continue; // comments describe, they don't import
    if (/^import\s+['"][^'"]*\/register['"]/.test(trimmed)) {
      fail(
        `${projName}: ${serverEntry} imports a custom-element register side effect ` +
          `(${trimmed.trim()}) — element upgrade is client-side; keep it in main.ts only`,
      );
    }
  }
}

if (failures > 0) {
  console.error(`\n${failures} SSR-entry conformance failure(s). Pattern: app/elohim-app/src/main.server.ts`);
  process.exit(1);
}
console.log('lint-ssr-entry: clean');
