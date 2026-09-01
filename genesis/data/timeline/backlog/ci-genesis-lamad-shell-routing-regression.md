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
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1108/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1105/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1104/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1522/
  - genesis/a2o/features/deployment/staging-validation.feature
  - genesis/a2o/features/lms/deep-link-delivery.feature
  - genesis/a2o/steps/ui/navigation.steps.ts
  - genesis/a2o/steps/lamad/deep-link-delivery.steps.ts
  - app/elohim-app/src/index.html
  - app/elohim-app/src/app/app.routes.ts
  - elohim/elohim-storage/src/db/rea_commitments.rs
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
(`features/lms/deep-link-delivery.feature:38`, `@browser-only`,
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

## Root cause (VERIFIED — foreground session, 2026-06-07)

NOT the Slice-2/3 dispatch commits (the original triage's suspect list was wrong)
and NOT gap `#6-2` (that claim was a false negative — see below). **Both
fingerprints share ONE root cause: alpha's doorway `EprRouter` is empty because a
single poisoned `rea_commitments` row fails the whole projection set.**

The chain, verified live and in code:

1. A `project-epr` commitment row on alpha has
   `in_scope_of = ["doorway:alpha-elohim-host|epr:elohim-host-landing"]` —
   **array-wrapped**, not the canonical bare pipe-string
   `doorway:…|epr:…`. Confirmed live:
   `GET /db/rea_commitments?action=project-epr&doorwayId=alpha-elohim-host`
   → `Internal error: Scope missing 'doorway:' prefix: ["doorway:alpha-elohim-host|epr:elohim-host-landing"]`.
   The wrapping came from a STALE storage binary (pre-`43951281f`, 2026-05-26)
   that replayed the DHT entry's `in_scope_of_json` (always a JSON array, zome
   `content_store` line ~11876) verbatim into the SQLite column instead of
   unwrapping via `first_or_none`. The `IfNotPresent`/`Always` stale-image
   window (`b23c86c26` era) is how an old binary ran long enough to poison.
2. `find_active_projections` (`db/rea_commitments.rs`) used
   `.map(commitment_to_projection_view).collect::<Result<_,_>>()` —
   **fail-closed**: the array-wrapped row's `parse_projection_scope` errored,
   so the WHOLE `collect()` returned `Err`. The LIKE filter
   (`%doorway:…|%`) still matched the wrapped string, so the bad row was in the
   set and poisoned every sibling.
3. `find_active_projections` → empty/Err → the doorway's
   `fetch_projections_from_storage` yields nothing → `EprRouter.replace_all`
   never populates (sitemap.xml empty `<urlset/>`, generation 0). Confirmed
   live: `GET /sitemap.xml` → empty urlset.
4. With an empty router: `/` has no `urlPath:"/"` projection → falls to
   `/threshold` → serves the doorway **operator dashboard** (title "Welcome")
   → **F3** (`606fa2a22a53`). And `/lamad` has no mount → the lamad bundle is
   never served → Angular never boots → its `**` route never renders → **F4**
   (`a969e96c4361`).

**Why "gap #6-2" was a false negative:** the original triage greps
`app/elohim-app/src` for `data-testid="lamad-not-found"` and finds nothing — but
`/lamad/lamad/path/…` is served by the SEPARATE `app/lamad/` bundle, whose
`lamad.routes.ts:208` `**` route loads `LamadNotFoundComponent`, whose template
(`app/lamad/src/app/components/not-found/lamad-not-found.component.html:1`)
ALREADY carries `data-testid="lamad-not-found"`. The designed page exists and is
wired. F4's app side was never broken — once the `/lamad` mount is restored, the
bundle boots and the not-found render-verifies. **No `#6-2` dependency.**

Product/deploy-data regression; matches NO museum entry. No museum citation.

## Fix (LANDED — foreground session)

Defense-in-depth in `elohim/elohim-storage/src/db/rea_commitments.rs`:

1. **`parse_projection_scope` heals legacy shapes** — a leading `[` is parsed as
   a JSON array and unwrapped: `["doorway:X|epr:Y"]` → `doorway:X|epr:Y`, and the
   pre-`66f16ab5e` two-element form `["doorway:X","epr:Y"]` → `doorway:X|epr:Y`.
   The existing poisoned alpha rows now serve correctly on read (no reseed
   required, though a reseed also canonicalizes them via `upsert_with_anchor`).
2. **`find_active_projections` degrades per-row** — `collect::<Result>()`
   replaced with `filter_map` + `tracing::warn!`. One unparseable row now costs
   exactly one projection, never the whole router. This is the durable fix: a
   future poison of any shape can no longer empty the EprRouter.

6 new tests (all green; full `db::rea_commitments::` suite 42/42):
`parse_projection_scope_{accepts_bare_pipe_string,heals_array_wrapped_pipe_string,heals_two_element_legacy_array,rejects_garbage}`,
`find_active_projections_{skips_unparseable_rows_instead_of_failing_set,serves_array_wrapped_legacy_row}`.

F3 needs NO app change — `app/elohim-app/src/index.html:5` already titles
`elohim.host …`; restoring the `/` mount serves the shell and the title follows.

## Current decision

`ci_status: in-progress` → fix `f38be2635` landed, now **merged to `dev` AND
deployed to alpha** (commit-only; integrator pushed). On deploy of the fixed
storage binary, alpha's EprRouter repopulates by healing the array-wrapped rows
on read; F3 + F4 both clear. **Immediate-recovery option for the operator**
(without waiting on a full edge rebuild): reseed the `project-epr` commitments
on alpha (writes bare-string `in_scope_of` back via `upsert_with_anchor`), which
the CURRENT deployed binary can already parse.

**Deploy/confirmation status (reconciled 2026-06-09):** `f38be2635` landed on
`dev` 2026-06-07 19:02Z. The fixed storage binary reached alpha via the
**elohim#1522 redeploy (2026-06-09 12:04Z)**. Since #1108, NO genesis build has
re-run these E2E scenarios against the redeployed alpha to confirm: **#1109
ABORTED** (museum trap #1 — lossy 0-failure measure, no signal) and **#1110 ran
0 a2o scenarios** (`Findings: 0 (scenarios: 0)` for both cucumber profiles — no
signal, NOT a green confirmation). The failures' absence since #1105 is the
museum-trap-#1 "no signal" gap, not yet a confirmed disappearance.

Ledger stamped `status: triaged` + `triaged_at_build: 1105` for both
fingerprints (each's own `last_build` at triage — the sweep's recurrence
reference). The harvester's disappearance-sweep is now armed: it confirms by a
genesis green-streak ≥3 with no recurrence of `606fa2a22a53` / `a969e96c4361`
once a real E2E run lands against the #1522-deployed alpha. NOT stamped
`decompose_on_confirm` — the EprRouter-empties-on-one-poisoned-scope-row lesson
is already gospel-tier memory (`project_epr_router_empties_on_poisoned_scope`)
and museum-worthy (a fail-closed `collect()` over a stale-binary-poisoned row
emptying the whole router), so this graduates-then-decomposes rather than
auto-deleting.

Sweep confirms by disappearance (genesis green-streak ≥3, no recurrence of
`606fa2a22a53` / `a969e96c4361`). Recurrence reference: `triaged_at_build` = 1105.
