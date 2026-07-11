# RED-FIRST (2026-07-06): the inventory-gossip apply plane does not converge under
# WAN-adversarial conditions. A full-arc sink (adam, the alpha seed peer) re-applies
# full BlobInventorySnapshots non-idempotently — ~53 "Inventory snapshot applied"/sec
# (design cadence ≈ 0.1/sec, ~500× amplified) — pegging ~5.9 cores, starving the tokio
# runtime, so the conductor-composed surfaces on the doorway hosts (alpha-A / elohim.host)
# degrade and the projection never catches up.
#
# WHY THIS MATTERS: elohim is the resilient layer running OVER k8s/WAN. A P2P inventory
# plane that amplifies redundant gossip 500× and never converges is the layer failing to
# tolerate exactly the loss/reorder a WAN-native system must absorb. shem-over-WireGuard
# didn't cause it — it EXPOSED it (on-prem near-zero-loss adam just barely stayed ahead).
#
# THE FIX (receive-side snapshot idempotency): apply_snapshot dedups on a content
# fingerprint stored on peer_inventory_cursor.last_content_hash — a byte-identical snapshot
# is a cheap no-op (no set churn, no commitment re-score), keyed on CONTENT (not
# peer_id+sequence) so the accept-regardless-of-sequence attack-recovery path is preserved.
#   Migration:  elohim/elohim-storage/migrations/2026-07-06-120000_peer_inventory_cursor_content_hash
#   Unit guard: elohim/elohim-storage/src/db/peer_blob_inventory.rs (12 idempotency tests —
#               the fine-grained, deterministic regression floor)
#   Design:     genesis/data/timeline/backlog/genesis-pipeline-substrate-gated-adam-arc-saturation.md
#               genesis/data/timeline/backlog/inventory-gossip-amplifier-three-layer-idempotency.md
#
# These scenarios are the SYSTEM-level regression: they pass once the fix drains the storm
# (apply-rate collapses → cores freed → runtime un-starved → projection catches up → the
# doorway surfaces serve again) and go red if receive-side idempotency ever regresses.
# @requires:alpha-cluster-6peer — needs the live multi-peer alpha mesh (the storm only
# manifests under real cross-peer gossip fan-in).
@e2e @dataplane @concern:inventory-convergence @requires:alpha-cluster-6peer
Feature: Inventory-gossip apply plane converges under WAN-adversarial gossip
  A peer that sinks the full arc must drain inbound inventory idempotently: redundant
  re-delivered or re-emitted snapshots of an unchanged blob set carry no new information
  and must not trigger a full re-apply or commitment re-score. When they do, the peer
  never converges, pegs its cores, and starves every surface that shares its runtime.
  This feature asserts the emergent property the receive-side idempotency fix restores —
  the mesh catches up and the admission plane keeps headroom — rather than the unit-level
  no-op (that is locked by peer_blob_inventory.rs's tests).

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: the seed-facing doorway peer catches its projection up under sustained gossip
    # Under the storm the seed peer (adam) cannot drain inventory, so the projection it
    # feeds never reaches caughtUp on the doorway host. Convergence restores it.
    Then peer "alpha-A" /health p2p.caughtUp is true
    And peer "elohim.host" /health p2p.caughtUp is true

  Scenario: the admission plane keeps headroom instead of wedging under the CPU storm
    # The non-idempotent re-apply + re-score pegs the runtime; HTTP admission backs up and
    # semaphore headroom collapses toward zero. A converged plane leaves permits available.
    Then peer "alpha-A" /health semaphorePermits >= 1
    And peer "elohim.host" /health semaphorePermits >= 1
