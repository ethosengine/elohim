# Multi-peer test harness — option (i) vs (ii) investigation

**Sprint:** light-up-the-topology (T16/T17/T18)
**Question:** drive multi-peer protocol tests in Eclipse Che, or route through Jenkins?
**Recommendation:** **(i) — Jenkins for mesh integration; locally, extend the *already-existing* in-process libp2p harness for any 2-peer wire-level tests T16–T18 need.** Do **not** build a new mpsc substrate.

---

## 1. Existing seams in `elohim/elohim-storage/src/p2p/`

`P2PNode` is constructed in `mod.rs:1462-1504`. The libp2p `Swarm` is **baked through the API surface**: a field on the struct (`mod.rs:420` — `swarm: Arc<RwLock<Swarm<ElohimStorageBehaviour>>>`), built from `SwarmBuilder::with_existing_identity().with_tokio().with_tcp(...).with_dns().with_relay_client(...)`. No `Transport`/`NetworkBehaviour` parameter on `P2PNode::new`. **You cannot swap libp2p out at the `P2PNode` boundary.**

What you *can* swap — what existing tests already swap:

- **`P2PHandle::for_testing()`** (`mod.rs:914-975`) drains `P2PCommand`s with stub replies. The seam every single-peer test uses.
- **`P2PCommand` enum** (`mod.rs:678-820`) — each outgoing action is a variant with an oneshot reply, including `FetchBlob { peer_id, hash, reply }` (`mod.rs:817`) and `TriggerCustodyReconcile`.
- **`reconcile::custody`** (`src/reconcile/custody.rs:18-37`) defines `LocalBlobStore` + `FetchKicker` traits *explicitly so the reconcile pass can be unit-tested without a real blob store or swarm*.
- **Wire codecs are pure**: `BlobInventorySnapshot/Delta` (`inventory_gossip.rs:30,45`), `BlobFetchRequest/Response` (`blob_protocol.rs:58,70`), the projection writer — all callable in-process.
- **A two-peer in-process libp2p harness already exists.** `tests/harness/mod.rs` (806 lines) and `tests/harness_d8/mod.rs` (772 lines) spawn real `Swarm`s on `/ip4/127.0.0.1/tcp/0`, dial each other, run real request-response over loopback. Used today by `epr_atom_federation_integration.rs`, `epr_atom_federation_d8.rs`, `aunt_and_rage_bait_integration.rs`, `manifest_resolver_integration.rs`. **The premise that "Eclipse Che can't run libp2p meshes locally" is wrong** — loopback TCP libp2p works in Che today, every commit.

## 2. What T16/T17/T18 actually require

From `genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md:296-300, 333-339`:

- **T16 (custody reconciliation controller)** — test plan: idempotency, multi-trigger fires, simulated fetch-success → `serve-blob`, gap-grace → `placement-gap`, cooldown suppresses re-emission. **All single-peer**: pre-seed `peer_blob_inventory` rows, call `reconcile_pass(LocalBlobStore, FetchKicker)`, assert the `FetchKicker` was invoked / events were written.
- **T17 (GET-time blob fallback)** — test plan: local-hit, peer-iteration over candidate list, hash verification, 404-on-all-miss. **Single-peer**: pre-seed inventory, mock `P2PCommand::FetchBlob` reply. The race-3-peers logic can be unit-tested with three scripted oneshots.
- **T18 (filesystem parity sweep)** — test plan: parity sweep detects mismatch; broadcast scheduler corrects on next snapshot. **Single-peer**: assert `compute_parity(LocalInventory)` (`p2p/inventory_broadcaster.rs:235`) over a constructed pair (filesystem set, last-gossiped set).

The design doc itself is explicit (`blob-custody-reconciliation-design.md:269,300`): *"Multi-peer integration tests in Eclipse Che: deferred to Jenkins per `feedback_shift_measure_jenkins`. Local TDD uses unit-level tests against mocked P2P channels and a unit-mockable reconciliation pass."*

The answer to the question "(a), (b), or (c)?" is **(b) for unit tests** — one peer with a scripted reply where the wire would be — **plus (a) for any wire-level cross-peer regression**, using the *existing* `tests/harness/mod.rs` pattern (real libp2p, loopback TCP). T16/T17/T18 do **not** need a third option.

## 3. Crate ecosystem — honest survey

| Crate | Maintenance | libp2p compat? | Solves *this*? |
|---|---|---|---|
| `madsim` | Active (v0.2.32, 2024–25) | No libp2p shim. Rewrites the async runtime; libp2p expects real `tokio::net`. | No — heroic graft. |
| `turmoil` | Active (tokio-rs, 2025 commits) | TCP/UDP-socket simulator; no libp2p adapter exists. | No — sockets without an adapter is just sockets. |
| `shuttle` | Active | Concurrency-permutation testing; not network simulation. | No. |
| `netsim` | Last commit 2020; `netsim2` quiet. | None. | No — unmaintained, don't recommend. |

**libp2p version:** `elohim/elohim-storage/Cargo.toml:148` and `steward/node/Cargo.toml:16` are both on **0.54** (CLAUDE.md's "0.53 vs 0.54" line is stale; fix separately). T16–T18 live in `elohim-storage` — no split in this scope. **No sim crate solves this for libp2p 0.54.** The only field-tested local pattern is `tests/harness/mod.rs`: real Swarms, loopback TCP, in-process drivers.

## 4. Cost of (ii) — the bespoke mpsc substrate

A parallel `TestCluster` that wires N `P2PNode`s through in-memory mpsc:

- **Build:** 5–9 engineering days (±3, skewed long). The work is not channel plumbing — it is faking `request_response` correlation, gossipsub topic semantics, `ConnectionEstablished/Closed`, and `OutboundFailure` taxonomy with enough fidelity that reconcile triggers fire in order.
- **Maintenance drift:** structurally bad. Phase 3 view-federation (`view_federation.rs`, 386 lines, `pending_view_federations` at `mod.rs:500`) forces a new shim arm; every future gossipsub topic likewise. libp2p 0.55 is the *first* thing that breaks, because the fake shadows internal event shapes; real-loopback harness recompiles.
- **False-positive coverage:** worst class. A reconcile pass that races correctly against the fake but deadlocks against real `OutboundRequestId` reuse passes green locally and fails only in production.

Extending `tests/harness/mod.rs` instead — add `request_response::Behaviour<BlobFetchCodec>` and `gossipsub::Behaviour` to `HarnessBehaviour` (`mod.rs:51-56`) — is **0.5–1.5 days**, uses the production libp2p, and 0.55 is a single `cargo update`.

## 5. Failure modes

- **(i) pure-Jenkins:** sequence-gap detection (delta sequencing in `inventory_gossip`), placement-gap-after-grace timing, T17 race-and-cancel are first exercised against an actual gap on Jenkins (~10–25 min/attempt). Bugs that hide: out-of-order delta apply, snapshot-replaces-delta racing, cancellation-leaks-pending-fetch. Slow feedback, but truth.
- **(ii) bespoke mpsc fake:** bugs that hide are the ones the fake doesn't model — yamux substream backpressure, request-response-timeout × `OutboundFailure::DialFailure` interplay, gossipsub mesh-grafting under churn. **Strictly worse failure class:** (i) is slow truth; (ii) is fast falsehood.
- **Hybrid (recommendation):** real libp2p loopback for the wire-level T16–T18 paths a unit test cannot reach. Same risk profile as the existing EPR-atom integration tests we already trust.

## 6. Recommendation — option (i), with a hybrid condition

**Adopt option (i): defer multi-peer mesh integration to Jenkins, as the sprint design already specifies.** Specifically:

1. **Default to single-peer unit tests** against `LocalBlobStore` + `FetchKicker` (`reconcile/custody.rs:29-37`) and `P2PHandle::for_testing()` with scripted `P2PCommand::FetchBlob` replies. Sufficient for the design doc's listed test-plan checkboxes.
2. **For residual wire-level paths** (sequence-gap, race-and-cancel, parity-after-rebroadcast), extend `tests/harness/mod.rs:51-56` with `request_response::Behaviour<BlobFetchCodec>` and `gossipsub::Behaviour` for `INVENTORY_TOPIC`. Reuse the existing two-peer driver. **0.5–1.5 days, no new crate, no new substrate.**
3. **Do not build the bespoke mpsc cluster.** It buys nothing the existing harness doesn't already provide and ships a long-lived maintenance liability.

What we give up: full mesh dynamics (≥3 peers, churn, real NAT/relay, identify-roundtrip reordering at scale) — those stay on Jenkins, where they already are.

### Sketch of harness extension API (extend, do not replace)

Add to `tests/harness/mod.rs` alongside the existing `spawn_test_node` / `connect`:

```rust
// Extension to existing HarnessBehaviour:
//   epr_atom + identity_handshake + identify  ← already there
//   blob_fetch:        request_response::Behaviour<BlobFetchCodec>
//   inventory_gossip:  gossipsub::Behaviour

let a = spawn_test_node("a").await;       // existing
let b = spawn_test_node("b").await;       // existing
harness::connect(&a, &b).await;            // existing in harness_d8

// New verbs (T16/T17/T18 surface only):
a.publish_inventory_snapshot(vec!["sha256-..".into()]).await;
b.advance_until_inventory_for(a.peer_id(), Duration::from_secs(2)).await;
assert_eq!(b.inventory_for(a.peer_id()), &["sha256-.."]);

let bytes = b.fetch_blob_from(a.peer_id(), "sha256-..").await?;
```

`advance_until_*` blocks on the swarm event loop until a predicate fires or a deadline elapses — same pattern the existing harness uses to await `IdentityBindingApplied`. No new "settled" abstraction.

## Eclipse Che image — proposed changes

Read of `che-devworkspaces/containers/rust-dev/Dockerfile` and `containers/udi-plus/Dockerfile`: Rust toolchain and clang/openssl dev headers are present; loopback `127.0.0.1` is what `tests/harness/mod.rs` listens on today. **No image change requested** — no apt packages, no sysctls, no toolchain components needed to extend the existing harness. (If gossipsub-mesh tests need more headroom under CI load later, raising `net.core.somaxconn` would help, but it is not load-bearing for T16–T18.)

## Conditions on this recommendation

- The sprint accepts that sequence-gap, race-and-cancel, and parity-after-rebroadcast paths get their first **timing-faithful** exercise on Jenkins (option (i)) and, where added, on the extended harness (real libp2p, loopback TCP).
- We do not invest in `madsim`/`turmoil`/`shuttle`/`netsim` and do not build a bespoke mpsc fake.
- If a future task genuinely needs ≥3-peer mesh churn locally, revisit — but that task is not in this sprint, and the answer is still likely "Jenkins" rather than "rebuild the wire stack in mpsc."
