---
title: Opus Handoff Roadmap — what the substrate still needs to be trustable at scale
id: opus-handoff-roadmap-2026-07-12
date: 2026-07-12
status: reference
author: Fable session close — synthesis of the cold outside review + known residue
kind: roadmap
---

# Opus Handoff Roadmap

For the agents (Opus, GPT) inheriting this substrate. This ranks what remains
by **load-bearing-ness for trust at scale**, folds the context-blind cold
review (`cold-outside-substrate-review-2026-07-12`) together with the residue
this session already knew, and states scale claims as **falsifiable
experiments** so you can prove or kill each rather than believe it.

Read first: the trust contract (`substrate-trust-contract-runbook`) for what
holds *today* and how to check it; the five-defect museum record
(`substrate-convergence-five-defect-arc`) for how it broke; the cold review
for what's missing. This doc is the "what next," ranked.

## The one reframing to internalize

The zero-ICE-for-weeks bug was not a config typo — it was a **rig diagnostic**.
The test topology has no real NAT, no scale, and no adversary, so the three
failure classes that will actually kill this in the wild (NAT traversal at
consumer scale, storage/scale limits, and adversarial identity) are exactly
the three nothing in CI can currently see. **Every "it works" below the app
layer is currently unfalsifiable.** The top roadmap item is therefore not a
feature — it is a rig.

## Tier 0 — finish the in-flight convergence (this session's tail)

Bounded, mechanical, mostly done; close it so the substrate has ONE demonstrated
converged head as the floor everything else builds on.

- [ ] Roll wave (edge >#1185) deploys the canonical-aware coordinator + heal +
      validator fix + guards to all 7 nodes. **Watcher `bt2aiwy14` owns this.**
- [ ] B adopts A's head via canonical-aware heal (or the 12-min retry-laddered
      `[build:app]` propagation). Verify with runbook step 4 (the Loki
      `heal left it to the canonical channels` vs `HEALED` discriminator) if it
      stalls.
- [ ] Bank notary scenario 2 green ×2 fresh edge validations.
- [ ] Apply the green-flip patch (`/tmp/green-flip.patch`): seam-smoke
      `--gate`, de-`@wip` the native-omni scenario. (Doorbell scenario stays
      `@wip` — its 3 step defs are unwritten; that's T4 glue.)
- [ ] One-line spine delta: notary-authority red→green with evidence.

## Tier 1 — the three that block scale (build the rig, then the levers)

Ordered by the cold review's "matters first at 10-100 nodes." Each is a
falsifiable experiment, not a feature request.

### 1. The 1000-node adversarial NAT rig (PRIMARY — everything else is faith without it)
Nothing below can be *proven* until this exists. Today's CI runs where peers
already reach each other; it cannot see NAT, scale, or Sybil.
- **Build:** a harness that stands up N conductors (containers are fine) behind
  *simulated* NAT classes (full-cone, symmetric, CGNAT — netns + iptables or a
  NAT-emulating relay), with a corpus knob (× a node's RAM) and a Sybil knob
  (k hostile identities near a target hash).
- **Falsifiable claim it must be able to test:** "One DHT space with kitsune2
  sharding holds head-convergence + p99-read SLOs at N=1000, per-node working
  set bounded, while k Sybils cannot become the sole authority neighborhood for
  a target EPR."
- **Why first:** the zero-ICE incident proves the current rig would pass a
  totally broken consumer path. This is the instrument that makes items 2-3
  (and the whole scale thesis) measurable.

### 2. Ship `arc_factor < 1` (horizontal scaling — the named, unbuilt lever)
`target_arc_factor=1` = every node holds the whole corpus; per-node RAM ∝
corpus; conductor already OOM'd 2→4GB. `arc-factor<1` is named "the scale
lever" and is unbuilt (kitsune2 upstream-gates fractional at {0,1} — that
constraint must be confirmed/lifted).
- **Experiment:** on the rig, set arc_factor to bound per-node holdings against
  a corpus 100× a node's RAM; measure cold-id convergence + p99 read. Monotonic
  RAM or blown SLO = the "one space scales" thesis is falsified.

### 3. Storage GC + per-node quota (the quiet OOM schedule)
No eviction, no quota, no GC of superseded heads/orphaned blobs found; full-arc
+ monotonic corpus + 85% chronic PVC = disk before scale ceiling.
- **Experiment:** run one node against a continuously-appending corpus for a
  week; plot disk. Monotonic with no eviction knee = false. Design an eviction
  policy keyed on custody commitments + reach (low-reach superseded blobs GC
  first) — the quilt/pantry vocabulary is the right home.

## Tier 2 — the trust story's load-bearing holes (block the economics pitch)

The differentiator that makes this "p2p YouTube with creator payments" is
**earned authority + REA value-flow**. Both rest on primitives that are
scaffolded-or-drifted, not proven. Until these land, the trust label is real
but the *economics* is a slide deck.

### 4. Cross-signed identity binding (gates ALL economic attribution)
Three identity namespaces (`agent_cid`/libp2p/iroh) silently empty joins if
crossed; the resolver is specced-but-BLOCKED; bindings are unsigned
(`STAGE1_SIGNATURE_SENTINEL`). Their own doc forbids consuming bindings for
attribution until cross-signed.
- **Experiment (falsifies the pitch today):** attempt to claim another agent's
  served-bytes credit via a false `AgentPeerBinding`. It currently SUCCEEDS.
  Not fixed until this fails cryptographically.

### 5. Reach vocabulary reconciliation + HTTP-path enforcement
Reach enum is three-way drifted (schema 8 / `reach_earning.rs` 8-different /
resilience-doc 5); HTTP-path reach enforcement is a filed gap on the exact
surface a web2 product uses. "Reach earns the right to serve" is designed with
a hole where it's consumed.

### 6. Independent failure detector (not self-reported PeerStatus)
`heartbeat.rs` publishes health from the node's own policy; a partitioned or
lying node looks Online until a human reads Loki. Placement + reconcile run on
self-report.
- **Experiment:** kill -9 a node mid-heartbeat + partition another; measure how
  long the fleet serves/places against the dead peer. > one detection interval
  = self-report insufficient. (SWIM/phi-accrual over the existing gossip.)

## Tier 3 — control-plane maturity (make the substrate safe to change)

### 7. Collapse the ~30 heal loops onto the ONE reconcile rail
The `GapTracker` rail is the right primitive at ~40% adoption; ~30
reconcile/heal/sweep/backfill files surround it, many bespoke. Five invisible
defects to converge one head is the symptom. Audit each loop: compose onto the
rail or justify divergence. A mature control plane has one failure mode per
concern, one probe each.

### 8. Versioned rollout/rollback of the content-plane logic
The churning reconcile/heal code has no canary/staged-rollout/rollback beyond
"redeploy and clear the churn window." Backwards for a control plane. The
coordinator hot-swap is the safe *channel*; it needs a canary *discipline*
(one node, observe seam smoke, then fleet).

### 9. Sybil/eclipse resistance at the authority neighborhood
Every current hit is a design doc. R-nearest-by-hash (the scale model) is
exactly the eclipse target. Needs a real answer before "socially-notarized
trust" survives contact with adversaries — the rig (item 1) is the test bed.

## Tier 4 — already-filed seeds (design spaces, cheap to verify, Opus-friendly)

Captured this session, wide-open, verification cheap — good Opus + operator
work, not blocking the floor:
- Sovereign TURN as Tier-A transport commons (`sovereign-turn-relay-transport-commons`)
  — and the utility-plane exit doctrine (borrow freely, no borrow without a
  filed exit).
- The doorbell scenario T4 glue (3 step defs) → then de-`@wip` scenario 5.
- Version-aware EPR-head chrome + reach-gated canary opt-in as the governance
  input that elects heads (`epr-head-chrome-version-aware-optin-canary-governance`).
- The scale-envelope design session (folds item 1's results into the seam-map
  atlas).
- content_store lib.rs + doorway http.rs modularization (both filed, pure-
  coordinator / native-Rust safe classes).
- Heal throughput smell (~10s/row × thousands post-restart = ~8h churn).
- eprfs/brit graduation of the five edit-time guards into content-addressed
  validator EPRs with cid+fuel (the point advisories become blocking).

## How to use this doc

Tier 0 is this session's tail — finish it for the demonstrated floor. Tier 1
item 1 (the rig) is the true unblock: build it and items 2, 3, 9 become
measurable instead of faith. Tier 2 gates the economics pitch specifically —
do it before promising creator payments. Tiers 3-4 are maturity and captured
seeds. The trust *primitive* is real and worth building on; this list is
everything between "one head converges across two doorways" and "an adversary
can't lie to a planet."
