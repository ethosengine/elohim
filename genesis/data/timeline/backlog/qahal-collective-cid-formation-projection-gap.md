---
id: "backlog-qahal-collective-cid-formation-projection-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "household-formation E2E — collective_cid never stamped on family-dowell: CollectiveProjected signal path needs ceremony/peer-subscription verification"
slug: "qahal-collective-cid-formation-projection-gap"
written: "2026-06-08"
author: "tackle-top-three investigation (wf_e3cc3753-f1a + follow-up agent)"
status: "backlog"
priority: "high"
jobs: [elohim-genesis]
ci_status: blocked
fingerprints: [afd13ee9d0bc]
relatedNodeIds:
  - "backlog-seed-provenance-anchor-gap"
tags: [qahal, household-formation, collective-cid, reconcile-controller, dna-signal, e2e, app-scope, steward-gate, dht-integration-race]
cites:
  - genesis/a2o/features/qahal/household-formation.feature
  - genesis/a2o/steps/qahal-formation.steps.ts
  - genesis/seeder/src/seed-household-formation.ts
  - genesis/seeder/src/seed-collectives.ts
  - elohim/elohim-storage/src/reconcile/controller.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1145/
---

# `collective_cid not stamped — formation projection has not run` is a SIGNAL-PATH gap, not the FK storm

Two scenarios fail in `features/qahal/household-formation.feature`: "All three
members are affirmed participants" and "The household collective is coherent —
family-layer, CID-stamped". The 2026-06-07 investigation proved these do NOT
trace to the jessica-alpha FK storm (fixed by the stub-materialize change in
`do_account_import` `23b1ba135` — see
`participation_without_parent_fk_fails_and_stub_materializes`).

**SEPARATE from the provenance gate — do NOT merge.** This is the
`CollectiveProjected` DNA-signal → reconcile-controller `collective_cid` stamp path
(`controller.rs:760-783`/`816-837`), a distinct mechanism from `require_provenance`
content reads. It is *why household-formation x2 persist despite* the FK fix
`23b1ba135`. The `relatedNodeIds` link to `seed-provenance-anchor-gap` is a
sibling-context pointer (both are alpha-stack/seed-coherence reds surfaced in the same
#1104→#1106 sweep), **not** a sub-capture relationship.

**HOUSEHOLD-PROVABLE — not BLOCKED-BY-ENV (corrected 2026-06-08).** An earlier note called
this blocked-on-degraded-cluster; that was wrong. The feature is tagged
`@requires:household-nodes` (`cluster-state.yaml`: `available: true`) and it RUNS — and fails.
The whole signal path is **intra-household**: matthew's imagodei conductor → matthew's storage
peer's subscription (`main.rs:1861,1898`), all on `household-nodes`. No shem, no 6-peer soak,
no federation involved. The mesh is live (doorway-alpha /health, 2026-06-08: p2p peerCount 2,
conductor 4/4, discoveryComplete). So this is a **real household-substrate bug to fix on the
stable architecture**, not an env-block to hold — exactly the kind of deep-on-the-floor bug the
household triad exists to surface. The verification questions in §"To verify" run against the
healthy household now; they do not need shem.

## Mechanism (evidence-backed, medium confidence on the live trigger)

- `collective_cid` is stamped ONLY by the reconcile controller's
  `on_collective_projected` handler (`controller.rs:760-783` slug-alias merge;
  `:816-837` fallback create+stamp), driven by a `CollectiveProjected` DNA
  signal from the **imagodei conductor** the storage peer subscribes to
  (`main.rs:1861,1898`).
- The signal originates when `seed-household-formation.ts:510-524` calls
  `create_collective` on the FOUNDER's (matthew's) conductor.
- The stamp targets **`family-dowell`** (the charter's `slugAlias`), a
  DIFFERENT row from the `household-dowell`/community ids the account-import
  joins use. Fixing the joins cannot stamp the CID.

## To verify in the next CI run (live observation needed)

1. Does the household-formation ceremony actually run in CI, against
   matthew's conductor?
2. Does `E2E_STORAGE_URL` point at the storage peer that SUBSCRIBES to
   matthew's imagodei conductor (signal delivery), and did
   `seed-collectives.ts` seed `family-dowell` to that same peer (the
   slug-alias merge needs the row present)?
3. Does the ceremony's affirmation choreography project participations under
   the post-slug-merge `family-dowell` id (not a transient `collective:uhCkk…`
   id)? Otherwise the triad-participants scenario stays red even with the FK
   fixed.

## Riding coherence cleanup

Collectives app-scope is split: `/db/collectives` seed POSTs land under the
legacy `h_app_id="lamad"` (`http.rs` `extract_app_context` legacy-prefix
branch) while account-import participations use `qahal`. The FK is on bare
`collectives(id)` so this doesn't break inserts, but scoped reads
(`get_collective`) see inconsistent worlds. Decide ONE scope (tables default
`'qahal'`) and reconcile the seeder + legacy-prefix branch. The stub
materialization deliberately uses "lamad" today so the projection
controller's upsert converges instead of PK-colliding — flip both together.

**Sibling (2026-06-27):** this is the SAME CLASS as the humans-projection scope split
(`resilience-card-membership-humans-projection-gap-2026-06-19.md`), reconciled by the `HUMANS_HAPP_ID`
const in `genesis/docs/superpowers/plans/2026-06-27-humans-projection-scope-reconciliation-plan.md`.
The humans fix was **reader-side only** — production writers were already uniformly `imagodei`, so it
did NOT need writer convergence. Collectives are DIFFERENT: they ARE written under BOTH `lamad`
(seed/legacy-prefix) and `qahal` (account-import), so reconciling them needs the "decide ONE scope,
flip BOTH writers together" step above. It is the **next instance** of this pattern, NOT folded into
the humans fix.

## UPDATE genesis #1145 (2026-06-14): root cause is ONE RUNG DEEPER — formation never reaches a full collective; jessica's affirm is rejected at the STEWARD GATE

The 2026-06-08 framing ("the `CollectiveProjected` signal/stamp path never
fires") was correct that the FK fix doesn't stamp the CID — but #1145's
evidence (ci-investigator, build #1145) shows the projection never runs for a
more fundamental reason: **the collective is only ever 1/3 affirmed, so there
is no fully-formed collective to project.**

`Seed Household Formation` on #1145 (the upstream stages all GREEN — conductor
identities `3 existing/0 failed`, peer bindings `9 written/0 failed`, so this
is NOT the founder-binding concern, which now passes):

```
[+] collective created: collective:uhCkkSm6cpsyDek1n4v0vK4CWllVXoTLexLM5_ZYeVedwkMbP4kk3
[X] affirm_membership for human-jessica-spouse:
    Wasm runtime error … WasmError { file: "zomes/imagodei/src/qahal_coordinator.rs", line: 412,
    error: Guest("caller is not a current Steward of collective:uhCkk…") }
[X] human-james-son: no conductor bound — cannot affirm
=== Results: 1/3 affirmed, custody ok=0 fail=0 ===
WARNING: Household formation partial: affirmed=[human-matthew-manager]
```

Then `qahal-formation.steps.ts:45` (re-confirmed verbatim) reads
`family-dowell` from storage, finds `collective_cid` `undefined`, and fails
`afd13ee9d0bc` — because the partial collective never reached the projection
stamp path. (The `When I fetch the collective "family-dowell"` step PASSES —
the row exists from `seed-collectives`; only the CID field is unstamped — so
storage/signal reachability is fine, consistent with the 2026-06-08
"HOUSEHOLD-PROVABLE, not blocked-by-env" reading.)

### The steward-gate circularity (read from the zome)

`affirm_membership` (`qahal_coordinator.rs:699`) requires the invite token's
**issuer/sponsor** to be a current Steward of the collective — step 2,
`require_caller_is_steward_of(&issuer_cid, &token.collective_cid)` at `:717`.
And minting the token via `issue_household_invite` (`:665`) ALSO requires the
issuer to be a current Steward (`:673`). `require_caller_is_steward_of`
(`:399`) traverses `HasMembership` links from the collective's ActionHash and
checks for a non-withdrawn `Steward` Membership matching the agent CID.

So jessica's affirm fails "caller is not a current Steward" because the
**founder (matthew)'s own Steward Membership is not yet DHT-visible** to the
steward-link traversal at the moment the invite/affirm chain runs for jessica —
the seconds-old founder-steward entry isn't integrated/queryable yet. This is
the **same `DepMissingFromDht` / "seconds-old identity entries aren't
DHT-integrated when formation commits against them" class** the founder-binding
concern named at its #1123 update (`ci-genesis-household-founder-binding.md`),
now manifesting one membership-rung deeper: founder binding itself now passes
(matthew IS affirmed), but the founder's *Steward* standing isn't yet visible
when his conductor issues/affirms the co-members' memberships. james failing is
expected (no conductor bound — household-only, not in the alpha conductor set).

### What this means for the concern

The CID-stamp signal path (the original 2026-06-08 framing) may well ALSO have
a gap — but it is **unreachable to verify** until formation reaches a full
(3/3, or at least the affirmable subset) collective. The mover is the
**seeder's formation choreography / steward-grant ordering**: it must ensure
the founder's Steward Membership is DHT-integrated (settle-retry on the
steward-link traversal, mirroring the founder-binding `4×15s DepMissingFromDht`
retry already landed) BEFORE issuing co-member invites — or issue the invites
from a path that doesn't depend on the not-yet-gossiped steward link. The
`steward-grant-fixture-surface.md` backlog is the sibling that owns making
steward standing reliably present for the seeder; this concern stays the home
for "formation completes and `collective_cid` gets stamped on `family-dowell`."

## Current decision

**BLOCKED — real household-substrate bug, needs a bounded `/shift` (not a
sentinel fix).** The fix is a seeder formation-ordering / steward-settle change
(+ possibly the signal-path verification once formation completes), a multi-file
change across `seed-household-formation.ts` and the affirm choreography with
DNA-integration timing — beyond a sentinel's bounded-fix mandate and needing a
live genesis run to verify the settle window. Named for the operator as a
`/shift` Objective: "get household formation to 3/3 (or the affirmable subset)
affirmed so the projection stamps `collective_cid` on `family-dowell`,"
pairing with `steward-grant-fixture-surface.md`.

Ledger `afd13ee9d0bc` → `status: blocked` (blocker: founder Steward standing
not DHT-visible when co-member affirms run → formation partial → CID never
stamped). **No `triaged_at_build`** (nothing landed this run). It disappears on
a green streak once the formation-ordering `/shift` lands and a genesis run
reaches a full affirmed collective with the CID stamped.

## Verification path (updated for #1145)

The 2026-06-08 "To verify" questions still hold, but are now GATED behind
reaching a full collective. The new first question: **does the founder's
Steward Membership become DHT-queryable (via `require_caller_is_steward_of`'s
link traversal) before the co-member invite/affirm chain runs?** Instrument or
settle-retry that, get to 3/3-affirmed (matthew+jessica; james held as
no-conductor), THEN re-check whether `collective_cid` stamps — if it still
doesn't, the original signal-path gap is real and isolated; if it does, the
steward-settle ordering WAS the whole concern.
