---
title: "History: The link-architecture arc — the Signal Rule won the canon while its cleanup phases stalled (Dec 2025 – Jun 2026)"
id: link-architecture-arc
type: history-gotcha
status: noted
tier: history
created: 2026-06-11
topic: [holochain, dna, link-types, 256-cap, dht-as-notary, projection, multi-dna-split, deprecation, signal-rule]
# Provenance breadcrumb: the retiring island doc this record distills.
derived_from:
  - elohim/holochain/dna/LINK_ARCHITECTURE.md  # retired to git 2026-06-11 (holochain dna/ island recompose; authored 2025-12-26, never content-updated after creation)
canonical:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
  - genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md
cites:
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
  - elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs
  - holochain-integrity-layer-gospel | the living DHT-as-notary stance + link-budget rail the Signal Rule survives in | sha256:30351bbf65d5c8b9 | path: elohim/holochain/dna/CLAUDE.md
  - records-lifecycle-part-d-substrate-gaps-plan | the plan home of Backfill 3 — the *By* sweep that closes this doc unexecuted Phase 2 | sha256:9f15d649953c9ac7 | path: genesis/docs/superpowers/plans/2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md
  - genesis/docs/superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md
  - genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md
  - elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs
  - genesis/data/timeline/backlog/lamad-imagodei-humantomastery-dual-home.md
  - genesis/data/timeline/backlog/recovery-m4-dual-anchor-stubs.md
---

# History: The link-architecture arc (Dec 2025 – Jun 2026)

> **Hot-context pointer (the one sentence to remember):**
> The Signal Rule ("if a link exists ONLY to enable queries, it should be a
> projection query") won the canon while every one of its cleanup phases
> stalled — six months later the lamad link enum still burns 225/256 slots
> with 46 live `*By*` variants and zero deprecation markers. Philosophy
> compounds through citation; checklists rot the day they're written.

## The cap crisis and the philosophy (December 2025)

Holochain's `#[hdk_link_types]` uses a `u8` discriminant — 256 link types per
zome. The original monolithic `content_store_integrity` hit **255**, the
absolute limit. LINK_ARCHITECTURE.md (authored 2025-12-26, commit
`7084a9b04`; never content-updated afterward — only moved on 2026-03-10,
`471bc0fcf`) answered the crisis with two moves: a **multi-DNA split**
(doorway → infrastructure DNA, identity → imagodei DNA) and a **link-vs-
projection philosophy** whose core test it named the Signal Rule. The
`{Entity}By{Attribute}` naming pattern was its smell-test for links that
should be projection queries instead.

The philosophy is the part that survived — but it lives elsewhere now. The
link-triage rule and the 256-cap budget at fleet scale are canonized in the
records-lifecycle design (its D.11 "Backfill 3 — LINK_ARCHITECTURE
deprecation sweep", ~L2827, formally owns the retirement), and the
DHT-side stance is the gospel in `elohim/holochain/dna/CLAUDE.md` ("the DHT
is the notary, not the database") plus the
`2026-06-01-dht-is-a-notary-not-a-byte-store` history record. Read those for
the model; this record keeps only the arc.

## What shipped: the multi-DNA split (mechanism-shipped)

Phase 1 executed, for links:

- Doorway links (`IdToDoorway`, `OperatorToDoorway`, `DoorwayToHeartbeat`,
  `DoorwayToSummary`) are gone from lamad — only the tombstone comment block
  remains (`content_store_integrity/src/lib.rs` ~L4208–4212), and
  `infrastructure_integrity/src/lib.rs` carries its own link enum (~L238,
  **10 variants** measured 2026-06-11 vs the doc's planned 8).
- Identity links (`IdToHuman`, `AgentKeyToHuman`, `HumanByAffinity`, the
  relationship family) are gone from lamad — tombstone block ~L3911–3917 —
  and live in `imagodei_integrity/src/lib.rs` (link enum ~L953, **80
  variants** measured 2026-06-11 vs the doc's planned 21: witnesses, key
  stewardship, anomalies, stewardship grants, device policies, appeals,
  activity logs all grew there afterward).

The split itself was real and load-bearing. But note the asymmetry recorded
below: the *entry structs* were never de-duplicated the same way.

## What never shipped: Phases 2 and 3 (never-executed)

Measured at source 2026-06-11 (`content_store_integrity/src/lib.rs`, link
enum at ~L3807):

- **225 link types** still in the lamad enum (down from 255 only via the
  Phase-1 removals; the doc's appendix target of ~175 was never approached).
- **46 `*By{Attribute}` variants** still live — `TypeToContent` (L3813),
  `TagToContent` (L3814), `AuthorToContent` (L3815), `EventByAction`
  (L3944), `EventByLamadType` (L3945), `ResourceBySpec` (L3955), and 40
  more. The only `*By*` removals ever done predate the doc (PathByCreator /
  PathByDifficulty / PathByType / PathByTag, tombstoned at L3844).
- **Zero** `// DEPRECATED: Use projection query` markers exist anywhere in
  the integrity zome — Phase 2's step 5 was never started. The only
  DEPRECATED markers in the file belong to the LearningPath surface
  (L721–810), a different deprecation with its own canonical backlog entry.
- Phase 3 (signal-only link annotation) likewise has no trace in either the
  integrity or coordinator zome.

The sweep is now owned end-to-end by
`genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md`
(blocked, sequenced in records-lifecycle Wave A as Backfill 3 of three
coordinated substrate-floor backfills; the part-d plan is the plan home).
That backlog entry explicitly anticipated this recompose carrying the
checklist-closure note — this record is that note: **the checklist closes
unexecuted; the work item lives in the backlog, not the retired doc.**

## Two corrections later canon made to the doc's mechanics

1. **Keep-in-enum → retire-and-reclaim (a genuine guidance inversion).**
   Phase 2 step 4 said "Keep link type in enum (for DHT compatibility)".
   Records-lifecycle Backfill 3 says the opposite: retire the `*By*`
   variants *from the enum* and return the slots to the 256-cap budget so
   structural additions (D.1's `EprToEvent`/`EprToResource`) can land. The
   philosophy held; the retention mechanics inverted.
2. **"Query-only" is no longer true of the flagship example.**
   `TypeToContent` — first on the doc's deprecate list — gained a
   load-bearing *integrity* consumer in Recovery M4: the cross-DNA
   gate-reader helpers for `governance-action:key-revocation` /
   `identity-freeze` floors traverse `TypeToContent` links
   (`content_store/src/lib.rs` ~L3275–3304, creation at ~L4537–4543; the
   imagodei `commit_key_rotation` gates must read DHT truth, not SQLite).
   The Signal Rule's premise ("exists ONLY to enable queries") must be
   re-audited per link before the sweep retires anything — the sweep
   backlog should inherit this caveat.

## The projection target moved under the doc

Every "Why Query?" example in the doc is MongoDB (`$lookup`, `$group`, text
indexes). The projection layer actually shipped as **SQLite/diesel in
elohim-storage** (canonical:
`genesis/docs/superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md`).
The stale vocabulary outlived the doc in code: `pub projection: bool, //
Maintains MongoDB projections` is a live field comment in BOTH
`content_store_integrity/src/lib.rs` (~L3442) and
`infrastructure_integrity/src/lib.rs` (~L65), plus coordinator comments
(`content_store/src/lib.rs` ~L10387) and elohim-storage comments
(`src/cache_stream.rs` L5, `src/identity.rs` L27). Retiring the doc does not
retire the vocabulary fossil; it just stops minting new citations to it.

## The budget appendix was stale in both directions

The appendix projected infrastructure ~8 / imagodei ~21 / lamad ~175, total
~204. Measured 2026-06-11: infrastructure **10**, imagodei **80**, lamad
**225** — every number wrong, two by growth, one by never-executed
shrinkage. A point-in-time budget table in a design doc is a snapshot
costume over a moving target; the durable form is the cap-accounting rule
(records-lifecycle owns it), not the numbers.

## The phantom-citation incident

`elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs` (~L429–431)
carries a correction dated 2026-06-11: a test design comment had cited
`LINK_ARCHITECTURE.md §3 "dual-anchor primacy"` — **a section that never
existed in any version of the doc**. An island doc with a reputation
attracts citations to content it never had; nobody re-reads a 203-line
orphan to check. (The dual-anchor pattern itself was then superseded
anyway — KeyRevocation and its anchors were removed from imagodei in
Recovery M4 Task 15, tombstones at `imagodei_integrity/src/lib.rs` L744,
L1021; the stub tests still premised on the removed links are tracked in
`genesis/data/timeline/backlog/recovery-m4-dual-anchor-stubs.md`.)

## Residue carried forward (recorded, not blessed)

- **`HumanToMastery` is dual-homed**: present in BOTH the lamad link enum
  (`content_store_integrity/src/lib.rs` L3865) and the imagodei link enum
  (`imagodei_integrity/src/lib.rs` L981), six months after the doc ordered
  its removal from lamad ("ContentMastery moved to imagodei"). Tracked:
  `genesis/data/timeline/backlog/lamad-imagodei-humantomastery-dual-home.md`.
- **Entry-struct dual-homes outlived the link split**: `Human`,
  `HumanProgress`, `Agent`, `AgentProgress`, `HumanRelationship`,
  `ContentMastery`, `ContributorPresence` appear in BOTH the lamad entry
  enum (~L3670, with `Human` marked "Legacy, kept for backward
  compatibility") and the imagodei entry enum; `DoorwayRegistration` in both
  lamad and infrastructure. Phase 1's "remove duplicated types from lamad"
  completed for links only.
- **Planned rename, not yet landed**: records-lifecycle D.1 plans
  `ContentToResource` (live at `content_store_integrity/src/lib.rs` L3959) →
  `EprToResource`, alongside new `EprToEvent`/`EprToResource` structural
  links. As of 2026-06-11 no `EprToResource` exists in any zome. When D.1
  lands, this record is the link-type history where the rename gets noted
  (the records-lifecycle touch-list item near its L1538 points here).
- **Cross-bundle alias**: `pub struct Commitment` exists in
  `content_store_integrity/src/lib.rs` (L1381, the lamad/Shefa REA
  commitment the doc's era knew) AND `mishpat_integrity/src/lib.rs` (L275,
  the governance/REA compute-commitment primitive). Same name, different
  semantics, different DNAs — disambiguate by DNA whenever "Commitment"
  appears unqualified.

## The lesson (candidate)

A design doc's philosophy propagates by being *cited into* living canon
(records-lifecycle, the dna/ gospel, the notary history record); its
checklists and budget tables propagate only by being *executed*, and nothing
in the repo re-runs a checklist. Six months was enough for the philosophy to
win completely and the mechanics to execute ~one phase of three. When you
write a phased migration doc, file the unexecuted phases as backlog entries
with owners on day one — the doc itself is where they go to be forgotten.

OPEN QUESTION: whether any of the 46 `*By*` links besides `TypeToContent`
have quietly gained integrity-side consumers since December — the sweep's
per-link audit (Backfill 3) is the place that answers this; no inventory
existed as of 2026-06-11.
