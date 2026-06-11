# PHASE-3 GATE ARTIFACT — holochain dna/ island (pass A)

Prepared 2026-06-11. NOTHING below is applied — the operator gate is consolidated across all six
pillars. Pass-A placements are committed; this file is what the gate ceremony consumes.

## Disposition map (FINAL)

| Island file | Disposition | Recomposed into |
|---|---|---|
| elohim/holochain/dna/NETWORK_UPGRADES.md | SPLIT + RETIRE | Policy/governance → architecture/2026-06-11-dna-upgrade-governance.md (seed; residue test PASSED — wave-1 execution plan absent from tree, policy homed nowhere else). Arc/philosophy-graduation story → history/2026-06-11-network-upgrades-stewarded-coordination-arc.md. Philosophy itself ALREADY lives in elohim/holochain/rna/README.md (seed summarizes + cites, never restates). Gospel ## Upgrades pointer applied. |
| elohim/holochain/dna/LINK_ARCHITECTURE.md | HARVEST + RETIRE | Arc + checklist-closure note → history/2026-06-11-link-architecture-arc.md (closure note for the sentinel-tracked checklist line is explicit there; sweep stays open in deprecation-link-architecture-query-index-sweep.md). Gospel link-budget rail (225/256 + Signal Rule) applied. Triage rule already canonical in records-lifecycle-design (untouched). |
| elohim/holochain/dna/SCHEMA_VERSIONS.md | HARVEST + RETIRE | v1 museum → history/2026-06-11-lamad-v1-schema-museum.md. Export seam documented live in the seed §6. ZERO inbound refs (re-verified) — cleanest retirement of the three. |

## Retirement list (operator gate: git rm)

1. elohim/holochain/dna/LINK_ARCHITECTURE.md
2. elohim/holochain/dna/NETWORK_UPGRADES.md
3. elohim/holochain/dna/SCHEMA_VERSIONS.md

## Deferred ref-repairs

Exact old/new pairs: /tmp/island-recompose-holochain-dna/deferred-ref-repairs.md
- 27 text-repair pairs across 15 files (every replacement target now EXISTS — the PENDING marks were
  pre-placement; re-verified at commit time: seed + both arcs + museum + both backlog entries placed).
- 1 census/registry flip: genesis/data/timeline/backlog/subject-routing-locus-census.md L138 (remove
  `holochain {LINK_ARCHITECTURE}` from the still-to-route set; optionally log the recompose at ~L140-146).
- subject-routing docs_island line for the holochain locus (if/where declared) — flip with census.
- 9 no-action sites (recovery_m4.rs repaired this session; NU self-ref dies with file; recipe doc;
  6 generated memory-kit/ledger artifacts).
- OPTIONAL (not retirement-gated, already-dead pointer): manifest-hygiene README "wave-1 execution
  plan §7" → seed (pairs in §D of deferred-ref-repairs.md).

## Pass-B coupling notes (docs/ cluster session MUST read)

1. elohim/holochain/docs/README.md L118 + docs/ARCHITECTURE.md L268 reference dna/NETWORK_UPGRADES.md
   via ALREADY-BROKEN ./dna/ relative paths — exact repair pairs (→ seed) in deferred-ref-repairs.md §B3.
2. docs/ARCHITECTURE.md shares the stale MongoDB projection framing LINK_ARCHITECTURE had; the
   projection actually shipped SQLite/diesel (graph-native-projection-substrate-design). Note: the
   MongoDB vocabulary is ALSO fossilized in live code comments (content_store_integrity ~L3442,
   infrastructure_integrity ~L65, content_store ~L10387, elohim-storage cache_stream.rs L5,
   identity.rs L27) — retiring docs does not retire the fossil.
3. Broken /elohim-node/ refs (census flag) are ALL in docs/: README.md L107, ARCHITECTURE.md
   L100,102,113,115,245,262 (../elohim-node/ paths — verify target existence in pass B).
4. docs/claude.md lowercase normalization = pass B.
5. docs/ARCHITECTURE.md:163-167 describes rna/ as schema tooling — pass B should cite the seed's §6
   liveness verdict rather than re-deriving (hc-rna = live library + seeding CLI; migration pipeline unwired).

## Open questions (carried by placed docs; gate need not resolve)

- Seed §8: migration import/transform pipeline; elohim consensus mechanism; notification; rollback;
  self-hosted participation; lineage backfill once upstream stabilizes; additive-lineage policy call.
- Museum: "2024-12" v1 date — predecessor-repo history vs typo (undecidable in this tree).
- Arc: which other *By* links have gained integrity-side consumers (Backfill 3 per-link audit owns).
- humantomastery backlog: which DNA's signal storage's mastery projection subscribes to; whether the
  elohim ContentMastery declaration is load-bearing for v1 healing until lamad-v1 retires.
