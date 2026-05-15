# EPR Wave 2A — record_predecessor on libp2p EPR Atom Announce

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the missing `record_predecessor` call on the libp2p EPR Atom Announce path so the back-prop graph captures sender PeerId for every Content EPR arrival. Closes T18 (LUG plan) and T22 (P3.5 plan) — convergent gap surfaced by the Wave 0 audit.

**Status:** ✅ LANDED in dev. Live code at `elohim/elohim-storage/src/p2p/mod.rs:5317–5372`; comment block updated at `api/epr.rs:189–191`; confirmed by Wave 0 audit on 2026-05-15 (`genesis/docs/plans/2026-05-15-epr-wave0-audit-results.md` §D6). Checkboxes ticked as plan-tracking debt cleanup on the same date.

**Architecture:** Hook into `P2PNode::handle_epr_atom_request` in `p2p/mod.rs` (around line 5271 — the libp2p Announce handler that delegates to `EprAtomService`). After a successful Content-kind Announce ingest, call `services::back_prop::record_predecessor(conn, target_cid, peer.to_string(), keys)`. FeedbackSignal-kind ingests are excluded (the back-prop graph is for content provenance, not signal propagation). Sealing keys are plumbed from the same `fan_out_ctx` pattern that `api/epr.rs` uses for FeedbackSignal fan-out.

**Tech Stack:** Rust (elohim-storage), libp2p 0.54, dryoc 2-of-2 sealing.

**Spec / parent master:** `genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md` Wave 2A.

**Convergent context (Wave 0 audit findings):**
- LUG audit: T18 marked as the single remaining gap, "redirected to `epr_atom_service.rs` / `p2p/mod.rs` but not yet implemented there either"
- P3.5 audit: T22 record_predecessor at `api/epr.rs:626` is documented as intentionally deferred to the libp2p path — `epr_atom_service.rs:189` confirms "P2PNode-side wiring for now"
- The `record_predecessor` function exists at `services/back_prop.rs:145` — fully implemented, idempotent on duplicates, takes `(conn, target_cid, predecessor_peer_id, keys)`

## P2P Design Gate Output

| Entity | Category | Source of Truth | Justification |
|---|---|---|---|
| `predecessor_records` table (existing) | C — operational | DHT (FeedbackSignal arrival logs are the rebuildable source) | Already exists; this plan only adds a writer call site |
| `record_predecessor` call site (new) | helper invocation | None — pure function call | No new entity; wires existing function to existing handler |
| `peer.to_string()` PeerId capture | ephemeral parameter | libp2p `request_response` Behaviour | Caller-supplied at the protocol boundary; no persistence beyond the predecessor seal |

**Anti-pattern check:** ✓ No new entry types. ✓ No new tables (predecessor_records exists from Phase 3.5 T10). ✓ No new HTTP routes. ✓ No CID-as-FK. ✓ Source of truth declarations preserved at module level.

## File Structure

### Modified files
| Path | What changes |
|------|--------------|
| `elohim/elohim-storage/src/p2p/mod.rs` | Add `record_predecessor` call after successful Content-kind Announce ingest in `handle_epr_atom_request` |
| `elohim/elohim-storage/src/api/epr.rs` | Update T18/T22 TODO comment block (lines 618-626) to reflect wiring lands in p2p/mod.rs |
| `elohim/elohim-storage/src/epr_atom_service.rs` | Update the `record_predecessor TODO is intentionally left in P2PNode-side wiring` comment at line 189 to reflect wiring landed |

### New files
| Path | Responsibility |
|------|----------------|
| `elohim/elohim-storage/tests/back_prop_record_predecessor_announce_e2e.rs` | E2E test: Content EPR Announce arrives via libp2p → predecessor row appears in DB; FeedbackSignal Announce does NOT create a predecessor row |

---

## Task T22.1 — Wire record_predecessor on libp2p Announce

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (around lines 5271-5290 in `handle_epr_atom_request`)
- Create: `elohim/elohim-storage/tests/back_prop_record_predecessor_announce_e2e.rs`

- [x] **Step 1: Verify the implementation context**

Read `p2p/mod.rs` lines 5260-5320 in full. Confirm:
- `handle_epr_atom_request` signature is `async fn handle_epr_atom_request(&self, peer: libp2p::PeerId, request: EprAtomRequest) -> EprAtomResponse`
- It currently delegates to `self.epr_atom_service().handle(&peer.to_string(), caller, request)` and returns the response
- The `EprAtomRequest::Announce` variant carries `envelope_bytes` (or similar — verify the exact field shape from `EprAtomProtocol`)

Then grep for how `fan_out_ctx.sealing_keys` is plumbed into `api/epr.rs`:
```
grep -n "sealing_keys\|SealingPubKeys\|fan_out_ctx" /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs /projects/elohim/elohim/elohim-storage/src/api/epr.rs
```
Identify whether `P2PNode` already has access to sealing keys (check the `Self` struct definition in `p2p/mod.rs`). If yes, use them directly. If no, the keys need to be plumbed in via P2PNode construction — which is a wider change. STOP and report BLOCKED if sealing keys are not already available to P2PNode; this plan assumes they are.

- [x] **Step 2: Write the failing E2E test**

```rust
// tests/back_prop_record_predecessor_announce_e2e.rs
//! W2A — verify libp2p EPR Atom Announce of Content kind records the
//! sender PeerId as a predecessor; FeedbackSignal Announce does not.

use elohim_storage::services::back_prop::read_predecessors;
use elohim_storage::db::test_helpers::{fresh_pool_with_unsealing_keys, sealing_keys_for_test};
// Adapt imports to the actual test_helpers API — verify before writing

#[tokio::test]
async fn announce_content_epr_records_predecessor() {
    let (pool, unsealing_keys) = fresh_pool_with_unsealing_keys();
    let sealing = sealing_keys_for_test();
    let sender_peer_id = "12D3KooWtest1";
    let content_cid = "bafkreitestcontent1";

    // Construct a Content-kind EPR atom (not FeedbackSignal).
    let envelope = build_test_content_envelope(content_cid);
    let envelope_bytes = elohim_epr::canonical_envelope_bytes(&envelope).unwrap();

    // Simulate the libp2p Announce arrival path.
    let request = EprAtomRequest::Announce { envelope_bytes };
    let p2p_node = construct_test_p2p_node(pool.clone(), sealing.clone()).await;
    let response = p2p_node.handle_epr_atom_request(
        libp2p::PeerId::from_str(sender_peer_id).unwrap(),
        request,
    ).await;

    // Assert: ingest accepted.
    matches!(response, EprAtomResponse::Announced { accepted: true, .. });

    // Assert: predecessor row written.
    let mut conn = pool.get().expect("conn");
    let predecessors = read_predecessors(&mut conn, content_cid, &unsealing_keys.as_keys()).unwrap();
    assert_eq!(predecessors, vec![sender_peer_id.to_string()],
        "Content EPR Announce must record sender as predecessor");
}

#[tokio::test]
async fn announce_feedback_signal_epr_does_not_record_predecessor() {
    let (pool, unsealing_keys) = fresh_pool_with_unsealing_keys();
    let sealing = sealing_keys_for_test();
    let sender_peer_id = "12D3KooWtest2";
    let signal_cid = "bafkreitestsignal1";

    let envelope = build_test_feedback_signal_envelope(signal_cid);
    let envelope_bytes = elohim_epr::canonical_envelope_bytes(&envelope).unwrap();

    let request = EprAtomRequest::Announce { envelope_bytes };
    let p2p_node = construct_test_p2p_node(pool.clone(), sealing.clone()).await;
    let _ = p2p_node.handle_epr_atom_request(
        libp2p::PeerId::from_str(sender_peer_id).unwrap(),
        request,
    ).await;

    let mut conn = pool.get().expect("conn");
    let predecessors = read_predecessors(&mut conn, signal_cid, &unsealing_keys.as_keys()).unwrap();
    assert!(predecessors.is_empty(),
        "FeedbackSignal Announce must NOT record predecessor — back-prop graph is for content only");
}
```

If `construct_test_p2p_node` doesn't exist, adapt to the closest existing test pattern in `tests/` (search for similar harness usage). If P2PNode is not test-constructible without a full swarm, adapt the test to invoke the post-ingest record_predecessor call directly with a synthetic EprAtomRequest::Announce — the goal is to verify the kind-filter + record_predecessor wiring, not to spin a swarm.

- [x] **Step 3: Run test to verify it fails**

```
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --test back_prop_record_predecessor_announce_e2e 2>&1 | tail -40
```
Expected: FAIL — `predecessors` is empty for the content-kind test (record_predecessor not yet called).

- [x] **Step 4: Implement the wiring in handle_epr_atom_request**

In `p2p/mod.rs::handle_epr_atom_request` (around line 5283 where `service.handle` returns):

```rust
async fn handle_epr_atom_request(
    &self,
    peer: libp2p::PeerId,
    request: EprAtomRequest,
) -> EprAtomResponse {
    let caller = self.identity_map.lookup(&peer);

    // Capture the EPR kind + CID before consuming `request` (for back-prop).
    // Only Content-kind ingests record predecessors; FeedbackSignal/other kinds
    // do not contribute to the back-prop graph.
    let predecessor_target = if let EprAtomRequest::Announce { envelope_bytes } = &request {
        decode_envelope_kind_and_cid(envelope_bytes)  // helper — see below
    } else {
        None
    };

    let response = self.epr_atom_service().handle(&peer.to_string(), caller, request);

    // T22 / T18 wiring: record sender PeerId as predecessor on successful
    // Content-kind Announce ingest. Idempotent — duplicates are no-ops per
    // services::back_prop::record_predecessor (see services/back_prop.rs:145).
    if let EprAtomResponse::Announced { accepted: true, .. } = &response {
        if let Some((EprKind::Content, cid)) = predecessor_target {
            if let Some(sealing_keys) = self.sealing_keys_for_back_prop() {  // adapt to actual accessor
                if let Ok(mut conn) = self.db_pool.get() {
                    if let Err(e) = crate::services::back_prop::record_predecessor(
                        &mut conn,
                        &cid,
                        &peer.to_string(),
                        &sealing_keys,
                    ) {
                        tracing::warn!(
                            target = "epr::back_prop",
                            cid = %cid,
                            peer = %peer,
                            error = ?e,
                            "record_predecessor failed (best-effort, non-fatal)"
                        );
                    }
                }
            }
        }
    }

    response
}

/// Decode the kind + CID from a canonical envelope without doing the full
/// validation EprAtomService does. Returns None if the envelope can't be
/// minimally decoded — record_predecessor is best-effort.
fn decode_envelope_kind_and_cid(envelope_bytes: &[u8]) -> Option<(EprKind, String)> {
    // Use elohim_epr's canonical decode.
    let envelope: elohim_epr::Envelope = elohim_epr::decode_canonical(envelope_bytes).ok()?;
    Some((envelope.kind, envelope.cid))
}
```

**Adapt to actual code:**
- The accessor `self.sealing_keys_for_back_prop()` may not exist by that name — find the actual field/method in P2PNode struct definition. If sealing keys live on `fan_out_ctx`, route through that.
- `EprAtomRequest::Announce` field name (`envelope_bytes` vs `bytes` vs `atom`) must match actual definition in `p2p/epr_atom_protocol.rs`.
- `EprKind::Content` variant name must match actual `EprKind` enum in `elohim_epr::kind`.

**STRICT FORBID:** do not change function signatures of `record_predecessor`, `EprAtomService::handle`, or any pre-existing public API. Wire only.

- [x] **Step 5: Run test to verify it passes**

Same command as Step 3. Expected: PASS (2 tests).

- [x] **Step 6: Update stale TODO comments**

In `api/epr.rs` lines 618-626, update the comment block:
```rust
// T22 wiring landed: see p2p/mod.rs::handle_epr_atom_request — Content-kind
// Announce ingests record sender PeerId via services::back_prop::record_predecessor.
// HTTP put_epr (this function) does NOT record predecessors because the caller
// is the local HTTP client, not a remote peer; back-prop is for cross-peer
// content provenance.
```

In `epr_atom_service.rs` line 189, update:
```rust
// record_predecessor wiring lives in p2p/mod.rs::handle_epr_atom_request
// (W2A landed; see services/back_prop::record_predecessor for the recorder).
// EprAtomService stays transport-neutral and does not call record_predecessor —
// the libp2p sender PeerId is only available at the protocol boundary.
```

- [x] **Step 7: Run cargo clippy + fmt**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo clippy -- -D warnings 2>&1 | tail -20
cargo fmt --check
```
Both must pass clean.

- [x] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/src/api/epr.rs \
        elohim/elohim-storage/src/epr_atom_service.rs \
        elohim/elohim-storage/tests/back_prop_record_predecessor_announce_e2e.rs
git commit -m "feat(storage): W2A — wire record_predecessor on libp2p EPR Announce

Closes T18 (LUG) and T22 (P3.5) — convergent gap surfaced by Wave 0
audit. After successful Content-kind EPR Atom Announce ingest, the
sender PeerId is sealed and recorded in predecessor_records via
services::back_prop::record_predecessor.

FeedbackSignal-kind Announce does NOT record a predecessor — the
back-prop graph is for content provenance, not signal propagation.

Best-effort + non-fatal: record_predecessor errors are logged at warn
level and never bubble to the EprAtomResponse. record_predecessor is
idempotent on duplicates per its existing contract.

Stale TODO comments at api/epr.rs:618-626 and epr_atom_service.rs:189
updated to point readers at the new wiring site.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
```

---

## Self-Review

- ✅ T18 / T22 convergent gap closed at the correct site (libp2p Announce handler, where sender PeerId is available)
- ✅ Kind-filter excludes FeedbackSignal — back-prop graph stays content-only per Phase 3.5 design
- ✅ Best-effort + non-fatal — DB / seal failures don't break the wire response (matches existing FeedbackSignal fan-out semantics in api/epr.rs)
- ✅ Idempotent — leverages existing record_predecessor contract (T10 on_conflict_do_nothing)
- ✅ No new entities, no signature changes, no scope creep
- ✅ Stale TODO comments updated so future readers find the actual wiring site
