# D1–D5: Node / Household / Doorway / Shem Canonical Decisions

**Date:** 2026-04-19
**Spec:** 2026-04-19-p2p-dataplane-visibility-design.md

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
