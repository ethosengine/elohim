---
title: "History/ADR: D1–D5 Node / Household / Doorway / Shem canonical decisions"
type: history-gotcha
status: Accepted
tier: history
created: 2026-04-19
topic: [node, household, doorway, peerstatus, shem, topology]
# Settled sprint decisions (2026-04-19). Source spec body retired to git history.
distills:
  - 2026-04-19-p2p-dataplane-visibility-design.md   # source spec — body in git history
# Bidirectional: the CANONICAL architecture these decisions live under.
canonical:
  - ../architecture/2026-05-02-elohim-hub-boundaries-design.md       # D1/D2 node + household topology
  - ../architecture/2026-05-23-doorway-access-tier-patterns.md       # D4 doorway stays web2-only
memory_anchors:
  - project_doorway_peer_registration        # peers register with doorway
  - project_household_is_resilience_unit     # household = collective kind, resilience-first UI
  - project_shem_is_p2p_live_canvas          # shem as acceptance target
  - project_three_layer_truth_model          # DHT notary / SQLite projection
---
# D1–D5: Node / Household / Doorway / Shem Canonical Decisions

**Date:** 2026-04-19
**Spec:** 2026-04-19-p2p-dataplane-visibility-design.md (body retired to git history)

## D1. PeerStatus canonical for visibility, elohim-node canonical for topology.
PeerStatus (infrastructure DNA) is the notarized entry type answering "is this peer alive right now?" via the existing `record_peer_status` coordinator zome function. Elohim-node publishes durable node shape (hostname, archetype, committed resources, household binding) to storage via POST /api/v1/nodes/shape, which commits through the new `register_node_shape` coordinator fn (introduced in this sprint — see plan Task C4) to the existing `NodeRegistration` DHT entry type (node-registry DNA). Source of truth for both is the DHT; SQLite is projection only. /shefa/devices joins both: hard node inventory + live peer vitals. node-registry DNA's custodian-assignment pieces remain; frontend NodeRegistryAnchor retires.

## D2. Household reuses `collectives` with kind: "household".
Place-grounded hard collective type. humans.json spouse/family edges become membership. `householdId` derives on humans. Place-as-first-class is v2.

## D3. Resilience UI is household-first.
Tooltip, dashboard, devices page all lead with household counts. Per-peer is drilldown.

## D4. Doorway stays web2-only.
ZERO per-domain proxy files added. All domain routes via elohim-storage's build_manifest(). Only doorway-service changes: wire /admin/routes handler, fix /admin/users auth.

## D5. Shem is acceptance target.
>100GB RAM, ~4TB storage. Full persona roster runs as real peers. Dashboards lighting up on shem is the bar.
