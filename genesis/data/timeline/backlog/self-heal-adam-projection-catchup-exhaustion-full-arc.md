---
id: "backlog-self-heal-adam-projection-catchup-exhaustion"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam (B / elohim.host) projection catch-up stalls after a deploy restart — cells are NOT authorities until their storage arc reconverges, so every heal get_links leaves the box and dies on the 60s conductor request timeout"
slug: "self-heal-adam-projection-catchup-exhaustion-full-arc"
written: "2026-07-27"
updated: "2026-09-12"
author: "claude (resiliency-saga sprint-3 delivery — ch06 runtime blocker RCA); mechanism corrected 2026-07-29 (rust-architect, probe-confirmed); ledger-bound 2026-09-12 (runtime-triage)"
status: "wip"
priority: "high"
self_heal_status: blocked
severity: high
ci_status: blocked
jobs: [elohim-edge]
fingerprints: [79f357281ca5]
nodes: [alpha-b, elohim-adam-alpha]
tags: [self-heal-exhaustion, projection-reconcile, catch-up, storage-arc, arc-convergence, kitsune2-gossip, get-strategy-local, adam, shem, restart-churn, heal-timeout, ch06, declare, chronic-flap, elevate-arm]
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
