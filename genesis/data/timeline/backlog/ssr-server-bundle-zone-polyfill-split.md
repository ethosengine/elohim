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
  - elohim/elohim-render/src/shim/mod.rs
  - genesis/data/timeline/backlog/onpush-eager-debt-inventory.md
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
