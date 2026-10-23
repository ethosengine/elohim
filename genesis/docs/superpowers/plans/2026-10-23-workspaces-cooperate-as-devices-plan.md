---
title: Workspaces cooperate as devices — the steward channel and steward-scoped berth
id: workspaces-cooperate-as-devices-plan
status: Draft
class: architecture
date: 2026-10-23
steward: human:matthew
serves: [devices-share-one-berth, ledgers-fold-once]
domain: D5
sprint: Sprint-2
cites:
  - "workspaces-cooperate-as-devices-design | the spec this plan lanes; read §3 before any task | sha256:88c203269b34f963 | path: genesis/docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md"
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
  - "placement-contract-architecture-plan | sibling plan: eprfs tier/walk lanes this link serves over iroh | sha256:5417b3363dffdc44 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "elohim-rust-style-guide | Global Constraints: strict-from-birth eprfs-peer, RS-GOV-02 paydown per touched crate | sha256:30c95d2e27e9087f | path: genesis/docs/rust-style-guide.md"
  - .epr-meta/ledgers-fold-once.habit.md
---

# Workspaces cooperate as devices: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task by task. Each `- [ ]`
> item is one gap item that `epr flow project` mints. Its sub-bullets are the steps, written
> test first.

**Goal:** Two workspaces bound to the same human share one view of who holds `dev`, Jenkins dispatch,
the alpha fleet and each lane, over their own iroh link. Neither reads an offline peer as a free
resource.

**Architecture:** berth's records become FlowEvents (the first move of ledgers-fold-once). Each device
projects the allowed kinds into a signed eprfs tree and announces its head over a gossip topic that
only the steward's devices may join. Peers fold each other's events through the eprfs tier, served on
an iroh ALPN by a new crate, `eprfs-peer`. Steward-scoped berth claims are effective only once every
other device's current head has been folded.

**Tech stack:** Python (berth, hooks); Rust 1.96.1 in the eprfs workspace (`epr-rea`, `epr-cli`,
`eprfs-serve` and the new `eprfs-peer`); iroh and iroh-gossip `=0.92`; Gherkin a2o.

**Spec:** `genesis/docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md`.
Read §3 before any task. This plan does not restate it.

## Global constraints

- Rust style guide: `eprfs-peer` carries `rust-style-strict@1` in its own `.epr-meta` from its first
  commit. Every `allow` or `expect` needs a literal `reason`. Each touched indebted crate (`epr-rea`,
  `epr-cli`, `eprfs-serve`, `eprfs-core`) removes at least one finding per push.
- Run the gate with `just gate <project>`. In this container, export
  `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0='*'` and
  `PATH=/opt/rust/cargo/bin:$PATH RUSTUP_HOME=/opt/rust/rustup` in background shells.
- A new crate needs its `seam-registry.yaml` **routed** in `.claude/epr-meta/seam-catalog.yaml` in
  the same commit. An unrouted registry fails every gated push; that happened on 2026-10-23.
- `iroh` and `iroh-gossip` are pinned `=0.92`, matching `elohim/elohim-storage/Cargo.toml`.
- No key, keystore or credential is copied between devices, printed or logged. The iroh secret
  stays at `$XDG_CONFIG_HOME/epr/peer/iroh.key` with mode 0600.
- The crossing allow-list is default-deny. Recall, session and memory stores never enter the
  channel tree.
- elohim-storage takes no dependency on `eprfs-serve` or `eprfs-peer` (operator ruling, R-E3 kept).
- Commit by pathspec only. Ledgers under `.claude/data/` are never staged by a task.

## Review focus

1. **Offline peer.** The claim reads `unconfirmed: <device> last seen <age>`, exits 3, and never
   reads as free. Pinned in T6.
2. **Simultaneous claim.** Exactly one claim stays effective after both folds, by the tie-break
   (earlier `claimed_at`, then the lower did:key). Pinned in T6.
3. **Revoked device.** After a Contest on its binding, its heads are ignored and its tier
   connections refused, without restarting either side. Pinned in T2 and T4.
4. **Replay and rollback.** A replayed or lower-`seq` head folds nothing, and the
   `channel_rollbacks_refused` count rises. Pinned in T5.
5. **Allow-list leak.** An event kind not on the list never appears in the channel tree. Pinned in T5.

---

## Lane 0: story and habit

- [ ] **T0 — the devices-share-one-berth story and habit, born red** (serves devices-share-one-berth)
  - Create `genesis/a2o/features/devflow/devices-share-one-berth.feature`, tagged
    `@e2e @devflow @concern:devices-share-one-berth @act:host @wip`. It has four scenarios on two
    devices in one container: a held claim refused naming the holder; an offline device read as
    unconfirmed; a simultaneous claim settled once; a revoked device ignored. A fifth scenario,
    tagged `@requires:shem`, runs the first scenario across the two workspaces. Opus authors it,
    then a fresh `blind-reader` with the `a2o-story` profile reviews it until READY.
  - Create `genesis/agentic/.epr-meta/devices-share-one-berth.habit.md` with the invariant, checks
    and retire-when from spec §6. Set `status: red` and `active: false`.
  - Verify: `.claude/scripts/habits-project.py` regenerates `genesis/manifests/habits.yaml`, and
    `--check` passes. The blind reader's verdict is recorded in the item report.

## Lane 1: berth records become folds

- [ ] **T1 — berth emits every record kind as one dev-berth-events@1 fold** (serves ledgers-fold-once)
  - Test first, in `genesis/agentic/berth_test.py`: `test_fold_every_kind_emits_once`. Each of
    moor, unmoor, claim, renew, release, override, say and shed calls the injected `obs` emitter
    exactly once, with measure `dev-berth-events@1`, subject = the resource (or `.` for moor/say),
    `reason` = the kind, and `env` holding every field of the record. Also
    `test_fold_emitter_missing_costs_fold_not_lease`: with an emitter that returns False, the lease is
    still written and the claim still exits 0.
  - Implement in `Berth.record(self, kind, **fields)`, `genesis/agentic/bin/berth:235`. After the
    ledger append, call `self._obs.emit(...)`, defaulting to `_observation.emit`, the same lazy
    import the class already uses. Declare the measure `dev-berth-events@1` wherever the measure
    vocabulary lives (find it with `epr flow note --measure` refusals).
  - Verify: `python3 -m unittest genesis.agentic.berth_test -k fold` passes. Run one live
    `berth claim cargo` plus `berth release cargo`, then show both events with
    `epr flow report --measure dev-berth-events@1`. Add a DELTA to `ledgers-fold-once.habit.md`;
    the status stays red.

## Lane 2: admission

- [ ] **T2 — the PeerEndpoint roster row and `epr actor device endpoint`** (serves devices-share-one-berth)
  - Tests first, in `elohim/epr-rea/src/participants.rs`:
    - `peer_endpoint_signing_message_is_stable`: the message bytes are pinned in a fixture.
    - `peer_endpoint_valid_for_genesis_device` and `peer_endpoint_valid_for_bound_controller`.
    - `peer_endpoint_refused_for_unbound_device` and `peer_endpoint_refused_after_binding_contest`.
    - `newest_peer_endpoint_supersedes`.
  - Implement the variant
    `ParticipantRow::PeerEndpoint { handle, chain_root, device, node_id, nonce, signature }`. Serde
    follows the enum's existing `tag = "kind"` and camelCase.
  - Implement `pub fn admitted_endpoints(rows: &[ParticipantRow], chain_root: &str) -> BTreeMap<String /*node_id*/, String /*device did*/>`.
    It applies the validity rules above, and contested or unbound devices are absent from its result.
  - Add the verb `epr actor device endpoint --node-id <id>` in `elohim/eprfs/epr-cli/src/actor.rs`,
    beside `binding_row` at `:1806`. It signs with this device's key and appends to
    `.eprfs/status/participants/<handle>.jsonl`. Test it in epr-cli: `device_endpoint_appends_one_signed_row`.
  - Verify: `just gate eprfs` passes. The rust-style report shows paydown in `epr-rea` and `epr-cli`.

## Lane 3: the device peer

- [ ] **T3 — the eprfs-peer crate: key, endpoint, admission** (serves devices-share-one-berth)
  - Create `elohim/eprfs/eprfs-peer/` and add it to the workspace `members` in
    `elohim/eprfs/Cargo.toml`. Give it a `.epr-meta` binding `rust-style-strict@1`, a
    `seam-registry.yaml`, and an entry in `seam-catalog.yaml` beside eprfs-serve.
  - Tests first:
    - `key_generated_once_with_mode_0600`.
    - `key_reused_across_restarts`.
    - `key_never_in_debug_output`: the Debug impl is redacted.
    - `admission_maps_node_id_to_device`, built on `admitted_endpoints` from T2.
  - Implement:
    - `pub struct PeerKey`, with `load_or_generate(dir: &Path) -> Result<PeerKey, PeerError>`.
    - `pub struct Admission`, with `from_roster(rows, chain_root) -> Admission`,
      `admits(&self, node: &iroh::NodeId) -> Option<&str>` and `reload(&mut self, rows)`.
    - `pub async fn bind(key: &PeerKey, relay: Option<Url>, alpns: Vec<Vec<u8>>) -> Result<iroh::Endpoint, PeerError>`.
  - Verify: `just gate eprfs` passes, `bash genesis/orchestrator/rust-style.sh --scope elohim/eprfs/eprfs-peer`
    reports strict files with 0 debt, and `seam_matrix_test.py` passes.

- [ ] **T4 — the tier ALPN with StewardDevicesGate** (serves devices-share-one-berth)
  - Tests first, in `eprfs-peer/tests/tier_alpn.rs`, using two in-process endpoints over iroh's
    test relay or direct addresses:
    - `admitted_device_reads_and_diffs_a_root`.
    - `stranger_gets_uniform_refusal_for_held_and_unknown_roots`, mirroring eprfs-serve's
      `an_unadmitted_requester_cannot_distinguish_private_and_unknown_roots`.
    - `contested_device_refused_after_admission_reload`, which covers review focus item 3.
  - Implement:
    - `pub const TIER_ALPN: &[u8] = b"/elohim/eprfs-tier/1.0.0"`.
    - `pub struct StewardDevicesGate`, implementing `eprfs_core::ReachGate`. Its `admit_root` and
      `admit_object` admit only a `Requester::authenticated(device_did, …)` that `Admission` produced.
    - `pub struct TierProtocol`, which implements iroh's `ProtocolHandler` over
      `TierEndpoint<StewardDevicesGate>`. Its frames are length-prefixed dag-cbor of eprfs-serve's
      existing request and answer types. Where those types lack `Serialize`/`Deserialize`, derive
      them in their home crate; that touch must pay down one finding.
  - Verify: `just gate eprfs` passes, including the existing `eprfs-serve/tests/endpoint.rs`,
    which must be unchanged.

## Lane 4: the channel

- [ ] **T5 — channel tree, signed heads, gossip and `epr flow fold`** (serves devices-share-one-berth)
  - Tests first, in `eprfs-peer`:
    - `channel_tree_holds_only_allowed_kinds`, which covers review focus item 5.
    - `head_signature_verifies_against_device_did` and `forged_head_dropped_and_counted`.
    - `lower_seq_head_refused_as_rollback`, which covers review focus item 4.
    - `fold_is_idempotent_by_event_cid`.
    - `fold_records_via_device_provenance`.
    - `folding_a_newer_head_reannounces_with_updated_ack`: after folding B's head with seq `s`, A's
      next head carries `folded[B] == s`.
  - Implement:
    - `pub fn project_channel(store: &SidecarFlowStore, allow: &CrossingAllowList) -> Result<BlobCid, PeerError>`.
    - `pub struct ChannelHead { device, root, seq, at, folded: BTreeMap<String, u64>, signature }`,
      with `sign(...)` and `verify(&self, device_did: &str) -> Result<(), PeerError>`. `folded` is
      inside the signed bytes.
    - `pub struct HeadTable`: per device, the last folded `seq`, `root`, head `at`, its `folded`
      vector, and the local `folded_at`. It is persisted as JSON under the peer state dir, and T6
      reads it.
    - `pub fn topic_for(chain_root: &str) -> iroh_gossip::proto::TopicId`, which is blake3 of
      `"elohim/steward-channel/1/" ‖ chain_root`.
    - `pub struct Channel`, holding announce, debounce (1 s), re-announce (30 s) and a per-device
      last-folded `(seq, root)`.
    - Read the allow-list from `pool-policy.json` at `berth.channel.crosses`.
  - Add the verbs `epr peer serve`, `epr peer status` and `epr flow fold --from <device> --root <cid>`
    in epr-cli. The fold verb appends events through `SidecarFlowStore::transaction`
    (`elohim/epr-rea/src/store.rs:224`).
  - Verify: `just gate eprfs` passes. A two-daemon smoke in one container (two `XDG_CONFIG_HOME`s,
    two `BERTH_DIR`s, two fixture device keys bound under a test roster) shows each side's
    `epr peer status` listing the other's head within 5 s.

## Lane 5: berth across devices

- [ ] **T6 — steward-scoped resources and the lease view** (serves devices-share-one-berth)
  - Tests first, in Rust (`eprfs-peer`, or `epr-cli` if the fold lives there):
    `steward_lease_view` covers six cases:
    - Every other device's head acknowledges the claim's seq: the lease is `Effective`.
    - A device has not acknowledged: `Pending` before `confirm_wait_s` has passed, then
      `Unconfirmed { device, last_seen }`. This is review focus item 1.
    - A folded overlapping lease: `HeldBy { device, session, since }`.
    - A concurrent pair: the tie-break holds and both sides compute the same winner. This is review
      focus item 2.
    - A folded lease from a device whose binding is contested is ignored (spec §3.1, amended).
    - No other admitted device: `Effective` at once.
  - Implement `pub fn steward_lease_view(own: &[LeaseEvent], folded: &[LeaseEvent], heads: &HeadTable, admitted: &Admission, now, confirm_wait_s) -> BTreeMap<Resource, LeaseVerdict>`.
    Expose it as `epr peer leases --json`. A berth claim polls it, in-process or at short intervals,
    until the verdict leaves `Pending` or `confirm_wait_s` (10 s, `berth.channel.confirm_wait_s`) has
    passed.
  - In `genesis/agentic/bin/berth`:
    - Resources declared `scope: steward` in `pool-policy.json` call `epr peer leases --json` on
      claim and on `owner-check`.
    - `Unconfirmed` and `HeldBy` exit 3 with the spec's wording.
    - `--offline-ok` takes the lease and writes `kind: override`.
    - With no other admitted device, the lease is effective at once.
  - Tests in `berth_test.py` use a stub `epr`:
    `test_steward_claim_unconfirmed_names_device`, `test_steward_claim_held_names_holder`,
    `test_offline_ok_writes_override`, `test_single_device_steward_claim_effective` and
    `test_container_scope_never_calls_peer`.
  - Declare `dev-integration`, `jenkins-dispatch`, `alpha-fleet` and the `lane:` prefix as
    `scope: steward` in `genesis/agentic/pool-policy.json`. Set `berth.channel.freshness_s: 90` and `berth.channel.confirm_wait_s: 10`.
  - Verify: `python3 -m unittest genesis.agentic.berth_test` passes, `just gate eprfs` passes, and
    the orchestrator gate passes (`pool-policy.json` is shared with `rust-toolchain-pin.test.mjs`).

- [ ] **T7 — integration through the channel: pre-push, build tags, status** (serves devices-share-one-berth)
  - `.husky/pre-push.bash`, for a push to `dev` or `main`:
    - Run `berth claim dev-integration --class verify --ttl 1800` before the gates, and release it
      on exit (trap).
    - Exit 3 refuses the push and prints berth's reason line.
    - Exit 4 (no session) proceeds unleased and says so.
    - A `[build:edge]` in the pushed commits also needs `alpha-fleet`; any `[build:*]` also needs
      `jenkins-dispatch`.
  - `berth status` adds a section for other devices: moorings, steward leases and lane claims.
  - Tests use the pre-push harness pattern the hook already has (find it under `.husky/` or
    `genesis/orchestrator/*.test.mjs`): `prepush_refuses_while_peer_holds_dev_integration`,
    `prepush_releases_on_gate_failure` and `build_edge_requires_alpha_fleet`.
  - Verify: the hook tests pass, and the a2o scenarios from T0 lose `@wip` and pass with
    `cd genesis/a2o && pnpm exec cucumber-js --tags '@concern:devices-share-one-berth and not @requires:shem'`.
    Add a DELTA to the habit atom. It flips green only on the T8 receipt.

## Lane 6: across the two workspaces

- [ ] **T8 — the cross-workspace receipt** (serves devices-share-one-berth) @requires:shem
  - Add a devfile command `peer-serve`, on demand and never in postStart, to both workspace devfiles.
  - On each workspace: `epr actor device endpoint`, then commit and push the roster row, then
    `epr peer serve`.
  - Run the `@requires:shem` scenario. Store the receipt under
    `genesis/a2o/reports/recovery/devices-share-one-berth/`.
  - Verify: the graduation trigger in spec §1 holds as written. The habit flips green with the
    receipt as its evidence.

---

## Complementary work (backlog, not this plan)

- `genesis/data/timeline/backlog/ci-as-steward-channel-participant.md`: Jenkins writes build
  verdicts as crossing events, and the orchestrator honours `jenkins-dispatch`.
- `genesis/data/timeline/backlog/workspace-artifacts-over-steward-link.md`: the second plan (spec §8),
  rakia key and closures on the same ALPN.
- `genesis/data/timeline/backlog/che-background-shell-git-and-toolchain-env.md`: background shells
  lack `RUSTUP_HOME`, `PATH` and `safe.directory`, so gates refuse or report UNMEASURED.
