---
name: project_trust_as_performance_primitive
title: "Trust: security AND performance — the compute/trust gradient"
description: "Security and performance COMPOSE — the compute/trust gradient makes high-trust peer edges fast while commons browsing stays witnessed-safe."
metadata: 
  node_type: memory
  type: project
  originSessionId: c87b3bc9-e95e-42be-bb11-a094d8c482c6
  modified: 2026-08-20T14:11:45.255Z
---

Operator's active design trajectory across the quiesce/dataplane work (named
2026-08-20, corrected the same day). Canonical statement:
`trust-as-efficiency-signal.md` — "trusted content must measurably cost less to
propagate."

**Trust is BOTH a security and a performance primitive, and the point is that they
COMPOSE.** Do not frame it as trust being promoted from one to the other — that
either/or IS the error. The composition yields the **compute/trust gradient**: compute
cost falls as trust rises, because one witnessed trust act both *authorizes* (safe to
serve to a browsing human) and *prices* (cheaper to verify and propagate).
Verification done once, socially, amortizes into both planes instead of being paid
twice — which is why speed here is not bought at safety's expense.

Together they give a performant p2p-dataplane substrate AND a safe, trusted commons
browsing experience **backed by real witnessed provenance**. Provenance is what makes
the gradient *earned* rather than configured — the difference between this and a CDN
with an allowlist. See [[project-eprfs-witnessed-interaction-primitive]].

**The operational target is the day-to-day edge.** Leverage high-trust peer
relationships to make ordinary commons browsing blazing fast. Expensive ceremony
belongs at the boundary — first contact, unknown peer, unwitnessed content, contested
claims — never on the hot path.

## The head-as-manifest model (operator, repeated — hold it)

- The **declared head is a MANIFEST** — desired state, exactly like a k8s manifest.
- **AMBER = the manifest is readable.** This is the floor for BOTH serving and sync.
  libp2p/iroh/sqlite push bytes as fast as the architecture allows off the declared
  head the moment it is readable. Corroborated in code: every external HTTP boundary
  passes `MinTrust::Amber`; `MinTrust::Green` has **zero** production callers.
- **GREEN = the Holochain DHT has synced on that manifest** — the **lagging** trust
  indicator, notarizing that the head is **socially agreed**. Green is *allowed* to
  lag; nothing on the data path waits for it.
- **Never gate the fast lane on the slow lane.** Gating sync, serving, or dataplane
  MEASUREMENT on `caughtUp` / reconcile-drained / converged is gating amber work on a
  green fact. (2026-08-20: `scripts/ci/fleet-quiesce-gate.sh` does exactly this — its
  content-200 legs are amber and correct, its `A_CAUGHT_UP` + `QUIESCED_OK` legs and
  its "sweeps must advance" sustain requirement are green.)

**The newer primitive being explored:** resolve the **peer relationships across EPR
head-sets BEFORE byte sync**, down at the libp2p/iroh/sqlite layer — so round-trips
scale **O(peers), not O(EPRs)**. <20 peers against 4000+ EPRs is the whole leverage.
This is the substrate half (head-set digests / signed snapshots / set reconciliation),
distinct from the measurement half (the CI gate).

**Binding constraint:** the reach ceremony is architecturally real, so speed comes
from **compressing** it, never skipping it. A proposal that goes fast by weakening a
check is the wrong shape.

Three compressions, all legitimated by *relationships*:

1. **Per-peer, not per-EPR** — one trust act covers a peer's whole advertised corpus;
   cost O(EPRs) → O(peers). Valid because <20 peers know each other and pre-agreed to
   replicate. The scale asymmetry IS the leverage: bulk set reconciliation, not
   per-item negotiation.
2. **Custody is not readability** — holding ≠ decrypting, so the replication set is
   identical across peers and a set-level digest *cannot lie*. That moves the digest
   onto the replication plane, off the head plane ([[feedback_reach_head_replication_distinct_planes]]).
3. **Staging/dev toggles are the declaration layer** — `NetworkStage`
   (Simulacra<Bootstrap<Coordinated<Enforced), fixture frontmatter declaring required
   trust, deploy-time grant minting, cluster-state ↔ `ELOHIM_REMOTE_COMPUTE_STATUS`.
   Cost model follows the declaration. Failure mode: a **stale declaration** silently
   narrows scope.

**The inversion to watch:** "tolerate it / retry longer / ride the shed" is what you do
when you cannot *price* a relationship. A trust-graded system does the opposite — it
knows an edge is trusted and treats failure there as *more* significant. Reaching for
tolerance is a tell that the pricing signal is missing; fix the signal first.
**You cannot price a relationship you cannot observe** — so an error that erases which
edge failed and how is a performance defect, not just a logging one.
