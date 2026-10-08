---
id: "backlog-arch-storage-content-diesel-modularization"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Modularize elohim-storage db/content_diesel.rs into a directory module by concern, keeping the db::content_diesel path stable (finding bad2e3aafc10)"
slug: "arch-storage-content-diesel-modularization"
written: "2026-10-08"
author: "rust-architect (architecture finding bad2e3aafc10)"
status: "backlog"
priority: "medium"
area: "elohim-storage/db"
domain: "D-dataplane"
finding: "bad2e3aafc10"
policy: "source-file-loc-ceiling@1"
tags: [architecture, refactor, elohim-storage, diesel, loc-ceiling, head-election, mod-decomposition]
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
  - genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md
shift_objective: |
  Turn elohim/elohim-storage/src/db/content_diesel.rs (8,726 lines at dev tip; the
  finding's measurement was 9,002) into a directory module db/content_diesel/ with
  one file per concern, ONE BOUNDED MOVE PER COMMIT, smallest first, following the
  move order in this entry. Each move is code motion only: items plus their own
  #[cfg(test)] tests travel together, private helpers that cross a module boundary
  become pub(super), and mod.rs re-exports the public surface, so every
  crate::db::content_diesel::X path in src/ and tests/ compiles without edits. Gate
  each move with `just gate elohim-storage` (echo EXIT=$? on its own line, never
  piped) plus the seam census, and update the seam-registry rows and
  _HEAL_CANONICAL_CHANNEL_FILES in the same commit as the move that relocates the
  items they cite. No behaviour change, no renames, no new logic. Do not start the
  head-election moves until the in-flight DocHeadOrdering / columns_changed change
  has landed on dev.
---

# Modularize `db/content_diesel.rs`

## Finding

`elohim/elohim-storage/src/db/content_diesel.rs` is over the `source-file-loc-ceiling@1`
hard ceiling (7,000). The finding measured 9,002 lines; dev tip at writing is 8,726, and a
sprint worktree adds about 141 more: `DocHeadOrdering` beside `CanonicalOrdering`, and a
`columns_changed` return on the stamp path. Both belong to the head-election cluster below.
The production code is 3,626 lines with 57 `pub`/`pub(crate)` fns. The `#[cfg(test)] mod tests`
is 5,100 lines with 81 `#[test]`s. The tests are 58% of the file, so moving them with their
code does most of the work.

**Refactor-safety class.** This is plain native Rust in a non-DNA crate. fmt, clippy
`-D warnings` and `cargo test` are the whole gate. Coordinator zomes hot-swap via
`update_coordinators`, and integrity-zome edits move the DNA hash. Neither applies here, so
no DNA, conductor or reinstall concern rides on any move.

## (1) Seam map (dev tip, line ranges)

| Cluster | Lines | Size | Public items |
|---|---|---|---|
| Aliases, `MinTrust` + `apply_min_trust` (trust gate) | 1–42, 432–452 | ~65 | 1 type (`apply_min_trust` private, used by reads + stats) |
| Anchor liveness (DEAD-anchor class): `AnchorState`, `ConductorAnswer`, `classify/mark/confirm_anchor_*`, `list/count_unverified_anchored`, dead-anchor lists, `list_declared_blob_pointer_candidates` | 43–431 | ~390 | 8 fns, 3 types |
| Query/input types: `CreateContentInput`, `UpdateContentInput`, `ContentQuery`, `BulkResult` + serde defaults | 453–599 | 147 | 4 types |
| Content reads: `get_content*`, `list_content`, `list_all_content_rows`, `check_content_exists` | 600–789 | 190 | 6 fns |
| Content writes (CRUD): `create/bulk_create/update/delete_content`, `get_content_by_tag` | 790–1129 | 340 | 5 fns |
| Projection patch: `ContentProjectionPatch`, `body_from_verified_entry`, `replace_content_tags`, `mirror_committed_reach`, `set_row_reach`, `widen_to_adopted_earned_reach`, `project_authored_edges`, `server_bundle_from_metadata`, `apply_content_patch_fields` | 1130–1490 | 361 | 5 fns, 1 type |
| Anchored upsert: `HeadElection`, `upsert_with_anchor(_transaction)` | 1491–1775 | 285 | 1 fn, 1 type |
| Declared-head reads: `declared_head_for`, `blob_hash_for`, `reach_for`, `declared_head_for_existing_row`, `declared_head_with_election`, `declared_heads_for` | 1795–1979 | 185 | 6 fns |
| Election ordering: `CanonicalOrdering`, `ElectionLink`, `canonical_ordering_from_columns`, `StaleReason`, `canonical_move_verdict` (incoming `DocHeadOrdering`) | 2114–2344 | ~230 | 1 fn, 3 types |
| Head stamping: `stamp_declared_head(_mode)`, `StampMode`, `StampOutcome`, `AnchorWitness`, `stamp_own_conductor_canonical_head`, `stamp_declared_head_witnessed`, `POINTER_GENERATION` + `pointer_generation`/`bump_`, `stamp_declared_head_mode_transaction` (~325 lines alone; incoming `columns_changed`), `ElectionColumnHeal`, `heal_election_columns` | 1980–2113, 2345–2956 | ~745 | 6 fns, 4 types |
| Presence + reach: `content_ids_present`, `list_half_blob_row_ids`, `content_ids_with_reach`, `*content_reaches_for_ids`, `DISTRIBUTION_SAFE_REACH`, `is_distribution_safe_reach` | 2957–3172 | 216 | 6 fns |
| Anchor inventory + head corpus: `ContentAnchorInventoryRow` (at 1776, misplaced), `list_content_anchor_inventory`, `ContentHeadPricingRow`, `content_head_pricing_inputs`, `HeadCorpusDigestReadiness`, `head_corpus_digest(_readiness)` | 1776–1794, 3173–3393 | ~240 | 4 fns, 3 types |
| Stats + publish state: `content_count`, `count_content`, `tag_count`, unpublished/unanchored lists, `mark_published`, `count_publish_state` | 3394–3626 | 233 | 9 fns |
| Tests | 3627–8726 | 5,100 | 81 tests: fixtures 3627–3767; reach/inventory/digest 3768–4331; CRUD/provenance/publish 4332–5058; amber/trust/blob 5059–5351; release-channel slug 5352–5482; upsert/declared/stamp/ordering 5483–8008 (~2,525); anchor liveness 8009–8726 |

## (2) Proposed split

**Keep the module name.** 56 files reference `crate::db::content_diesel::…`: 552 occurrences,
led by `content_service.rs` 62, `http.rs` 54, `sync/projector.rs` 51, `head_adoption.rs` 51,
`projection_reconcile.rs` 47 and `rea_projection.rs` 46. Renaming to `db/content/` would touch
all of them for no gain. Use `db/content_diesel/` as a directory module and have `mod.rs`
re-export the public surface with `pub use`. The rename can be its own last move, or be
dropped. It should probably wait for the `elohim-db` crate extraction (row 5 of the
storage-crate-decomposition spec), which changes the path anyway.

```
db/content_diesel/
  mod.rs              aliases, MinTrust + apply_min_trust (pub(super)), query/input types, pub use
  test_support.rs     #[cfg(test)] setup_test_db, insert_anchored_row, mk_bundle, mk_plain, tie_row …
  reads.rs            content reads
  writes.rs           CRUD writes
  projection_patch.rs ContentProjectionPatch + field/tag/reach/edge appliers
  anchor_liveness.rs  liveness classifier, marks, unverified/dead lists
  presence_reach.rs   presence, reach selection, DISTRIBUTION_SAFE_REACH
  anchor_inventory.rs inventory rows, pricing inputs, head corpus digest
  stats.rs            counts, publish state, unanchored lists
  head_election/
    mod.rs            pub use
    ordering.rs       CanonicalOrdering, ElectionLink, StaleReason, canonical_move_verdict (+DocHeadOrdering)
    declared.rs       declared_head_* reads
    upsert.rs         HeadElection, upsert_with_anchor
    stamp.rs          StampMode/Outcome, stamp_*, pointer generation, heal_election_columns
```

**One-way arrows (must hold after every move):**
`stamp → {ordering, projection_patch, anchor_liveness, mod}`;
`upsert → {projection_patch, ordering, mod}`; `declared → {ordering, mod}`;
`reads, stats → mod (apply_min_trust)`. `ordering`, `projection_patch`, `anchor_liveness`,
`presence_reach` and `mod.rs` import nothing from `stamp` or `upsert`. The only
back-references found today are rustdoc intra-doc links, at lines 200, 249, 1293, 1373 and
2185. Requalify those as `crate::db::content_diesel::…` paths. `cargo doc` is not in the gate,
so a broken link would not fail anything; check them by hand.

**Crate-root `head_election` (deferred, named here so it stays one-way).** `CanonicalOrdering`
and `ElectionLink` are wire↔db ordering types. `services/conductor_writes.rs` builds them
(`CanonicalElectionWire`, lines 577–656), which makes a service→db arrow. The prior review
(`head-authority-carried-with-content-sync-unit` item 10) places their future home in a
crate-root `head_election` module beside `epr_head.rs`, moved only once a second consumer
exists, and never into `elohim-epr`. `head_election/ordering.rs` is shaped to make that later
lift one file move plus a re-export. `ordering.rs` must therefore stay free of diesel row types
beyond `canonical_ordering_from_columns`. If that function blocks the lift, split it out.

## (3) Move order (each one commit; compiles and keeps every test)

0. Rename `content_diesel.rs` to `content_diesel/mod.rs` (a pure git rename). Nothing else
   changes. Re-point the four seam-registry `file:` rows and `_HEAL_CANONICAL_CHANNEL_FILES`.
1. `test_support.rs`: lift the shared test fixtures. This is a prerequisite for moving tests
   with their code.
2. `stats.rs` (233 lines plus publish tests).
3. `presence_reach.rs` (216 lines plus reach tests).
4. `anchor_inventory.rs` (~240 lines plus inventory/digest tests; carries
   `HeadCorpusDigestReadiness`).
5. `reads.rs` + `writes.rs` (530 lines plus CRUD/provenance/amber tests).
6. `anchor_liveness.rs` (~390 lines plus ~720 test lines).
7. `projection_patch.rs` (361 lines).
8. `head_election/ordering.rs` (ordering tests 7581–7875, including both C2 contract tests).
   **Only after DocHeadOrdering has landed on dev.**
9. `head_election/declared.rs` + `upsert.rs`.
10. `head_election/stamp.rs` (the remainder of stamping plus ~2,000 test lines, including the
    release-channel slug tests).

Moves 8–10 rewrite the C2 canonical-channel surface. Run each one as its own commit with no
other edits. A reviewer should see only removed and added lines with identical bodies:
`git diff -M --color-moved=zebra`.

## (4) Blast radius per move

- **Rust callers.** None for any move, provided `mod.rs` re-exports. The guard is
  `rg -o 'content_diesel::[A-Za-z_]+' src tests | sort -u`, compared before and after: the
  same set must still resolve. Also do a before/after `rg '^impl ' content_diesel/ | wc -l`
  count, as a check against silently dropped impls.
- **seam-registry.yaml** (`elohim/elohim-storage/seam-registry.yaml`). The census checks that
  each contract test is contained in the cited file (`fn <testName>`), so moving a test
  without re-pointing its row turns it red.
  - `canonical_move_verdict`: rows 538/558/561. Re-point at move 8.
  - `StaleReason`: rows 568/585. Re-point at move 8.
  - `HeadCorpusDigestReadiness`: rows 1659/1737. Re-point at move 4.
  - `a_verified_version_carries_type_tags_and_edges`: row 5158. Re-point at the move that
    carries that test (stamp or upsert).
  - Prose rows 23, 1860 and 2014: update in the same commits.
  - The `line:` values (1321, 1261, 1816) are already stale. Set them correctly when you touch
    each row.
- **C2 validator.** `.claude/scripts/_lib/epr_meta.py` `_HEAL_CANONICAL_CHANNEL_FILES`
  (line ~1003) lists `db/content_diesel.rs` as the only file where `StampMode::Declare` is
  allowed. Move 10 must change that entry to `head_election/stamp.rs`. If it does not, arm (a)
  flags the relocated call sites as net-new. Arm (b) keys on contract tests cited in the same
  file, which is another reason tests travel with their code. The `c2-monotonic-authority` prose
  in `policies.yaml` (~line 755) names the path too. That row carries a `contentHash`; follow
  the registry's re-hash procedure when you edit it.
- **Derived projection.** `.claude/epr-meta/generated/concern-seam-matrix.json` has 19
  references. Regenerate it with `seam-audit.py --matrix --write` after moves 4, 8 and 10.
- **Line-number anchors** (`content_diesel.rs:NNN`). There are 86 of them across 33 genesis
  files and none in `.epr-meta/`, a2o features or a2o steps. a2o TypeScript has 5 path-only
  comments (`content-sync.ts`, `doorway-client.ts`, `federation-epr.steps.ts`,
  `intimate-reach.steps.ts` ×2), which stay valid. The densest are:
  - `specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md` (16)
  - `2026-08-10-familiar-reach-origin-archaeology.md` (6)
  - `serving-edge-failover-balance-stream-campaign-plan.md` (4)
  - `seed-provenance-anchor-gap.md`, `content-projection-patch-cannot-express-clear.md` and
    `2026-08-10-post-decay-adjudication-cascade-trace.md` (4 each)

  These are dated historical citations and most have already drifted. Leave closed and dated
  ones alone. In any **open** backlog entry that a move invalidates, rewrite `file:line` to
  `module::symbol`. `dataplane-convergence.habit.md` cites the module path, not line numbers,
  and is unaffected.

## (5) Gate per move

1. `just gate elohim-storage`, with `EXIT=0` echoed on its own line and the output not piped.
   This runs fmt, clippy `-D warnings` and the full crate lib test.
2. The seam census:
   `python3 -c 'import sys;sys.path.insert(0,".claude/scripts");from _lib.seam_census import census_data,render_census;from pathlib import Path;print(render_census(census_data(Path("."))))'`.
   The baseline is 14 errors and 2 warnings, none in content_diesel. The error count must not
   rise, and no new `missing-contract-test` may name a content_diesel point.
3. `python3 .claude/scripts/seam-audit.py --spot-check` must exit 0.
4. The test count for the module stays at 81 (`rg -c '#\[test\]' db/content_diesel/`).

## (6) Ratchet

Move 0 creates `elohim/elohim-storage/src/db/content_diesel/`. In move 1, add a governance
package `.epr-meta/manifest.md` there that binds `rs-loc-ceiling` (`source-file-loc-ceiling@1`)
with `params: {loc-hard: 6000}`. Move 1 leaves `mod.rs` at about 8,600 lines, so the binding
fires from that point and keeps the finding open until the file actually shrinks.
After each move, lower `loc-hard` to the largest file in the directory rounded up to the next
500. The expected path is: 5,500 after move 4, then 3,000 after move 6, then 2,500 after move
10, where `head_election/stamp.rs` is the largest file at about 2,750 lines including tests.
At that point no file exceeds the repo soft ceiling of 3,000. Only ever lower the value.
