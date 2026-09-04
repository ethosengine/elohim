---
id: "backlog-arch-scale-risk-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Scale-risk cluster — the shapes in landed code that grow badly with chain length, peer count, or migration count"
slug: "arch-scale-risk-backlog"
written: "2026-09-04"
author: "claude (risk discipline, operator-directed 2026-09-04)"
status: "backlog"
priority: "high"
area: "architecture/scale-envelope"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "habit:happ-lineage-migration"
  - "habit:conductor-capacity-represented"
  - "feature:genesis/a2o/features/delivery/happ-lineage-migration.feature"
cites:
  - "holochain-evolution-epic | Holochain Evolution Epic | sha256:ac9f29f9ae06b776 | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - genesis/data/timeline/backlog/dht-scale-envelope-and-web2-projection-at-planetary-scale.md
tags: [risk, scale, architecture, head-plane, lineage, node-registry, elohim-storage, p2p-design-gate]
---

# Scale-risk cluster

A **risk** row is a concern with a *measurable trigger* that has not fired. It is filed here at
the moment the code that carries it lands, not when it bites. The discipline (tag, row shape,
where each row is surfaced and recalled) is in `genesis/data/timeline/CONVENTIONS.md` §Risks.
Sibling: the 2026-07-11 DHT scale-envelope entry holds the *planetary* question; this cluster
holds the *landed-code* shapes, each pinned to a file and a number.

Every row names the smallest change that retires it and the surface it graduates to. A row that
fires flips to `regression` (with the receipt) and gets a chronicle entry; the row itself stays
until the mitigation lands.

## Rows — Holochain Evolution Epic, measured from code 2026-09-04 (r15/r16 scale: node_registry, tens of records, three peers)

| # | Risk | Where | Trigger (measurable) | Horizon | L / I | Mitigation → graduates to | Status |
|---|---|---|---|---|---|---|---|
| 1 | **Export re-walks the whole chain per page (quadratic carry).** `export_records` runs a full `query(include_entries(true))`, sorts, and recomputes the whole-chain digest, then takes one `EXPORT_CAP`=64 page; `export_held_records` does the same over `get_agent_activity`. Carrying N records costs ≈N²/64 record loads inside WASM. | `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` `export_records` (~:1396), `export_held_records` (~:1516) | Any role chain past ~3k app entries. lamad today ≈3.5k heads → ≈190k record loads per carry; 50k → 39M. Measure: carry elapsed vs `v1_count` on the Station 3 receipt (add elapsed to the receipt line). | First lamad crossing | certain at lamad scale / carry time + WASM memory + window length | Compute the digest once per walk and pin it by the cursor (coordinator-only, DNA-hash-NEUTRAL) → epic gap **G8**, a plan task on the coordinator lane | open |
| 2 | **Witness validation walks the carrier's whole v2 activity.** `refuse_carried_after_close` calls `must_get_agent_activity` from `prev_action` back to genesis and `must_get_entry` on every earlier same-lineage witness: O(W) per witness, O(W²) per carrier, re-run on every integrating authority. Compounds with the G6 fix (more couriers = more chains). | `elohim/holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs` `refuse_carried_after_close` (~:700–760) | >~200 witnesses per carrier per lineage (lamad ≈220 → ≈24k activity fetches per validator) or >~5 couriers. Measure: validation time of the last witness of a carry vs the first. | First lamad crossing; worse after Tasks 21–23 | likely / validator CPU on every peer, integration lag | Carry `close_seq` forward in each witness, or bound the scan to the last close — INTEGRITY change, hash-moving → rides the **Tasks 21–23 sunset-hardening crossing** as gap **G9** | open |
| 3 | **Chains double per migration and the held-carry fans out.** Self-carry re-creates every v1 record on v2 plus one witness per 16 (≈×1.06 per migration, never pruned). The bridge sweep has every crossed peer held-carry every neighbour: DHT writes ∝ peers × records. `HELD_PAGE_LIMIT`=16 at `LINEAGE_SWEEP_SECS`=30 → a 3.5k-record neighbour takes ≈110 min per courier; the window stays open at least that long. | `elohim/elohim-storage/src/services/lineage_bridge.rs` (consts ~:86–94); `carry_from` in the coordinator | Third migration of one role, or >~10 peers in a role. Measure: v2 chain length / v1 chain length after a crossing; sweep catch-up minutes on the Station 5 receipt. | Second/third crossing; alpha at 7 peers is fine | likely over years / storage + validation surface ×migrations; window length | Courier election (one courier per neighbour, not all), page/interval tuning, and a witness-backed compaction of carried v1 facts (design, p2p-design-gate) → `arch-dataplane-refactor-backlog` head-plane row + the sunset design | open |
| 4 | **Dual cells per role during the window.** v1 and v2 both run under one key, so conductor RAM, arc coverage and gossip ≈ double until the sunset. Stock 0.7 conductors already reached ~14 GB overnight on the mesh (epic ledger 2026-09-05) before any dual-cell window. | conductor; `install_lineage` in `elohim/elohim-storage/src/services/release_adoption/apply.rs` | A window longer than a night on household tier, or a lamad-sized role. Measure: conductor RSS during the Station 3→8 window vs before. | First lamad crossing | likely / household peers OOM mid-window (group kill) | Sunset SLA measured on the Station 8 receipt; the jemalloc fork conductor on household peers; arc factor during the window → `conductor-capacity-represented` habit guard | open |
| 5 | **Per-entry idempotency reads.** `entry_already_witnessed` does `get_links` plus one `get` per witness targeting the entry, once per carried record: O(records × witnesses-per-entry). | coordinator `entry_already_witnessed` | Entries witnessed by many lineages (>~5 crossings). | Later crossings | low / carry time | Key the `EntryToWitness` link tag by lineage hash so one `get_links` answers → coordinator-only | open |
| 6 | **Adoption controller sweep load on a converged fleet.** `watch.rs` sweeps every 60 s over ≤8 channels with one uncancellable `call_zome` each, plus DHT reads for path evidence, roster and state links per peer, forever. Reads/min ∝ peers × channels. | `elohim/elohim-storage/src/services/release_adoption/watch.rs` (`SWEEP_INTERVAL_SECS`, `MAX_CHANNELS_PER_SWEEP`); `path_evidence.rs` | >~100 peers, or >8 followed channels per peer. Measure: conductor-diagnostics read rate on a converged fleet. | Beyond alpha | low now / background DHT load | Cache TTL + backoff when converged (the cache exists); gossip the verdict instead of re-deriving it → release-adoption controller | open |

## Recall wiring for this cluster (the discipline, applied)

- **At the point of touch:** `inject` rules at `elohim/holochain/dna/.epr-meta` (fires on `lib.rs` edits naming the export/carry/witness functions) and `elohim/elohim-storage/.epr-meta/manifest.md` (fires on `lineage_bridge.rs`) point here.
- **At planning:** the `risk` tag is the query; `/converge` ranks this cluster by `priority: high`; the p2p-design-gate's head-plane question cites row 3 for any lineage entity.
- **At session start:** rows 1–4 sit in the `guard:` of the `happ-lineage-migration` habit atom, rendered by `just status habits --full`.
- **Every measured run:** the Station 3 / 5 / 8 receipts carry the trigger numbers (elapsed, counts, RSS) — the first live measure of these rows is the next lamad-scale run, not node_registry.
