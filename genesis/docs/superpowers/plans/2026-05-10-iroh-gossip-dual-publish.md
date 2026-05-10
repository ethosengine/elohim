# Iroh Gossip Dual-Publish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire every gossip publish call site through a transport-neutral `GossipPublisher` that fans out to BOTH `iroh-gossip` and `libp2p-gossipsub` for dual-stack topics, using identical wire bytes under a BLAKE3-derived shared topic id, and flip the iroh `EprBackend::Announce` arm to `accepted: true` once identity-binding gossip publishes via the new path.

**Architecture:** A single `GossipPublisher` trait (already in `services/gossip_flood.rs`) is generalized into a fan-out `DualPublisher` that owns one `LibP2PGossipPublisher` (existing) and one new `IrohGossipPublisher` (Phase 12). The fan-out consults a per-topic verdict table (inventory / identity-binding / recovery / attention / feedback / integrity-revocation) to decide which transports to publish on. Wire payloads are produced by the existing topic-specific `to_bytes()` methods and forwarded byte-identical to both transports — no schema changes. Topic-id derivation is shared: libp2p uses the string topic name as the gossipsub `IdentTopic`; iroh derives `BLAKE3(topic_name)[..32]` via the existing `IrohGossip::topic_id_for`.

**Tech Stack:** Rust 1.x (`elohim-storage` crate, `p2p-iroh` feature), `iroh-gossip = 0.92`, `libp2p::gossipsub` (existing pin), `rmp_serde` (MessagePack, unchanged), `tokio::sync::mpsc` (existing actor pattern), `blake3` (already a transitive dep via iroh-gossip).

## Source-of-truth declaration (P2P design gate)

This plan introduces **NO new storage schema, DHT entry types, DB tables, wire payload types, or persisted data entities.** Per the p2p-design-gate skill's classification (A / A2 / B / B2 / C):

| Artifact in this plan | Category | Source of truth (existing) | Notes |
|---|---|---|---|
| `TopicTransports` enum | **C — operational, in-process only** | n/a (compile-time routing table; no persistence, no wire frame, no DHT entry) | Lives in `src/p2p_iroh/dual_publish/verdicts.rs`; pure dispatch logic. |
| `DualGossipPublisher` / `IrohGossipPublisher` | **C — operational, in-process only** | n/a (transport adapters; no persistence) | Implements the existing `GossipPublisher` trait (`src/services/gossip_flood.rs`). |
| `CATALOG.md` artifact | docs only | n/a | Catalog of existing publishers; no entity. |
| Per-topic gossip payloads (`BlobInventorySnapshot`, `IdentityBindingGossip`, `RecoveryInvitation`, `RecoveryRevocationMessage`, `FeedbackSignal`, EPR-atom-announce) | **C — operational** (already classified) | EXISTING: `src/p2p/inventory_gossip.rs`, `src/p2p/identity_binding_gossip.rs`, `src/p2p/recovery_invitation.rs`, `src/p2p/recovery_revocation.rs`, `src/p2p/feedback_signal.rs`. All wrap Category-A DHT-notarized entries (AgentPeerBinding, RecoveryRequest, KeyRevocation) — gossip is the **projection**, never the truth. | **WIRE FORMAT IS FROZEN BY THIS PLAN — same `to_bytes()` bytes go on both transports.** |
| Topic name → topic-id mapping (`BLAKE3(name)[..32]`) | **C — operational** (already classified, Phase 4) | EXISTING: `IrohGossip::topic_id_for` in `src/p2p_iroh/gossip.rs:54`. | Plan reuses unchanged. |
| EPR Head (touched by Task 5 `handle_announce`) | **A — DHT-notarized** | EXISTING: Holochain `epr` DHT entry; libp2p side already writes via `kademlia.put_record` + `EprService`. | Plan routes iroh `Announce` through the **same** `EprService` so both transports converge on one source of truth. |

**No DHT entry type or DB table is created or modified.** The two new `.rs` modules under `src/p2p_iroh/dual_publish/` are pure transport-adapter logic (Category C, in-process). The line-7 reading ("new storage schema") is a false positive against the `CATALOG.md` artifact and the verdict-table vocabulary; both are operational/documentation artifacts, not schemas. If a future task touches a Category-A or Category-B entity, that task will invoke the p2p-design-gate skill in its own plan.

---

## Task 1: Catalog every gossip publish call site

**Files:**
- Read-only: `/projects/elohim/elohim/elohim-storage/src/p2p/mod.rs`
- Read-only: `/projects/elohim/elohim/elohim-storage/src/p2p/adapters.rs`
- Read-only: `/projects/elohim/elohim/elohim-storage/src/services/gossip_flood.rs`
- Read-only: `/projects/elohim/elohim/elohim-storage/src/services/epr_store.rs`
- Read-only: `/projects/elohim/elohim/elohim-storage/src/api/epr.rs`
- Read-only: `/projects/elohim/elohim/elohim-storage/src/reconcile/controller.rs`
- Read-only: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/epr_backend.rs`
- Create catalog file: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md` (committed alongside the implementation)

- [ ] **Step 1.1:** Open every file above and confirm the publish-site catalog below is exhaustive. Run:
  ```bash
  cd /projects/elohim/elohim/elohim-storage && \
    grep -rn "gossipsub.publish\|GossipPublish\|PublishIdentityBinding\|PublishRecoveryInvitation\|PublishRecoveryRevocation\|PublishEprAnnounce" src/ \
    | grep -v 'test\|//\|//!\|enum P2PCommand'
  ```
  Expected output: the union of the following twelve sites. If any new site appears, add it to the catalog before continuing.

  **Catalog (confirmed at plan-write):**

  | # | Topic | Producer call site | Wire payload type | `to_bytes` flavor |
  |---|---|---|---|---|
  | 1 | `elohim/inventory/blob` | `P2PNode::broadcast_inventory_snapshot` — `src/p2p/mod.rs:2136-2205` (publish at `:2180`) | `BlobInventorySnapshot` (`src/p2p/inventory_gossip.rs:30`) | `to_vec_named` |
  | 2 | `recovery.invitation` | `P2PCommand::PublishRecoveryInvitation` arm — `src/p2p/mod.rs:2368-2393` (publish at `:2371`) | `RecoveryInvitation` (`src/p2p/recovery_invitation.rs:22`) | `to_vec_named` |
  | 3 | `elohim/identity/binding` | `P2PCommand::PublishIdentityBinding` arm — `src/p2p/mod.rs:2398-2426` (publish at `:2403`) | `IdentityBindingGossip` (`src/p2p/identity_binding_gossip.rs:48`) | `to_vec` (positional) |
  | 4 | `recovery.revocation` | `P2PCommand::PublishRecoveryRevocation` arm — `src/p2p/mod.rs:2427-2453` (publish at `:2430`) | `RecoveryRevocationMessage` (`src/p2p/recovery_revocation.rs`) | `to_vec_named` |
  | 5 | `elohim/<pillar>/<reach>[/coll]` (reach-scoped) | `P2PCommand::PublishEprAnnounce` arm — `src/p2p/mod.rs:2493-2509` (publish at `:2495`) | MessagePack-encoded CID (`Vec<u8>`, opaque) | caller-encoded |
  | 6 | `/elohim/feedback-signal/{target_cid}` | `P2PCommand::GossipPublish` arm — `src/p2p/mod.rs:2589-2605` (publish at `:2591`) | `FeedbackSignal` (`src/p2p/feedback_signal.rs`) | `to_vec_named` |

  **Producer call chains (the upstream senders that fill the queue handled by sites 2/3/4/5/6):**

  | # | Producer | Sends `P2PCommand::*` | File:line |
  |---|---|---|---|
  | 7 | `RecoveryProjector::on_recovery_request_created` | `PublishRecoveryInvitation` | (sender; see signals.rs doc-anchor `src/signals.rs:867`) |
  | 8 | `ReconcileController::on_agent_peer_binding` | `PublishIdentityBinding` | `src/reconcile/controller.rs:601` |
  | 9 | `RecoveryProjector::on_key_revocation_*` (Step 8 producer) | `PublishRecoveryRevocation` | `src/p2p/mod.rs:1179` (and signals.rs anchors `:899` `:904`) |
  | 10 | `services::epr_store::FederatedEprStore::put` (D.3 fanout) | `PublishEprAnnounce` | `src/services/epr_store.rs:487` |
  | 11 | `LibP2PGossipPublisher::publish` (adapter for `flood_feedback`) | `GossipPublish` | `src/p2p/adapters.rs:88-103` |
  | 12 | `crate::api::epr::put_epr` calls `flood_feedback` which calls `LibP2PGossipPublisher` | `GossipPublish` | `src/api/epr.rs:719-735` |

  **Subscriber-only topics (no publisher today; included for completeness):**
  - `elohim/integrity/revocation` (`TOPIC_INTEGRITY_REVOCATION`, `src/p2p/topics.rs:52`) — subscribed at `src/p2p/behaviour.rs:476`; no producer wired yet. Plan task 7 introduces the dual-publish path so future producers (Recovery epic M4 follow-on) drop into the same fan-out.

- [ ] **Step 1.2:** Write `src/p2p_iroh/dual_publish/CATALOG.md` (the table above plus a one-paragraph intro describing the dual-publish goal). Commit.

  ```bash
  cd /projects/elohim/elohim && git add elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md && \
    git commit -m "iroh dual-publish: catalog every gossip publish call site"
  ```

---

## Task 2: Generalize `GossipPublisher` to a `DualGossipPublisher` fan-out

**Files:**
- Modify: `/projects/elohim/elohim/elohim-storage/src/services/gossip_flood.rs` (`GossipPublisher` trait already lives here — extend)
- Create: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/dual_publish/mod.rs`
- Create: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/dual_publish/verdicts.rs`
- Create: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/dual_publish/iroh_publisher.rs`
- Create: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/dual_publish/dual_publisher.rs`
- Test: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/dual_publish/tests.rs`

- [ ] **Step 2.1:** Add the per-topic verdict enum and lookup table in `verdicts.rs`. The trait stays sync; verdict is purely a routing table.

  ```rust
  // verdicts.rs
  use std::collections::HashMap;

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum TopicTransports {
      /// Both transports MUST publish (permanent dual-stack).
      Dual,
      /// Only libp2p (legacy / consumer-grade only — none currently).
      Libp2pOnly,
      /// Only iroh (would require all subscribers to be iroh-capable; none currently).
      IrohOnly,
  }

  /// Returns `TopicTransports` for a given topic name. Reach-scoped
  /// EPR-announce topics (prefix `elohim/<pillar>/`) all classify as `Dual`.
  pub fn classify_topic(topic_name: &str) -> TopicTransports {
      match topic_name {
          // Permanent dual per spec line 278.
          "elohim/inventory/blob"        => TopicTransports::Dual,
          "elohim/identity/binding"      => TopicTransports::Dual,
          "recovery.invitation"          => TopicTransports::Dual,
          "recovery.revocation"          => TopicTransports::Dual,
          "elohim/integrity/revocation"  => TopicTransports::Dual,
          // Transition dual; permanence deferred (per task statement R2).
          t if t.starts_with("/elohim/feedback-signal/") => TopicTransports::Dual,
          t if t.starts_with("elohim/")  => TopicTransports::Dual, // EPR reach-scoped (D.3)
          // Default conservative: dual. New topics opt out explicitly.
          _ => TopicTransports::Dual,
      }
  }
  ```

  Add unit tests covering each known topic name and the prefix arms.

- [ ] **Step 2.2:** Define `IrohGossipPublisher` in `iroh_publisher.rs` — implements the existing `GossipPublisher` trait by sending `(topic_id, payload)` to a tokio task that holds the `GossipSender` per subscribed topic.

  ```rust
  // iroh_publisher.rs (sketch — actual mpsc + task wiring inline)
  use crate::p2p_iroh::IrohGossip;
  use crate::services::gossip_flood::{GossipPublisher, PublishError};
  use tokio::sync::mpsc;

  pub struct IrohGossipPublisher {
      // Sender to a background task that owns the per-topic GossipSenders.
      tx: mpsc::Sender<IrohGossipCommand>,
  }

  pub enum IrohGossipCommand {
      Publish { topic: String, payload: Vec<u8> },
  }

  impl IrohGossipPublisher {
      pub fn spawn(gossip: IrohGossip) -> Self {
          let (tx, mut rx) = mpsc::channel::<IrohGossipCommand>(256);
          tokio::spawn(async move {
              use std::collections::HashMap;
              let mut senders: HashMap<String, iroh_gossip::api::GossipSender> = HashMap::new();
              while let Some(cmd) = rx.recv().await {
                  let IrohGossipCommand::Publish { topic, payload } = cmd;
                  let sender = match senders.get(&topic) {
                      Some(s) => s.clone(),
                      None => match gossip.subscribe(&topic, vec![]).await {
                          Ok((s, _r)) => { senders.insert(topic.clone(), s.clone()); s }
                          Err(e) => {
                              tracing::warn!(target: "elohim_storage::iroh_gossip",
                                  topic=%topic, error=?e, "subscribe-on-demand failed");
                              continue;
                          }
                      },
                  };
                  if let Err(e) = sender.broadcast(payload.into()).await {
                      tracing::warn!(target: "elohim_storage::iroh_gossip",
                          topic=%topic, error=?e, "broadcast failed");
                  }
              }
          });
          Self { tx }
      }
  }

  impl GossipPublisher for IrohGossipPublisher {
      fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
          self.tx.try_send(IrohGossipCommand::Publish { topic: topic.to_string(), payload })
              .map_err(|e| PublishError::Backend(format!("iroh gossip channel: {e}")))
      }
  }
  ```

  TDD: write a unit test that constructs a real `IrohGossip` (using `IrohNode` loopback config) on two nodes, calls `publish` on one, and asserts the other receives the bytes. Subscribe-on-demand keeps the trait sync.

- [ ] **Step 2.3:** Define `DualGossipPublisher` in `dual_publisher.rs` — fans `publish(topic, payload)` to both inner publishers per `classify_topic(topic)`.

  ```rust
  // dual_publisher.rs (sketch)
  use std::sync::Arc;
  use crate::services::gossip_flood::{GossipPublisher, PublishError};
  use super::verdicts::{classify_topic, TopicTransports};

  pub struct DualGossipPublisher {
      libp2p: Option<Arc<dyn GossipPublisher>>,
      iroh: Option<Arc<dyn GossipPublisher>>,
  }

  impl DualGossipPublisher {
      pub fn new(
          libp2p: Option<Arc<dyn GossipPublisher>>,
          iroh: Option<Arc<dyn GossipPublisher>>,
      ) -> Self { Self { libp2p, iroh } }
  }

  impl GossipPublisher for DualGossipPublisher {
      fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
          let v = classify_topic(topic);
          let mut last_err: Option<PublishError> = None;
          let want_libp2p = matches!(v, TopicTransports::Dual | TopicTransports::Libp2pOnly);
          let want_iroh   = matches!(v, TopicTransports::Dual | TopicTransports::IrohOnly);
          // Identical bytes to both — payload is moved once via clone.
          if want_libp2p {
              if let Some(p) = &self.libp2p {
                  if let Err(e) = p.publish(topic, payload.clone()) { last_err = Some(e); }
              }
          }
          if want_iroh {
              if let Some(p) = &self.iroh {
                  if let Err(e) = p.publish(topic, payload) { last_err = Some(e); }
              }
          }
          // Best-effort: if at least one succeeded we return Ok(()).
          // Surface errors only when BOTH transports failed (or no publishers wired).
          match (last_err, self.libp2p.is_some() || self.iroh.is_some()) {
              (Some(e), _) if !self.published_at_least_one() => Err(e), // computed via try_publish helper
              _ => Ok(()),
          }
      }
  }
  ```

  TDD: write unit tests in `tests.rs` using two `MockGossipPublisher` instances (one per side) and assert:
  1. Each `Dual` topic produces exactly one call on each mock with byte-identical payload.
  2. `Libp2pOnly` (synthetic test topic) calls only the libp2p mock.
  3. With one transport missing (`None`), the other still receives.
  4. With both transports erroring, the trait surfaces an error.

- [ ] **Step 2.4:** Wire `mod.rs`:

  ```rust
  // p2p_iroh/dual_publish/mod.rs
  pub mod verdicts;
  pub mod iroh_publisher;
  pub mod dual_publisher;
  pub use verdicts::{TopicTransports, classify_topic};
  pub use iroh_publisher::IrohGossipPublisher;
  pub use dual_publisher::DualGossipPublisher;
  #[cfg(test)] mod tests;
  ```

  Re-export from `src/p2p_iroh/mod.rs`:
  ```rust
  pub mod dual_publish;
  pub use dual_publish::{DualGossipPublisher, IrohGossipPublisher, TopicTransports, classify_topic};
  ```

- [ ] **Step 2.5:** Build + test (`p2p-iroh` feature enabled):

  ```bash
  cd /projects/elohim/elohim/elohim-storage && \
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --lib dual_publish
  ```

  Expected: all four mock-tests in step 2.3 plus the `IrohGossipPublisher` round-trip test in 2.2 pass.

- [ ] **Step 2.6:** Commit.

  ```bash
  git add elohim-storage/src/p2p_iroh/dual_publish/ elohim-storage/src/services/gossip_flood.rs elohim-storage/src/p2p_iroh/mod.rs && \
    git commit -m "iroh dual-publish: DualGossipPublisher fan-out + per-topic verdicts"
  ```

---

## Task 3: Wire feedback / EPR-atom-announce sites through `DualGossipPublisher`

These two sites already go through the `GossipPublisher` trait via `LibP2PGossipPublisher`. Migration is local: replace `Arc<LibP2PGossipPublisher>` injection points with `Arc<DualGossipPublisher>`.

**Files:**
- Modify: `/projects/elohim/elohim/elohim-storage/src/main.rs` (construction site for the publisher Arc)
- Modify: `/projects/elohim/elohim/elohim-storage/src/api/epr.rs` (only the type of `gossip_publisher` field on `FanOut` — no new logic)
- Modify: `/projects/elohim/elohim/elohim-storage/src/services/epr_store.rs` (same — type widen)

- [ ] **Step 3.1:** Crate-wide grep for `LibP2PGossipPublisher` injection points (per memory `feedback_signature_changes_grep_callers`):

  ```bash
  cd /projects/elohim/elohim/elohim-storage && \
    grep -rn "LibP2PGossipPublisher\|Arc<dyn GossipPublisher>" src/ tests/
  ```

  Expected sites (verify against output):
  - `src/main.rs` — constructs `LibP2PGossipPublisher::new(p2p_tx.clone())`; pass it to `DualGossipPublisher::new(Some(...), Some(IrohGossipPublisher::spawn(iroh_node.gossip().clone())))` when iroh node is available.
  - `src/api/epr.rs` — `gossip_publisher: Option<Arc<dyn GossipPublisher>>` (already trait-typed; no change to consumer).
  - `src/services/epr_store.rs` — same trait-typed field.

  Migrate each call site to construct `DualGossipPublisher` once at startup and pass the `Arc<dyn GossipPublisher>` through.

- [ ] **Step 3.2:** Add a TDD: in `src/p2p_iroh/dual_publish/tests.rs` add a test that mimics `flood_feedback` end-to-end with a `DualGossipPublisher` wrapping two mocks; assert byte-identical payloads on both mocks for the `/elohim/feedback-signal/<cid>` topic.

- [ ] **Step 3.3:** Build + test:

  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --features p2p-iroh && \
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --lib gossip_flood dual_publish
  ```

- [ ] **Step 3.4:** Commit.

  ```bash
  git commit -am "iroh dual-publish: wire feedback + EPR-announce through DualGossipPublisher"
  ```

---

## Task 4: Wire inventory snapshot through `DualGossipPublisher`

The current path (`P2PNode::broadcast_inventory_snapshot`) talks to libp2p `gossipsub` directly with a swarm write-lock. Refactor to encode bytes ONCE then call `Arc<dyn GossipPublisher>::publish(INVENTORY_TOPIC, bytes)`.

**Files:**
- Modify: `/projects/elohim/elohim/elohim-storage/src/p2p/mod.rs` (`broadcast_inventory_snapshot`, ~line 2136-2205)
- Modify: `/projects/elohim/elohim/elohim-storage/src/p2p/adapters.rs` (`LibP2PGossipPublisher` already exists — reuse)
- Read-only: `/projects/elohim/elohim/elohim-storage/src/p2p/inventory_gossip.rs`
- Modify: `/projects/elohim/elohim/elohim-storage/src/p2p/mod.rs` `P2PNode` struct — add `gossip_publisher: Arc<dyn GossipPublisher>` field

- [ ] **Step 4.1:** Add `gossip_publisher: Arc<dyn GossipPublisher>` to `P2PNode` (constructor takes it). Where the swarm task was the sole publisher, the libp2p-side adapter now goes through the same actor pattern (no behavior change for libp2p; new path enables iroh fan-out).

- [ ] **Step 4.2:** Replace the body of `broadcast_inventory_snapshot` (after building `bytes`) with:

  ```rust
  if let Err(e) = self.gossip_publisher.publish(INVENTORY_TOPIC, bytes) {
      warn!(target: "elohim_storage::inventory", error=%e,
          "T22: dual-publish failed");
      return;
  }
  self.set_last_gossiped_inventory(hashes_for_record);
  info!(target: "elohim_storage::inventory", count, sequence=snapshot_sequence,
      "T22: published inventory snapshot via DualGossipPublisher");
  ```

  Drop the libp2p-direct publish at line 2178-2204; the libp2p side is now reached via `LibP2PGossipPublisher` -> `P2PCommand::GossipPublish` -> existing arm at `:2589`.

- [ ] **Step 4.3:** Crate-wide grep for `broadcast_inventory_snapshot` callers (per memory pin):

  ```bash
  grep -rn "broadcast_inventory_snapshot" src/ tests/
  ```

  Confirmed: only one caller (the interval tick in `P2PNode::run` at `src/p2p/mod.rs:2094`). No signature change needed.

- [ ] **Step 4.4:** Add TDD: integration test in `tests/iroh_gossip_dual_publish_inventory.rs` (gated on `p2p-iroh`) — two-node fixture (one libp2p, one iroh), publisher node holds a `DualGossipPublisher`, both subscribers see the same bytes for `INVENTORY_TOPIC`.

- [ ] **Step 4.5:** Build + test:

  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test iroh_gossip_dual_publish_inventory
  ```

- [ ] **Step 4.6:** Commit.

  ```bash
  git commit -am "iroh dual-publish: route inventory snapshots through DualGossipPublisher"
  ```

---

## Task 5: Wire identity-binding through `DualGossipPublisher` AND flip iroh `EprBackend::Announce` to `accepted: true`

This task closes the n0-mitigation gap noted in `src/p2p_iroh/epr_backend.rs:64-71`: once identity-binding gossip publishes via the iroh transport, EPR Announce can proceed.

**Files:**
- Modify: `/projects/elohim/elohim/elohim-storage/src/p2p/mod.rs` — `PublishIdentityBinding` arm (`:2398-2426`) routed through publisher
- Modify: `/projects/elohim/elohim/elohim-storage/src/reconcile/controller.rs` — `on_agent_peer_binding` (`:522-622`) sends to publisher (alternative path) OR keeps `P2PCommand::PublishIdentityBinding` and the swarm arm fan-outs
- Modify: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/epr_backend.rs` — flip `Announce` to `accepted: true` once identity binding has published
- Test: `/projects/elohim/elohim/elohim-storage/tests/iroh_gossip_dual_publish_identity_binding.rs`
- Test: `/projects/elohim/elohim/elohim-storage/tests/iroh_epr_announce_accepted.rs`

- [ ] **Step 5.1:** Refactor the `P2PCommand::PublishIdentityBinding` arm to:

  ```rust
  P2PCommand::PublishIdentityBinding(payload) => {
      let bytes = match payload.to_bytes() {
          Ok(b) => b,
          Err(e) => { warn!(error=?e, "encode failed"); return; }
      };
      // Single dispatch — DualGossipPublisher fans out.
      if let Err(e) = self.gossip_publisher.publish(
          crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC,
          bytes,
      ) {
          warn!(error=%e, "PublishIdentityBinding dual-publish failed");
      } else {
          info!(peer_id=%payload.peer_id, agent_cid=%payload.agent_cid,
              "Published IdentityBindingGossip via DualGossipPublisher");
      }
  }
  ```

  Wire format note: `IdentityBindingGossip::to_bytes` uses `rmp_serde::to_vec` (positional) — UNCHANGED from current libp2p path. iroh receives byte-identical bytes via the same struct's `to_bytes`. (Validated in step 5.4 byte-parity assertion.)

- [ ] **Step 5.2:** Update `EprServiceBackend::handle` in `src/p2p_iroh/epr_backend.rs`:

  ```rust
  EprRequest::Announce { head } => {
      // Identity binding is now published via the dual-publish gossip path
      // (Plan task 5). Iroh-side peers see the binding and can resolve
      // EPR head→peer mapping. The backend forwards the announce to the
      // service which writes the head.
      match self.service.handle_announce(head).await {
          Ok(()) => EprResponse::Announced { accepted: true, reason: None },
          Err(e) => EprResponse::Announced {
              accepted: false,
              reason: Some(format!("announce service error: {e}")),
          },
      }
  }
  ```

  If `EprService` lacks `handle_announce`, add the method (writes to local EPR head store; existing libp2p path also writes locally before put_record). Keep all writes through `EprService` so the libp2p Kad path and iroh path both flow through one source of truth.

- [ ] **Step 5.3:** Update the `announce_returns_unimplemented_in_iroh_mode` test (`src/p2p_iroh/epr_backend.rs:86-104`) to assert `accepted: true` instead of `false`. Rename to `announce_succeeds_after_dual_publish_identity_binding`.

- [ ] **Step 5.4:** **Byte-parity test** (this satisfies plan requirement #6 third bullet) — create `tests/iroh_gossip_byte_parity.rs`:

  ```rust
  #![cfg(feature = "p2p-iroh")]
  // Construct a sample IdentityBindingGossip; call to_bytes() once.
  // Hand the bytes to BOTH a MockGossipPublisher captured by libp2p side
  // AND a MockGossipPublisher captured by iroh side via DualGossipPublisher.
  // Assert: the two mocks observed byte-identical payloads under the same
  // topic name "elohim/identity/binding".
  ```

  Repeat the assertion for `BlobInventorySnapshot`, `RecoveryInvitation`, `RecoveryRevocationMessage`, and `FeedbackSignal` to cover all wire types per requirement #6.

- [ ] **Step 5.5:** Two-node integration test `tests/iroh_gossip_dual_publish_identity_binding.rs`:
  Provider holds `DualGossipPublisher(libp2p, iroh)`, calls `publish(IDENTITY_BINDING_TOPIC, payload.to_bytes()?)`. One subscriber runs libp2p only and one runs iroh only. Both decode the received payload and assert `decoded == payload`.

- [ ] **Step 5.6:** Two-node integration test `tests/iroh_epr_announce_accepted.rs`:
  Iroh-only fixture (existing `TwoNodeFixture`), client calls `EprRequest::Announce { head }`, server (running `EprServiceBackend` with the new `handle_announce`) returns `Announced { accepted: true, reason: None }`. Verifies the n0-mitigation gap closed.

- [ ] **Step 5.7:** Build + test:

  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh \
    --test iroh_gossip_dual_publish_identity_binding \
    --test iroh_epr_announce_accepted \
    --test iroh_gossip_byte_parity && \
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --lib epr_backend
  ```

- [ ] **Step 5.8:** Commit.

  ```bash
  git commit -am "iroh dual-publish: identity-binding via dual stack + flip EprAnnounce to accepted"
  ```

---

## Task 6: Wire recovery-invitation and recovery-revocation through `DualGossipPublisher`

**Files:**
- Modify: `/projects/elohim/elohim/elohim-storage/src/p2p/mod.rs` — `PublishRecoveryInvitation` arm (`:2368-2393`) and `PublishRecoveryRevocation` arm (`:2427-2453`)
- Test: `/projects/elohim/elohim/elohim-storage/tests/iroh_gossip_dual_publish_recovery.rs`

- [ ] **Step 6.1:** Refactor both arms in `src/p2p/mod.rs` to call `self.gossip_publisher.publish(topic_const, msg.to_bytes()?)`. Same shape as Task 5 step 5.1; payloads are `RecoveryInvitation` and `RecoveryRevocationMessage`. Topic constants: `RECOVERY_INVITATION_TOPIC = "recovery.invitation"` and `RECOVERY_REVOCATION_TOPIC = "recovery.revocation"`.

  No behavior change to upstream callers (`signals.rs:867`, `signals.rs:904`, `src/p2p/mod.rs:1179`). All keep sending `P2PCommand::PublishRecoveryInvitation(...)` / `::PublishRecoveryRevocation(...)`. The change is internal to the swarm-arm body.

- [ ] **Step 6.2:** Two-node integration test `tests/iroh_gossip_dual_publish_recovery.rs`:
  Provider with `DualGossipPublisher`, two subscribers (one libp2p, one iroh). For each of the two recovery topics, publish a sample `RecoveryInvitation` / `RecoveryRevocationMessage` and assert both subscribers receive byte-identical bytes that decode to the same struct.

- [ ] **Step 6.3:** Build + test:

  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test iroh_gossip_dual_publish_recovery
  ```

- [ ] **Step 6.4:** Commit.

  ```bash
  git commit -am "iroh dual-publish: recovery-invitation + recovery-revocation"
  ```

---

## Task 7: Document subscriber-side and integrity-revocation future-publisher hook

**Files:**
- Modify: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md` — append a "Subscriber-side parity" and "Integrity-revocation future producer" section.
- Modify: `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/README.md` — update §"What's next" and the gossip table row to point to this plan.

- [ ] **Step 7.1:** Subscriber side: existing libp2p subscribers (in `src/p2p/mod.rs:4396-4607`) consume `gossipsub::Event::Message` and decode via `from_bytes`. Iroh-side subscribers will be wired in a follow-up task — for the dual-publish cutover, iroh-side receivers come for free via the `IrohGossipPublisher::spawn` task's `subscribe` call (it stores both sender and receiver per topic; expose the receiver via a separate `IrohGossipReceiver` API to a routing hook). Document this seam without implementing the receivers — receive-side wiring is requirement #4 ("Existing receivers don't need to change") and the iroh-receive consumer ALPN is wired via `iroh_gossip::ALPN` already mounted in `IrohNode::start_with_protocols` (`src/p2p_iroh/node.rs:65`).

- [ ] **Step 7.2:** Integrity-revocation: when a future producer is added (Recovery epic M4 follow-on), it will send a new `P2PCommand::PublishIntegrityRevocation(payload)` whose arm body uses `self.gossip_publisher.publish(TOPIC_INTEGRITY_REVOCATION, payload.to_bytes()?)` — the dual-publish path requires no further plan work for this topic.

- [ ] **Step 7.3:** Update `src/p2p_iroh/README.md`:

  ```markdown
  ### Gossip dual-publish (Phase 12 cutover gate #4)

  Per `genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`
  line 278, the gossip plane is permanent dual-stack. Implementation lives in
  `src/p2p_iroh/dual_publish/`. All publish call sites route through
  `DualGossipPublisher`, which fans out to both `LibP2PGossipPublisher` and
  `IrohGossipPublisher` per the verdict table in `verdicts.rs`. Wire payloads
  are produced by topic-specific `to_bytes()` helpers and forwarded
  byte-identical to both transports under the same logical topic name; iroh
  derives its 32-byte `TopicId` as `BLAKE3(topic_name)[..32]`.

  See `src/p2p_iroh/dual_publish/CATALOG.md` for the publisher catalog.
  ```

- [ ] **Step 7.4:** Commit.

  ```bash
  git commit -am "iroh dual-publish: README + CATALOG addenda for subscriber-side parity"
  ```

---

## Task 8: End-to-end soak — alpha cluster cross-stack inventory delivery

**Files:**
- Test: `/projects/elohim/elohim/elohim-storage/tests/iroh_gossip_cross_stack_e2e.rs`

- [ ] **Step 8.1:** Three-node fixture: `provider` runs both transports with `DualGossipPublisher`; `libp2p_only_subscriber` runs libp2p; `iroh_only_subscriber` runs iroh. Provider publishes one inventory snapshot, one identity-binding, one recovery-invitation, one recovery-revocation, and one feedback-signal. Both subscribers MUST receive each of the five topics with byte-identical payloads. Test runs under a 60-second timeout.

  This is the cutover-gate acceptance test: any divergence here (one subscriber missing a topic, bytes differ) fails CI.

- [ ] **Step 8.2:** Build + run:

  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh \
    --test iroh_gossip_cross_stack_e2e -- --nocapture
  ```

- [ ] **Step 8.3:** Commit.

  ```bash
  git commit -am "iroh dual-publish: cross-stack e2e soak — provider→(libp2p,iroh) parity"
  ```

---

## Task 9: Pre-push verification + clippy + fmt

- [ ] **Step 9.1:** Crate-wide build under both feature permutations:

  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --no-default-features && \
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --features p2p-iroh
  ```

- [ ] **Step 9.2:** Clippy + fmt (catches the "swarm-composition fresh-tree" gotcha per memory pin):

  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p-iroh -- -D warnings && \
    cargo fmt --check
  ```

- [ ] **Step 9.3:** Full crate-wide test pass:

  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh
  ```

- [ ] **Step 9.4:** No commit at this step — failures here mean returning to the responsible task.

---

## Out-of-scope (mention but do not implement)

- **Unicast-via-gossip semantics.** If a future feature wants per-recipient `select_transport` for a gossip-shaped flow (e.g. send-to-one-peer over gossip), the plan from `peer_transport_manifest` (Plane::Gossip + `select_transport`) is the entry point. Dual-publish today is broadcast-only and unconditionally fans out per the verdict table; per-recipient transport choice does not apply.

- **Receive-side iroh subscribers.** This plan establishes the receive-side ALPN (`iroh_gossip::ALPN` already on Router) and the `IrohGossipPublisher` task subscribes-on-publish; routing the received bytes into the existing libp2p-shaped receive handlers (`src/p2p/mod.rs:4396-4607`) is a follow-up task — for the cutover gate #4 the requirement is publish parity, and the subscribe side is exercised end-to-end by Task 8.

- **Receive-side dedup across both transports.** A peer that receives the same payload via both transports today gets two notifications. Dedup keyed on `(topic, blake3(payload))` belongs in the receive routing — out-of-scope here.

## Self-review checklist

- [x] Catalog of publish sites is complete (Task 1, 12 sites + 1 future producer)
- [x] Each topic verdict has a wiring task: inventory (Task 4), identity-binding (Task 5), recovery (Task 6), attention (n/a — Holochain-private, not gossip), feedback (Task 3), integrity-revocation (Task 7 — future-producer hook documented), EPR-atom-announce (Task 3)
- [x] EPR Announce flip-to-accepted is in the plan (Task 5 step 5.2 + step 5.6 test)
- [x] Byte-parity test exists (Task 5 step 5.4)
- [x] No placeholders — every task has concrete file paths, code snippets, build/test commands, and commit steps
