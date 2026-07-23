---
id: "backlog-reach-vocabulary-frontend-strand"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Reach enum drift has a 4th (and 5th) vocabulary: the TypeScript geographic family — unrecorded in the canonical reconciliation item"
slug: "reach-vocabulary-frontend-strand"
written: "2026-06-11"
author: "claude (elohim-pillar island recompose)"
status: "backlog"
priority: "medium"
tags: [reach, vocabulary-drift, frontend, sdk, reconciliation, lamad, elohim-library, storage-client-ts, deprecation]
fingerprints: [cad8d5f51f6f, 247dc16fb9d5]   # RESOLVED 2026-07-23: aliases deleted by slice 3 (8a7ec681d); ledger lines removed — a re-fire on these fingerprints is a regression
derived_from:
  - app/elohim-app/src/app/elohim/ARCHITECTURE.md   # retired to git 2026-06-11 (elohim-pillar island recompose) — carried the geographic 8 verbatim
  - elohim/elohim-storage/REACH.md                  # retired to git 2026-06-11 (storage island recompose) — design-doc ORIGIN of the geographic 8 (§Core Mapping carries the ladder verbatim)
  - doorway/doorway-service/REACH.md                # retired to git 2026-06-11 (doorway island recompose) — sibling origin-strand; same geographic 8 as its enforcement ladder + access matrix
  - elohim/holochain/docs/REACH.md                  # retired to git 2026-06-11 (holochain docs island recompose) — the SYSTEM-WIDE overview strand; same geographic 8 (pre-reorg holochain/REACH.md — the "../REACH.md" target the doorway + imagodei dead pointers intended)
cites:
  - resilience-protocol-spec | the canonical reconciliation home — gap-matrix row :628 + roadmap item 13 :704 name only three of the (now five+) reach vocabularies | sha256:b27fc4e09bd6eb33 | path: genesis/docs/content/elohim-protocol/resilience/README.md
  - genesis/data/timeline/backlog/http-reach-enforcement-gap.md
  - app/elohim-app/src/app/elohim/models/protocol-core.model.ts
  - elohim/sdk/storage-client-ts/src/protocol-core.model.ts
  - app/lamad/src/app/models/trust-badge.model.ts
  - app/elohim-library/projects/elohim-service/src/cache/types.ts
  - app/elohim-library/projects/elohim-service/src/models/holochain.model.ts
  - elohim/sdk/schemas/v1/enums/reach.schema.json
  - steward/node/src/storage/reach.rs
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_enums.rs
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs
---

# Reach drift: the TypeScript geographic vocabulary is an unrecorded 4th strand

The canonical reconciliation item — resilience README gap-matrix row (`genesis/docs/content/elohim-protocol/resilience/README.md:628`) and roadmap item 13 (`:704`) — names THREE vocabularies: Rust services enum (`elohim/elohim-storage/src/services/epr_kind.rs:88-97` — personal/intimate/household/neighborhood/collective/community/district/public), schema enum (`elohim/sdk/schemas/v1/enums/reach.schema.json` — private/self/intimate/trusted/familiar/community/public/commons; matched by `elohim/epr/src/reach.rs:18-37`), and resilience-epic Part V (household/neighborhood/community/organization/commons).

The TypeScript side carries a **4th vocabulary, unrecorded there**: the 8-value GEOGRAPHIC family `private/invited/local/neighborhood/municipal/bioregional/regional/commons`, defined at FOUR sites:

1. `app/elohim-app/src/app/elohim/models/protocol-core.model.ts:50-72` (+ `reachEncompasses()` ordinal comparison)
2. `elohim/sdk/storage-client-ts/src/protocol-core.model.ts:50-124` — the SDK twin exporting `ReachLevel`, `REACH_LEVEL_VALUES`, `reachEncompasses()`; this is what the lamad bundle imports (`app/lamad/src/app/models/content-node.model.ts:31`, `app/lamad/src/app/quiz-engine/services/discovery-attestation.service.ts:20`, etc.)
3. `app/lamad/src/app/models/trust-badge.model.ts:20-28` (inlined copy)
4. `app/elohim-library/projects/elohim-service/src/cache/types.ts:19-40` (numeric const 0-7, same vocabulary — feeds the reach-aware cache)

**Design-doc origin (noted 2026-06-11, storage island recompose):** the geographic 8 is not a
frontend invention — it originates in `elohim/elohim-storage/REACH.md` (2026-04-15, the storage
data-plane reach design; retired to git 2026-06-11). Its §"The Core Mapping: Reach → Trust →
Action" table carries the ladder verbatim, driving designed-but-never-built encryption/eviction/
replication tiers (residue: `genesis/data/timeline/backlog/storage-island-harvest-residue.md`).
The four TS sites are descendants of that design, not drift that arose frontend-side.

**Sibling origin-strand (noted 2026-06-11, doorway island recompose):** `doorway/doorway-service/REACH.md`
(retired to git 2026-06-11; its `../REACH.md` "system-wide" pointer was already dead) carries the SAME
geographic 8 as its HTTP enforcement ladder + access matrix. Doorway's live code knows the full 8 —
`REACH_LEVELS` + `can_serve_at_reach` in `doorway/doorway-service/src/cache/access_control.rs`
(private → beneficiary match; invited → "simplified: authenticated only"; local…regional → any
authenticated; commons → everyone; unknown → deny) — but enforces LESS than REACH.md documented:
no invite-list, no relationship check, no `REACH_DENIED` error code anywhere in src/, and the only
`X-Reach` response header is hardcoded `"commons"` (`doorway/doorway-service/src/routes/apps.rs:355`).
The machinery is zero-consumer beyond its own module: `can_serve_at_reach` is called only from
`src/cache/reach_aware_serving.rs`, whose functions have no callers outside `src/cache/` — no doorway
HTTP route gates by reach this way. The live enforcement gap is already tracked in
`genesis/data/timeline/backlog/http-reach-enforcement-gap.md` (do not re-fork it here). The "7th mixed
variant" table noted in the blast-radius paragraph below (`doorway/CLAUDE.md:139-144`) was corrected to
the geographic 8, with accurate simplified-enforcement language, in the 2026-06-11 doorway island recompose.

**System-wide overview strand (noted 2026-06-11, holochain docs island recompose):**
`elohim/holochain/docs/REACH.md` (retired to git 2026-06-11) was the SYSTEM-WIDE overview of the
same geographic 8 — its level table, flow diagram, and relationship matrix carry the ladder
verbatim. Pre-reorg it lived at `holochain/REACH.md` (born 2026-01-07 in commit 403ddd460, the
SAME commit that created doorway's REACH.md at `holochain/doorway/REACH.md`; moved into
`elohim/holochain/docs/` by eb5b53133, 2026-03-10). It is therefore the "`../REACH.md`
system-wide pointer" doorway's retired REACH.md pointed at, and the `../../REACH.md` that
`elohim/holochain/dna/imagodei/STEWARDSHIP_PHILOSOPHY.md:1023` cited while quoting the geographic
ladder with ordinals 0–7 verbatim — that ref was VALID when authored 2026-01-13 at
`holochain/dna/imagodei/`, broke at the 2026-03 reorg (`elohim/holochain/REACH.md` never existed
at any commit), and was repaired 2026-06-11 to point at this strand entry. The same recompose
retires `elohim/holochain/docs/COMMUNITY-COMPUTE.md`, which sketches a 6-value SUBSET
(`Private/Invited/Local/Neighborhood/Municipal/Commons`, lines 648–656) attributed to
`elohim-storage/src/reach.rs` — a path that never existed in any commit (`git log --all` over
`elohim-storage/src/reach.rs`, `holochain/elohim-storage/src/reach.rs`,
`elohim/elohim-storage/src/reach.rs`: zero); the sketch's enum lives with the same 6 variants at
`steward/node/src/storage/reach.rs` instead — another live definition site, steward-side,
dormant (its `can_serve` is `#[allow(dead_code)]`). docs/REACH.md's §Implementation Status table
over-claims liveness on most rows; claim-by-claim verdicts at the end of this entry.

And a **5th, mutually inconsistent** 6-value family `private/invited/local/community/federated/commons`:

5. `app/elohim-library/projects/elohim-service/src/models/holochain.model.ts:319-326` — `VALID_REACH_LEVELS`, comment claims "matching Rust validation" (false: matches neither Rust enum)
6. ~~`app/elohim-app/src/app/elohim/CLAUDE.md` "Key Types" block (same 6 values)~~ — corrected to the geographic 8 (with a drift annotation) in the elohim-pillar island recompose 2026-06-11; the `VALID_REACH_LEVELS` code site remains.

Cross-bundle blast radius: 72 files reference `ReachLevel` across app/elohim-app, app/lamad, app/elohim-library (incl. dist/spec); 96 non-spec literal usages of geographic values (`bioregional`/`municipal`/`neighborhood`) across the three trees, e.g. `app/lamad/src/app/models/content-attestation.model.ts:76` maps attestation types to geographic reaches, `app/lamad/src/app/services/content.service.ts:97,358` defaults to `bioregional`. ~~Doorway's documented table is a 7th mixed variant (`doorway/CLAUDE.md:139-144` — commons/regional-private/local/private)~~ — corrected to the geographic 8 (with accurate simplified-enforcement language) in the doorway island recompose 2026-06-11; the gospel no longer carries the mixed 4-row table.

**Why this matters for roadmap item 13**: a reconciliation scoped only to Rust/schema/epic will under-scope. The TS geographic ordinals feed `reachEncompasses()` comparisons and reach-aware cache eviction; renaming the vocabulary changes ordinal semantics across ~70 consuming files in three separately-built bundles, two of which (lamad via `@elohim/storage-client`, elohim-library locally) cannot be fixed by editing elohim-app alone.

**Action**: when roadmap item 13 is picked up, extend the gap-matrix row at `resilience/README.md:628` to name the TS strands, and treat `elohim/sdk/storage-client-ts/src/protocol-core.model.ts` (hand-written, NOT ts-rs-generated despite living in the SDK) as the single TS edit point — the other three sites should re-export rather than redefine.

---

## docs/REACH.md §Implementation Status — claim-by-claim verdicts (verified 2026-06-11)

Liveness verdicts correcting the retired system-wide overview's status table (precedent: the
doorway-enforcement paragraph above — reach-LIVENESS verdicts live here when they correct a
retired doc's claims; unconsumed storage scaffolding stays in
`genesis/data/timeline/backlog/storage-island-harvest-residue.md`; do not record twice).

- **"DNA (content_store) | Entry validation | ✅ Implemented" — mechanism TRUE, vocabulary
  FALSE.** Content entries DO validate reach at create: the integrity validate callback
  (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:4278` →
  `content.validate()`) runs the check at `content_store_integrity/src/healing.rs:73` against
  `ALL_REACH_LEVELS` — which is the schema 8
  (`private/self/intimate/trusted/familiar/community/public/commons`,
  `content_store_integrity/src/generated_enums.rs:315,327`; CORE and ALL are identical).
  Coordinator-side checks agree: `content_store/src/providers.rs:56` and
  `content_store/src/republish_epr.rs:67-78` (inline list, same 8). Net: SIX of docs/REACH.md's
  documented eight values (`invited`, `local`, `neighborhood`, `municipal`, `bioregional`,
  `regional`) would be REJECTED by the very validation the table cites as implementing them. The
  Content struct's own comment says `// Visibility: private, community, public, commons`
  (`lib.rs:502`) — a fourth-different sketch in the same crate.
- **"Doorway | HTTP/WS gating | ✅ Implemented" — FALSE as stated.** Already recorded in this
  entry's doorway paragraph: no doorway HTTP route gates by reach; `can_serve_at_reach` is
  zero-consumer outside `src/cache/`; the only `X-Reach` response header is hardcoded
  `"commons"` (`doorway/doorway-service/src/routes/apps.rs:355`). Tracked:
  `genesis/data/timeline/backlog/http-reach-enforcement-gap.md`.
- **"elohim-storage | Reach field tracking | ✅ Implemented" — fields exist, but the live
  enforcement MODEL is different in kind.** Live storage-side reach is author-side earning +
  receiver-side pre-authorization (`elohim/elohim-storage/src/p2p/reach_authorization.rs` —
  Stage 1 structural; consumes `elohim_epr::Reach`, i.e. the schema 8 of
  `elohim/epr/src/reach.rs`), NOT the receive/delivery-side filtering docs/REACH.md describes
  ("Gates blob replication to peers"). The module doc-comment explicitly rejects receive-side
  filtering ("It is NOT a per-message filter").
- **"Delivery gating / Replication gating | 🔄 Partial (sovereignty only)" — the sovereignty
  scaffolding is recorded as UNCONSUMED** (`storage-island-harvest-residue.md`; routed from
  `elohim/elohim-storage/CLAUDE.md` §P2P Data Plane & Reach).
- **"Encryption at rest / LRU by reach priority / RS shard distribution | ❌ Not started" —
  accurate.** Designed-never-built; same residue entry.
- **"Agent apps | DHT validation | ✅ Inherent" — true in mechanism** (DNA validation runs on
  every conductor) **but it enforces the schema 8**, not the geographic 8 the doc documents.
- **"Geographic verification | 📋 Planned" — no implementation found** anywhere (no
  location-based reach code in dna/ or elohim-storage).
- **CustodianCommitment ("Custodian Override") — entry type LIVE, override semantics NOT.** A
  `CustodianCommitment` entry type exists in the elohim DNA
  (`content_store_integrity/src/lib.rs:3334`; documented as the substrate primitive for the
  Sheila community-custody case at `genesis/docs/content/elohim-protocol/resilience/README.md:365`),
  with coordinator create/fetch + wire conversion (`content_store/src/lib.rs:129,136,611-629`
  via `shefa_types`). But its shape is custody/replication/recovery (content_filters_json —
  which may carry reach levels — shard topology, emergency_triggers_json,
  recovery_instructions_json), NOT docs/REACH.md's sketch
  (`custodian/beneficiary/category/approved_reach_levels`). No code path grants a custodian
  ACCESS beyond their reach level — there is no reach-gated access anywhere for an override to
  pierce (see doorway verdict). Declared data, not enforced behavior. (Adjacent: the modern
  commitment primitive is `Mishpat::Commitment`, `mishpat_integrity/src/lib.rs:275` — a separate
  lineage; the content_store CustodianCommitment predates it.)
- **Emergency escalation — declared fields, no activation path.** Five emergency-trigger types
  exist as JSON-string fields on CustodianCommitment (resilience README:365 enumerates them); no
  coordinator or storage code found that ACTS on a trigger to expand reach (and imagodei's
  `steward_emergency` / `emergency_access_enabled` machinery,
  `imagodei_integrity/src/lib.rs:215,396`, is KEY-RECOVERY quorum, not content-reach
  escalation). Vision beyond the entry-type declaration.
- **Reach promotion — NOT implemented.** No promote/promotion code in `dna/elohim/zomes/` or
  `elohim/elohim-storage/src/` (only WAL-promotion hits in `db/mod.rs`).
  `content_store/src/attestation.rs:273` and `content_store/src/migration.rs:241` COPY/preserve
  the original reach; no audit-trail-preserving promotion mechanism exists.

---

## Slice-1 disposition (2026-07-23, shift/reach-vocab-slice1)

Per `reach-ontology-vocabulary-split-spec` §1: vocabulary **#2 (Rust services kebab-8)** and
**#5 (`VALID_REACH_LEVELS` 6)** are RETIRED — elohim-storage re-exports `elohim_epr::Reach`
(schema-8) with `ReachStandingExt` floor semantics and a legacy-alias parser
(`parse_reach_key`; data-aware migration, old manifests keep evaluating); both TS
`VALID_REACH_LEVELS` definitions deleted (zero consumers). Drift test:
`elohim-storage/tests/reach_vocabulary_contract.rs` pins Rust↔schema.
Remaining strands: geographic-8 rename (locality) and Part-V custody rename — later slices.

## Slice-2 disposition (2026-07-23, shift/reach-vocab-slice2)

Per `reach-ontology-vocabulary-split-spec` §1, the two strands slice-1 deferred are now dispositioned:

- **TS geographic-8 RENAMED → `LocalityLevel`** (`afd8ee1c8`). Single edit point stays
  `elohim/sdk/storage-client-ts/src/protocol-core.model.ts` — `ReachLevel`, `REACH_LEVEL_VALUES`,
  `reachEncompasses` are kept as `@deprecated` aliases (`/** @deprecated Renamed 2026-07-23 — use
  LocalityLevel. */`, lines 78/80/136) so the ~37 existing consumers keep compiling; the other
  three sites (`app/elohim-app/.../protocol-core.model.ts`, `app/lamad/.../trust-badge.model.ts`,
  `app/elohim-library/.../cache/types.ts`) re-export rather than redefine. Burn-down of the 37
  call sites onto `LocalityLevel` is **open**, tracked by +23 deprecation-lint findings surfaced
  once the aliases landed (deprecation-triage backlog, not re-litigated here). Reminder for
  whoever burns the aliases down: SDK edits need `pnpm build` before app/lamad tsconfigs see them
  — both resolve `@elohim/storage-client` via compiled `dist/`, not source (dist-freshness trap).

- **Part-V custody 5 NAMED, not renamed.** `resilience/README.md` Part-V's `household /
  neighborhood / community / organization / commons` ladder is confirmed a distinct **custody**
  vocabulary — who holds/replicates for whom, anchored on the `CustodianCommitment` /
  `Mishpat::Commitment` lineage — never a reach vocabulary. No code changed (there was never a
  drifted implementation to retire, only prose ambiguity); the gap-matrix row and a clarifying
  sentence at the Part-V ladder occurrence now say so explicitly
  (`resilience/README.md:290,628`).

**Live-degradation fix folded into this slice:** `ReachClass` (the distribution-view replica
ladder — `elohim-storage/src/graph_views/shefa/distribution.rs`,
`services/distribution_view.rs`) now speaks schema-8 directly
(`private2/self2/intimate4/trusted6/familiar8/community12/public14/commons16`; `07adc0ccc`). This
**fixed a live degradation**: declared `trusted`/`familiar`/`commons` content was silently
computing a 2-replica `Private`-floor target instead of its declared 6/8/16-replica ladder.
`parse_reach_class` (`distribution_view.rs:312`) keeps a legacy-alias heuristic for stored rows —
old `"public"` (pre-migration top rung) still parses to `Public`/14, deliberately diverging from
`parse_reach_key`'s legacy `"public"→Commons` mapping (services enum retirement, slice-1), because
one reads old stored data and the other parses canonical wire keys; both sides are documented at
their own definition, not reconciled into one function.

**FOLLOW-UP (reviewer-proposed, accepted) — open, not yet scheduled:** stored legacy-ambiguous
`"public"` rows parse canonically (→`Public`/14) for now; that is a stand-in, not the durable
state. The durable fix is a **one-time backfill** disambiguating pre-migration `"public"` rows to
`"commons"` where that was the original intent, after which `parse_reach_class` retires its
legacy-alias heuristic entirely and reads the column as pure schema-8. Until the backfill runs,
any pre-migration row whose original `"public"` meant the old top rung (now `commons`) will
under-replicate at 14 instead of 16 — narrow blast radius (replica count only, not access control)
but real.

**NEWLY-DISCOVERED live strand (out of scope here, slice-3 territory):** `elohim-library`'s
`ContentReach` (`app/elohim-library/projects/elohim-service/src/models/content-node.model.ts:46`)
and `trust.service.ts`'s own `ReachLevel`
(`app/elohim-library/projects/elohim-service/src/services/trust.service.ts:14`) independently
duplicate the retired-6 vocabulary (`private/invited/local/community/federated/commons`) —
missed by both slice-1's `VALID_REACH_LEVELS` deletion (different type name, same values) and
slice-2's `LocalityLevel` rename (different package, not re-exporting the SDK type). Flagged
2026-07-23 (`2acb47731`); ~41 identifier refs package-wide including ~40 spec literal usages.
Needs its own migration task, not swept into this slice.

**CORRECTION to the slice-1 final-review note:** the note that 3 seed rows carried a stray
`"agent-private"` reach value was a **misdiagnosis**. Those 3 occurrences
(`elohim/sdk/domains/{infrastructure,lamad}/*.json`) belong to `observation-kind.schema.json`'s
own live, schema-valid enum — `agent-private / household / community / commons /
commons-attested` (`elohim/sdk/schemas/v1/manifest/observation-kind.schema.json:30`) — a distinct
event-observation vocabulary, unrelated to content-declared `Reach`. Confirmed 2026-07-23
(`2acb47731`); repo-wide `genesis/data` has zero `"agent-private"` matches, so there was never
drifted data to migrate. **No action needed, ever** — recorded here so a future pass doesn't
"fix" a vocabulary that was never broken.

**Attestation data migrated** (completing the retired-6→canonical mapping on stored data):
`federated`→`public` (5 rows, `2acb47731`), `local`→`trusted` (4 rows, `5844682fc`), both in
`genesis/data/lamad/attestations/index.json`.

**Still open, unchanged from slice-1:** `steward/node/src/storage/reach.rs`'s dormant 6-value
enum (`#[allow(dead_code)]` `can_serve`) — not reconciled, not scheduled. Verdict surface (spec §7
item 3) and fixture harness (item 4) remain later slices, as does the a2o composition-law
scenario suite (item 6).

## Slice-3 alias burn-down — deprecation-triage canonicalization (2026-07-23)

The deprecation-sentinel captured two NEW @deprecated-tag fingerprints once slice-2's aliases
landed in `elohim/sdk/storage-client-ts/src/protocol-core.model.ts` (`afd8ee1c8`):

- **`cad8d5f51f6f`** — `REACH_LEVEL_VALUES` alias (`:80`, "use LOCALITY_LEVEL_VALUES")
- **`247dc16fb9d5`** — `reachEncompasses()` alias (`:136`, "use localityEncompasses")

(The sibling `ReachLevel` type alias at `:78` is the same concern; if/when the sentinel surfaces
its fingerprint it maps here too.)

**Current decision — owned by the active slice-3 arc; deprecation-triage takes NO code action.**
These aliases are the *deliberate* migration bridge slice-2 authored so the ~37 existing
`ReachLevel`/`REACH_LEVEL_VALUES`/`reachEncompasses` consumers keep compiling. Their burn-down onto
`LocalityLevel`/`LOCALITY_LEVEL_VALUES`/`localityEncompasses` is the **first workstream of slice 3**,
actively in flight under `genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md`
by the main session's slice-3 implementers **in this shared worktree**. A background deprecation-fix
here would collide with those in-flight edits, so the correct disposition is
canonicalize-and-suppress, not fix: the ledger entries are marked `triaged` pointing at this doc,
so the sentinel stops re-firing. **The ledger lines are cleared (deleted) by whoever completes the
slice-3 burn-down — deleting the `@deprecated` alias definitions is what makes the tag fingerprints
read as gone; a reintroduced alias then correctly re-fires as NEW.** Do not re-scan or re-fork this
concern; it is fully recorded here and in the slice-2 disposition above.

Reminder carried from slice-2 for the burn-down implementer: SDK edits need `pnpm build` before
app/lamad tsconfigs see them (both resolve `@elohim/storage-client` via compiled `dist/`, not
source — the dist-freshness trap).

## Slice-3 dispositions (2026-07-23)

Slice 3 (`shift/reach-vocab-slice3`, six tasks) closes out per
`reach-ontology-vocabulary-split-spec` §7 Definition-of-Done items 2 and 5. Final dispositions:

- **Library retired-6 strand migrated to canonical** (`c20b64e6f`). The independently-drifted
  `elohim-library` duplicates flagged at the end of slice-2 (`ContentReach` at
  `app/elohim-library/projects/elohim-service/src/models/content-node.model.ts:46` and
  `trust.service.ts`'s own `ReachLevel`) are reconciled onto the canonical vocabulary.
- **Aliases burned down, then DELETED** (`8a7ec681d`). The ~37 call sites onto
  `ReachLevel`/`REACH_LEVEL_VALUES`/`reachEncompasses` (slice-2's deliberate migration bridge) are
  moved onto `LocalityLevel`/`LOCALITY_LEVEL_VALUES`/`localityEncompasses`, and the `@deprecated`
  alias definitions themselves are deleted from
  `elohim/sdk/storage-client-ts/src/protocol-core.model.ts`. **The two sentinel fingerprints this
  strand recorded above — `cad8d5f51f6f` (`REACH_LEVEL_VALUES` alias) and `247dc16fb9d5`
  (`reachEncompasses()` alias) — are DELETED along with the aliases they tag.** Their ledger lines
  are gone because the tagged code is gone; the deprecation-sentinel's regression re-fire is
  **armed**: if either alias (or the sibling `ReachLevel` type alias) is ever reintroduced, it is a
  NEW fingerprint, not a resurrection of these two, and should be triaged fresh rather than pointed
  back at this (now-historical) disposition.
- **`reachIcon` complete** (`c9759c7d8`). The reach→icon presentation mapping is migrated onto the
  canonical vocabulary alongside the alias burn-down.
- **Bootstrap standing-policy canonical** (`248c13db5`). `reachThresholds` in the bootstrap policy
  is the canonical 8-key schema-8 shape (`private/self/intimate/trusted/familiar/community/public/
  commons`) — no more geographic-8 leakage into standing-policy config.
- **`content.reach` one-time canonicalization migration** (`d9fcd353c`). Pre-migration rows whose
  stored `content.reach` value used the old top-rung sense of `"public"` are backfilled to
  `"commons"` — the durable fix slice-2's `parse_reach_class` follow-up called for (§ "FOLLOW-UP
  (reviewer-proposed, accepted)" above). Downstream: `elohim/elohim-storage/src/services/epr_kind.rs`'s
  `legacy_manifest_keys_still_parse` test comment is corrected (comment-only, this task) to state
  that stored `"public"` now genuinely means canonical `Reach::Public` post-migration, since the
  ambiguous historical rows have already been moved to `"commons"`.
- **Steward `reach.rs` documented as locality-seed, not canonized** (this task, comment-only). Its
  6-value `Reach` enum + `replication_policy` matrix (`FullSync`/`MetadataOnly`/`OnDemand`/`Skip`
  over content-locality × peer-trust) is recorded, in a new module doc-comment, as the seed of the
  locality/placement engine the reconciliation spec sequences *behind* itself (§7 out-of-scope),
  citing the spec and naming SDK `LocalityLevel` as the future alignment target if that engine is
  ever built. No rename, no deletion, no code change — `sync/coordinator.rs:99` still constructs a
  live `Reach::Local` default, so the enum is not fully inert, but nothing in it is canon.
- **SDK `LocalityLevel` source-of-record declaration added** (this task, comment-only). The block
  in `elohim/sdk/storage-client-ts/src/protocol-core.model.ts` now states explicitly that this file
  is the single generative source-of-record for the locality vocabulary (drift-prevention law, spec
  §1), lists the known projections (app `elohim/models/protocol-core.model.ts` re-export, library
  `cache/types.ts` mirror), and names steward `reach.rs` as a future-alignment target rather than a
  current projection.

**Two NEWLY-DISCOVERED same-name vocabularies, recorded here so they cannot hide** (found during
slice-3 closeout; NOT touched by any slice-3 task — future work, not urgent):

- `app/elohim-library/projects/elohim-service/src/client/types.ts:148` — a numeric `ReachLevel`
  **enum** (`Commons = 0` … `Private = 7`, TypeScript `enum`, not a union type), independent of both
  the retired-6 library duplicate (migrated above, lived in `cache/types.ts`/`content-node.model.ts`/
  `trust.service.ts`) and the SDK `LocalityLevel`. Same 8-value geographic ladder, reversed ordinal
  direction (0=Commons here vs. 7=Commons in `LOCALITY_LEVEL_VALUES`), same name as the now-deleted
  SDK alias. **Untouched, still live.** Needs its own migration task before it can be called
  reconciled — do not assume the alias burn-down above swept it; it is a different package,
  different definition, different file.
- `elohim/sdk/src/types.ts:1211` — a hand-written schema-8 `ReachLevels` const object +
  `ReachLevel` type (`private/self/intimate/trusted/familiar/community/public/commons`), living in
  `elohim/sdk/src/` (the hand-written SDK-helpers directory, NOT `storage-client-ts/generated/` and
  NOT ts-rs-anchored). Its *values* are already canonical schema-8 — this is not vocabulary drift
  in the ontological sense slice-1/slice-2/slice-3 fixed — but it is a **second hand-typed
  definition site** for the same 8 values the schema (`elohim/sdk/schemas/v1/enums/reach.schema.json`)
  already generates from. Per the drift-prevention law (spec §1: "exactly ONE generative
  source-of-record per vocabulary; every other appearance is a generated projection or an explicit
  re-export"), this is a candidate for codegen alignment (re-export from the generated schema enum,
  or delete in favor of it) in a later pass — not urgent since values agree today, but a latent
  drift site the moment the schema enum changes and this hand-written copy is not updated in
  lockstep.

**Remaining slice-4 queue** (per `reach-ontology-vocabulary-split-spec` §7 Definition-of-Done,
items 3, 4, 6, and the doorway residue of item 5 — none started, no code exists for any of these):

- **Verdict surface** (spec §7.3) — the generalized `verdict(content, viewer?, announcement?,
  freshness) → { Allowed | Blocked | Pending, evidence, explain? }` route + Rust shape, generalizing
  today's `ReachVerdict`/`StandingEvidence` (`reach_earning.rs`) and the `reach_authorization`
  pre-auth stage. Not designed yet beyond the spec's guiding principles.
- **Fixture harness** (spec §7.4) — offline, deterministic "given these declarations/tuples/
  commitments, agent A sees exactly {…}" invariant tests for the verdict function (SpiceDB `zed
  validate` pattern). Blocked on the verdict surface existing to test.
- **Doorway `can_serve_at_reach` re-keying + `reach:`-named `LocalityLevel` wire-field residue**
  (spec §7.5 residue) — `doorway/doorway-service/src/cache/access_control.rs`'s `REACH_LEVELS` +
  `can_serve_at_reach` still speak the geographic-8/locality vocabulary under the name "reach";
  per this strand's earlier doorway paragraph, the function is also zero-consumer outside
  `src/cache/` (no live HTTP route gates by it), so this re-keying is a rename-for-honesty pass, not
  a live-behavior fix. Also unreconciled: any wire field literally named `reach` that carries
  `LocalityLevel` values (e.g. `ContentVisibility.reach`, `GeographicContext.reach`,
  `Attestation.reach` in `protocol-core.model.ts` itself) — the *field name* `reach` on a
  `LocalityLevel`-typed value is itself a small residual name collision the rename didn't chase into
  every struct.
- **Composition-law a2o scenarios** (spec §7.6) — regression stories for narrow-never-widen,
  anonymous→commons, revocation-orders-before-serve. Not written.
- **`content.reach` schema `DEFAULT 'public'` open-by-default smell** — noted but not fixed in this
  slice: the column/schema default for `content.reach` is `'public'` (open-by-default) rather than a
  more conservative floor. Flagging here as a smell for whoever next touches the content-creation
  path; not a vocabulary-drift item per se, but adjacent enough to record alongside the migration
  that just ran.
