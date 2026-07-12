---
title: Cold Outside Review — a stranger P2P expert on "a k8s-like substrate for P2P"
id: cold-outside-substrate-review-2026-07-12
date: 2026-07-12
status: reference
author: adversarial cold-review agent (Opus, context-blind), commissioned Fable session close
kind: review
---

# Cold review: "a k8s-like substrate for P2P"

> Commissioned deliberately context-blind: an expert who has never seen the
> project, owes no politeness, and is incentivized to find missing primitives
> — not to admire the arc. Verbatim below; the roadmap consequences are in
> the sibling doc `opus-handoff-roadmap`. The one-line diagnostic to carry:
> **the test rig has no NAT, no scale, and no adversary — so the three things
> that will actually kill this are exactly the three CI cannot currently see.**

## 1. What's the container here?

The unit of trust is the **notarized content head** — an EPR/content id
resolved to a `head_action_hash` the *receiving peer's own conductor*
verified (I1; `heal_content_one` at `projection_reconcile.rs:985`, the
`StampMode::Declare` vs `GapFill` split at `:1014-1018`, test at `:1164`).
Well-chosen; the best idea in the repo; load-bearing in code, not aspiration.

But the unit of *deployment* is muddy and the reconciler is half-primitive,
half-pile. The declared-state↔reconciler pair exists as a named primitive
(`reconcile_rails::GapTracker` — "ONE controller pattern … a parallel bespoke
fetcher is a coherence violation"), and P1 ("DHT is the manifest; storage is
a k8s-style controller") is the correct analogy, ~40% realized. But ~30
files match reconcile/heal/sweep/backfill in `elohim-storage/src`; some
compose onto the rail (`witness_bootstrap` reuses `reanchor_backfill`), many
are bespoke loops wearing the reconciler's vocabulary. The tell: it took
FIVE stacked invisible defects to converge ONE head across TWO doorways. A
mature reconciler has one failure mode with one probe; theirs needed a
hand-built 4-seam smoke written *during* the incident.

## 2. What's the real value?

Stripped of mission language: **socially-notarized canonical heads with
conductor-terminated verification** — any edge serves any content iff its
own conductor validated the head, and trust travels *with* the content as a
first-class label. Nearest neighbor is ATProto + a per-consumer verification
requirement, which nobody ships. IPFS (no canonical head / no vouching),
Ceramic/OrbitDB (CRDT, no notarization/reach gate), Matrix (room DAG, no
content-address trust) all lack it.

Load-bearing status, honestly: **trust-label verification is real** (I1/I2/I3
enforced). **Earned authority / earned reach is mostly aspirational** — reach
enum is three-way drifted (`project_reach_enum_drift`), and HTTP-path reach
enforcement is a *filed gap* (`http-reach-enforcement-gap.md`) on the exact
surface a web2-projection product uses. **REA value-flow is a slide deck with
a few structs behind it** — per-view on-DHT notarization is admitted "absurd";
the rollup/aggregation shape is undesigned.

## 3. K8s-like powers — gap list

| Primitive | State | Evidence |
|---|---|---|
| Reconcile vs declared state | Partial (best thing) | `reconcile_rails.rs`; ~30 heal loops |
| Liveness/readiness | Weak — SELF-asserted | `heartbeat.rs` publishes `PeerStatus` from the node's own policy; no independent detector |
| Scheduler/placement | Nascent, INERT in prod | `reconcile/placement.rs`; `household_id` NULL → degrades to XOR |
| Admission control | Design-thin | `doorway-membrane-prosocial-routing-design.md`; per-request routing, not resource admission |
| Horizontal scaling | MISSING where it counts | `target_arc_factor=1` = full corpus per node; `arc-factor<1` is the named-but-UNBUILT lever |
| Declarative rollout/rollback | MISSING for content-plane | DNA change re-keys/partitions; coordinator hot-swap is the only safe path; no canary/rollback |
| Namespace isolation | Partial | one DNA = one integrity space; fractal domains are design |
| Resource quotas / GC | MISSING | §4 |

**Three that matter FIRST at 10-100 nodes:** (1) a real failure detector
(not self-report); (2) horizontal scaling = ship `arc-factor<1`; (3) storage
quota + GC/eviction (full-arc + monotonic corpus = an OOM schedule; conductor
already OOM'd 2→4GB).

## 4. Missing X, Y, Z (the hard ones)

- **Membership/failure detection:** self-asserted only; no phi-accrual/SWIM/
  suspicion. A partitioned or lying node looks Online until a human reads Loki.
- **Sybil/eclipse resistance:** effectively absent at transport; every hit is
  a design doc. R-nearest-by-hash (the scale-envelope's own model) is exactly
  what eclipse targets, with no filed defense. For a *trust* system this is
  the sharpest conceptual hole.
- **Identity coherence — the sharpest LIVE wound:** three namespaces
  (`agent_cid`/libp2p/iroh) that silently empty joins if crossed; the resolver
  is specced-but-BLOCKED; bindings are self-asserted/unsigned
  (`STAGE1_SIGNATURE_SENTINEL`). Their own doc: do NOT consume bindings for
  economic attribution until cross-signed. **This gates the entire REA
  value-flow story** — you cannot pay creators when "who served" isn't
  cryptographically bound to "who authored."
- **Back-pressure:** thoughtful INTRA-node pacing (`WITNESS_MAX_PER_TICK=200`,
  25ms spacing, 120s budget) but no inter-peer fairness/QoS.
- **Storage economics/GC:** custody/quilt/pantry vocabulary exists; NO
  eviction, NO per-node quota, NO GC of superseded heads/orphaned blobs found.
  Will hit disk before the scale ceiling (already at 85%+ chronic PVC).
- **NAT reality:** the zero-ICE-for-weeks incident proves the **test rig has
  no real NAT** — a totally broken consumer path passed every gate. At
  consumer scale ~80% are behind symmetric/CGNAT; TURN-relay is the common
  case, and there's neither a sovereign relay nor a test that would catch its
  absence.
- **Key rotation/compromise recovery:** scaffolded (`recovery_rotation.rs`,
  KeyRotation entry) further than most pre-scale projects, but the end-to-end
  compromise→rotate→re-establish flow is not demonstrated.
- **No versioned rollout of the content-plane logic itself** — the churning
  reconcile/heal code has no canary/staged rollout/rollback. Backwards for a
  control plane.

## 5. This won't scale until ____

**PRIMARY:** until sharded arc (`arc_factor<1`) is built, measured, and the
notary-neighborhood math is validated against eclipse.
- *Falsifiable:* "One DHT space with kitsune2 sharding holds convergence +
  read-latency SLOs at N=1000, per-node working set bounded, while an
  adversary controlling k identities cannot become the sole authority
  neighborhood for a target EPR."
- *Experiment:* 1000 conductors, corpus 100× a node's RAM, arc_factor bounding
  holdings; measure (a) cold-id head-convergence + p99 read, (b) eclipse trial
  — k Sybils near a target hash, does an honest reader still converge to the
  true head. Either failure falsifies "one DHT space scales to 7B." **No rig
  can run this today.**

**SECONDARY:**
- until an independent failure detector exists (kill -9 mid-heartbeat +
  partition; measure serve/place-against-dead-peer duration).
- until storage GC+quota exist (run a node against an appending corpus a week;
  monotonic disk with no eviction knee = false).
- until identity binding is cross-signed (attempt to claim another agent's
  served-bytes credit via a false `AgentPeerBinding` — today it SUCCEEDS).

## 6. What they got right (earned)

- Content-addressed, conductor-terminated trust (the right north star, built).
- The reconcile-rail intention (`GapTracker`; composing not forking).
- "Every trust claim gets a probe, every probe names itself" doctrine,
  instantiated in the seam smoke.
- Brutal internal honesty — docs lead with their own scale doubt and flag
  their own drift instead of hiding it.
- Recovery/rotation entry types actually scaffolded.

**Bottom line:** the trust primitive is real and worth building on; the
"k8s-like control plane" is one shared reconcile rail surrounded by ~30
bespoke heal loops, no failure detector, no scaling lever, no GC, unsigned
identity binding under the economics story. Build the 1000-node adversarial
NAT rig FIRST — everything else is unfalsifiable until that experiment runs.
