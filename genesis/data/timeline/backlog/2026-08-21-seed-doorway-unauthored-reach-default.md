---
id: "backlog-seed-doorway-unauthored-reach-default"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The two seeders disagree on the reach an UNGRADED corpus row keeps (doorway seed public vs inverted-burden private)"
slug: "seed-doorway-unauthored-reach-default"
written: "2026-08-21"
author: "systematic-debugging of local-mesh code-reds #3-6 (reach-enforced-http / reach-commons)"
status: "open"
priority: "medium"
tags: [content, reach, seeder, corpus-grading, bounded-code-fix]
---

# One reach policy, two seeders — the ungraded default is still split

`earnedReach` (now `genesis/seeder/src/reach-resolver.ts`) implements the **inverted
burden**: an ungraded row defaults to `private` and rises only via an authored grade or
an account-package advisory. `seed-sqlite.ts` resolves through it. The doorway seed path
(`seed.ts`, what `pnpm seed` runs, and what seeds the fleet and the local mesh) does not:
it keeps `UNAUTHORED_CORPUS_REACH = 'public'`.

That literal was the *whole* of `seed.ts`'s reach policy until 2026-08-21 — the authored
grade was discarded too, which is the defect fixed in this commit. The remaining half is
the **default**, and it is deliberately left as `public` because flipping it is a
corpus-wide product decision, not a defect fix:

- 88 of 3431 content files carry an authored `reach` (49 `commons`, 38 `community`,
  2 `intimate`). 3 of 9 path files carry one.
- Flipping the ungraded remainder to `private` takes the anonymously-listable corpus from
  ~3400 rows to ~90 in one commit. Every a2o scenario that reads an arbitrary content row
  anonymously, and anonymous browsing in the app, changes behavior at once.

**To close:** finish the grading pass over `genesis/data/lamad/content/**` and
`genesis/data/lamad/paths/**` (an authored `reach` on every row that should be readable),
then delete `UNAUTHORED_CORPUS_REACH` and route `seedReach` through `earnedReach` so both
seeders share one policy. `src/__tests__/seed-doorway-reach.test.ts` pins the current
default so the disagreement cannot drift silently in the meantime.

**Evidence (local mesh, 2026-08-21).** All 1993 anonymously-listable rows in matthew's
`content.db` read `reach="public"`, including `manifesto` and `autonomous-entity-epic`
(both authored `commons`) and `bdd-smoke-tests` (ungraded). The reach gate itself is
sound and unconditional — an anchored row created at `reach=community` answered
`403 {"error":"Authentication required","requiredReach":"community"}` on storage :8090 AND
through the dev-mode doorway :8888 under `ELOHIM_NETWORK_STAKES=simulacra`. Stage-pricing
never enters `handle_db_content_by_id`; this was never a priced relaxation.

## Adjacent finding (separate fix location, not covered by this entry)

A reach-carrying `PATCH /db/content/{id}` answered **200 and silently did not apply the
field**: `PATCH {"reach":"community"}` on `community-garden-club` returned the full row
with `reach` unchanged and only `updated_at` bumped. `handle_db_content_by_id`'s PATCH
branch routes a notarized-field change through `update_via_conductor`, and on this row —
bulk-seeded, never DHT-authored — the re-notarization does not carry the new grade
through, but the response is a success. `reach-commons.feature` attributes the fleet's
stale grades to a circuit-breaking PATCH; on the local mesh there was no error to break
on. A write that reports success and changes nothing is the worse half of that gap. Fix
lives in `elohim/elohim-storage` (conductor write path), not in the seeder.
