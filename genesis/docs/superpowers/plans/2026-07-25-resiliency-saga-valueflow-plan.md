---
title: "Resiliency Saga — executable eprfs valueflow driving the epic to live completion"
id: resiliency-saga-valueflow
tier: plan
status: Executed (CLAIMED — verify via CI dataplane runs + jenkins-sync)
created: 2026-07-25
maintainers: Matthew Dowell + Claude Fable 5
sprint: verify-track
requires_env: []
topic: [resiliency-saga, valueflow, epr-flow, fulfill, a2o, dataplane, custody, commitment, capacity, identity-fill, frontier]
cites:
  - "epr-rea-valueflow-fabric | EPR-REA ValueFlow Fabric | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md"
  - "sealed-contract-edges-governor-frontier | Sealed Contract Edges | sha256:ace1788fa44a293f | path: genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md"
  - "epr-durability-replication-arc-plan | EPR Content Durability Arc | sha256:f263ed845af2f916 | path: genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md"
  - "resiliency-card-p2p-weave-sprint-plan | Resiliency-card + P2P-sync + Operational-Weave sprint | sha256:834716e333f5b01f | path: genesis/docs/superpowers/plans/2026-06-21-resiliency-card-p2p-weave-sprint-plan.md"
  - "resilience-card-lighting-plan | Resilience card lighting | sha256:be6dfb65e5e8a433 | path: genesis/docs/superpowers/plans/2026-06-19-resilience-card-lighting-plan.md"
  - "experience-story-epr-design | Experience-Story EPRs | sha256:6a82cd4508e28a39 | path: genesis/docs/content/elohim-protocol/architecture/2026-04-18-experience-story-epr-design.md"
---

# Resiliency Saga — executable eprfs valueflow driving the epic to live completion

## Why

Every session restarted the resiliency story from symptoms (card zeros) because the story existed only as scattered epics and memories. A month of dataplane work missed that the story's Chapter 2 — household formation — had never executed once: the ceremony exited 1 on every genesis build (founder unbindable: doorway auth minted a UUID Human; james absent from a roster read from a file that never existed), so the imagodei DHT holds zero household memberships and every downstream identity join is honestly empty. Meanwhile the live commitment producer notarized replicates-* commitments every 60s into `mishpat_commitments` while every resilience reader queried `rea_commitments` — commitments were written the whole time, structurally invisible.

This plan (a) finished the custody production chain, and (b) converted the epic's narrative into a second recipe in the eprfs valueflow fabric: ten a2o chapter features as `scenario`-stage artifacts (each minting an Active `a2o:scenario-green` Commitment), with CI dataplane verdicts flowing back as REA fulfillment events via the new `epr flow fulfill` — the fabric spec's §5 joint-5 emitter that had never landed. Each session now reads a computed frontier (`SAGA resiliency: N/10 green · frontier: chNN …`) instead of re-deriving the story.

## The saga (stage table)

matthew boots a device → household forms → matthew uploads elohim-host-landing into his eprfs → hosts a doorway → adam co-stewards (rea-agreement, Mishpat-notarized) → their blobs sync to one head → custody witnessed honestly → each node reports capacity → in-kind doorway-operator agreements, projector caches carry the head → the resilience card tells the truth on elohim.host and alpha.elohim.host. (Anycast deferred.)

Canonical home: `genesis/a2o/features/dataplane/resiliency-saga/` (README carries the narrative + chapter/proof table; concerns `saga-01…saga-10` in the dataplane taxonomy). Recipe: `resiliency-saga` in `.claude/epr-meta/recipes.yaml`. Frontier reader: `.claude/scripts/saga-status.py` (session-start line) + `genesis/scripts/jenkins-sync.sh` (fetch report → project → fulfill; agent-side, since `.eprfs/` is per-checkout derived state — renamed/relocated 2026-07-25: the Jenkins-facing fetch is the bridge-shaped sliver, housed with the manual pipeline orchestration that all deletes together at rakia graduation).

## Tasks (executed 2026-07-25)

- [x] T1 — Review + commit in-flight cures: ceremony founder election + deployments.json roster (839a2f9b8), doorway human_id preference (b20a739f1), capacity reporter + identity-fill darkness observability (9b97db750)
- [x] T2 — mishpat→rea replication mirror (cid=entry_hash; Revoke cancels), `replication_commitment` fold in elohim-facings (provider_role tiering, never the humans.household_id join), three T15 stubs replaced, `load_custodian_relation` state filter (6eaaf4046)
- [x] T3 — `custody_facing` observation loader feeding the ae7c695b8 typed folds: evidence from `shard_locations.status`, commitment standing from active custody-blob rows, shard→blob via manifests; binding honesty rules (failure→unknown-preserving-evidence; StockedWarm only by recorded pledge; PeerAsserted never reaches Stocked); `elohim_custody_class_count{class}` gauges (6eaaf4046)
- [x] T4 — ten chapter features + label-aware Prometheus + runLook glue; born-red chapters 2/5/7/9/10 are the loop's work queue (89eb92c64)
- [x] T5 — `resiliency-saga` recipe + `epr flow fulfill` (Produce on all-green, Dismiss on regression, generatedAt-idempotent, discharged-set skip) (a581fee74)
- [x] T6 — `saga-status.py` frontier reader (green/emit-due/regressed/red/pending-env/unprojected; <0.1s), session-start SAGA line, `jenkins-sync.sh` actuator (born `.claude/scripts/saga-sync.sh`, relocated same day), 8 backlog items for deliberate v1 exclusions (35abe6f38)

## Verification ladder (the loop from here)

1. Next genesis build: `Seed Household Formation` exits 0/2 with affirmed ≥ 1 (was: FATAL exit 1).
2. `elohim_identity_fill_discovered_cids ≥ 1` then `elohim_identity_fill_total{action="created"} > 0` → ch02 flips.
3. `elohim_custodian_free_bytes > 0` per pod → ch08 flips.
4. `rea_commitments` gains mirrored replicates-* rows → ch05; `commitmentBackedReplication.totalPledgedBytes ≥ 1` → ch09.
5. Custody class gauges non-vacuous → ch07.
6. Both doorways report the same non-zero stewarding count → ch10 — the epic's finish line.
After each CI dataplane run: `genesis/scripts/jenkins-sync.sh` (or the delivery-stasis loop on `emit-due`) converts green verdicts into fulfillment events; `saga-status.py` shows the frontier.

## Deliberately out of scope (backlogged)

Recipe-edge enforcement in Rust · `walk <recipe-id>` · `epr flow status` perf (25.1s debug / 2.6s release, measured) · `.epr-meta flows:` slice-2 · regression re-commitment · per-scenario body CIDs in reports · Loki step primitive · committing/sharing flows.jsonl (fabric spec gap #11) · anycast · matthew's captured UUID chain (operator-scope migration) · commons Capacity producer policy · CollectiveStewardMode unblock (coordinator-zome change).
