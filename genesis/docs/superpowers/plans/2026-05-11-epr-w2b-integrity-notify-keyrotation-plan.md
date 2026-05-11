# EPR Wave 2B — IntegrityNotify KeyRotation handler

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the libp2p `IntegrityNotify` direct-notify pipeline to handle `KeyRotation` events alongside the existing `KeyRevocation` handler. Per master-plan decision D3 (Stage 2): KeyRotation lands this sprint; AgentPeerBinding waits on Phase 12 (iroh master); RevocationAttestation deferred to graph-native sprint.

**Architecture:** Mirror the proven `KeyRevocation` pattern at `epr_atom_service.rs:289-352`. Add a new `RecoveryRotationMessage` wire type (mirroring `RecoveryRevocationMessage` at `p2p/recovery_revocation.rs`). Add a `"KeyRotation"` match arm in `handle_integrity_notify` that decodes, dedups, logs, returns `IntegrityAck { received: true }`. The substantive write (`upsert_key_rotation` into `key_rotations` table) happens via the local conductor's signal stream — direct-notify is informational/optimistic delivery, not the canonical write path. This mirrors how KeyRevocation works today.

**Tech Stack:** Rust (elohim-storage), libp2p 0.54, MessagePack via rmp_serde.

**Spec / parent master:** `genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md` Wave 2B.

**Coordinates with:** Recovery M4 per `project_epr2b_recovery_m4_convergence`. The `RecoveryRotationMessage` wire shape MUST be coordinated — Recovery M4 owns the producer side (peer A's `commit_key_rotation` → DnaSignal → IntegrityNotify publish). This plan owns the consumer side. Use the existing `KeyRotationPayload` shape (`signals.rs:643`) as the canonical field set.

## P2P Design Gate Output

| Entity | Category | Source of Truth | Justification |
|---|---|---|---|
| `RecoveryRotationMessage` (new wire type) | C — operational protocol payload | Holochain DHT (`KeyRotation` integrity entry, imagodei zome) | Wire-only — peers exchange it via libp2p direct-notify, but it carries projections of DHT-notarized facts. Persisted only via the existing signal-stream → `key_rotations` table path (already landed). |
| `key_rotations` table (existing) | C — operational projection | Same DHT entry above | Already exists; this plan touches only the wire path |
| `handle_integrity_notify` KeyRotation arm | helper invocation | Pure function call site | No new entity |

**Anti-pattern check:** ✓ No new entry types. ✓ No new tables. ✓ No new HTTP routes. ✓ Wire format mirrors existing pattern. ✓ Source-of-truth declarations preserved.

## File Structure

### New files
| Path | Responsibility |
|------|----------------|
| `elohim/elohim-storage/src/p2p/recovery_rotation.rs` | `RecoveryRotationMessage` struct + `to_bytes` + `from_bytes` mirroring `recovery_revocation.rs` |

### Modified files
| Path | What changes |
|------|--------------|
| `elohim/elohim-storage/src/p2p/mod.rs` | Re-export `recovery_rotation` module |
| `elohim/elohim-storage/src/epr_atom_service.rs` | Add `"KeyRotation"` match arm in `handle_integrity_notify` (around line 339) |

### Test files
| Path | Responsibility |
|------|----------------|
| `elohim/elohim-storage/src/p2p/recovery_rotation.rs` | inline `#[cfg(test)]` round-trip test (mirror recovery_revocation pattern) |
| `elohim/elohim-storage/src/epr_atom_service.rs` | extend existing `#[cfg(test)] mod tests` with `integrity_notify_keyrotation_acks_received_true` |

---

## Task W2B.1 — RecoveryRotationMessage wire type + IntegrityNotify handler

**Files:**
- Create: `elohim/elohim-storage/src/p2p/recovery_rotation.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (re-export)
- Modify: `elohim/elohim-storage/src/epr_atom_service.rs` (add match arm)

- [ ] **Step 1: Read the templates**

```
sed -n '1,98p' /projects/elohim/elohim/elohim-storage/src/p2p/recovery_revocation.rs
sed -n '283,352p' /projects/elohim/elohim/elohim-storage/src/epr_atom_service.rs
sed -n '640,670p' /projects/elohim/elohim/elohim-storage/src/signals.rs
```
Confirm:
- `RecoveryRevocationMessage` is `#[serde(rename_all = "camelCase")]`, struct-as-map MessagePack via `to_vec_named`
- `KeyRevocation` arm decodes via `from_bytes`, dedups on `format!("KeyRevocation:{}", msg.revocation_id)`, logs at info level, returns `IntegrityAck { received: true }`
- `KeyRotationPayload` (signals.rs:643) is the canonical field set: `human_id`, `previous_key`, `new_key`, `effective_at`, etc. — match exact field names from that struct.

- [ ] **Step 2: Write the failing round-trip test inline**

In `elohim/elohim-storage/src/p2p/recovery_rotation.rs` (file does not yet exist — Step 4 creates it; for now, write the test file separately):

```rust
// elohim/elohim-storage/tests/recovery_rotation_wire.rs
use elohim_storage::p2p::recovery_rotation::RecoveryRotationMessage;

#[test]
fn rotation_message_roundtrips_msgpack() {
    let original = RecoveryRotationMessage {
        rotation_id: "u-rotation-1".into(),
        human_id: "human-matthew".into(),
        previous_key: "base64previouskey".into(),
        new_key: "base64newkey".into(),
        effective_at: "2026-05-11T12:00:00Z".into(),
        sender_peer_id: "12D3KooWtest".into(),
        sent_at: "2026-05-11T12:00:01Z".into(),
    };

    let bytes = original.to_bytes().expect("encode");
    let decoded = RecoveryRotationMessage::from_bytes(&bytes).expect("decode");
    assert_eq!(decoded, original);
}
```

(Adapt the field set if `KeyRotationPayload` in signals.rs has additional fields — this plan uses a representative shape. Match canonical field names exactly.)

- [ ] **Step 3: Run test to verify it fails**

```
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --test recovery_rotation_wire 2>&1 | tail -20
```
Expected: FAIL — module `recovery_rotation` does not exist.

- [ ] **Step 4: Create the wire type**

```rust
// elohim/elohim-storage/src/p2p/recovery_rotation.rs
//! ## Source of Truth
//!
//! Wire payload (Category C operational protocol) for the libp2p
//! IntegrityNotify direct-notify path. Carries projections of the
//! DHT-notarized KeyRotation entry (imagodei zome). The DHT entry is
//! authoritative; this message is delivery-optimistic — peers can also
//! discover the same rotation via the local conductor's signal stream.
//!
//! Coordinated with Recovery M4 producer per
//! project_epr2b_recovery_m4_convergence memory pin.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRotationMessage {
    /// Coordinator-assigned rotation ID (stable deduplication key).
    pub rotation_id: String,
    /// Legacy String human id of the human whose key is rotating.
    pub human_id: String,
    /// Base64-encoded AgentPubKey being superseded.
    pub previous_key: String,
    /// Base64-encoded AgentPubKey replacing the previous one.
    pub new_key: String,
    /// ISO-8601 timestamp when the rotation became effective.
    pub effective_at: String,
    /// Base58 PeerId of the publishing node.
    pub sender_peer_id: String,
    /// ISO-8601 timestamp when this message was assembled for publishing.
    pub sent_at: String,
}

impl RecoveryRotationMessage {
    /// Encode to MessagePack bytes for IntegrityNotify direct-notify.
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        // Named-field MessagePack (struct-as-map) so adding/reordering
        // fields doesn't silently break across rolling upgrades.
        rmp_serde::to_vec_named(self)
    }

    /// Decode from IntegrityNotify-received bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}
```

Re-export from `p2p/mod.rs`:
```rust
pub mod recovery_rotation;
```

- [ ] **Step 5: Run round-trip test → PASS**

Same command from Step 3.

- [ ] **Step 6: Add the KeyRotation match arm in `handle_integrity_notify`**

In `elohim/elohim-storage/src/epr_atom_service.rs` around line 339, INSERT a new arm BEFORE the `other_kind` catch-all:

```rust
"KeyRotation" => {
    match crate::p2p::recovery_rotation::RecoveryRotationMessage::from_bytes(
        &payload_bytes,
    ) {
        Ok(msg) => {
            // W2B: dedup on synthetic KeyRotation:<id> key. Same rotation
            // arriving via direct-notify + signal stream will not double-
            // process after the first delivery.
            let dedup_key = format!("KeyRotation:{}", msg.rotation_id);
            if !self.dedup.insert(&dedup_key) {
                debug!(
                    target: "elohim_storage::dedup",
                    from = %peer_label,
                    rotation_id = %msg.rotation_id,
                    "duplicate KeyRotation direct-notify — dropped"
                );
                return EprAtomResponse::IntegrityAck {
                    received: true,
                    reason: Some("duplicate".to_string()),
                };
            }
            info!(
                target: "elohim_storage::recovery",
                from = %peer_label,
                rotation_id = %msg.rotation_id,
                human_id = %msg.human_id,
                effective_at = %msg.effective_at,
                "W2B: Received KeyRotation via direct-notify"
            );
            // Note: the canonical write to key_rotations table happens via
            // the local conductor's RecoveryV2Signal::KeyRotationCommitted
            // handler (signals.rs:1013). Direct-notify is delivery-
            // optimistic — it does not write to the projection here, to
            // avoid divergence with the signal-stream-driven canonical path.
            EprAtomResponse::IntegrityAck {
                received: true,
                reason: None,
            }
        }
        Err(e) => {
            warn!(
                target: "elohim_storage::recovery",
                from = %peer_label,
                error = %e,
                "W2B: Failed to decode RecoveryRotationMessage from direct-notify"
            );
            EprAtomResponse::IntegrityAck {
                received: false,
                reason: Some(format!("decode failed: {e}")),
            }
        }
    }
}
```

- [ ] **Step 7: Add a regression test in `epr_atom_service.rs::tests`**

Find the existing `integrity_notify_unhandled_kind_acks_with_reason` test (line ~399). Add a sibling test:

```rust
#[test]
fn integrity_notify_keyrotation_acks_received_true() {
    let service = EprAtomService::new_for_test();
    let msg = crate::p2p::recovery_rotation::RecoveryRotationMessage {
        rotation_id: "u-rotation-test1".into(),
        human_id: "human-matthew".into(),
        previous_key: "k1".into(),
        new_key: "k2".into(),
        effective_at: "2026-05-11T12:00:00Z".into(),
        sender_peer_id: "12D3KooWtest".into(),
        sent_at: "2026-05-11T12:00:01Z".into(),
    };
    let bytes = msg.to_bytes().expect("encode");

    let response = service.handle(
        "test-peer",
        CallerIdentity::Anonymous,
        EprAtomRequest::IntegrityNotify {
            kind: "KeyRotation".to_string(),
            payload_bytes: bytes,
        },
    );

    match response {
        EprAtomResponse::IntegrityAck { received: true, reason: None } => {}
        other => panic!("expected IntegrityAck {{ received: true }}, got {:?}", other),
    }
}

#[test]
fn integrity_notify_keyrotation_dedup_returns_duplicate_reason() {
    let service = EprAtomService::new_for_test();
    let msg = crate::p2p::recovery_rotation::RecoveryRotationMessage {
        rotation_id: "u-rotation-dedup".into(),
        // ... same minimal fields as above
        human_id: "h".into(),
        previous_key: "k1".into(),
        new_key: "k2".into(),
        effective_at: "2026-05-11T12:00:00Z".into(),
        sender_peer_id: "12D3KooWdedup".into(),
        sent_at: "2026-05-11T12:00:01Z".into(),
    };
    let bytes = msg.to_bytes().expect("encode");

    // First delivery: received: true, reason: None
    let _ = service.handle("test-peer", CallerIdentity::Anonymous,
        EprAtomRequest::IntegrityNotify { kind: "KeyRotation".into(), payload_bytes: bytes.clone() });

    // Second delivery: dedup'd, received: true, reason: Some("duplicate")
    let response = service.handle("test-peer", CallerIdentity::Anonymous,
        EprAtomRequest::IntegrityNotify { kind: "KeyRotation".into(), payload_bytes: bytes });

    match response {
        EprAtomResponse::IntegrityAck { received: true, reason: Some(r) } if r == "duplicate" => {}
        other => panic!("expected dedup'd IntegrityAck, got {:?}", other),
    }
}
```

(Adapt `EprAtomService::new_for_test()` and import paths to match what's actually in `epr_atom_service.rs::tests`. If a fresh service constructor doesn't exist, look at how the existing `integrity_notify_unhandled_kind_acks_with_reason` test constructs it.)

- [ ] **Step 8: Run tests to verify all pass**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --test recovery_rotation_wire 2>&1 | tail -10
cargo test --lib epr_atom_service 2>&1 | tail -20
```
Expected: round-trip test PASS, both new service tests PASS, existing `integrity_notify_unhandled_kind_acks_with_reason` STILL PASS (unhandled kinds are still rejected with reason).

- [ ] **Step 9: Run clippy + fmt**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -10
cargo fmt --check
```

- [ ] **Step 10: Commit**

```bash
git add elohim/elohim-storage/src/p2p/recovery_rotation.rs \
        elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/src/epr_atom_service.rs \
        elohim/elohim-storage/tests/recovery_rotation_wire.rs
git commit -m "$(cat <<'EOF'
feat(storage): W2B — IntegrityNotify KeyRotation handler

Per master-plan D3 Stage 2: extend IntegrityNotify direct-notify
pipeline beyond KeyRevocation to include KeyRotation.

Adds RecoveryRotationMessage wire type mirroring
RecoveryRevocationMessage shape: camelCase, struct-as-map
MessagePack via rmp_serde::to_vec_named for forward-compatible
field evolution.

epr_atom_service::handle_integrity_notify now matches "KeyRotation"
explicitly: decode → dedup on KeyRotation:<id> → log at info →
return IntegrityAck { received: true }. Mirrors KeyRevocation
exactly. The canonical write to key_rotations table stays on the
local conductor's signal stream (signals.rs:1013) — direct-notify
is delivery-optimistic, not the canonical write path.

AgentPeerBinding waits on Phase 12 caller identity (iroh master).
RevocationAttestation deferred to graph-native sprint per master D3.

Coordinated with Recovery M4 per project_epr2b_recovery_m4_convergence.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review

- ✅ Mirrors KeyRevocation pattern exactly — same dedup key shape, same log structure, same response shape
- ✅ Coordinates with Recovery M4 (canonical write path stays on signal stream, direct-notify is delivery-optimistic)
- ✅ Wire format uses named MessagePack — forward-compatible field evolution
- ✅ Three tests: round-trip, first-delivery, dedup
- ✅ No scope creep — does NOT touch key_rotations table writer, signal stream, or other handler kinds
- ✅ AgentPeerBinding + RevocationAttestation explicitly deferred per master plan D3
- ✅ No new entry types, no migrations, no HTTP routes
