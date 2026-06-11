---
title: "History: The lamad v1 schema museum — the RNA toolkit's companion snapshot vs what actually grew (Dec 2025 – Jun 2026)"
id: lamad-v1-schema-museum
type: history-gotcha
status: noted
tier: history
created: 2026-06-11
topic: [holochain, dna, lamad, schema, entry-types, migration, mastery-levels, rna-toolkit, v1]
# Provenance breadcrumb: the retiring island doc this record distills.
derived_from:
  - elohim/holochain/dna/SCHEMA_VERSIONS.md  # retired to git 2026-06-11 (holochain dna/ island recompose; authored 2025-12-14 in the RNA-migration-toolkit commit a815c944c, never content-updated afterward)
canonical:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-11-bloom-mastery-progression-design.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-attestation-consolidation-phase2a-dedup.md
  - genesis/data/timeline/backlog/deprecation-learning-path-zome-surface-retire.md
cites:
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/holochain/dna/lamad-v1/zomes/content_store/src/lib.rs
  - elohim/holochain/rna/typescript/src/config.ts
  - elohim/holochain/rna/rust/src/config.rs
  - elohim/sdk/schemas/v1/enums/mastery-level.schema.json
  - elohim/elohim-storage/src/db/models.rs
  - elohim/holochain/dna/hrea/workdir/README.md
  - elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs
  - lamad-deleted-concepts-2024 | evidence lamad predates the monorepo — keeps the v1 table 2024-12 date plausible as predecessor-repo history | sha256:8b1b969c2132ec61 | path: genesis/docs/content/elohim-protocol/history/2026-06-11-lamad-deleted-concepts-2024.md
---

# History: The lamad v1 schema museum (Dec 2025 – Jun 2026)

> **Hot-context pointer (the one sentence to remember):**
> The v1 schema snapshot documented 30 entry types; the live enum holds 75 —
> yet 29 of the 30 survive unchanged in name, the migration-export seam it
> specified is live in the *current* coordinator, and the 8-level mastery
> scale it recorded outlived every structure around it. Schemas here grow by
> accretion, almost never by removal — plan migration tooling for addition,
> not churn.

## Provenance: born as the RNA toolkit's companion

SCHEMA_VERSIONS.md was created 2025-12-14 in the same commit as the
Holochain RNA migration toolkit (`a815c944c`, "feat: add Holochain RNA
migration toolkit") — it was the toolkit's human-readable companion, listing
the v1 surface the toolkit would export. The pairing is still visible: the
toolkit's defaults are wired to exactly the export functions the doc lists
(`elohim/holochain/rna/typescript/src/config.ts` L43–46
`exportFn: 'export_for_migration'` / `versionFn: 'export_schema_version'`;
`elohim/holochain/rna/rust/src/config.rs` L116–136 same defaults). It was
never content-updated after December 2025 and has **zero inbound references
repo-wide** (verified 2026-06-11) — a clean retirement.

Date caveat: the doc's version-history row says "v1 | 2024-12". That date is
unverifiable in this tree (the monorepo's first commit is 2025-08-14; the file
itself was created 2025-12-14). Lamad work demonstrably predates the monorepo
(the Nov-2024 DocumentNode-deletion arc:
genesis/docs/content/elohim-protocol/history/2026-06-11-lamad-deleted-concepts-2024.md),
so "2024-12" may be genuine predecessor-repo history rather than a typo for
2025-12. OPEN QUESTION — undecidable from this tree.

## What the snapshot recorded vs what grew

Measured 2026-06-11 in `content_store_integrity/src/lib.rs` (entry enum at
~L3670): **75 entry types**. The doc's tables document **30** (5 knowledge +
7 user-state + 7 economic + 3 intent/commitment + 4 gamification + 4
access-control). The delta is almost pure accretion — blob/shard entries,
EPR-phase entries (Manifest, FeedbackSignal, AttentionTending,
CollectiveFilterPattern), the whole Shefa expansion (insurance mutual,
requests/offers, stewarded resources, constitutional limits, flow planning),
content succession, doorway federation.

Of the 30 documented types, **29 survive today under their original names**.
The single removal: `Attestation` — collapsed in the attestation
consolidation (tombstone at `content_store_integrity/src/lib.rs` ~L1090
"REMOVED: `Attestation` DHT entry type (attestation-consolidation C.4)";
the arc is canonical in the 2026-06-02 attestation-consolidation history
record, 18+ attestation-shaped types → one discriminated Content).

Survival ≠ health, though:

- **LearningPath / PathStep / PathChapter** survive in the enum but the zome
  surface is formally deprecated — paths are `Content` entries with
  `content_type = "path"`, `content_format = "epr-composite"`; the legacy
  write functions return errors (DEPRECATED markers at
  `content_store_integrity/src/lib.rs` L721–810). Canonical tracking:
  `genesis/data/timeline/backlog/deprecation-learning-path-zome-surface-retire.md`.
- **Human / HumanProgress** survive marked "Legacy, kept for backward
  compatibility" (~L3710) — their primary home is now the imagodei DNA.
- **Commitment** survives in lamad (struct at L1381) but is now shadowed by
  the *other* `Commitment` in `mishpat_integrity/src/lib.rs` (L275) — the
  governance/compute-commitment primitive. The doc's "Intent & Commitment"
  table describes the lamad one only; unqualified "Commitment" is ambiguous
  across bundles today.
- The doc's "Known Future Changes" note "consider whether hREA should be
  used instead" landed, in evolved form: hREA is consumed as an external DNA
  bundle (`elohim/holochain/dna/hrea/workdir/README.md` — Wave 3 projection
  target via the `bridges/valueflows` bridge), without removing the lamad
  economic types.

The doc's eight "key link types" (IdToContent, TypeToContent, TagToContent,
AuthorToContent, IdToPath, PathToStep, AgentToPathProgress, HumanToMastery)
all still exist in the link enum — including the four that the sibling
LINK_ARCHITECTURE doc simultaneously ordered deprecated or removed; see the
link-architecture arc record for that contradiction's outcome. The
`all_paths/index` and `agent_progress/{agent_id}` anchors survive verbatim
(`content_store/src/lib.rs` ~L3791, ~L4671).

## The migration-export seam is LIVE — in the current coordinator

The doc's listed export functions are not archive material: they exist today
in the **current** coordinator (`content_store/src/lib.rs`:
`export_schema_version` ~L7962, `export_all_content` ~L7969,
`export_all_paths_with_steps` ~L8009, `export_all_mastery` ~L8079,
`export_all_progress` ~L8086, `MigrationExport` + `export_for_migration`
~L8092–8103), with cache rules registered (~L1530–1545). The
`dna/lamad-v1/` directory is a thin **v1 coordinator archive for healing
migration** ("v2 queries v1 via bridge calls" — its lib.rs header), with
per-id exports in its `healing_exports` module. The seam the snapshot
specified became permanent infrastructure: schema evolution support also
grew *into* the entry structs as `schema_version: u32` +
`validation_status` self-healing fields (Content struct ~L490, ContentMastery
~L1066) — which also satisfies, in evolved shape, the doc's anticipated
"adding version field to Content".

## v1 design decisions — each re-verified against current structs (2026-06-11)

1. **`metadata_json: String` extensibility — still holds.** Live on Content
   (~L507) and 40 other sites in the integrity zome; still the
   add-fields-here-first pressure valve.
2. **String IDs — still holds.** `Content.id: String`,
   `ContentMastery.id: String` etc.; content addressing arrived elsewhere
   (blob_cid manifest fields on Content) without displacing string ids.
3. **String timestamps — still holds.** `created_at: String` /
   `updated_at: String` on both structs verified.
4. **The 8-level mastery scale — outlived everything.** The exact ladder the
   doc lists (`not_started, seen, remember, understand, apply, analyze,
   evaluate, create`) is the protocol schema enum today
   (`elohim/sdk/schemas/v1/enums/mastery-level.schema.json` L7–11) and is
   canonized with engagement semantics in the bloom-mastery-progression
   design (2026-06-11). One drift: the DNA coordinators index level 1 as
   `"aware"`, not `"seen"` (`content_store/src/lib.rs` L892–904; imagodei
   coordinator L1527); elohim-storage treats `"aware"` as a legacy alias for
   `"seen"` (`src/db/models.rs` L484, L504). OPEN QUESTION: whether the
   coordinator helpers get reconciled to the schema vocabulary or the alias
   stays load-bearing forever — the bloom design carries the adjacent open
   questions (its §7).

## The lesson (candidate)

A schema snapshot here decays by *omission*, not by *falsehood*: six months
on, almost nothing the v1 doc recorded is wrong — it is simply 40% of the
truth, and silently so (zero inbound references meant nothing ever forced an
update). The durable artifacts were the *seams* it specified (export
functions, metadata_json, the level ladder), not the inventory tables.
Document the seams in living canon; let `git log` keep the inventories.

OPEN QUESTION: the doc's other anticipated v2 changes — structured
`metadata` replacing the JSON string, `archived_at` soft deletes, global
content index / pagination — have no verified implementation as of
2026-06-11 (not checked exhaustively this session; no claim either way).
