---
id: "backlog-ssr-server-bundle-zone-polyfill-split"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "SSR server bundle ships and runs zone.js despite zoneless DI (shared browser+server polyfills option)"
slug: "ssr-server-bundle-zone-polyfill-split"
written: "2026-07-30"
author: "angular22-campaign"
status: "backlog"
priority: "low"
tags: [angular, ssr, zone.js, zoneless, bundle-size, elohim-render, performance]
cites:
  - app/elohim-app/angular.json
  - app/lamad/angular.json
  - app/elohim-app/scripts/ssr-shim-node.mjs
  - elohim/elohim-render/src/shim/mod.rs
  - genesis/data/timeline/backlog/onpush-eager-debt-inventory.md
  - genesis/data/timeline/backlog/elohim-render-v22-elohim-app-stall.md
---

## What

`@angular/build:application` (the builder both `elohim-app` and `lamad` use — see
`architect.build.options` in each app's `angular.json`) exposes exactly ONE `polyfills` array
per build target, shared across the browser entry (`browser: "src/main.ts"`) and the server
entry (`server: "src/main.server.ts"`) that same target's `ssr.entry` config wires up. Both
apps declare:

```json
"polyfills": ["zone.js"]
```

There is no separate `polyfills.server` / `serverPolyfills` option on this builder to diverge
the two. The practical effect: **zone.js ships in and runs inside the server bundle**
(`dist/<app>/server/main.server.mjs`) even though both apps are zoneless-DI-shaped for SSR
purposes — the deno_core isolate elohim-render drives has no DOM, no real event loop in the
browser sense, and no need for zone.js's monkey-patched async API interception.

## Cost

zone.js is ~267KB (uncompressed) of `ZoneAwarePromise` global-patching machinery: it replaces
`Promise`, wraps `setTimeout`/`addEventListener`/etc., and installs itself as the ambient
async-scheduling layer for the entire isolate. In the server bundle this buys nothing —
`renderApplication()` doesn't rely on zone-triggered change detection running through zone.js's
patched microtask queue the way a real browser app's `ApplicationRef.tick()` does — but it
still pays: bundle parse/eval cost on every isolate boot, and it becomes the SAME async
substrate every `web_api.js` Promise/timer op runs through.

## Risk note: this repo has direct history with zone-vs-native-async phantom bugs

See `[[feedback_zone_native_await_unhandled_rejection]]`: zone.js checks for uncaught
rejections at drain-end, BEFORE a native `await`'s V8 thenable-job has attached its handler —
producing false-flagged "unhandled rejection" errors for rejections that ARE actually handled,
just not yet attached when zone.js's drain-end check runs. That class of bug is exactly the
kind zone.js's presence in the server isolate keeps alive as a standing risk: every `await` in
every server-rendered component, every `web_api.js` Promise (`ReadableStream` reads, `fetch()`,
`AbortSignal.timeout` — all touched in the sibling web_api.js hardening pass, see
`onpush-eager-debt-inventory.md`'s neighbor commit) runs under zone.js's patched `Promise`
instead of V8's native one, inheriting that drain-order hazard for no benefit on the server
side.

## Fix direction

1. **Investigate `@angular/build:application` for a server-specific polyfills escape hatch**
   past what's documented — check if a `ssr.experimentalPlatform` or similar undocumented knob
   exists in the Angular 22 builder, or if this needs an Angular CLI feature request (the
   builder may simply not support divergent polyfill sets by design, since CLI SSR assumes
   symmetric hydration between server and browser output).
2. **If no builder-level split exists**: investigate `main.server.ts` intercepting/no-op'ing
   zone.js at the entry point (e.g., stub `Zone.current` /  monkeypatch-undo before
   `bootstrapApplication` runs), OR building the server bundle through a separate esbuild/ng
   invocation outside this builder's single-polyfills constraint. Both are real surgery on the
   SSR entry contract documented in `app/CLAUDE.md`'s "Adding SSR to an EPR app" section —
   coordinate with that contract, don't drift from it silently.
3. **Verify no change-detection regression**: elohim-render's server config already runs
   `NoopNgZone` + `REQUESTS_CONTRIBUTE_TO_STABILITY = false` (per `app/CLAUDE.md`'s SSR
   deno_core constraints) — confirm removing zone.js from the server bundle doesn't disturb
   whatever `ApplicationRef.whenStable()` timing those settings currently protect.

## Acceptance sketch

1. Server bundle (`dist/<app>/server/main.server.mjs`) no longer contains zone.js
   (`ZoneAwarePromise` symbol absent from the built output).
2. `render_url` example still emits correct HTML at the current byte-size/timing baseline
   (regression check: elohim-app + lamad SSR routes, same acceptance bar as the web_api.js
   hardening pass — `~4.2KB` for lamad's `/lamad` route in single-digit seconds).
3. `[[feedback_zone_native_await_unhandled_rejection]]`'s reproduction case (if it has one)
   re-verified absent under the native-Promise server bundle.
4. Browser bundle unaffected (still ships zone.js for real-browser change detection, unless a
   separate zoneless-migration effort removes it there too — out of scope for this item).

**Owner:** angular-architect (builder/bundle-config surface) with rust-architect consult
(elohim-render's isolate/async assumptions the server bundle runs under).

## Investigation (2026-07-30) — measured on the branch's actual `@angular/build` 22.1.0

Read the installed builder source, not the docs. Three findings; two of them correct this entry.

### 1. There is NO server-specific polyfills option — but the split already exists, keyed off `zone.js`

`@angular/build:application`'s schema (`node_modules/@angular/build/src/builders/application/schema.json`)
still exposes exactly one `polyfills` array. `ssr` accepts only `{entry, platform}`. Confirmed: no
`polyfills.server`, no `serverPolyfills`.

BUT the builder does **not** reuse the app's polyfills array for the server bundle. It rebuilds it
from scratch (`src/tools/esbuild/application-code-bundle.js`, `createServerPolyfillBundleOptions`):

```js
const serverPolyfills = [];
if (!isZonelessApp(options.polyfills)) {
  serverPolyfills.push(isNodePlatform ? 'zone.js/node' : 'zone.js');
}
if (localize) serverPolyfills.push('@angular/localize/init');
serverPolyfills.push('@angular/platform-server/init');
```

and `isZonelessApp` (`src/tools/esbuild/utils.js:274`) is:

```js
return !polyfills?.some((p) => p === 'zone.js' || /\.[mc]?[jt]s$/.test(p));
```

So the server polyfill set is derived, and the ONLY lever is the browser `polyfills` array. Both apps
declare `["zone.js"]`, so `zoneless === false` and `zone.js/node` is injected. Note the second clause:
adding ANY local `.ts`/`.js` polyfill file would also force `zoneless === false` even with `zone.js`
removed — relevant if a future change moves the SSR preamble into `polyfills`.

**Consequence:** dropping zone.js from the server bundle is not a build-config task. It is exactly
"make the browser zoneless too", i.e. `provideZonelessChangeDetection()` in `app.config.ts` +
`"polyfills": []`. One switch, both outputs, no builder surgery. Track it there, not here.

### 2. The `main.server.ts` no-op fallback is mechanically unavailable

`createServerCodeBundleOptions` sets `banner: { js: "import './polyfills.server.mjs';" }`, and the
built artifact confirms it — the first line of `dist/elohim-app/server/main.server.mjs` is
`import './polyfills.server.mjs';`. zone.js has fully installed itself before a single byte of
`main.server.ts` executes. Nothing at the SSR entry can pre-empt it. Fallback direction #2 in this
entry's "Fix direction" is retired.

(A postbuild strip is technically reachable — `app/elohim-app/scripts/ssr-shim-node.mjs` already
rewrites `polyfills.server.mjs` to inject the SSR globals preamble — but it would mean surgically
excising minified zone.js from a 296 KB concatenation. Not worth it for the payoff below.)

### 3. The stated cost is ~8x too high — zone.js is ~33 KB, not ~267 KB

Measured, minified, via esbuild from the app's own resolution root:

| entry | bytes |
|---|---|
| `zone.js/node` | 32,675 |
| `@angular/platform-server/init` | 234,528 |
| SSR globals preamble (`scripts/ssr-globals-preamble.mjs`) | 28,937 |
| **built `polyfills.server.mjs`** | **296,313** |

The 267 KB figure this entry opened with is the polyfill bundle minus the preamble — and it is
dominated by `@angular/platform-server/init`, which is mandatory. zone.js itself is **32.7 KB, about
0.24% of the 13.5 MB server dist**. The bundle-size argument for this item is effectively void.

### What the item is actually worth, restated

Two non-size reasons survive, and one of them is new:

1. **Async substrate risk** (the original, and still the real one): every `await` and every
   `web_api.js` Promise in the isolate runs through `ZoneAwarePromise` instead of V8's native one,
   keeping `[[feedback_zone_native_await_unhandled_rejection]]`'s drain-order hazard alive server-side
   for no benefit.
2. **NEW — zone.js also changes how the server bundle is COMPILED.** `getFeatureSupport(zoneless)`
   (`src/tools/esbuild/utils.js:154`) sets `'async-await': zoneless`. With zone.js present, esbuild
   **downlevels every `async`/`await`, async generator, and `for await...of` in the server bundle to
   generator form** ("Native async/await is not supported with Zone.js"). This is visible in the
   shipped output — SSR stack traces run through `Generator.next` and the `__async(this, null,
   function*(){...})` helper rather than native frames. It costs code size beyond zone.js's own
   33 KB, costs runtime, and makes isolate stack traces harder to read (it materially slowed the
   `elohim-render-v22-elohim-app-stall` investigation). `createWasmPlugin({ allowAsync: zoneless })`
   is gated on the same flag.
3. `ssr.platform: "neutral"` would swap `zone.js/node` for plain `zone.js` (dropping Node-API
   patching that is meaningless in the deno_core isolate), but it also stops esbuild treating Node
   built-ins as external, which the server bundle currently relies on via `ssr-shim-node.mjs`. Not a
   free win; noted only so the next reader does not re-derive it.

### Status

**Blocked as scoped — and should be re-scoped.** There is no builder-level split to implement on
Angular 22.1, the entry-level no-op is impossible, and the size payoff is ~33 KB rather than ~267 KB.
The genuine prize is the async-substrate + native-async-compilation change, and the only clean lever
for it is the browser-zoneless migration (`onpush-eager-debt-inventory.md`'s neighbourhood). Recommend
folding this entry into that migration as an acceptance criterion — "server bundle contains no
`ZoneAwarePromise` and compiles with native async/await" — rather than pursuing it standalone.

Applies identically to `app/lamad` (also `"polyfills": ["zone.js"]`, same builder, same derivation).
