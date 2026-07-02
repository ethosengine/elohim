---
id: "backlog-alpha-manifesto-content-403"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Alpha reach-drift: 11 landing marquee destinations 403 on anonymous /db/content/{id} body read — repo seeds now say commons, alpha rows don't"
slug: "alpha-manifesto-content-403"
written: "2026-06-10"
updated: "2026-07-02"
author: "claude (seed-data coherence pass, elohim.host landing redesign)"
status: "refined"
priority: "medium"
tags: [reach, alpha, doorway, seed-data, landing-page, epr-head, reach-drift]
relatedNodeIds:
  - "genesis/data/lamad/content/manifesto.json"
  - "genesis/data/lamad/content/value-scanner-epic.json"
  - "genesis/data/lamad/content/autonomous-entity-epic.json"
  - "genesis/data/lamad/content/governance-epic.json"
  - "genesis/data/lamad/content/social-medium-epic.json"
  - "genesis/data/lamad/content/economic-coordination-epic.json"
  - "genesis/data/lamad/content/public-observer-epic.json"
  - "genesis/data/lamad/content/quiz-who-are-you.json"
  - "genesis/data/lamad/content/concept-path-forward-policymakers.json"
  - "genesis/data/lamad/content/concept-path-forward-developers.json"
  - "genesis/data/lamad/content/concept-path-forward-communities.json"
cites:
  - genesis/docs/superpowers/specs/2026-07-02-epr-resolution-provider-design.md
---

## Widened scope (2026-07-02) — this entry originally tracked manifesto alone

The 2026-06-10 papercut probe caught one 403 (`/db/content/manifesto`, anonymous). A
2026-07-02 pass auditing seed reach for the redesigned elohim.host landing (commit
`095694ed0`, which renders live EPR reference cards for 14 destinations) reproduced
the same 403 verdict against **11 of those 14** anonymous `/db/content/{id}` reads:
the six epics (`value-scanner-epic`, `autonomous-entity-epic`, `governance-epic`,
`social-medium-epic`, `economic-coordination-epic`, `public-observer-epic`),
`quiz-who-are-you`, the three `concept-path-forward-{policymakers,developers,
communities}` nodes, and `manifesto` itself. This is one class, not eleven separate
papercuts: every affected row is a **public marquee document** the landing links to,
and every one of them resolves an anonymous-safe `/epr-head/{id}` `200` (title +
reach visible) while the **body** read 403s. That head/body split is real and by
design (heads are anonymous-safe, bodies are reach-gated) — the bug is that these
rows' *reach* landed too closed for content whose whole purpose is public marquee
visibility.

## Root cause, disambiguated per row (repo-seed audit, 2026-07-02)

The 2026-06-10 entry asked "decide which side is lying" (seed vs. client fetch). Per
row, it's the seed side — but two different failure shapes:

1. **`manifesto` and `evolution-of-trust`**: the repo seed JSON already declares
   `"reach": "commons"` (the most-open value in the ordinal). Alpha's live row 403s
   anyway. This is **pure deploy-side drift**: a DNA-content-style gap where the
   already-seeded alpha row predates (or wasn't re-seeded after) the seed's `reach`
   field landing — the seeder's `reach` PATCH-reconciliation path (`seed-sqlite.ts`
   `reconcile reach onto already-present rows`) only fires on re-seed, and alpha
   hasn't been re-seeded with it. **No further repo change closes this leg** — it's
   an alpha re-seed/PATCH action.
2. **The other 9 rows** (six epics, `quiz-who-are-you`, three
   `concept-path-forward-*`): the repo seed JSON carried **no `reach` field at all**.
   `genesis/seeder/src/seed-sqlite.ts`'s `earnedReach()` resolves an absent authored
   value under an *inverted burden* — default `private`, raised only by an
   account-package advisory (content type) or nothing at all (path type,
   `elohim-protocol.json`, which also lacked `reach` and got no advisory boost). Per
   `genesis/data/account-packages/*.json`, the advisories these particular ids
   collected landed anywhere from `familiar` to `community` to (for the
   concept-path-forward trio) `commons` depending on which human's package
   contributed the max — never a *guaranteed* commons. **This was a genuine repo-seed
   gap, not only deploy drift** — fixed 2026-07-02: all 9 rows (plus the
   `elohim-protocol` path) now carry an explicit `"reach": "commons"` in their own
   seed JSON, which `earnedReach`'s most-open-wins reduce makes authoritative
   regardless of any account-package advisory.

## What's still open (alpha-side, operator-executed)

Adding `reach: "commons"` to the repo seed does **not** retroactively fix the rows
already sitting in alpha's content table — per the DNA/content redeploy gotcha
(CLAUDE.md "Deployment Contexts"), seed-data changes don't reach a running conductor
until something re-seeds or PATCHes the row; a normal edge redeploy doesn't rewrite
existing content rows. Closing this class requires either:

- a full re-seed of alpha (`--use-account-packages` reconciliation now picks up the
  authored `commons` on all 11 rows), or
- a targeted `reach` PATCH against the 11 ids (the reach-reconciliation PATCH path
  `seed-sqlite.ts` already carries for exactly this drift shape).

Both are **alpha-cluster actions the operator executes** — never `kubectl`, never a
live-seed run from this dev environment (see CLAUDE.md "Cluster ops are
operator-owned"). The repo-side half of this fix (seed audit + `reach: commons` on
all 14 landing destinations) is done; this entry stays open, scoped to the alpha-side
reach repair, until an operator re-seed/PATCH lands and a fresh anonymous probe of
the 11 ids comes back non-403.

## Why the landing itself isn't blocked on this anymore

`2c3271997` ("head-first link resolution with typed degradation") moved
preview/reference-card resolution from the reach-gated `GET /db/content/{id}` to the
anonymous-safe `GET /epr-head/{id}` — the landing's reference cards render title +
reach from the head regardless of this drift, with typed degradation
(`resolved | forbidden | missing | error`) instead of a silent raw-string fallback.
**This entry no longer blocks the landing surface.** What it still blocks:
click-through — a learner who opens one of the 11 cards to read the actual document
still hits the reach-gated body endpoint and gets a 403 until the alpha-side repair
above lands. See `genesis/docs/superpowers/specs/2026-07-02-epr-resolution-provider-design.md`
for the full head/body split design and increments 2-3 (ambient provider,
manifest-declared route claims) this entry composes with.

## Original 2026-06-10 finding (superseded scope, kept for provenance)

The original probe: `look`'s `httpErrors` capture (records HTTP >=400 responses that
neither console-error text nor `requestfailed` surface) caught an anonymous render of
`https://doorway-alpha.elohim.host/` requesting `/db/content/manifesto` and receiving
403, alongside an unrelated 404 trio (`version.json`,
`/api/v1/epr/elohim-host-landing/nav-context`, `/wasm/elohim-cache-core/...`) that is
a **different shape** (missing-route/asset, not a reach verdict) and is not part of
this entry's scope. Evidence:
`genesis/a2o/reports/look/papercut-httperrors-probe/capture.json`. Also distinct: the
EprRouter poisoned-scope 404 lesson (memory `project_epr_router_empties_on_poisoned_scope`).
