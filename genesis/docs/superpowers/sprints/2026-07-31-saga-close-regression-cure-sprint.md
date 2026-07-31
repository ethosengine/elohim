---
title: "Saga-close regression-cure sprint — day session (2026-07-31)"
id: saga-close-regression-cure-sprint
tier: sprint-result
status: Open
created: 2026-07-31
maintainers: Matthew Dowell + Claude Fable 5
sprint: resiliency-saga
topic: [resiliency-saga, reconcile-convergence, pin-retirement, onpush-eager, resilience-cards, custody-reclaim, lamad]
---

# Saga-close regression-cure sprint — 2026-07-31 day session

**Objective:** cure the two operator-reported live regressions (lamad
content not resolving; resiliency cards not converging), dissolve the
blocker pinning saga ch06/ch09/ch10, and stage the tree for a clean
validate-only run recording 10/10 candidacy.

**Outcome at handoff: every root cause is pinned with evidence and every
bounded fix is committed locally (7 commits, gates green, not pushed).**
The overnight journal's "operator topology call" (hostAliases vs
pool-scoped CONDUCTOR_URLS) is **dissolved** — traced in code as a
mis-attribution; no topology decision is needed.

## The three RCAs (all evidence-verified, none matched the priors)

### 1. Lamad "content not resolving" — an OnPush root, not a data hang

The content API answered in 13ms the whole time. Angular 22 made OnPush
the implicit default; the lamad bundle's root component lost its
change-detection stamp in the v22 migration wave, and an unmarked OnPush
root skips the entire tree on every global tick. Diagnostic fingerprint
for next time: signal-backed components stay live while plain-property
components freeze ⟹ non-Eager root, not a data-layer stall. Vitest was
structurally blind (TestBed defaults to zoneless in v22); the new specs
mirror the browser (zone CD, autoDetectChanges, no post-mutation
detectChanges) and each fails with its stamp removed. Same latent class
found and stamped in the other three bundle roots (elohim-app,
doorway-app, imagodei-portal).

### 2. Resiliency cards — fossil holder keys, not joins, not the omnibar

The card read zero on BOTH doorways (not divergent). H1 (NULL
agent_pub_key) is real but secondary; H2 (omnibar endpoint mismatch)
refuted against the deployed asset. Actual cause: after a re-key
(non-prod DNA reinstall mints a new agent key), `manifest_backfill_pass`
could never re-claim already-manifested blobs under the live identity —
both nodes hold the bytes with a holder row under a key belonging to
nobody. Cure: honesty-gated re-claim arm in custody reconcile;
self-heals within one backfill tick of the next deploy.

### 3. Saga ch06/09/10 pin — two storage defects, zero topology

Prometheus (12h): `reconcile_converged` 0 on ALL 7 pods;
`heal_outcomes{missing}` 60k–155k per pod. Mechanisms:

- **Convergence starved by adjudicated divergence.** `converged` required
  `divergent_anchor == 0`, but the dominant class is rows heal is
  *forbidden* to move (refused_declared — canonical channels own them).
  Cure (Leg A): classify divergence actionable-vs-refused at discovery
  (same predicate as `SkippedDeclared`); converge on actionable==0; new
  `divergent_refused{stream}` series keeps the accounting honest; a
  cross-sweep MissLedger makes `exhausted` real and kills the
  ~1000-rows/sweep conductor-ask treadmill (hourly re-admission).
- **Pins never retire.** The only runtime status-flip was the HTTP DELETE
  route; exhausted pins re-scanned forever held `pull.caughtUp=false`
  (alpha-A: 73 total / 3 fetched / 70 failed). Cure (Leg B): `retired`
  status (no migration, no wire change), inventory-driven re-admission +
  6h cooldown, retry budget sized to the live fabric, retirement metrics.
- **CONDUCTOR_URLS red herring, corrected:** `pools_healthy` feeds only
  hosted-user admission routing; nothing the saga asserts consumes it.
  The backlog doc's framing is corrected (dated) via the susan item.

Plus: ch06 had a real false-green channel (absent `healed` label series
→ pending → converged assertion skipped) — closed with a `strictly` step
wording and step reorder.

## Commits at handoff (feat/angular22-node24, in order, not pushed)

| Commit | What |
|---|---|
| `d3b6c1a13` | CI triage: sophia mathquill allowlist canonicalized (fixed prior; verified in #146) |
| `9278a47e5` | a2o: ch06 false-green closed (strict labelled-metric + reorder) |
| `429e2f669` | storage: custody re-claim after re-key (resilience card fossil-key zero) |
| `d8ab176d0` | lamad: Eager root + 9 components + 2 browser-mirroring specs |
| `90abd6c83` | backlog: susan conductor-WS heal-storm; landing shard byte-divergence |
| `b19f12014` | storage Leg A: convergence actionable/refused split + MissLedger |
| `192f77cd0` | storage Leg B: pin retirement/re-admission + fabric-sized retry budget |
| `cd7b73472` | apps: Eager stamps on elohim-app/doorway-app/imagodei-portal roots + specs |
| `0f9b4145b` | CI: zero-scenario Dataplane Validation exits 3 → UNSTABLE, never SUCCESS |
| `0d0b830af` | storage: migration admitting 'retired' pin status (review-critical fix) |
| `18b5d4747` | a2o: saga-11 story carries its stake (blind-reader revision, 4 rounds) |

The saga-11 blind-reader loop surfaced one deliberately-unresolved item:
the reader's recurring ask to move narrative from comments into Gherkin
bodies is a saga-wide convention decision (all 10 sibling chapters carry
narrative in comments) — operator call, not a per-file fix.

New a2o coverage: ch06 honesty-guard scenario (divergence measured AND
converged); `11-pull-queue-retires.feature` (@concern:saga-11) registered
in both READMEs. Gates: storage `cargo test --lib` 2293 green, schema
contract 221 green, export_bindings doc-only drift; lamad vitest
2802/2802 + AOT build; a2o tsc/eslint/gherkin-lint clean; per-app suites
37/33/30 green with negative controls.

## Path to 10/10 (in order)

1. **Integrator pushes the batch** (one push; concurrent pushes mutually
   abort). Wave builds edge + DNA-adjacent + app pipelines.
2. **Lamad fix reaches alpha only via the App pipeline re-staging the
   `lamad-spa` blob** (edge redeploy alone will not move it).
3. **Post-deploy probes** (falsifiable, from the fix reports):
   converged→1 with divergent{content} still ≥1 and
   (divergent − divergent_refused)→0; exhausted>0 and missing-rate
   materially below ~3.5/s; `pull.caughtUp=true` with retirement series
   present; both doorways `stewardingCollectives ≥ 1` within one
   300s backfill tick; `pnpm look <host>/lamad --wait-testid
   home-path-grid` renders on both doorways.
4. **One clean validate-only edge run** (measurement decoupled from
   deploy restarts) records ch04+ch06+ch09+ch10 → 10/10 candidacy; then
   record the flows (the mid-churn report was deliberately never
   recorded).

## Still open (owned, not blocking the push)

- **susan conductor-WS dead** (live incident, shem node) + heal-pacing
  blind to instant errors — backlogged with code anchors; (a) may be
  operator-side, (b) is bounded code work.
- **Landing shard byte-divergence** across doorways (replication plane,
  third layer under two resolved prior docs) — needs its own RCA.
- **ch10 rendered-card scenario** remains @wip @browser-only — CI proves
  the API claim, not the pixels; candidate for a browser-lane follow-up.

## Review outcome (independent, adversarial)

The code review of the two reconcile commits found one CRITICAL, fixed
before handoff: `acquisition_pins.status` carries a CHECK constraint
(`active|paused|removed`) from its 2026-06-07 migration, so Leg B's
`'retired'` write was engine-rejected and debug-swallowed — the fix was
inert as committed. Cured by `0d0b830af` (table-rebuild migration
admitting `retired`, DB-level round-trip regression test, swallowed-error
sites bumped to `warn!`). Lesson, dated 2026-07-31: "free-form TEXT" is a
claim about the *column type*, not the *constraint set* — verify CHECK
constraints in the original migration before asserting no-migration.

Non-blocking follow-ups from the same review (both observable via the
new metrics): (1) REA arm's `divergent_refused` is hardcoded 0, so a
genuine cross-peer REA anchor divergence could hold `converged` false
from that stream alone — partition REA's exhausted set like content's;
(2) inventory-driven pin re-admission has no failed-provider/dwell gate,
so a peer that advertises but never delivers can flap retire↔readmit at
content-list cadence; (3) verify `drain_acquisition_queue`'s rotation
actually visits distinct peers so "budget = providers probed" holds.
