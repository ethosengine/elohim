---
id: "backlog-agent-peer-binding-cross-signed-proof"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "AgentPeerBinding is self-asserted (STAGE1 sentinel) — economic ATTRIBUTION must not join through it until a cross-signed binding proof lands"
slug: "agent-peer-binding-cross-signed-proof"
written: "2026-06-20"
author: "claude (doorway-membrane Wave-2 sprint — security backlog routed from the coherent-transport-identity resolver recon §0:41-48)"
status: "open"
priority: "high"
tags: [security, attribution, agent-peer-binding, transport-identity, cross-signature, reciprocity, recognition, toll, serve-routing, p2p-design-gate]
cites:
  - genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md
  - elohim/holochain/dna/zomes/imagodei_integrity/src/agent_peer_binding.rs
  - elohim/elohim-storage/src/reconcile/controller.rs
  - elohim/elohim-storage/src/views/reciprocity_view.rs
  - elohim/elohim-storage/src/views/cluster_view.rs
  - genesis/docs/superpowers/specs/2026-06-22-imagodei-profile-page-viewer-lens-design.md
relatedNodeIds:
  - backlog-resilience-card-self-cid-provide-loop-gate
---

# AgentPeerBinding cross-signed proof — the missing trust root for attribution

The coherent-transport-identity resolver recon (`2026-06-15-coherent-transport-identity-resolver-design.md`
§0, the LIVE SECURITY ISSUE at lines 41-48) routed this here. It is not deferred
speculation — it is a standing property of bindings that are **already consumed for
attribution today**.

## The gap (code-verified at the writers)

The `AgentPeerBinding` entry asserts a `(agent_cid, peer_id)` pair — "agent X owns
transport peer Y." Today that assertion is **self-asserted and unsigned in the
load-bearing sense**:

- **Integrity validation checks only shape, not control.** `imagodei_integrity`'s
  `validate_create` for the binding checks non-empty fields + an ordered validity
  window + a *non-empty* signature field — it does **not** verify a cross-signature
  proving control of both identities (`agent_peer_binding.rs:155-190`).
- **The gossip publisher writes a `STAGE1_SIGNATURE_SENTINEL`** — a placeholder, not a
  real signature (`reconcile/controller.rs:623`).
- **The "proof seam" `synthesise_dht_anchor_hash(peer_id, agent_cid)` is `sha256(peer_id+agent_cid)`**
  — a deterministic *row key*, not an authorization proof.
- **The two keys are independent.** The libp2p transport keypair and the Holochain
  agent (`uhCAk…`) key are generated independently; nothing today makes one sign over
  the other, so neither side proves it controls the other.

Consequence: a gossiped spoof `(agent_cid = attacker X, peer_id = victim Y)` passes
validation. Any join that resolves "who is peer Y" → X, then credits Y's economic
activity to X, has been spoofed.

## Why serve-routing's USE is SAFE; why ATTRIBUTION is NOT

The distinction is the whole point — do not over-scope the freeze:

- **SELECTION (serve-routing dial choice) is SAFE.** Content-addressing bounds the
  worst case. If a spoofed binding routes a dial to the wrong peer, the peer either
  serves the content-addressed bytes (which verify by hash) or it does not — a failed
  dial falls back to another candidate. A spoof costs at most a wasted dial, never a
  corrupted result. Routing MAY consume the unsigned binding for *selection*.
- **ATTRIBUTION (joining economic facts onto `agent_cid` through the binding) is NOT
  safe.** Toll receipts, appreciation, recognition rollups — these *assign value/credit
  to an identity*. A spoofed binding here silently credits the wrong agent, and the
  error is durable (it lands in the REA projection / recognition rollup), not transient.
  Bindings are already consumed on the attribution path: `reciprocity_view.rs:48-58`
  and `cluster_view.rs:252`. **Attribution MUST NOT join through the binding until a
  cross-signed proof exists.**

The cut line: a binding may decide *where to dial*, never *whom to pay/credit*.

## Why it matters for the arc

Everything downstream joins on `agent_cid`:

- The **Wave-4 toll / recognition layer** assigns economic credit per agent — its entire
  correctness rests on "this transport activity belongs to that agent."
- **Serve-routing recognition** (the appreciation/recognition rollup for who served
  what) joins served-bytes → serving agent.
- The coherent-transport-identity **resolver** itself (the spec this came from) is
  explicitly demoted to "speculative until a cross-signed control proof lands" — and
  the resolver is precisely the component that would gate custody/economics on these
  bindings. It cannot be trusted for that until this proof exists.
- The **imagodei profile/page viewer-lens arc** (`2026-06-22-imagodei-profile-page-viewer-lens-design.md`)
  is the newest dependent: its reflexive "how the network sees you" feed is the
  attribution-bearing facet. That arc **severed this as leg 3** (operator decision
  2026-06-22) and renders the feed honestly-caveated ("observed, not proven") meanwhile —
  it does NOT join attribution through the binding until this proof lands. This backlog
  item is that arc's gate for *cryptographically-proven* recognition attribution.

The unsigned binding is the **missing trust root** for all of it. Until it is real,
the attribution joins either stay off the binding or accept a spoofable input.

## Design gate

This is data-entity-bearing (a new proof field / validation rule on the
`AgentPeerBinding` integrity entry, plus the cross-signing emit path). Run the
`p2p-design-gate` skill before proposing the design: the binding is already a
Notarized (A) entry — no new entry type — so the work is (a) what cross-signature the
integrity validation requires, (b) which coordinator mints it (the node's self-emit
path that signs its own `(agent_cid, peer_id)`), and (c) the rotation/lineage
interaction (`superseded_by`) so a recovered key inherits standing without re-proving
from zero. The resolver spec §3.5 / §6 carry the lineage constraints to honor.

## Status 2026-08-18 — the cut this item asks for is BUILT; the proof it gates on is not

The last bullet of "Done-when" below ("until then: attribution paths either do not join
through the binding, or are gated behind the proof") is now **implemented rather than
merely observed**: `db::peer_identity_bindings::list_attributable_for_agent` is the gated
door, `AttributableBindings` makes the gate a type rather than a discipline, and the
attribution consumers named in this doc (`reciprocity_view`, `cluster_view`) take it. See
`agent-peer-binding-signing.md` § "Landed 2026-08-18" for the full slice.

Two honest qualifications, because this item is easy to read as closed and is not:

1. The gate ships in **`observe` posture by default** — it counts what rides through
   (`elohim_attribution_unverified_bindings_total`) but does not yet refuse, because no peer
   can mint a proof (C2-S2). Enforcing before minting would blank every economic surface
   without making anything safer.
2. The *first three* Done-when bullets — a real cross-signed proof replacing the sentinel,
   integrity-side `validate_create` verifying it, and attribution being allowed to trust the
   binding — remain open. Verification today is **receiver-local**, not notarized.

## Done-when

- A **cross-signed `AgentPeerBinding` proof** replaces the `STAGE1_SIGNATURE_SENTINEL`:
  the transport identity signs over the `agent_cid` **and** the agent key signs over the
  transport id (both directions — neither side can be claimed unilaterally).
- `imagodei_integrity`'s binding `validate_create` **verifies that cross-signature**
  (control of both identities), not merely a non-empty signature field.
- With that proof in place, attribution joins (`reciprocity_view.rs`, `cluster_view.rs`,
  and the Wave-4 toll/recognition + serve-routing recognition rollups) MAY trust the
  binding to resolve transport activity → `agent_cid`.
- Until then: attribution paths either do not join through the binding, or are gated
  behind the proof; only selection/routing consumes the unsigned binding.

## Boundary note

This is security/protocol-canonical, operator/security-owned — not an autonomous repo
fix. The cross-signing emit path also interacts with the TOFU/portal-handoff trust
model (a node minting a binding from its own keys asserts "I am agent X"); see the
blocked resolver design and the resilience-card item
(`resilience-card-self-cid-provide-loop-gate.md` Reconciliation 2026-06-18) where the
same "do not consume the self-asserted binding for economic attribution until a
cross-signed control proof lands" rule already bites the `commitmentBacked` column.
