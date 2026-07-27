# Handoff 2026-07-27 (evening) — Sprint: heads converge reliably + truthful EPR resilience on the p2p dataplane

**Objective (operator's words):** take another large bite out of the resiliency
saga — "I really want these heads to converge reliably AND hold truthful
representation about any EPR's resilience on the p2p dataplane."

That is two workstreams with one shared finish line: **both doorways tell the
same, non-zero truth about `elohim-host-landing` — and keep telling it across a
deploy restart.**

---

## Where the substrate stands tonight (all verified live, not inferred)

- **Relay plane healed.** The coturn pod-CIDR carve-out (31cd4aa9f) killed the
  `403 Forbidden IP` class; adam's heal sweeps complete instead of aborting on
  15s timeouts. Residual `400 Bad Request` class is characterized and
  upstream-only (coturn 4.6.2 handler asymmetry on expired standby-leg
  allocations — see backlog `turn-relay-pod-cidr-carveout-port-pool-shem-leg.md`).
  Do NOT chase it as a manifest fix.
- **Sync plane cured and measured.** The Freenet NOW slice deployed (edge
  #1246): digest round opener + counted `InSync`. `elohim_sync_in_sync_total`
  fires on all 7 pods; per-pod enumeration fell ~16-23k → ~1.8-5.1k docs/min.
  Spine `sync-scale-honesty` is GREEN. Guard: a flat-zero `in_sync` counter or
  a second opener construction site is a regression.
- **Edge #1246 baseline: 51 scenarios — 7 failed / 7 pending / 36 passed.**
  The 7: caughtUp-false (elohim.host), identity-fill 0 (ch02), NULL
  agent_pub_key ×7, E2E_DOORWAY_BETA env, and THREE divergence reds that are
  this sprint's target: EPR canonical head diverges (A `uhCkkiKDV…` vs B
  `uhCkkl4C9…`), DHT anchor diverges (same pair), stewardingCollectives 1 vs 0.
- **The resiliency card on elohim.host "lost its household" tonight** — it
  didn't. B never had `household-dowell` (0 log mentions in 3d; its resilience
  projection empty since 07-23). While B was degraded it served A-derived
  truth; healthy again, it truthfully serves its own empty projection. The gap
  was always there; B's recovery made it visible. ch10's felt-safety scenario
  (`resiliency-saga.steps.ts:517`) caught exactly this in the same build.

## The RCA that scopes this sprint (read first)

`genesis/data/timeline/backlog/content-divergence-unhealable-without-canonical-heads.md`
— four findings, all evidence-backed:

1. Divergent anchors are **structurally un-healable by heal** (fills-never-moves,
   by design). Only `StampMode::Declare` via `POST /db/content/{id}/canonical-head`
   moves a head. The ~4,300-row corpus has NO canonical heads; fleet-wide
   `refused_stale` = 0 (the forward-move path never fires).
2. The ch06 gate `divergentAnchor <= 100` thresholds a **rotating-window
   sample** (oscillates 0↔3,500 on an unchanged pod). It flapped green in
   #1246. Don't trust it either way; re-spec is on the ceiling menu.
3. Identity-fill 0 (ch02) needs per-member household formation on alpha —
   operator ceiling, not code.
4. **Collectives (and the resilience snapshot's joins) have NO reconcile arm.**
   `ProjectionInventory` serves only `rea_commitments` + `content`
   (`projection_reconcile.rs:36-38, 70-72`). Authoring conductor keeps the
   rows; no peer can ever acquire them.

---

## Workstream 1 — heads converge RELIABLY (ch06 / spine `notary-authority`)

**The crux to investigate FIRST (systematic-debugging, before any design):**
after tonight's restart, B **re-authored its own head** for
`elohim-host-landing` instead of adopting A's declared head. Divergence
re-opens on every deploy — that is the opposite of "reliably." Find the
boot/author path that creates B's independent root (suspects: SSR blob
materialization on doorway start; `authorHeadOnce` only running in the App
pipeline, which a storage-only deploy never triggers; declare-carries-Record
landing on A but B not consulting the DHT canonical-head record before
authoring). The reliable cure is almost certainly **adopt-before-author**: on
boot/materialize, resolve the DHT canonical-head record
(`gather_canonical_head_record`, `content_store/src/lib.rs:2874-2907`) and
adopt it when present + provably newer, author only when none exists. That
makes convergence a property of the substrate, not of pipeline choreography.

Second leg (operator-gated, mechanics ready): the **bounded declaration sweep**
— matthew's ~513 `refused_declared` ids through the existing declare route
(pattern: `scripts/ci/stage-spa-blob.sh:118-131`; auth currently god-mode
open). Validate the falsifiable prediction on ONE id first: heal flips
`Refreshed → Stamped/healed` within ~2 sweeps of the link gossiping. If it
shows `refused_stale` instead, the ordering-proof gate is the real blocker —
stop and report. Pace it: eve/susan/gertrude are wedged (100%
timeout_exhausted) and adam degrades under load.

**DoD-1:** `Then EPR "elohim-host-landing" resolves the same canonical head
across peers "alpha-A" and "elohim.host"` AND the DHT-anchor-same step
(`resiliency-saga.steps.ts:574`) green in Dataplane Validation — and still
green on the NEXT deploy after (the restart-survival proof is the point).

## Workstream 2 — truthful resilience for ANY EPR (ch10 / the card)

Build the **collectives + resilience-snapshot reconcile arm**: extend the
content arm's inventory/stamp machinery (`ProjectionInventory`,
`classify_content_gap` at `projection_reconcile.rs:1451`, stamp modes at
`content_diesel.rs:975-1145`) so the tables feeding
`household_resilience::snapshot_with_staleness_secs` reconcile across peers —
collectives first, then whatever shard-contract/commitment joins the snapshot
needs (enumerate them from the handler `get_household_resilience`,
`http.rs:12095` region). Gates and rails:

- **p2p-design-gate before designing** — classify each entity (Collective has
  DHT entries + `CollectiveCommitted` signals already; the projection arm is
  A2-shaped derivation, not a new truth source). No new slug identities.
- **Heal-exemption rule** (now in `sync_round.rs` module doc): the reconcile
  path must never sit behind an admission/budget gate.
- **Dead-config lint**: any new knob gets a watchlist entry in
  `scripts/ci/dead-config-lint.sh` in the same commit that introduces it.
- The arm must handle the NULL-`collective_cid` seed rows honestly (they are
  pre-coherence rows — reconciling garbage everywhere is worse than a gap;
  decide filter-vs-heal explicitly and write it down).

**DoD-2:** `Then peers "alpha-A" and "elohim.host" report the same non-zero
stewardingCollectives for "elohim-host-landing"` green (the scenario already
exists and is red — story-first is pre-satisfied), and the rendered card on
https://elohim.host shows the Dowell Household again — verify with `pnpm look`
(eyes-first), not just the API.

**DoD-overall:** edge Dataplane Validation strictly improves on the #1246
baseline (≤ 6 failed, no new failure names), `elohim_sync_in_sync_total` still
climbing (don't regress yesterday's cure), full storage suite + clippy + fmt
green, one push per batch.

---

## Rails (unchanged, non-negotiable)

- shem WAN is still down — **nothing may depend on cross-node/@requires:shem**;
  household-nodes + alpha (A/B doorways) are the floor.
- Cluster ops are operator-owned: no kubectl; repo manifests + pipeline only.
- elohim-storage keeps `RUSTFLAGS='--cfg getrandom_backend="custom"'`;
  `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev`.
- Commit path-limited in the shared worktree; one push per batch, wait for
  builds COMPLETE; edge deploys restart conductors → every deploy re-opens
  adam's catch-up window (budget for it in verification timing).
- Jenkins result flips UNSTABLE at the first catchError while later stages
  still run — poll `building:false`, not `result != null`. P2P Simulation
  UNSTABLE = `docker-compose: command not found` (env-red, in both baselines).

## Ceiling menu (operator decisions, NOT sprint scope)

1. Authorize + pace the corpus declaration sweep (after the 1-id validation).
2. Per-member household formation on alpha (greens ch02 identity-fill).
3. Restart/right-size the wedged conductors (eve, susan, gertrude; adam
   arc-factor decision still open — `self-heal-adam-projection-catchup-exhaustion-full-arc.md`).
4. Re-spec the `divergentAnchor <= 100` windowed-sample gate.
5. shem WAN router port-forward, whenever the router cooperates.

## Probe kit (fastest truth checks)

```
# card truth, both doorways
curl -s https://elohim.host/api/v1/resilience/elohim-host-landing/household | head -c 400
curl -s https://doorway-alpha.elohim.host/api/v1/resilience/elohim-host-landing/household | head -c 400
# head divergence
curl -s "https://jenkins.ethosengine.com/job/elohim-edge/job/dev/<N>/consoleText" | grep -A2 "canonical head"
# heal outcomes (Loki uid "loki", Prometheus uid "prometheus")
sum by(pod,outcome)(increase(elohim_projection_heal_outcomes_total{stream="content"}[3h]))
# sync cure still alive
elohim_sync_in_sync_total   # nonzero + climbing on all 7 pods
```

Today's full context: backlog docs `content-divergence-unhealable-…`,
`turn-relay-pod-cidr-carveout-…`, `2026-07-27-anti-entropy-egress-baseline.md`,
plan `2026-07-27-freenet-now-slice-plan.md` (delivered), edge #1245/#1246 logs.
