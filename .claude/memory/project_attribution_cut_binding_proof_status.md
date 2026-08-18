---
name: project-attribution-cut-binding-proof-status
title: Attribution cut — bindings carry proof_status
description: Economic joins take AttributableBindings (typed cut); posture defaults observe — never flip to enforce before C2-S2 minting exists.
metadata: 
  node_type: memory
  title: Attribution cut — bindings carry proof_status
  type: project
  originSessionId: e4e91b9c-ca18-4881-89e5-0bc36a68ce54
  modified: 2026-08-18T04:25:22.201Z
---

Landed 2026-08-18 (`8c739a723`, habit `identity-cross-signed` unwired→red): the
attribution cut in elohim-storage. **A binding may decide where to DIAL, never whom to
CREDIT.**

- `peer_identity_bindings` gained `signature` + `proof_status` (DEFAULT `'unverified'` NOT
  NULL). The projection used to DROP the signature — that absence, not the missing
  predicate, was why "verify on consume" had nowhere to stand.
- `p2p::binding_proof_wire::classify_binding_signature` is the single writer chokepoint and
  the ONLY constructor of the `cross_signed` value (`BindingProofStatus`'s inner enum is
  private and is the insert model's field type → hand-assigning a verified status is a
  compile error). Read it via `PeerIdentityBindingRow::is_cross_signed`, never
  `!= 'unverified'`.
- Economic joins take `AttributableBindings` **by type** (`list_attributable_for_agent`);
  routing/display keeps `list_active_for_agent` and its honest self-asserted rows.
- Posture `ELOHIM_ATTRIBUTION_CROSS_SIGNED=enforce|observe`, **default observe** + counter
  `elohim_attribution_unverified_bindings_total`. Do NOT flip to enforce before C2-S2
  (minting) exists — no peer can satisfy it, so it would blank every economic surface
  without making anything safer.

**Why this stays red, and what not to over-claim:** verification is receiver-local, NOT
notarized (integrity fold is C2-S7); nothing mints proofs yet; freshness is still
unanchored (the "pincer" — C2-S3); and durable PLACEMENT is still off the cut —
`services/transport_resolve.rs`'s source-2 fallback can redirect shard PUSH bytes to a
spoofed peer while `shard_locations` records the victim.

Decomposition + the 4-lens red-team review this implements:
`genesis/data/timeline/backlog/agent-peer-binding-signing.md` (C2-S1..S7).
Related: [[feedback_reach_head_replication_distinct_planes]],
[[feedback_concurrent_sessions_shared_worktree]].
