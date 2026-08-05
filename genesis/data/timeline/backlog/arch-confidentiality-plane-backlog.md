---
id: "backlog-arch-confidentiality-plane-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Confidentiality-plane backlog — the unbuilt encryption layer, clustered (§3.13)"
slug: "arch-confidentiality-plane-backlog"
written: "2026-08-04"
author: "claude (research mint pass, operator-directed clustering)"
status: "backlog"
priority: "high"
tags: [architecture, confidentiality, encryption, key-envelope, reach, security, research-derived]
cites:
  - genesis/research/ssb-scuttlebutt-ancestor-retrospective-2026-08-03.md
  - genesis/research/holepunch-p2p-dataplane-cross-pollination-2026-06-24.md
  - genesis/research/p2panda-cross-pollination-2026-08-04.md
  - elohim/elohim-storage/src/services/private_replica.rs
---

# Confidentiality-plane backlog (research mint pass, 2026-08-04)

Three consecutive surveys reached the same verdict: the encryption layer (seam-map §3.13) is our
sharpest built-vs-unbuilt gap, and it is **blocked on being scheduled, not on hard problems** —
SSB shipped envelope encryption for a decade; p2panda ships a research-grounded group-encryption
crate in our own language; our `private_replica.rs` proof-of-concept has the math proven in tests.
This cluster is the subject's single re-surfacing point; items graduate individually.
**Fold new confidentiality-layer concerns here — do not mint siblings.**

| # | Item | What + why (grounded) | Gate/blocker | Owner shape |
|---|------|----------------------|--------------|-------------|
| 1 | **Fail-closed reach classifier** | [SSB](epr:ssb-scuttlebutt-ancestor-retrospective-2026-08-03) take #1a, verified verbatim: the P2P reach classifier **fails open** on DB-pool errors (`reach_authorization.rs` returns `true` with a warning at Stage 1) while the HTTP path (`epr_service.rs`) already fails closed. Make fail-closed the classifier's contract. Smallest item, highest principle-per-line. | none — bounded bugfix + regression scenario | quality-deep / rust-architect (small) |
| 2 | **`p2panda-encryption` adoption evaluation** | [p2panda](epr:p2panda-cross-pollination-2026-08-04) study #8 — I/O-free, MIT/Apache, fuzz-tested DCGKA/2SM/Double-Ratchet group encryption in Rust; could collapse the `KeyEnvelope` build-out. Evaluate consuming the *crate*, not the protocol stack. | ⚠ their security audit announced Feb 2025, never confirmed published — resolve first; then p2p-design-gate for `KeyEnvelope` (likely B2) | rust-architect study → operator decision |
| 3 | **Three-way credential separation (`KeyEnvelope` design)** | [Holepunch](epr:holepunch-p2p-dataplane-cross-pollination-2026-06-24) borrow #4 — verify-key ≠ locate-token ≠ at-rest-key; today holding the locate-token (bare hash in inventory gossip) implies fetch rights. Maps to CID (verify) · topic membership (locate) · per-reader sealed DEK (read). p2panda's PSI discovery ([borrows](epr:arch-dataplane-borrows-backlog) #6) is the locate-leg companion. | p2p-design-gate (B2) — the survey explicitly did NOT pre-clear the entry type | rust-architect brainstorm → spec |
| 4 | **Pluggable encryption-format + reindex-on-new-key shape** | SSB take #1b — steal `ssb-db2`'s `installEncryptionFormat` / `reindexEncrypted()` interface shape when building `KeyEnvelope`: cleanly separates "which messages can I decrypt *now*" from the log itself; exactly what a late-added reader-key substrate needs. Design input to #2/#3, not standalone work. | rides #2/#3 | (design input) |
| 5 | **ed25519→X25519 key-conversion substrate** | The blocker both surveys name: Holochain agent keys are ed25519; every sealed-DEK path needs X25519. Named-but-unbuilt since the Holepunch survey; gates #2 and #3's production legs. Weave Wave C context. | design decision (conversion vs dual-key binding) | rust-architect shift |
| 6 | **Encrypt-then-erasure-code production path** | Holepunch DEFER #6, still true: `private_replica.rs` math proven in tests (`reader_with_envelope_recovers_custodian_cannot`), but `parity_shard_count: 0` hardcoded, no custodian-dispatch path, blobs plaintext at rest. Unblocks when #3 + #5 land + RS(4,7) leaves test-only. | blocked on #3, #5 | backlog-only until then |

**Sequencing note:** #1 is independent and immediate. #2's audit-status question is a one-hour
web/email check that gates a large fork in effort — do it before any `KeyEnvelope` design so #3
knows whether it's designing around an external crate or building from primitives. #4 folds into
whichever of #2/#3 proceeds. #5 unblocks the production legs; #6 is terminal.
