---
name: project-attribution-cut-binding-proof-status
title: Attribution cut — bindings carry proof_status
description: Economic joins take AttributableBindings (typed cut); posture stays observe until the LIVE unverified count hits zero — minting existing is not the gate.
metadata: 
  node_type: memory
  title: Attribution cut — bindings carry proof_status
  type: project
  originSessionId: e4e91b9c-ca18-4881-89e5-0bc36a68ce54
  modified: 2026-08-20T22:39:46.347Z
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
  `elohim_attribution_unverified_bindings_total`.

**The flip gate is the live COUNT, not the existence of a minter (corrected 2026-08-20).**
This entry used to say "do NOT flip before C2-S2 (minting) exists". C2-S2 now exists —
`p2p/binding_mint.rs` (828 lines: `assemble_core` → `agent_half_preimage` → `seal_proof`
→ `mint_own_binding`), spawned at boot from `main.rs` with retry backoff, default-ON
(`ELOHIM_BINDING_MINT=off` is the kill switch). Existence was a proxy and it has now been
met while the real condition has not. **The gate is
`elohim_attribution_unverified_bindings_total{posture="enforce"} == 0` on the fleet.** A
node being ABLE to mint says nothing about whether the peers whose bindings the economic
joins read HAVE minted, are running a build containing the minter, or have propagated
proofs that classify `cross_signed` on the reading node. Flip on fleet state, never on
code existence, or one deploy blanks every economic surface.

Two named reasons the count stays non-zero, both structural:
- **Minting is libp2p-only.** `main.rs` passes `identity.keypair()` regardless of
  `config.transport_backend` (Libp2p|Iroh|Dual), and `binding_proof_wire` derives a
  transport id only for `TRANSPORT_KIND_LIBP2P` — an iroh-kind row classifies
  `unverified` BY CONSTRUCTION, permanently, not transiently. Alpha has had dual enabled
  since 2026-08-05.
- **The minted proof reaches 2 of 3 writers** (DHT + gossip, not the handshake path).

**What else not to over-claim:** verification is receiver-local, NOT notarized (integrity
fold is C2-S7); freshness is still unanchored (the "pincer" — C2-S3); and durable
PLACEMENT is still off the cut — `services/transport_resolve.rs`'s source-2 fallback can
redirect shard PUSH bytes to a spoofed peer while `shard_locations` records the victim.

⚠ `genesis/data/timeline/backlog/agent-peer-binding-signing.md` still lists C2-S2 under
"Still NOT done" — **that file is stale**; it nearly authorized a night of rebuilding
landed code. Re-read the tree before trusting its done/not-done list.

**The live count is ZERO, and that is silence — not the gate being met (2026-08-20c).**
The gate above says flip when `unverified{enforce} == 0` on the fleet. MEASURED:
`max_over_time(elohim_attribution_unverified_bindings_total[7d]) == 0` across all 56 alpha
pod instances, and no `posture="enforce"` series exists at all. So by the letter of the
gate as written the flip condition is ALREADY satisfied — and acting on that would turn the
habit green having verified nothing. The zero cannot mean "all bindings are proven" either,
because the two structural reasons above predict a NON-zero count; an expectation of
non-zero and 7 days of flat zero cannot both be right. The counter increments only when
`unverified_seen > 0`, so it had no denominator and could not tell "every binding examined
was cross-signed" from "no attribution join reached a binding."

Cure landed: `elohim_attribution_joins_total` + `elohim_attribution_bindings_examined_total`
(posture-labelled, incremented UNCONDITIONALLY) and `AttributableBindings::examined()`,
counted before the posture filter so `Enforce` cannot shrink the denominator it is judged
against. **The gate is now the CONJUNCTION `examined > 0 && unverified == 0`.** Still
unknown until one deploy carries the counters: whether the joins are absent or the bindings
are. Three production callers exist (`api/reciprocity.rs`, `graphql/resolvers.rs`,
`services/cluster_view.rs`) and `/api/v1/reciprocity` answers 200 on alpha, so "no caller"
is not the obvious explanation. Read all three series together, never the numerator alone.

Decomposition + the 4-lens red-team review this implements:
`genesis/data/timeline/backlog/agent-peer-binding-signing.md` (C2-S1..S7).
Related: [[feedback_reach_head_replication_distinct_planes]],
[[feedback_concurrent_sessions_shared_worktree]].
