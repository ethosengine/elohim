---
id: "backlog-ci-genesis-lamad-shell-routing-regression"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Lamad/shell routing E2E regression — home title 'Welcome' not 'elohim.host'; doubled /lamad/lamad/ legacy URL no longer renders the designed not-found"
slug: "ci-genesis-lamad-shell-routing-regression"
written: "2026-06-07"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [606fa2a22a53, a969e96c4361]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, genesis, e2e, lamad, doorway, routing, spa-fallback, epr-dispatch]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1105/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1104/
  - genesis/a2o/features/deployment/staging-validation.feature
  - genesis/a2o/features/lamad/deep-link-delivery.feature
  - genesis/a2o/steps/ui/navigation.steps.ts
  - genesis/a2o/steps/lamad/deep-link-delivery.steps.ts
  - app/elohim-app/src/index.html
  - app/elohim-app/src/app/app.routes.ts
  - doorway/doorway-service/src/server/http.rs
  - genesis/data/timeline/backlog/epr-routing-complementary-captures.md
---

# Lamad / shell routing E2E regression (elohim-genesis)

Two E2E surfaces that depend on the doorway serving the elohim-app shell and the
Angular router's `**` fallback both regressed in the same onset window as the
doorway/EPR Slice-2/Slice-3 landing.

## The failure

Two fingerprints, one root concern — both are "the doorway served the wrong thing
at a deep/root URL," same SPA-shell-via-doorway surface.

`606fa2a22a53` — Scenario "Essential page elements are present"
(`features/deployment/staging-validation.feature:17`), navigating to `/` on
alpha:

```
✔ When the page loads
✖ Then the page title should contain "elohim.host"
AssertionError [ERR_ASSERTION]: Expected title to contain "elohim.host" but got "Welcome"
```

The app's own static title is correct — `app/elohim-app/src/index.html:5` reads
`<title>elohim.host - Digital Infrastructure for Human Flourishing</title>`. A
served title of "Welcome" therefore means the doorway is NOT serving the
elohim-app shell at `/` on alpha (it is serving some other document/shell whose
title is "Welcome"). This is a doorway-dispatch / root-route regression, not an
app title-tag drift.

`a969e96c4361` — Scenario "Legacy doubled URL shows the designed not-found"
(`features/lamad/deep-link-delivery.feature:38`, `@browser-only`,
data-flow spec §12.4.2):

```
✔ Given a learner opens the deep link "/lamad/lamad/path/foundations-christian-technology" cold
✖ Then the lamad designed not-found page renders
AssertionError [ERR_ASSERTION]: Expected the DESIGNED lamad not-found page to render
  (data-testid="lamad-not-found"); URL is "https://alpha.elohim.host/lamad/lamad/path/foundations-christian-technology"
```

The doubled `/lamad/lamad/` prefix is **intentional** in the test (legacy
doubled-prefix URLs are no longer minted per §12.3 but must still degrade
gracefully): the doorway fallback should serve the bundle and Angular's router
should fall through to `**` → the DESIGNED not-found page. NOTE: the test expects
`data-testid="lamad-not-found"`, which does NOT yet exist anywhere under
`app/elohim-app/src/app/lamad/` — the designed not-found page is the deferred
gap `#6-2` work captured in `epr-routing-complementary-captures.md`
("LamadNotFoundComponent → designed gate experience"). So this fingerprint has a
compound shape (see Root cause).

**Occurrence evidence:** ledger `seen: 1`, `first_build: 1105`, `last_build: 1105`
— but BOTH signatures are also present in build **#1104** (identical assertion
text + step line numbers `:146`, `:236`). Onset bounded: build **#1102** clean,
#1104/#1105 failing. (#1103 ABORTED — lossy 0-failure measure, museum trap #1.)

## Verdict

**Real regression** for the served-title failure (deterministic, reproducing
#1104→#1105, clean at #1102). The doubled-URL failure is **compound**: a genuine
routing behavior question (does `/lamad/lamad/...` still serve the shell + fall
through to `**`?) PLUS a not-yet-built designed page the scenario asserts on.
Neither is a flake; neither matches a museum trap (the four traps are
infra/measure-class).

## Root cause (suspected — onset window)

Same onset window as the portal-handoff concern (#1102 clean → #1104 failing),
same suspect commit cluster (doorway/EPR Slice-2/Slice-3, Jun 5–6):

- `bcf1b7a28` universal `/epr/{id}` address — reserved prefix + root-bundle dispatch
- `385a7485a` Slice-3 dispatch — alias 302 at B13, claims-aware `/epr/{id}` resolver, `/sitemap.xml`
- `5ccbbd577` fix(doorway): journal derivative cards stop minting pillar mount URLs (§12.3)
- `28773e763` anon-reach coherence — {commons,public} readable set, single authority

For the served-title (`606fa2a22a53`): the doorway root/dispatch reshuffle most
likely changed what `/` resolves to on alpha (serving a non-app shell titled
"Welcome"). For the doubled-prefix (`a969e96c4361`): the §12.3/§12.4.2 doorway
fallback behavior for legacy doubled prefixes interacts with the new reserved-
prefix dispatch; the designed not-found page the scenario targets is also still
unbuilt (gap `#6-2`). The foreground fix owner with the live repro should
disentangle: (a) restore the doorway root + doubled-prefix fallback so the shell
is served and Angular reaches `**`, then (b) the designed-page half is gap `#6-2`
work (NOT a CI fix — see Current decision).

Product-behavior regression; matches NO museum entry. No museum citation.

## Current decision

`ci_status: in-progress`. **The foreground (main) session is actively driving the
routing/navigation fix as of 2026-06-07.** This run canonicalizes + supplies the
evidence base only; no code landed (coordination — avoid racing on shared
doorway/routing files). The served-title and doubled-prefix-fallback halves are
the routing regression the foreground owns. The "designed not-found page"
(`data-testid="lamad-not-found"`) half is pre-existing deferred scope tracked in
`epr-routing-complementary-captures.md` gap `#6-2` (the §6 gate-face UI deferred
out of Slice-3) — if the foreground restores the fallback-to-`**` behavior but
the designed page is still unbuilt, fp `a969e96c4361` stays red until gap `#6-2`
lands; in that case re-scope this entry to `blocked` pointing at gap `#6-2`.

Ledger left `status: open` (no `triaged_at_build` stamp) — no fix landed here.
The integrator/foreground commit carries the fix; the sweep confirms by
disappearance (genesis green-streak ≥3, no recurrence).

## Fix trail

- None landed in this triage run (foreground-owned by coordination).
- Recurrence reference for the sweep once a fix lands: `last_build` at triage = 1105.
- Cross-link: `epr-routing-complementary-captures.md` (gap `#6-2`, LamadNotFoundComponent designed gate experience) for the not-yet-built designed-page dependency.
