---
name: feedback-structural-verify-canonical-wire-shape
description: "Structural-verify tests must use canonical wire-format string shapes, not synthetic constructor shapes. Otherwise tests pass while the production verifier rejects every real message.; fixed in c5d6dd827..6f66ffeb5 (Sprint 0 of REA-compute-substrate roadmap, 2026-05-28)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: f5ed8831-8faa-47bc-a508-fa91142db0de
---

Structural-verify tests must use the canonical wire-format string shape, not synthetic constructor shapes. Otherwise tests pass while the production verifier rejects every real message.

**Why:** During substrate-rea Task 10 verification on alpha (2026-05-27), every healthy storage peer was spamming `WARN elohim_storage::inventory: Inventory snapshot failed structural verify — dropped … error=InvalidHashFormat("sha256-1f3ed518a975f0eb55ae72c7cca8ef396c8f73c61ecf730ad54920ea0a24a955")`. The verifier `is_blob_hash_shaped` at `elohim/elohim-storage/src/p2p/inventory_gossip.rs:132–134` requires exactly 64 lowercase hex chars; the canonical wire format per `elohim-storage/CLAUDE.md` is `sha256-<64-hex>` (71 chars total). The verifier landed 2026-05-02 (T13, commit `9169ab99d`) and its tests at lines 140–225 used `"a".repeat(64)` as the hash fixture — bare hex, never prefixed. The mismatch went latent for ~25 days until substrate-rea exercised inventory gossip end-to-end on a real multi-peer cluster.

**How to apply:** When writing structural-verify tests for any wire type:
1. Use the actual production producer's output shape, not a constructor-friendly synthetic. If the wire format is `sha256-<hex>`, the test fixture is `format!("sha256-{}", "a".repeat(64))`, not `"a".repeat(64)`.
2. If the producer and verifier sit in different files, prefer a shared fixture function that round-trips through the canonical serialization. The test should not be able to pass while the production data flow rejects.
3. When the wire format is documented in a `CLAUDE.md` ("`sha256-{hex}` keeps its existing name"), cross-reference that doc in the test fixture comments so the next person knows the format isn't optional.

Related: [[project_quilt_pantry_vocabulary]] (canonical storage vocab); [[project_three_layer_truth_model]] (libp2p vs DHT vs doorway boundaries — verifiers live at libp2p layer, must match libp2p wire format).

**Status (2026-05-28):** Fixed across Sprint 0 of the REA-compute-substrate roadmap (`c5d6dd827`..`6f66ffeb5`). The predicate accepts canonical `sha256-<hex>` wire format AND the `BlobAddress` newtype now makes producer-↔-verifier drift unrepresentable at the type level. See `[[project_canonical_wire_shape_newtype_pattern]]` for the generalized pattern this incident produced.

**Sibling-verifier audit:** none. `IdentityBindingGossip::verify_structural` (the only other gossip verifier in the crate) has no hash-shape predicate — only non-empty checks.

**Roll-up status:** code-complete on `sprint/cross-pillar-cleanup`; cluster re-probe deferred to roll-up with other in-flight work.
