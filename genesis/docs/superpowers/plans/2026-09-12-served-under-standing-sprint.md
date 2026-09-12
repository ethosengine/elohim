---
title: "Served under standing — failover finished on both planes, the doorway's commons standing measured, hostnames become heads"
id: served-under-standing-sprint
status: Draft
domain: D8
sprint: "2026-09-12 operator sprint — ratchet of five rungs, one named red each; drains doorway-failover, blob-durability, dataplane-convergence; births served-under-standing"
cites:
  - doorway/doorway-service/.epr-meta/served-under-standing.habit.md
  - doorway/doorway-service/.epr-meta/doorway-failover.habit.md
  - elohim/elohim-storage/.epr-meta/blob-durability.habit.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - app/elohim-app/.epr-meta/epr-atom-home.habit.md
  - genesis/a2o/features/dataplane/served-under-standing.feature
  - genesis/a2o/features/federation/name-routing.feature
  - genesis/a2o/features/resilience/chaos-peer-churn.feature
  - genesis/a2o/features/dataplane/doorway-apex-transition.feature
  - "doorway-federation-failover-sprint-plan | Doorway Federation & Failover Sprint | sha256:c66fd04c3b4f16e2 | path: genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md"
  - "doorway-federation-three-reds-to-green-plan | Doorway federation | sha256:d2b8f066817b690f | path: genesis/docs/superpowers/plans/2026-09-10-doorway-federation-three-reds-to-green-plan.md"
---

# Served under standing — the sprint

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Every rung names the red it moves and the run id or build number that moves it. Never push while a fleet roll is in flight; one push per batch; a habit atom delta is the deliverable.

**Goal:** finish failover on both planes on the household mesh and the fleet, give the doorway's commons standing a measured habit (born red 2026-09-12), and begin the topology transition in which a hostname is a head channel served by every doorway rather than a doorway's identity.

**Design source:** the four-pass operator ruling of 2026-09-12 under WS3 of the 2026-07-31 failover plan (parked branch `sprint/apex-multi-a-ingress`): doorways are the federated load balancer at the WAN addresses; a public name is a DHT fold of projection contracts × reach × requester standing × liveness; multiple projections are the CDN tier; a doorway's public projection is a commons standing the public can challenge. Hosted-human cost is anti-capture pressure, not a bottleneck; doorways are a bridge toward device stewardship.

**Ratchet:** five rungs, each disjoint in write set, each moving one named red. Rungs 1 and 2 run in parallel in isolated worktrees on the sprint cargo slots; rung 3 follows the habit; rung 4 follows a design-gate pass; rung 5 is the fleet and waits on the operator's ingress classes.

---

## Rung 1 — Storage: the three chaos reds (blob-durability · dataplane-convergence)

Evidence today (runs 20260912T145758Z, 20260912T152941Z): custody rows converge but their `state` does not travel on the wire; `humans.agent_pub_key` has no reconcile arm; `onlinePeers.live` reads a 900 s heartbeat so a killed peer stays live. Write set: `elohim/elohim-storage` only, branch `sprint/2026-09-12-served-under-standing-storage`.

- [ ] **Step 1: custody state on the wire** — `CommitmentWireFields` carries `state`; `project_commitment_from_wire` threads it; insert paths stop hardcoding `proposed` when the wire carries a state; the `rea_commitments` arm's divergence test compares state and re-heals already-wrong rows; cross-peer test. Evidence: gate EXIT lines + the chaos flapping drill's settled-record step green on the mesh.
- [ ] **Step 2: a `humans` reconcile arm** — divergent agent-key bindings converge to the DHT's current binding; InSync / KeyDivergent / Missing classified and counted; reuse `membership_identity_reconcile` as the write path. Evidence: cascade Given seeds 3 and the fold counts 3.
- [ ] **Step 3: liveness from the connected-peer view** — `onlinePeers.live` reflects the p2p connected set (ping timeout ≈35 s), `known` stays the heartbeat join; no wire change. Evidence: simultaneous-loss drill reads at-risk within the ping timeout.
- [ ] **Step 4: deploy and measure** — feature-full bin from the sprint slot pinned under the scratchpad, `just mesh storage-restart`, `just test mesh features/resilience/chaos-peer-churn.feature` and `features/resilience/doorway-footprint-convergence.feature`; deltas in both storage atoms with run ids.

## Rung 2 — Doorway: name routing (served-under-standing · doorway-failover)

Evidence today: `fetch_from_remote_doorway` is defined and never called; a doorway that does not host a name answers 404. Write set: `doorway/doorway-service` only, branch `sprint/2026-09-12-served-under-standing-doorway`.

- [ ] **Step 1: registry fold** — candidate holders for a root the local doorway cannot serve = DHT-registered doorways holding a live hosting contract for it, ordered health-first (status classification) then owner order; a named hook for attested RTT.
- [ ] **Step 2: one-hop forward** — relay to the first live holder with the loop-prevention header and a budget of one; try the next holder on a shed before answering the shed; preserve the original 404 when every holder fails; add the served-by origin header so the sticky client goes direct next time; wired as DeliveryRelay's final tier; dead code and the FEDERATION.md claim reconciled. Tests first.
- [ ] **Step 3: measure** — doorway bin from the sprint slot, restart the household doorways, glue for `features/federation/name-routing.feature` (drop `@wip` per scenario as glue lands), run under the lane lock; delta in served-under-standing with the run id.

**Routing dimensions (operator steering 2026-09-12 — the feature must cover the standard range; the fold/selector/relay shape makes each a field or a term, never a rewrite):**

| Dimension | Rung 2 (today) | Generalizes at |
|---|---|---|
| Path / mount prefix | yes — segment-boundary match, same rule as the local router | — |
| Host (virtual hosts; hostnames as head channels) | fold key is `RouteKey { host: Option, path }`, host unconstrained | rung 4 — contract gains host + channel |
| Health-based failover | yes — serving / shedding / uncertain / unreachable; next holder on a shed | — |
| Reach and requester standing | enforced at the holder, not in selection | rung 3 — eligibility fold |
| Nearest (attested RTT, region) | named hook; owner order today | selector term when the advertisement lands |
| Weighted / canary | no | the candidate channel is the canary; weight is a selector term if wanted |
| Relay mode | `Proxy`, one hop, served-by hint; `Redirect` declared, unimplemented | small addition |
| Sticky | client-side via the hint and the shipped fallback client | stays client-side |
| Rewrites | no — contracts declare mounts | not planned |

**Adaptation model (operator steering 2026-09-12 — mature frameworks, SSR runtimes such as PHP/LAMP and Node, nginx/Apache, ingresses and proxies must be able to attach at their foreseeable depth):** the hosting contract is the routing intermediate representation. Its vocabulary is a deliberate subset of the Kubernetes Gateway API HTTPRoute shape (hostnames; path, header, method and query matches; redirect, rewrite and header filters; weighted backends) plus the terms only this protocol has (reach, requester standing, channel, mode: proxy | redirect | render | static). Attachment by seam, per the seam map: frameworks and SSR runtimes attach at the SDK seam (the app package manifest declares mounts and render mode; the packaging adapter is where a non-Angular bundle describes itself; the execution engine is a mod under the render registry, as V8 rendering is today); foreign routing declarations (nginx/Apache config, Ingress, Caddy, Traefik) attach at the bridge seam as crates that translate them INTO contracts; foreign execution engines attach outward — contracts project to an nginx config or a Gateway controller, so the doorway is one controller of the contracts, not the only one. Guard: the doorway core never learns a framework — a change that names PHP, Next, nginx or Ingress inside the router or the fold is in the wrong seam; the contract grows a field, a bridge or adapter grows the knowledge. Rung 4's design gate decides the contract vocabulary against this shape.

## Rung 3 — The standing story (served-under-standing · epr-atom-home)

Evidence today: the habit is born red with two READY stories and no glue; the chrome's five affordances are named as epr-atom-home's commons checks.

- [ ] **Step 1: reach at serve time** — the doorway re-asks reach against the current EPR head on every serve of a warm shell or cached bundle (bytes stay warm; permission is never cached); a collective's reach ruling propagates on the next reconcile. Scenarios 1–3 of `served-under-standing.feature` get glue; first honest reds recorded.
- [ ] **Step 2: the fair-trade receipt and the owed response** — the chrome names what was exchanged for this serve and who was credited (REA), and a visitor's challenge becomes a witnessed commitment with a due window. Scenarios 4–5; epr-atom-home's commons checks measured.
- [ ] **Step 3: the transition count** — a humans-served sibling counts hosted now, stewarding now, crossed this period, so the bridge has a number the chrome can show.

## Rung 4 — Hostnames become heads (design gate first)

Target: `elohim.host` = the converged (elected) head at commons reach, served by both doorways; `alpha.elohim.host` = the candidate head (a CID on the staging channel) at stewards-or-collective reach, served by both doorways; promotion is the collective's election.

- [ ] **Step 1: design gate** — p2p-design-gate on the hosting contract growing `channel` and `reach` per host (it is a notarized commitment; entry type, head-plane cost, identity, coordinator, signal answered before any route), and on keying the route registry by host.
- [ ] **Step 2: contracts and registry** — seed writes host + channel + reach; the doorway keys its route registry by host and picks the head by channel (reuse the bundle-heads reconciler and the canary's long-lived candidate channel).
- [ ] **Step 3: the candidate host is reach-gated** — anonymous at alpha refused with the chrome's reason; a steward served; the A/B test as head. Measured by served-under-standing's scenarios against a real second channel.
- [ ] **Step 4: rename cascade** — the a2o vocabulary's "alpha" moves from doorway identity to channel name (env names, fixtures, LAYERS) in one deliberate pass; `prod.yaml` retired; deployments.json loses alpha-versus-apex as doorway identity.

## Rung 5 — Fleet: the apex (doorway-failover flips)

- [ ] **Step 1 (operator):** create the two premise-pinned ingress classes (`public-ethosengine`, `public-shem` proposed) with controllers bound to their nodes.
- [ ] **Step 2:** land `sprint/apex-multi-a-ingress` on dev (class-keyed conflict checker, both doorway ingresses accepting the apex, both beacon legs contributing it); one push, the roll it triggers is the measurement.
- [ ] **Step 3:** apex-transition measured on the fleet (Task 21 of the 2026-09-10 plan); `doorway-failover` red → green with the build number, by `epr flow note --kind ruling`.

---

**Done means:** every rung's story green, or red for a named mesh reason written into its atom; deltas in every touched atom; the register re-projected; one push per batch with the fleet confirming what the household proved.
