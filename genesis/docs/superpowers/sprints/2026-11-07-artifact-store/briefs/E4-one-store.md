---
id: artifact-store-brief-e4
status: active
---

# Brief E4 — one authoritative payload store per device (landing slice)

Read first: `common.md` in this directory, then `../design.md` §2 (E4) — this brief **amends** it where the
current code differs. Anchors come from the 2026-11-09 census (`genesis/local-dev/as-e4/e4-census.receipt.md`,
Codex sol, read-only, against branch `artifact-store/e4`).

## What changed since design.md was written

- `Exists::Verified` now verifies bytes (`blob_store.rs:1161-1254`); structural-only moved to `present()`.
  Design §2.3(b) is satisfied; do not touch it.
- H2's durability tests landed (`power_loss_after_marker_reopens_verified` :2302,
  `delete_racing_store_never_serves_partial` :2346, `delete_racing_read_returns_whole_or_typed_absent` :2393,
  `quarantine_leaves_unrelated_objects_readable` :2432, `enospc_returns_error_and_no_file` :2016). The
  owner/shared split and a cross-process test did **not** land; E4 adds them.
- `BlobStore` has 59 caller files inside elohim-storage (36 src, 23 tests). It is coupled to
  `crate::compute_payload_store::cleanup` (`blob_store.rs:327-335`).
- `BlobStoreTier` (`eprfs_tier/blob_store_tier.rs`) is constructed only in its own tests; `dedicated()` opens
  `<storage_root>/eprfs-tier`, a second store.
- epr-cli opens its payload store through `eprfs-peer/src/blob_mount.rs:983-1010` (`open_payload_store`,
  an unmarked `OffloadedDirectory<DirectoryStorage>`), from `peer.rs:708-727,756-764` and
  `artifact.rs:1187-1195`.
- No umbrella workspace covers `crates/`; `crates/elohim-address` is its own single-crate workspace and is
  depended on by path + version + registry (`elohim/epr/Cargo.toml:18-23`) because published eprfs crates
  link it. `publish_eprfs.py --precheck` (`justfile:688-697`, `scripts/ci/publish_eprfs.py:373-401`) refuses a
  changed published crate without a version bump.

## Rulings (orchestrator, 2026-11-09)

1. **Relocate, shim, change no caller.** Move the store into `crates/elohim-blob-store` (own workspace and
   lockfile, like `elohim-address`) with its own `BlobStoreError`. In elohim-storage, `src/blob_store.rs`
   becomes a thin shim that keeps every current public name and signature (methods return `StorageError`,
   mapped from `BlobStoreError` variant-for-variant: `Internal`, `BlobNotFound`, `HashMismatch`,
   `InvalidContentAddress`, `Io`). Prefer a newtype wrapper with explicit delegating methods over changing
   59 caller files. If you find the From-conversion route changes fewer than ~5 call sites, you may take it
   instead; say which and why in the report. `ChunkRef`, `Exists`, `StoreResult`, `LayoutRecord`,
   `byte_cid_serde`, `MARKER_MAGIC`, `STORE_PROFILE` re-export unchanged.
2. **Sever `compute_payload_store::cleanup`.** Whatever `BlobStore::new` calls there becomes either a
   caller-side step in elohim-storage (the shim's `new` calls it after opening) or a generic hook; the new
   crate must not know elohim-storage exists.
3. **The tier adapter moves too.** `BlobStoreTier` moves into the new crate behind a feature `tier`
   (it needs `eprfs-core`'s `EprfsStorage`), gains `BlobStoreTier::at(root: &Path, mode: OpenMode)`.
   elohim-storage keeps `eprfs_tier::BlobStoreTier` as a re-export; `dedicated()` stays for its tests.
4. **Owner/shared split (integrity class).** `BlobStore::open_owner(root)` takes an exclusive advisory
   lock on `root/owner.lock` for the life of the instance (fail with a typed `OwnerHeld` if another
   process holds it — non-blocking); `BlobStore::open_shared(root)` refuses `delete`, `quarantine`,
   `scrub`'s moving step and chunk GC with a typed `SharedInstance` error, and on a failed verify returns
   `Corrupt`/`HashMismatch` and moves nothing. `BlobStore::new` keeps today's behaviour (owner semantics
   without the lock file) so elohim-storage's `/blob` store is unchanged. Writes stay tmp+fsync+rename and
   idempotent on a verified existing payload, so two processes storing one CID converge.
5. **epr-cli `src/payload.rs` with a format marker.** `PayloadFormat { BlobStoreV1, DirectoryV1 }`,
   `root/STORE` holds `blob-store-v1` or `directory-v1`. An existing directory **without** a marker is
   `DirectoryV1` (every device's payload dir today, including shem's, which holds the F2r 468 MB binary
   ethosengine is fetching now — it must keep working byte-for-byte). First creation writes `BlobStoreV1`
   only when feature `blob-store` is on, else `DirectoryV1`. A marker naming a format this binary does not
   link → `PayloadStoreError::Format { found, supported }`, refused, with no second store created beside it (enforced: `epr-cli/tests/payload.rs::a_payload_store_of_another_format_is_refused_not_duplicated`).
   `peer.rs` and `artifact.rs` call `payload::open_payload_store`; `epr peer serve` opens owner, the
   daemon-down artifact put opens shared. Print the format in `epr peer status` if that is a small change;
   otherwise name it as a follow-up.
6. **Feature `blob-store` defaults OFF in this slice.** Flipping the default, exporting
   `EPR_PAYLOAD_STORE` from `hc-mesh.sh`, and migrating unmarked directories are **E4b**, after the F2r
   receipt — not this slice. Do not edit `app/elohim-app/scripts/hc-mesh.sh` (ethosengine owns that file
   tonight for B3).
7. **elohim-storage composition.** `main.rs:1496-1497` keeps `BlobStore::new(config.blobs_dir())` for `/blob`.
   Add: if `EPR_PAYLOAD_STORE` is set and its marker is `blob-store-v1`, compose
   `BlobStoreTier::at(path, Shared)` as the storage process's tier (log one line naming path and format);
   if it is set and the marker is anything else or absent, log one warning and compose nothing (never
   create a store there). Unset → today's behaviour exactly. Nothing reads this tier yet beyond what
   `eprfs_tier/mod.rs` already exports; do not wire routes.
8. **Versions and publishing.** The new crate must be publishable the way `elohim-address` is (read how
   `publish_eprfs.py` and the eprfs workspace treat `elohim-address`: path + version + registry) and the
   eprfs build manifest (`elohim/eprfs/build-manifest.json:30-44`) gains its watch glob. Bump: `eprfs-storage`
   0.3.6 → 0.3.7 (already needed by the cherry-picked E2 fix `63ae4070d`), and whatever else the precheck
   names (epr-cli/eprfs-peer inherit 0.3.3 from the eprfs workspace). Regenerate the affected lockfiles
   (`elohim/eprfs/Cargo.lock`, `elohim/elohim-storage/Cargo.lock`, `elohim/Cargo.lock`) with `--offline`
   where possible; if a registry publish of the new crate is a precondition the precheck enforces and you
   cannot satisfy it locally, stop and report exactly what it refuses.

## Tests (must exist and pass)

- `crates/elohim-blob-store/tests/multi_process.rs::two_instances_storing_one_chunk_converge_to_one_verified_copy`
  (two `open_shared` instances, or two processes via a helper bin/`std::process::Command` on the test binary
  — real processes preferred; say which).
- `…::a_shared_instance_never_moves_a_file` (shared delete/quarantine refused; a corrupt read moves nothing).
- `…::a_second_owner_is_refused_while_the_first_holds_the_lock`.
- `epr-cli/tests/payload.rs::the_same_cid_is_never_held_twice_on_one_device_by_the_two_paths` (feature
  `blob-store`: put via the tier adapter as storage composes it, put via epr-cli's payload store at the same
  root, assert one payload on disk).
- `…::a_payload_store_of_another_format_is_refused_not_duplicated`.
- `…::an_unmarked_directory_opens_as_directory_v1_unchanged`.
- `elohim-storage/src/p2p_iroh/blob_store.rs::an_iroh_index_references_a_tier_payload_and_never_copies_it`
  (after a tier put, `IrohBlobStore::has(hash_for(chunk_cid)) == false`; pure-iroh answers `NoBytePath`).
- Every existing blob_store test moves with the code and still passes; elohim-storage tests compile
  unchanged except imports where unavoidable.

## How to work

- Worktree `/projects/elohim/.worktrees/as-e4`, branch `artifact-store/e4`. Do not `cd` into the worktree in a
  bare command; use `(cd /projects/elohim/.worktrees/as-e4 && …)` subshells or `-C`/`--manifest-path`.
- Use `brit` for repo operations (`brit add -- <paths>`, `brit commit -F -`); `brit status --no-write`.
  Git fallback only with a stated reason (`# Brit fallback: <reason>` as the command's first line).
- Cargo: `RUSTUP_TOOLCHAIN=1.96.1`, `CARGO_BUILD_JOBS=1`, explicit `CARGO_TARGET_DIR` under
  `/projects/.cargo-target-pool/family/as-e4/<crate-slug>/dev` (one slot per workspace). Claim the cargo
  lease first: `berth claim cargo --session f1769d1c-3a6c-4679-8e49-811cb052eaf4 --class verify --ttl 3600`
  and release it when done. Echo `EXIT=$?` on its own line; do not judge cargo from piped output. RAM guard
  sheds cargo above 80% of 41 GB — one cargo at a time.
- Rust style: `genesis/docs/rust-style-guide.md`; no new `allow`/`expect` without an RS-GOV-04 reason; clippy
  `-D warnings`; `cargo fmt`.
- Commit in small steps with messages that say what changed and why; end each with
  `Covenant: bafyreicbwyor2jvmd6dvqbvh4l4m4h4lhoudxhf6n2cx7qyym7sn4fqiki (steward), bafyreiebyv5xpphc55dgbscrqpzta6fvasod3emgdzt3ztxvbmi2drhfqu (purpose)`
  and the attribution lines `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` /
  `Claude-Session: https://claude.ai/code/session_01PJB8fqTWWfW4SgFPvTmybr`.
- Do not push, do not run `just mesh *`, do not touch the peer daemon or `genesis/local-dev/artifact-store/`.
- Report: `genesis/docs/superpowers/sprints/2026-11-07-artifact-store/task-e4-report.md` — what moved, every
  ruling you departed from and why, test names with EXIT lines, the version bumps, and open frontiers.
