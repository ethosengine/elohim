# Overnight Objective 2026-07-29 — deliver the saga (iterate against CI until the card tells one truth)

Authorized by operator: "be ready to push, and you'll have overnight to
iterate to try and deliver the saga." Push authority granted for this arc
(one batched push, then evidence-gated iteration pushes); the standing
commit-only default resumes when this arc closes.

## Finish line (ch10 arbiter, unchanged)

`GET /api/v1/resilience/elohim-host-landing/household` reports the SAME
non-zero `stewardingCollectives` on doorway-alpha.elohim.host AND
elohim.host — the felt-safety scenario (`resiliency-saga.steps.ts:517`).
Score movement: saga chapters green in the edge Dataplane Validation
verdict (jenkins-sync → saga-status), not just locally.

## Iteration loop

1. Push (16 commits, fast-forward). Pre-push runs the heavy dev gates
   including sweettest — first real exercise of the qahal batch-get
   (6a3507ae0). Let long gates finish; never orphan a running cargo.
2. Watch orchestrator dispatch → genesis + edge (+ app if triggered). ONE
   push per batch; wait for COMPLETE before any further push
   (concurrent-push mutual abort).
3. Genesis Seed Substrate: formation should now bind james (agencyPhase
   fix). Expect jessica founder + james affirm = 2/3 (matthew stays
   conflict — captured-UUID-chain, operator-scope, do NOT fight it).
4. Edge deploy ships: identity_fill timeout, witness counters, hostAliases
   manifests. Post-deploy (~20min churn window — don't measure inside it):
   - Loki: `Not updating agent info because we don't have a current url`
     gone on adam/eve/gertrude/susan = hairpin cure holding in-pipeline.
   - jessica: `identity_fill: run_once exceeded budget` WARN = the hang
     diagnosis confirmed; her loop completing = unwedged.
   - First non-zero `collectives_ids_discovered` anywhere = bootstrap
     circle broken.
   - B conductor-diagnostics agentCount > 0 (should already hold from the
     live patch; the manifests make it survive redeploys).
5. jenkins-sync → saga-status after each edge Dataplane Validation run;
   chase the reds that move, capture the ones that don't (timeline
   backlog, mintable-station shape).

## Decision rails (pre-decided so the night doesn't stall)

- **Head direction (ch06):** let the machinery converge first — the
  forward-ordering guard (newest declared_at wins) IS the conflict rule;
  with shem conductors back on the DHT, adopt-before-author + heal should
  converge toward B's newer declaration organically. If still wedged one
  full reconcile cadence past churn, use the carried-record declare lever
  TOWARD THE NEWEST declaration (invariant-consistent, reversible by a
  later deliberate operator Declare). Never declare toward the older head.
- **ch02 green path needs no matthew:** discovery ≥1 + created ≥1 on
  alpha-A via jessica+james memberships → collectives arm → identity_fill.
  If formation still lands 1/3, capture james's binding evidence and stop
  pushing on it.
- **divergentAnchor 638>100 gate (peer-mesh):** operator re-spec pending —
  re-measure after the fleet settles, report the number, do not touch the
  gate.
- **Floors:** no kubectl; no integrity-zome edits (DNA hash); alpha writes
  only through pipelines/sanctioned admin routes; path-limited commits;
  no bulk seeding; storybook is out of scope (infra-fixed at image layer).

## Bail conditions (stop iterating, write the handoff)

- Shem conductors still DHT-silent after a post-manifest deploy → the
  hairpin model is wrong or incomplete; capture evidence, stop.
- Pre-push gates red on something not caused by this sprint's commits →
  triage by origin/dev byte-identity (PVC-deferral memory), don't absorb
  someone else's red overnight.
- Any lever that would need a decision only the operator owns (new head
  re-declares beyond the rail above, matthew chain migration, suspending
  humans).

## Evidence trail from tonight (context for a fresh session)

`HANDOFF-2026-07-29-saga-sprint-gaps-closed.md` + backlog:
`shem-conductors-signal-hairpin-suspect-dht-silent.md` (CONFIRMED section),
`declare-sweep-hash-only-cannot-converge-missing-action.md`,
`jessica-identity-fill-loop-silent.md` (resolved). Saga entering the night:
6/10 green local (report jenkins-elohim-edge-dev-1253).
