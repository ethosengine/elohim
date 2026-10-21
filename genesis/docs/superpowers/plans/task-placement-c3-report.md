---
id: placement-c3-report
title: "C3 report — ark-core task.rs: task spec, berth offer, task kind"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#26
actor: agent:implementer@claude-opus-5-5
commits: []
habits: [materialize-what-is-read]
cites:
  - "placement-c3-brief | C3 | sha256:fe86b7dd48c23651 | path: genesis/docs/superpowers/plans/task-placement-c3-brief.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:de0208f9b6e426db | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:5417b3363dffdc44 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
---

Item C3 (gap `#26`). The work is in the working tree and is not committed. Following the brief
(architecture before landing), nothing was staged, committed or pushed. The write-set was
`elohim/ark/core/**`. C2 (placement-core) and C4 (eprfs-core) were already in the tree, and this lane
built on them as they stand.

**Gate evidence:** `RUSTUP_TOOLCHAIN=1.96.1 just gate elohim-ark` → **`EXIT=0`**, final run on the
final bytes. The gate runs fmt `--check`, clippy `-D warnings` and `cargo test --all-targets` over
ark-core, ark-supervisor and ark. 144 tests passed with 0 failed. In ark-core: 74 unit tests, 6 in
`candidate_contract` and 15 in the new `task_contract`. Separately,
`cargo clippy -p elohim-ark-core --all-targets -- -D warnings` → **`EXIT=0`** (the gate's clippy step
leaves out test targets). Two earlier gate runs gave `EXIT=1`:
- the first had a type error in the arbitration test;
- the second had four test failures, caused by a fixture whose berth capacity was smaller than the
  task's disk plus its view size.

Both were fixed before the final run. Cargo berth: `berth claim cargo --class verify --ttl 3600`
(`EXIT=0`) and `berth release cargo` (`EXIT=0`, "released by" this session).

**Mutation check:** I disabled three checks in `task.rs` at once:
- the exec-binding check;
- the refusal of a derivation record on a non-clean exit;
- the posture's reach filter on held roots.

The run was `cargo test --test task_contract --no-fail-fast`, giving **`EXIT=101`** with exactly four
failures: `new_and_decode_refuse_…`, `two_specs_with_one_method_cid_…`, `a_failed_run_records_no_derivation`
and `a_berth_offer_carries_no_local_path_…`. I then restored the source byte-identical (`cmp`) before
the final gate.

## Files changed

| File | Change |
|---|---|
| `elohim/ark/core/src/task.rs` (new) | `TaskSpec`, `TaskChild`, `BerthOffer`, `TaskKind` (`impl PlacementKind`), `TaskOutcome`, `From<EffectiveTier> for Isolation`, `From<&ResourceQuota> for Capacity`, `placement_depth`/`eprfs_depth`, `MAX_OFFER_HOLDS`, `TASK_CHILD_NAME` |
| `elohim/ark/core/src/lib.rs` | `pub mod task` and its re-exports; `task.rs` added to the purity scan's `SOURCES`; rail R-A1 rewritten (below) |
| `elohim/ark/core/src/manifest.rs` | `Eq` derived on `ArtifactRef`, `StdinSource` and `Probe`, which the spec's `TaskChild: Eq` needs. This is additive and moves no encoding |
| `elohim/ark/core/Cargo.toml` | `eprfs-core` and `elohim-placement-core` as path dependencies. Dev-dependencies: placement-core `testing` and seam-contracts `harness` |
| `elohim/ark/core/tests/task_contract.rs` (new) | the §F contract suite |
| `elohim/Cargo.lock` (outside the write-set; cargo wrote it) | elohim-ark-core gains two edges, and an `eprfs-core 0.2.0` package entry is added. The placement-core entry came from C2 |

**Rail R-A1.** `no_runtime_or_io_deps` used to look for the needles `"\n{pkg} "` and `"\n{pkg}="`, and
so missed `[dependencies.tokio]`, `tokio.workspace = true`, `package = "tokio"` renames, and the
`[target.*]` and `[build-dependencies]` tables (C2's concern). It now parses the manifest's non-dev
dependency tables. It also checks eprfs-core's and placement-core's own `[dependencies]` one hop down,
so a runtime that arrives through either new core fails ark-core's build. A new parser test covers 8
forms that must be seen and 4 that must be skipped. `no_fs_in_pure_core` and
`boundary_scan_covers_every_module` pass with `task.rs` scanned.

## Tests added (`tests/task_contract.rs`, 15)

Each test pins a §F contract clause:
- an absent envelope moves no CID (`TaskSpec` and the exec binding are byte-identical to a struct with no `envelope` field);
- `new` and decoding both refuse a child or envelope that is not the method's `exec`, an input whose paths are not the view, and an unknown field (JSON and DAG-CBOR round trips);
- two specs under one method CID run one program;
- the child is `Restart::Temporary` only. A passphrase stdin, `env_scrub: false`, a channel artifact, an empty argv, a pinned `cid` that is not the raw CID of its sha256, an output prefix that escapes `{output}`, a raw input root and an empty view are all refused;
- the need, scope and derivation key: `{1,1}`, bytes = disk + view, carriers, terms; `a2o-stage:` gives `measure-stage`, and anything else gives `sweettest-feedback`;
- no `BerthOffer` contains a data_root, passphrase, port, port key or artifact path, and no posture lists a root below its own reach. A duplicate root keeps its narrower reach. An empty passport has `None` isolation, and otherwise isolation is the weakest tier;
- holds are sorted and capped at 64, and capacity is the declared bound with `disk_bytes` as bytes;
- the isolation and depth ladders map 1:1;
- the kind passes `assert_scope_is_derived`, and its default method is NearestBytes, which is the built-in Task set's;
- a forged scope and an unread spec are both refused `Kind { "scope" }`;
- `Kind { "scope" }`, `Kind { "isolation" }` and `OverCapacity` (pids) agree between core's `admit` and `TaskKind::fits`, and an honest need is charged exactly what it requires;
- with no matching grant every remote berth is `NoConsent` (absent, `NotNeeded`, wrong scope); the requester's own envelope is admitted, never `IsReceiver`; `--on` gives `NotNamed` for every other carrier;
- NearestBytes ranks input holder, then base holder, then neither, under all six role-to-agent assignments;
- the single-holder choice passes `check_arbitration`;
- a failed run records no derivation, and a record's output must be the outcome's.

## Where the code and the interfaces spec disagreed (revisions for the spec seat)

1. **`TaskKind` cannot be a unit struct.** A need carries only `NeedKey::Task(cid)`, so the scope
   cannot be derived from the `TaskSpec` without the spec itself. `TaskKind` therefore holds the specs
   this peer has read (`TaskKind::new().with_spec(&spec)?`, `knows(&cid)`).
   - If a task's spec has not been read, the kind derives no scope. Core then refuses the declared
     scope as `Kind { "scope" }`, and `fits` refuses as `Unknown { input: "task-spec" }`, so an unread
     spec is never carried.
   - `action` for an unread task falls back to `required_scope(None)`. That fallback names no carrying.
   - One consequence: the first refusal an unread task shows is `Kind { "scope" }`, not "spec unknown".
     A truer label would need a core stage, which is the spec seat's call.
2. **`as_need` gains a parameter, `view_bytes: Option<u64>`.** The spec says bytes = `disk_bytes` plus
   the materialized view's size, but no input in the signature carries that size. The size is the
   requester's own reading from `walk_spine`. With neither figure given, no byte requirement is
   invented.
3. **"the envelope's isolation" does not exist.** The six-field `RuntimeEnvelope` declares no
   isolation floor, so `as_need` sets `requires.isolation: None`. The isolation refusal is still
   implemented and tested for needs that carry a floor. Adding a floor to the envelope (skipped when
   absent, so no CID moves) is a manifest-schema decision.
4. **`TaskSpec::scope` cannot be `None` today.** `required_scope` is total, so the refusal "when the
   method's derived scope is none" cannot happen. The scope is read from `method.name`, which stands in
   for `compute_tasks.rs`'s `envelope.project` (`a2o-stage:<stage>`). The spec says "one way from the
   method" but does not name the field; I chose `name`.
5. **`exec` binding encoding.** `method.exec` is defined as `BlobCid::compute(dagcbor({ child, envelope? }))`,
   with the envelope omitted when absent. `TaskChild::exec_cid(envelope)` is public so method authors
   compute exactly this. The spec says "the CID of (child, envelope)" without fixing an encoding.
6. **Decoding enforces `new`'s checks.** The spec derives `Deserialize` directly, which would admit a
   decoded spec that `new` refuses. The refusal "when `input`'s paths are not the descriptor's view"
   only means anything at decode time. `TaskSpec` now decodes through `try_from` a wire struct with
   `deny_unknown_fields`.
7. **The `TaskChild` subset is narrowed for safety.** The spec lists `env_scrub` and `stdin` as fields
   without restricting them. `new` refuses `stdin: Passphrase`, because it would hand the berth's
   passphrase to a requester's program. It also refuses `env_scrub: false`, because the program would
   inherit the carrying ark's environment.
   - Further refusals: a `Channel` artifact (the spec says `ArtifactRef::Pinned`), a pinned `cid`
     that is not `raw_cid_from_sha256_hex(sha256)`, and an output prefix that is empty, absolute, or
     contains `.`, `..` or `\`.
   - The task root and base root must be DAG-CBOR CIDs, which `TreeView`'s own decoder requires.
   - `to_child_spec()` names the child `TASK_CHILD_NAME` ("task"), because `TaskChild` has no name.
8. **The Depth map is two functions, not `From`.** Both `Depth` types are foreign to ark-core, so the
   orphan rule forbids `From`. The functions are `placement_depth` and `eprfs_depth`.
9. **`BerthOffer::from_berth` stays infallible.** `manifest` is copied from `berth.manifest`, and
   capacity is read from the `manifest` argument. Their consistency is not checked; a fallible form
   would be the spec seat's call. Holds are deduplicated by root, keeping the narrower reach, and then
   the first 64 in CID order are kept. The spec does not say which 64 to keep.
10. **`TaskOutcome` gains a checked constructor.** The spec shows only public fields. `TaskOutcome::new`
    refuses a `DerivationRecord` unless the exit is `Exited { code: 0 }`, and refuses a record whose
    output is not the outcome's. A failed run may still carry an output, such as logs.
11. **`TaskSpec` serde.** `base_root` serializes as `null` when absent, as the spec shows: it has no
    `skip_serializing_if`, unlike `envelope`. If a `base_root`-less spec should keep a pre-field CID,
    that rule needs the same skip.

## Concerns

- **Consumer lockfiles are stale.** ark-core now depends on eprfs-core, which brings in `eprfs-core`,
  `multihash-codetable` and `serde_bytes`. `cargo metadata --locked` now fails in
  `elohim/elohim-storage` (`EXIT=101`) and in `bridges/k8s` (`EXIT=101`); both say "cannot update the
  lock file". Neither lockfile is in this write-set, so neither was touched.
  - elohim-storage's lock belongs to C5, which adds eprfs-core there anyway.
  - `bridges/k8s/Cargo.lock` has no owning lane.
  - Both must be refreshed before anything lands, or a `--locked` CI build of either fails.
- **Not built here:**
  - the §F flow steps 1–9: walk/project, the `DerivationIndex` lookup, provider-less acceptance in the
    `compute_task.rs` coordinator, TaskRunner materialization with `{input}`/`{output}` and sealing,
    the completion event, the `PendingStatus` cursor;
  - `rea.rs`'s REA intent for the berth offer;
  - the schema mappings for `compute-task` and `compute-receipt`;
  - the contracts "a derivation hit raises no need", "one request yields one commitment" and "a
    completed task is never re-adopted". These live in eprfs-core (slice 4), the content_store zome,
    ark-supervisor and placement-core's floor, not in ark-core.
- **No seam-registry row was added** for `TaskKind`/`TaskSpec::new` in `elohim/ark/core/seam-registry.yaml`.
  Their concern-by-concern statuses deserve a reviewed pass, not a guessed one.
- The brand-vocabulary advisory flagged the `MishpatAction` import. That is placement-core's spec name,
  already noted by C2.
- The habit atom delta for `materialize-what-is-read` is not written. This lane adds no adapter that
  materializes anything, and the atom lives outside this write-set.
- The covenant floor was not needed: every edit stayed inside the write-set, except the cargo-written
  `elohim/Cargo.lock` edges described above.

## Codex review 1 fixes

I re-checked all seven findings against the code. They collapse to four distinct defects, and all
four held, so all are fixed. No finding was refused outright. One part of finding 7 was answered by
a different mechanism than the one proposed; see (d).

**Gate evidence (round 1, final bytes):** `CARGO_BUILD_JOBS=1 RUSTUP_TOOLCHAIN=1.96.1 just gate elohim-ark`
→ **`EXIT=0`**, with 149 passed and 0 failed:
- ark-core: 77 unit, 6 `candidate_contract`, 17 `task_contract`;
- supervisor and cli: unchanged.

Also on the final bytes:
- `cargo test -p elohim-ark-core --doc` → **`EXIT=0`** (3 `compile_fail,E0616` doctests). The gate's
  `--all-targets` does not run doctests, so this is the only witness for them.
- `cargo clippy -p elohim-ark-core --all-targets -- -D warnings` → **`EXIT=0`**.
- `cargo tree -p elohim-ark-core -e features -i elohim-compute` → **`EXIT=0`**. No `default` or
  `runtime` feature node for elohim-compute is reached.

Earlier attempts this round:
- 1st: `EXIT=1`, from `fmt --check`; `cargo fmt -p elohim-ark-core` fixed it.
- 2nd and 3rd: `EXIT=1`, recipe exit 143. The workspace RAM guard shed clippy, then the test build,
  at 80% committed memory held by other sessions. Re-run with `CARGO_BUILD_JOBS=1`.

Cargo berth: `berth claim cargo --class verify --ttl 3600` (`EXIT=0`), then `berth release cargo`
(`EXIT=0`).

The mutation check was not run this round. The permission classifier refused the sed that would have
disabled the floor and terms checks. The new tests are the only witness that those checks bite: they
fail if a check is removed, which shows when the checks are disabled by hand, but not by an automated
mutation run.

**(a) Findings 1 and 5. An unknown view size became zero bytes. Fixed.**
- `TaskSpec::as_need` now takes `view_bytes: u64`, which is required. This supersedes revision 2's
  `Option<u64>`.
- `bytes` is `disk_bytes.unwrap_or(0).checked_add(view_bytes)`. An overflow refuses with
  `ManifestError::Invalid`, so every task need states a byte requirement.
- `TaskKind::fits` also refuses a task need whose `requires.bytes` is `None` as
  `Unknown { input: "view-bytes" }`, so a hand-built need cannot skip the view either.
- Tests:
  - an envelope-less need's bytes are the view's size;
  - `u64::MAX` disk plus 1 refuses;
  - a sizeless need is refused by both `fits` and core `admit`.

**(b) Findings 2, 4 and 6. Public fields let a constructed value bypass validation. Fixed by making
the invariant-bearing fields private.**
- `TaskSpec`'s six fields are private, with getters `method()`, `input()`, `base_root()`, `child()`,
  `envelope()`, `outputs()` and a new infallible `child_spec()`.
- Every `TaskSpec` value has therefore passed `new` or `try_from` decoding, and `Clone` preserves
  that. Revalidating at each consumer would be redundant. `TaskKind::with_spec` still calls
  `spec.check()` at the registration boundary, as defense in depth.
- `TaskOutcome`'s fields are private too, with getters `task()`, `exit()`, `output()`, `record()` and
  `completion_event()`. A built outcome cannot gain a record or a failing exit.
- `TaskChild` keeps public fields. It is the raw declaration a spec is built from.
  `TaskChild::to_child_spec` now returns `Result` and runs the child's checks, so an unsafe child
  (passphrase stdin, unscrubbed environment, channel artifact) never becomes a `ChildSpec`.
- Regressions:
  - three `compile_fail,E0616` doctests in `task.rs`: mutating `spec.child`, assigning
    `spec.envelope`, assigning `outcome.exit`. A literal construction fails on the same privacy
    error;
  - `to_child_spec` refuses a passphrase-stdin child and an unscrubbed child.
- **Spec revision:** §2.F shows `pub` fields on `TaskSpec` and `TaskOutcome`, and an infallible
  `to_child_spec`. Both should change to the private-field and checked forms described here.

**(c) Finding 3. Admission trusted requirements the need could lower. Fixed.**
- For each read spec, `TaskKind` now keeps a `Registered { scope, input_root, base_root, floor }`
  record. `floor` is `Capacity::from(&envelope.bound)`.
- `fits` refuses a need whose `NeedTerms::Task` input or base root is not the spec's, or whose terms
  are not `Task` at all, as `Kind { label: TASK_TERMS }`. `TASK_TERMS` is `"task-terms"` and is
  re-exported.
- `fits` checks and charges every component at `max(need.requires, floor)` against `remaining`.
  Core's `floor_charge` then takes `max(asked, charge)`, so lowering a need under an unchanged
  `NeedKey::Task` never admits it for less than its envelope.
- Placement-core's `stage_capacity` still reads only the supplied requirements. It runs before
  `fits`, and `fits` always runs before a charge, so the kind's refusal is final. Placement-core was
  outside this write-set and is unchanged.
- Tests:
  - `a_need_cannot_lower_its_requirements_below_the_specs_envelope`: charged the envelope bound when
    there is room, and `OverCapacity` from both core and kind when only the lowered need fits;
  - `a_need_whose_roots_are_not_its_specs_is_refused`: a swapped input root, a swapped base root, a
    dropped base root, and `NeedTerms::None`.
- **Residual:** the view-size part of `bytes` cannot be checked against the spec, because the spec
  does not carry it (revision 2). The kind enforces only `bytes >= disk_bytes`. The carrying berth's
  TaskRunner must measure the view when it materializes it.

**(d) Finding 7. Compute's `runtime` feature was reachable through ark-core. Fixed.**
- ark-core's `elohim-compute` dependency now has `default-features = false`. ark-core uses only
  `Governor`, `LimitOwner`, `Refusal` and `RefusalCode`, all from `actuation`, which is not gated.
  The supervisor and cli do not declare compute, so they cannot have relied on its runtime feature.
- New rail `boundary::compute_runtime_is_unreachable`:
  - every manifest in the pure graph (ark-core, eprfs-core, placement-core) that names compute must
    disable its default and must not name `"runtime"`;
  - compute's own `chrono` must be `optional = true`, which pins the premise the rail relies on.
- The dependency parser now returns each dependency's spec text (`declared_dependencies`).
  `the_runtime_check_refuses_every_form_that_reaches_it` covers 4 reaching forms, 2 non-reaching
  forms and the dotted-table body.
- New rail `boundary::no_clock_in_pure_core` refuses `Utc::now`, `Local::now`, `SystemTime::now` and
  `Instant::now` in any scanned source.
- **Partly answered differently:** ark-core's own direct `chrono` dependency predates C3 and is used
  for `DateTime` types in `rea.rs` and `candidate.rs`. It still compiles chrono's `clock` feature,
  because the workspace `chrono` entry keeps chrono's defaults, and a member cannot turn off a
  workspace default. The no-clock boundary is therefore enforced at source level by the new rail,
  not by the dependency graph. Removing the clock from the graph needs a workspace-level chrono
  change, which is outside this write-set.

**Files touched this round:**
- `elohim/ark/core/src/task.rs`
- `elohim/ark/core/src/lib.rs`: re-exports `TASK_TERMS`; adds the two rails and the parser refactor
- `elohim/ark/core/Cargo.toml`: compute's `default-features = false`
- `elohim/ark/core/tests/task_contract.rs`: getter call sites, the required view size, and 2 new
  tests

No lockfile moved; a feature change does not alter `Cargo.lock`. Nothing was staged or committed.

## Codex review 2 fixes

I re-checked all seven findings against the code. They reduce to six distinct defects: findings 1 and
3 are the same one. Every defect held.
- Five are fixed as proposed.
- Finding 6's literal fix is refused, because the file it would change is outside this lane's
  write-set. The defect is closed inside the write-set by another mechanism; see (e).

**Gate evidence (round 2, final bytes):** `CARGO_BUILD_JOBS=2 RUSTUP_TOOLCHAIN=1.96.1 just gate elohim-ark`
gave **`EXIT=0`**, with 154 passed and 0 failed:
- ark-core: 78 unit, 6 `candidate_contract`, 21 `task_contract`;
- supervisor: 28 unit, 5 `native_reap`, 9 `supervise_death`;
- cli: 7 `cli_smoke`, and 0 in its `main.rs` unit target.

Also on the final bytes:
- `cargo test -p elohim-ark-core --doc` gave **`EXIT=0`** (3 `compile_fail` doctests).
- `cargo clippy -p elohim-ark-core --all-targets -- -D warnings` gave **`EXIT=0`**.
- `cargo fmt -p elohim-ark-core` gave `EXIT=0` before the runs.

Cargo berth: `berth claim cargo --class verify --ttl 3600` (`EXIT=0`), then `berth release cargo`
(`EXIT=0`).

**Mutation check (run this round).** I backed up `task.rs` and applied four mutations at once:
- `holds` made `pub`;
- the output selection dropped from the exec binding;
- the byte floor reduced to the disk bound alone;
- the decoded-holds count check disabled.

`cargo test -p elohim-ark-core --all-targets --no-fail-fast` gave **`EXIT=101`** with exactly these
failures:
- `invariant_bearing_fields_stay_private`;
- `a_need_cannot_lower_its_requirements_below_the_specs_envelope`;
- `admission_charges_the_disk_bound_plus_the_registered_view`;
- `different_output_selections_never_share_a_derivation_key`;
- `a_decoded_berth_offer_is_bounded_and_lists_each_root_once`.

I then restored the source from the backup, `cmp` gave `CMP=0`, and the final gate ran on the
restored bytes.

**(a) Findings 1 and 3: admission charged the disk bound without the view. Fixed.**
- `TaskKind::with_spec(&spec, view_bytes: Option<u64>)` now takes the registering peer's own
  measurement of the materialized view. This is the `walk_spine` reading `as_need` already takes.
- The registered byte floor is `disk_bytes.unwrap_or(0).checked_add(view)`. An overflow refuses at
  registration.
- A spec registered with `None` is refused at every admission as `Unknown { input: "view-bytes" }`,
  whatever bytes the need claims.
- `fits` charges `max(need.bytes, disk + view)`.
- New test `admission_charges_the_disk_bound_plus_the_registered_view`:
  - a berth with exactly `disk_bytes` of room is `OverCapacity` from core and from the kind, both for
    a need lowered to 1 byte and for the honest need;
  - with exactly `disk + view` of room, the need is admitted and charged the sum;
  - an unmeasured registration is refused;
  - an overflowing registration is refused.
- The round-1 test now expects `charge.bytes == (1 << 20) + VIEW`, not `1 << 20`.
- **Spec revision:** §2.F's unit `TaskKind` (already revised to a registry in revision 1) takes the
  view size at registration. This closes round 1's stated residual, "the kind enforces only
  `bytes >= disk_bytes`".

**(b) Finding 2: the berth offer's REA intent was missing. Fixed** in `rea.rs`.
- New method: `BerthOffer::as_rea_intents(&self) -> Result<Vec<ReaIntent>, ReaProjectionError>`.
- The projection reuses the module's existing vocabulary:
  - verb `ReaVerb::Use`: a task uses the capacity and returns it, so no new verb is minted;
  - the two-slot `classified_as`: `["runtime:berth-offer", <berth CID>]`;
  - `in_scope_of`: the berth's manifest CID, parsed as `RuntimeScope::from_berth` parses it;
  - `raised_by`: the blade.
- The projection emits one intent per declared capacity component, each a `Magnitude::Count`. The
  units are `disk-bytes`, `memory-bytes`, `cpu-millis` and `pids`, because a VF intent carries a
  single quantity.
- A berth with no declared envelope projects one intent with no quantity, as `Intent::as_rea_intent`
  does.
- A manifest that is not a CID is refused with `ReaProjectionError::Cid`.
- Holds, scopes and isolation are the posture's business, not offered resources, so they are not
  projected.
- New exports: `RUNTIME_TAG_BERTH_OFFER`, `UNIT_DISK_BYTES`, `UNIT_MEMORY_BYTES`, `UNIT_CPU_MILLIS`
  and `UNIT_PIDS`.
- New test `a_berth_offer_projects_one_use_intent_per_offered_resource` covers attribution, scope,
  the offered resources and their units, the unquantified case and the non-CID refusal.
- This supersedes the round-0 concern that listed "`rea.rs`'s REA intent for the berth offer" as not
  built.
- **Spec revision:** the spec names where the intent is written, not its shape. The shape above (a
  `Use` verb, one intent per component) is for the spec seat to confirm.

**(c) Finding 4: different output selections shared a derivation key. Fixed.**
- The exec binding is now `{ child, envelope?, outputs? }`. The selection is normalized (sorted,
  each prefix once) and omitted when empty, so a selection-less task keeps its former exec CID.
- `TaskChild::exec_cid(envelope, outputs)` therefore takes the selection.
- `TaskSpec::new` normalizes `outputs`. `check` (shared by decoding) refuses a selection that is
  not already normalized, so a decoded spec's CID is the CID of the bytes it was decoded from.
- The method CID now covers the selection, so the derivation key (method CID, projected input)
  differs whenever the selection differs.
- New test `different_output_selections_never_share_a_derivation_key` covers:
  - `["report"]` and `["logs"]` give different method CIDs and different keys;
  - a method bound to `["report"]` refuses a spec selecting `["logs"]`;
  - `["report","logs"]` and `["logs","report","logs"]` give one spec and one key;
  - a decoded unsorted selection is refused.
- The fixtures now bind the selection through `method_with`. Every expected-error call passes the
  bound selection, so each one still fails for the reason it names.
- **Spec revision:** F2's "`exec` is the CID of (child, envelope)" becomes (child, envelope,
  normalized outputs).

**(d) Finding 5: the holds bound and normalization could be bypassed. Fixed.**
- `BerthOffer.holds` is private, read through `holds()`.
- `BerthOffer` decodes through `try_from` a `deny_unknown_fields` wire struct. The decoder refuses
  more than `MAX_OFFER_HOLDS` roots, and roots that are not strictly ascending. A root listed twice
  at any reach, including intimate and public together, is not ascending, so it is refused.
- `from_berth` already produces exactly this form.
- The other fields stay public. They are independent declarations with no invariant between them;
  the tests mutate `capacity` to build tight berths.
- New test `a_decoded_berth_offer_is_bounded_and_lists_each_root_once`:
  - 65 roots are refused;
  - exactly 64 are admitted;
  - one root at intimate and public is refused;
  - unsorted roots are refused;
  - an unknown `dataRoot` field is refused.
- **Spec revision:** §2.F shows `pub holds` and a derived `Deserialize`. Both change to the private,
  checked form.

**(e) Finding 6: the privacy rails did not run in the gate. Fixed by a different mechanism; the
proposed fix is refused.**
- **Refused:** adding `cargo test -p elohim-ark-core --doc` to `_gate-elohim-ark`. That recipe is in
  the root `justfile`, and the manifest's `run.recipe` points to it. Both are outside this lane's
  write-set (`elohim/ark/core/**`).
  - The finding itself is correct: Cargo's `--all-targets` excludes doctests.
  - Recommended for the lane that owns the root `justfile`: append
    `&& cargo test -p elohim-ark-core --doc` to `_gate-elohim-ark`.
- **What closes it here:** a new boundary rail, `invariant_bearing_fields_stay_private`, in
  `src/lib.rs`. It runs under the gate's `--all-targets`. It reads `task.rs` and fails if any field
  of `TaskSpec` or `TaskOutcome`, or `BerthOffer.holds`, is declared `pub`, and it asserts that it
  read at least one field of each struct.
- The mutation check above shows the rail fails the gate when `holds` is made public.
- The three `compile_fail` doctests stay as a second witness. They passed with `--doc` this round.

**(f) Finding 7: the hand-written dependency parser could be bypassed. Fixed.**
- The line scanner is gone. `boundary::declared_dependencies` now parses each manifest with
  `toml_edit`. It is a dev-dependency only, `default-features = false, features = ["parse"]`, at the
  0.25.11 the workspace lock already carried through proc-macro-crate. The lock gains only the edge
  from ark-core and no new package.
- It reads `[dependencies]`, `[build-dependencies]` and `[build_dependencies]` structurally, at the
  top level and under every `[target.*]`.
- Each entry resolves `package` (in any string style), `default-features` or `default_features`,
  `features` and `optional`.
- `compute_runtime_is_unreachable` and the chrono-optional check now read those fields rather than
  substrings.
- Regression cases added:
  - seen: `[dependencies] # runtime dependencies`; `rt = { package = 'tokio', version = '1' }`; the
    dotted single-quoted rename; a quoted key; a compact inline table; `[build_dependencies]`; a
    commented `[target."cfg(unix)".dependencies]`;
  - skipped: `[dev_dependencies]`, and a commented-out `# tokio = "1"` line.
- The runtime-check test covers 6 reaching forms (including a single-quoted `['runtime']`) and 4
  non-reaching forms (including a single-quoted renamed compute).
- I also moved a doc comment that round 1 had attached to the wrong test (`no_fs_in_pure_core`'s doc
  sat on `no_clock_in_pure_core`).

**Files touched this round** (all inside `elohim/ark/core/**`, plus the cargo-written lock edge):
- `src/task.rs`
- `src/rea.rs`
- `src/lib.rs`: re-exports, the TOML-backed parser, the privacy rail and the doc-comment move
- `Cargo.toml`: the `toml_edit` dev-dependency
- `tests/task_contract.rs`: the `outputs()`/`VIEW`/`method_with` fixtures, call-site updates and 3
  new tests
- `elohim/Cargo.lock`: `toml_edit` added to elohim-ark-core's dependency list

Nothing was staged or committed.

**Concern carried forward.** The doctest step for the root `justfile` is a one-line change for its
owning lane. Until it lands, the source-scan rail is the gate's only witness for field privacy.

## Codex integration addendum — 2026-10-21

Actor: `agent:implementer@codex`. This append preserves the original seat and review history above. Claim for gap `#26` was recorded once with the exact brief and `materialize-what-is-read`, `EXIT=0`. No files were staged or committed; scoped commits: none.

### Current revision summary

The validated, private `TaskSpec`/`TaskOutcome` and normalized, bounded `BerthOffer::holds()` contracts from the preceding review remain the current interfaces. `TaskKind` uses the registered measured input-view footprint plus the trusted task envelope; a public need cannot lower that admission charge. The reported private-field analyzer error was already absent from the current test source at handoff, and the fresh gate compiles the consumer tests successfully.

Integration adds `a_pending_task_reserves_its_trusted_view_before_earlier_classes` to `task_contract.rs`. It exercises the production `TaskKind` against placement's pending reservation pass: a 4,096-byte measured view, a one-byte declared pending requirement and a 4,097-byte berth. A task in an earlier serve class cannot double-book that space. The test covers both need orders and an unknown pending-view measurement, which conservatively holds the offered room. Root independently reviewed the changed reservation logic and regression with no finding; the regression passed the declared gate.

The historical missing-doctest concern is closed by the owning root lane. `_gate-elohim-ark` now also runs `cargo test -p elohim-ark-core --doc`. Both the all-target privacy rail and all three compile-fail doctests passed in the actual gate.

### Actual integration evidence

The root orchestrator ran native gates serially with the cargo berth and pinned Rust 1.96.1. Durable logs are under `genesis/local-dev/placement-codex-handoff/`.

- **Gate evidence:** `just gate elohim-ark` → `EXIT=0` (`elohim-ark-gate-final.log`): fmt and Clippy passed; 7 CLI smoke, 78 core unit, 6 candidate contract, 22 task contract, 28 supervisor unit, 5 native reap and 9 death-supervision tests passed, followed by 3 core doctests. No test failed or was ignored.
- **Gate evidence:** `just gate elohim-placement` → `EXIT=0` (`elohim-placement-gate-final.log`): 129 placement tests/doctests plus 21 compute tests with default features off passed; both Clippy legs passed.
- **Gate evidence:** `just gate eprfs` → final `EXIT=0` (`eprfs-gate-final.log`): 1,326 tests passed. The initial run's unrelated migration-steward assertion remains an unexplained intermittent failure; the unchanged test passed 21 focused repetitions and the full rerun. No fix is claimed for that failure.
- The k8s consumer lock was refreshed by Cargo after extending the integration write-set. The first offline attempt selected unrelated cached versions and failed on a missing archive; its generated delta was discarded. Online metadata then completed `EXIT=0` while preserving every existing package version. The final lock adds only `async-trait`, `elohim-placement-core` and `eprfs-core`, and removes the now-unreachable compute runtime dependency graph. The consumer's locked gate and locked metadata are pending at this append; their completion evidence will be appended before terminal fulfilment.

### Scope still open

The runner does not yet materialize task inputs/outputs, consult or seal derivations, or record task completion through the production path. Production Floor wiring and actual custody/provider evidence remain separate follow-ons. This addendum claims pure contract and native test evidence; the household mesh stayed stopped and no live task execution is claimed.

### Consumer verification complete — 2026-10-21

**Status: `DONE_WITH_CONCERNS`.** The consumer rows pending in the integration addendum are now complete. The remaining concerns are the explicitly scoped runtime follow-ons and the unexplained earlier eprfs test failure, whose unchanged final full gate passed.

**Gate evidence:** `just gate k8s-bridge` → `EXIT=0` (`k8s-bridge-gate-final.log`): format, workspace/all-target Clippy with `--locked`, and workspace tests with `--locked` passed. The suite ran 1 binary, 17 contract and 8 observer tests, all green. `cargo metadata --manifest-path bridges/k8s/Cargo.toml --format-version 1 --locked --offline` → `EXIT=0` (`k8s-metadata-locked-final.log`), using the declared native pool and pinned toolchain. The reviewed Cargo-generated lock preserves every prior package version and contains only the three necessary additions and removal of the unused compute runtime graph.

The C3 scope has no remaining code or gate blocker. Scoped commits: none; all changes remain reviewable in the shared working tree.
