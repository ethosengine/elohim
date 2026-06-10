---
id: "backlog-ci-genesis-discovery-network-assertion-unfiltered"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis discovery E2E 'no failed network requests' asserts on the raw unfiltered list → third-party externalities (YouTube/buymeacoffee/shields) make it red even on a healthy alpha; the real signal (cache-core wasm 404) is buried"
slug: "ci-genesis-discovery-network-assertion-unfiltered"
written: "2026-06-10"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
ci_status: in-progress
fingerprints: [341560bcddc1]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, elohim-genesis, browser-only, e2e, network-assertion, third-party-allowlist, discovery-assessment, brittle-assertion]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1118/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1113/
  - genesis/a2o/steps/ui/discovery-assessment.steps.ts
  - genesis/a2o/src/framework/utils/console-filters.ts
  - genesis/a2o/steps/ui/auth.steps.ts
  - genesis/a2o/src/framework/devices/playwright-device.ts
  - genesis/data/timeline/backlog/ci-app-wasm-cache-core-sha-pin-blocks-nondna-deploys.md
  - genesis/data/timeline/backlog/ci-alpha-cluster-degraded-substrate.md
---

# Genesis discovery E2E network assertion runs unfiltered → third-party noise makes it red even when alpha is healthy

## The failure

```
341560bcddc1  AssertionError [ERR_ASSERTION]: Failed network requests:
  https://www.youtube.com/embed/6g6v7ZMEAxk; https://www.youtube.com/embed/sVXwZ087ffA;
  https://alpha.elohim.host/wasm/elohim-cache-core/elohim_cache_core.js;
  https://doorway-alpha.elohim.host/health;
  https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png;
  https://img.shields.io/badge/Buy%20me%20a-Crypto%20Coffee-blue.svg?…;
  https://alpha.elohim.host/assets/fonts/webfonts/fa-brands-400.woff2;
  https://alpha.elohim.host/assets/fonts/google/…woff2;
  https://alpha.elohim.host/wasm/elohim-cache-core/elohim_cache_core.js   (genesis #1118)
```

Occurrence evidence: seen 1, first_build 1118, last_build 1118 (job
`elohim-genesis`). The assertion is `discovery-assessment.steps.ts:306` — "no
failed network requests should be captured" — failing `9 !== 0` in the
`know-thyself-discovery.feature:57` scenario "No console errors during assessment
navigation."

## Verdict

**real (test-brittleness) — a genuinely-fixable assertion seam, NOT the
degraded-substrate condition.** This is what makes #1118 distinct from its coarse
sibling `e9b60b28964c` (#1113, owned by `ci-alpha-cluster-degraded-substrate`):

- On #1118 the **alpha substrate was HEALTHY**: `SUBSTRATE PROBE — remote pool
  (shem): AVAILABLE` (build log line 219, NOT the REDUCED-SCOPE path), the
  `Verify Target Health` stage **passed** (opened line 1436, no exit-124), the
  pipeline progressed all the way into the browser E2E suite, the scenario logged
  in (Terrance on alpha), advanced through chapters, and the **JS-errors check
  PASSED** (the app loaded and ran). Live probe at triage time confirms it:
  `https://alpha.elohim.host/` → **HTTP 200**, `doorway-alpha…/health` →
  **HTTP 200**. So the "alpha is down" reading does not hold for this build.
- The ONLY failing assertion is the raw network-error count, and its nine entries
  decompose into two genuinely different buckets — which the assertion conflates.

## Root cause

`discovery-assessment.steps.ts:306` asserts on the **raw, unfiltered**
`device.getErrors().network` list:

```ts
assert.equal(errors.network.length, 0,
  `Failed network requests: ${errors.network.map(e => e.url).join('; ')}`);
```

The framework already HAS the right filter — `isExpectedNetworkFailure(req)` in
`src/framework/utils/console-filters.ts` (allowlists `youtube.com`, `ytimg.com`,
`shields.io`, `googleapis.com` and `net::ERR_ABORTED` SPA-nav aborts) — and the
auth After-path already uses it (`auth.steps.ts:276`
`device.failedRequests.filter(r => !isExpectedNetworkFailure(r))`). The discovery
step simply never imported or applied it. So:

- **Bucket A — third-party externalities (test-environment noise, NOT a defect):**
  `youtube.com/embed` ×2, `buymeacoffee.com/…orange_img.png`, `shields.io/badge`.
  These fail in CI's sandboxed egress regardless of app health. `youtube`/`shields`
  *would* have been filtered had the allowlist been applied; `buymeacoffee.com`
  was additionally **missing from the allowlist**.
- **Bucket B — alpha-owned asset failures (the real signal, correctly red):**
  `alpha.elohim.host/wasm/elohim-cache-core/elohim_cache_core.js` ×2 (live probe:
  **HTTP 404**), two `alpha.elohim.host/assets/fonts/…woff2`, and the transient
  `doorway-alpha…/health`. The cache-core wasm 404 is the **already-canonicalized
  SHA-pin concern** (`ci-app-wasm-cache-core-sha-pin-blocks-nondna-deploys.md`):
  the app build sha-pins the wasm-cache-core image, which only the DNA pipeline
  builds, so non-DNA commits 404 on the asset.

Because the assertion ran unfiltered, Bucket-A noise (4 of 9) was indistinguishable
from the Bucket-B real failures (5 of 9) — a single third-party flake could turn
the scenario red, and conversely the genuine cache-core 404 was buried in a wall
of donation-badge URLs.

## Current decision

**Bounded fix LANDED for Bucket A; Bucket B remains owned by its existing
concern.** The assertion-de-noising is in-tree, follows the established
`auth.steps.ts` reference, and is the correct largest step this run supports.
It does NOT, by itself, turn the scenario green — the cache-core wasm 404
(Bucket B) is alpha-owned and would still (correctly) fail the now-filtered
assertion until the SHA-pin concern lands. That residual is the right signal, now
unburied. So this entry is `ci_status: in-progress` (the fix landed; the
fingerprint will NOT disappear until the cache-core wasm serves), not `triaged`
with a disappearance stamp — there is no honest `triaged_at_build` here because
the bounded fix doesn't clear the fingerprint's last_build cause.

The fingerprint disappears when BOTH hold: (1) this allowlist fix reaches a
genesis run (silences Bucket A), AND (2) the cache-core wasm SHA-pin concern lands
so `alpha.elohim.host/wasm/elohim-cache-core/…` serves 200 (clears Bucket B). The
font woff2 entries should be re-evaluated then — if they 404 on a healthy
deploy they are a third small asset-serving facet; if they 200, they were
collateral of the same cache-core deploy gap.

## Fix trail

- **`genesis/a2o/src/framework/utils/console-filters.ts`** — added `buymeacoffee.com`
  to the `externalHosts` allowlist in `isExpectedNetworkFailure` (the donation
  widget on the assessment surface was the one third-party host not yet listed).
- **`genesis/a2o/steps/ui/discovery-assessment.steps.ts`** — imported
  `isExpectedNetworkFailure` and filtered the `errors.network` list before the
  count assertion (`unexpected = errors.network.filter(req => !isExpectedNetworkFailure(req))`),
  mirroring the existing `auth.steps.ts:276` pattern. The assertion message now
  reports only the unexpected (app/backend-owned) failures.
- Local verification: `tsc --noEmit` clean (exit 0); `eslint` clean on both files
  (exit 0 — the one remaining `no-unsafe-assignment` warning at line 238 is
  pre-existing, unrelated to this change); `prettier --check` clean. Cannot run
  the browser E2E locally (needs the alpha deploy + Playwright Chromium + live
  backend); the change is a pure list-filter using a type-checked, already-proven
  framework helper, so the risk surface is the filter wiring, which the typecheck
  + the auth.steps.ts precedent cover.
- Committed locally (integrator pushes; sentinel cannot trigger builds — anonymous
  Jenkins MCP). Ledger `341560bcddc1` → `status: in-progress` via
  `ci_status: in-progress` on this entry; NO `triaged_at_build` (fix is partial
  by design — Bucket B blocker remains).

## Classifier note (harvester-side, not sentinel)

`341560bcddc1` and `e9b60b28964c` share a near-identical coarse signature (both
the `discovery-assessment.steps.ts:306` "Failed network requests:" assertion) yet
have **different root causes on different builds**: #1113 was alpha-DOWN
(REDUCED-SCOPE / Verify-Target-Health exit-124 → degraded-substrate concern),
#1118 was alpha-HEALTHY with an unfiltered-assertion + cache-core-404 (this
concern). The "Failed network requests: <url-list>" line fingerprints on a
volatile URL set, so two builds with the same brittle assertion but different
underlying conditions get split into two fingerprints that should be read against
the build's substrate state (the `SUBSTRATE PROBE` / `Verify Target Health`
result), not collapsed by signature alone. A more stable fingerprint would key on
the step ref (`discovery-assessment.steps.ts:306`) plus the dominant failing host
class, not the full URL enumeration. Noted here; not opened as a separate concern.
