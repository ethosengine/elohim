---
index: false
name: feedback_reach_head_replication_distinct_planes
title: Reach ≠ content_head ≠ replication — three orthogonal planes
description: "Reach (audience, earned) ≠ content_head (version, declared) ≠ replication (availability, custody) — three orthogonal planes; landing-page divergence is a replication bug, not head election."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: def7446b-f76b-492e-8464-1dfb0da18ef6
---

Operator ontology-guard (2026-07-09), given while refocusing C3. AI agents (me) reached for "content_head election" to explain the `elohim-host-landing` divergence and let the head-declaration signal lean on reach/attestation — fusing two independent concepts. Keep three planes strictly separate:

- **Reach** — *who may see this EPR* (audience/visibility scope). Earned via attestation/standing, amber → … → **commons**.
- **content_head** — *which VERSION is canonical*. Declared via `declare_content_head` (authority over THIS EPR's versioning).
- **Replication / custody** — *how many peers serve the head's bytes* (availability). Custody commitments + salvage.

**Why:** they're independent. An EPR at commons reach still has exactly ONE head; a private-reach EPR also has a head. Earning commons reach ≠ declaring the head ≠ replicating it.

**How to apply:**
- `elohim-host-landing` = ONE commons-reach head that must be REPLICATED across both doorways. The measured two-`dhtAnchorHash`/two-`blobHash` split is a **replication-coherence** bug (per-host deploy build+notarize fractured one head), NOT a fork needing election and NOT a reach problem. Cure: one build → one head → replicated (replication plane + single-notarized-head arc).
- **C3 `resolve_head` must be reach-CLEAN**: serve *the* declared head (declaration-over-recency), no reach coupling.
- Head *election among competing heads* is a FUTURE sub-commons fork scenario only — deferred, see backlog `content-head-election-vs-reach-fork-arbitration.md`. Don't build it before a real fork + the earned-authority criterion exist. Relates to [[project_earned_reach_governance_pr_ceremony_vision]] and [[feedback-identity-sovereignty-ontology-guard]] (the same "don't collapse distinct authority concepts" discipline).

## Sharpening (2026-08-20): custody ≠ readability, and what it unlocks

Architect directive during quiesce-acceleration design. The three planes carry a **performance
contract**, not just an ontology:

- The **DHT is the notary** — it supplies the notarized trust signal and is *allowed to be the slow
  long-tail plane* (amber → green over time).
- The **byte plane must not inherit notary latency**. Getting blobs present → addressable →
  deliverable over iroh/SQLite should be limited by nothing in the head plane.
- **Scale asymmetry is the leverage**: <20 peers who know each other and agreed up-front to replicate
  most of what each other hold, against 4000+ EPRs. Per-item election among consenting peers is the
  wrong shape; bulk set reconciliation is.
- **Custody is not readability** — sharded data need not be decryptable by the replica holder.

**Why this matters:** it dissolves the standing objection to set-level/bucketed digests. That objection
was that reach is earned per-node, so two honest peers legitimately hold different visible sets and any
shared digest would lie. If custody is universal (encrypted shards) and reach governs *decryption and
serving* rather than *holding*, the replication set is identical across peers and the digest cannot lie
from reach differences.

**How to apply:** put set-level digests on the **replication** plane, never the head plane. Invariant
that survives: a peer holding ciphertext it cannot read gains **no** reach — serving still requires
authorization. And when scoping quiesce work, ask which plane the latency is actually in before
optimizing; measured 2026-08-20, the trust pricer's cheap corner is gated on
`declared_head_action_hash.is_none()`, so identity-trust pricing structurally cannot touch the declared
population that dominates post-restart cost (see [[project_alpha_substrate_probe_rails]]).
