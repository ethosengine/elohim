---
id: "backlog-self-heal-release-controller-dead-after-conductor-roll"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The release-adoption controller captures its conductor handle at boot — one conductor pod roll parks every channel on conductor_unavailable at the 1800 s rung, permanently"
slug: "self-heal-release-controller-dead-after-conductor-roll"
written: "2026-09-08"
author: "claude (rust-architect — live READ-ONLY diagnosis of the alpha fleet via doorway-alpha + Prometheus, 03:28Z)"
status: "cure-landed-unmeasured"
priority: "high"
relatedNodeIds: []
tags: [self-heal-exhaustion, release-adoption, rung-5, conductor-bridge, backoff, runtime-upgrade-propagation, alpha-fleet, declare]
cites:
  - elohim/elohim-storage/src/services/release_adoption/watch.rs
  - elohim/elohim-storage/src/services/release_adoption/state.rs
  - elohim/elohim-storage/src/services/release_adoption/mod.rs
  - elohim/elohim-storage/src/hc_client_registry.rs
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/metrics.rs
---

# A conductor roll kills the adoption controller for the life of the process

> Diagnosed READ-ONLY against the running alpha fleet at **2026-09-08 03:28 UTC**,
> after edge build **#1443** deployed commit `6bef71b8`. Nothing was restarted,
> deployed, or written to by this diagnosis.

## Symptom, with numbers

`GET https://doorway-alpha.elohim.host/db/p2p/adoption?peer=<x>` for
matthew / jessica / james / adam / gertrude / susan / eve — **every channel on
every peer**:

```
verdict.state      = refused
refusal.reason     = conductor_unavailable
refusal.detail     = "this peer could not resolve the channel through its own
                      conductor — unreachable, which is never absence"
refusal.arm        = watch
refusal.transient  = true
consecutiveRefusals= 9–10
nextCheckNotBefore = the 1800 s rung
controller.running = true
controller.sweeps  = 159
```

The controller is **running and sweeping**. It is not wedged, not shed, not
backpressured. It is asking a corpse.

**The roll, from the pod clocks** (Prometheus `kube_pod_start_time`, namespace
`elohim-alpha`):

| pod | start (unix) | UTC |
|---|---|---|
| `elohim-matthew-alpha-0` (storage) | 1788825162 | 23:52:42 |
| `elohim-james-alpha-0` (storage) | 1788825163 | 23:52:43 |
| `elohim-james-alpha-conductor-0` | 1788825638 | **00:00:38** |
| `elohim-matthew-alpha-conductor-0` | 1788827033 | **00:23:53** |

`installedReality.readAtUnixSecs` = **1788825248** (23:54:08) on matthew,
1788825328 on adam — i.e. each storage pod read its conductor successfully
**once, at boot**, and the conductor pod rolled **6–31 minutes later**. Three
hours on, every channel still reports `conductor_unavailable`.

**Zero container restarts. All containers `Ready=1`.** Readiness does not cover
this, and cannot: the storage process is healthy; only one of its two conductor
paths is dead.

**The other conductor path is alive.** `GET /db/p2p/conductor-diagnostics` on
the same doorway at 02:22 UTC returned **33 fresh agent-infos**, "preferring the
embedded conductor's" view. So the conductor is up, reachable, and answering —
for everything except the adoption controller.

*Read caveat:* doorway-A reads are single-target for the household peers (the
identical numbers across matthew/jessica/james suggest so); adam routes
differently and is also refused. Seven distinct channel reports, one shape.

## The mechanism, file:line

**One line captures the handle.**

`elohim/elohim-storage/src/main.rs:5734`

```rust
let hc_for_adoption = hc_registry_for_http.as_ref().and_then(|r| r.lamad_client());
```

`lamad_client()` clones the `Arc<HcClient>` **out of** the registry slot. That
clone is handed to the controller at `main.rs:5893-5894`
(`controller.with_conductor(hc.clone())`) and stored at
`services/release_adoption/watch.rs:521` as `hc: Option<Arc<HcClient>>` — a
field, read for the life of the process.

**The bridge underneath it is re-minted, and the controller never follows.**
`hc_client_registry.rs:410-470` (`spawn_bridge_supervisor`) probes each
supervised role every `BRIDGE_PROBE_INTERVAL` (20 s); on a failed `ping` it
clears the slot (`set_client(role, None)`), runs `connect_role_forever`, and
stores a **fresh** `Arc<HcClient>` in the slot. Its own doc names this defect
exactly, at `hc_client_registry.rs:456`:

> *"Drop the dead handle FIRST. **Anything holding an Arc clone keeps failing
> until it re-reads**, but nothing NEW picks up a corpse…"*

The adoption controller was the thing holding an Arc clone that never re-read.
Why the roll is fatal rather than merely disruptive is stated at
`hc_client_registry.rs:390-399`: `holochain_client` 0.8 has **no reconnect**,
and a conductor restart both closes the sockets *and* invalidates the in-memory
app-auth token — so recovery is a full re-mint, not a socket retry. A captured
`Arc<HcClient>` can never heal itself.

Every read the controller makes goes through that field —
`resolve_head` (watch.rs:773), `resolve_record_summary` (:829),
`resolve_candidate_head` (:932), `read_threshold` (:1024),
`fetch_path_evidence` (:1597), and both lineage-window reads
(`watch/lineage_windows.rs:56,68`). `resolve_head` maps any `Err` to
`Answer::Unreachable` (watch.rs:790-799), which `check_channel` (watch.rs:1206)
turns into the `conductor_unavailable` refusal seen on the wire. Correct
behaviour on honest evidence — the evidence is just permanently wrong.

**Why `transient: true` still climbs to 1800 s.** `transient` is the *retry
axis* (`mod.rs:911` `is_transient`) — it says "keep asking", not "keep asking
often". `state.rs:679-684` computes the rung as
`min(consecutive_refusals, BACKOFF_LADDER_SECS.len() - 1)` over
`[0, 30, 120, 600, 1800]`, with **no class-specific ceiling**. Four refusals
reach 1800 s; alpha's channels were at 9–10.

**And the ladder's own escape hatch is powered by the dead handle.**
`should_skip_for_backoff` (state.rs:635) bypasses the timer when the channel's
canonical head has *changed* — but the cheap probe that reads that head
(`sweep_once`, watch.rs:1064-1071) is the same `resolve_head` through the same
corpse. It answers `Unreachable`, which maps to `current_cid = None`, which
honours the timer. The one mechanism that could have broken the park depends on
the exact thing that is broken.

> **The exhaustion, stated once:** the controller's self-heal is the sweep, and
> the sweep re-asks a handle that can never come back. Three hours of
> `transient: true` is the controller truthfully reporting that it will retry,
> retrying, and having no path to a different answer.

This blocks **every rung-5 measure**: james is the canary (`mode=canary`), and a
canary that cannot resolve a head adopts nothing.

## Cure landed (2026-09-08)

Two changes, both in this batch. Neither adds a second reconnect strategy — the
supervisor's re-mint is the only one, and the controller now follows it.

1. **The conductor leg is a SOURCE, not a captured handle.** New
   `watch::ConductorBridge` trait (`handle()` + a defaulted `call_zome`), with
   `RegistryConductorBridge::lamad(registry)` holding the **registry** and
   re-reading `registry.client("lamad")` on every use. `main.rs` wires it
   whenever a registry exists; `with_conductor(Arc<HcClient>)` survives only as
   the no-registry fallback (and now wraps a `StaticConductorBridge`, so the
   pinned shape is named rather than implied). All eight `self.hc` call sites go
   through `self.conductor()` / the bridge. No caller-side timeout was added —
   the `Background` admission class remains the bound, per the
   conductor-call-is-uncancellable rule.
   New counter `elohim_release_channel_conductor_reconnect_total` (registered at
   boot, so a flat zero is readable) increments when the controller picks up a
   handle whose identity differs from the last one it used — i.e. when it
   *followed* a re-mint. Deliberately distinct from the supervisor's own
   reconnect signal, which cannot say whether the controller noticed.
   *(House metric prefix is `elohim_`; the task named it
   `storage_release_channel_conductor_reconnect_total`. One name, house
   convention — trivially renamable if the operator prefers the other.)*

2. **A class ceiling on the backoff ladder.** `RefusalReason::max_backoff_rung()`
   (`mod.rs`) caps `conductor_unavailable` at `OUR_OUTAGE_MAX_BACKOFF_RUNG = 2`
   — the **120 s** rung — and leaves every other class on the full ladder.
   `state.rs`'s `record()` applies it as a second `.min()`. The reasoning is the
   one the refusal itself states: `conductor_unavailable` is a fact about US,
   not a verdict about the channel. A peer that could not ask has learned
   nothing, so throttling it throttles only our own recovery. A conductor roll
   is ≈20 min of churn; parking for 30 min to observe it is self-inflicted
   blindness. At 120 s the cost stays one Background-classed local resolve per
   channel per two minutes, well inside `MAX_CHANNELS_PER_SWEEP`.

**TDD, red then green:**

- `watch::tests::a_rolled_conductor_is_re_resolved_and_the_next_read_answers` —
  a `RollingBridge` dead for its first call, alive after. Asserts
  `Unreachable → Absent` (Absent is reachable *only* when a conductor answered)
  and `handle_reads == 2`. The pre-cure controller could not express this test
  at all: it asks its source once, ever.
- `state::tests::our_own_outage_never_climbs_past_the_two_minute_rung` —
  12 consecutive `conductor_unavailable` refusals, asserting the delay never
  exceeds the 120 s rung. Verified **RED without the cure**: fails at sweep 3
  with *"our own outage parked at 600s"*.
- `state::tests::the_our_outage_cap_does_not_leak_into_other_transient_classes`
  — the cap is class-scoped; `artifact_unavailable` / `threshold_unmet` /
  `quorum_unmet` / `installed_reality_unknown` keep the full ladder.

## Which habit this serves

- **`runtime-upgrade-propagation`** — primary. Every rung-5 measure runs through
  the adoption controller; a controller that dies on an ordinary conductor roll
  makes the whole election path a function of pod-restart ordering.

## Done when

**All seven alpha channels resolve within 2 sweeps of the next edge deploy, read
via the adoption route** — `GET /db/p2p/adoption?peer=<x>` for
matthew / jessica / james / adam / gertrude / susan / eve shows a verdict other
than `conductor_unavailable` (`ok`, `idle`, or an honest channel-level refusal)
within two sweep intervals of each peer's conductor having last rolled.

Supporting reads on the same pass:

1. `elohim_release_channel_conductor_reconnect_total` is **non-zero** on peers
   whose conductor rolled after their storage pod — the controller followed the
   re-mint, rather than the series staying flat while channels refuse.
2. No channel's `nextCheckNotBefore` sits more than 120 s out while its refusal
   reason is `conductor_unavailable`.
3. `consecutiveRefusals` for that class stops climbing past the point the
   conductor is answering `/db/p2p/conductor-diagnostics`.

## Residue — same disease, not cured here

`SoakAttestor::from_runtime` (`main.rs:5897-5911`) is built from the same
boot-time `hc_for_adoption` clone and keeps a captured handle
(`release_adoption/apply.rs:1799`). A post-apply soak attestation attempted
after a conductor roll will fail the same way. It is the attestation rail, not
the resolve path, so it does not cause this symptom — but it is the second
instance of the class and belongs in the same sweep. Left explicitly, not
silently: it needs the `ReleaseAttestationCtx` to take a bridge rather than a
client, which is a wider signature change than this cure.
