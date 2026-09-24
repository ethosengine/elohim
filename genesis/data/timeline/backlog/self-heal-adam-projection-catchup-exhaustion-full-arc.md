---
id: "backlog-self-heal-adam-projection-catchup-exhaustion"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam (B / elohim.host) projection catch-up stalls after a deploy restart — cells are NOT authorities until their storage arc reconverges, so every heal get_links leaves the box and dies on the 60s conductor request timeout"
slug: "self-heal-adam-projection-catchup-exhaustion-full-arc"
written: "2026-07-27"
updated: "2026-09-24"
author: "claude (resiliency-saga sprint-3 delivery — ch06 runtime blocker RCA); mechanism corrected 2026-07-29 (rust-architect, probe-confirmed); ledger-bound 2026-09-12 (runtime-triage); mechanism corrected AGAIN 2026-09-13 (runtime-triage, Prometheus-confirmed — admission ceiling, not arc convergence); re-triaged 2026-09-17 (runtime-triage — node condition unchanged and flat; third flap traced to CLOSE_STREAK == predicate window, fixed); fourth filing 2026-09-24 (runtime-triage — the chronic ceiling CLEARED and closed honestly; re-filed by a fresh post-deploy process that has not yet converged)"
status: "wip"
priority: "high"
self_heal_status: blocked
severity: high
ci_status: blocked
jobs: [elohim-edge]
fingerprints: [79f357281ca5]
nodes: [alpha-b, elohim-adam-alpha]
tags: [self-heal-exhaustion, projection-reconcile, catch-up, storage-arc, arc-convergence, kitsune2-gossip, get-strategy-local, adam, shem, restart-churn, heal-timeout, ch06, declare, chronic-flap, elevate-arm, conductor-admission, admission-shed, sensing-gap, multi-process-counter, closure-hysteresis, re-dispatch-amplifier, post-deploy-catch-up, genuine-closure]
cites:
  - resiliency-saga-sprint3-objective | Resiliency Saga Sprint 3 Objective | path: genesis/docs/superpowers/plans/2026-07-26-resiliency-saga-sprint3-objective.md
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - genesis/orchestrator/manifests/humans/adam-firstman.yaml
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - https://elohim.host/admin/self-healing
  - https://elohim.host/p2p/status
  - .claude/scripts/_lib/runtime_harvest.py
  - genesis/data/timeline/backlog/self-heal-render-degenerate-cumulative-counter-false-positive.md
  - genesis/data/timeline/backlog/fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11.md
  - elohim/elohim-storage/src/conductor_admission.rs
  - elohim/elohim-storage/src/metrics.rs
  - .claude/scripts/_lib/__tests__/runtime_harvest_test.py
  - .claude/data/runtime-findings.jsonl
  - .claude/data/runtime-cursor.json
  - genesis/data/timeline/backlog/dataplane-reanchor-dead-remaining-rekeyed-peer.md
  - genesis/data/timeline/backlog/runtime-sensing-gap-poller-unscheduled-no-throttle-alert-2026-09-11.md
---

# adam's post-restart catch-up cannot complete — corrected mechanism

> **2026-07-29 — SUPERSEDES the original diagnosis and its prescribed cure.**
> The 2026-07-27 record attributed this to a full-arc working set (RAM/latency ∝
> corpus) and prescribed **`target_arc_factor < 1` for adam**. That reading is
> WRONG on mechanism, and acting on it would have deepened the outage — see
> "The prescription that would have made it worse" below. The symptom record from
> 2026-07-27 is preserved verbatim in the next section because it is accurate;
> only the causal explanation and the cure change.

## The symptoms (2026-07-27, live, blocking ch06 delivery — unchanged)

After the sprint-3 coordinator hot-swap restarted the alpha conductors (edge
#1243 deploy, ~23:40 UTC), **adam** (backing doorway B / elohim.host) entered a
projection catch-up it had not completed 80+ minutes later — 4× the ~20-min
restart-churn the substrate trust contract expects.

- `GET https://elohim.host/db/content/*` → `503 {"status":"catching-up"}`; the
  doorway `/health` otherwise green (conductor connected, 7/7 pools healthy,
  uptime advancing — not crash-looping).
- adam's `projection-reconcile` logs each sweep: `heal complete … caught_up:
  false, content_healed: 0, content_local_anchored: 4188,
  content_divergent_anchor: 3599, content_ids_discovered: 8717` — thousands of
  gaps, **zero healed**.
- Repeated every sweep: `projection-reconcile[content]: conductor resolve
  failed; retry next sweep — Request timeout: heal conductor call exceeded
  per-attempt timeout 15s (transient)`. Also on the REA leg.
- Conductor-internal, every ~15-30s, steady-state 2h+ past boot:
  `get_links.rs:76 Host("Other: get_links response channel dropped: likely
  response timeout")` from `content_store::resolve_content_head`.
- NOT resource starvation: 1.9GiB of an 8GiB limit, 1.2–3.5 of 8 cores, light
  throttling, no OOMKills. The conductor was busy-but-alive — *waiting*, not
  computing.

## The actual mechanism (2026-07-29, probe-confirmed)

**adam is not slow because its arc is full. It is slow because its arc is NOT
full — so every `get_links` leaves the box and dies on a 60s network timeout.**

The chain, each link verified in source:

1. **Every cell's storage arc resets to `Empty` on every conductor start.**
   `kitsune2_core-0.3.2/src/factories/core_space.rs:419` — `local_agent_join`
   calls `set_cur_storage_arc(DhtArc::Empty)`, regardless of `target_arc_factor`.
2. **The arc only becomes FULL after a gossip round returns zero mismatched
   sectors.** `kitsune2_gossip-0.3.2/src/storage_arc.rs:99-105` (the
   `not(feature = "sharding")` arm — sharding is off). It logs
   `tracing::info!("Updating storage arc to full")` at `:102`.
3. **The authority check reads the CURRENT arc, not the target.**
   `holochain_p2p/src/spawn/actor.rs` `authority_for_hash` tests
   `agent.get_cur_storage_arc().contains(loc)`. Empty arc ⇒ `false`.
4. **So the cascade takes the network branch.** `holochain_cascade/src/lib.rs:788-791`
   — `if let GetStrategy::Network = strategy { if !authority { fetch_links(..) } }`.
   This is the ONLY path that emits the observed error.
5. **`GetStrategy::default()` is `Network`** (`holochain_zome_types/src/entry.rs:94-105`),
   used at all 211 `get`/`get_links` sites in `content_store`; `GetStrategy::Local`
   appeared **zero times in any DNA** before this fix.
6. **Each network `get_links` fans out hard**: `PARALLEL_GET_AGENTS_COUNT = 3`
   (`actor.rs:23`) × `.buffered(10)` (`host_fn/get_links.rs:67`) = up to 30
   in-flight requests per zome call.
7. **Each dies at `request_timeout_s` = 60s**, and the timer starts *before* the
   send (`actor.rs:1155-1172`), which can itself burn 45–60s in tx5 WebRTC
   connect. Channel drop ⇒ `actor.rs:1192`.
8. **Storage's 15s deadline could never win, and abandoning did not shed load.**
   `HcClient::call_zome` has no cancellation, so each of the 3 attempts kept
   running in the conductor — 3 concurrent zome calls per row, ~30 network
   requests each, for zero progress.
9. **That traffic starved the gossip that would end it.**
   `kitsune2_gossip-0.3.2/src/initiate.rs:66` — while any local agent is below
   its target arc, the space waits on `fetch.notify_on_drained()` or a 120s
   timeout (`:80-103`), and there is only **one initiated round per space at a
   time** (`:104-107`).

**Positive-feedback deadlock:** the heal loop's own traffic prevented the arc
convergence that would have made the heal loop's calls local.

### The probe that confirmed it

`Updating storage arc to full` in adam's conductor log. Since adam's 10:57Z boot
on 2026-07-29: **exactly ONE such line (11:41:58Z, agent `uhCAk_hiBZ…`) across
~28 hosted agents.** Arc convergence — not corpus size — is this node's
bottleneck. Household nodes are fine because 1–2 agents over a small corpus
complete a round quickly, after which everything resolves locally.

### The prescription that would have made it worse

The original record recommended `target_arc_factor < 1` for adam. A lower arc
factor makes the node authority for **less**, which sends **more** reads to the
network; at `0` it is a leecher, authority for nothing, and *every* read becomes
a 60s round-trip permanently. [[project_per_node_memory_is_conductor_authority_arc]]
is correct that arc factor is the **memory** scale lever — but for **latency** it
points the opposite way. Do not reach for it here.

## The cure (implemented 2026-07-29, awaiting push + deploy verification)

Four bounded changes, none requiring a DNA reinstall or a re-key:

1. **Cure 3 — stop amplifying** (`elohim-storage/src/p2p/projection_reconcile.rs`).
   Retry only *answered* transient errors, never our own synthetic per-attempt
   timeout (`should_retry_attempt` / `is_synthetic_attempt_timeout`), plus a
   per-leg `HealCircuit` that sheds the remainder of a leg after 3 consecutive
   synthetic timeouts and closes on the first success.
2. **Cure 1 — a SEPARATE local read path** (`content_store` **coordinator** zome).
   `GetStrategy` is threaded through `gather_canonical_head_record` /
   `gather_content_chain` / `resolve_root_author`, and the head election is
   shared by two externs: `resolve_content_head` (**`Network`**, unchanged
   semantics) and a new `resolve_content_head_local` (**`Local`**). Only the
   storage heal loop calls the local variant. The DECLARE paths keep `Network` (a
   `Local` author gate would reject legitimate declares with "not in the version
   chain"). Turns a 60s heal hang into a sub-millisecond `None` and stops feeding
   the fetch queue that blocks arc convergence. **Coordinator-only: the DNA hash
   does not move**; ships via `update_coordinators` under `ALLOW_COORDINATOR_UPDATE`.

   > **Review catch (2026-07-29):** the first cut switched the single shared
   > `resolve_content_head` extern to `Local`. That extern also backs the HTTP
   > author gate (`POST /db/content/{id}/head`, `http.rs`), which turns `None`
   > into `404 "content has no version chain on this notary"` — so on a cold-arc
   > node it would have fast-404'd legitimate authors by reading a Local `None`
   > as authoritative absence. Splitting the externs is what makes the local read
   > safe. **Invariant to preserve: a Local `None` is "not in my view YET", never
   > proof of absence — never use it to gate authorship, deny a declare, or 404.**
3. **Cure 2 — timed-out rows reach the peer-adoption arm.** A transient failure
   with a `PeerHeadHint` now routes to `adopt_candidates`
   (`timeout_should_route_to_adopt`) instead of falling out of both candidate
   lists and being silently re-dropped every sweep. Adoption goes through the
   existing verified path — `PeerHeadRecordFetcher` over view-federation, then
   declare with `carried_record`, which `validate_carried_record` checks for
   action-hash binding, author signature, and entry↔action binding. Evidence, not
   authority: the DHT stays the manifest and **no stamp mode changed**.

   > **Known contract deviation (documented at both ends, 2026-07-29):** these
   > candidates reach `try_adopt_canonical_head` as `LocalResolve::Known(None)`,
   > whose doc means *observed* absence. A timed-out row is **unknown**, not
   > observed-absent. It is conservative-safe only because `None` merely
   > forecloses the `AdoptLocal` arm, and the timeout route is gated on a peer
   > hint — so the reachable verdicts are `AdoptPeer` / `Hold`, neither of which
   > asserts absence. **A future arm that needs "the conductor observed nothing"
   > must split the variant** (`Known(None)` vs `Unresolved`) rather than let a
   > timeout read as an observation. Noted on `LocalResolve::Known` and on
   > `adopt_deferred_heads`.
4. **Cure 4 — adam's conductor config only** (`adam-firstman.yaml`). `k2Gossip`
   reverted to upstream defaults (`roundTimeoutMs` 60000→15000,
   `maxConcurrentAcceptedRounds` 4→10) — the convergence-relevant part. The
   household slow-WAN profile those values came from (2026-07-20) is still
   correct for households and stays in `_edgenode-consolidated.template.yaml` —
   **do not propagate this revert there.**

   > **Considered and DROPPED (2026-07-29): lowering `request_timeout_s` to 10.**
   > Proposed so the conductor's give-up would land under storage's 15s heal
   > deadline. Rejected on review: the key is **conductor-wide**, and tx5
   > first-contact alone can burn 45–60s in WebRTC connect — a 10s cap would
   > fast-fail the declare paths deliberately kept on `Network`, plus every other
   > DNA's network gets across all of adam's hosted agents. With the heal read
   > path now Local, the cap has no remaining purpose. Do not re-propose without
   > a **per-call** timeout mechanism instead of a conductor-wide one.

## The real ceiling (operator decision — replaces the old one)

**Not `target_arc_factor`. Cap or shard doorway-B's agent provisioning onto adam.**

Kitsune2 budgets gossip **per space**, not per agent: one outbound initiate round
at a time (`initiate.rs:104-107`), and a single local agent still below its
target arc holds the whole space in the slow initiate path (`initiate.rs:66`).
Meanwhile `doorway/doorway-service/src/conductor/pool_map.rs:44`
(`DEFAULT_MAX_AGENTS_PER_CONDUCTOR = 50`, no env lever) keeps adding agents to
this one conductor. Arc-convergence cost therefore scales with hosted agents
while the convergence budget does not. That ceiling is structural and cannot be
tuned away.

Also still true: adam re-opens this window on **every deploy restart** (step 1
above), so the trust contract's ~20min restart-churn does not hold for this node
until the hosted-agent count comes down.

## Post-cure measurement (2026-07-30, Loki 05:00Z→19:30Z — the escalation evidence bundle)

The cures are deployed and behaving as designed, and the ceiling still holds:

- **Cure code live on adam** (build `5290e3f`, running since 17:24Z): the content-leg
  `HealCircuit` OPENED/shed 28 times across the window (first 11 min after a boot, most
  recently 19:16Z) — the storm is *contained*, not gone; `resolve_content_head_local`
  observed as a called extern. The `get_links response channel dropped` storm: 4 lines
  total, all 06:55–07:33Z, **zero** after the 07:33Z redeploy.
- **Arc convergence stalled at the population level**: exactly ONE `Updating storage arc
  to full` agent (`uhCAk_hiBZ…`, the same one every boot) in 14.5h across ~35 hosted
  agents; `caught_up=false` on every sweep all day; `content_local_anchored` 4226→4231
  (+5) over the window; `content_divergent_anchor` oscillates 691–2919 (windowed-scan
  noise, no trend).
- **Restart churn compounds it**: four process boots in one day (05:04, 05:12, 07:33,
  17:24Z — rolling redeploys), each resetting every cell's arc to Empty per
  `core_space.rs:419`.

Verdict: with the amplification cured, what remains is exactly the structural ceiling
above — per-space gossip budget × hosted-agent count. No further storage/DNA-side code
change moves this; the next state change is the operator's provisioning decision
(cap/shard doorway-B's agents onto adam), or a long deploy-quiet stretch getting lucky.
saga ch04 (elohim.host `GET /` 200) and ch06 (declared-head equality) both wait behind
B's `caught_up`, plus ch06's separate head-direction decision.

## What would land ch06

adam serving 200 (`caught_up=true`) → re-run the App pipeline's `authorHeadOnce`
(a `[build:app]` push) so the declare-carries-Record cross-declare lands A's head
on B against a responsive conductor. The mechanism is proven; it needs a declare
cycle against a non-503 B.

---

# 2026-09-12 — the elevate arm now files this concern as a fingerprint (`79f357281ca5`)

## What is exhausted

Nothing new. This is the SAME ceiling, six weeks later, arriving through the runtime
poller instead of a CI chapter — and at a magnitude two orders smaller than the
2026-07-27 incident. Ledger line (fp `79f357281ca5`, node **alpha-b**, filed poll 82,
`2026-09-12T12:12:45+00:00`):

```
projector caughtUp=false sustained >= 3 polls
```

`_projector_lag` (`.claude/scripts/_lib/runtime_harvest.py`) fires on
`projector.caughtUp is False` across `LAG_POLLS = 3` consecutive samples. The stored
cursor window shows it false on the last four samples of the ring buffer. The report is
HONEST — the node really did say "I am not caught up" four polls running.

## Re-fetch at triage — SELF-RESOLVED, and the interesting part is what did NOT change

`GET https://elohim.host/admin/self-healing` (12:22:38Z, HTTP 200):

```json
"projector": {"lagSeconds": null, "caughtUp": true, "divergentAnchor": 1}
```

`caughtUp` back to **true** and `divergentAnchor` **9 → 1**. But `GET /p2p/status` the
same minute says what actually happened:

```json
"projectionReconcile": {"pending": 0, "completed": 0, "failed": 0, "caughtUp": true,
                        "peersAsked": 0, "divergentAnchor": 1, "healedTotal": 0,
                        "sweeps": 209, "exhausted": 0, "converged": false}
```

**209 sweeps, `healedTotal: 0`, `completed: 0`, `converged: false`.** The projector did
not catch up by healing anything — it healed nothing, in two hundred and nine sweeps.
`caughtUp` went true because the windowed divergence scan happened to come back small,
which is precisely the "`content_divergent_anchor` oscillates … windowed-scan noise, no
trend" behaviour the 2026-07-30 post-cure measurement above already recorded. Same
signature, four-digit divergence then, single-digit now.

So this fingerprint is the ceiling's **flap**, not a new stall: `caughtUp` oscillates
false↔true with the scan window while `healedTotal` stays pinned at zero. The chapter-
blocking 2026-07-27 shape (`content_divergent_anchor: 3599`, 80+ minutes continuously
false) and today's shape (`divergentAnchor: 9`, four polls) are the same mechanism at
different amplitudes.

## Current decision

**BLOCKED — unchanged blocker, now ledger-bound.**

The blocker is exactly the one "The real ceiling (operator decision)" names above:
kitsune2 budgets gossip **per space**, not per agent (`initiate.rs:104-107`), while
`DEFAULT_MAX_AGENTS_PER_CONDUCTOR = 50`
(`doorway/doorway-service/src/conductor/pool_map.rs:44`) keeps adding hosted agents to
adam's one conductor. Arc-convergence cost scales with hosted agents; the convergence
budget does not. **No storage-side or DNA-side code change moves this** — the 2026-07-29
cures are deployed and measured working. The next state change is an operator
provisioning decision (cap or shard doorway-B's agents onto adam), which is a cluster
action and therefore out of a background triage agent's hands by rule.

Ledger entry `79f357281ca5` is marked `blocked` so the poller stops dispatching on the
present fingerprint. The condition having already self-resolved, the poller will delete
the line by disappearance within `CLOSE_STREAK` polls; the next flap re-files as NEW.

## The re-dispatch hazard this creates, and the sensing follow-on NOT taken

Because `caughtUp` flaps, this fingerprint will close by disappearance and re-file as
NEW on every future oscillation, dispatching a fresh triage agent each time for a
condition whose verdict is already written here. That cost is real and should be named
rather than absorbed.

The cheap mitigation exists and is deliberately **left to the deterministic-layer
owner**: `_projector_lag` reads `caughtUp` alone, while the discriminators between a
flap and a stall are **already sampled on every poll** and simply not consulted —
`projectionReconcile.sweeps`, `healedTotal`, and `divergentAnchor` on `/p2p/status`. A
predicate that required, say, a divergence magnitude floor or sweep-count-without-
progress would have been silent today and loud on 2026-07-27.

Two reasons this triage did not just implement it:

1. **A magnitude floor can blind the poller to a genuine small-corpus stall.** A
   household peer with nine divergent anchors and no ability to heal them is exhausted;
   today's alpha-b with nine and a working scan is not. `divergentAnchor` alone does not
   separate those, and guessing a threshold is how the sibling `_render_degenerate`
   predicate acquired two false positives (see the cited record).
2. **The standing lesson in this file's sibling applies**: a predicate that has misfired
   gets a *state field the runtime publishes*, not a third hand-tuned threshold. The
   runtime-side version of that is a projector self-report that distinguishes
   "sweeping, making progress", "sweeping, healing nothing", and "cannot sweep" —
   which is a `projection_reconcile.rs` change with a fleet roll behind it, not a poller
   tweak.

Written as a specified follow-on, owner-assigned, not silently taken.

## Verification

- 2026-09-12 12:22:38Z — `/admin/self-healing` and `/p2p/status` re-fetched on
  elohim.host (HTTP 200, both quoted above).
- Condition confirmed self-resolved at triage: `caughtUp: true`, `divergentAnchor` 9→1.
- Confirmed NOT healed: `healedTotal: 0` over 209 sweeps, `converged: false`.
- Regression signature to watch: `healedTotal` STILL 0 with `divergentAnchor` climbing
  into the hundreds or thousands, or `caughtUp` false for a wall-clock hour — that is
  the 2026-07-27 amplitude returning, and it blocks ch04/ch06 again.

---

# 2026-09-13 — `79f357281ca5` re-fired, and the mechanism has moved: this is the CONDUCTOR ADMISSION ceiling, not arc convergence

> **SUPERSEDES the 2026-07-29 mechanism as the CURRENT cause.** That record's chain
> (cold arc ⇒ network `get_links` ⇒ 60s timeout) is preserved because it was right in July
> and its four cures are still deployed and still load-bearing. What is measured TODAY on
> adam is a different link: the heal calls do not die in the network, they never leave the
> box — the storage→conductor admission gate sheds them. The July prescription is not what
> moves this; see "The lever, and why it is not the admission cap" below.

## What is exhausted

The fingerprint closed by disappearance overnight and re-filed as NEW at poll 86
(`2026-09-13T12:32:57+00:00`) — exactly the re-dispatch hazard the 2026-09-12 section
predicted, costing a second Opus triage on an unchanged condition. Ledger line:

```
projector caughtUp=false sustained >= 3 polls
```

**This time it did NOT self-resolve at triage.** `GET https://elohim.host/admin/self-healing`
(12:42:18Z, HTTP 200):

```json
"projector": {"lagSeconds": null, "caughtUp": false, "divergentAnchor": 59},
"admission": {"maxInflight": 256, "available": 256, "shedTotal": 0},
"upstreams": [{"endpoint": "http://elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090",
               "circuit": "closed", "errorStreak": 0, "recentFailures": 0}]
```

`GET https://elohim.host/p2p/status`, same minute:

```json
"projectionReconcile": {"pending": 11, "completed": 0, "failed": 2, "caughtUp": false,
                        "peersAsked": 6, "divergentAnchor": 59, "healedTotal": 0,
                        "sweeps": 23, "exhausted": 0, "converged": false}
```

**Zero healed in 23 sweeps**, six peers asked, 59 divergent anchors. Doorway-side admission
is wide open (256 of 256 free, `shedTotal: 0`) and the upstream breaker is CLOSED — so
nothing on the surfaces the poller reads explains it. The explanation is one layer down.

## The mechanism, corrected (Prometheus, 2026-09-13, read-only)

The July record says adam's problem is that its **arc is not full**, so heal reads leave the
box. Two numbers falsify that as the current cause:

| gauge (instant, all 7 alpha peers) | adam | matthew | fleet range |
|---|---|---|---|
| `elohim_projection_reconcile_divergent_actionable` | **11** | 28 | 11–53 (**adam is the LOWEST**) |
| `elohim_projection_reconcile_gaps` | 12 | 9 | 9–91 (adam near the bottom) |
| `elohim_projection_reconcile_exhausted` | **87** | 53 | 27–76 (**adam is the HIGHEST**) |

adam has the *least* actionable divergence in the fleet and the *most* abandoned rows. It is
not failing to find what to heal. It is failing to heal what it has already found.

**Where the heal calls die** — `sum by (pod, outcome) (increase(elohim_projection_heal_outcomes_total{namespace="elohim-alpha"}[24h]))`:

| outcome | adam | matthew |
|---|---|---|
| `call_failed` | 286.25 | 91.12 |
| `timeout_exhausted` | 277.29 | 246.33 |
| `healed` | **0** | 35.05 |
| `refreshed` | **0** | 144.21 |
| `missing` | **0** | 108.20 |
| `refused_declared` / `refused_stale` / `no_row` / `unattempted` / `deferred_to_adopt` / `failed` | **0** | (nonzero) |

Every hourly bucket, all 25 of them, reads the same: adam's heal outcomes are **100 %
`call_failed` + `timeout_exhausted`, 0 % everything else, continuously for 24h** — not a
recent onset. The taxonomy is the tell. An arc-coverage failure produces `missing` /
`refused_declared` (the conductor answered "not in my view"); those are *entirely absent* on
adam while matthew, on the same DNA and the same DHT, produces 108 of them. adam's calls are
not being answered wrongly. They are not being admitted.

**The gate that is shedding them** — `sum by (pod, class) (increase(elohim_conductor_admission_shed_total{namespace="elohim-alpha"}[24h]))`:

| | adam | matthew |
|---|---|---|
| `class="interactive"` | **668.14** | 2.006 |
| `class="background"` | **136.20** | 2.004 |
| `avg_over_time(elohim_conductor_admission_in_flight[6h])` | **4.28 of 5 (86 %)** | 1.37 of 5 (27 %) |

adam sheds ~330× matthew and runs at 86 % of its admission capacity sustained over six hours.
Hourly interactive shed over 24h: `71.6, 82.7, 82.7, 25.2, 22.2, 23.2, 4.0, 9.1, 12.1, 18.2,
39.3, 81.7, 63.5, 117.0, 8.1, 21.3, 0, 0, 0, 9.1, 0, 0, 12.1, 32.2, 8.1` — it eased for a
three-hour stretch overnight and resumed. `elohim_projection_reconcile_exhausted{stream="rea"}`
on adam's current instance climbed `2 → 32 → 87` over ~2h while `divergent_actionable` fell
`51 → 11` in the same window: rows are transitioning from divergent into **abandoned**, not
into healed.

This is the same reading the overnight shift took from the other end (the quiesce gate and
the app-head PATCH both answered `{"status":"catching-up","cause":"upstream"}`), now bound to
this fingerprint.

## Root-cause inventory

- `elohim/elohim-storage/src/conductor_admission.rs` — the gate. Its module doc,
  §"Sizing — borrowed from the conductor, not guessed", derives capacity as
  **`db_max_readers − 3`**, where `db_max_readers = max(2*cpus, 8)`; every alpha peer reads
  `elohim_conductor_admission_capacity = 5`. `:381` `inc_admission_shed(class, zome)`;
  `:482`/`:492` publish the capacity; `:414` `is_admission_shed` is the backpressure contract.
- `elohim/elohim-storage/src/p2p/projection_reconcile.rs:1374-1425` —
  `ProjectionReconcileStatus`: `caught_up` is documented "true when this SWEEP ended … It is
  NOT a convergence signal" and "that is why `caught_up` alone overstates"; `converged` is the
  field "an SLO may be offered over"; `divergent_anchor` is deliberately the TOTAL (adjudicated
  rows included).
- `elohim/elohim-storage/src/p2p/projection_reconcile.rs:270-302` — `MissLedger`: the wire
  `exhausted` is a PER-SWEEP count (reads 0 on adam) while
  `elohim_projection_reconcile_exhausted` is the cross-sweep gauge (reads 87). Not a
  contradiction; two different denominators, and only the gauge shows the abandonment.
- `elohim/elohim-storage/src/metrics.rs:4551-4562` — `converged_blockers`: `pending`, `failed`,
  `divergent_actionable`, `unmeasured`. The wire field and the gauge are written from one place.
- `genesis/data/timeline/backlog/fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11.md`
  — **owns the cure design**. Its 2026-09-11 measurement already recorded "admission gate cap 5
  (adam 3/5 in flight, shed 54 in 6 h)". Two days later adam is at 4.28/5 and shedding 61
  interactive in 6h: the saturation deepened, the diagnosis was already written.

## The lever, and why it is not the admission cap

The obvious move — raise the admission capacity so adam's heal calls get through — is the
2026-07-29 "prescription that would have made it worse" in a new costume. The cap is not a
guess to be tuned: it is *derived* from the conductor's own `db_max_readers`, reserving three
permits so the pool's keeper is never starved by its callers. Raising it does not create
conductor capacity; it removes the only thing currently protecting a conductor that the
fleet record measures at **100 % CFS throttle for ~24h**. The shed is the system working.

The real levers are all outside a background triage agent's hands, and all already written
down in the fleet record: flip a full-arc shem conductor to leecher (cluster action), shard or
cap doorway-B's hosted-agent count onto adam, raise adam's conductor CPU, or land the
demand-driven warm-up / reconcile ramp that record designs (a `k2:` boot profile plus a
concurrency ramp keyed on `elohim_conductor_admission_*`). The last of those is an
**actuation** loop, which this agent's remit explicitly excludes.

## Current decision

**BLOCKED — same verdict, corrected blocker, and the fix that WAS bounded has landed.**

1. The node condition is blocked on an operator/cluster lever (above). The cure design is
   owned by the fleet record; nothing storage-side or DNA-side moves it, and the 2026-07-29
   cures remain deployed and correct for the failure mode they addressed.
2. The one storage-side surface that could publish a better projector self-report —
   `elohim/elohim-storage/src/p2p/projection_reconcile.rs` — is under active operator WIP in
   the working tree and is out of this pass's write-set by rule. Named, not touched.
3. **The sensing defect WAS bounded, and is fixed** (next section). That is what stops the
   flap that re-dispatched this triage.

## The predicate that kept re-dispatching this, fixed

The 2026-09-12 section reserved this to "the deterministic-layer owner" on the grounds that
the honest cure needed "a projector self-report that distinguishes 'sweeping, making
progress', 'sweeping, healing nothing', and 'cannot sweep' — a `projection_reconcile.rs`
change with a fleet roll behind it". **That report already ships.** `healedTotal`, `sweeps`,
`divergentAnchor` and `converged` are on `/p2p/status` today, sampled into the cursor on every
poll, and simply were not consulted. No fleet roll, no new threshold class, no Rust change.

The proof that `caughtUp` cannot carry this predicate is two nodes in the same fleet in the
same minute, both reporting `caughtUp: false`:

| 12:42Z | `caughtUp` | `healedTotal` | `sweeps` | verdict |
|---|---|---|---|---|
| alpha (matthew/A, `doorway-alpha`) | false | 57 → **73** | 29 → 32 | healing fine — a sweep merely in flight |
| alpha-b (adam/B, `elohim.host`) | false | **0** | 21 → 23 | the ceiling |

`_projector_lag` (`.claude/scripts/_lib/runtime_harvest.py`) now fires on the projector's own
report — `healedTotal == 0` **and** `sweeps >= MIN_SWEEPS` **and** `divergentAnchor > 0`
**and** `converged is False`, sustained across `LAG_POLLS` — falling back to the legacy
`caughtUp` arm only for a node that does not publish `projectionReconcile`. Replayed against
the stored cursor windows: **alpha SILENT, alpha-b fires with fingerprint `79f357281ca5`
unchanged** (provenance is the fingerprint input, and it did not move — so the existing
blocked ledger line keeps suppressing dispatch, and now stays present instead of flapping).

**A trap found while building it, worth more than the fix.** The first cut required
`healedTotal` unchanged and `sweeps` strictly ADVANCING across the window — a cross-poll
delta. The stored alpha-b window reads `sweeps` **59, 79, 207, 207, 87, 88, 109, 21**:
non-monotonic, because consecutive polls of these endpoints are answered by DIFFERENT storage
processes (the doorway's upstream pool plus pod churn — corroborated from the other side by
five distinct Prometheus `instance` IPs for `elohim-adam-alpha-0` inside 24h, in disjoint
time bands, with `kube_pod_container_status_restarts_total` flat at 0). Subtracting counters
across that series is the `_render_degenerate` cumulative-counter defect one layer out, and it
would have gone SILENT on the very condition it was written for. Every term is now read
within a single sample. **Standing lesson for this poller: a counter on these endpoints
belongs to a process, not to a node — never difference one across polls.**

A second, unrelated leak was found and fixed in the same file: the shell-mode test
`shell degrades quietly on unreachable node` ran without a `CLAUDE_PROJECT_DIR` override, so
running the suite polled a fake node against the **repo's own live ledger** — advancing
`poll_index`, adding a `clean_poll_streak` to every real finding (closure-by-disappearance is
3, so two suite runs can DELETE a live triage line) and filing a spurious `harvester-blind`
row that would dispatch a triage agent at the next SessionStart. It now writes to a tmpdir
like the write-side tests beside it, with an assertion that proves it.

## The sensing gap this leaves NAMED (not built)

The poller's `_admission_shed` predicate reads the **doorway's** inbound gate — `maxInflight:
256, available: 256, shedTotal: 0` on elohim.host, healthy all day. The gate actually shedding
is `elohim_conductor_admission_shed_total` on the **storage** peer (668 interactive in 24h),
and storage publishes neither its capacity nor its shed count on any admin JSON the poller
reads — only to Prometheus. That is why a conductor-admission ceiling reaches the ledger
wearing a projector costume, and why `admission-shed` has never once fired for it. Closing it
means adding the storage admission gauge to `/p2p/status` (or `/admin/self-healing`), which is
a storage change plus a fleet roll — deliberately NOT taken here, and NOT a threshold question.

## Verification

- 2026-09-13 12:42:18Z — `/admin/self-healing`, `/p2p/status` and `/admin/render-stats`
  re-fetched on elohim.host (HTTP 200, quoted above); `/health` `uptime: 8771` (boot ≈10:17Z).
- Condition confirmed **LIVE**, not self-resolved: `caughtUp: false`, `divergentAnchor: 59`,
  `healedTotal: 0` over 23 sweeps, `converged: false`.
- Prometheus (read-only, datasource `prometheus`): the four PromQL families quoted above over
  24h / 6h / hourly-25-bucket windows.
- A-side control at the same minute (`doorway-alpha.elohim.host/p2p/status`):
  `{"completed": 7, "healedTotal": 73, "sweeps": 32, "caughtUp": false, "converged": false}`.
- `python3 .claude/scripts/_lib/__tests__/runtime_harvest_test.py` → **59 assertions passed**,
  exit 0, with the live ledger and cursor byte-identical before and after the run (checked).
- Replay of the fixed predicate over `.claude/data/runtime-cursor.json`: alpha SILENT,
  alpha-b → `projector healed NOTHING (healedTotal 0 over 21-109 sweeps, divergentAnchor 59,
  converged=false) sustained >= 3 polls`, fp `79f357281ca5`.
- **Regression signature to watch (updated)**: `elohim_projection_heal_outcomes_total` on adam
  showing a nonzero `healed`/`refreshed` bucket is this concern clearing. `divergent_actionable`
  climbing past the fleet's upper range (~55) while `healed` stays 0, or the interactive shed
  rate exceeding its 24h peak of ~117/h, is it deepening.

---

# 2026-09-17 — third flap, re-triaged: the condition did not move, the SENSING did

## What is exhausted

Ledger line as re-filed (`.claude/data/runtime-findings.jsonl`), and note `seen: 1` /
`first_poll: 91` — this fp was filed as **NEW**, i.e. the previous, correctly-`blocked`
line for the SAME fingerprint had been deleted by closure-by-disappearance:

```json
{"fp": "79f357281ca5", "class": "self-heal-exhaustion", "node": "alpha-b",
 "provenance": "projector:reconcile",
 "line": "projector healed NOTHING (healedTotal 0 over 73-144 sweeps, divergentAnchor 58, converged=false) sustained >= 3 polls",
 "status": "open", "seen": 1, "first_poll": 91, "last_poll": 91,
 "clean_poll_streak": 0, "ts": "2026-09-17T12:25:48+00:00"}
```

Re-fetched live at triage (2026-09-17, `https://elohim.host/p2p/status`, HTTP 200) —
condition **LIVE**, not a transient:

```json
"projectionReconcile": {"pending": 22, "completed": 0, "failed": 2, "caughtUp": false,
  "peersAsked": 5, "divergentAnchor": 58, "healedTotal": 0, "sweeps": 144,
  "exhausted": 0, "converged": false}
```

`https://elohim.host/admin/self-healing` in the same pass: `admission {maxInflight: 256,
available: 256, shedTotal: 0}` (the doorway gate is idle — still the wrong gate, per the
2026-09-13 sensing-gap section), upstream circuit `closed`, `conductor {connected: true,
connectedWorkers: 4, totalWorkers: 4}`, `warmup {completed: true, attempts: 1}`,
`render {total: 164, degenerateRate: 0.0366}`, and **all seven conductor peers
`"status": "Degraded", "lastSeen": null`**.

**The domain reading: the ceiling is STABLE, not deepening.** Against 2026-09-13
(`healedTotal 0` over 21–23 sweeps, `divergentAnchor 59`), four days later the projector
has run ~120 MORE sweeps, healed **zero**, and divergence sits at **58** — flat, one
below. It is not converging and it is not degrading; `divergent_actionable` has not
climbed past the ~55 fleet range and the regression signature named on 2026-09-13
(`elohim_projection_heal_outcomes_total` showing a nonzero `healed` bucket) has not
fired. Adam's `/p2p/status` is otherwise healthy — 6 connected peers, `drain` 3445/3445
published, `pull` caught up, iroh paths at ~8ms RTT and `successRate: 1.0`. The blocked
verdict below is unchanged and correctly blocked.

## Root-cause inventory — of the RE-DISPATCH (the node's own root cause is unchanged)

The 2026-09-13 rewrite fixed the cross-poll-delta defect and predicted the finding would
now "file ONE ledger line that stays present — which is what stops the re-dispatch." It
did not. Replaying `evaluate()` over the persisted window in `.claude/data/runtime-cursor.json`
(read-only, `_heals_nothing` per sample) shows why:

| sample | healedTotal | sweeps | divergentAnchor | converged | `_heals_nothing` |
|---|---|---|---|---|---|
| 0 | 0 | 88 | 9 | false | True |
| 1 | 0 | 109 | 9 | false | True |
| 2 | 0 | 21 | 59 | false | True |
| 3 | 0 | 20 | 51 | false | True |
| 4 | 0 | 52 | **0** | false | **False** |
| 5 | 0 | 73 | 51 | false | True |
| 6 | 0 | 144 | 58 | false | True |
| 7 | 0 | 144 | 58 | false | True |

`healedTotal` is **0 in every single sample of the window** — the condition never lapsed
for one poll. The lone `False` is sample 4's `divergentAnchor: 0`, which is the SAME
multi-process artifact `_projector_lag`'s own docstring names for `sweeps`: consecutive
polls are answered by different storage processes behind doorway-B's upstream pool, and a
young process reports `divergentAnchor: 0` because it has not yet discovered divergence.
The 2026-09-13 fix removed cross-poll *arithmetic*, but each per-sample term is still drawn
from a possibly-different process, and the predicate requires **every** sampled process to
agree (`all(_heals_nothing(r) for r in reports)`).

**The structural amplifier — the actual defect, and it is not in the predicate.** A
predicate that ANDs over a sliding window of `W` samples goes silent for exactly `W` polls
after one aberrant sample (the bad sample sits in the last `W` windows). With
`LAG_POLLS == CLOSE_STREAK == 3`, **one junk sample is exactly sufficient** to delete a
live ledger line:

- window `[2,3,4]` → contains sample 4 → silent (clean_poll_streak 1)
- window `[3,4,5]` → contains sample 4 → silent (clean_poll_streak 2)
- window `[4,5,6]` → contains sample 4 → silent (clean_poll_streak 3 → **DELETE**)
- window `[5,6,7]` → all True → re-files as **NEW** → full Opus triage dispatch

That is this dispatch, and it is the third flap of fp `79f357281ca5` on a condition that
has not changed since 2026-07-27.

- `.claude/scripts/_lib/runtime_harvest.py:24` — `CLOSE_STREAK` (the amplifier)
- `.claude/scripts/_lib/runtime_harvest.py:~203` — `_projector_lag`, the `all(...)` quorum
- `.claude/scripts/_lib/runtime_harvest.py:~190` — `_heals_nothing`, the per-sample terms

**Committed-ledger proof that the line was deleted, not merely re-worded.** `git diff` of
`.claude/data/runtime-findings.jsonl` against HEAD shows the fp's previous incarnation was
already terminal-state and already cited this file:

```json
{"fp": "79f357281ca5", ..., "line": "projector caughtUp=false sustained >= 3 polls",
 "status": "blocked", "seen": 2, "first_poll": 81, "last_poll": 82,
 "backlog": "genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md"}
```

The fingerprint is byte-identical to the one re-filed at poll 91 (fp is computed from
node+class+**provenance**, not the line text — so the 2026-09-13 line rewrite correctly did
NOT move it). Between poll 82 and poll 91 the line was removed by
closure-by-disappearance and then re-filed at `seen: 1`, losing both `status: blocked` and
the backlog citation — i.e. **losing the entire suppression state this concern exists to
hold.** Blocked is supposed to be terminal for automation; closure-by-disappearance
silently un-terminals it.

**And it is not confined to this fingerprint.** The same diff shows fp `2b4761b2eaf6`
(`provide-loop-dead-remaining-stuck` on alpha-b, `status: blocked`, cited to
`genesis/data/timeline/backlog/dataplane-reanchor-dead-remaining-rekeyed-peer.md`) was
closed over the same interval — while the live 2026-09-17 `/p2p/status` still reports
`reanchorDeadRemaining: 9` **unchanged** and `reanchorCaughtUp: false`. Only
`stuckSweeps` moved (3 → 1), which is a per-PROCESS counter resetting under the same pool
churn. Two independently-triaged `blocked` lines were therefore retired by sampling noise
rather than by resolution, which makes this a defect of the closure rule itself, not of any
one predicate.

## Fix path

**Applied (bounded, monotone-safe): `CLOSE_STREAK` 3 → 5**, with the arithmetic written
into the constant. Closure hysteresis must be STRICTLY GREATER than the widest predicate
window (`max(OPEN_POLLS, SHED_POLLS, LAG_POLLS, DEGEN_POLLS) == 3`); 4 absorbs one aberrant
sample, 5 absorbs two adjacent ones. This change can only make a line persist LONGER before
closure — it cannot make any predicate go silent, so it adds no blind spot, which is the
failure mode the previous two rewrites of this predicate family each introduced. A
genuinely self-resolved finding still closes promptly. **Standing invariant:
closure-by-disappearance must be slower to believe a condition ended than a predicate is to
stop asserting it.**

**Proposed, deliberately NOT applied (needs a daylight pass, not a background agent):**
`_heals_nothing` currently treats an *abstaining* sample as a *refutation*. A sample with
`healedTotal == 0` and `divergentAnchor == 0` is a process with nothing to say, not
evidence that the projector is healthy. The honest quorum is: a window affirms exhaustion
when at least one sample affirms it and **no** sample REFUTES it, where refutation is
`healedTotal > 0` or `converged is True` — the two readings that actually mean "some
process healed something / adjudicated the divergence". This is a semantic change to a
predicate that has been rewritten twice in five days, and the standing lesson from both
rewrites is that a hasty tightening goes silent on the very condition it was written for.
It wants the full stored-window replay plus a fresh fixture, in daylight.

**Second proposal, also NOT taken — a design question for the deterministic-layer owner,
not a background agent.** `CLOSE_STREAK = 5` raises the noise floor but does not change the
rule that a `blocked` line is deletable at all. Design note D5 in `reconcile()` says
closure applies to "ANY status — runtime exhaustions self-resolve without triage", and for
an `open` line that is exactly right. For a `blocked` line it is questionable: `blocked` is
documented as **terminal for automation** — a human decided this needs an operator lever —
and closure-by-disappearance silently discards that decision along with its backlog
citation, so the next flap pays a fresh Opus dispatch to re-derive a verdict that was
already written down. The candidate rule is that `open` closes on `CLOSE_STREAK` as today,
while `triaged`/`blocked` require either a much longer streak or an explicit re-check
(which is what the stasis sweep already exists to do). That is a change to the ledger's
state machine and to the role boundary between the poller and the stasis sweep; it should
be decided deliberately, not inside a triage pass.

**Also still open from 2026-09-13, unchanged:** storage publishes neither its admission
capacity nor its shed count on any admin JSON the poller reads, which is why a conductor-
admission ceiling keeps reaching the ledger wearing a projector costume. Closing that is a
storage change plus a fleet roll; `elohim/elohim-storage/src/p2p/projection_reconcile.rs`
remains under operator WIP and out of this pass's write-set by rule.

## Current decision

**BLOCKED — node condition unchanged and correctly blocked; the re-dispatch amplifier is
fixed.** The operator/cluster levers named in the 2026-09-13 "Current decision" are still
the only things that move adam's projector, and all of them remain outside a background
triage agent's remit (cluster action, hosted-agent sharding, conductor CPU, or the
demand-driven reconcile ramp — the last being an **actuation** loop this agent's remit
explicitly excludes). Nothing about the 2026-09-17 re-fetch argues for reopening that
verdict: divergence is flat at 58 and no regression signature fired.

What the poller should cite on re-encounter: this file. With `CLOSE_STREAK = 5` the
`blocked` ledger line now survives the multi-process sampling noise that deleted it, so the
fingerprint stays present and dispatch stays suppressed. The stasis sweep owns the re-check.

## Verification

- 2026-09-17 — `https://elohim.host/admin/self-healing` and `https://elohim.host/p2p/status`
  re-fetched (HTTP 200, both quoted verbatim above). Condition confirmed **LIVE**:
  `healedTotal: 0` over `sweeps: 144`, `divergentAnchor: 58`, `converged: false`,
  `caughtUp: false`.
- Predicate replayed read-only over the persisted `.claude/data/runtime-cursor.json` window
  (table above): `healedTotal == 0` in 8 of 8 samples; exactly one sample refutes, on
  `divergentAnchor: 0`.
- `python3 .claude/scripts/_lib/__tests__/runtime_harvest_test.py` → **60 assertions
  passed**, exit 0, after the `CLOSE_STREAK` change. The closure test is written against
  `rh.CLOSE_STREAK` rather than a literal, so it follows the constant.
- `.claude/data/runtime-findings.jsonl` and `.claude/data/runtime-cursor.json` verified
  byte-identical (sha256) before and after the suite run — the ledger-pollution leak fixed
  on 2026-09-13 has not regressed.
- No cargo, no mesh, no cluster action, no push taken in this pass; the two in-flight
  worktrees were not touched.
- **Regression signature to watch (unchanged from 2026-09-13)**:
  `elohim_projection_heal_outcomes_total` on adam showing a nonzero `healed`/`refreshed`
  bucket is this concern clearing. `divergent_actionable` climbing past ~55 while `healed`
  stays 0, or interactive shed exceeding its 24h peak of ~117/h, is it deepening.
- **New signature for the sensing fix**: fp `79f357281ca5` should now hold `status:
  blocked` with `seen` advancing monotonically. Another `seen: 1` re-file on this fp means
  `CLOSE_STREAK` was insufficient and the `_heals_nothing` quorum change above is required.

---

# 2026-09-24 — fourth filing: the chronic ceiling CLEARED and closed honestly; a fresh post-deploy process re-filed it

## What is exhausted

The ledger line was re-filed as NEW (`.claude/data/runtime-findings.jsonl`, poll 161):

```json
{"fp": "79f357281ca5", "class": "self-heal-exhaustion", "node": "alpha-b",
 "provenance": "projector:reconcile",
 "line": "projector healed NOTHING (healedTotal 0 over 7-45 sweeps, divergentAnchor 59, converged=false) sustained >= 3 polls",
 "status": "open", "seen": 1, "first_poll": 161, "last_poll": 161,
 "clean_poll_streak": 0, "ts": "2026-09-24T00:38:46+00:00"}
```

The 2026-09-17 section predicted that "another `seen: 1` re-file on this fp means
`CLOSE_STREAK` was insufficient". **That prediction does not hold for this re-file.** The
stored alpha-b window in `.claude/data/runtime-cursor.json`, polls 154–161, replayed
read-only through `_heals_nothing`:

| poll | sweeps | healedTotal | divergentAnchor | pending | peersAsked | converged | `_heals_nothing` |
|---|---|---|---|---|---|---|---|
| 154 | 125 | 0 | 59 | 0 | 6 | **true** | False |
| 155 | 144 | 0 | 59 | 0 | 6 | **true** | False |
| 156 | 150 | 0 | 59 | 0 | 6 | **true** | False |
| 157 | 163 | 0 | 59 | 0 | 6 | **true** | False |
| 158 | 166 | 0 | 59 | 0 | 6 | **true** | False |
| 159 | **7** | 0 | 2 | 1 | **0** | false | True |
| 160 | **7** | 0 | 2 | 1 | **0** | false | True |
| 161 | 45 | 0 | 59 | 2 | 6 | false | True |

Polls 154–158 are ONE process (sweeps monotonic 125 → 166), and it reported `converged:
true, caughtUp: true, pending: 0` on every poll. That is the projector's own SLO field
saying all 59 divergent anchors are adjudicated. The 2026-09-17 incarnation was therefore
deleted because **the condition really ended**, over five clean polls from one monotonic
series. It was not deleted by sampling noise. `CLOSE_STREAK = 5` behaved as designed, and
the `_heals_nothing` quorum change proposed on 2026-09-17 is **not** indicated by this event.

The chronic ceiling cleared somewhere between 2026-09-17 (144 sweeps, pending 22,
unconverged) and poll 154. The storage cures that landed in that interval are the likely
cause. The record cannot attribute it more precisely because Prometheus was unreachable
from this pass. The cures are: `0ff361417` (chain writes queued without holding
admission), `c52651ebe` (the REA heal leg reads on the background lane and remembers a
verdict it cannot change), `ef18d08a4` (an id the conductor cannot see is retried on the
clock, backing off), and `aa55b2e71`/`2402a3fd8` (held candidates and standing contests
are not re-minted every sweep).

The re-file comes from a **new process**. Sweeps reset to 7 with `peersAsked: 0` and
`divergentAnchor: 2`: a process still inside its first discovery pass, which had asked no
peers. `/db/p2p/conductor-diagnostics` on both doorways shows every iroh connection opened
at `1790198196`–`1790199588` (≈ 2026-09-23 21:16–21:40Z). `https://elohim.host/health`
reported `uptime: 12968` at 00:40:11Z (boot ≈ 21:04Z). A fleet roll happened around 21:05Z,
consistent with the conductor pin commits `d8b8aa19f` (19:20Z) and `d3d7175ce` (20:37Z).
Polls 159 and 160 are byte-identical, so they landed inside the same sweep. "Sustained >= 3
polls" covered minutes of wall-clock here, not hours.

## Re-fetch at triage — LIVE, but this is post-deploy catch-up, not the chronic ceiling

`https://elohim.host/p2p/status`, sampled once a minute for ten minutes (HTTP 200 each time):

```
00:40:11Z {"pending": 2,  "failed": 0, "caughtUp": false, "peersAsked": 6, "divergentAnchor": 59, "healedTotal": 0, "sweeps": 45, "converged": false}
00:40:53Z {"pending": 5,  "failed": 3, "caughtUp": false, "peersAsked": 6, "divergentAnchor": 59, "healedTotal": 0, "sweeps": 46, "converged": false}
00:47:54Z {"pending": 55, "failed": 3, "caughtUp": false, "peersAsked": 5, "divergentAnchor": 59, "healedTotal": 0, "sweeps": 47, "converged": false}
00:49:54Z {"pending": 55, "failed": 3, "caughtUp": false, "peersAsked": 5, "divergentAnchor": 59, "healedTotal": 0, "sweeps": 47, "converged": false}
```

One sweep takes about **7 minutes** on adam right now (46 → 47). `https://elohim.host/admin/self-healing`
at the same time returned: `admission {maxInflight: 256, available: 256, shedTotal: 0}`,
upstream circuit `closed`, **all seven conductor peers `Healthy`** (they were `Degraded` on
2026-09-17), `warmup.completed: true`, `conductor {connected: true, connectedWorkers: 4/4}`,
and `projector {caughtUp: false, divergentAnchor: 59}`.

A-side control, `https://doorway-alpha.elohim.host/p2p/status`, from the same restart:

```
~00:38Z  {"pending": 0, "caughtUp": true,  "divergentAnchor": 59, "healedTotal": 0, "sweeps": 58, "converged": true}
00:50:10Z {"pending": 2, "caughtUp": false, "divergentAnchor": 59, "healedTotal": 0, "sweeps": 60, "converged": false}
```

Matthew's fresh process has converged at least once (sweep 58). Its `converged` then
switches sweep to sweep as new pending rows appear and are adjudicated. Adam has not been
seen converged in any sample of its fresh process: sweep 7, sweep 45, or the live 45–47. So
adam still catches up more slowly than matthew after a roll, which is the original July
shape of this concern ("catch-up stalls after a deploy restart"). It is no longer the
weeks-long ceiling the 2026-09-13 and 2026-09-17 sections measured. **That ceiling
converged before this deploy.**

## Root-cause inventory

- **The re-file itself.** Nothing is defective. The predicate
  (`.claude/scripts/_lib/runtime_harvest.py` `_projector_lag` / `_heals_nothing`) read three
  samples from a real, currently unconverged process. Closure by disappearance
  (`reconcile`, `CLOSE_STREAK = 5`) retired the previous line on genuine convergence. The
  agent contract calls this "regression handling for free". The cost is that each fleet
  roll can re-file this fp during adam's post-restart catch-up window.
- **`healedTotal` no longer discriminates.** Both nodes now report `healedTotal: 0` and the
  same `divergentAnchor: 59`, in converged and unconverged samples alike. The verdict-memory
  cures mean rows now **adjudicate** rather than heal. `healedTotal: 0` is the fleet's
  healthy norm, and in practice `_heals_nothing` has become `converged is False` held across
  `LAG_POLLS` samples. On matthew, `converged` switches sweep to sweep (58 true, 60 false).
  Three polls that each land on an unconverged sweep would file a matthew finding. Nothing
  has done so yet, because matthew's window holds 6 converged samples out of 8. The risk is
  latent and named here. It is not fixed.
- **Poll count is not a duration.** `LAG_POLLS = 3` counts SessionStart-driven polls. Polls
  159 and 160 were the same sweep. The per-sample `sweeps >= MIN_SWEEPS (3)` floor treats a
  7-sweep process with `peersAsked: 0` as having a "base". The `booted` fixture in
  `.claude/scripts/_lib/__tests__/runtime_harvest_test.py` models a fresh boot as sweeps
  0–2, but a real fresh boot measured here is 7 sweeps, 0 peers asked.
- **Substrate (unchanged owner).** Adam's post-roll convergence rate is still set by
  conductor admission and full-arc saturation on its conductor:
  `elohim/elohim-storage/src/conductor_admission.rs` (capacity = `db_max_readers − 3`) and
  `elohim/elohim-storage/src/p2p/projection_reconcile.rs` (the sweep and the `converged`
  fold). The cure design (coordinated warm-up and a reconcile ramp keyed on
  `elohim_conductor_admission_*`) is owned by
  `genesis/data/timeline/backlog/fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11.md`.

## Fix path

**No code change in this pass. That is deliberate.** Each candidate poller change trades a
flap for a blind spot, and this predicate family has already been rewritten twice for that
exact trade:

- *A `peersAsked > 0` abstain term* would have kept polls 159 and 160 from counting. That
  only delays this filing by two polls if adam is still unconverged then. It also blinds
  the predicate to an isolated projector that never reaches a peer ("cannot sweep"), which
  is a real exhaustion.
- *Raising `MIN_SWEEPS`* adds a warm-up grace measured in sweeps. Sweep length depends on load.
  It is about 7 minutes on adam now, and sweeps 7 → 45 took roughly three hours, so no
  sweep count maps to a stable duration.
  The genuine 2026-09-13 filing happened at 21–23 sweeps. Any floor above that would have
  delayed a real detection by hours.
- *Detecting a process reset* by comparing `sweeps` across polls would break the standing
  lesson on this poller: a counter on these endpoints belongs to a process, not a node.

The honest cure is a **duration-denominated warm-up grace**. It needs storage to publish its
process age on `/p2p/status`, for example a `projectionReconcile.startedAt` or an
`uptimeSecs` field. The predicate would then abstain while `uptime < restart-churn
envelope`, the "~20 min churn + hours of catch-up" the substrate trust contract names.
Adding that field is a storage change plus a fleet roll, and `projection_reconcile.rs` is
under operator WIP in the working tree. The change is named here and not taken.

## Current decision

**BLOCKED.** The verdict is the same, but it now rests on a narrower concern. The chronic
ceiling is **resolved** (five converged polls on one process, polls 154–158). What remains
live is adam's slow post-deploy convergence. Its lever is the coordinated warm-up and
reconcile ramp owned by the fleet saturation record. That is an **actuation** loop, and
this agent's remit excludes it.

Ledger line `79f357281ca5` is set to `status: blocked` and cites this file, so dispatch
stays suppressed while adam's fresh process catches up. **Expected next state:** adam
converges the way its previous process did. After five clean polls the poller deletes the
line by disappearance, and that deletion is honest. The next fleet roll may re-file the fp
during catch-up. That filing is known to be this post-deploy shape, and the fix for it is
the uptime-graded grace above.

**Stasis-sweep re-check criterion:** if adam is still `converged: false` on every sampled
sweep more than 12 hours after the 21:05Z 2026-09-23 roll (so by about 2026-09-24T09:00Z),
the chronic ceiling has regressed under conductor pin `7e553f9c3`. The 2026-09-13
Prometheus families (`elohim_projection_heal_outcomes_total`,
`elohim_conductor_admission_shed_total`, `elohim_conductor_admission_in_flight`) are then
the next read.

## Verification

- 2026-09-24 00:40–00:50Z: `https://elohim.host/p2p/status` sampled once a minute for 10
  minutes. `https://elohim.host/admin/self-healing`, `https://elohim.host/health`,
  `https://doorway-alpha.elohim.host/p2p/status` and `/db/p2p/conductor-diagnostics` on both
  doorways were read once each. All returned HTTP 200 and are quoted above. The condition
  is **LIVE**: sweeps 45 → 47, `converged: false` throughout, pending 2 → 55, failed 0 → 3.
- Cursor window replay (read-only, table above): converged on 5 of 5 samples from the prior
  process; three affirming samples from the fresh process, two of them with `peersAsked: 0`.
- `git diff HEAD -- .claude/data/runtime-findings.jsonl` shows the committed `blocked` line
  (poll 91) removed and replaced by the `seen: 1` line (poll 161). This is consistent with
  the honest closure shown in the window.
- Not verified here: Prometheus heal-outcome and admission-shed families. The observability
  datasource was not reachable from this pass, so attribution of the clearing to the
  2026-09-17..21 storage cures is inferred from commit timing, not measured.
- No cargo, mesh, cluster action or push in this pass.
- **Correction to the 2026-09-17 signature:** a `seen: 1` re-file of this fp is evidence
  that `CLOSE_STREAK` was insufficient **only if** the cursor window before the re-file
  contains no run of 5 converged samples from one monotonic sweep series. Check the window
  before concluding.
