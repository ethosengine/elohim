---
id: task-as-f2-report
title: the sibling fetch, producer and consumer (artifact-store F2a + F2b)
status: verified-on-branch
landed_commit: 5c53da02
verified_by: "astra review + re-review (evidence/f2-astra-review.md, f2-astra-rereview.md), orchestrator ruling (f2-ruling.md) and delta review of 5c53da02 (f2-delta-review.md, APPROVE); gates on e626043f eprfs EXIT=0, orchestrator EXIT=0"
concerns: "Astra BLOCK on ed29026b6 addressed in fa3aba0f3 + 47b57023; named residuals: build-time TOCTOU, node_modules excluded from the ignored-file refusal, same-account staging swap between inode check and rename, env -i not yet run on a real cargo build (F2r), honest computation out of scope (F5/I6). The key input is the labelled interim KeyInput::Raw(git-tree closure CID) with slot basis-key:git-tree, not KeyInput::Projected: no producer of the HEAD tree as an eprfs Tree v1 closure exists on this tree, so an F2r receipt on this key does not close F2. The 454 MB receipt (F2r) and the two-device relay throughput floor are not run here."
actor: agent:as-f2@claude-opus-5-5
cites: []
---

# F2: the sibling fetch (producer F2a, consumer F2b)

Worktree `as-f2`, branch `artifact-store/f2`, based on `artifact-store/e3` (`a655b6ba3`).
Specification: the sprint's `design.md` §3 (and §1 for the serving side). Commits: `ed29026b6` (implementation and tests), `7a9833a24` (this report), `fa3aba0f3` (the astra review fixes), `47b57023` (the deadline-race fix), and the commit adding the review section.

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

## After review: astra BLOCK (`evidence/f2-astra-review.md`)

The review confirmed the network core: manifest CID, key and output checks; chunks by CID and
length; the whole output verified before `Completed`; a fetch confers neither custody nor onward
serving. It blocked on five highs and four mediums. All nine are fixed in `fa3aba0f3`. Where a fix
supersedes a deviation listed above, the deviation's number is given.

- **H1, staging substitution** (`fetch.rs` `install_fetched`).
  - The bytes are staged in `artifact`, created with `create_new` (O_EXCL, mode 0600) inside a
    fresh directory beside the destination. `mkdir` is exclusive, the mode is 0700 and the name is
    128 random bits.
  - On `Completed` the file is made 0755 and fsynced through its own descriptor. The staged path
    must still name that inode (device and inode, regular file). It is renamed onto `dest`, `dest`
    must then name the same inode, and the directory is fsynced.
  - The staging directory is removed on every exit path (a `Drop` guard), including a deadline
    that drops the install.
  - `mesh_fetch`'s `chmod` is no longer what publishes the executable.
  - Test: `a_precreated_staging_link_never_receives_bytes`.
- **H2, queue identity** (`fetch.rs` queue types, `artifact.rs` `wait_for_answer`).
  - Every request carries a 128-bit nonce. Its files are `<key>-<nonce>.json`, `.done.json` and
    `.cancel`.
  - `FetchDone` names its request in `for`: nonce, key, output and destination. The CLI takes only
    an answer whose four fields equal its own and removes any other answer at its path.
  - A timed-out CLI withdraws an unclaimed request, or writes `.cancel` for a claimed one; the
    daemon aborts the fetch and answers `Cancelled`. Answers nobody collected are pruned after an
    hour.
  - Test: `a_completion_for_another_request_is_never_accepted`.
- **H3, effective build environment** (`artifact.rs` `build_env_refusal`, `cargo_config_lines`,
  `describe`; `hc-mesh.sh`). Supersedes deviation 3's claim about wider views.
  - A recorded mesh build runs cargo under `env -i`. What passes is: the method's own variables
    (`mesh_build_env`, `RUSTFLAGS`, `CARGO_TARGET_DIR`); `PATH`, `HOME`, `LANG`, `TMPDIR`; where the
    toolchain lives (`CARGO_HOME`, `RUSTUP_HOME`); and the native toolchain variables (`CC`, `CXX`,
    `AR`, `LIBCLANG_PATH`, `PKG_CONFIG_PATH`, `OPENSSL_*`, `PROTOC`), each of which `describe`
    records when set.
  - `epr artifact key` and `put` refuse (exit 3) when any `CARGO_*`, `RUST*` or native variable is
    set without being recorded with the same value, or a recorded one is unset. Only these are
    benign: `CARGO_HOME`, `RUSTUP_HOME`, `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS`,
    `RUST_TEST_THREADS`, `RUST_BACKTRACE`, `RUST_LOG`; plus `RUSTUP_TOOLCHAIN` equal to the
    recorded toolchain.
  - `hc-mesh.sh` runs the `epr` calls with every other cargo and rust variable unset.
  - `describe` records the crate directory (`crate`). The closure adds one line per
    `.cargo/config` and `.cargo/config.toml` found from the crate upward, nearest first, by content
    CID and not by path (so two checkouts in different places key alike), plus
    `$CARGO_HOME/config(.toml)` (`-` when absent).
  - Tests: `an_unrecorded_cargo_env_var_refuses_the_key`, `an_ancestor_cargo_config_moves_the_key`.
- **H4, "clean" was not what was read** (`artifact.rs` `closure_key`). Supersedes deviation 2's
  during-build claim.
  - Each closure line is `path\t<mode> <type> <oid>` from `git ls-tree`, so a selected file's mode
    is keyed.
  - A tracked symlink in the view whose target is absolute or resolves outside the view refuses.
  - `assume-unchanged` or `skip-worktree` flags in the view refuse, and so does a submodule
    overlapping the view with an `ignore` setting other than `none`.
  - Status runs with `--ignored=matching --ignore-submodules=none`; ignored files inside the view
    refuse.
  - Tests: `entry_mode_moves_the_key`, `an_out_of_view_symlink_refuses`, `an_index_flag_refuses`
    (which also covers an ignored file).
- **H5, queue surface** (`daemon.rs` `fetch_queued`, `next_request`; `fetch.rs`
  `read_queue_file`, `InstallScope`).
  - The daemon creates the queue 0700 at start and reads it only while it stays private. The CLI
    creates it the same way.
  - An entry is read only when `symlink_metadata` says it is a regular file of at most 64 KiB. It
    is opened `O_NOFOLLOW | O_NONBLOCK` and checked again on the descriptor. Anything else (a link,
    a FIFO, a device) is removed unread.
  - The destination must resolve, through its nearest existing ancestor and with no `.` or `..`
    component, under an `--install-root`, `EPR_INSTALL_ROOTS`, or a `mesh-bin` directory under the
    cargo pool (`CARGO_TARGET_POOL_ROOT`, else `/projects/.cargo-target-pool`). Otherwise the answer
    is `InstallRefused`. `epr peer serve` logs where it will install.
  - Tests: `a_queue_entry_naming_an_outside_destination_is_refused`,
    `a_fifo_in_the_queue_never_blocks_the_tick` (a FIFO and a link to `/dev/zero` sorted ahead of
    the real request).
- **M6, one deadline.** Supersedes deviation 6.
  - `fetch_artifact_until` runs the whole fetch under `timeout_at(deadline)`: manifest, resume
    checks, dials, chunks and store writes.
  - Secondary holders are dialled together, each bounded by what remains.
  - The daemon's install runs under the same deadline. A cut-off fetch reports
    `DeadlineExceeded { fetched, total }` from live counters.
- **M7, wasted bytes.** Supersedes deviation 8.
  - `tier_request_counted` (a counted frame read added to `tier.rs`) adds every answer byte to a
    counter as it arrives, so the count survives a decode error, a stream error, or a deadline
    that drops the request.
  - Every discarded answer (corrupt, refused, failed, malformed, stalled) and every manifest not
    accepted adds what actually arrived. The corrupt-chunk test now asserts the chunk's whole frame
    (its length plus at most the frame overhead).
- **M8, relay floor.** Supersedes deviation 7.
  - On a relay-only holder each request is bounded by
    `min(stall, 10 s × ceil(len / 4 MiB))`, measured on the client.
  - A relay holder that stalls or loses its streams is abandoned, and when the remaining holders
    are all relay paths so abandoned, the gap is `TransportCannotDeliver { "relay" }`. No test
    reaches a relay-only path in-process.
- **M9, two payload stores.** Supersedes deviation 10.
  - `epr peer serve` records the canonical payload store it serves in `<peer dir>/payload-store`.
  - `put` refuses (`unrecorded:`) when a serving daemon recorded a different store than the one
    `put` would write.

**Named residuals, not closed here.**
1. **Time of check, time of use.** A source file changed after the key is computed and restored
   before `put` re-checks passes both checks. The bytes are then recorded under a closure they were
   not built from. Closing this needs the build to read from a snapshot (a fresh export of HEAD's
   view), not the working tree.
2. **`node_modules` is excluded from the ignored-file refusal by declaration.** Cargo does not read
   it, but a `build.rs` could. Without the exclusion every storage build refuses, because
   `elohim/sdk/**` holds pnpm's `node_modules`.
3. **A same-account process can still swap the staging path** between the inode check and the
   rename. The post-rename inode check then reports `IntegrityFailed`, and `mesh_fetch` builds
   here and overwrites; until then the substituted file stands at `dest`. A cross-account process
   cannot reach the private directory.
4. **`env -i` is exercised by the dry run and by `describe` + `key` on the real views only.** On
   this tree both keyed cleanly: storage `bafyreigehtzpexo…`, doorway `bafyreihu5mrawly…`, each
   recording `CC` and `LIBCLANG_PATH`. A real cargo build under the allowlist, and so whether the
   native variables suffice, is exercised at F2r.
5. **Out of scope: an admitted builder that publishes bytes it did not compute.** Byte
   verification proves the bytes correspond to the declared output, not that they were honestly
   computed from the claimed sources. That is a question of reach and standing (F5/I6: signed
   records, recomputation, agreement across builders).

**Files beyond the brief's write-set for these fixes.** `eprfs-peer/Cargo.toml` adds
`libc = "0.2"` (already in the lock) for `O_NOFOLLOW | O_NONBLOCK`. `eprfs-peer/src/tier.rs` adds
the counted request. `PeerConfig` gains `install: InstallScope`, and every literal in
`eprfs-peer/tests/{daemon,fold,relay_compat,serve_blob}.rs` and `epr-cli/tests/peer.rs` gains
`install: Default::default()`. `FetchGap` gains `InstallRefused` and `Cancelled`.

**Tests after the fix** (focused, `RUSTFLAGS=""`):
- eprfs-peer: `fetch` 15/15 (the twelve above plus `a_precreated_staging_link_never_receives_bytes`,
  `a_queue_entry_naming_an_outside_destination_is_refused`,
  `a_fifo_in_the_queue_never_blocks_the_tick`), `serve_blob` 19/19, `daemon` 12/12, `local_blob`
  5/5, `tier_alpn` 14/14.
- epr-cli: `artifact` 9/9 (the three above plus `an_unrecorded_cargo_env_var_refuses_the_key`,
  `an_ancestor_cargo_config_moves_the_key`, `entry_mode_moves_the_key`,
  `an_out_of_view_symlink_refuses`, `an_index_flag_refuses`,
  `a_completion_for_another_request_is_never_accepted`), `peer` 29/29.
- `cargo clippy -p eprfs-peer -p elohim-epr-cli --all-targets -- -D warnings`: EXIT=0.
- The first test run of the new CLI tests failed in the fixture, not the code: the workspace
  exports `CC` and `LIBCLANG_PATH`, which the build-environment check correctly refused. The
  fixture now scrubs them.

**Gates after the fix:** both on committed `47b57023` (`fa3aba0f3` plus the deadline fix below), `EPR_BIN` per
common.md.
- `just gate eprfs`: `gate-F2-eprfs.log`, **EXIT=0** (`fetch` 15/15, `artifact` 9/9; 20 min).
- `just gate orchestrator`: `gate-F2-orchestrator.log`, **EXIT=0**.
- Earlier logs are kept beside them: `gate-F2-*.run1-pre-review.log` (the original EXIT=0 runs at
  `ed29026b6`), `gate-F2-orchestrator.run2.log` (EXIT=0 at `fa3aba0f3`) and
  `gate-F2-eprfs.run2-red.log` (EXIT=1 at `fa3aba0f3`).
- The red was in `a_trickling_holder_is_abandoned_on_the_total_deadline_and_the_chunk_re_raced`,
  phase 2: it got `NoRoute` instead of `DeadlineExceeded`. M6 caps each request's bound at the time
  remaining, so near the deadline a request could elapse a tick before the outer `timeout_at` and
  abandon its holder. Fixed in `47b57023`: a request, or a manifest-phase dial, that elapses at
  the deadline is `DeadlineExceeded`. The `fetch` tests then passed five times in a row at four
  threads.
- Neither run recorded evidence. The runner names the three dirty paths in the closure: this
  report while it was being written, and the two uncommitted brief copies
  (`briefs/common.md`, `briefs/F2-sibling-fetch.md`).

## After the re-review: astra BLOCK again, and the orchestrator's ruling

The re-review (`evidence/f2-astra-rereview.md`, 13 items) was ruled in `evidence/f2-ruling.md`.
F2 is a developer tool on the steward's own workspaces. A process running as the same account
already holds what those attacks would win, so findings that need one are named residuals, not
gates. Four items were ruled to fix; they are fixed in `5c53da02`.

- **Item 3 (H3): a fixed PATH, and the tools it resolves, are keyed.**
  - The recorded mesh build runs cargo under `env -i` with the fixed
    `PATH=${CARGO_HOME:-$HOME/.cargo}/bin:/usr/local/bin:/usr/bin:/bin`, not the inherited one.
  - `describe` records that PATH in `non_path_inputs`.
  - The key adds one line per keyed tool resolved under it (`cc`, which is rustc's default linker
    driver on the gnu targets; `ld`; and `$CC` when the method records it). Each line holds the
    tool's canonical path and the CID of its `--version` output, or `-` when it does not resolve.
  - Test: `changing_the_path_resolved_cc_moves_the_key` covers another version from the same cc and
    another cc on another PATH.
- **Item 9 (M9): a missing record fails closed.** While the serving lock is held, `put` refuses
  (`unrecorded:`) when `<peer dir>/payload-store` is missing, unreadable or empty, as well as when
  it names another store. Because `serve` writes the record just after taking the lock, the
  interval between the two also refuses. Test: `a_served_daemon_without_a_store_record_refuses_put`
  covers a missing record, an empty one, one naming another store, then the right one.
- **Item 11: the bound that fired decides; there is no tolerance window.**
  - Every dial and request now carries a `Bound { wait, by_deadline }`. `by_deadline` is true when
    the time remaining, rather than the holder's own stall or relay bound, set the wait.
  - When that bound elapses, the gap is `DeadlineExceeded`. When the holder's own bound elapses, the
    holder is abandoned (stalled) and its chunk re-raced.
  - A dial now tells an elapsed bound apart from a refusal.
  - The 20 ms `past()` window is gone. The trickle test passed five consecutive runs at four
    threads (16/16 each run; epr-cli `artifact` 11/11; clippy `-D warnings` clean).
- **Item 12: crash-orphaned staging is swept.** This is lock based.
  - A staging directory is named `.fetch-<destination name>.<32 hex>`. Its installer holds an
    advisory lock (`File::try_lock`) on the directory's `lock` file for as long as it lives, and the
    kernel releases the lock however the process ends.
  - Before staging, an install removes each directory beside its destination with that
    destination's prefix whose lock nobody holds now. A directory without a lock file is removed
    only after a 60 s grace, since it may be a live installer's first moment. Links are not
    followed, and other destinations' directories are not touched.
  - Test: `an_orphaned_staging_dir_from_a_killed_run_is_removed_by_the_next_install`. The killed
    run's directory is removed; a held one, a lockless young one and another destination's are kept.
  - Residual: between a live installer creating its lock file and taking the lock, a concurrent
    sweeper could take it first and remove the directory. That installer then fails `LocalStore`
    and the caller builds here. One daemon runs one fetch at a time.

**Residuals and follow-ups the ruling carries** (`evidence/f2-ruling.md`; numbers are the
re-review's):
1. **Item 1 (H1), same account.** In-place modification of the staged inode, and the rename race,
   stay open. The inode checks detect neither the first nor, before publication, the second.
2. **Item 2 (H2), same account.** A queue writer that reads a request's nonce can forge an
   `Installed` answer, and the CLI does not re-verify the installed bytes. A `.cancel` entry is
   honoured by its name, not its contents.
3. **Item 4 (H4), same account / honest-rare.** An edit made during the build and restored before
   `put` re-checks still passes. `node_modules` stays excluded from the ignored-file refusal, by
   declaration. An edit that is not restored is caught by the dirty check.
4. **Item 5 (H5), same account.** A parent directory of the destination swapped for a symlink
   between the scope check and the install can redirect it. The daemon also trusts the queued
   key, output and holder selection rather than re-resolving the crossed records.
5. **Item 6 (M6), operational / same account.** The synchronous flush, fsync and rename cannot be
   interrupted by the async deadline, so a publication can finish after the deadline. Withdrawal
   races a claim already read. `abort()` is answered `Cancelled` without awaiting termination.
6. **Item 10, frontier F5/I6.** An admitted builder can publish bytes it did not compute. Byte
   verification proves the bytes match the declared output, not that the computation was honest;
   independent computation evidence is the release plane's job.
7. **Follow-up, item 7 (M7).** Wasted bytes are still undercounted when the outer deadline drops
   in-flight request tasks or returns from the manifest phase, and partially read length headers
   are not counted.
8. **Follow-up, item 8 (M8).** Relay classification uses the advertised addresses. Enforcing the
   floor on the actual path needs the connection's real route.
9. **Item 13, carried to F2r.** No real cargo build has run under the `env -i` allowlist (now with
   the fixed PATH); F2r's first build is the validation.

**Brief copies.** The orchestrator's `briefs/common.md` was restored to HEAD in `as-f2` with
`brit restore --worktree`. The untracked `briefs/F2-sibling-fetch.md` was moved out of the
worktree, not deleted; the hub copy in the main checkout is canonical. Neither was committed or
edited.

**Gates on the final head.** Both run on the commit that adds this section, with the brief copies
restored so the runner can record evidence: `evidence/gate-F2-eprfs.log` and
`evidence/gate-F2-orchestrator.log`, each ending in its `EXIT=` line. The runs at `47b57023` are
kept as `gate-F2-*.run3-47b57023.log`.
