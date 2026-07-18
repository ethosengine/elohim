---
id: backlog-ssr-multi-app-renderer-enablement
kind: backlog
title: SSR multi-app renderer enablement — lamad-spa server bundle + per-app selection (and the residual sheds the vocabulary now names)
created: 2026-07-18
status: OPEN
domain: D-delivery
source: SSR compose regression root-cause session (2026-07-18; manifesto page dark to crawlers)
severity: medium
tags: [ssr, elohim-render, doorway, projection, multi-app, observability]
---

The 2026-07-18 fix (selector-agnostic `compose_ssr_with_shell` + typed `ComposeError` in
elohim-render; `renderer-app-mismatch` pre-gate + granular `x-ssr-skipped` vocabulary in
doorway) stops the wrong-app render waste and makes every SSR shed name its seam. What it
deliberately does NOT do is make `/lamad/*` SSR — that requires the enablement below.
A2o anchor: `genesis/a2o/features/ssr/compose-serves-the-projected-app.feature`.

**1. lamad-spa server bundle + per-app renderer selection (the enablement).**
The doorway loads ONE server bundle (`SSR_BUNDLE_SLUG=elohim-host-landing`); routes projected
to `lamad-spa` now skip render-free with `x-ssr-skipped: renderer-app-mismatch`. To SSR the
manifesto: build `app/lamad`'s server bundle (its `angular.json` already has the ssr entry?
verify), seed `serverBlobHash` on the `lamad-spa` EPR node (root pipeline stageSpaBlob
currently populates only `elohim-host-landing`), and select a renderer PER projection app at
dispatch — the `SSR_BUNDLES_DIR` + `RenderCapabilityProfile` path (`doorway/doorway-service/
src/render/capability.rs`) is the designed home; live alpha runs the degraded single-bundle
path (`/admin/capability` returns null). The compose primitive is already selector-agnostic,
so no per-app code follows the bundle.

**2. Peer /apps fetch stalls (the leg the vocabulary now names).**
doorway-alpha-b's shell fetch to `elohim-adam-alpha:8090/apps/...` intermittently rides the
full 10s `EPR_DISPATCH_TIMEOUT_SECS` and sheds (`x-ssr-skipped: shell-fetch-failed`, warn log
carries `shell_url` + `elapsed_ms`). Same-minute retries succeed — intermittent peer stall,
not config. Dataplane concern (substrate-trust-contract probes), not doorway code.

**3. Composed pages drop the render's component styles (FOUC).**
Angular SSR emits component styles as `<style ng-app-id="ng">` in the rendered doc's head;
the compose splice transplants only the root element + ng-state, so composed pages paint
server markup unstyled until hydration. Extend the chunk extraction to carry the ng-app-id
style blocks into the shell head (elohim-render compose.rs; behavior-changing, needs its own
verify pass — kept out of the regression fix on purpose).

**4. `elohim-host-landing-ssr` app-index churn.**
Doorway's app-file cache logs `Removed app from index (will re-resolve on next request)`
slug=`elohim-host-landing-ssr` every few seconds — the retired `-ssr` sibling row still
emits invalidation signals somewhere upstream (init_renderer's comment says the sibling-row
convention was replaced by `serverBlobHash` on the ONE node). Find the signal source and
retire the row; harmless-looking but it's a per-seconds invalidation loop on every doorway.
