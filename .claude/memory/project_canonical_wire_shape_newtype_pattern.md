---
name: canonical-wire-shape-newtype-pattern
description: "Wire-format types deserve a constructor-validated newtype, not raw String. Verifier becomes pleonastic; producer↔verifier drift becomes impossible at type level. BlobAddress in elohim-storage is the reference instance."
metadata:
  node_type: memory
  type: project
---

When a wire format has structural rules (prefix, length, character class — like `sha256-<64-lower-hex>`), wrap it in a newtype that constructor-validates the shape. The serde `try_from = "String"` attribute makes the deserializer enforce the shape on every receive. The newtype then propagates as a type-level guarantee through every downstream consumer.

**Why:** Producer-↔-verifier drift is a recurring bug class. See `[[feedback_structural_verify_canonical_wire_shape]]` — the inventory verifier rejected 100% of real cluster gossip for ~26 days because the test fixtures lied about the wire shape. A newtype makes the bug literally unrepresentable: the producer cannot construct an invalid value; the deserializer cannot accept one; the verifier check becomes a no-op (or just non-empty / cross-field rules).

**How to apply:**

1. For every wire-format String field that has shape rules, create a newtype: `pub struct Foo(String);`.
2. Implement `TryFrom<String>` with your error type and use `#[serde(try_from = "String", into = "String")]`.
3. Implement `Display` (delegates to inner string), `From<Foo> for String` (consuming move, no allocation), `AsRef<str>` or `as_str() -> &str` for borrow access.
4. Use `thiserror::Error` for the error type so callers get `std::error::Error` for free.
5. Reference the canonical shape spec (`CLAUDE.md`, schema doc) in the newtype's doc-comment.
6. If the newtype survives a future architectural graduation, say so in the doc-comment — it tells the next contributor not to over-design around a soon-to-be-deprecated structure.
7. Lock in a producer-↔-verifier round-trip integration test that uses the real producer + real wire codec + real verifier. The `expect("real BlobStore returns canonical wire format")` message becomes the regression seatbelt — when the wire shape ever drifts, the message tells future-you exactly what broke.

**Reference instance:** `BlobAddress` in `elohim/elohim-storage/src/p2p/inventory_gossip.rs`. Survives the Stage-1 inventory-gossip → Stage-4 `serve-url-projection` Commitment graduation per `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md`.

**Related:** `[[feedback_schema_first_ioc]]` (schemas drive Rust + TS via codegen; newtypes are the in-process complement), `[[feedback_structural_verify_canonical_wire_shape]]` (the bug-class this pattern prevents), `[[project_rea_compute_commitment_primitive]]` (the destination where these newtypes get referenced as Commitment scope types).
