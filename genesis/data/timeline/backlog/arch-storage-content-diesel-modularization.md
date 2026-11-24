---
id: "backlog-arch-storage-content-diesel-modularization"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Modularize elohim-storage db/content_diesel.rs into a strict directory module by concern — exception-hygiene paydown first, then one move per commit, db::content_diesel path stable (finding bad2e3aafc10)"
slug: "arch-storage-content-diesel-modularization"
written: "2026-10-08"
revised: "2026-10-23"
author: "rust-architect (architecture finding bad2e3aafc10)"
status: "refined"
priority: "medium"
area: "elohim-storage/db"
domain: "D-dataplane"
finding: "bad2e3aafc10"
policy: "source-file-loc-ceiling@1"
tags: [architecture, refactor, elohim-storage, diesel, loc-ceiling, head-election, mod-decomposition, rust-style, rs-gov-04, exception-hygiene, decision, constraint]
relatedNodeIds:
  - backlog-elohim-storage-p2p-modrs-modularization
  - backlog-arch-content-store-zome-modularization
  - backlog-head-authority-carried-with-content-sync-unit
  - backlog-arch-workspace-discipline-backlog
cites:
  - elohim/elohim-storage/src/db/content_diesel.rs
  - elohim/elohim-storage/seam-registry.yaml
  - .claude/epr-meta/policies.yaml
  - .claude/scripts/_lib/epr_meta.py
  - .claude/hooks/rust-style.py
  - genesis/docs/rust-style-guide.md
  - genesis/orchestrator/rust-style/.epr-meta
  - genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md
shift_objective: |
  Turn elohim/elohim-storage/src/db/content_diesel.rs (9,312 lines at 28a561cb3; the
  ceiling is 7,000) into a strict directory module db/content_diesel/, in the order
  this entry fixes:
  STEP 1, one commit with no code motion: pay down all six RS-GOV-04 suppressions in
  the file (two type_complexity allows become named row types, the deprecated
  count_distinct becomes count(..).aggregate_distinct(), and the three
  too_many_arguments allows become #[expect(.., reason = "..")]). Apply it as one
  scripted change, because the edit hook denies any Edit that leaves the file with an
  unreasoned suppression. Prove it with rust-style.sh --hook on the file (EXIT=0) and
  `just gate elohim-storage`.
  STEP 2, move 0: git-rename the file to content_diesel/mod.rs and bind
  rust-style-strict@1 plus the lowered rs-loc-ceiling in a file-form .epr-meta in the
  new directory.
  STEPS 3+: one bounded move per commit, smallest first. Each move is code motion
  only: items travel with their own tests, seam-registry contract tests stay in the
  file that holds the guard they prove, and mod.rs re-exports, so every
  crate::db::content_diesel::X path compiles unedited.
  Gate every commit with `just gate elohim-storage` (echo EXIT=$? on its own line,
  never piped), the seam census and seam-audit --spot-check. No behaviour change, no
  renames, no new logic. Each push after the one carrying STEP 1 must remove at least
  one other elohim-storage RS-GOV-04 finding (RS-GOV-02).
---

# Modularize `db/content_diesel.rs`

## Finding

`elohim/elohim-storage/src/db/content_diesel.rs` is over the `source-file-loc-ceiling@1`
hard ceiling (7,000). It measured 9,002 lines when the finding fired, 8,726 on 2026-10-08,
and **9,312 at `28a561cb3` (2026-10-23)**. It is still growing.

The file splits into two parts:

- **Production code:** lines 1–3958.
- **The `#[cfg(test)] mod tests` block:** lines 3959–9312. That is 5,354 lines holding 87
  `#[test]`s, or 57% of the file.

Moving each test with the code it tests therefore does most of the work. 60 files in `src/`
and `tests/` reference `content_diesel::` (611 occurrences). The heaviest are
`content_service.rs`, `http.rs`, `sync/projector.rs`, `head_adoption.rs`,
`projection_reconcile.rs` and `rea_projection.rs`.

**Refactor-safety class (policy `why`).** This is plain native Rust in a non-DNA crate, so the
gate is fmt, clippy `-D warnings` and the full crate `cargo test`. Neither zome rule applies:
no coordinator hot-swap via `update_coordinators`, and no integrity edit that would move the
DNA hash. No DNA, conductor or reinstall concern rides on any move. The policy also requires
that this work never happen mid-edit. Every step below is its own bounded commit, never folded
into feature work that happens to touch the file.

## Revision 2026-10-23: what changed since the 2026-10-08 plan

1. **The Rust style guide is active** (`genesis/docs/rust-style-guide.md`, RS-GOV-02/03/04).
   The file carries six unreasoned suppressions. The edit hook (`.claude/hooks/rust-style.py`)
   checks the whole prospective file, so it denies any Edit or Write that leaves even one of
   them unreasoned. Finding identity includes the path, so a move would turn all six into new
   findings. Exception-hygiene paydown is therefore STEP 1, ahead of every move.
2. **`DocHeadOrdering` has landed** (line 2197, with `From<&Content>` and
   `From<&CanonicalElectionWire>`). The old precondition on the head-election moves is met.
3. **A new cluster has appeared: local authorship.** It covers `AUTHORED_VIA_*`,
   `mark/clear/is_locally_authored`, `locally_authored_ids` and the
   `count_*_not_locally_authored` functions (lines 3764–3910).
4. **A flat `stamp.rs` would land at about 3,200 lines**, over the 3,000 soft ceiling. It
   becomes a `stamp/` directory instead (see "C2 co-location" below).
5. **Correction to the old plan.** `c2-monotonic-authority@2` has landed. Under the registry's
   never-mutate contract its prose cannot be re-hashed in place. See the blast radius section.

## STEP 1: RS-GOV-04 exception-hygiene paydown (one commit, no code motion)

The native checker (`rust-style.sh --hook --path <file> --stdin`, EXIT=1) reports exactly these
six. Identity is the enclosing item plus the normalized attribute:

| Line | Enclosing item | Attribute | Treatment |
|---|---|---|---|
| 2453 | `stamp_declared_head_mode` | `allow(clippy::too_many_arguments)` | `#[expect(clippy::too_many_arguments, reason = "…")]`. The doc comment above it already states the invariant: every argument is explicit at each call site, because a builder is how a heal path quietly acquires `Declare` semantics. |
| 2553 | `stamp_declared_head_witnessed` | same | `expect` with the same reason, plus that it is the witnessed arm of the same explicit-argument contract |
| 2648 | `stamp_declared_head_mode_transaction` | same | `expect` with the same reason, plus that it is the transaction body that the two wrappers forward into |
| 2662 | `stamp_declared_head_mode_transaction::let existing` | `allow(clippy::type_complexity)` | **Delete the suppression.** Add a module-level named row type (for example `type StampExistingColumns = (Option<String>, Option<i64>, …, String, …)`) and annotate `let existing: Option<StampExistingColumns>`. Clippy does not flag type aliases. |
| 3027 | `heal_election_columns::let outcome::let existing` | `allow(clippy::type_complexity)` | **Delete the suppression** the same way (for example `type ElectionHealColumns`) |
| 3628 | `tag_count` | `allow(deprecated)` | **Delete the suppression.** Replace `diesel::dsl::count_distinct(content_tags::tag)` with `diesel::dsl::count(content_tags::tag).aggregate_distinct()`. diesel 2.3.11 deprecates `count_distinct` ("Use `AggregateExpressionMethods::aggregate_distinct` instead") and only compiles it under `with-deprecated`. Add a `tag_count` test asserting that duplicate tags count once, unless one already covers it. |

`#[expect]` is the right form for the first three because the lint fires in every build and
feature configuration: the signatures carry no `cfg`. These would be the crate's first
`#[expect]`; Rust has supported it since 1.81, and the gate toolchain is 1.96.1. The aliases
live beside the functions that use them, so they travel with those functions at move 10.

**How to apply it.** A single-site Edit leaves five findings and is denied. Apply all six
changes as one change: either a short scripted patch run through the shell, or one Write of the
whole file. Then verify:

- `bash genesis/orchestrator/rust-style.sh --hook --path elohim/elohim-storage/src/db/content_diesel.rs --stdin < …`
  must print `EXIT=0`, with no failures.
- `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0='*' bash genesis/orchestrator/rust-style.sh --scope elohim/elohim-storage --json`
  must show crate debt falling by 6, with no new identities. The baseline is 149 at
  `28a561cb3`, so the result should be 143. The env override is needed because the checker's
  internal git calls fail on dubious ownership in this container.
- `just gate elohim-storage` must print `EXIT=0`. This covers clippy `-D warnings`,
  `unfulfilled_lint_expectations` and the full lib test.

**Why this comes before any move.** The checker keys findings on crate, path, enclosing item
and attribute. Line movement is harmless, but **moving debt to another path is a new finding,
and moving is never paydown** (RS-GOV-02). If move 0 ran first, the rename would retire six
identities and mint six new ones at `content_diesel/mod.rs`, and the gate would fail. Once
STEP 1 lands, the file carries zero findings. Every later move then relocates clean code and
mints nothing.

**RS-GOV-02 budget for later pushes.** Paydown is evaluated per pushed change against its merge
base, not per commit. A push that carries STEP 1 satisfies it six times over. A pure move adds
and removes nothing, so any later push of moves alone still needs one other elohim-storage
finding removed. Take it from the `db/` neighbours first: `economic_events.rs`,
`mechanism_selection.rs` and `shard_manifests.rs` carry 2 each, and five other files carry 1.
Do this in a separate commit inside the same push, never folded into a move commit, so the
move diff stays pure code motion.

## STEP 2: strict scope (RS-GOV-03) and ratchet, bound at move 0

RS-GOV-03 says to designate the scope strict *before* the major refactor. `src/db/` cannot be
the strict boundary, because it carries 17 findings across 9 files. The boundary is the new
directory, bound in **the same commit as move 0**. Move 0 creates the directory, and no split
move has run yet at that point. Follow the file-form exemplar
`genesis/orchestrator/rust-style/.epr-meta` and add
`elohim/elohim-storage/src/db/content_diesel/.epr-meta`:

```yaml
---
epr-meta-version: 1
id: storage-content-diesel
covers: subtree
purpose: >
  Content-row persistence for elohim-storage, split by concern out of a god-file
  (finding bad2e3aafc10). Strict under RS-GOV-03 from the first move onward.
rules:
  - id: rust-style-strict
    policy: rust-style-strict@1          # no when/params: the checker rejects overrides
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1
    params: { loc-hard: 6000 }           # lower after each move; never raise
---
```

The checker refuses any later weakening, including a new descendant file that arrives with an
unreasoned suppression. Every file the split creates is therefore born strict.

The LoC binding fires from move 0, because `mod.rs` sits at about 9,300 lines. It keeps the
finding open until the file actually shrinks. After each move, lower `loc-hard` to the largest
file in the directory, rounded up to the next 500. The expected path is 8,500 after move 1,
5,500 after move 5, 3,000 after move 8, and 2,500 after move 11. At that point the largest file
is `head_election/stamp/tests.rs` at about 2,300 lines, and no file exceeds the repo soft
ceiling of 3,000. Only ever lower the value.

## Seam map (line ranges at `28a561cb3`)

| Cluster | Production lines | Tests (lines) |
|---|---|---|
| Aliases, `MinTrust`, `apply_min_trust` (trust gate, private; used by reads and stats) | 1–65, 441–466 | — |
| Anchor liveness: `AnchorState`, `ConductorAnswer`, `classify/mark/confirm_anchor_*`, unverified/dead lists, `list_declared_blob_pointer_candidates` | 66–440 | 8555–8827 (6), with `insert_anchored_row`/`anchor_state_of` |
| Query/input types: `CreateContentInput`, `UpdateContentInput`, `ContentQuery`, `BulkResult`, serde defaults | 467–618 | — |
| Content reads: `get_content*`, `list_content`, `list_all_content_rows`, `check_content_exists`, `get_content_by_tag` | 619–803, 1119–1148 | provenance reads in 4794–5520 |
| Content writes (CRUD): `create/bulk_create/update/delete_content` | 804–1118 | 4154 app isolation; 4794–5520 (17); amber/trust 5521–5813 (4) |
| Projection patch: `ContentProjectionPatch`, tag/reach/edge appliers, `widen_to_adopted_earned_reach`, `apply_content_patch_fields` | 1149–1502 | tested through stamp (F19) |
| Anchored upsert: `HeadElection`, `upsert_with_anchor(_transaction)` | 1503–1781 | inside 5945–6543 |
| Anchor inventory: `ATTESTATION_KIND_PREFIX`, `ContentAnchorInventoryRow` (both misplaced at 1782–1813), inventory, pricing, `HeadCorpusDigestReadiness`, digest | 1782–1813, 3301–3574 | 4408–4793 (6) |
| Declared-head reads: `LocalDeclaration`, `declared_head_*`, `blob_hash_for`, `reach_for`, `declared_heads_for` | 1814–1992 | inside 5945–6543 |
| Election ordering: `CanonicalOrdering` (two impl blocks), `ElectionLink`, `DocHeadOrdering` and its two `From` impls, `canonical_ordering_from_columns`, `StaleReason`, `canonical_move_verdict` | 2127–2453 | 3965–4080 (4); 8127–8421 (6) |
| Head stamping: `stamp_declared_head`, `StampMode`, `StampOutcome`, `stamp_declared_head_mode`, `AnchorWitness`, own-conductor/witnessed stamps, `POINTER_GENERATION`, the transaction body (~330 lines), `ElectionColumnHeal`, `heal_election_columns` | 1993–2126, 2454–3100 | release slug 5814–5944; 6211–8126 (most); 8422–8554; F19 8828–9312 (8) |
| Presence and reach: `content_ids_present`, half-blob rows, reach selection, `DISTRIBUTION_SAFE_REACH` | 3101–3300 | 4230–4407 (3) |
| Stats and publish state | 3575–3763, 3911–3958 | 5164–5309 |
| **Local authorship (new):** `AUTHORED_VIA_*`, `mark/clear/is_locally_authored`, `locally_authored_ids`, `count_*_not_locally_authored` | 3764–3910 | none named |

## Proposed split

**Keep the module name.** Renaming to `db/content/` would touch 60 files for no gain. Use
`db/content_diesel/` as a directory module whose `mod.rs` re-exports the public surface with
`pub use`. The rename, if wanted at all, waits for the `elohim-db` crate extraction (row 5 of
the storage-crate-decomposition spec), which moves the path anyway. The ts-rs views and the
generated TS mention `content_diesel` only in doc comments, so no ts-rs import path moves.

```
db/content_diesel/
  .epr-meta           strict + lowered loc ceiling (move 0)
  mod.rs              aliases, MinTrust + apply_min_trust (pub(super)), query/input types, pub use
  test_support.rs     #[cfg(test)] setup_test_db, mk_bundle, mk_plain, insert_anchored_row, shared fixtures
  stats.rs            counts, publish state, unanchored lists
  local_authorship.rs AUTHORED_VIA_*, locally-authored marks and counts
  presence_reach.rs   presence, reach selection, DISTRIBUTION_SAFE_REACH
  anchor_inventory.rs ATTESTATION_KIND_PREFIX, inventory rows, pricing inputs, head corpus digest
  reads.rs            content reads
  writes.rs           CRUD writes
  anchor_liveness.rs  liveness classifier, marks, unverified/dead lists
  projection_patch.rs ContentProjectionPatch + field/tag/reach/edge appliers
  head_election/
    mod.rs            pub use
    ordering.rs       CanonicalOrdering, ElectionLink, DocHeadOrdering, StaleReason, canonical_move_verdict
    declared.rs       LocalDeclaration, declared_head_* reads
    upsert.rs         HeadElection, upsert_with_anchor
    stamp/mod.rs      StampMode/Outcome, stamp_*, pointer generation, heal_election_columns, row aliases
    stamp/tests.rs    non-contract stamp tests (child module, so it keeps private access)
```

**One-way arrows (must hold after every move):**

- `stamp → {ordering, projection_patch, anchor_liveness, mod}`
- `upsert → {projection_patch, ordering, mod}`
- `declared → {ordering, mod}`
- `reads, stats → mod (apply_min_trust)`
- `ordering`, `projection_patch`, `anchor_liveness`, `presence_reach`, `local_authorship` and
  `mod.rs` import nothing from `stamp` or `upsert`.

Rustdoc intra-doc links that point backwards must be requalified as `crate::db::content_diesel::…`
paths. `cargo doc` is not in the gate, so check them by hand.

**Deferred crate-root `head_election`.** `ordering.rs` keeps the
`From<&services::conductor_writes::CanonicalElectionWire> for DocHeadOrdering` impl. That impl
is a db→services arrow, the mirror of the service→db one in `conductor_writes.rs`. The prior
review (`head-authority-carried-with-content-sync-unit` item 10) homes these wire↔db ordering
types in a crate-root `head_election` module beside `epr_head.rs`. They move there once a
second consumer exists, and never into `elohim-epr`. Keep `ordering.rs` free of diesel row
types beyond `canonical_ordering_from_columns`, so that lift stays a single file move.

## Move order (each one commit; compiles and keeps all 87 tests)

- **STEP 1.** RS-GOV-04 paydown, as above.
- **Move 0.** Pure `git mv` from `content_diesel.rs` to `content_diesel/mod.rs`, plus the
  `.epr-meta`. Re-point the seam-registry `file:`/`path:` rows and
  `_HEAL_CANONICAL_CHANNEL_FILES` to `content_diesel/mod.rs`.
- **Move 1.** `test_support.rs`, holding the shared fixtures. This is a prerequisite for
  moving tests with their code.
- **Moves 2–4.** `stats.rs`, then `local_authorship.rs`, then `presence_reach.rs`.
- **Move 5.** `anchor_inventory.rs`. It carries the `HeadCorpusDigestReadiness` contract test.
- **Move 6.** `reads.rs` and `writes.rs`, with the CRUD, provenance, amber and app-isolation
  tests.
- **Move 7.** `anchor_liveness.rs`.
- **Move 8.** `projection_patch.rs`.
- **Move 9.** `head_election/ordering.rs`, with the ordering tests. Three of them are contract
  tests: `canonical_move_verdict_is_antisymmetric_and_total`,
  `stale_reasons_are_distinct_and_labelled` and `ordering_advances_replays_the_stamp_verdict`.
- **Move 10.** `head_election/declared.rs` and `upsert.rs`.
- **Move 11.** `head_election/stamp/{mod.rs,tests.rs}`, holding the rest.

Moves 9–11 rewrite the C2 canonical-channel surface. Each runs as its own commit with no other
edits. A reviewer should see only removed and added lines with identical bodies:
`git diff -M --color-moved=zebra`.

## C2 co-location (validator `heal-fills-never-moves`, `.claude/scripts/_lib/epr_meta.py`)

- **Arm (b): which tests the validator can see.** Arm (b) asks the census for contract tests
  registered *at the edited file's own path*. If a file has none registered, arm (b) skips that
  file entirely. Moving `a_verified_version_carries_type_tags_and_edges` into `stamp/tests.rs`
  would leave `stamp/mod.rs`, which holds `stamp_declared_head_mode` and the `StampMode::*`
  guard arms, with no registered tests, and the guard would lose its teeth with no error.
  **Rule: a seam-registry contract test lives in the same file as the guard symbol it proves.**
  That puts the `canonical_move_verdict` and `StaleReason` tests in `ordering.rs`, and
  `a_verified_version_carries_type_tags_and_edges` in `stamp/mod.rs`. Only non-contract tests
  go to `stamp/tests.rs`.
- **Arm (a): the canonical-channel file list.** `StampMode::Declare` appears at 18 sites today:
  3 in production (2008, 2769, 2909) and 15 in tests, spread across the release-slug, stamp,
  ordering, pointer-generation and F19 clusters. After each move, `_HEAL_CANONICAL_CHANNEL_FILES`
  must list exactly the files that hold `Declare` sites, changed in the same commit. After move
  11 that means `stamp/mod.rs` and `stamp/tests.rs`, plus any other file that still holds a
  `Declare` test. Before committing, check with `rg -l 'StampMode::Declare\b' db/content_diesel/`.
  The tuple's own comment ("the wrapper, the guard arm and their in-file contract tests") still
  describes the list.

## Blast radius per move

- **Rust callers.** None for any move, provided `mod.rs` re-exports. Guard it before and after
  with `rg -o 'content_diesel::[A-Za-z_]+' src tests | sort -u` (the set must still resolve) and
  `rg '^impl ' content_diesel/ | wc -l` (catches silently dropped impls).
- **`seam-registry.yaml`.** The census checks `fn <testName>` inside each cited file, so a test
  moved without re-pointing its row turns red. Re-point these rows:
  - `canonical_move_verdict` (538, 558, 561) and `StaleReason` (568, 585): move 9.
  - `ordering_advances_replays_the_stamp_verdict` (1170): move 9.
  - `HeadCorpusDigestReadiness` (1696, 1774): move 5.
  - `a_verified_version_carries_type_tags_and_edges` (5195): move 11.
  - Prose rows 23, 1897 and 2051 cite module paths, which stay valid. Update their wording
    when you touch them.
  - The `line:` values (1321, 1816) are already stale. Fix each one when you touch its row.
  - `modulePath: db::content_diesel::X` stays valid through the re-exports.
- **`policies.yaml` `c2-monotonic-authority@2`.** Its `why` names `db/content_diesel.rs`. That
  version has landed, so its text is immutable (registry contract, header lines 10–18). Do not
  edit or re-hash it. The operative list is the code tuple above. Record the drifted prose path
  with `epr flow note --kind correction`, and fold the new path into the next deliberate
  `@3`, if one comes.
- **Derived projection.** `.claude/epr-meta/generated/concern-seam-matrix.json` has 19
  references. Regenerate it with `seam-audit.py --matrix --write` after moves 5, 9 and 11.
- **Line-number anchors.** 81 `content_diesel.rs:NNN` anchors sit across 33 genesis files.
  Leave closed and dated entries alone. In an **open** backlog entry that a move invalidates,
  rewrite `file:line` as `module::symbol`. Two in-crate comments cite stale lines:
  `projection_reconcile.rs:5276` and `:10167`. Rewrite those as symbol references in the same
  commit that moves their target. That is a comment-only edit in another file, but still a
  touch.

## Gate per commit

1. `just gate elohim-storage`, with `EXIT=0` echoed on its own line and the output not piped.
   This runs fmt, clippy `-D warnings`, the full lib test and the project's style floor.
2. `rust-style.sh --scope elohim/elohim-storage --json` (with the safe.directory env override)
   must show no new identities, and `strict_files` must count every file under
   `db/content_diesel/`.
3. The seam census (`_lib.seam_census` `render_census`) must not gain errors, and no new
   `missing-contract-test` may name a content_diesel point. Then
   `python3 .claude/scripts/seam-audit.py --spot-check` must exit 0.
4. The `#[test]` count under `db/content_diesel/` must stay at 87, plus any test STEP 1 adds.

## Done when

- No file under `db/content_diesel/` exceeds 3,000 lines.
- The directory is strict with zero findings.
- `loc-hard` has been ratcheted down to 2,500.
- The seam census and `--spot-check` are green, and finding `bad2e3aafc10` is closed with the
  move-11 commit as its evidence.
