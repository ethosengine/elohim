---
id: artifact-store-task-e4-report
status: claimed-not-verified
landed_commit:
verified_by:
concerns: [R4-integrity-class, review-fixes-1-4, store.rs-over-soft-LoC-ceiling, no-CI-home-for-the-new-crate's-tests, licence-of-the-moved-code]
actor: agent:e4@claude-opus-5-5 (Claude Opus 5.5, session f1769d1c)
---

# Task E4 — one authoritative payload store per device: report

Brief: `briefs/E4-one-store.md`, amended by the orchestrator's message after astra's design
challenge (`genesis/local-dev/as-e4/e4-gate.receipt.md`), rulings R1–R8 below. Branch
`artifact-store/e4` on top of `63ae4070d` (E2 null-ext refusal) and `e42b5cbe5` (blob_cid
rename), neither rewritten.

## Commits

| commit | rulings | what |
|---|---|---|
| `45f807408` | R1, R2, R3, R4, R8 (crate metadata, gate) | `crates/elohim-blob-store` (store, marker, tier, multi-process tests, own lockfile); storage shim, tier re-export, `compose_payload_tier_from_env`; Dockerfile; gate project + recipe; storage watch glob |
| `54f065fae` | R5, R6, R7 (CLI side) | epr-cli `payload.rs`, feature `blob-store` (off), serve/put open inside their runtimes; `tests/payload.rs`; `tests/artifact.rs` reads through the marker; watch globs for the epr-cli closure |
| `3fdd51e6e` | R7 (storage side) | `main.rs` holds the composed tier |
| `371b627dc` | R8 | eprfs-storage 0.3.7; `elohim/eprfs/Cargo.lock`, `elohim/Cargo.lock` |
| (this report) | | |

Steps 1 and 2 of the brief share one commit: the owner/shared split (R4, as replaced by the
amendment) is a rewrite of the store's coordination, so the store was written once with it
rather than moved and then rewritten.

## What moved

- `elohim/elohim-storage/src/blob_store.rs` → `crates/elohim-blob-store/src/store.rs`: the whole
  store and every one of its tests, now returning its own `BlobStoreError`. The crate is its own
  workspace with its own committed `Cargo.lock`, `publish = ["elohim"]`, path + version + registry
  deps (`elohim-epr` 0.2; optional `eprfs-core` 0.3.4 behind feature `tier`), like
  `crates/elohim-address`. It knows nothing of elohim-storage.
- `elohim/elohim-storage/src/eprfs_tier/blob_store_tier.rs` → `crates/elohim-blob-store/src/tier.rs`
  (feature `tier`) with its six tests. `dedicated(root)` keeps `root/eprfs-tier`; new
  `at(root, OpenMode)` opens exactly `root`; `over(Arc<BlobStore>)`. The storage-local
  `epr_codec::DAG_CBOR_CODEC` became the canonical `elohim_epr::address::RECORD_CODEC`.
  New `compose_shared_payload_tier(root)`: a shared tier iff `root/STORE` says `blob-store-v1`,
  else `NotBlobStore { marker }` with nothing opened or created.
- New `crates/elohim-blob-store/src/marker.rs`: `FILE = "STORE"`, `BLOB_STORE_V1`, `read()`.
- elohim-storage keeps every name. `src/blob_store.rs` is a newtype shim (`BlobStore { core }`)
  with a delegating method for every former method (async and sync, `store_stream`'s bounds
  unchanged) and `storage_error()` mapping `BlobStoreError` → `StorageError` variant for variant
  (`OwnerHeld`/`SharedInstance`/`UnsafeRoot`, which the unlocked store does not produce, →
  `Internal`). `ChunkRef`, `Exists`, `StoreResult`, `RecordStoreResult`, `StorageStats`,
  `LayoutRecord`, `byte_cid_serde`, `MARKER_MAGIC`, `STORE_PROFILE` (and `BlobStoreError`)
  re-export. The shim's `new` opens the crate's unlocked store, then schedules the env-gated
  compute-payload cleanup (immediate first tick, 60 s loop) exactly as before (R2). The sibling
  test helpers `xorshift_bytes`, `spoil_inline_payload`, `nothing_quarantined` are shim-local
  copies, so `sharding.rs`, `signals.rs`, `http.rs`, `custody*.rs` tests compile unchanged (R1).
  `eprfs_tier` re-exports `BlobStoreTier`, `TIER_SUBDIR`, `PayloadTier`, `OpenMode`.
- `src/main.rs`: `BlobStore::new(config.blobs_dir())` unchanged for `/blob`; after it,
  `let _payload_tier = eprfs_tier::compose_payload_tier_from_env().await;` held for
  `async_main`'s life (R7).
- epr-cli: new `src/payload.rs` (feature `peer`), feature `blob-store` (off by default),
  `peer.rs` serve opens `Access::Owner`, `artifact.rs` put opens `Access::Shared`, both inside the
  runtime they keep (R7).

## The owner/shared rule (R4, integrity class)

Three openers on one root:

| opener | locks | may move/remove | on a failed verify |
|---|---|---|---|
| `new` (unlocked; storage's `/blob`) | none, no lock files | yes, as before | quarantine, as before |
| `open_owner` | `owner.lock` exclusive non-blocking `flock` for life (`OwnerHeld` otherwise) + `ops.lock` | yes, under `ops.lock` exclusive, after re-verifying | re-verify, then quarantine |
| `open_shared` | `ops.lock` | no: `SharedInstance` | `HashMismatch` / `Exists::Corrupt`; nothing moved or overwritten |

Both lock files are opened once (created if absent, not truncated; no code path unlinks or
recreates them — `open_lock_file` in `store.rs`) and held by the instance. `ops.lock` is per operation: every publication (staging write → chunks →
layout → marker; `store_stream` from first chunk to marker; a record; a `put_chunk` repair; an
owner's repair of a corrupt existing payload inside `store`) holds it **shared**; every
destructive act (delete and its chunk GC, explicit quarantine, the quarantine a read/verify path
leads to, the owner's startup `tmp/` sweep) holds it **exclusive**. Because `flock` is per open
file description, the shared lock is reference-counted per process (first publication takes it,
last releases it) and the in-process `RwLock` keeps a destructive act from ever converting a lock
this process holds shared.

Audit of every destructive path astra listed (`e4-gate.receipt.md` item 4):

| path | owner / unlocked | shared |
|---|---|---|
| startup staging cleanup | unlocked: as before; owner: under exclusive `ops.lock` | not run (`Inner::open`) |
| startup/manual `scrub` | per-defect condemn (exclusive, re-verify) | refused `SharedInstance` (not spawned) |
| `get`, `get_range` whole mismatch | condemn payload + bad chunks | `HashMismatch`, nothing moved |
| `get_chunk` | condemn the chunk (re-read under the lock) | `HashMismatch`, nothing moved |
| `size` → `refuse` → `settle` | via `exists` | via `exists` → `Corrupt` → `HashMismatch` |
| `exists` (incl. chunk verification) | verdict computed with no side effects, then condemn | `Exists::Corrupt` |
| `list_units`, `holds_verified` | through `get_chunk` / `exists` | the same, nothing moved |
| `store`/`store_stream` existing-object check | reported; the publication steps out and repairs under the exclusive lock (review fix 1) | `HashMismatch`, nothing overwritten (an unreadable held copy is refused too) |
| `put_chunk` repair | reported; repaired under the exclusive lock, then put again (review fix 1) | `HashMismatch` |
| `delete` + chunk GC | exclusive | `SharedInstance` |
| `quarantine` | exclusive (owner) / none (unlocked, as before) | `SharedInstance` |

`Exists::Corrupt` was added for the shared case; owner and unlocked instances do not return it
(only the `shared()` branches in `store.rs` construct it).
**No elohim-storage match site needed editing**: every caller compares with `==` or `matches!`.
The variant's only consumer edits are the tier (`has_blob`, `read_error`) and one test match.

Limitation, documented in the module docs: with no owner running, nobody performs destructive
maintenance — a corrupt object stays in place (refused on every read) and unreferenced chunks
stay on disk until an owner opens. An owner's background startup scrub holds the instance (and
`owner.lock`) until it finishes.

## The format marker (R5)

`root/STORE` holds `blob-store-v1` or `directory-v1`. `payload::detect`:

- marked → that format if this binary links it, else `PayloadStoreError::Format { found,
  supported }` (no backend opened, nothing created);
- unmarked with flat `<CID>` files or DirectoryStorage `.tmp*` residue (and no blob-store names)
  → `DirectoryV1`, opened through `eprfs_peer::open_payload_store` exactly as before, **not
  stamped** (enforced: `epr-cli/tests/payload.rs::an_unmarked_directory_opens_as_directory_v1_unchanged`) (shem's directory, holding the F2r binary, is this case);
- unmarked blob-store layout (`blobs chunks index records quarantine tmp owner.lock ops.lock`),
  a mix of both, or only unrecognisable entries → `PayloadStoreError::Layout`;
- absent or empty → new: `select_format` stages the marker under a reserved `.STORE-<pid>-<ns>-<tid>`
  name in the root, fsyncs it and hard-links it to `STORE` (fails if present); the loser reads
  the winner's marker. Detection ignores `.STORE-*` entries. New stores are `blob-store-v1` only
  with feature `blob-store`, else `directory-v1`.

The BlobStore format refuses a symlinked root or one open to group/other (`UnsafeRoot`), as
`DirectoryStorage::open` does; a root it creates is 0700.

## Departures from the rulings, and why

1. **Steps 1–2 in one commit** (above).
2. **`open_payload_store(root, access)` is async and takes an `Access`** (the design's sketch was
   `open_payload_store(root)`, sync): R4 needs owner vs shared, R7 needs the open inside the kept
   runtime.
3. **The marker contract lives in two places, tested equal.** Storage reads the marker through
   `elohim_blob_store::marker` (it cannot depend on epr-cli); epr-cli's `PayloadFormat` names
   both formats (it links the crate only under `blob-store`). The parity test
   `payload::tests::the_blob_store_marker_is_the_one_storage_composes_from` (feature on) pins
   `STORE`, `blob-store-v1` and the lock-file names.
4. **Iroh never-copies test dropped**, per the amendment (vacuous without iroh↔tier integration).
5. **`epr peer status` does not print the format** (its `StatusOutcome` JSON is a wire shape a
   schema may pin); `epr peer serve` logs `payload store <path> (<format>)` instead. Follow-up.
6. **eprfs-peer's `open_payload_store` stays** as the DirectoryV1 backend; only epr-cli's callers
   moved to `payload::open_payload_store`.
7. **(Superseded by review fix 1.)** An owner's repair inside `store` first ran under the
   publication's *shared* lock, as R4 classifies `put_chunk` repair; taking the
   exclusive lock there would need a lock upgrade inside this process's own publication. The
   race it leaves is with another publication of the same CID; both write identical verified
   bytes by rename, and any defect left is refused and condemned by verify-on-read.
8. **A shared `store` also refuses an *unreadable* held copy** (not only a corrupt one): it
   cannot tell what it would overwrite.
9. **Read-path condemnation now re-verifies before it moves** (owner under the exclusive lock;
   the unlocked store under the stripe only, so `/blob` keeps today's lock behaviour), and it
   moves every chunk of the payload that fails its own CID, not only the first one `exists`
   meets. Strictly more careful; every moved test still passes unchanged.
10. **chrono dropped from the store** (quarantine directory names use `SystemTime` micros, same
    digits). `fanout_path` is public (the shim's test helper needs it).
11. **The `dedicated()` tier store no longer inherits the compute-payload cleanup** it used to get
    through `BlobStore::new` when `ELOHIM_COMPUTE_LOCAL_API=1` (sweeping
    `<root>/eprfs-tier/compute-payloads`, which nothing writes). The cleanup now runs only beside
    the `/blob` store, as R2 asks.
12. **Files outside the design's write-set:** `elohim/elohim-storage/Dockerfile` (the new path
    dependency needs re-rooting or the edge image fails at resolve), `justfile` and
    `elohim/epr/build-manifest.json` (R8's gate home), and the bridges/orchestrator manifests (the
    closure test requires their globs). `genesis/orchestrator/build-inputs-closure.test.mjs`
    needed no edit.
13. **Licence:** the crate keeps the moved code's `AGPL-3.0`. epr-cli is `Apache-2.0` and links it
    under feature `blob-store`. I did not relicense; this is the orchestrator's (or steward's) call.
14. **No interim R4 estimate was sent:** this run has no messaging tool besides the final
    handback. R4 was implemented in full.

## Tests

Every cargo run below was judged by its own `EXIT=` line (no pipes), `RUSTUP_TOOLCHAIN=1.96.1`,
`CARGO_BUILD_JOBS=1`, one cargo at a time, slots under `/projects/.cargo-target-pool/family/as-e4/`.

**crates/elohim-blob-store** (`RUSTFLAGS=""`)

- `cargo test --locked --all-features`: EXIT=0 — 42 unit (the 34 moved store tests, 6 moved
  tier tests, 2 marker tests) + 6 in `tests/multi_process.rs`.
- `cargo test --locked` (default features): EXIT=0 — 36 unit + 6 multi-process.
- `cargo clippy --locked --all-features --all-targets -- -D warnings`: EXIT=0. `cargo fmt --check`: EXIT=0.
- `tests/multi_process.rs` (children are this test binary re-executed with
  `ELOHIM_BLOB_STORE_CHILD_ROLE`; real processes throughout):
  - `two_instances_storing_one_chunk_converge_to_one_verified_copy` — 3 child processes per
    payload (a chunked one and a one-chunk inline one) plus two in-process shared instances; one
    marker, one inline file, one file per distinct chunk, empty `tmp/` and `quarantine/`.
  - `a_shared_instance_never_moves_a_file` — get/get_range/get_chunk `HashMismatch`, exists
    `Corrupt`, store refuses, delete/quarantine/scrub `SharedInstance`, the whole tree
    byte-identical after; the owner then quarantines and heals.
  - `a_second_owner_is_refused_while_the_first_holds_the_lock` — owner in a child process;
    `OwnerHeld`, shared admitted, ownership passes on exit, exclusive in-process too.
  - `owner_gc_never_collects_a_chunk_a_shared_writer_is_publishing` — a child streams a payload
    sharing the owner's prefix chunks, pauses before its marker; the owner's delete must return
    only after the child resumes, and the child's payload reads whole.
  - `an_owner_opening_never_sweeps_a_shared_writers_staging` — owners open (sweep) in child
    processes over and over while a shared child writes ten multi-chunk payloads.
  - **Mutation check:** with the two cross-process `flock` calls replaced by no-ops,
    `cargo test --test multi_process` → MUTANT_EXIT=101, exactly the GC and sweep race tests
    failing; restored, EXIT=0. (A first version of the GC test passed under the mutant — its window
    was too narrow and the owner's re-store masked the loss — and was replaced by the paused
    publication; its seed generator also collided `seed | 1`, fixed.)

**elohim-storage** (`RUSTFLAGS='--cfg getrandom_backend="custom"'`)

- `cargo check --all-targets`: EXIT=0 (no caller file changed; no match site needed `Exists::Corrupt`).
- `cargo check --features p2p-iroh --all-targets`: EXIT=0 (the `p2p_iroh` BlobStore callers).
- `cargo clippy --all-targets -- -D warnings`: EXIT=0. `cargo fmt --check`: EXIT=0.
- `cargo test --lib`: EXIT=101 — 4961 passed, 2 failed, 3 ignored. The two failures are
  `services::release_adoption::verify::tests::{release_manifest_mirror_agrees_with_the_rakia_schema,
  the_app_bundle_fixture_agrees_with_the_schema_once_the_pin_carries_the_class}`: "T1 schema
  unreadable at ../rakia/schemas/v1/release-manifest.schema.json" — this worktree has no `elohim/rakia`
  submodule checkout. Environmental, untouched by E4. The new shim and tier tests passed
  (`blob_store::tests::every_store_error_maps_to_its_storage_error`,
  `blob_store::tests::the_shim_opens_the_unlocked_store_with_no_lock_files`,
  `eprfs_tier::tests::a_blob_store_marker_composes_a_shared_tier_over_exactly_that_root`,
  `eprfs_tier::tests::another_marker_or_none_composes_nothing_and_creates_nothing`), as did every
  sibling test using the shim-local helpers.
- `cargo test --test blob`: EXIT=0 — 41 passed, 1 ignored.

**eprfs workspace** (`RUSTFLAGS=""`)

- `cargo test -p elohim-epr-cli --features blob-store --test payload`: EXIT=0 — all five:
  `an_unmarked_directory_opens_as_directory_v1_unchanged`,
  `a_payload_store_of_another_format_is_refused_not_duplicated`,
  `concurrent_first_opens_agree_on_one_format`,
  `a_marker_without_its_layout_opens_and_builds_the_layout`,
  `the_same_cid_is_never_held_twice_on_one_device_by_the_two_paths` (9 MiB and 7 MiB payloads,
  one put first by each path; reads verify through both; per payload one marker + one layout
  record, each chunk once under `chunks/`, chunk bytes on disk = payload bytes, no `eprfs-tier`
  store and no flat object anywhere under the device directory).
- `cargo test --no-fail-fast -p elohim-epr-cli` (default features; payload tests run without the
  blob-store test): EXIT=0 across all 75 test binaries. (A first run, EXIT=101, failed only
  `tests/session_envelope.rs`, which needs device-runtime's `bin-launch` in the target directory; a
  package-scoped run does not build it. Built with `cargo build -p elohim-device-runtime --bins`,
  EXIT=0, then rerun.)
- `cargo test --no-fail-fast -p elohim-epr-cli --features blob-store`: EXIT=101, the only failing
  binary `tests/artifact.rs::put_records_one_produce_that_both_stays_local_custody_and_crosses`
  (the test read the payload store through the raw directory opener). Fixed to read through
  `payload::open_payload_store`; `--test artifact` then EXIT=0 with the feature (11 passed) and
  EXIT=0 without it (11 passed).
- `cargo test -p eprfs-storage`: EXIT=0.
- `cargo clippy -p elohim-epr-cli -p eprfs-storage --all-targets -- -D warnings`: EXIT=0;
  `cargo clippy -p elohim-epr-cli --features blob-store --all-targets -- -D warnings`: EXIT=0;
  `cargo fmt --check`: EXIT=0.

**Other**

- `node --test genesis/orchestrator/build-inputs-closure.test.mjs`: EXIT=0 (pass 52, fail 0).
- `cargo metadata --locked --offline` on each touched workspace: EXIT=0 ×4.
- Not run, per the brief: `just gate`, `just mesh *`. Not exercised: a Docker build of the
  storage image.

## Versions and publishing (R8)

- `eprfs-storage` 0.3.6 → 0.3.7 (carries the E2 fix `63ae4070d`). Nothing else published
  changed: epr-cli and eprfs-peer are `publish = false`; `publish_eprfs.py` is not extended.
- New crate `elohim-blob-store` 0.1.0, `publish = ["elohim"]`, not in the publisher's list
  (as `elohim-address` is not).
- `publish_eprfs.py --precheck`:

```
EXIT=0
verified existing: elohim-seam-contracts 0.1.2
verified existing: eprfs-core 0.3.4
verified existing: eprfs-host 0.3.3
absent: eprfs-storage 0.3.7
verified existing: eprfs-meta 0.3.5
verified existing: eprfs-local 0.3.5
eprfs precheck complete
```

No registry publish is a precondition: nothing in-tree consumes `elohim-blob-store` from the
registry.

- Lockfiles: `crates/elohim-blob-store/Cargo.lock` (new, `generate-lockfile --offline`),
  `elohim/elohim-storage/Cargo.lock` (adds the crate), `elohim/eprfs/Cargo.lock` (adds the crate;
  eprfs-storage 0.3.7), `elohim/Cargo.lock` (eprfs-storage 0.3.7 via `elohim/ark/cli`).

## Gates and build plumbing

- New gate project `elohim-blob-store` in `elohim/epr/build-manifest.json` (beside
  `elohim-address`), recipe `_gate-elohim-blob-store` in the root `justfile`: test (default
  features), test `--all-features`, clippy `--all-features --all-targets -D warnings`, fmt. Its
  consumers' gates do not run a dependency's tests, so without it the crate's tests would have
  no local gate.
- Watch globs `crates/elohim-blob-store/**` added where the closure test required them: eprfs
  `rust-build-test`, the holochain manifest's `epr-storage` project, `bridges/cargo`,
  `bridges/jenkins`, `genesis/orchestrator` `lint-jenkinsfiles` (storage's `cargo-build-storage`
  already watches `crates/**`). `node --test genesis/orchestrator/build-inputs-closure.test.mjs`:
  pass 52, fail 0, EXIT=0.
- `elohim/elohim-storage/Dockerfile`: the crate rides the wholesale `COPY crates`; one sed re-roots
  storage's dep path, one RUN re-roots the crate's `../../elohim/epr` and
  `../../elohim/eprfs/eprfs-{core,serve}` paths and drops `registry = "elohim"`, failing by name
  if an in-repo path survives (simulated locally on a copy: OK). Not exercised by a Docker build.

## Open frontiers

- **E4b** (after the F2r receipt): flip epr-cli's `blob-store` default; export
  `EPR_PAYLOAD_STORE` into the storage daemon from `hc-mesh.sh` (both the start block,
  `hc-mesh.sh:4779-4815`, and the restart overlay, `:4136-4159`); migrate unmarked directories.
  Migration must also decide the one ambiguity this slice leaves: an existing *empty* directory
  is treated as new, so with the feature on an older, marker-unaware epr could still write flat
  objects into a directory a newer one marked `blob-store-v1`.
- **Iroh index without a copy** (`an_iroh_index_references_a_tier_payload_and_never_copies_it`):
  tied to the R1.4 / FsStore ruling; vacuous until iroh reads the tier.
- **`epr peer status` prints the format** (deviation 5).
- **CI home for the crate's tests.** The new gate project runs them locally; no Jenkinsfile runs
  them (Jenkinsfiles are KEEP for this sprint). Candidate: the epr pipeline, beside
  `elohim-address`.
- **`store.rs` is 3179 lines**, over the 3000-line soft ceiling (about 1060 are tests); moving the
  test module to a sibling file is a bounded follow-up.
- **No owner, no maintenance** (documented): corrupt objects and unreferenced chunks wait for an
  owner. `flock` grants no writer preference, so a continuous stream of shared publications can
  delay an owner's exclusive act. An owner's startup scrub keeps `owner.lock` until it finishes.
- **Licence of the moved code** (deviation 13).
- **Docker build of the storage image** not exercised (the sed was simulated on a copy).
- **`concerns:` for the reviewer:** R4 is integrity class; review `Coordination`
  (`crates/elohim-blob-store/src/store.rs`) and the two race tests first.

## Review fixes (astra's code review, `genesis/local-dev/as-e4/e4-review.receipt.md`)

Each finding was fixed in its own commit, after a test that fails without the fix
(`BEFORE_FIX_EXIT`, the test alone against the unfixed code) and passes with it (`EXIT`).

### 1. BLOCKER — repair under the shared lock (`d95200630`)

A publication no longer moves anything. `held()` (was `held_whole_or_repaired`) and `put_chunk`
only *report* damage (`Held::Damaged`, `ChunkPut::Damaged`); the publication then drops its stripe
and its share of `ops.lock`, takes the exclusive lock (`repair_payload` / `repair_chunk`, which
re-verify under it and quarantine only what still fails), and starts again (`store`: up to four
attempts). Nothing is upgraded while held. `store_stream` cannot replay its reader, so it repairs
in place with the chunk still in memory and resumes; if it paused, it checks that every chunk it
wrote earlier is still on disk before writing the marker, and otherwise fails with a retryable
error rather than publish a marker naming a missing chunk. The unlocked `/blob` store now also
repairs under its in-process write lock (it used to repair under the read lock).

- `store::tests::a_stale_repair_never_quarantines_a_chunk_another_publication_healed` — two
  payloads share a first chunk whose file is corrupt; publication B reads it and is held (a
  test gate); publication A reads it too, repairs it and completes; B is released with its next
  write failing (ENOSPC). A must still read whole and verified, and the healed chunk must still be
  in place. BEFORE_FIX_EXIT=101 (A's payload `Quarantined`); after: lib EXIT=0 (43 passed), multi-process EXIT=0, clippy EXIT=0.

### 2. MAJOR — `store_record` replaced records on a shared instance (`0a3613718`)

`store_record` now tells verified / absent / corrupt / unreadable apart. A shared instance refuses
a corrupt held record (`HashMismatch`) and an unreadable one (new `BlobStoreError::HeldCopyUnreadable`,
which a shared `store` now also returns for an unreadable held payload) and writes nothing; owners
and the unlocked store rewrite as before. elohim-storage's shim maps the new variant to `Internal`.

- `multi_process::a_shared_instance_never_replaces_a_record` (core API) and
  `multi_process::a_shared_tier_never_replaces_a_record` (`BlobStoreTier::put_blob_verified`,
  feature `tier`), each for a corrupt and an unreadable record. BEFORE_FIX_EXIT=101 (both: the shared store returned `Ok` and replaced the record); after: multi_process EXIT=0 (8 passed).

### 3. MAJOR — the same chunk held twice by the CLI custody path and a whole-payload put (`7b93f536b`)

**Which representation wins:** whichever arrives first. Nothing moves when the second arrives.
- A standalone object (≤ one chunk) whose CID is already a verified chunk file is published as a
  one-chunk layout record + marker naming that chunk file: `blobs/<cid>` is a 52-byte marker,
  the bytes stay in `chunks/<cid>`. (`LayoutRecord::decode_for` admits a one-chunk record only
  when that chunk is the payload itself; its `layout_cid` is the one `StoreResult` always
  reported for an inline payload.)
- A chunked payload whose chunk CID is already held as a verified inline object reuses
  `blobs/<cid>`: `put_chunk` writes no chunk file.
- Readers find a unit in either place (`read_unit`, `unit_len`: `chunks/<cid>`, else an inline
  `blobs/<cid>`), so `get`, `exists`, `present`, `size`, `get_chunk`, `list_units` and `scrub`
  work over both. Existing stores, storage's `/blob` store included, are read exactly as before:
  nothing is migrated; the fallback only adds a second place to look.

**How GC counts references across them:** a chunk is named by every layout record under
`index/` (a one-chunk alias record names its own chunk). Deleting a chunked payload removes its
`chunks/` files that no remaining layout record names, as before; a chunk whose one copy is an
inline object is untouched (that object is a payload of its own). Deleting an inline payload that
some layout record still names, with no verified chunk file of it, *moves* its file to
`chunks/<cid>` (one rename, under the exclusive lock; a corrupt chunk file in the way is
quarantined first) instead of removing it. Deleting an alias removes its marker and record, then
collects the chunk file if no other record names it.

- `one_copy::a_chunk_stored_standalone_after_its_payload_is_not_held_twice`,
  `one_copy::a_payload_stored_after_its_chunk_reuses_the_standalone_copy` (both on the unlocked
  store and an owner; reads, size, presence, listing, idempotence, then deletion in both orders),
  `one_copy::a_standalone_copy_a_payload_reuses_is_verified_when_the_payload_is_read`. BEFORE_FIX_EXIT=101 (all three; two physical copies); after: EXIT=0.
- `epr-cli/tests/payload.rs::the_same_cid_is_never_held_twice_on_one_device_by_the_two_paths`,
  rewritten: the real custody path (`payload::open_payload_store` as `epr artifact put` opens it,
  then `put_local_blob_with`, which stores each chunk as its own object and a manifest record)
  against storage's marker-gated tier doing a whole-payload put, in both orders, 9 MiB; every
  chunk and the whole read verified through both; per chunk CID exactly one file carrying its
  bytes across `blobs/` and `chunks/`; no file carrying the whole; byte-carrying files sum to the
  payload length. BEFORE_FIX_EXIT=101 (`cli_first=true`: a chunk in both `blobs/` and `chunks/`); after: EXIT=0.

### 4. MAJOR — concurrent first opens could refuse the winner (`116edee3a`)

`detect` reads the marker again after its directory scan, before classifying an unmarked layout
or refusing one: the marker is authoritative, and a first open racing this one may have published
it (and built its layout) in between.

- `payload::tests::an_open_that_loses_the_first_open_race_opens_the_winners_format` (through
  `open_payload_store`'s own path, `open_with`, with a hook that runs the winner's marker
  selection and layout between the loser's marker read and its scan). BEFORE_FIX_EXIT=101 ("holds a blob-store layout … but no STORE marker"); after: EXIT=0 with and without `blob-store`.
- `epr-cli/tests/payload.rs::racing_first_opens_through_the_opener_all_open_one_format` (six
  threads per round, 50 rounds, staggered, public `open_payload_store`). It passed before the fix too (the unsynchronised window is microseconds wide), so it is supplementary; the hooked test above is the one that reproduces the bug.

### Verification after the fixes

Run after all four commits, one cargo at a time, judged by `EXIT=` alone:

- `crates/elohim-blob-store`: `cargo test --locked --all-features` EXIT=0 (43 unit, 8
  multi-process, 3 one-copy); `cargo test --locked` (default) EXIT=0 three runs in a row;
  `cargo clippy --locked --all-targets -D warnings` EXIT=0 both with and without `--all-features`;
  `cargo fmt` clean.
- elohim-storage: `cargo check --lib` EXIT=0; `cargo test --lib` EXIT=0 (4963 passed, 3 ignored —
  the two rakia-schema tests that failed earlier pass now); `cargo test --test blob` EXIT=0
  (41 passed, 1 ignored); `cargo clippy --all-targets -D warnings` EXIT=0; the shim is
  rustfmt-clean.
- eprfs: `cargo fmt --check` EXIT=0; `cargo clippy -p elohim-epr-cli --all-targets -D warnings`
  EXIT=0 with and without `--features blob-store`; `cargo test --no-fail-fast -p elohim-epr-cli`
  EXIT=0 with and without `--features blob-store` (after `cargo build -p elohim-device-runtime
  --bins`, EXIT=0).
- No `allow`/`expect` attribute was added.

Also in `7b93f536b`: `a_second_owner_is_refused_while_the_first_holds_the_lock` retries its
post-release owner opens. It failed once under default features: a sibling test forking a child
shares the just-closed lock's file description until that child execs (std opens files
close-on-exec), a few microseconds of `OwnerHeld`. In a long-lived `epr peer serve` the same window
only delays a successor owner's open; it never admits two.

Process note: the orchestrator's gate run on this worktree held the host's one-build slot
(`cargo-disk-guard`) from about 13:35 to 16:12, so the fixes waited for it. While it ran, I held my
edits outside the worktree and reset the tree to HEAD so the gate read only committed source. (A
few uncommitted crate edits — the new error variant and the test hook — were in the tree for
about 20 minutes at the start of the `epr-storage` project.)

New frontier from finding 3: an inline object that a payload reuses stays a payload in its own
right, and deleting one costs a scan of `index/` (`referenced_chunks`), the cost a chunked delete
already paid.
