# Chapter 7 of the resiliency-saga: custody is witnessed. A shard's custody state
# is not real until a peer OBSERVES and classifies it, not merely intends it. The
# operational-weave fold (elohim/elohim-facings/src/folds/operational_weave.rs
# CustodyClassCounts: none/shelved/stocked/stocked_warm/unknown/observed_lost,
# folded by observed_custody_class_counts()) already computes this classification
# and is unit-tested. The Prometheus gauge is also registered and emitted:
# `emit_custody_class_gauges` (elohim-storage/src/api/mod.rs:782) publishes all six
# `elohim_custody_class_count{class=...}` series.
#
# Trigger truth (discovered 2026-07-29): the sweep is 100% REQUEST-triggered —
# its only call site is the GET /api/v1/weave handler (api/mod.rs:282). It is NOT
# one of the periodic background sweeps its two sibling gauges ride (identity-fill
# has run_fill_loop, custodian-capacity has run_report_loop; custody-class has no
# loop). Nothing on alpha calls /api/v1/weave — no CI scenario (the one feature
# that does, resilience/operational-weave.feature, is fully @wip and filtered out
# of Dataplane Validation) and no app consumer — so the series had ZERO samples
# in Prometheus retention until a manual probe fired it. Hence the When step
# below: issue the same GET a real WeaveView consumer would, then poll.
# The active-local-session gate is NOT a blocker on alpha: genesis self-heal
# (GENESIS_SELF_HEAL_IDENTITY=1) mints an ambient session at boot.
#
# Proof signal: elohim_custody_class_count{class="stocked"} >= 1 — matching the
# existing elohim_* custody-flow naming convention (elohim_custody_announce_total
# already exists with a "direction" label per the same custody-announcement flow
# in metrics.rs).
#
# New glue (steps/dataplane/resiliency-saga.steps.ts): the label-aware metric step
# (chapter 2 also uses it) — elohim_-prefixed metrics live on elohim-storage's own
# /metrics, not the doorway's port-8080 /metrics, so this step routes storage-owned
# metric names to the direct storage URL the same way the existing bare-metric step
# routes p2p_/reconcile_/dedup_ prefixes, extended to cover elohim_ too.
#
# Station discovered mid-flight (2026-07-29) — between sweep-fires → stocked>=1,
# missing node: shard→blob resolution + custody commitment. With the sweep firing,
# the fold honestly reports unknown=32, stocked=0 on alpha-A: derive_class
# (services/custody_facing.rs:332-364) sends a shard to `unknown` when no
# shard_manifests row maps its shard_hash to a blob_hash — true for ALL 32 of
# matthew's shard_locations rows — and `stocked` further needs an active
# rea_commitments row (action='custody-blob', state='active') naming the peer.
# Probe: /api/v1/weave then /metrics; current state: sweep fires (station green
# once the When lands), shard_manifests resolution empty (red), custody-blob
# commitment absent (red). The finish line stays exactly where it was.
#
# Station named at RCA (2026-08-18) — the commitment station's red is ROTATION:
# active custody-blob commitments exist (seeded 2026-08-15) but classify the
# blob hashes of THAT DAY's bundles; every SSR-bundle redeploy rotates
# content.blob_hash/server_blob_hash underneath them, and nothing re-stated the
# pledge (the seeder's own comment declares that reconcile pass out of scope).
# Cure: services/custody_rotation.rs — level-triggered rotation pass (5min tick,
# CUSTODY_ROTATION_ENABLED default on) authors the successor via the notarized
# create-then-ACTIVATE path (DNA create stamps state="created"; the fold only
# reads state='active') and retires the predecessor through the projection-layer
# supersession ceremony. Probe: rea_commitments shows the successor active with
# metadata.origin="rotation" and the predecessor state='superseded'; then this
# scenario's gauge assertion. Residual (separate node, not this finish line):
# location rows under superseded shard hashes keep class="unknown" high (1543
# on alpha-A at RCA) until peer-announced orphan hygiene exists.
@e2e @dataplane @concern:saga-07-custody-witnessed @act:i
Feature: Chapter 7 — custody is witnessed
  A household entrusts its content — a family's writings, a learner's record —
  to peers who PROMISE to hold the bytes. Until some peer OBSERVES that holding
  and classifies it, the promise is unfalsifiable: availability can rot silently
  and no one is entitled to reassurance. The resiliency saga proves, chapter by
  chapter, that the dataplane's availability story holds under real operational
  conditions; this chapter is its custody-observability leg.

  "Witnessed" here is a real chain, not a metaphor: peers record custody
  observation rows (self-held / verified / announced possession evidence); the
  operational-weave fold joins each observed shard-holder pair against the
  holder's ACTIVE custody promise and classifies it — none, shelved, stocked,
  stocked_warm, unknown, or observed_lost; and the storage peer "alpha-A" (the
  author-side storage node of the alpha deployment pair) publishes those counts
  as a labeled Prometheus gauge. Reading "/api/v1/weave" is not a passive fetch:
  that request IS the trigger that runs the sweep and populates the series.

  "stocked" is the strongest honest class — an active promise PLUS
  locally-witnessed possession of the promised bytes. One stocked shard proves
  the whole witnessing mechanism end-to-end (observation -> promise join ->
  classification -> visible gauge); full-coverage classification of every
  observed shard is a subsequent concern, tracked in the header notes. In
  particular, observed shards left under superseded deployments' hashes remain
  honestly "unknown" — that residual is a separate hygiene chapter, not this
  finish line.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: At least one shard is witnessed as actively stocked
    When I read "/api/v1/weave"
    Then labeled metric "elohim_custody_class_count" with label "class" "stocked" on peer "alpha-A" >= 1
