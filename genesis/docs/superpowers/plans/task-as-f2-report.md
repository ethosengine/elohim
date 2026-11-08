---
id: task-as-f2-report
title: the sibling fetch, producer and consumer (artifact-store F2a + F2b)
status: claimed-not-verified
landed_commit: ed29026b6
verified_by:
concerns: "The key input is the labelled interim KeyInput::Raw(git-tree closure CID) with slot basis-key:git-tree, not KeyInput::Projected: no producer of the HEAD tree as an eprfs Tree v1 closure exists on this tree, so an F2r receipt on this key does not close F2. The 454 MB receipt (F2r) and the two-device relay throughput floor are not run here."
actor: agent:as-f2@claude-opus-5-5
cites: []
---

# F2: the sibling fetch (producer F2a, consumer F2b)

Worktree `as-f2`, branch `artifact-store/f2`, based on `artifact-store/e3` (`a655b6ba3`).
Specification: the sprint's `design.md` §3 (and §1 for the serving side). Commits: `ed29026b6` (implementation and tests) and the commit adding this report.

## Which key input, and why

`KeyInput::Raw(<closure CID>)`, labelled with the slot `basis-key:git-tree` on the custody
Produce and as `basis` in `epr artifact key --json`. `project_view` exists in
`eprfs-core/src/derive.rs`, but it projects a `SpineWalk` of a Tree v1 closure held in a store, and
nothing on this tree imports the HEAD git tree into one (that is Track A's producer, F1r). The
closure is the `epr bin` source-state form (`device-runtime/src/bin_store/source.rs:271-285`): one
`path\t<git oid>\n` line per path of the descriptor's own normalized view at HEAD, addressed with
`BlobCid::compute_bytes`. Two additions over that form, each of which only adds to what moves the key: a path that
does not exist at HEAD is `path\t-`, and a path at or under a submodule is
`path\tgitlink:<submodule>@<commit>` (git cannot see inside a gitlink from the parent), with the
gitlink included in the dirty check. A closure with uncommitted or untracked changes inside the view
has no key at all. One function (`epr-cli/src/artifact.rs::closure_key`) computes it for `put` and
`fetch` alike, and `eprfs_peer::key_cid` is the one key CID (the DAG-CBOR CID of the
`DerivationKey`).

## What changed

- **`eprfs-peer/src/fetch.rs` (new; integrity class).**
  - `fetch_artifact(endpoint, store, key, output, holders, budget) -> Result<FetchedArtifact,
    FetchGap>`, in the design's order. The manifest is asked first with
    `tier_request_capped(.., manifest_cap)`; an answer past the cap is `ManifestTooLarge` before any
    chunk is asked. The served CID must be the manifest the holder's crossed record named. It is
    decoded through `read_tier_manifest` over a scratch `MemoryStorage` (so this reader's epoch and
    profile rules apply and the real store receives nothing yet). It is accepted only if
    `record.key == key && record.output == output`, where `key` is the one this device computed.
    Resume asks only chunks `has_blob` does not report `Local`. The rest race round-robin over one
    connection per holder (the accepting holder's connection is reused), at most
    `in_flight_per_holder` each, every request under `timeout(stall)`. A stalled, refused, failed
    or corrupt answer abandons that holder and re-queues the chunk; each chunk is checked again for
    CID and length before `put_blob_verified`. The manifest bytes are stored last, exactly as
    received. `total` bounds the whole fetch (`DeadlineExceeded { fetched, total }` in bytes).
  - `install_fetched(store, flow, fetched, context, dest)`: the install read is
    `stream_local_blob_with` into `.<name>.fetching` beside `dest`, renamed onto `dest` only on
    `Completed` (whole-blob BLAKE3); its one `Use` carries `draw:hit`, `copy:self`,
    `basis:sibling`, one `fetched-from:<did>` per serving device, `wasted:<n>`. Any other outcome
    removes the staging file and leaves `dest` untouched.
  - `FetchBudget`, `Holder`, `FetchedArtifact`, `FetchGap` (`kind()` and a printable reason with
    its cure), `FetchTransport`, `key_cid` (the one key CID), `put_method_record`,
    `resolve_holders` (each device's newest record; one output, else `NoRecord` / `Mismatch`), and
    the queue types `FetchRequest`, `RequestedHolder`, `FetchDone`, `fetch_queue_dir`,
    `write_queue_file`.
- **`eprfs-peer/src/daemon.rs` (the fetch-queue tick only).** `Run::fetch_queued` on each tick:
  one fetch at a time on a task of its own. It claims (removes) the oldest
  `<state>/fetch-queue/<key CID>.json`, checks the request's key hashes to the CID it is named by,
  resolves holders under this daemon's admission now (unadmitted devices and itself dropped; the
  address from `--peer` or the relay, as `fold` dials), runs `fetch_artifact` then
  `install_fetched` into its own payload store and flow store, and writes `<key CID>.done.json`.
  The task is aborted when the loop ends.
- **`eprfs-peer/src/local_blob.rs`.** `put_local_blob_with` and `stream_local_blob_with` take extra
  classification slots; with none they record the same bytes as before (the existing functions
  delegate).
- **`eprfs-peer/src/lib.rs`.** The new exports, and `pub use bytes::Bytes`.
- **`epr-cli/src/artifact.rs` (new; integrity class: key computation).** `epr artifact describe |
  key | put | fetch`. Exit 3 is a typed refusal (`unrecorded: <reason>`, `fetch gap: <kind>:
  <reason>`).
  - `describe` builds the `mesh-build` descriptor: tools from `rustc -vV` (release, commit-hash,
    host), `cargo -V` and `RUSTUP_TOOLCHAIN`; flags `--bin=`, `--features=` (storage),
    `--profile=dev`, `--target=<host>`; view = the build step's `inputs.sources` (each glob cut to
    its directory prefix), the crate's `Cargo.toml`, `Cargo.lock`, `.cargo/config.toml` and
    toolchain file when present, and every path package `cargo metadata` resolves for the
    build; non-path inputs = `RUSTFLAGS` and every `mesh_build_env` line except
    `CARGO_BUILD_JOBS` / `RUST_TEST_THREADS`; `deterministic: false`.
  - `key` prints the key (method, input, key CID, basis, head, dirty paths).
  - `put` re-computes the closure and refuses unless it is clean, equal to `--input` and at the
    same HEAD; reads `brit-build-ref signer --json` for `computed_by`; runs `brit-build-ref build
    put --step mesh-build:<p> --manifest <method CID> --output <bafkr4i…> --inputs-hash <key CID>
    --commit <sha> --success true --duration-ms <n>`; stores the descriptor record; writes one
    custody Produce through `put_local_blob_with` into the flow store the daemon serves (its head
    table's store place, else the checkout) with `crossing: ("artifact-produce@1","produce")` under
    `--cross`, and slots `basis-key:git-tree`, `env:project|method|input|key|output|bytes|commit|
    agentId`. Witness: `Attestation { epr: "brit-build-ref:mesh-build:<p>@<sha>" }`.
  - `fetch` computes the key (a dirty closure is `no-key`), refuses `DaemonDown` with its cure
    when `ServeLock::held` is not `Some(true)`, resolves holders with `resolve_holders`, queues the
    request, and waits up to `total + stall + 30 s` for the daemon's answer.
- **`epr-cli/src/peer.rs` (the `artifacts` verb and the read the fetch shares).**
  `epr peer artifacts --key <cid> [--json]`; `sibling_artifacts` applies `verdicts`' filter
  (`via:` an admitted device other than this one) and also requires a `custody-blob` Produce with
  `measure:artifact-produce@1`, `reason:produce`, the key, and resource = subject = `env:output`,
  and exactly one `manifest:` slot. Rows: `{device, output, manifest, bytes, commit, agentId?, at,
  headSeq?}`. `served_root` and `payload_store_dir` are shared with `artifact.rs`.
- **`epr-cli/src/{main.rs, lib.rs}`.** `epr artifact` dispatch, `COMMANDS`, usage; the module.
- **`app/elohim-app/scripts/hc-mesh.sh`.** `mesh_fetch <method> <dest>`. `_mesh_build_one` takes
  the step (`elohim/holochain/build-manifest.json#cargo-build-storage|doorway`); before the cargo
  build it describes the method and computes the key input, then tries `mesh_fetch` unless
  `MESH_BUILD_FETCH=0` (a gap prints its reason and the build proceeds). After `install -D` + `cmp`
  it runs `epr artifact put … --cross`; any failure prints `[mesh build] unrecorded: <reason>` and
  writes `<slot>/mesh-bin/.unrecorded` (removed on a recorded or fetched build). The beacon is
  neither recorded nor fetched. The dry run prints the fetch and put commands.
- **`justfile`.** The mesh build recipe's comment documents the fetch and `MESH_BUILD_FETCH=0`.
- **`genesis/agentic/pool-policy.json`.** `berth.channel.crosses` gains
  `"artifact-produce@1": ["produce"]`, and its comment says what crossing it does and does not
  confer.

## Tests

Focused runs, `RUSTFLAGS=""`, toolchain 1.96.1, worktree target slot
(`/projects/.cargo-target-pool/family/as-f2/elohim__eprfs/dev`):

`eprfs-peer/tests/fetch.rs` (12/12 ok):
- `a_sibling_fetches_the_closure_from_one_holder_and_records_use_against_the_holders_produce`: ok
- `an_interrupted_fetch_resumes_only_missing_chunks_from_the_remaining_holder`: ok
- `a_trickling_holder_is_abandoned_on_the_total_deadline_and_the_chunk_re_raced`: ok
- `an_oversized_manifest_is_refused_before_any_chunk_moves`: ok
- `two_holders_with_different_outputs_for_one_key_is_a_mismatch_not_a_fetch`: ok
- `a_manifest_keyed_for_another_closure_is_rejected`: ok
- `a_chunk_that_fails_its_cid_is_metered_as_wasted_and_re_fetched`: ok
- `pure_iroh_mode_reports_a_typed_transport_gap`: ok
- `the_fetch_draws_one_artifact_over_one_connection`: ok (a real E3 holder records exactly one
  `draw:completed` with `chunks == of`; a scripted holder counts one connection, one manifest
  ask, one ask per distinct chunk)
- `a_manifest_whose_record_carries_ext_keeps_its_cid_through_the_fetch` (F-E2-1): ok
- `a_held_manifest_with_a_missing_chunk_records_an_unavailable_use` (F-E2-2): ok
- `a_queued_fetch_is_run_by_the_devices_daemon_and_answered` (extra: two real daemons; the
  request names an unadmitted holder too, which is dropped): ok

`epr-cli/tests/artifact.rs` (3/3 ok):
- `put_records_one_produce_that_both_stays_local_custody_and_crosses`: ok (the repository's own
  `pool-policy.json` admits the event; E3's `BlobMount` indexes it as this device's custody; the
  stub brit saw `--inputs-hash <bafyrei… key CID>`; a moved closure and a dirty closure are
  `unrecorded` and record nothing)
- `peer_artifacts_lists_only_admitted_siblings_records_for_the_key`: ok
- `fetch_without_a_daemon_is_daemon_down_with_a_cure`: ok

`cargo clippy -p eprfs-peer -p elohim-epr-cli --all-targets -- -D warnings`: EXIT=0 (the first run
found four redundant closures and one large `Err` in `artifact.rs`; fixed before the commit).
`hc-mesh.sh build storage` with `MESH_BUILD_DRY_RUN=1`: EXIT=0, prints the fetch and put
commands. The household mesh and the 454 MB receipt were not run (F2r is the orchestrator's).

## Gates

Evidence directory: `genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/` (main
checkout). Both ran on committed state at `ed29026b6`, with `EPR_BIN` at the candidate common.md
names.

- `just gate eprfs`: `gate-F2-eprfs.log`, **EXIT=0** (fmt, clippy `-D warnings`, the whole
  workspace's tests; `fetch` 12/12, `artifact` 3/3; 36 min, over `gate-cycle-full-ceiling@1`). The
  runner wrote no evidence ("working tree dirty under elohim/eprfs"); after the run nothing under
  `elohim/eprfs` was modified, and the worktree held the orchestrator's uncommitted brief copies,
  as in E3.
- `just gate orchestrator`: `gate-F2-orchestrator.log`, **EXIT=0** (`pool-policy.json` and
  `hc-mesh.sh` are among its inputs). No evidence recorded ("working tree dirty under
  genesis/orchestrator"), for the same reason.

## Follow-ups owned here

- **F-E2-1 (record `ext` changes the manifest CID on re-encode): recorded why it stays.** No F2
  path re-encodes a manifest. The fetcher stores the bytes it received exactly as they verified
  against their CID (`fetch.rs`, step 4), decodes only for the acceptance checks, and the holder
  serves and indexes the stored bytes. The decoded `DerivationRecord` drops `ext`, which carries no
  decision meaning (D7), and the key and output compared are outside it. Test:
  `a_manifest_whose_record_carries_ext_keeps_its_cid_through_the_fetch`. Decoding through the
  retained-record wrapper would matter to a signature verifier over the record body, which arrives
  with Track C's D7 signature; the current witness is an attestation reference.
- **F-E2-2 (a peer-level test that a held manifest records an `unavailable` Use): added.**
  `a_held_manifest_with_a_missing_chunk_records_an_unavailable_use` covers both
  `stream_local_blob` and the install read.

## Deviations from design.md §3, with reasons

1. **The key input is the interim `KeyInput::Raw`** (see above). Two lines are added to the
   `epr bin` closure form, for absent paths and paths inside a gitlink. An F2r receipt on this key
   does not close F2.
2. **`put` computes the input itself and checks it equals `--input`,** instead of trusting the
   caller; it also refuses a dirty closure or a moved HEAD. `hc-mesh.sh` computes the input
   before the build, so a closure that changed during the build is `unrecorded`.
3. **Two verbs beyond §3.a.3, `describe` and `key`,** so the descriptor and key come from one
   implementation on both devices. The view uses the build step's full `inputs.sources`
   (`steps.cargo-build-storage|doorway`) rather than the `epr-storage` gate project's subset at
   `build-manifest.json:404-425`, plus cargo's path packages; `elohim/.cargo/config.toml` does
   not exist, so the crate's own `.cargo/config.toml` is used where present. A wider view costs reuse; it does not let
   a change inside the build go unkeyed.
4. **The crossing record names the manifest by its existing `manifest:<cid>` slot, not
   `env:manifest`.** The manifest CID exists only after `put_tier_blob` cuts the chunks;
   computing it ahead would derive it twice. It adds `basis-key:git-tree` and `env:input`.
5. **`FetchGap` has one more variant, `LocalStore { reason }`,** for this device's own store or
   flow store failing (typed rather than mislabelled as a holder's fault). **`FetchBudget` has a
   `transport` field** (`tier | iroh-blobs`) carrying §3.b.5's mode. **The result type is
   `FetchedArtifact`,** because `fold` already exports `Fetched`. **The install is a separate
   function,** `install_fetched`; the daemon calls it right after `fetch_artifact`, so the order is
   §3.b.3's.
6. **Trickling.** A request past `stall` abandons its holder and re-races the chunk; `total` ends
   the whole fetch as `DeadlineExceeded { fetched, total }` (bytes held here of the artifact's
   length). There is no throughput projection. The trickle test covers both: a holder slower
   than `stall` whose chunks finish from the other holder, and a lone holder within `stall` that
   runs out `total`.
7. **Relay.** A relay-only holder that was reached but stalls or loses its streams is
   `TransportCannotDeliver { "relay" }` when it was the last holder; an unreachable one is
   `NoRoute`. The 4 MiB per 10 s floor is the server's own `FRAME_DEADLINE`, not measured here.
8. **Wasted bytes of an answer that fails its address are metered at the chunk's declared length.**
   `tier_request`'s `check_answer` discards the bytes before the fetcher sees them, and `tier.rs`
   is outside this write-set. A holder that serves one corrupt chunk is abandoned.
9. **Files outside the brief's write-set, mechanically:** `eprfs-peer/src/local_blob.rs` (the two
   `_with` variants; existing event bytes unchanged), `epr-cli/src/lib.rs` (the module), and
   `pub use bytes::Bytes` in `eprfs-peer/src/lib.rs` (epr-cli has no `bytes` dependency and its
   `Cargo.toml` is E4's).
10. **The payload store and flow store `put` writes are the daemon's by the same resolution:** the
    flow store from the head table's recorded store place, the payload store by `serve`'s rule
    (`--payload-store`, `EPR_PAYLOAD_STORE`, XDG). A daemon started with an explicit
    `--payload-store` that `put` is not also given would split them; E4's one-store-per-device
    marker closes this.
11. **The fetching device records no custody Produce,** so it does not re-serve a fetched artifact.
    Serving onward needs a design ruling on custody by fetch.
12. **No a2o features.** §3.e lists `sibling-fetches-the-build.feature` and
    `artifact-custody.feature`; the brief assigns only the Rust tests, and a2o authoring sits under
    the blind-reader covenant clause that track S works under.
13. **The session envelope was not opened.** `epr session open --model claude-opus-5-5` refused
    `session-conflict` (the session is the orchestrator's, bound to the main checkout), as in E3;
    the commit printed the expected `UNMEASURED` line.
14. **An extra daemon-level test** (`a_queued_fetch_is_run_by_the_devices_daemon_and_answered`),
    because the queue path is not otherwise exercised.
