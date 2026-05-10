# Gossip Dual-Publish Catalog

This file catalogs every gossip publish call site in `elohim-storage`, as required by Plan 4
(iroh gossip dual-publish, cutover gate #4). All sites are wired through `DualGossipPublisher`
which fans out byte-identical payloads to both the `LibP2PGossipPublisher` (libp2p-gossipsub)
and `IrohGossipPublisher` (iroh-gossip) transports. Topic-id derivation for the iroh side is
`BLAKE3(topic_name)[..32]` via `IrohGossip::topic_id_for` (`src/p2p_iroh/gossip.rs:54`).

Per the complementarity spec, the gossip plane is **permanent dual-stack**: both transports
MUST publish and both sides receive byte-identical MessagePack payloads.

---

## Publisher Sites (12 total)

| # | Topic | Producer call site | Wire payload type | `to_bytes` flavor |
|---|---|---|---|---|
| 1 | `elohim/inventory/blob` | `P2PNode::broadcast_inventory_snapshot` — `src/p2p/mod.rs:2136-2205` (publish at `:2180`) | `BlobInventorySnapshot` (`src/p2p/inventory_gossip.rs:30`) | `to_vec_named` |
| 2 | `recovery.invitation` | `P2PCommand::PublishRecoveryInvitation` arm — `src/p2p/mod.rs:2368-2393` (publish at `:2371`) | `RecoveryInvitation` (`src/p2p/recovery_invitation.rs:22`) | `to_vec_named` |
| 3 | `elohim/identity/binding` | `P2PCommand::PublishIdentityBinding` arm — `src/p2p/mod.rs:2398-2426` (publish at `:2403`) | `IdentityBindingGossip` (`src/p2p/identity_binding_gossip.rs:48`) | `to_vec` (positional) |
| 4 | `recovery.revocation` | `P2PCommand::PublishRecoveryRevocation` arm — `src/p2p/mod.rs:2427-2453` (publish at `:2430`) | `RecoveryRevocationMessage` (`src/p2p/recovery_revocation.rs`) | `to_vec_named` |
| 5 | `elohim/<pillar>/<reach>[/coll]` (reach-scoped) | `P2PCommand::PublishEprAnnounce` arm — `src/p2p/mod.rs:2493-2509` (publish at `:2495`) | MessagePack-encoded CID (`Vec<u8>`, opaque) | caller-encoded |
| 6 | `/elohim/feedback-signal/{target_cid}` | `P2PCommand::GossipPublish` arm — `src/p2p/mod.rs:2589-2605` (publish at `:2591`) | `FeedbackSignal` (`src/p2p/feedback_signal.rs`) | `to_vec_named` |

## Producer Call Chains (upstream senders filling the command queue)

| # | Producer | Sends `P2PCommand::*` | File:line |
|---|---|---|---|
| 7 | `RecoveryProjector::on_recovery_request_created` | `PublishRecoveryInvitation` | (sender; see signals.rs doc-anchor `src/signals.rs:867`) |
| 8 | `ReconcileController::on_agent_peer_binding` | `PublishIdentityBinding` | `src/reconcile/controller.rs:601` |
| 9 | `RecoveryProjector::on_key_revocation_*` (Step 8 producer) | `PublishRecoveryRevocation` | `src/p2p/mod.rs:1179` (and signals.rs anchors `:899` `:904`) |
| 10 | `services::epr_store::FederatedEprStore::put` (D.3 fanout) | `PublishEprAnnounce` | `src/services/epr_store.rs:487` |
| 11 | `LibP2PGossipPublisher::publish` (adapter for `flood_feedback`) | `GossipPublish` | `src/p2p/adapters.rs:88-103` |
| 12 | `crate::api::epr::put_epr` calls `flood_feedback` which calls `LibP2PGossipPublisher` | `GossipPublish` | `src/api/epr.rs:719-735` |

## Subscriber-Only Topics (no publisher today)

- `elohim/integrity/revocation` (`TOPIC_INTEGRITY_REVOCATION`, `src/p2p/topics.rs:52`) — subscribed at
  `src/p2p/behaviour.rs:476`; no producer wired yet. Plan task 7 introduces the dual-publish path so
  future producers (Recovery epic M4 follow-on) drop into the same fan-out without further plan work.

---

## Subscriber-Side Parity (receive-path seam)

Existing libp2p subscribers (in `src/p2p/mod.rs:4396-4607`) consume `gossipsub::Event::Message`
and decode via `from_bytes()`. The iroh-side receive path is established by the
`IrohGossipPublisher::spawn` background task, which calls `IrohGossip::subscribe` on first publish
per topic — storing both sender and receiver per topic. Routing received bytes from iroh into the
existing libp2p-shaped receive handlers is a follow-up task (out-of-scope for Plan 4). For cutover
gate #4, the requirement is **publish parity**; the subscribe-side is exercised end-to-end by
Task 8 (cross-stack soak).

The iroh ALPN (`iroh_gossip::ALPN`) is already mounted in `IrohNode::start_with_protocols`
(`src/p2p_iroh/node.rs`), so the receive-side transport is ready for wiring.

## Integrity-Revocation Future-Producer Hook

When a future producer is added (Recovery epic M4 follow-on), it will send a new
`P2PCommand::PublishIntegrityRevocation(payload)` whose arm body uses:

```rust
self.gossip_publisher.publish(TOPIC_INTEGRITY_REVOCATION, payload.to_bytes()?)
```

The dual-publish path (via `DualGossipPublisher`) requires no further plan work for this topic —
the `classify_topic` verdict table already classifies `"elohim/integrity/revocation"` as `Dual`.
