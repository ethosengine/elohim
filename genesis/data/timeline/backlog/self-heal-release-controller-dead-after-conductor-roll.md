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
  - elohim/elohim-storage/src/conductor_bridge.rs
  - elohim/elohim-storage/src/services/conductor_commitment_author.rs
  - elohim/elohim-storage/src/services/salvage_commitment_author.rs
  - elohim/elohim-storage/src/services/spool_custody_author.rs
  - elohim/elohim-storage/src/services/custody_rotation.rs
  - elohim/elohim-storage/src/services/holochain_humans_replayer.rs
  - elohim/elohim-storage/src/services/membership_identity_reconcile.rs
  - elohim/elohim-storage/src/services/feedback_projector/conductor.rs
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

## Residue — closed 2026-09-08 (`1416b9b27`)

`SoakAttestor` and `ReleaseAttestationCtx` held the same boot-time
`Arc<HcClient>` clone. That is the second half of rung 5's receipt — the measure
is adoption AND attestation (james attested ~+86 s after adopting), so a peer
adopting through the re-read registry and attesting through a corpse still fails
the leg the next deploy is meant to prove. The soak observer's window makes it
structural rather than rare: it is spawned at apply time and makes its ONE call
`soakSecs` later, and alpha's conductor pods rolled 6-31 minutes after their
storage pods.

Closed by extending the same seam, not a second one:

- `ConductorBridge` moved out of `release_adoption::watch` into
  `src/conductor_bridge.rs` — a conductor-leg concern with two consumers, and
  the same defect had now landed twice. `watch` re-exports it, so `watch::…`
  paths and the `main.rs` wiring read unchanged.
- The trait gained `call_zome_classed(…, AdmissionClass)`, with `call_zome`
  delegating at `Background`. The class is a parameter because the consumers
  legitimately differ: a controller sweep is `Background`, authoring a soak
  attestation is the `Interactive` write it has always been. Moving to a bridge
  must not silently re-class either — the test asserts the class.
- `ReleaseAttestationCtx` holds `Arc<dyn ConductorBridge>`; `from_bridge` is the
  production constructor, `new(Arc<HcClient>)` survives wrapping a
  `StaticConductorBridge` so every existing caller and test stands. The ctx is
  built **after** the soak sleep, from the bridge, not captured when the
  observer was spawned. In the threshold reader a mid-re-mint `None` deflates
  the count as `Unresolved` — the same degrade rule the per-issuer arm already
  applied, never a failed sweep.
- `main.rs` builds **one** bridge and hands it to both the controller and the
  soak attestor, so the two halves of the rung-5 receipt cannot drift apart
  again.

Test (RED first — pre-cure shape panics
`ConductorRefused("captured handle")` on the second call):
`release_attestation::tests::an_attestation_lands_after_the_conductor_rolled_during_the_soak_window`
— one `ReleaseAttestationCtx`, refused while the bridge is mid-re-mint, landing
once the supervisor re-minted it, `handle_reads == 2`. A ctx holding an
`Arc<HcClient>` cannot express that: there is nothing to flip.

---

# Systemic pass (2026-09-08)

Third leg of the same defect class, and the one with a **measured consequence on
the ward**. Household mesh, `/tmp/elohim-local-mesh/logs/jessica.log`, receipts in
`/projects/.claude-config/jobs/a41a0b08/tmp/t11c-mesh.log`:

- 04:27:18 — jessica's conductor SIGKILLed.
- 04:27:23 — `conductor bridge RE-MINTED after a conductor restart` (supervisor).
- 04:27:27 — `HcClient bridge connected (late)` for lamad and imagodei.
- 04:27:25 → 04:33:15 — **812 lines** of `ELECTION READ FAILED … Zome call
  failed: Websocket`, `reanchor_backfill: row not healed this sweep`,
  `projection-reconcile: conductor get failed; retry next sweep`, the spool
  custody tick, `record_peer_status`, and the household replayer — all against
  a handle latched **before** the kill, for ~6 minutes after the substrate was
  healthy again.
- Consequence: the death witness authored inside that window never anchored on
  jessica and never replicated to matthew, so **death-witness station 3b-ii**
  fails on "matthew holds a copy within 120 s" while james (which received it by
  another path) passes. The same pattern appears 2026-09-07 22:40–22:58 after an
  earlier kill.

## The 21 sites, classified

Class **(a)** = one-shot boot use that completes before any conductor could roll,
or a site that already resolves per use — leave it. Class **(b)** = captured into
a long-lived task/service — must resolve per use.

| main.rs | site | class | what resolves it now |
|---|---|---|---|
| 1498 | `infrastructure_client()` boot probe | (a) | predicate only; the task inside acquires via `connect_role_forever`, and the heartbeat's `ZomeCallPublisher`/`DefaultProbe` already hold the **registry** and resolve per tick |
| 1565, 1567 | comment + `set_client("infrastructure", …)` | (a) | a writer, not an acquisition |
| 2121 | provide `ConductorCommitmentAuthor` | **(b) CURED** | `190c7e077` — `ConductorCommitmentAuthor::from_bridge`, `RegistryConductorBridge::lamad`, re-read per tick |
| 2277 | `SalvageCommitmentAuthor` | **(b) CURED** | `190c7e077` — `SalvageCommitmentAuthor::from_bridge`, same bridge |
| 2378 | `SpoolCustodyAuthor` | **(b) CURED** | `190c7e077` (author converted) — but main.rs:2394 still passed the CAPTURED `Arc<HcClient>` to `::new`, wrapping a `StaticConductorBridge`; `5fc2981dc` wires the boot site through the (new, shared) `lamad_conductor_bridge()` helper instead |
| 2464 | `ConductorRotationAuthor` | **(b) CURED** | `190c7e077` — `ConductorRotationAuthor::from_bridge`; the tick's `resolve_self_agent_cid` now shares the SAME bridge instead of the boot-time client it used to close over |
| 2577 | `reanchor_lamad_boot` | (a) | one-shot boot sweep; the task returns after one run. Its **periodic** re-runs ride projection-reconcile's heal leg, converted below |
| 2676 | `lamad_client().is_none()` | (a) | boot-time predicate; the task it guards stores the late client into the registry |
| 2731 | self-agent-cid resolver closure | (a) | already resolves `lamad_client()` **inside** the closure, per call |
| 2966 | `agent_key_uhcak()` | (a) | extracts a value; no handle retained |
| 3058 | `agent_key_uhcak()` cell-key snapshot | (a) | same |
| 3134 | comment | (a) | — |
| 3272 | `binding_mint` imagodei | (a) | one-shot idempotent task; no loop |
| 3504 | `cell_id().dna_hash()` | (a) | value extraction |
| 3800 | `agent_key_uhcak()` announce | (a) | value extraction |
| 4781 | `cell_id().dna_hash()` | (a) | value extraction |
| 4820 | `membership_identity_reconcile::spawn_periodic` | **(b) CURED** | `190c7e077` — takes `Arc<dyn ConductorBridge>` (`RegistryConductorBridge::imagodei`, new constructor), builds `ConductorMembershipReader` fresh inside the loop each pass |
| 5300 | `lamad_hc_boot` → projection-reconcile heal leg | **(b) CURED** | `RegistryConductorBridge::lamad`, re-read every tick via `conductor_bridge::sweep_leg` |
| 5337 | `agent_key_uhcak()` iroh agent_cid | (a) | value extraction |
| 5734 | `hc_for_adoption` | **(b) CURED** | `80354948d` (controller) + `1416b9b27` (attestation rail) |
| 5977 | feedback projector `ConductorReader` | **(b) CURED** | `190c7e077` — `ConductorReader::from_bridge`, reads route through `ConductorBridge::call_zome_classed` at `Interactive`; `origin_dna_hash` memoized on first live resolve |

Sites **outside** `main.rs` were audited and are already correct: `http.rs`,
`api/*`, `p2p/view_federation.rs` and `p2p/mod.rs` all hold the **registry** and
resolve `lamad_client()` / `imagodei_client()` inside the handler or per call —
`view_federation.rs:319` even says so in a comment. The defect is confined to
`main.rs` composition sites that clone a handle into a long-lived task.

## Cured this pass (`e1a62ce52`)

The projection-reconcile heal leg — the home of election-obey, the re-anchor
sweep and the commitment heal, i.e. three of the six loops the 04:27 window
names, and the one the 3b-ii failure runs through.

Its conductor leg was `StdArc<OnceLock<Arc<HcClient>>>`: seeded once, read
forever. `bridge_up = hc_cell.get().is_some()` therefore stayed `true` across a
roll, so every tick spawned a heal against a corpse instead of reporting an
unavailable leg and retrying. It is now
`StdArc<dyn ConductorBridge>` = `RegistryConductorBridge::lamad`, read per tick
through the new `conductor_bridge::sweep_leg` → `SweepLeg::{Unavailable, Live{generation}}`,
and re-resolved a second time inside the single-flight CAS (the window is small,
but "we asked once and kept the answer" is the whole defect class). The
off-ticker late-connect now stores into the **registry** rather than a private
cell, so the bridge sees it — and cannot re-introduce the capture.

`NoConductorBridge` was added for the honest third case: a node wired with
neither a registry nor a boot client reports `Unavailable` forever, which is
different from a bridge whose handle merely happens to be re-minting.

**No call's behaviour changed** — only who resolves the handle and when.
`heal_decision`, `run_heal`, `run_discovery`, `reanchor_backfill::run_once` and
every `head_adoption` entry point keep their `&Arc<HcClient>` signatures.

**Tests, both RED first** (verified by swapping the fake's leg to set-once
semantics; the discriminating sequence is `healthy → SIGKILL → re-mint`, because
a set-once cell handles plain *late-connect* correctly and only breaks on a
**roll**):

- `p2p::projection_reconcile::tests::a_conductor_rolled_between_two_sweeps_heals_on_the_second`
  — RED: *"a set-once cell still reports the pre-kill handle here — 812 log
  lines' worth"*.
- `services::reanchor_backfill::tests::a_conductor_rolled_between_two_sweeps_reanchors_on_the_second`
  — RED: *"no handle is honest backpressure; the next sweep asks again"*.

## Cured this pass (`190c7e077`) — the four periodic authors, membership, and the feedback projector

The six remaining class-(b) sites (2121, 2277, 2378, 2464, 4820, 5977) shared
one shape: a ticker that built an author/projector **once**, outside its loop,
from a captured `Arc<HcClient>` (or `Arc<HcClient>`-returning
`registry.*_client()` clone), and reused it for the life of the process. Each
now holds `Arc<dyn ConductorBridge>`, resolved fresh per tick/call:

- `ConductorCommitmentAuthor`, `SalvageCommitmentAuthor`, `SpoolCustodyAuthor`
  gained `from_bridge` constructors (`new(Arc<HcClient>)` survives wrapping a
  `StaticConductorBridge`, unchanged for any caller/test that holds a client
  directly); the sync `CommitmentAuthor` trait impls resolve
  `self.bridge.handle()` at the top of `author_custody_blob`/`author_commons`,
  returning an honest `Err` (skip this tick) on `None` before any conductor
  write is attempted.
- `ConductorRotationAuthor` — same shape. Its main.rs ticker ALSO used to
  resolve `resolve_self_agent_cid` against the same boot-time captured
  handle; that call now reads through the SAME bridge the author uses, so
  identity resolution and authoring heal together rather than the identity
  half staying stale.
- `ConductorMembershipReader` (`services::holochain_humans_replayer`) gained
  `from_bridge`/`new`; `membership_identity_reconcile::spawn_periodic` now
  takes `Arc<dyn ConductorBridge>` and builds the reader fresh INSIDE the
  loop each pass, rather than once outside it. New
  `RegistryConductorBridge::imagodei` constructor (the identity/qahal role
  slot) — `RegistryConductorBridge::lamad` was the only one that existed.
- `feedback_projector::conductor::ConductorReader` gained `from_bridge`/`new`;
  its `call` helper now goes through `ConductorBridge::call_zome_classed` at
  `AdmissionClass::Interactive` (pinned explicitly — the trait's `call_zome`
  convenience defaults to `Background`, which would have silently re-classed
  every feedback read). `origin_dna_hash()` (a sync trait method with no
  handle to await) memoizes the DNA hash on first successful resolve: the
  hash is a property of the DNA, not the connection, so it survives a roll,
  and a `""` fallback during a mid-remint window would otherwise misclassify
  every envelope as foreign-DNA rather than honestly pending.

**No zome call's behaviour changed** — same zome names, same fn names, same
admission classes, same cell targets (mishpat/imagodei-cell calls still go
through `HcClient::call_zome_mishpat`/`call_zome_imagodei` on the
bridge-resolved handle, not through the bridge's own default-cell
`call_zome`).

**Tests** (`just gate elohim-storage`, `EXIT=0`): each site gets a fake
`ConductorBridge` counting `handle()`/`call_zome_classed` reads across two
calls, proving per-tick resolution rather than a captured handle —
`Arc<HcClient>` cannot be constructed in a unit test (real
`AdminWebsocket`/`AppWebsocket` required), so most of these assert the
DECISION surface (a `None` handle refuses honestly; a second call re-reads),
the same technique `projection_reconcile`'s and `reanchor_backfill`'s cured
tests use:

- `conductor_commitment_author::tests::author_commons_re_reads_the_conductor_bridge_every_tick_never_a_captured_handle`
- `salvage_commitment_author::tests::author_custody_blob_re_reads_the_conductor_bridge_every_call_never_a_captured_handle`
- `spool_custody_author::tests::author_custody_blob_re_reads_the_conductor_bridge_every_call_never_a_captured_handle`
- `custody_rotation::tests::author_rotation_successor_re_reads_the_conductor_bridge_every_call_never_a_captured_handle`
- `holochain_humans_replayer::tests::list_memberships_re_reads_the_conductor_bridge_every_call_never_a_captured_handle`
- `feedback_projector::conductor::tests::refs_for_target_lands_after_the_conductor_rolled_between_two_sweeps` — this one
  DOES round-trip a full "refused while dead, lands once re-minted" cycle
  without a real `Arc<HcClient>`, because `ConductorReader`'s reads go
  entirely through `call_zome_classed`, which a fake can override directly
  (mirroring `release_attestation`'s cured test); plus
  `origin_dna_hash_is_empty_with_no_bridge_ever_live_never_a_guess`.

**Measured 2026-09-08 06:44** (household mesh, `kill -9` of jessica's
conductor at 06:44:11, re-minted at 06:44:24): reanchor_backfill (2),
election-obey (2), peer_status (2), discovery (2), identity_fill (1), and
the humans replayer (1) all recovered inside the ~13 s dead window — EXCEPT
the spool-custody tick, which was still on the `StaticConductorBridge` wrap
(main.rs:2394 passed a captured handle to `::new` despite the readiness
predicate reading the registry) and failed `Websocket closed: No
connection` 78 times over 6.5 minutes, still failing every 5 s at 06:50:59.
Fixed in `5fc2981dc`, which also converts all six author-construction sites
to two shared `lamad_conductor_bridge()`/`imagodei_conductor_bridge()`
helpers and adds `ConductorBridge::bridge_kind()` so a composition test
(`main::conductor_bridge_composition_tests`) asserts what main.rs actually
constructs is `BridgeKind::Registry` — not measured against a live roll
again yet.
