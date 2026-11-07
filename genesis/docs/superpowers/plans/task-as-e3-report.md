---
id: task-as-e3-report
title: serve-blob mount with scope enforcement (artifact-store E3)
status: claimed-not-verified
landed_commit:
verified_by:
concerns: "TierRequest/TierAnswer gained variants (a breaking change to exhaustive matches; fold.rs's fixture updated). A granted artifact root passes admit_root for tree reads too (design §1.3 as written); the tree endpoint then answers Failed/Unanswered from the channel store and never serves artifact bytes. A session closes on connection close, so a fetcher that opens one connection per request records one partial Use per request: F2 should draw one artifact over one connection."
actor: agent:as-e3@claude-opus-5-5
cites: []
---

# E3: `serve-blob` mount with scope enforcement

Worktree `as-e3`, branch `artifact-store/e3`, base `303af86f`. Specification: the sprint's
`design.md` §1 (and §0 F0.1, F0.2, F0.6 to F0.9). Commits: `4a362a06` (implementation) and `b308cf96` (the payload store creates its
private parents; the CLI tests mount their own payload store).

## What changed

- **`eprfs-peer/src/gate.rs`.** `Served { Channel, Artifact { method }, Unknown }` and
  `trait ServeScope { classify, object_of, methods_of, grants }`. `StewardDevicesGate` now holds
  an `Arc<dyn ServeScope>`. `new` keeps the old behaviour through `WholeStoreScope`; `with_scope`
  takes a scope, and `admitted(who)` is public. The rules:
  - `admit_root`: admit(who)?. Then Channel is Ok; Artifact{m} granted is Ok; Artifact{m} not
    granted is `OutsideStanding` with standing `serve-blob:<m>`; Unknown is `OutsideStanding`
    with no standing.
  - `admit_object`: admit(who)?. Then an object indexed under some artifact whose method is
    granted to the device is Ok; anything else is `NotUnderRoot` with no standing.

  The doc comment records the deliberate narrowing of `reach.rs`'s normalization rule: labels go
  to the person's own admitted devices only.
- **`eprfs-peer/src/serves.rs` (new).** `ArtifactServeList { by_method }` with
  `POLICY_KEYS = ["berth","artifacts","serves"]`, `from_policy` and
  `grants(device, method, &Admission)`. `steward` admits every device `admits_device` admits now.
  `carrier:did:key:…` parses, enters the digest, and grants nothing (reserved for D9). Any other
  audience is a `PeerError::Policy`; keys starting with `_` are skipped.
- **`eprfs-peer/src/rules.rs`.** `Rules` carries `serves`, and `Rules::digest` includes it. An
  empty grant is skipped in the canonical form, so a policy that serves nothing keeps its old
  digest. Relays stay outside the digest.
- **`eprfs-peer/src/blob_mount.rs` (new).**
  - Types: `BlobMount { store, index, served, sessions, refused }`,
    `ArtifactIndex { by_output, by_object, unindexed }`, `Held`,
    `ServedDraw { requester, artifact, manifest, bytes, chunks, of, outcome, in_scope_of }`,
    `DrawOutcome { Completed, Partial, Refused { reason } }`,
    `Unindexed { MethodUnknown, ManifestInvalid { reason } }`, `DeviceScope` (channel roots, then
    mount index, then the grant read under the current admission), and `open_payload_store`.
  - Indexing: `index_manifest` reads the manifest with `read_tier_manifest`. It requires the
    Produce event's output to equal `manifest.record.output`, fetches the method record locally,
    decodes it as `MethodDescriptor`, and requires `descriptor.cid() == method` and equal
    `deterministic` flags. Any failure fails closed.
  - `rebuild_from_flow` scans `custody-blob` + `copy:self` + `manifest:<cid>` Produce events.
  - `answer` runs `admit_root`, then `admit_object`, then `object_of`, then
    `fetch_blob(LocalOnly)`, and verifies the bytes before answering.
  - Sessions are keyed (requester did, artifact). A session closes on its last distinct chunk
    (`Completed`), on its connection's close, or after `SERVE_IDLE = 60 s` (`Partial`). An
    admitted device's refusal is one `draw:refused` draw, deduplicated per (device, artifact,
    reason) for `SERVE_IDLE`.
- **`eprfs-peer/src/tier.rs`.**
  - `TierRequest::{Manifest { artifact }, Blob { artifact, cid }}` and
    `TierAnswer::Blob { cid, bytes }`, on the same ALPN and framing.
  - `TierProtocol::with_scope(self_id, store, admission, limits, scope, mount)`; `new` delegates
    to it with `WholeStoreScope` and no mount.
  - The accept loop passes the connection's `stable_id` and calls `mount.connection_closed` at
    its end.
  - `check_answer`: a Blob answer's bytes must hash to its CID, and a `Blob` request's answer
    must be the CID asked; otherwise `TierLeafIntegrity`.
  - New counters: `served_manifests`, `served_chunks`, `refused_scope`.
- **`eprfs-peer/src/daemon.rs`.** `PeerConfig.payload_store: Option<Arc<dyn EprfsStorage>>`.
  `start` builds the mount, `DeviceScope` and `TierProtocol::with_scope`. `Run::tick` calls
  `reindex` (re-scans when the flow log's stamp moves) and `record_serves` (sweeps idle sessions,
  appends one Use per draw; an append failure is logged as `PeerError::ServeUnrecorded`). A rules
  reload swaps the shared grant.
- **`eprfs-peer/src/local_blob.rs`.** `pub(crate) fn custody_use_event(ctx, subject, resource,
  bytes, slots)` is factored out of `stream_local_blob`, whose event bytes are unchanged.
  `LocalBlobContext.crossing: Option<(String, String)>` adds `measure:`/`reason:` to the Produce
  only.
- **`eprfs-peer/src/error.rs`.** `ArtifactUnindexed { manifest, reason }` and
  `ServeUnrecorded { artifact, source }`.
- **`eprfs-storage/src/tier_blob.rs`.** Appended `pub async fn read_tier_manifest`; E2's
  `decode_manifest` body is untouched. `lib.rs` re-exports it.
- **`epr-cli/src/peer.rs`.** `serve` passes
  `payload_store: Some(open_payload_store(dir)?)`. The directory is `--payload-store`, else
  `EPR_PAYLOAD_STORE`, else `$XDG_DATA_HOME/epr/payload` (else `$HOME/.local/share/epr/payload`).
  The usage text is updated.
- **`genesis/agentic/pool-policy.json`.** `berth.artifacts = { _comment, serves: { "mesh-build":
  ["steward"] } }`.

## Refusal shapes (`self_id = channel_tier_id()`)

- **Stranger, for any artifact** (granted, held ungranted or unknown, Manifest or Blob):
  `TierAnswer::Unanswered { cid: <artifact asked>, path: TreePath::root(), why:
  ItemAbsence::refused(channel_tier_id(), ReachRefusal { reason: OutsideStanding, needs_standing:
  None }) }`. This is byte-identical to the tree endpoint's `List { root: <same cid> }` refusal;
  with the echoed CID set aside, the three frames are one frame. Nothing is recorded; only
  `refused_scope` counts it.
- **Admitted device, artifact of an ungranted method `m`:** the same envelope with
  `ReachRefusal { reason: OutsideStanding, needs_standing: Some("serve-blob:<m>") }`. It is
  recorded as `draw:refused` + `reason:serve-blob:<m>`.
- **Admitted device, object not under the named artifact** (or not under any granted artifact):
  `ReachRefusal { reason: NotUnderRoot, needs_standing: None }`. It is recorded with
  `reason:not-under-root`.
- **Admitted device, root that is no indexed artifact** (unknown, a manifest CID in the artifact
  slot, a channel root, an unindexed artifact): `ReachRefusal { reason: OutsideStanding,
  needs_standing: None }`. It is recorded with `reason:outside-standing`.
- **Admitted device, granted object missing from the store:**
  `Unanswered { cid: artifact, path: root, why: ItemAbsence::not_held([]) }`. A store error or a
  corrupt held object gives `Failed { reason }`.

## The `serve-blob` Use event

```
action: Use, provider: <holder did>, receiver: <requester did>, resource: <artifact>
quantity: Count { value: <chunk bytes served>, unit: "bytes" }
in_scope_of: <the custody Produce's scope> (the artifact CID when the artifact is not indexed)
classified_as: ["custody-blob", <artifact>, "serve-blob", "copy:<requester did>",
                "manifest:<cid>"?, "draw:completed|partial|refused", "reason:<…>"?]
```

It has no `measure:` slot, so `CrossingAllowList::admits` is false for it and the channel projection leaves it out (enforced: elohim/eprfs/eprfs-peer/tests/serve_blob.rs `an_admitted_sibling_draws_a_granted_artifact_and_the_holder_records_one_serve_blob_use`).

## Tests

Focused runs pass: `cargo test -p eprfs-peer` with every target green after the fixes below.
`tests/serve_blob.rs`:

- `an_admitted_sibling_draws_a_granted_artifact_and_the_holder_records_one_serve_blob_use`: ok
- `an_admitted_sibling_asking_outside_its_grant_is_refused_with_a_typed_reason`: ok
- `a_stranger_gets_the_uniform_refusal_for_granted_ungranted_and_unknown_artifacts`: ok
- `a_device_dropped_by_a_reload_is_refused_on_its_next_chunk_on_an_open_connection`: ok
- `a_daemon_down_put_is_indexed_and_served_after_start`: ok
- `a_manifest_without_its_method_record_is_unindexed_and_never_served`: ok
- `a_swapped_manifest_cid_for_output_cid_is_refused_both_ways`: ok
- `the_serve_grant_parses_its_vocabulary_and_refuses_anything_else` (extra): ok

Other test files:

- `tests/daemon.rs`: `a_started_daemon_serves_only_its_channel_tree` (was :968) is amended and
  renamed `a_started_daemon_serves_its_channel_roots_and_granted_artifact_roots_only`: ok
- `tests/rules.rs`: `serves_enters_the_rules_digest_and_relays_still_do_not`: ok
- `src/blob_mount.rs`: unit `rfc3339_utc_reads_known_instants`: ok
- `src/endpoint/relays.rs`: `relay_fields_stay_outside_the_rules_digest_regression_guard`: still
  ok

## Gates

Evidence directory: `genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/`
(main checkout). Both gates ran on committed state. The runner wrote no evidence ("source closure
dirty") because of the orchestrator's uncommitted `briefs/common.md` copy in this worktree.

- `just gate eprfs` at `b308cf96`: `gate-E3-eprfs.log`, **EXIT=0**.
- The first run, at `4a362a06`, is kept as `gate-E3-eprfs.run1-red.log` (EXIT=1).
  `epr-cli/tests/peer.rs::the_serving_line_is_on_stdout_and_sigterm_sigint_stop_cleanly` saw no
  serving line: `serve` exited because `$XDG_DATA_HOME/epr` did not exist, and
  `DirectoryStorage::create` makes only the leaf. Fixed in `b308cf96`. That run also showed the
  epr-cli fixtures would have written to the account's real data directory; they now set
  `EPR_PAYLOAD_STORE` in their own config dir.
- `just gate orchestrator` at `b308cf96`: `gate-E3-orchestrator.log`, **EXIT=1**.
  - The one failing test is `gate-runner.test.mjs:757` "gate evidence rejects dirty external
    dependency and changes key when it is committed". It fails on its first assertion
    (`unavailable (source closure dirty)` on its own temp fixture).
  - It is **pre-existing and not this track's**. It is red in track B1's orchestrator logs
    (`gate-B1-orchestrator.log`, `gate-B1-orchestrator-run1-dirty.log`) and in the gate-live run
    `20261107T185125Z-303af86f…` at this branch's base, which has none of E3's changes.
  - Every other orchestrator test is green, including those that read `pool-policy.json`.
  - The test's code lies outside E3's write-set, so it is left unfixed here.

## Deviations from design.md §1, with reasons

1. **The `ServeScope` trait has a fourth method, `methods_of(cid)`.** `admit_object` needs "is
   this object indexed under *some* granted artifact", which `object_of(artifact, cid)` cannot
   answer without enumerating artifacts.
2. **`ServeScope` is a trait, and the daemon's struct is `DeviceScope { channel, mount, serves,
   admission }`.** §1.1's `ServeScope { channel, mount, serves }` and §1.3's trait share a name.
   The struct also holds admission, because `grants` is judged against admission now.
3. **`TierProtocol::new` keeps its signature (with `WholeStoreScope`, no mount); the daemon uses
   the new `TierProtocol::with_scope`.** Existing tier tests and callers are unchanged.
4. **`epr peer serve` opens the store through `eprfs_peer::open_payload_store` instead of
   constructing `OffloadedDirectory` itself.** epr-cli does not depend on eprfs-storage, and its
   `Cargo.toml` is E4's write-set. The helper creates the same
   `OffloadedDirectory::new(DirectoryStorage::create(path)?)`.
5. **Files touched outside the brief's write-set, mechanically:**
   - `eprfs-storage/src/lib.rs`: the re-export of `read_tier_manifest`.
   - `PeerConfig` literals (`payload_store: None`): `eprfs-peer/tests/{fold.rs, relay_compat.rs}`
     and `epr-cli/tests/peer.rs`.
   - `LocalBlobContext` literal (`crossing: None`): `tests/local_blob.rs`.
   - `fold.rs` fixture: a match arm for the new request variants.
6. **The `tests/daemon.rs` test is renamed** because its old name ("only its channel tree") is
   now false.
7. **A tree read of a granted artifact root passes `admit_root`, as §1.3 specifies; the tree
   endpoint reads only the channel store and answers `Failed`/`Unanswered`.** The amended daemon
   test asserts that no tree answer ever lists or reads an artifact.
8. **Refusal events are deduplicated per (device, artifact, reason) for `SERVE_IDLE`.** This
   keeps a retrying device from writing one event per request; §1.5 says "one `draw:refused`
   event".
9. **The epr session envelope was not opened per common.md.** `epr session open` refused
   `session-conflict`: the runtime session is the orchestrator's, bound to the main checkout. The
   commit prints the expected `UNMEASURED` line (STATE.md 18:55Z).

## Notes for F2

- Draw one artifact over one connection; the session closes when its connection does.
- `TierAnswer::Blob` for `Manifest` carries the manifest's own CID. Decode it and check
  `record.output == artifact` in the fetch layer.
- `LocalBlobContext.crossing` is ready for the crossing Produce.
