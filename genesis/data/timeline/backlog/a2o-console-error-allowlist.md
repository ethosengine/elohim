---
id: "backlog-a2o-console-error-allowlist"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Strict console-error After-hook needs a per-scenario @allow-doorway-flake allowlist so env flake doesn't mask passing assertions"
slug: "a2o-console-error-allowlist"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "genesis/a2o"
recurrence: 1
source_shifts:
  - "2026-05-07"
domain: "code"
relatedNodeIds:
  - "memory:feedback_a2o_is_human_experience_not_dev_bugs"
  - "memory:feedback_cascade_halt_masks_failures"
tags: [genesis, a2o, cucumber, console-error, allowlist, code-domain]
shift_objective: |
  The strict console-error After-hook fails any scenario whose page logged a console error,
  which is the right default — but environmental doorway flake (a transient proxy/network
  error logged to console) then fails scenarios whose actual assertions passed. There's no way
  to say "this specific scenario tolerates this specific known-environmental error" (observed
  2026-05-07).
  Resolve it with a per-scenario allowlist tag — e.g. `@allow-doorway-flake` — that the
  After-hook reads to permit a narrow, named set of known-environmental console errors for that
  scenario only, while keeping the strict default for everything else. This is code-domain (the
  cucumber After-hook + a tag convention). Keep the allowlist NARROW and named so it can't
  become a blanket mute. Done when a scenario tagged @allow-doorway-flake tolerates the named
  environmental error while still failing on any other console error.
---

# Per-scenario console-error allowlist for known environmental flake

## Why this matters

Code-domain. A strict console-error gate is correct, but without an escape valve for named
environmental noise it produces false failures that erode trust in the gate — and the usual
"fix" is to weaken the gate globally, which is worse. A narrow, named, per-scenario allowlist
keeps the strict default everywhere else.

## The failure shape

- The After-hook fails any scenario whose page logged a console error.
- Transient doorway/proxy flake logs an error even when the scenario's assertions passed.
- No per-scenario tolerance → the passing scenario fails on environmental noise.

## Shape of the fix (code-domain)

A tag (e.g. `@allow-doorway-flake`) that the After-hook reads to permit a **narrow, named**
set of known-environmental console errors for that scenario only; strict default preserved
elsewhere. Keep the allowlist scoped so it can't become a blanket mute
(`feedback_cascade_halt_masks_failures` — don't bury real failures).

## Acceptance

A scenario tagged `@allow-doorway-flake` tolerates the named environmental error while still
failing on any other console error.

## The opposite failure on the same seam: navigation scenarios cannot see a 404

*(Moved here 2026-09-24 from `handoff-sprawl-decompose-2026-06-23.md`, Track C, when that entry
closed. Re-verified in-tree the same day: still open.)*

The allowlist above deals with a gate that is too strict. The browser navigation scenarios have the
opposite problem, a gate that is too loose, in the same filter layer.
`Then the page should load successfully` (`genesis/a2o/steps/ui/navigation.steps.ts`) only waits for
`<body>` to be visible. `isSpaRoutingNoise` (`genesis/a2o/src/framework/utils/console-filters.ts`)
deliberately drops 404, 403 and status-0 resource errors. As a result, a route can 404 on its data,
render the not-found component, and still pass every scenario in
`genesis/a2o/features/browser/navigation-browser.feature`. A related trap: an undeclared doorway
path answers `200 text/html` with the SPA shell. A probe of `/p2p-peers` did exactly that on
2026-09-24. Any JSON assertion therefore has to check `content-type`, not just the status code.

**The cure is scoped, never blanket.** Do not add a global "no httpErrors" gate. Some live
signatures are intended (for example a 403 at a commons reach gate), and a blanket gate would keep
genesis red. Instead, each fixed route gets a navigation scenario that asserts on a rendered
`data-testid` and on the absence of the *specific* signature the fix removed. `look.ts` capture
(`httpErrors`, `pageErrors`) and the `apex-transition.steps.ts` `httpErrors` assertions are the
precedent. Candidate routes from the 2026-06-23 shakeout:

1. `/identity*` and root deep-links render the SPA shell, not a conductor or JSON 404. The doorway
   side is locked by unit test `shakeout_service_path_identity_narrowed_to_did`.
2. No `/wasm/elohim-cache-core/...` request fires on alpha or prod. The live residue is tracked in
   `wasm-cache-core-404-persists-after-preferwasm-gate-2026-06-23.md`.
3. `/map` degrades gracefully: `data-testid="map-error"` is present and there is no uncaught
   pageerror.
4. `custodians/metrics/recommendations` returns an honest 404, never a panic 503.
5. Operator portal and auth: `/threshold/*` versus `/dashboard`. Author this one only after the
   hosted-auth surface settles.

Done when each route above that is still live has a scoped scenario that fails on its specific
signature, and the generic page-load step is either retired or documented as a render-only smoke
check.
