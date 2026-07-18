---
id: "backlog-agent-peer-binding-signing"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Sign the AgentPeerBinding — replace STAGE1_SIGNATURE_SENTINEL so transport-id bindings are cryptographically verified (identity-head C2)"
slug: "agent-peer-binding-signing"
written: "2026-07-18"
author: "identity-head arc (Wave C2, operator: defer as security follow-on)"
status: "open"
priority: "high"
area: "imagodei/identity-transport-binding"
domain: "D2"
jobs: [elohim-edge]
cites:
  - genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
  - genesis/data/timeline/backlog/keyrotation-mint-path-witness-backed.md
tags: [identity, agent-peer-binding, transport-id, signature, security, did-elohim, alsoKnownAs, witnessed-binding]
---

# Sign the AgentPeerBinding — the deferred witnessed-binding leg (identity-head C2)

## Why this exists
The identity-head arc (Waves A–C1) ships the primitive: `did:elohim` resolves real
controllers + lineage. It also emits transport ids (libp2p PeerId / iroh NodeId) as
`alsoKnownAs` entries — but those are **self-asserted / unverified today**
(`STAGE1_SIGNATURE_SENTINEL = "c3RhZ2UtMS1zaWduYXR1cmU="` at
`elohim/elohim-storage/src/p2p/identity_binding_gossip.rs:129`; the binding gossip
payload carries the sentinel instead of a real signature). The DID document reflects
this honestly (phase-1 behavior C1 preserves): a transport-id `alsoKnownAs` is a
claim, not a proof.

## The work (C2 — security-critical, design-first)
Replace the sentinel with a real signature so a transport-id binding is
cryptographically verified before it becomes a `did:elohim` `alsoKnownAs`:
- **Design the scheme FIRST** (this is why it was deferred from the execution loop):
  which key signs `(agent_cid ‖ transport_id)` — the conductor-held head key, the
  transport key, or a cross-signature of both? How does elohim-storage reach the
  conductor-held head key to sign (admin/app-interface signing call)? Replay
  protection / freshness (a nonce or the existing `identity_handshake.rs`
  challenge/response scaffold — `elohim/elohim-storage/src/p2p/identity_handshake.rs`).
- Verify on receipt in the binding gossip path (`identity_binding_gossip.rs`,
  `reconcile/controller.rs:630`) and gate the `did:elohim` `alsoKnownAs` on a valid
  signature (the assembly currently populates from `store.transport_ids` with no
  verification — `bridges/did/did-bridge/src/did_elohim.rs`).
- **Red-team review before landing** — a weak scheme is worse than the honest
  sentinel (identity-transport binding is a spoofing surface).

## Scope note
Bounded to elohim-storage p2p (`identity_binding_gossip.rs` + `identity_handshake.rs`
+ `reconcile/controller.rs`) + the did:elohim assembly gate — NOT a DNA change (the
`AgentPeerBinding` entry type already exists in imagodei; this is the signing/verify
layer). Pairs with `keyrotation-mint-path-witness-backed` as the two identity-head
hardening follow-ons.
