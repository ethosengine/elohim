---
id: "backlog-lamad-imagodei-humantomastery-dual-home"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ContentMastery + HumanToMastery are dual-homed in lamad and imagodei DNAs — decide the owning DNA"
slug: "lamad-imagodei-humantomastery-dual-home"
written: "2026-06-11"
author: "dna-island-recompose-phase0"
status: "backlog"
priority: "medium"
relatedNodeIds: []
tags: [holochain, dna, link-types, entry-types, lamad, imagodei, mastery, dual-home, dht-as-notary]
cites:
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/healing_integration.rs
---

## What is dual-homed

The `ContentMastery` entry type AND its mastery link types are declared in BOTH
DNAs' integrity zomes (all verified at source 2026-06-11):

**lamad (elohim) DNA** — `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`:
- `ContentMastery` struct ~L1066; `EntryTypes::ContentMastery` variant ~L3681;
  validated at ~L4294.
- Link types ~L3864-3867: `IdToMastery`, `HumanToMastery`, `ContentToMastery`,
  `MasteryByLevel`.

**imagodei DNA** — `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`:
- `ContentMastery` struct ~L457 (doc ~L445); `EntryTypes::ContentMastery`
  variant ~L900; validated at ~L1110 (`validate_content_mastery` ~L1278).
- Link types ~L981-983: `HumanToMastery`, `ContentToMastery`, `MasteryByLevel`.

## Who actually writes (verified)

- **imagodei coordinator is the only live writer.**
  `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` `upsert_mastery`
  (~L1540) creates the `ContentMastery` entry and the `HumanToMastery` link
  (`create_link` ~L1611-1616). Reads: `get_mastery` ~L1631, `get_my_all_mastery`
  ~L1678.
- **The elohim (lamad) coordinator never creates a `HumanToMastery` link.**
  Every mastery extern in `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`
  is a cross-DNA bridge to imagodei: `get_my_mastery` ~L907, `get_my_all_mastery`
  ~L943, `upsert_mastery` ~L979 (all `call(CallTargetCell::OtherRole(IMAGODEI_ROLE), ...)`);
  `export_all_mastery` ~L8079 wraps the bridge.
- The only local mastery touch in the elohim DNA is the v1-healing path:
  `healing_integration.rs` reads `LinkTypes::IdToMastery` (~L339) and
  `create_entry(&EntryTypes::ContentMastery(healed))` (~L387) — and creates NO
  link for the healed entry (no `create_link` in healing_integration.rs /
  healing_impl.rs / migration.rs).
- `ContentToMastery` and `MasteryByLevel` are created by NO coordinator in
  either DNA (the only non-integrity hit is a dead input struct
  `MasteryByLevelQueryInput` at content_store/src/lib.rs ~L2328).

## Risk

Two DNAs both declare ownership of mastery records and mastery links. If a
future coordinator change (or a healing/migration pass) starts writing
`HumanToMastery`/`ContentMastery` on the elohim DNA while imagodei remains the
live write home, mastery state splits across two DHTs with no reconciliation
rule — split-brain mastery records, and projections/readers cannot know which
DHT is the notary for a given record. The dormant declarations also burn
entry-type and link-type budget on the elohim DNA (the 256 link-type cap is at
225 used per genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
~L1271), and the elohim-side healing path already creates linkless orphan
`ContentMastery` entries that no link query can find.

## What resolution requires

1. **Decide the owning DNA** for mastery truth (current de-facto: imagodei —
   it has the only live writer; the elohim externs are bridges).
2. **Remove the dormant declarations from the other DNA**: the elohim DNA's
   `ContentMastery` entry type + `HumanToMastery`/`ContentToMastery`/`MasteryByLevel`
   link types (keeping whatever the v1-healing path genuinely needs — likely
   only `IdToMastery` + the entry type until healing is retired, or re-route
   healing output through the imagodei bridge so even that goes away).
3. **Re-route coordinator + projection**: confirm elohim-storage's mastery
   projection subscribes to the imagodei DNA's signals (not lamad's), and that
   the healing read path's recovery of v1 mastery lands in the owning DNA.
4. Integrity-zome change = DNA-hash-breaking on the trimmed DNA; sequence with
   the network-seed/lineage upgrade ladder
   (genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md).

OPEN QUESTION: which DNA's post-commit signal does elohim-storage's mastery
projection actually subscribe to today (not verified this session)?

OPEN QUESTION: is the elohim-DNA `ContentMastery` declaration load-bearing for
the lamad-v1 → v2 healing/migration path (healing_integration.rs), such that
removal must wait for v1-archive retirement?

## Provenance

Surfaced during the holochain `dna/` island recompose Phase-0 verification
(2026-06-11), while verifying LINK_ARCHITECTURE.md claims against the integrity
zomes. Not previously tracked: no match for `HumanToMastery` in
`genesis/data/timeline/` or `genesis/docs/superpowers/plans/` as of this
writing.
