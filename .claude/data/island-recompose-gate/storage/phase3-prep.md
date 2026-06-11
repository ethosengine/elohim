# Phase-3 PREPARED artifacts — elohim-storage island (session 3; execution awaits consolidated operator gate)

## Disposition map (file → recomposed-into)

| Island file | Verdict | Recomposed into |
|---|---|---|
| elohim/elohim-storage/P2P-ARCHITECTURE.md | SUPERSEDED (dual-plane canonized; status table falsified; 4 proposals never shipped as drawn) | history: genesis/docs/content/elohim-protocol/history/2026-06-11-storage-dual-plane-design-arc.md (canon homes: tiered-quilt §Three truth layers + dht-is-a-notary record) |
| elohim/elohim-storage/EDGE-ARCHITECTURE.md | SPLIT: cache-core → already homed (elohim-cache-core gospel + 2026-03-29 extraction design); doorway sections → session-4 handoff (below); seeder → genesis locus current practice; sha256-hex → storage gospel §Design Vocabulary | history record (landed/not-landed ledger) + doorway handoff notes |
| elohim/elohim-storage/REACH.md | SUPERSEDED vocabulary (geographic 8 = dead 4th strand) + INVERTED philosophy (delivery-filtering vs live author-side earning) + unconsumed scaffolding | history record §vocabulary-ghost + backlog: genesis/data/timeline/backlog/storage-island-harvest-residue.md + origin note added to reach-vocabulary-frontend-strand.md |

NO new architecture seed — residue test negative (matches qahal + elohim-pillar precedent): every still-true claim already homed.

## Retirement list (the gate's git rm)

```
git rm elohim/elohim-storage/P2P-ARCHITECTURE.md elohim/elohim-storage/EDGE-ARCHITECTURE.md elohim/elohim-storage/REACH.md
```

## Planned ref-repairs (post-rm)

ZERO repairs required in non-retiring files. Every inbound ref lives in another session's island
(retiring at the same consolidated gate) or in lineage prose:

- doorway/doorway-service/RECOVERY-PROTOCOL.md:979-980 → session-4 island (itself on the retirement list). IF session 4 does NOT retire it: delete the two "Edge Architecture"/"P2P Architecture" related-doc lines (their targets are gone; :979's ../EDGE-ARCHITECTURE.md was ALREADY dead — no doorway/EDGE-ARCHITECTURE.md exists).
- steward/node/ARCHITECTURE.md:523 → session-6 island. IF not retired: delete the line `- [elohim-storage/P2P-ARCHITECTURE.md](../elohim-storage/P2P-ARCHITECTURE.md) - Blob storage P2P`.
- elohim/holochain/docs/{README.md:110, P2P-DATAPLANE.md:374,435-436, SYNC-ENGINE.md:479, ARCHITECTURE.md:266-267, REACH.md:131,211-213} → session-5 island. NOTE: these relative `./elohim-storage/*` paths were ALREADY dead before this recompose (no such subdir under docs/). IF not retired: drop the rows/lines.
- genesis/data/timeline/backlog/pillar-island-recompose-recipe.md:145 → lineage prose inside the session-3 fan-out prompt — keep as-is (historical instruction text).
- All `derived_from:` comments in placed docs already use anticipated-retirement phrasing ("retired to git 2026-06-11") per sessions-1/2 precedent — no post-gate edits needed.

## Registry updates (post-rm)

### 1. CREATE elohim/elohim-storage/.claude/subject-routing.yaml (census row 10 "id; route ..." action)

```yaml
# elohim/elohim-storage/.claude/subject-routing.yaml
# Cascade sub-manifest (locus: elohim-storage — the operational data plane crate).
# Same SHALLOW-MERGE caveat as app/lamad: this manifest's active effect today is REGISTRATION
# + the locus pin; do NOT add a partial `classes:` block until the resolver deep-merges.
# Spec: genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md

version: 1
default_class: protocol-canonical

locus:
  subject: elohim-storage
  pillar: elohim
  role: implementation-truth-home            # truth: SELF for the HTTP/blob/P2P surface
  gospel: elohim-storage-gospel              # elohim/elohim-storage/CLAUDE.md, cites tiered-quilt-stewardship-design
  # docs_island RETIRED 2026-06-11 — P2P-ARCHITECTURE/EDGE-ARCHITECTURE/REACH recomposed into
  # the storage-dual-plane-design-arc history record + storage-island-harvest-residue backlog entry
  # (recipe: genesis/data/timeline/backlog/pillar-island-recompose-recipe.md)
```

### 2. Census row 10 flip (genesis/data/timeline/backlog/subject-routing-locus-census.md)

OLD (line 65):
```
| 10 | `elohim/elohim-storage` | true-locus | protocol-canonical | self | plain | id; route `P2P-ARCHITECTURE`, `EDGE-ARCHITECTURE`, `REACH` | med |
```
NEW:
```
| 10 | `elohim/elohim-storage` | true-locus | protocol-canonical | self | plain | ~~id; route docs~~ DONE 2026-06-11 — island retired; recomposed → `storage-dual-plane-design-arc` history + `storage-island-harvest-residue` backlog; sub-manifest declared | med |
```
(If the consolidated gate maintains a "RECOMPOSED" section like lamad's, append: storage island
{P2P-ARCHITECTURE, EDGE-ARCHITECTURE, REACH} → 1 history record + 1 backlog entry + origin-note
edit + gospel concern-routing rails; no architecture seed — residue test negative.)

## Session-4 (doorway) handoff notes

1. EDGE-ARCHITECTURE.md doorway-owned sections (judged per-section, NOT moved by session 3):
   §Architecture Overview doorway box, §doorway component responsibilities ("should/should-not"),
   §Route Registration Protocol, §Migration Path Phases 2-3. Their still-true content is ALREADY
   gospel in doorway/CLAUDE.md (manifest-declared routes :30; thin-bridge identity). Recommended
   home for anything residual: doorway/CLAUDE.md concern routing — but session 3 found NOTHING
   unhomed; recommend NO new doorway doc.
2. LIVENESS REFINEMENT session 4 should inherit (verified at source): `__doorway_routes` DNA
   introspection is an explicit not-yet-implemented stub (doorway-service/src/services/discovery.rs:276
   "Not yet implemented — returns None. Routes come from steward self-registration via
   build_manifest(), not DNA introspection"); only `__doorway_import_config` runs as designed
   (dna/elohim/zomes/content_store/src/lib.rs:1743). Do not bless "dynamic route discovery from
   DNAs" as live when triaging doorway docs that claim it.
3. RECOVERY-PROTOCOL.md:979 cites ../EDGE-ARCHITECTURE.md — a path that NEVER resolved from there
   (doorway/EDGE-ARCHITECTURE.md does not exist). Pre-existing dead link, session 4's to retire.
4. doorway/CLAUDE.md:139-144 documents a reach table that is a "7th mixed variant" per
   reach-vocabulary-frontend-strand.md:42 AND does not match actual HTTP behavior per
   http-reach-enforcement-gap.md — if session 4 touches the doorway gospel's reach section,
   compose with those two backlog entries; do not canonize the table.
