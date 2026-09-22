---
name: project_trust_as_performance_primitive
title: "Trust = security AND performance"
id: project-trust-as-performance-primitive
description: "Security and performance COMPOSE: trusted peer edges run fast while commons browsing stays witnessed-safe."
metadata: 
  node_type: memory
  type: project
  originSessionId: c87b3bc9-e95e-42be-bb11-a094d8c482c6
  modified: 2026-09-17T18:12:55.492Z
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

## Pay the cost once, up front, next to the human judgment (operator, 2026-09-17)

The operator names this the **foundational architectural enablement story of the
protocol** — something the architecture, and we, seek to *master*:

- Verification cost is paid **up front, as close to the human judgment as possible**,
  and **as few times as possible**; everything downstream relies on that minted trust
  rather than re-deriving it. Validation is *carried as it climbs* (reach), not redone
  per layer.
- Leaning on the complex dimensions of trust to drive real performance **lowers cost
  and increases reach at once**. Where trust buys performance at each layer should feel
  like a **natural consequence of the story**, and the gradient should be something one
  can **explore, play with and feel apply** — legible, not buried.

**Why:** said while reviewing the sync-triggered head-adoption fix: the head was
quiescing as its OWN property (content crosses by sync in <1 s, head authority trails
through a second convergence, adoption polls for it). The operator's read — one
verified copy from a trusted peer should settle it; compose the head when the other
trust is minted so there is no second holon quiescence on one property.

**How to apply:** a design smell checklist — (1) a property that converges separately
from the holon it belongs to; (2) the same fact verified again at a lower layer;
(3) retry/poll ladders standing in for a carried proof; (4) a fast path that is
trust-blind (same cost for a household steward and a stranger). Prefer: sync unit =
`{content, signed head record}` verified once and projected atomically; carried
records over re-resolution; per-relationship pricing over uniform tolerance. When a
conservative slice ships first (e.g. local-resolve + re-probe ladder), measure how
often the ladder is needed and name the carried-proof version as the next node.
See [[project_recall_reaches_authority_habit]], [[feedback_atomic_wins_compound_velocity]].

**Horizon the operator named the same day (after "the basics are mastered"):** dev
mode / tests inject *modeled contexts and stories* into a simulacra network; peers
resolve them; as network inference matures, negotiation and automated moderation flow
over many stories — and the run **proves the compute/trust correlation**: something as
complex as bad faith shows up as an *emergent shape in the edge compute-cost curve*,
while the network creates and protects high-fidelity, high-trust commons pools. Today's
work is "the mechanical minimum" toward that. Design implication now: every trust
decision on the dataplane should emit its *price* per edge (which path, how many
verifications, what relationship) so the curve is measurable later — see the
`peer_class` finding below. Keep the framing guards: the curve is a *cost signal that
informs Mishpat*, never a verdict or punishment.

**Operator correction (2026-09-17) to my "starved vs bad-faith look alike" caution:**
mechanically true on the curve, but the answer is not better classification — it is
the **deterministic floor / elohim ceiling** split. The floor prices edges
deterministically and blindly. The ceiling is relational: a peer starved of
relationships builds faith *through relationship with an elohim agent* (local
inference), and in a mature network elohim accelerate trust-building among
participants so **the community picks up the burden of a valid trust curve**. Good
faith is what accelerates the frictionful bits; trust becomes a **positive feedback
loop** that reaches back even to an intermittent rural peer. Only the mature network
delivers this — do not try to make the floor solve it, and do not read a high-cost
edge as a verdict: it is an *invitation for the ceiling to engage*. Design
consequence: the floor must expose cost and its cause legibly enough that an elohim
(or a neighbor) can see who is starved and extend relationship; the path OUT of the
expensive region must always exist and be earnable.

**Genesis stance (operator, same day):** right now WE are the ceiling. In genesis the
trust that a mature network would earn can simply be **declared** (NetworkStage,
steward relationships, declared external origins, fixture trust), and the job is to
**watch the floor respect the declaration**. So nothing on the trust axis is a "real"
blocker today — never park work waiting for earned trust or mature inference; declare
it, make the declaration explicit and observable, and verify the floor prices
accordingly. A stale or implicit declaration is the failure mode, not a missing one.

**Scope line (operator, same day):** the *mechanical* symptoms a struggling peer
causes — high CPU, noise, r/w storms — are NOT a trust or AI question. Graceful
network behaviour (backpressure, bounded queues, reactive streams, p2p negotiation,
peer status, admission, rate-limited refusal, honest readiness) is a set of
**deterministic primitives we must deliver with no online AI at all**. Never defer a
floor defect to the ceiling: a publish livelock, an unthrottled refusal storm, a
readiness probe that lies, or a sweep that starves its own fast path is plain
resilient-compute engineering, owed unconditionally.

**Measured 2026-08-28 (household mesh, binaries with the `peer_class` label):** every outbound sync request classifies `peer_class="public"` — the ambient trust handshake is a stub (sender: libp2p peer id as agent key, empty CID lists; receiver: asserts `agent_verified: true`, ceiling `public`, never calls `verify_trust_context`, which has zero callers). The gradient's pricing input does not exist on the dataplane yet; the label makes that absence measurable. Missing node: verifiable identity + relationship CIDs in the handshake, blocked on the transport-id→agent_cid resolver (self-asserted bindings). Backlog: sync-edge-susan-timeouts-per-edge-observability.
