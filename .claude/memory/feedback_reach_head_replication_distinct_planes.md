---
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
