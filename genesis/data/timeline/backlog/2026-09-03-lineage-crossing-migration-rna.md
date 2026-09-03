---
id: "backlog-lineage-crossing-migration-rna"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Lineage-crossing migration (RNA) — carry a network's data across a conductor-line / integrity-hash break without a wipe"
slug: "lineage-crossing-migration-rna"
written: "2026-09-03"
author: "holochain 0.7 cutover retrospective (operator + integrator, 2026-09-03)"
status: "open"
priority: "high"
tags: [upgrade-propagation, rung-6, rna, lineage, mishpat, ark, storage-p2p, p2p-design-gate, north-star]
cites:
  - genesis/docs/superpowers/plans/2026-09-02-holochain-0-7-upgrade-guide.md
  - genesis/docs/superpowers/plans/2026-09-03-holochain-0-7-fleet-cutover-runbook.md
  - genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
---

# Lineage-crossing migration — the rung above rung 5

**Concession recorded 2026-09-03.** Rung 5 (release channels, canary adoption, coordinator hot-swap, promotion,
convergence) is proven and survived the 0.7 line change. It propagates a release whose integrity zomes, DNA hashes and
conductor line do not move. The 0.6→0.7 cutover moved all three at once and the only path was a full wipe + re-genesis.
That is acceptable exactly once, on a pre-builders fleet whose chains were already torn. The north star
(`feedback_upgrade_propagation_north_star_wall_clock`: mixed-version peers keep talking, no big-bang rolls) needs this rung.

## The three breaks a line change carries (and which layer can bridge each)

| Break | Holochain's stance | Where the bridge lives |
|---|---|---|
| Conductor data layer (0.6 `holochain_sqlite` → 0.7 `holochain_data`, no migration) | none; re-genesis | the ark supervises TWO conductor children for the window: old (read-only source of chains) + new (writes) |
| Integrity hashes move (hdk/hdi bump ⇒ new wasm ⇒ new DNA ⇒ new DHT) | `unstable-migration` lineage + app-level migrate functions | a `migrate-from` recipe declared in the release manifest BEFORE data is held (rung 5's manifest already carries a lineage hint the verifier enforces) |
| Network protocol (kitsune2 0.4 ↔ 0.5 cannot talk; websocket API moved) | none | the elohim-storage P2P plane (libp2p/iroh + our codecs) is version-agnostic: it carries old records + signatures between peers; the DHT is per-line |

## Stations (each a p2p-design-gate question before any code)

1. **Identity continuity.** Does the lair keystore survive a line change (0.6 lair → 0.7 lair 0.7.1 store format)? If
   yes, agent keys persist and re-authored records keep their author; if no, the imagodei bindings are the identity
   carrier across lines and every line change re-keys. Today's wipe dropped `ks/` without knowing which. Measure on the
   household mesh with a preserved keystore.
2. **Provenance across re-authoring.** A migrated record gets a new action hash. Its EPR provenance must name the old
   DNA + old action hash (and the migrate-from recipe CID) or every commitment/attestation lineage breaks per line.
   Entry-type question: an attribute (A2) on the new record, content-addressed, never a slug.
3. **The recipe.** `migrate-from: {dna: <old bundle hash>, via: <zome fn>, revert: <zome fn>}` in the release manifest;
   verify refuses a manifest whose recipe names a DNA this peer is not running (mirrors the existing lineage-hint refusal).
4. **The window.** ark runs old+new conductors side by side; storage reads the old cells through the old admin/app API
   (holochain_client of the OLD line — a versioned client shim, or the storage P2P plane replaying from a peer that still
   runs the old line), writes through the new; the window closes when the new chain's head equals the recipe's expected
   projection of the old.
5. **Consent + revert.** Adoption of a lineage-crossing release is a Mishpat commitment (delegates-compute shape) with an
   explicit revert path; the canary's attestation must name what it checked across the lineage, not just "up".
6. **Mixed-version talk.** Prove on the household mesh: one peer on line N, two on N+1, content authored on either side
   reaches all three through the storage plane while their DHTs are disjoint.

## Evidence of the gap (2026-09-03)

Edge #1426's conductor phase: a 0.7 conductor on 0.6 volumes never became ready (jessica was 2/2 Ready in an
admin-websocket close loop; the rollout's 600 s gate expired) and the phase halted; the only remedy was scaling every
conductor to 0 and clearing `holochain-data-*` (`databases/`, `ks/`) and `storage-data-*` on all seven peers.
