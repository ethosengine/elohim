---
title: Staggered Conductor Restart Mitigation
id: staggered-conductor-restart-mitigation-design
status: Draft
class: operations
context-tier: disclosed
steward: cartographer
graduation-trigger: ratified-and-decomposed OR superseded-by-implementation
created: 2026-08-06
cites:
  - genesis/data/timeline/backlog/staggered-conductor-fleet-restarts.md
  - elohim-seam-map-concern-routing | Routes rollout classification to OS/packaging and wave admission to resource governance. | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - substrate-trust-contract-runbook | Supplies the hours-scale catch-up distinction and the fresh-sweep quiescence predicates reused by the proposed wave gate. | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---

# Staggered Conductor Restart Mitigation

## Outcome

An edge deploy may update provenance without restarting a conductor. When a
runtime-effective input changes, every affected peer still converges to it, but
the deploy advances in bounded waves and proves the restarted wave has rejoined
the serving set before applying the next wave.

This package is design plus standing-red evidence. It does **not** alter the
live deploy implementation. Until the armed transport flip lands, the write
fence excludes `elohim/holochain/Jenkinsfile`, both human StatefulSet sources,
and `doorway/**`.

## Seam placement

The concern composes two seams:

- **OS / packaging:** rendering and applying a StatefulSet is the deploy-target
  concern. A `RollingUpdate` starts when `.spec.template` changes during apply.
- **Resource governance:** wave admission and the convergence gate decide when
  the fleet has enough healthy capacity to risk the next disruption.

It is not hub-internal cluster ops: Kubernetes is the current packaging surface,
not the protocol's future household-hub swarm. It is not a new dataplane or
doorway feature either; existing service probes measure whether those surfaces
survived the packaging operation.

## Two hard constraints from the current implementation

1. The genesis guard does not detect a pod-template change. It detects only the
   raw `kubectl apply` word `unchanged`; any other StatefulSet result reaches an
   explicit `kubectl rollout restart`, and non-genesis peers restart regardless.
   Moving the Git SHA from pod-template metadata to StatefulSet metadata would
   still produce `configured`, so label relocation alone cannot cure the roll.
2. Both StatefulSets use `RollingUpdate`. A real `.spec.template` change begins
   rolling during `kubectl apply`, before the explicit restart command. Applying
   all seven manifests concurrently and staggering only `rollout restart` is
   therefore already too late.

The deterministic standing reds in
`genesis/orchestrator/design-tests/staggered-conductor-fleet-restarts.red.mjs`
pin both facts.

## Options

| Option | Shape | Strength | Cost / failure mode | Verdict |
|---|---|---|---|---|
| A. Runtime-content fingerprint + wave-sequenced apply | Hash the runtime-effective render, stamp that hash into the pod template, classify no-op from the hash, and run `apply → rollout → convergence probe` one wave at a time | One change detector controls both automatic and explicit rollout; preserves `RollingUpdate` | Needs a canonical render projection and a direct, fallback-free peer probe | **Recommend** |
| B. `OnDelete` + explicit wave controller | Applying a template never rolls a pod; the pipeline deletes one bounded wave after deciding it changed | Maximum sequencing control | Changes normal StatefulSet semantics; a controller failure can leave declared and running revisions split indefinitely | Hold as fallback |
| C. Live pod-template diff + wave-sequenced apply | Compare candidate and live runtime projections, then apply only changed peers by wave | Avoids a stored fingerprint as the primary classifier | Server/diff output becomes deploy authority; mounted ConfigMap payloads still need explicit inclusion | Reject unless A's canonicalizer proves infeasible |

Two partial mitigations are explicitly rejected: moving only the version label,
and sequencing only the explicit restart command. Each violates one of the hard
constraints above.

## Recommended contract

### Runtime-content fingerprint

The classifier is a digest of **runtime-effective inputs**, not Git history and
not the complete multi-document manifest:

```text
runtimeFingerprint = sha256(canonical(
  podTemplate minus volatile provenance fields,
  digests of mounted rendered configuration payloads,
  resolved hApp content digest
))
```

The canonical projection includes container images, commands, runtime env,
resources, volumes, and configuration payloads read at process boot. It excludes
the deploy Git SHA, timestamps, top-level Kubernetes metadata, Services,
Ingresses, and other provenance-only changes. The fingerprint annotation itself
is removed before hashing.

The projection must grow from the existing rendered inputs; it must not create a
second hand-maintained inventory of conductor-affecting fields. A failure to
resolve an image or hApp digest remains fail-safe: mint an explicit unresolved
marker, classify the peer as changed, and stop if the deployment cannot prove
which bytes it is rolling toward.

No-op behavior becomes independent of `kubectl` wording:

- candidate fingerprint equals the live fingerprint → apply metadata if needed,
  but do not explicitly restart;
- fingerprint differs → enter the wave controller;
- live fingerprint missing → treat as changed for the first migration deploy.

### Wave-sequenced apply

The controller computes the complete wave plan before mutating the cluster.
Within each wave it may operate peers in parallel, but it never applies a later
wave until the current one has passed both rollout and service convergence.

Initial conservative policy:

1. use one-peer waves until the arc/coverage overlap proof exists, beginning
   with one non-genesis canary;
2. keep Adam and Matthew in different waves;
3. retain at least one already-converged full-coverage provider for every
   serving-critical projection throughout a wave;
4. permit wave size greater than one only after an explicit arc/coverage overlap
   proof says the remaining set can answer the fleet's critical reads;
5. abort without applying the next wave when convergence misses its deadline.

The operation is therefore:

```text
render and classify all peers
→ compute coverage-safe waves
→ for each changed wave:
     apply only this wave
     wait for Kubernetes rollout completion
     wait for service-level convergence
     record evidence
→ finish
```

Applying all manifests first is forbidden: `RollingUpdate` makes apply itself
the disruptive action.

### Convergence gate

Kubernetes Ready proves that a process booted; it does not prove that its
conductor can serve a DHT-dependent request. Likewise, a doorway-level request
may now succeed through ZomeCaller fallback and accidentally certify the wrong
conductor.

The wave gate must therefore be peer-direct and fallback-free. Its minimum
contract is:

1. the restarted peer completed its StatefulSet rollout;
2. target-direct `/p2p/status` reports a non-null
   `projectionReconcile.converged: true`, a post-restart sweep count, and the
   existing peer-mesh floor;
3. target-direct `GET /db/content/<canary>/head-record` returns the same
   `headActionHash` as a known non-wave survivor; unlike the projection-only
   `/head` read, `head-record` resolves the projected hash and fetches the action
   through that target's conductor;
4. both public doorway projections still serve the canary and agree on its head;
5. the target's passing observation is sustained across a fresh reconcile
   sweep—the sweep counter must strictly advance—before the next wave applies.

Generic progress must not depend on acquisition `pull.caughtUp`: an empty or
all-retired pull queue can report false by design, and retry exhaustion can make
the flag true without projection convergence. Null, missing, or unreachable
projection/conductor fields fail closed.

`scripts/ci/fleet-quiesce-gate.sh` already supplies bounded polling, exact metric
matching, content-serving checks, and the two-observation/fresh-sweep proof. The
wave controller should reuse those parsing and sustain semantics, not invoke the
script unchanged: its current contract is fleet-level, A-biased, fallback-capable,
and returns a non-failing no-measure outcome on deadline. It protects downstream
measurement honesty; it does not preserve availability while peers roll. A deploy
admission gate must target the restarted peer directly and fail closed on deadline,
leaving every unstarted wave untouched.

## Standing-red contract tests

The standalone Node test is intentionally excluded from the orchestrator's
default green suite until implementation begins. It reads the fenced sources
without modifying them and records today's behavior:

- changing only `DEPLOY_VERSION_PLACEHOLDER` changes both pod templates (red);
- a real hApp digest change changes both pod templates (green control);
- restart admission still keys on the raw `unchanged` apply word (red);
- all active humans enter one `parallel(branches)` apply group (red);
- the all-human dispatch contains no convergence admission step (red).

Implementation must add behavioral sequencer fixtures for stale-gauge rejection
(converged twice without a sweep increment), projection-only false green (`/head`
200 while `/head-record` fails), survivor loss, and the intentionally false
`pull.caughtUp` case. Those tests belong with the future pure wave planner rather
than source-shape parsing in this design-only diff.

Run it explicitly:

```bash
node genesis/orchestrator/design-tests/staggered-conductor-fleet-restarts.red.mjs
```

The expected result before implementation is non-zero with the named invariant
failures. Once the flip batch lands and implementation is authorized, first
extract a pure runtime-classifier and wave-planner seam, then replace source-shape
assertions with unit tests over those functions. Only after every red flips green
should this file join the default orchestrator test list.

## Ratification questions

1. Which dependency-free canonicalization can run in the edge deploy container
   without recreating a fragile YAML parser in shell?
2. Which peer-direct conductor-backed probe works for every human without being
   masked by doorway fallback?
3. What measured coverage proof permits waves larger than one, and which roles
   must never share a wave?
4. Does a deadline abort leave the already-upgraded wave serving safely enough
   to retry, or must the controller offer an explicit rollback policy?

## Graduation

This design graduates when the operator ratifies the fingerprint projection,
wave policy, and direct convergence probe; implementation tasks are decomposed;
and the standing reds are attached to a green default suite. Until then it is a
collision-safe proposal, not deploy authority.
