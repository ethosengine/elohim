---
title: Genesis scenario failures — two in-repo code leads (seeder upsert + non-admin authz)
status: open
ci_status: open
severity: medium
discovered: 2026-07-06
discovered_by: shift/overnight-genesis-pipeline-stabilize
domain: content-pipeline / auth
pipelines: [elohim-genesis]
requires_env: alpha-cluster-6peer  # verification needs a genesis run against a HEALTHY alpha
---

## Context

The genesis API suite's UNSTABLE baseline (15 failed / 149, build #1258) is dominated by
substrate/seeding/recurring-flake classes (see the substrate-gated ceiling item). But two
failures look like genuine in-repo code defects worth a dedicated pass **once the alpha
substrate is healthy enough to give a trustworthy measurement** (blocked today by adam
arc-saturation → alpha.elohim.host 503).

## ⚠️ Lead 1 CORRECTED — the "create-not-upsert" hypothesis was wrong for the pipeline path

A source trace (2026-07-06) overturned the original one-line hypothesis. The literal
create→skip-without-update bug **exists**, but in code the genesis pipeline **does not run**.
The real reach-floor-403 cause is a *reconciliation-transport* failure, and affinity×3 is a
**different subsystem** entirely. Recording all three surfaces so the fix lands on the right one.

| Framing | Verdict |
|---|---|
| seeder calls `create_content`, skips on "already exists", never updates | **True only in `genesis/seeder/src/seed-production.ts:449-459`** — a standalone script NOT referenced by any `package.json` seed target and NOT imported by `seed.ts`. Fixing it (add `update_content` fallback) is a 2-line change but almost certainly doesn't touch the failing genesis job. |
| stale reach persists because no update path exists | **Wrong** — `update_content` exists at every layer (zome `content_store/src/lib.rs:2463`; storage `content_diesel.rs:547`, exposed as `PATCH /db/content/{id}`, which DOES write `content::reach`). |
| **manifesto reach-floor 403 = reach reconciliation routed through the conductor, circuit-breaks on bulk-only rows** | **ROOT (reach)** — see below. |
| affinity×3 = missing multi-steward allocation from a create-skip | **Wrong subsystem** — see Lead 1c. |

### 1a — pipeline path (what actually runs)

`seed.ts` (the `"seed"` package script) POSTs `/api/db/content/bulk` →
`bulk_create_content` (`elohim/elohim-storage/src/db/content_diesel.rs:466-540`), which is
**strict skip-on-exists** (`if exists { skipped += 1; continue; }` — never UPDATEs; tags use
`insert_or_ignore`). So bulk create alone never reconciles a changed `reach`.

### 1b — reach IS reconciled, but through a failing transport (the real 403 cause)

The seeder already compensates for strict-skip: `stampProvenance`
(`genesis/seeder/src/seed-sqlite.ts:967`, called at 1303/1368) PATCHes `reach` per row, and its
own note (943-957) documents exactly the stale-reach gap. **But that PATCH routes through the
conductor and has a 5-failure circuit breaker (988-1032); for rows that were bulk-seeded and
never DHT-authored the conductor round-trip fails, the breaker trips, and the corrected reach
never lands** → manifesto row keeps `community` < `commons` → storage 403s anon reads.
So the bug is a *coupling* defect, not an absent upsert.
- **Bounded fix (architectural, verify on a healthy alpha):** make the reach reconciliation land
  on the **storage truth layer directly** — route `stampProvenance`'s reach PATCH to
  `PATCH /db/content/{id}` (storage `update_content` already sets `content::reach`), OR give
  `bulk_create_content` an `ON CONFLICT DO UPDATE` for `reach` only. Prefer the former: it
  decouples reconciliation from the conductor without changing bulk-create's skip semantics that
  other callers depend on for idempotency. This is the same "idempotency belongs at the truth
  layer" thesis as the P0 snapshot fix — the reconciliation must not depend on a lossy round-trip.

### 1c — affinity×3 is a DIFFERENT path (not content-create)

Multi-steward allocations live on the EPR graph node `epr_shefa`
(`elohim/elohim-storage/src/graph/schema.rs:51-57` — `stewards`, `allocations`), seeded by
`genesis/seeder/src/seed-stewardship.ts` → `POST /db/allocations/bulk`. The historical
value-scanner "matthew-only" cause (#1100/#1102/#1104) was a **pagination truncation** in the
existing-allocation diff and **already appears fixed** (`seed-stewardship.ts:342-390` now pages a
`for(;;)` loop). Residual risk: the allocations bulk handler counts an existing `(content,steward)`
pair as `failed` (uniqueness violation, no UPDATE re-weight) — genuinely-new stewards insert but
existing ones are never re-weighted. Content-create idempotency has **no bearing** here.

**Verification gate for all of 1b/1c:** needs a genesis run against a HEALTHY alpha. Blocked
today by the adam snapshot-amplification storm (alpha.elohim.host 503) — pick up once the P0
receive-side idempotency fix has drained the storm.

## Lead 2 — non-admin access not denied (conductor-visibility)

- **Symptom:** API scenario "non-admin access not denied" fails — an authorization gate is
  too permissive (a genuine logic defect, unrelated to seeding/substrate).
- **Next:** locate the conductor-visibility authorization check (doorway/storage), confirm
  it denies non-admin. This is a security-sensitive change — verify carefully; needs a
  genesis run to confirm the scenario flips green.

## Why blocked tonight

Both need a genesis run against a HEALTHY alpha to get a trustworthy pass/fail delta. Alpha
is currently substrate-degraded (adam arc-saturation → alpha.elohim.host 503), so any
verification is confounded. Pick up on a substrate-healthy shift after the arc-factor
ceiling item lands.
