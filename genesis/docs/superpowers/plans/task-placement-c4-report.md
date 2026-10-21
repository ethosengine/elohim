---
id: placement-c4-report
title: "C4 — eprfs: slice-1 renames, the tier, reach and view interfaces, eprfs-serve, eprfs-ts and the eprfs seam governance (E4)"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#27
actor: agent:implementer@codex
habits: [materialize-what-is-read]
commits: []
cites:
  - "placement-c4-brief | C4 brief | sha256:a58479e2fab1d065 | path: genesis/docs/superpowers/plans/task-placement-c4-brief.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:de0208f9b6e426db | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
---

# C4 — eprfs interfaces, eprfs-serve, eprfs-ts, and E4

Item C4, gap `#27`. Nothing is staged, committed or pushed, as the brief requires.

**How this seat found the lane.** An earlier C4 seat had already written the lane's code into the
working tree between 17:06 and 17:47 UTC and left no report. Nothing was editing `elohim/eprfs/**`
when this seat started at 17:51, and no cargo lease was held. This seat checked that work against the
brief and the interfaces spec, ran the lane's gates and wrote this report. It changed no code. Every
line below was checked against the tree as it stands.

## Gate evidence

- `RUSTUP_TOOLCHAIN=1.96.1 just gate eprfs` (fmt `--check`, clippy `--workspace --all-targets -D
  warnings`, `cargo test --workspace`), with the cargo berth claimed: **`EXIT=0`**. It passed 1,292 tests
  with 0 failed, across 82 suites. The berth was released afterwards (`EXIT=0`).
  - Two earlier runs printed `EXIT=1`. Each time the workspace RAM guard killed the process (signal 15)
    at 80% committed memory, which the running household mesh was causing: first clippy, then `cargo
    test`. Neither run reached a failing test or lint. The third run waited until memory was below 77%
    and passed.
- eprfs-ts `vitest run`: **`EXIT=0`** (3 files, 18 tests). `tsc -p tsconfig.typecheck.json --noEmit`:
  **`EXIT=0`**. `eslint -c eslint.config.mjs src` (rail R-T1): **`EXIT=0`**. These used the root
  `node_modules/.bin` binaries. `just gate eprfs` does not run eprfs-ts; see concern 1.
- `python3 .claude/scripts/habits-project.py --check`: `genesis/manifests/habits.yaml is current`,
  **`EXIT=0`**.
- Seam-registry reference check (a throwaway script in the scratchpad, read-only). It reads every
  `decisionPoints` row in the eprfs-core, eprfs-serve, eprfs-storage and eprfs-local registries and
  checks two things: the line in `sourceLocation` names the symbol, and every `contractTests` test name
  exists in its file. Result: 0 bad.

Per-suite counts in the passing gate: eprfs-core lib 106, `eprfs-core/tests/vectors.rs` 1, eprfs-serve
lib 4, `eprfs-serve/tests/endpoint.rs` 7, eprfs-storage lib 5, `eprfs-storage/tests/boundary.rs` 2,
`tier_chain.rs` 2, `walk_directory_storage.rs` 3.

## Files changed (all under `elohim/eprfs/**`)

Modified: `Cargo.toml` (adds the `eprfs-serve` member), `Cargo.lock`, `build-manifest.json` (adds the
`crates/seam-contracts/**` input and corrects the description: the step runs tests, not nextest),
`eprfs-core/{.epr-meta, Cargo.toml, seam-registry.yaml}`,
`eprfs-core/src/{address, attestation, composition, error, lib, storage, tree}.rs`,
`eprfs-storage/{Cargo.toml, src/lib.rs}`.

New:
- `.epr-meta/materialize-what-is-read.habit.md`
- `eprfs-core/src/{tier, reach, view_decl, level_walk, derive, test_support}.rs` and `eprfs-core/tests/vectors.rs`
- `eprfs-core/src/walk.rs` (slice 1 plus the decision-12 renames)
- `eprfs-serve/{Cargo.toml, seam-registry.yaml, src/lib.rs, src/labels.rs, src/testing.rs, tests/endpoint.rs}`
- `eprfs-storage/src/budget.rs` (`SemaphoreGate`) and `eprfs-storage/tests/{boundary, tier_chain, walk_directory_storage}.rs`
- `eprfs-ts/{package.json, tsconfig.json, tsconfig.typecheck.json, vitest.config.ts, eslint.config.mjs}`,
  `eprfs-ts/src/{index, tree, walk, view, tier, verify, wire}.ts` and
  `eprfs-ts/test/{vectors, chain, purity}.test.ts` with `test/fixture.ts`
- `fixtures/tree-vectors.json`

## What the brief asked for, and where it is

- **Decision 12 slice-1 names.** `Coverage` (`InView | Spine | Outside`) replaces `Placement`.
  `coverage` and `entry_coverage` replace the old field names, and no `Placement`, `entry_placements`
  or `.placement` remains in the workspace. `AbsentReason::NotHeld { missing_at, cid, tried }` and
  `Withheld { missing_at, cid, why }` exist. The public constructors are `AbsentReason::{not_found,
  through_leaf, not_held, withheld}` and `WalkItem::{node, boundary, absent}`. Every seam-B type in the
  spec's table carries `#[non_exhaustive]` where the table marks it. So do `EprfsError` and
  `TreeError` (the 0.2.0 break), `ItemAbsence` and `DerivationKind`.
- **The defaulted `EprfsStorage` methods** (`storage.rs`): `tier_id`, `fetch_at`, `fetch_blobs`
  (fetches once per CID and fans out to each item, via `group_by_cid`), and `query_read_path`,
  `query_list_dir` and `query_diff`. `BatchItem`, `BatchOptions` (`From<FetchPolicy>`), `BlobSink`,
  `SinkControl` and `BatchReport` are also there. `EprfsError::Unanswered` and `RootRequired` are added.
- **`tier.rs`**: `TierId`, whose `own` form refuses to serialize; `TierClass`, `TierSource`,
  `TierAddress`, `TierRecord::merge`, `Proximity`, `TierOrder` and `ProximityOrder`; `ItemAbsence` with
  `from_error` and `integrity`; `ForwardGate`, `ForwardPermit` and `OverBudget`; `TierConnector`;
  `TierChain` with `new`, `own`, `tiers`, `stats` and `forward_blobs`, and its `EprfsStorage` impl
  (pathless reads give `RootRequired` or `Unknown`). This file also holds contract items 30 to 33.
- **`reach.rs`**: `Requester`, `KeyCustody`, `ReachRefusal`, `ReachReason`, `ReachGate`, `AskPolicy`
  and `ReachLevel`.
- **`view_decl.rs`**: `TreeView` in two serde forms. Decoding refuses a path list that is empty,
  unsorted or not an antichain. `new` normalizes, and `cid` follows from that. The shared vector pins
  `["a", "a/b"]`.
- **Rails.** R-E1 is `boundary::frozen_dependency_set`: the exact set, with seam-contracts admitted with
  `serde` only. R-E2 is `boundary::no_io_in_core` with split literals, plus
  `scan_covers_every_module`. Both are in `eprfs-core/src/lib.rs`. R-E3 is
  `eprfs-storage/tests/boundary.rs`; the half that belongs to elohim-storage is not in this write-set.
- **`eprfs-serve`**: `TierEndpoint` with `new`, `serve_batch`, `serve_read`, `serve_list`, `serve_diff`
  and `presence`, plus `ServeCounters`. The `ReasonLabel` faces `ItemAbsenceLabel`, `ReachReasonLabel`
  and `DerivationUnansweredLabel` all pass `check_reason_labels`. The `testing` feature provides
  `assert_tier_contract`, `assert_leaf_contract`, `CountingTier`, and the gate and ask doubles.
  `tests/endpoint.rs` holds the serving contract: a stranger under an intimate root is refused the same
  way whether or not the object exists; `NotUnderRoot` applies even when the root is public; a cold
  forwarder resolves the spine through its upstream; and the worked example (phone, laptop, hub) runs.
- **`eprfs-ts`**: pure (eslint R-T1, plus `purity.test.ts`; no DOM in `lib`). It has `decodeTree`,
  `View`, `pathString`, `walkBatched`, `readPath`, `diffTrees`, `NamedAbsence` and `TierChain` (the
  seam-A items on the shared vectors). The `fixtures/tree-vectors.json` file is shared with Rust's
  `tests/vectors.rs`. It carries: the frozen tag-42 link, an empty tree and a non-UTF-8 name, the walk
  cases, the diff cases, `TreeView` normalization, one `TierRecord`, every `ItemAbsence` and an
  `Unanswered`.
- **E4.**
  - `eprfs-core/.epr-meta`: the existing `immutable-tree-is-not-projection-manifest` rule is extended,
    not duplicated. Its trigger words now include the walk, tier and absence names, and its `why`
    describes the view-walk seam: one verifier, matcher and budget; root and path on every fetch;
    absence semantics; reach injected; a new tier runs `assert_tier_contract`.
  - Seam-registry rows: `walk`, `walk_batched`, `ItemAbsence`, `forward_blobs` and `TreeView` in
    eprfs-core, and `serve_batch` in the new `eprfs-serve/seam-registry.yaml`.
  - The habit moved from `unwired` to `red`. Its `checks:` line names the headline test
    `walk_ten_thousand_entries_three_path_view_fetches_only_spine`, its level-batched twin, the tier
    contract and the serving contract. A one-line DELTA was added, and `habits.yaml` is re-projected and
    current.

## Places the code and the interfaces spec disagree (for the spec to absorb)

1. **§A.2 `OverBudget`** gains `retry_at_epoch_secs` beside `retry_after_ms`. Without it, R7's
   `ComeBack::At { epoch_secs }` would need a clock in eprfs-core, which R-E2 forbids. The gate has the
   clock, so the gate supplies the absolute time. The TypeScript `ChainPolicy.admit` refusal carries
   `retryAtEpochSecs` for the same reason.
2. **§A.1 `BatchOptions`** gains `root_reach: Option<ReachLevel>`, which never goes on the wire. The
   trait's `fetch_blobs` gives the chain's own reads no root reach, so `AskPolicy::may_ask` had no third
   argument on the requester side. In TypeScript the same revision is `ChainOptions { localOnly,
   rootReach, selfId }`, an optional extra argument to the `TierChain` constructor. `localOnly` carries
   H20.
3. **§A.1 `EprfsError::Unanswered.why` and `TreeError::Unanswered.why`** are
   `Box<Unanswered<ItemAbsence>>`, because clippy's `result_large_err` fires under `-D warnings`.
4. **§A.2 `ItemAbsence::from_error`** maps `BlobNotFound` and `BlobNotLocal` to `NotHeld { tried: [] }`,
   where A17 says `NotHeld { [at] }`. B2 says `tried` is empty for a leaf store, and contract 9 says
   `tried` lists the ids the requester dialled. The code follows B2 and leaves the chain to append the
   dialled id. The spec contradicts itself here and should settle on B2. The corrupt-object case is
   matched from the `"corrupt"` text of `EprfsError::Storage`. A typed variant is a follow-on.
5. **§A.5 `TierEndpoint::new`** returns `Err` in every build; the spec says it panics in debug. It also
   refuses an `own` `self_id`, because an endpoint must stamp a real id (A2 and contract 9).
6. **§A.1 and §B additive surface** beyond what the spec writes: `BatchItem::new`, `BatchAnswer` and
   `CollectSink` in `storage.rs`; the `ItemAbsence` constructors (`not_held`, `refused`, `over_budget`,
   `unreachable`, `membership_unknown`), `kind`, `is_observed`, `rewrite_own` and the test-only
   `one_of_each`; `TreeView::paths`; `LevelWalk::root`; `TierCounts`; and `TierEndpoint::{self_id,
   counters}`.
7. **§H.1 generated wire types.** `eprfs-ts/src/wire.ts` holds hand-written shapes, kept deliberately
   outside `src/generated/`. They are pinned by the vectors Rust wrote and checked by guard functions.
   Lane S's schemas now exist (`objects/{unanswered,item-absence,tier-record,tree-view}.schema.json`).
   But `elohim/sdk/schemas/scripts/codegen-ts.mjs` has no eprfs-ts output entry, and that script is
   outside this write-set. Follow-on: add the output directory, then make `wire.ts` re-export the
   generated types.
8. **§H.1 `verifyAddress`.** `@elohim/epr` does not export it yet, so `eprfs-ts/src/verify.ts` carries
   the local check the spec allows, pinned to the same vectors.
9. **§C in eprfs-core: the types and ports only.** Present: `MethodDescriptor`, `ProjectedRoot` (no
   public constructor), `KeyInput`, `DerivationKey`, `DerivationBasis` (with `Unknown` via
   `serde(other)`), `Witness`, `DerivationRecord::{to_edge, to_attestation}`, `AdoptedHead`,
   `DerivationBundle::decode_lenient`, `DerivationLookup`, `DerivationUnanswered`, `DerivationIndex`,
   `TrustPolicy`, `DomainRoster`, `Derive`, `Derived`, `DeriveOutcome` and `TreePath::derived`. Not
   present: `walk_spine`, `SpineWalk`, `project_view`, `derive_or_reuse`, `HouseholdFirst`, the `P`
   vectors in `tree-vectors.json` that §H.1 lists, and `eprfs-core/tests/derive.rs`. All of these are
   slice 4.
10. **§A.6 `HttpTier`** (eprfs-storage feature `http`, reqwest) is not built. No remote tier exists
    outside tests, and this is why the habit stays red. The batch route's CBOR-sequence wire format is
    therefore not exercised in code yet.
11. **§2.0 `ReasonLabel` faces.** eprfs-serve also carries a `DerivationUnansweredLabel`, which §A.2
    does not name. It is needed so seam C's reason passes `check_reason_labels` without hitting the
    orphan rule.

## Concerns

1. **`just gate eprfs` does not run eprfs-ts.** `_gate-eprfs` in the root `justfile` runs only cargo,
   and `elohim/eprfs/eprfs-ts` is not in `pnpm-workspace.yaml`. Its `node_modules` is empty, so it runs
   only through the root binaries, and `vitest.config.ts` aliases `@elohim/epr` to its source. Both
   files are outside this write-set. Until the package is added to the workspace and the recipe gains
   `vitest run`, `tsc` and `eslint` steps, the TypeScript half of the vectors is enforced by no gate.
   The commands above are the evidence for this run only.
2. **Gate flakiness under memory pressure.** While the household mesh runs (one conductor at about
   6.5 GB, plus several rust-analyzers), the workspace sits at 77 to 82% committed memory, and the RAM
   guard kills the gate's clippy or test step at 80%. A reviewer re-running the gate should wait for
   headroom below about 77%.
3. **Not yet reviewed.** The earlier seat's code, and the slice-1 last fix round the brief names (diff
   externals on the spine, result-path metering, `Incomplete` assertions), have had no reviewer. This
   seat checked them for presence and signature agreement and ran them under the gate. It did not
   review them line by line.
4. **The R-E3 half in elohim-storage** (elohim-storage depends on eprfs-core and eprfs-serve only) is
   not in this write-set and is not written.

## Fix round 1

Fifteen review findings, several reported twice (5 and 12 repeat 2; 10 repeats 1; 13 repeats 8;
15 repeats 4). Each was re-checked against the code before it was acted on. Every one held in
substance. Nine are fixed in full, three in part, and one is deferred with a recorded revision.
Where a part is refused, the reason is given. Nothing is staged, committed or pushed.

### Fixed

- **1, 10: integrity and an own-store failure were absences in the depth-first walk.** Confirmed:
  `fetch_checked` turned `EprfsError::Unanswered { Integrity }` into `TreeError::Unanswered`, and
  `Frame::absences` then made it `Withheld`. There is now one rule, `tree.rs::absence_error`,
  backed by `ItemAbsence::is_walk_error`. `Integrity` becomes `TreeError::Integrity`, and
  `Unreachable { own }` becomes `TreeError::Storage`. `fetch_checked` and `LevelWalk::absent`
  both apply it, so `walk`, `read_path`, `list_dir`, `diff_trees`, `walk_batched` and
  `resolve_links` agree. The chain no longer lets a later tier's silence overwrite such an answer
  (`keep_soft`). Before this, a tampering tier followed by an unreachable one ended as
  `Unreachable`, which is Withheld. The TypeScript twin has the same rule (`rejectWalkError` in
  `walk.ts`, `keepSoft` in `tier.ts`). Tests: (34)
  `a_tampering_outer_tier_is_an_integrity_error_in_every_walk` and (35)
  `the_own_store_failing_a_read_is_a_storage_error_in_every_walk` in `tier.rs`. Both run the
  depth-first walk, the level-batched walk, `list_dir` and `resolve_links` over a `TierChain` and
  assert that all four give the same error. The vitest case 34 is in `chain.test.ts`.
- **2, 5, 12 (presence): `presence` was an existence oracle.** Confirmed: it checked
  `admit_root` and then answered `has_blob(cid)` for any CID. `presence(who, root, path, cid)` now
  takes the object's path and proves membership from what this tier holds, through
  `resolve_links` and never forwarded. The answer is `Missing` unless the path leads to `cid`,
  and `Unknown` when the spine is not held or fails its address. Test:
  `presence_never_confirms_an_object_outside_the_admitted_root`. It covers a stranger admitted to
  a public root asking about a CID held only under an intimate root, at a member path, at an
  absent path, and at the root. The registry row is `presence` in `eprfs-serve/seam-registry.yaml`.
- **7: `serve_batch` had no bound.** Confirmed. A batch of more than `max_pending_entries` items
  now gets `Err`. That means "this tier did not answer", which the requester reads as
  `Unreachable`, so its chain moves on. It is counted in `ServeCounters::oversized_batches`. I
  chose `Err` over the suggested `OverBudget` or `Refused` for two reasons. `OverBudget` is final
  in the requester's chain and needs a come-back time, and the core has no clock to give one.
  `Refused` would claim a reach verdict that was never made. A requester's walk already asks in
  chunks of that size (B13). Path bytes, depth, node bytes, total bytes and total entries are
  bounded by the one `Budget` that `resolve_links` holds for the whole batch. Test:
  `a_batch_past_the_pending_entries_bound_is_not_answered`.
- **8, 13: the depth-0 routes forwarded as the tier itself.** Confirmed: `query_target` handed
  the queries the bare chain, so they ran as `self.who` with `restrict: None`. There is now
  `TierChain::on_behalf(who, opts) -> OnBehalf<'_>`, an `EprfsStorage` bound to the inbound
  requester. It gates with that requester's budget, dials only `opts.via` in order, and carries
  the root's reach. `serve_read`, `serve_list` and `serve_diff` take `opts: &BatchOptions` and read
  through it, as the membership check does. Tests: (37)
  `on_behalf_gates_for_the_requester_and_dials_only_its_via` in `tier.rs`, and
  `depth_zero_reads_forward_on_the_requesters_behalf_and_only_to_its_via` in `endpoint.rs`. The
  endpoint test has a gate that admits the laptop but not matthew, so matthew's read comes back
  `OverBudget` and the hub is never asked. Admitted with `via = [hub]`, the read is served. With an
  empty `via`, the hub is never dialled.
- **9: the endpoint had its own traversal.** Confirmed. `membership()` and `spine_node()` are
  deleted. eprfs-core gains `resolve_links(storage, root, paths, opts, limits) ->
  TreeResult<Vec<TreeResult<BlobCid>>>` in `level_walk.rs`. It resolves level by level: one
  `fetch_blobs` per chunk of spine nodes, each node decoded once (A3), one `Budget`, the walk's
  verifier (`cid.verifies` then `decode_tree`), its matcher (`find`, now `pub(crate)`), and the
  B4 rule. The endpoint maps the result and makes no tree decision of its own:
  - `PathNotFound`, `NotADirectory` and `ExternalBoundary` become `NotUnderRoot`.
  - `Integrity` and `Invalid` become an `Integrity` absence.
  - `Storage` (its own store failing) becomes `Unreachable`.
  - Everything else becomes `MembershipUnknown`.

  A corrupt held spine node is no longer hidden as `MembershipUnknown`. Test:
  `a_corrupt_held_spine_node_is_integrity_not_membership_unknown`. The registry row is
  `resolve_links` in `eprfs-core/seam-registry.yaml`.
- **11: `TierId::own()` had two opposite meanings.** Confirmed. `Unreachable { own }` now means
  only "the own store failed". That holds because `own` never crosses the wire and the chain
  rewrites any remote `own` to the id it dialled, as `ItemAbsence::is_walk_error` documents. The
  level walk no longer invents `own`. The driver names the tier that stayed silent with the new
  `LevelWalk::unanswered(by)`, which goes through `absent`'s rule. `walk_batched` names
  `storage.tier_id()`. `advance` refuses (`Invalid`) an item its driver left unanswered rather
  than guess a tier. So silence from a remote tier is `Withheld { Unreachable { tier } }` (B3), and
  silence from the own store, or a chain over it, is an error, which matches the depth-first walk
  (B4). The TypeScript `collectWalk` names `tier.record.id` in the same way. Tests:
  `an_item_never_answered_is_withheld_unreachable_never_not_held` now asserts both the refusal
  and the named tier, and `the_own_store_leaving_an_item_unanswered_is_an_error_not_an_absence` is
  new.
- **14: the root's reach had two channels.** Confirmed. `forward_blobs` drops its `root_reach`
  parameter, and `answer` reads `opts.root_reach` only. `ReachGate` gains a defaulted
  `root_level(root) -> Option<ReachLevel>`, `None` by default. The endpoint fills
  `opts.root_reach` from it and never from the wire (`serving_opts`). `serve_diff` uses the
  narrower of the two roots, or unknown if either is unknown.

### Fixed in part, with what was refused

- **4, 15: release identity and the publish set.** Fixed inside the write-set: the eprfs
  workspace is now `0.2.0`, and the intra-workspace pins in eprfs-meta, eprfs-storage, eprfs-local
  and eprfs-serve move with it. `eprfs-serve` is `publish = false` until the publish set names
  it, as finding 15 allows. Refused, because the files are outside `elohim/eprfs/**`:
  - adding crates to `scripts/ci/publish_eprfs.py` `CRATES` and `test_publish_eprfs.py`;
  - bumping and publishing `elohim-seam-contracts`, whose `unanswered` module is still
    uncommitted at version 0.1.0;
  - regenerating `bridges/cargo/Cargo.lock` and `bridges/jenkins/Cargo.lock`.

  Both bridge lockfiles still record eprfs-core 0.1.2 without seam-contracts. They were stale
  before this round, because of this lane's new dependency. These are **landing blockers** for
  whoever lands C4: on dev, the eprfs publish step would publish an eprfs-core 0.2.0 that cannot
  resolve its registry seam-contracts. Side effect: the `epr` binary's `govern` evaluator
  identity now reports version `0.2.0` (`epr-cli/src/govern.rs`, `CARGO_PKG_VERSION`). It already
  carried the binary's hash, which changes on every rebuild.
- **6: a forwarder's `tried` list could strike tiers.** Fixed: only ids the chain handed that
  tier as its `via` are merged. A tier can no longer name a stranger, and `tried` is bounded by
  the chain's own outer set. Rust and TypeScript both changed. Tests: (36)
  `forwarded_tried_counts_only_the_tiers_the_chain_delegated` and vitest case 36. Refused: "use
  forwarded ids only for reporting, never for skipping". Interfaces spec §A.4 contract 5 requires
  that "the chain never re-asks a tier named in NotHeld.tried". In strict tiering the asked tier
  was delegated exactly those tiers. A tier that would lie about forwarding can as easily not
  forward, so re-asking adds no safety and doubles the read-pattern fan-out that the contract
  exists to prevent. The harness's contract-5 case still passes.
- **12 (admit_object): `admit_object` is unused.** Refused removal. §A.3 declares it for
  objects outside every root, and §A.5's serving contract names its route ("a leaf also
  referenced by a public content row stays servable through that row's own route"). That route
  belongs to elohim-storage, which is not in this write-set. It is still uncalled in this tree;
  the follow-on that serves public content rows calls it, or the spec drops it.

### Deferred

- **3: two-form serde on `ReadLeaf`, `DirListing`, `TreeChange` and `LeafKind` (§B.2).**
  Confirmed absent, and the earlier report missed it. Deferred to the HttpTier slice and recorded
  here as a revision. The spec gives the JSON form only for path components. `TreeEntry`'s serde
  is the DAG-CBOR node form, whose byte names have no slash-joined JSON form yet. `Bytes` would
  need the `bytes` crate's `serde` feature or a `serde_bytes` wrapper. Nothing reads these on the
  wire until `POST /eprfs/v1/{read,list,diff}` exists. That slice should settle the JSON form of a
  `TreeEntry` and add round-trip and non-UTF-8 refusal tests with the types.

### New places the code and the interfaces spec disagree (for §6)

12. **§A.2 `forward_blobs`** loses its `root_reach` parameter. The root's reach rides only in
    `BatchOptions::root_reach` (finding 14).
13. **§A.2 `TierChain::on_behalf` and `OnBehalf<'_>`** are added: the chain bound to one inbound
    requester's budget, `via` and root reach (A4 on the depth-0 routes and the membership check).
14. **§A.3 `ReachGate::root_level`** is added, defaulted to `None`. It is the serving side's own
    source for the root's reach.
15. **§A.5 signatures.** `serve_read`, `serve_list` and `serve_diff` take `opts: &BatchOptions`.
    `presence` takes `path: &TreePath`. `serve_batch` answers a batch over `max_pending_entries`
    with `Err`.
16. **§B.1 `LevelWalk::unanswered(by)`** is added, and `advance` refuses an item left
    unanswered. B3's `Withheld { Unreachable }` is reached by the driver naming the silent tier.
    `walk_batched` names `storage.tier_id()`, so a leaf or own store's silence is B4's error.
17. **§A.1 against §B.1.** §A.1 says `fetch_checked` turns every `why` other than `NotHeld` into
    `Withheld`. §B.1/B4 says `Integrity` and an own-store failure are errors. The code follows B4,
    and §A.1's sentence should be amended.
18. **§A.5 step 2 is a core function.** `eprfs_core::resolve_links` is the batch membership
    check, so eprfs-serve holds no traversal.
19. **§A.4 contracts 6 and 9.** A soft answer is replaced by a later tier's, except
    `Integrity` and an own-store failure, which persist (`keep_soft`). A forwarder's `tried` is
    merged only for ids in the `via` the chain gave it.
20. **§B.2 serde of the depth-0 answers** is deferred to the HttpTier slice (finding 3).
21. **Release.** The eprfs workspace is 0.2.0. `eprfs-serve` is `publish = false` until the
    publish set names it.

### Files changed in this round

All under `elohim/eprfs/**`:
- `Cargo.toml`, `Cargo.lock`, and `eprfs-{meta,storage,local,serve}/Cargo.toml`
- `eprfs-core/src/{tree,tier,level_walk,walk,reach,lib,test_support}.rs` and
  `eprfs-core/seam-registry.yaml`
- `eprfs-serve/src/lib.rs`, `eprfs-serve/tests/endpoint.rs` and `eprfs-serve/seam-registry.yaml`
- `eprfs-ts/src/{walk,tier}.ts` and `eprfs-ts/test/chain.test.ts`

Tests added: core (34)-(37) and
`the_own_store_leaving_an_item_unanswered_is_an_error_not_an_absence`; four endpoint tests;
vitest cases 34 and 36.

### Gate evidence (fix round 1)

- `RUSTUP_TOOLCHAIN=1.96.1 just gate eprfs` (fmt `--check`, clippy `--workspace --all-targets -D
  warnings`, `cargo test --workspace`), run with the cargo berth claimed (`berth claim cargo
  --class verify --ttl 3600`) and released afterwards (`EXIT=0`): **`EXIT=0`**. 1,301 tests
  passed and 0 failed, across 82 suites. The run took 12 minutes, which is over the 600 s soft
  ceiling and under the hard one.
  - An earlier run in this round printed `EXIT=1` at `cargo fmt --check`: one `assert_eq!` in a
    new test needed wrapping. It was fixed with `cargo fmt` (`EXIT=0`), and the run above followed.
- eprfs-ts, run with the root `node_modules/.bin` binaries:
  - `vitest run`: **`EXIT=0`**, 3 files and 20 tests.
  - `tsc -p tsconfig.typecheck.json --noEmit`: **`EXIT=0`**.
  - `eslint -c eslint.config.mjs src`: **`EXIT=0`**.
- `python3 .claude/scripts/habits-project.py --check`: current, **`EXIT=0`**. A one-line fix-round
  DELTA was added to `materialize-what-is-read.habit.md`; the status stays red.
- The seam-registry reference check (the same throwaway scratchpad script as before, run across
  the four eprfs registries) found 0 bad rows. It checks that every `sourceLocation` line names
  its symbol and that every `contractTests` name exists.

Status stays **DONE_WITH_CONCERNS**. The landing blockers under findings 4 and 15 are outside
this write-set: the publish set, the seam-contracts release, and the two bridge lockfiles.

## Fix round 1, second finding set (written up in Codex review 1)

The previous seat handled a second set of review findings between 18:47 and 19:08 UTC, after the
"Fix round 1" section above was written. It changed code but did not update this report, so the
habit DELTA that cites that round pointed at nothing. This seat read the tree before Codex
review 1 began; that round had added:

- **Streaming outer asks.** `TierChain::answer` streams each outer tier's answers through
  `OuterAsk`, which is a `BlobSink`. Each answer is matched through `storage::Awaiting`'s hash
  index, verified when it arrives and passed to the caller's sink straight away. An answer nobody
  asked for, or a repeat, is dropped before it is kept. A tier that sends more than its batch is
  told `Stop`, and so is a tier once the caller's sink says `Stop`. The level walk (`LevelSink`)
  and the membership resolver (`SpineSink`) read through the same index. Tests: tier.rs (38)
  `a_flooding_outer_tier_is_cut_off_and_nothing_unrequested_is_kept`, tier.rs (39)
  `the_callers_stop_reaches_the_outer_tier_at_once`, and level_walk.rs
  `the_level_walk_and_the_resolver_keep_only_what_they_asked_for`.
- **A harness for outer tiers.** eprfs-serve's `testing` feature gains `assert_outer_tier_contract`
  with its `StopFirst` sink. The endpoint loopback passes it, and a tier that serves unverified
  bytes fails it. Tests in `tests/endpoint.rs`:
  `an_endpoint_served_tier_passes_the_outer_tier_contract` and
  `a_tier_serving_unverified_bytes_fails_the_outer_tier_contract`. `TierEndpoint` also verifies
  bytes from its own store before it serves them (`Restamp { verify }`).
- **One manifest reader for R-E1 and R-E3.** `eprfs-core/src/boundary/manifest_deps.rs` reads a
  dependency in every form it can take: `[dependencies]`, `[target.<cfg>.dependencies]`, dotted
  tables, and renames. Tests: `every_dependency_form_is_read`, lib.rs
  `the_frozen_set_counts_target_and_dotted_tables`, and eprfs-storage/tests/boundary.rs
  `the_parser_sees_every_form_of_a_denied_dependency`.
- **That round's §6 revisions, recorded here for the first time.** `storage::Awaiting` is public,
  and the chain answers by streaming. `assert_outer_tier_contract` is a third harness beside the
  tier contract and the leaf contract, and the `.epr-meta` rule names all three. The habit's
  `checks:` line names the outer-tier contract.

That round's gate result was never recorded. The runs below cover its code, because they ran on a
tree that contains it.

## Codex review 1 fixes

There were seventeen findings, and each was re-checked against the code before anything changed.
Twelve are fixed in full and three in part. Two are refused, with reasons: one needs files outside
`elohim/eprfs/**`, and the other needs a ruling only the operator can give. Nothing is staged,
committed or pushed.

### Gate evidence (Codex review 1)

- `RUSTUP_TOOLCHAIN=1.96.1 just gate eprfs` ran with the cargo berth claimed (`berth claim cargo
  --class verify --ttl 3600`). The berth was released afterwards (`EXIT=0`). The gate runs fmt
  `--check`, clippy `--workspace --all-targets -D warnings`, and `cargo test --workspace`. Result:
  **`EXIT=0`**, with 1,317 tests passed, 0 failed, across 82 suites. The run took 12 minutes, which
  is over the 600 s soft ceiling and under the hard one. The log's only warnings are the 18
  existing ts-rs "failed to parse serde attribute" notes.
  - Before the gate, a focused run (`cargo test --no-fail-fast -p eprfs-core -p eprfs-serve -p
    eprfs-storage --features eprfs-serve/testing`) gave `EXIT=101` once. Two tests failed: test 34
    and its new companion. Both assumed that a tier's own corrupt copy would reach the requester as
    bytes. The default `fetch_blobs` turns that copy into an Integrity *statement*, and under
    finding 5 a statement is no longer a walk error. Both tests now dial the new `Unverified`
    double, which sends the bad bytes. The re-run gave `EXIT=0`.
- eprfs-ts, run with the root `node_modules/.bin` binaries:
  - `vitest run`: **`EXIT=0`**, 3 files and 24 tests.
  - `tsc -p tsconfig.typecheck.json --noEmit`: **`EXIT=0`**.
  - `eslint -c eslint.config.mjs src`: **`EXIT=0`**.
- `python3 .claude/scripts/habits-project.py` was re-projected, then run with `--check`: current,
  **`EXIT=0`**.
- Seam registries:
  - The reference check (a read-only scratchpad script) found no bad line or test reference in the
    eprfs-core, eprfs-serve, eprfs-storage or eprfs-local registries. The 7 rows it lists carry no
    `line:` field, as before.
  - `.claude/scripts/_lib/seam_census.py`'s `load_registry` validated the three registries this
    round edited against the schema, with 0 errors.

### Fixed

- **1. The report and the gate evidence were stale.** Confirmed. The second finding set is written
  up above, the gates were re-run on the current tree, and the new habit DELTA points at this
  section.
- **4. A later refusal replaced an Integrity answer.** Confirmed. Step 3 took `o.last.or(o.soft)`,
  and a gate refusal set `last` on every open item. Step 3 now ranks a walk-error soft answer above
  `last`. That also covers the gate path, because the gate sets `last` and step 3 reads it. The
  TypeScript `TierChain` follows the same rule (`isWalkError`). Tests: tier.rs
  `integrity_outranks_a_later_refusal_and_the_gate_refusing_the_next_dial` (a tampered tier
  followed by `Refused`, and a tampered tier followed by the gate refusing the next dial, each
  through all four walks), and vitest "34, continued".
- **5. A remote tier's report of Integrity aborted the requester's walk.** Confirmed. Any tier
  could fail any walk by answering `{kind: integrity}`, including an honest hub with one corrupt
  node. `OuterAsk::absent` now turns a remote `Integrity` into `Unreachable { tier }`
  (`reported_integrity`). Its meanwhile text says what the tier reported. That is absence not
  established, so the chain moves on.

  Integrity stays a walk error only where this side verified the bytes: bytes that arrived and
  failed (`OuterAsk::arrived`), an own-store copy that failed, or a store's typed `Corrupt`.

  Tests: (40) `a_remote_integrity_statement_is_withheld_not_a_walk_error` and vitest 40. In both
  the depth-first walk and the level walk the result is `Withheld { Unreachable }`, and a later
  holder still answers. Test 34 now dials `test_support::Unverified`, which puts bad bytes on the
  wire.

  Limit: a walk run directly over a remote tier, without a chain, still reads that tier's Integrity
  statement as an error. The chain is the requester's only entry to outer tiers (§A.4).
- **6. Level-walk frames kept an ancestor list that nothing metered.** Confirmed.
  - `Frame` no longer carries `ancestors`, and the cycle check went with it. A link cycle cannot
    occur, because a node cannot name its own address, and `max_depth` bounds any chain of links.
  - Each frame the level walk queues is charged its retained size under `"pending path bytes"`:
    its path bytes, plus 24 bytes per component, plus 192 bytes. These are fixed numbers, not
    `size_of`, so the TypeScript twin charges the same. `queued_frame_charge_covers_the_frame`
    checks that the real sizes fit under them.
  - The module doc no longer claims the two walks give "the same errors". Where the depth-first
    walk refuses on `"pending entries"`, the level walk refuses on `"pending path bytes"`.
  - The TypeScript twin has the same changes.
  - `diff_trees`' `DiffFrame` keeps its pair lineage. It is a depth-first stack whose size
    `max_pending_entries` already bounds.
- **7. `walk_batched` checked result paths only after it had kept them.** Confirmed. `LevelWalk`
  now holds a second `Budget`, `retained`, and charges each node and each absence when it keeps it.
  The check after the walk is gone, and TypeScript `collectWalk` does the same. Test:
  `the_level_walk_meters_the_paths_it_keeps_as_it_keeps_them`. With half the total kept-path size
  as the budget, it refuses on `"result path bytes"` before it has fetched the whole chain.
- **8. Absences that were kept were not metered.** Confirmed. `WalkItem::retained_bytes` charges,
  for each absence:
  - its view path;
  - its `missing_at` or `leaf`;
  - every string its reason and unanswered parts carry: tier ids, the rule, needs-standing, the
    event, the meanwhile text and the escalation.

  `walk_collect` and the level walk both charge it. Test:
  `absences_of_a_shared_subtrees_missing_children_are_metered`. In it, 50 long paths lead to one
  shared subtree with 10 missing children, which gives 500 absences, each with a 401-byte
  `missing_at`. The budget is set so that every other line fits. `walk_collect` then refuses on
  `"result path bytes"`, and the level walk refuses on whichever line it reaches first.
- **12. The serving adapter made the membership decision itself.** Confirmed. The decision now
  lives in eprfs-core:
  - `Membership` (non-exhaustive) and `Membership::of(cid, &link)` match `TreeError` exhaustively
    inside the crate, so a new variant must be classified before the crate compiles.
  - `check_membership(storage, root, items, opts, limits)` checks a whole batch.
  - `ReachLevel::narrower` combines the reach of two roots.

  `TierEndpoint` only names the verdicts. `NotALeaf`, which `resolve_links` never returns, now maps
  to `Unknown` instead of `NotUnderRoot`, which matches the resolver's registry row. eprfs-core has
  a new registry row, `Membership`.
- **13. The ask rule was written in two places.** Confirmed. `OpennessAsk { household, public }`
  now lives in `eprfs-core/src/reach.rs`. eprfs-serve's `testing` re-exports it, and the future
  elohim-storage host will use it. `fixtures/tree-vectors.json` gains an `ask` section of 10 cases.
  `tests/vectors.rs` computes each verdict with `OpennessAsk`, and vitest checks that
  `standingPolicy` gives the same answers. eprfs-core has a new registry row, `OpennessAsk`.
- **14. Node acceptance and the path step were written three times.** Confirmed.
  - `tree::accept_node(budget, cid, bytes) -> TreeResult<TreeResult<(TreeNode, usize)>>` verifies,
    checks size, charges and decodes. The outer `Err` means the shared budget is spent; the inner
    `Err` means the node failed.
  - `fetch_node`, `LevelWalk::offer` and `SpineSink::node` all call it. `fetch_node` reads through
    `fetch_unverified`, so each node is hashed once.
  - `resolve_entry` and `resolve_links` share the single-path steps `walk::descend` and
    `walk::link_of`.
  - `TreeError` derives `Clone`. `clone_error` is deleted; it silently turned unknown variants into
    `Invalid`.

  Test: `resolve_links_agrees_with_the_single_path_resolver_on_every_path`. It covers 15 paths:
  the root, members, misses, a path through a leaf, an external entry and a missing spine node.
  Each result must equal what `resolve_entry` plus `link_of` returns, as a value, or as an error of
  the same kind and text.
- **15. Corruption was detected by reading an error message.** Confirmed. `EprfsError::Corrupt(BlobCid)`
  is added, and `DirectoryStorage::read_object` and `MemoryStorage::put_blob_verified` return it.
  `ItemAbsence::from_error` maps only that variant to Integrity. A `Storage` message that says
  "corrupt" now maps to `Unreachable`, and the test asserts both. `fetch_checked` maps `Corrupt` to
  `TreeError::Integrity`.

  **Behaviour change:** a depth-first walk over a corrupt `DirectoryStorage` object now refuses
  with `Integrity`. It used to refuse with `Storage`. This matches the level walk, as B.1 level
  equivalence requires, and `directory_storage_corrupt_spine_object_refuses` now asserts
  `Integrity`. No caller outside the workspace sees the change, because brit pins eprfs-core 0.1.2
  from the registry.

  Refused in part: separate variants for configuration misuse and encoding failures. With the
  string match gone, nothing reads the text of `Storage`. New variants in a non-exhaustive error
  would change the API with nothing to read them.
- **3, in part: blocking I/O.** Confirmed. eprfs-storage gains `OffloadedDirectory`. It is
  `DirectoryStorage` with every read and write run on `tokio::task::spawn_blocking`. Its
  synchronous bodies, `has_object` and `put_object`, were extracted from the async impl, which now
  calls them. tokio's `rt` feature is named explicitly; the workspace already enabled it. Test:
  `offloaded_directory_passes_the_leaf_contract_on_a_multi_thread_runtime`, which also asserts
  `Corrupt` through both stores. The decoded-node LRU is deferred; see below.

### Fixed in part, or refused

- **2. The order in contract 7.** The finding holds. `BlobSink::arrived` is synchronous and
  `put_blob_verified` is async. While answers stream, which §4 requires, §A.4's "written before
  `sink.arrived`" cannot be met. I refused buffering per CID, because it would undo streaming and
  the caller's `Stop` (tests 38 and 39). Revision 22 below records the order kept instead.
  `assert_tier_contract` already pins it: once the batch returns, the object is in the own store
  and a second read hits it. The harness comment now says so.
- **3, the LRU: deferred.** `TierEndpoint` reads spine nodes from its own store, which is already
  the cache of verified objects. §4 itself counts `decode_tree` at spine scale as negligible. A
  decoded-node cache would also have to reach into `resolve_links`. It belongs with an E5
  measurement, the way §4 treats the own hop's double verify.
- **9 and 10. The seam-contracts release and the eprfs publish set: refused, outside the
  write-set.** All of this work lies outside `elohim/eprfs/**`:
  - bumping and publishing `elohim-seam-contracts`;
  - adding it to `scripts/ci/publish_eprfs.py` ahead of eprfs;
  - adding a missing-dependency check to that script;
  - regenerating `bridges/{cargo,jenkins}/Cargo.lock`.

  The in-tree alternative, "hold eprfs at `publish = false`", is refused too. `publish_eprfs.py`
  publishes a fixed `CRATES` list, and `cargo publish` refuses a `publish = false` crate, so the
  flag would turn the pipeline red rather than hold the release. The landing blockers from fix
  round 1 stand; finding 10 confirms them against the registry index.
- **11. An AGPL leaf under a permissive crate: an operator item.** Refused as a code change: only
  the operator can make the ruling. The open question is recorded next to R-E1's admission of
  seam-contracts, in the doc comment of `frozen_dependency_set` in `eprfs-core/src/lib.rs`. No
  covenant floor covers it, and this seat only flags it.
- **16. An absence stored the same fact twice: fixed in part.** The data shape comes from the
  spec. §B gives `WalkItem::Absent { view_path, reason, unanswered }`, and the §H.1 mapping table
  also names the reason and the unanswered parts separately. Moving `Unanswered` into the reasons
  would break both the declared shape and its wire mapping, so I refused that part.

  What made the wrong call possible to write is fixed:
  - `WalkItem::absent` returns `TreeResult<Self>` and refuses (`Invalid`) any combination where
    the reason and the unanswered parts disagree.
  - The new constructors cannot disagree: `WalkItem::unanswered(view_path, missing_at, cid, u)`
    reads the reason from `u`, and `not_found` and `through_leaf` carry no unanswered parts.
  - The walk builds every absence through these constructors.

  Test: `an_absence_cannot_name_one_reason_and_carry_another`. This is revision 26 below.
- **17. The TypeScript rail and vectors run in no gate: fixed inside the write-set.**
  - The habit's `checks:` line now declares rail R-T1 and the TypeScript half of the vectors
    **unwired**.
  - `package.json` gains `@typescript-eslint/parser` and `eslint` as devDependencies, and a `lint`
    script.
  - `test/purity.test.ts` now reads imports with the TypeScript compiler's own
    `ts.preProcessFile`. That covers both quote styles, bare imports, `export … from` and dynamic
    `import()`.
  - Browser globals are matched as identifiers, including through `globalThis.`, `self.` and
    `self['…']`. A self-test pins each of these forms.

  Refused, outside the write-set: the `pnpm-workspace.yaml` entry and a `_gate-eprfs-ts` recipe in
  the root `justfile`.

### New places the code and the interfaces spec disagree (for §6, continuing from 21)

22. **§A.4 contract 7.** Forwarded bytes are written back before the batch returns, not before
    `sink.arrived`. The chain streams (§4), and `BlobSink::arrived` is synchronous.
23. **§A.4 contract 6 and §B.1/B4.** Integrity is a walk error only where this side verified the
    bytes. A remote tier's statement of Integrity reaches the requester as `Unreachable { tier }`.
    A walk error ranks above a later `Refused` or `OverBudget`.
24. **§A.1.** `EprfsError::Corrupt(BlobCid)` is added. A17's corrupt-object case reads this
    variant, never a message.
25. **§B.1/B13 and §4's bounded memory.** `Frame` carries no ancestor list. A level walk charges
    each queued frame its retained size (path bytes, plus 24 per component, plus 192) under
    `"pending path bytes"`. Its bound is therefore `max_total_bytes` of queued frames, not one
    chunk. The depth-first walk still bounds its stack by `max_pending_entries`. Both walks charge
    what they keep (node paths, and absences with their strings) at the moment they keep it.
26. **§B.** `WalkItem::absent` returns `TreeResult<Self>`, and `WalkItem::{unanswered, not_found,
    through_leaf}` are added.
27. **§A.5 step 2 and §1.1.** `eprfs_core::{Membership, check_membership}` and
    `ReachLevel::narrower` are added. The serving adapter does no classification of its own.
28. **§A.6.** `OpennessAsk` is an eprfs-core type, and the shared vectors gain an `ask` section.
29. **§4, Blocking I/O and the cache table.** `eprfs_storage::OffloadedDirectory` is added. The
    decoded-node LRU is deferred until an E5 measurement.
30. **§B.** `TreeError` derives `Clone`, so many paths can share one spine node's error.

### Files changed in this round

Everything is under `elohim/eprfs/**` except `genesis/manifests/habits.yaml`, which is generated
and which the brief requires re-projecting.
- `eprfs-core/src/{error,tier,tree,walk,level_walk,reach,lib,test_support}.rs`,
  `eprfs-core/tests/vectors.rs`, `eprfs-core/.epr-meta`, `eprfs-core/seam-registry.yaml`
- `eprfs-serve/src/{lib,testing}.rs`, `eprfs-serve/seam-registry.yaml`
- `eprfs-storage/{Cargo.toml, src/lib.rs, tests/tier_chain.rs, tests/walk_directory_storage.rs,
  seam-registry.yaml}`
- `eprfs-ts/{package.json, src/tier.ts, src/walk.ts, test/chain.test.ts, test/vectors.test.ts,
  test/purity.test.ts}`
- `fixtures/tree-vectors.json` (the `ask` section)
- `.epr-meta/materialize-what-is-read.habit.md` (the `checks:` line and a DELTA)

Tests added:
- core:
  - `integrity_outranks_a_later_refusal_and_the_gate_refusing_the_next_dial`
  - (40) `a_remote_integrity_statement_is_withheld_not_a_walk_error`
  - `the_level_walk_meters_the_paths_it_keeps_as_it_keeps_them`
  - `absences_of_a_shared_subtrees_missing_children_are_metered`
  - `queued_frame_charge_covers_the_frame`
  - `an_absence_cannot_name_one_reason_and_carry_another`
  - `resolve_links_agrees_with_the_single_path_resolver_on_every_path`
  - the `ask` vectors
- storage: `offloaded_directory_passes_the_leaf_contract_on_a_multi_thread_runtime`
- vitest: "34, continued", 40, the `ask` vectors, and the purity self-test

Assertions changed:
- `from_error_maps_only_not_found_to_not_held`: it uses the typed variant and no longer reads the
  message.
- `directory_storage_corrupt_spine_object_refuses`: `Storage` became `Integrity`.

Status stays **DONE_WITH_CONCERNS**. The landing blockers from findings 9 and 10 (the
seam-contracts release, the publish set and the two bridge lockfiles) remain outside this
write-set. Finding 11's licence question waits on the operator. The TypeScript gate, finding 17's
recipe and workspace entry, is outside this write-set.

## Codex review 2 fixes

There were ten findings. Each was re-checked against the code before anything changed. Seven are
fixed in full and one in part. Two are refused, with reasons: one needs files outside
`elohim/eprfs/**` and an operator ruling, and the other needs a spec ruling. Nothing is staged,
committed or pushed.

### Gate evidence (Codex review 2)

- `RUSTUP_TOOLCHAIN=1.96.1 just gate eprfs` ran with the cargo berth claimed (`berth claim cargo
  --class verify --ttl 3600`). The berth was released afterwards (`EXIT=0`). The gate runs fmt
  `--check`, clippy `--workspace --all-targets -D warnings`, and `cargo test --workspace`.
  - The first run gave **`EXIT=1`** at `cargo fmt --check`: this round's new code had not been
    formatted. `cargo fmt --all` was applied (`EXIT=0`), and `--check` then passed (`EXIT=0`).
  - The re-run gave **`EXIT=0`**, with 1,325 tests passed, 0 failed, across 82 suites. It took
    12 minutes, which is over the 600 s soft ceiling and under the hard one. The log's only
    warnings are the 18 existing ts-rs "failed to parse serde attribute" notes.
  - Before the gate, a focused run (`cargo test --no-fail-fast -p eprfs-core -p eprfs-serve -p
    eprfs-storage --features eprfs-serve/testing`) gave `EXIT=0`, and every new test passed in it.
    The `order` vectors were written by `EPRFS_WRITE_VECTORS=1 cargo test -p eprfs-core --test
    vectors` (`EXIT=0`). Every other section of the fixture was unchanged, which a script compared
    section by section.
- eprfs-ts, run with the root `node_modules/.bin` binaries:
  - `vitest run`: **`EXIT=0`**, 3 files and 27 tests.
  - `tsc -p tsconfig.typecheck.json --noEmit`: **`EXIT=0`**.
  - `eslint -c eslint.config.mjs src`: **`EXIT=0`**.
- `python3 .claude/scripts/habits-project.py` was re-projected (`EXIT=0`), then run with
  `--check`: current, **`EXIT=0`**.
- Seam registries:
  - The reference check (a read-only scratchpad script) re-checked the eprfs-core, eprfs-serve,
    eprfs-storage and eprfs-local registries. Every `line:` points at its symbol, and every named
    contract test exists. Seven line numbers that had drifted were corrected.
  - `seam_census.load_registry` validated the three edited registries against the schema, with 0
    file errors and 0 row errors.

### Fixed

- **1. Write-back held a whole outer batch in memory.** Confirmed. `write_back` kept every verified
  object one tier delivered until that tier's `fetch_blobs` returned. The chain now has
  `ChainLimits { max_request_items, max_write_back_bytes }` (defaults 4,096 CIDs and 64 MiB),
  set with `TierChain::with_limits`. Once an ask holds `max_write_back_bytes` of verified bytes and
  other items are still awaited, `OuterAsk` pauses: it tells the tier `Stop` and takes nothing
  more. The chain then writes back what it holds and asks the same tier again for the items the
  round did not reach. Every pause follows a delivery, so each round makes progress. The chain's
  peak is the bound plus one object, whatever the batch.

  The finding's other route, a byte hint for the gate, cannot work: `BatchItem` carries no size,
  so nothing can be known before the bytes arrive. Finding 10 removes the knob nothing fed.

  Test: `the_chain_writes_back_between_rounds_within_its_byte_bound` (eprfs-serve
  `tests/endpoint.rs`). With a one-byte bound, the hub sees batches of 5, 4, 3, 2 and 1. A caller
  sink asserts at every arrival that every earlier arrival is already in the own store.
- **2. The chain sent an endpoint more items than it accepts.** Confirmed. `TierChain::answer`
  sent all of its wanting items in one `fetch_blobs`, and `TierEndpoint::serve_batch` refuses any
  batch over `max_pending_entries` with `Err`. Each outer tier is now asked in rounds of at most
  `ChainLimits::max_request_items`. That default is `TreeLimits::default().max_pending_entries`,
  so the requester and the serving tier share one default bound. The endpoint doc names it.

  The TypeScript chain asks in rounds too (`ChainOptions.maxRequestItems`, default
  `DEFAULT_MAX_REQUEST_ITEMS = 4096`). A round's answer map is therefore bounded by the round.

  Test: `a_chain_asks_an_endpoint_in_rounds_its_bound_admits`. A requester's chain at the
  defaults reads 4,097 leaves through a loopback `TierEndpoint` at the defaults. Every leaf
  arrives, the endpoint answers two batches, and none is refused as oversized. The test also
  asserts that the two defaults are equal. Vitest: "asks an outer tier in rounds of at most
  maxRequestItems, each item once".
- **3. A repeated `via` asked one upstream many times.** Confirmed. `TierChain::candidates` now
  normalizes `restrict` before the loop: it keeps `via`'s order, drops repeats and ids outside the
  outer set, and stops once it has every outer tier. The list is therefore never longer than the
  outer set. Each tier also keeps a queue of the items it is still to be asked in this call, so
  it is asked each item once, whatever it answers. This replaces the old per-tier filter, which
  re-asked a tier that answered soft. The per-pass clone of `admitted[position+1..]` now copies at
  most the outer set.

  Test: `a_repeated_via_asks_each_upstream_once`. An endpoint receives `via = [hub; 1000]`, and the
  hub answers `MembershipUnknown`. The hub sees exactly one batch, and the item stays
  `MembershipUnknown`.
- **4. A derived path had no wire form.** Confirmed: the binary `Deserialize` ran
  `from_components`, which refuses NUL. The binary decode now first tries
  `TreePath::parse_derived`. It accepts exactly the form `TreePath::derived` writes: one component,
  the `\0derived:` prefix, and a method CID in its canonical text, which must re-encode to the same
  bytes. Anything else falls through to `from_components` and is refused. The JSON form still does
  not decode, as before.

  The doc's claim that "a serving tier admits it through the record it holds" was false. It now
  says no serving tier admits a derived item yet: `TierEndpoint` answers it
  `Refused { NotUnderRoot }`, and admission is slice 4's work (revision 34).

  Test: `derived_path_round_trips_in_dag_cbor_and_near_forms_are_refused`. It round-trips a bare
  path and a `BatchItem::derived` path, and refuses three near forms: a bad CID, a second
  component, and an uppercase CID text.
- **5. The outer-tier harness never tested wrong paths or reach.** Confirmed. In
  `assert_outer_tier_contract`:
  - Contract 9: a held object asked at a path that leads elsewhere (`one` at `d`) must be
    `Refused { NotUnderRoot }`, never Arrived.
  - Contract 10: the factory is called a second time with `OuterTierSetup.requester_admitted:
    false`. Every item, held or not, must get a `Refused`, the refusals must all be equal, and
    nothing may arrive.

  The endpoint loopback serves matthew or a stranger to match the setup, and it passes both
  contracts. Two bite tests:
  - `a_tier_serving_a_cid_at_any_path_fails_the_outer_tier_contract` uses a `PathBlind` double
    that verifies bytes but ignores the path. It panics on contract 9.
  - `a_tier_serving_a_requester_outside_reach_fails_the_outer_tier_contract` uses an endpoint
    whose gate admits the stranger. It panics on contract 10.
- **8. The serving adapter still mapped verdicts to answers itself.** Confirmed. eprfs-core gains
  `Membership::absence(self, by) -> Option<Unanswered<ItemAbsence>>` and
  `Membership::presence(self) -> Option<BlobPresence>`. Both are exhaustive matches inside the
  crate, and `None` means a member.

  `serve_batch` and `presence` only call them. The `_ =>` arm is gone, and so are the
  `ReachReason` and `Membership` imports in eprfs-serve.

  The two answers now differ on purpose, and the code says so in one place:
  - a presence question says `Unknown` for anything that leaves membership unproved;
  - a batch answers `Integrity` and `OwnFailed` with what each one is.

  The Membership registry row's summary and C4 now name both methods, and the eprfs-serve row's
  justification says the tier only sends them.
- **9. The two chains ordered their tiers differently.** Confirmed. The Rust chain now keeps
  `order: Arc<dyn TierOrder>` and orders its candidates inside `answer`, for the root asked.
  `TierChain::new` loses `root_hint`; it now takes 7 arguments, and the `too_many_arguments` allow
  is gone. `TierContractSetup.root_hint` became `order`.

  `fixtures/tree-vectors.json` gains an `order` section: four tiers and two roots, with
  `ProximityOrder`'s answer for each root computed by eprfs-core. The two answers differ, and
  vitest checks that `proximityOrder` matches both.

  Tests:
  - `a_chain_orders_its_tiers_for_the_root_asked`: one long-lived chain, with write-back refused
    so that both reads leave the device. Under root two it asks the tier warm for root two; under
    root one it asks the other tier first.
  - Vitest "orders its tiers for the root asked".

  The root-reach part is fixed in part; see below.
- **10. The byte ceiling was never fed, and per-requester budgets grew without bound.** Confirmed.
  - `SemaphoreGate::with_max_request_bytes` and its field are removed. The chain cannot give a
    hint (finding 1), so the ceiling bounded nothing. The trait keeps its `bytes_hint` parameter,
    as the spec gives it.
  - A per-requester gate keeps at most `max_requesters` budgets (default 1024, set with
    `with_max_requesters`). Once the map is full, idle budgets are dropped before a new one is
    made. Idle means no permit outstanding (`Arc::strong_count == 1`) and every slot free.
  - If every budget is busy, a new requester is refused by the named rule `<rule>:requesters`.
  - `requester_budgets()` reads the map's size.

  Test: `idle_requester_budgets_are_dropped_and_a_busy_full_gate_refuses_by_name`. A thousand
  requesters in turn never hold more than 2 budgets, and with two busy budgets a third requester
  is refused by `hub-forward:requesters`.

### Fixed in part, or refused

- **9, root reach: fixed in part.** The Rust chain still takes root reach from
  `BatchOptions.root_reach`, where the TypeScript chain binds a `rootReach(root)` lookup at
  construction. I did not move it. That field is an earlier recorded revision of §A.1, and the
  serving path depends on it: `serving_opts` fills it from `ReachGate::root_level`, never from the
  wire, and `OnBehalf` binds it. Moving it would undo that revision without a ruling, and on a
  serving tier the reach comes from the tier's records anyway. The divergence is recorded as
  revision 35 so the spec can choose one shape.
- **6. Reach is proved by inclusion alone: refused as a code change; it needs a spec ruling.** The
  finding holds as described. An object is served to anyone admitted to any root whose spine
  links it, and `ReachGate::admit_object` is never called. Spec §12.3 rule 2 ("tree nodes inherit
  their root's reach") permits this as written. Either candidate rule changes running behaviour
  beyond this item's sections:
  - prove membership only from spine nodes the tier holds itself, which stops a cold forwarding
    hub from serving at all;
  - require `admit_object` under the narrowest root the tier records the object under.

  Recorded for §A.5 as open question 36. It should be ruled before any remote tier ships. No
  covenant floor covers it, and this seat only flags it.
- **7. eprfs-core 0.2.0 depends on a crate that is not published: refused, outside the write-set
  and an operator item.** Confirmed against the index again: `elohim-seam-contracts` is not in
  the publish set. All three of the finding's options need something this seat cannot do:
  - (a) needs the licence ruling, a seam-contracts release and an entry in
    `scripts/ci/publish_eprfs.py`, all outside `elohim/eprfs/**`;
  - (b) would give `Unanswered` a second home, against ruling 7 and the operator's approval that
    eprfs-core admits seam-contracts;
  - (c), holding the workspace at 0.1.2, was checked against `publish_eprfs.py`. It re-verifies
    an existing version's payload against the local package and raises `PublishError` ("packaged
    manifest identity differs from index") when they differ. A 0.1.2 that carries this lane's code
    would therefore turn the dev publish stage red rather than hold the release.

  The landing blocker stands as findings 9 and 10 of Codex review 1 recorded it. This lane does
  not land: the brief allows no commits.

### New places the code and the interfaces spec disagree (for §6, continuing from 30)

31. **§4's bounded memory, and §A.2's byte bucket.** The chain bounds what it holds for write-back
    with `ChainLimits::max_write_back_bytes`, pausing and re-asking. Its peak is that bound plus
    one object. `ForwardGate::admit` receives `bytes_hint: None` from the chain, because no item
    size is known before arrival, and `SemaphoreGate` has no byte ceiling.
32. **§A.4/§A.5, the per-request bound.** `ChainLimits::max_request_items` is the bound the chain
    and a serving endpoint share, and its default is `TreeLimits::default().max_pending_entries`.
    It is a shared default, not negotiated: a tier configured below it still refuses a full round.
    A tier record that carries its own bound is left for the spec to decide.
33. **§A.4, forwarding.** A forwarded `via` is normalized (in order, without repeats, outer set
    only), and each tier is asked each item at most once per call, whatever it answers.
34. **§C2, derived items.** `TreePath::derived` has a binary (DAG-CBOR) form that decodes back,
    and no JSON form. No serving tier admits a derived item yet; slice 4 must.
35. **§A.4 and §H.1.** `TierChain::new` takes `order: Arc<dyn TierOrder>` and no `root_hint`, and
    orders per request. Root reach still differs between the twins: Rust uses
    `BatchOptions.root_reach`, while TypeScript uses `ChainOptions.rootReach(root)`.
36. **§A.5 and §12.3 rule 2, open.** Membership by inclusion alone lets any admitted root expose an
    object it links. This needs a ruling (finding 6).
37. **§A.5.** `Membership::absence` and `Membership::presence` are eprfs-core methods, and the
    serving adapter only calls them.
38. **§A.6.** `SemaphoreGate` caps per-requester budgets (`max_requesters`, rule
    `<rule>:requesters`) and drops idle ones.

### Files changed in this round

Everything is under `elohim/eprfs/**` except `genesis/manifests/habits.yaml` (generated,
re-projected) and this report.
- `eprfs-core/src/{tier,level_walk,derive,walk,lib}.rs`, `eprfs-core/tests/vectors.rs`,
  `eprfs-core/.epr-meta` (the rule's membership clause and the chain's rounds and ordering),
  `eprfs-core/seam-registry.yaml` (the forward_blobs row's C3, C6a, notes and tests; the Membership
  row; and line numbers)
- `eprfs-serve/src/{lib,testing}.rs`, `eprfs-serve/tests/endpoint.rs`,
  `eprfs-serve/seam-registry.yaml`
- `eprfs-storage/src/budget.rs`, `eprfs-storage/tests/tier_chain.rs`
- `eprfs-ts/src/{tier,index}.ts`, `eprfs-ts/test/{chain,vectors}.test.ts`
- `fixtures/tree-vectors.json` (the `order` section)
- `.epr-meta/materialize-what-is-read.habit.md` (a DELTA)

Tests added:
- endpoint.rs:
  - `a_chain_asks_an_endpoint_in_rounds_its_bound_admits`
  - `a_repeated_via_asks_each_upstream_once`
  - `the_chain_writes_back_between_rounds_within_its_byte_bound`
  - `a_chain_orders_its_tiers_for_the_root_asked`
  - `a_tier_serving_a_cid_at_any_path_fails_the_outer_tier_contract`
  - `a_tier_serving_a_requester_outside_reach_fails_the_outer_tier_contract`
- derive.rs: `derived_path_round_trips_in_dag_cbor_and_near_forms_are_refused`
- budget.rs: `idle_requester_budgets_are_dropped_and_a_busy_full_gate_refuses_by_name`
- the `order` vectors in both suites
- vitest: rounds, and per-root ordering

Assertions changed: the per-requester budget test no longer checks the removed byte ceiling.

Status stays **DONE_WITH_CONCERNS**. Finding 7's landing blocker (the seam-contracts release, the
publish set and the licence ruling) is outside this write-set. Finding 6 waits on a spec ruling,
and the TypeScript gate is still unwired.

## Codex integration, 2026-10-21

Resumed by `agent:implementer@codex` from the interrupted workflow at base `6cc1a17d5`.
`ReachGate` retains named local diagnostics, but `TierEndpoint` now normalizes every root-admission
failure before returning it to the requester. A private held root and an unknown root both return
`OutsideStanding` with no standing label, through the same complete `Unanswered` envelope. The
serving `read`, `list`, `diff` and batch paths share this rule; denied presence remains `Missing`.
This prevents the internal gate reason from disclosing whether the private root exists.

The regression `an_unadmitted_requester_cannot_distinguish_private_and_unknown_roots` compares the
complete envelopes for all four serving operations, compares presence, and asserts zero underlying
store reads. `eprfs-core/src/reach.rs` and `eprfs-serve/seam-registry.yaml` distinguish local gate
diagnostics from requester-visible refusal. Detailed host diagnostics are not added by this change.
The existing `reach-on-tree-root.feature` remains the household acceptance story; this test is its
local serving station, not evidence that the household scenario has run.

Story-graph station: `reach-on-tree-root / between root admission → stranger sees no root existence /
missing node: every denied serving operation returns the same complete envelope for a held private
root and an unknown root, without a store read / probe: the endpoint regression above / current
state: proved by the final native gate (`eprfs-gate-final.log`, `EXIT=0`)`.

The C6 integration also connects `@elohim/eprfs` to the pnpm workspace, the shared address verifier
and its manifest-owned TypeScript gate. Its build and package-consumer evidence is recorded in
`task-placement-c6-report.md`. Fresh `just gate eprfs-ts` passed its build, typecheck, purity lint
and 27 tests; `just gate storage-client-ts` passed 131 tests plus the plain Node ESM consumer smoke.
The earlier "unwired" observation is superseded by these results.

New spec revision 39: external root refusal is uniform; `ReachRefusal` remains an internal diagnostic
and must not cross that boundary unchanged. Open questions 36 (membership-by-inclusion versus
per-object restrictions) and the seam-contracts licensing/publication decision remain unresolved.
They must be settled before remote serving or publishing, respectively. This pass does neither.

Both affected seam registries validate against their schema (`EXIT=0`). Fresh `just gate eprfs`
returned **`EXIT=0`**, with 1,326 tests passed, no failures and no ignored tests. Formatting and
workspace/all-target Clippy with warnings denied also passed. The final run includes the new
complete-refusal-envelope regression, the 10,000-entry/7-node spine proof, its three-batch twin,
the tier contract and the outer-tier contract. Receipt:
`genesis/local-dev/placement-codex-handoff/eprfs-gate-final.log`.

The first integration run failed the existing
`migration_provider_is_the_session_claim_when_one_exists` test: its provider was correct but its
steward fell back to the repository rather than the fixture human. The same test passed once in
isolation, in 20 further same-environment repetitions, and in the final full-suite rerun (all 50
`flow_memory_import` tests passed). No reproducible cause was found; neither the assertion nor
production attribution code was changed. The original failure (`eprfs-gate.log`), focused run and
repeat receipts remain beside the final log. This is an observed intermittent test concern, not
a claimed fix. Historical green runs above are not attributed to the new refusal boundary.

Status remains **DONE_WITH_CONCERNS** at the interface/first-adapter boundary. The materialization
habit remains red until production callers and household acceptance satisfy its retirement rule.
