---
title: Actuatable Self-Healing Control Plane
id: actuatable-self-healing-control-plane-design
status: Draft
class: substrate
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
created: 2026-06-13
cites:
  - conductor-authority-arc-memory-scaling | 2026-06-13-conductor-authority-arc-memory-scaling | sha256:18cbb190f6a8a3a1 | path: genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md
  - conductor-authority-arc-auto-policy | 2026-06-13-conductor-authority-arc-auto-policy | sha256:7fb163dd262e129c | path: genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-auto-policy.md
  - self-healing-user-agency-opportunity-map | Self-Healing & User-Agency Opportunity Map | sha256:31400dda6437b0dd | path: genesis/docs/superpowers/specs/2026-06-13-self-healing-user-agency-opportunity-map.md
  - self-healing-control-plane-program-roadmap | Self-Healing Control Plane | sha256:23e24b020eed9564 | path: genesis/docs/superpowers/plans/2026-06-13-self-healing-control-plane-program-roadmap.md
  - conductor-leak-jemalloc-cure-verdict | Conductor leak | sha256:049eccfdb959ebd6 | path: genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md
  - conductor-leak-rca-native-heap-reframe | Conductor leak | sha256:ec6d6d1baa3bbbf6 | path: genesis/docs/content/elohim-protocol/history/2026-06-18-conductor-leak-rca-native-heap-reframe.md
---

# Actuatable Self-Healing Control Plane

> ## ⛔ LEAK-AXIS CORRECTED — 2026-06-19 (the control-plane design STANDS)
> The "Open question" (~line 314) concludes "the durable fix is the arc, not the RAM limit." For the alpha
> conductor OOM this is WRONG: it was a native glibc-malloc arena leak, CURED by an allocator swap
> glibc→jemalloc (not arc-shrink — arc=0 nodes leaked the same shape; not a RAM bump). Arc-factor remains a
> legitimate *corpus-memory scaling* lever, but it is NOT this leak's fix. The self-healing control-plane
> design (observability/actuation/REA-bounded knobs, detect→recover→verify→elevate) is unaffected and stands
> — only the leak-remedy attribution is corrected.
> Truth: genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md · genesis/docs/content/elohim-protocol/history/2026-06-18-conductor-leak-rca-native-heap-reframe.md


## 1. Concept

The self-healing surface of an Elohim node is not a dashboard — it is a **control plane** that is both OBSERVABLE (every reliability primitive's live state is readable as a projection/view) and ACTUATABLE (every knob can be tuned, paused/resumed, run-now, reset, bounded-restarted, or re-derived through a typed contract). Its defining property — the headline invariant — is **STRUCTURAL NO-OVERWHELM**: no peer and no runtime can EVER be overwhelmed, because independent limits are layered at every communication edge (Auto ceiling, inbound admission, outbound restraint, propagated backpressure, circuit-break/quarantine, elevate) so the failure of any one layer cannot collapse the system. Four stances make this real: (1) **self-protection & negotiation** — each node manages its own stability in front of its peers, no operator required; (2) **defense in depth & bilateral flow control** — because WE OWN BOTH ENDPOINTS of every link, we mandate TWO-WAY contracts where a receiver advertises capacity and a sender restrains to that window, and BOTH enforce; (3) **resource-aware Auto defaults** — the node deterministically detects its container (cgroup cpu/mem/disk) and observed peer count and settles into optimal defaults, a video-game "Auto" preset whose ceiling IS the flow-control credit budget; (4) **agent-actuatable** — the plane is driven by typed contracts, not a human-only console. Three consumers read and drive it: a human operator, a future "controls" UI, and the real target — an AI agent self-debugging FOR the agent runtime on the user's own device. Per the p2p-design-gate and protocol gospel, an actuation is never an admin API key — it is a **bounded, audited, revocable REA action** modeled on `Mishpat::Commitment` with the `delegates-compute` action, so the actor's blast radius is exactly its granted, revocable commitment scope. The plane closes the loop **detect → recover → verify → elevate**: it senses pressure from the read model, actuates a bounded recovery, verifies the symptom cleared, and on failure ELEVATES a finding onto the existing deterministic ledger + sentinel rather than silently retrying. The whole design generalizes the ONE working precedent already in the tree — `render_semaphore` derived from `compute_budget` (`doorway/doorway-service/src/main.rs:557-600`) — to every edge that today has a one-way, swallowed defense.

## 2. Mutual protection & bilateral flow control — THE NO-OVERWHELM INVARIANT

This is the centerpiece, grounded in the live worked example: doorway CPU-throttled at `cgroup cpu:1` during warm-up by streaming from ~10 broken upstream peers. The mechanism is **NOT a code wedge and NOT a concurrency explosion** — `warm_stream::spawn_stream_task` (`doorway/doorway-service/src/projection/warm_stream.rs:269-298`) loops peers **sequentially**, one at a time, with no per-event fan-out. The freeze is **serial CPU-bound churn**: against ~10 broken upstreams, each peer burns the full 5× exponential-backoff ladder (`MAX_WARMUP_RETRIES=5`, `BASE=10s` → `MAX=120s`, `warm_stream.rs:20-26`) while SSE JSON parsing runs CPU-bound on the throttled core, and a single slow upstream can hold the per-stream `reqwest` timeout of **300s** (`warm_stream.rs:98`) — which **exceeds the entire ~150–210s liveness kill window** (`genesis/orchestrator/manifests/doorway/alpha.yaml:247-268`: startup 24×5s=120s, liveness 30s×5+15s≈165s). `/health` never touches the projection store (`doorway/doorway-service/src/routes/health.rs:237-249` returns unconditional 200), so the probe starves only because the CPU is pegged, and kubelet SIGKILLs the pod. It is the SIMULTANEOUS failure of stance 1 (no upstream self-protection — broken peers retried hot), stance 2 (no inbound admission on warm-up; upstreams advertise no capacity; doorway pulls past its cpu:1 ceiling — the missing two-way contract), and stance 3 (cpu:1 not reflected in sizing; `worker_threads` hardcoded at 4). The durable cure composes all three.

### (a) Per-upstream health + capacity model — on the EXISTING bones

Build on what exists, not a parallel scheme. The data sources already in the tree:

- **`peer_statuses` table + `PeerStatusRecorded` signal** (`elohim/elohim-storage/src/signals.rs:463-574`, projected at `elohim/elohim-storage/src/main.rs:687,725`) — peer online/degraded/unreachable + `last_seen`; already served at `/api/v1/peer-statuses`.
- **`record_health_attestation`** (`doorway/doorway-service/src/services/federation.rs:275`) — probes cached peers' `/health` every 5th 60s tick and writes a DHT attestation. **The advertisement channel exists; nothing consumes it back.**
- **`PullStatusInfo` / `AcquisitionState`** (`elohim/elohim-storage/src/p2p/acquisition.rs:213`) — tri-state receiver→requester pull signal.
- **`PeerHealthRegistry`** (`elohim/elohim-compute/src/peers.rs:38-93`) — per-peer Healthy/Degraded/Offline + `reconnect_attempts`; **observes but exposes NO `is_open()`/`should_skip()` circuit accessor**.
- **`peer_selection.rs:138`** already consumes `peer_statuses.status`/`last_seen` for shard placement — so wiring warm-up and blob-fetch to the SAME view is *wiring*, not a new primitive.

The model layers onto these: per upstream `{success_rate, latency_ewma, last_good, error_streak, advertised_capacity}`, keyed by `STORAGE_URLS`, fed by the existing `/health` probe and `peer_statuses`.

### (b) The TWO-WAY contract

We can mandate bilateral flow control precisely because **we own both ends** of app↔doorway↔storage↔conductor↔peer. The contract: the **receiver advertises capacity** (a credit/window on its `/health` or compute-budget view — `warm_credits`, `available_permits`, `Retry-After`), the **sender restrains to the advertised window** (clamps its `max_concurrent`/`parallelism`/dispatch to the receiver's stated credit AND to its own Auto ceiling), and **BOTH enforce** (receiver caps via its admission semaphore and sheds 429/503; sender caps via the credit and never exceeds it). Today only `reconcile_rails` (`elohim/elohim-storage/src/p2p/reconcile_rails.rs:142-154`, `DispatchBudget.max_inflight` with `available(in_flight)`) accounts credit locally — the wire never advertises it. The Auto ceiling (§3) IS the receiver's credit budget.

### (c) DEFENSE IN DEPTH — independent layers per edge

The invariant survives a single layer's failure because each edge stacks INDEPENDENT limits:

| Layer | Mechanism | Reference (existing or to-build) |
|-------|-----------|----------------------------------|
| 1. Auto ceiling | resource-derived concurrency cap = the credit budget | `render_semaphore ← compute_budget` (`main.rs:557-600`) — the working precedent |
| 2. Inbound admission | receiver semaphore + SHED 429/503/Retry-After | storage `MAX_CONCURRENT_REQUESTS=64` (`elohim/elohim-storage/src/http.rs:132`) — but queues, never sheds; doorway accept loop (`server/http.rs:1129`) has **ZERO** |
| 3. Outbound restraint | sender clamps to receiver's advertised window | `KICK_FETCH_PER_PEER_PER_MINUTE`, `MAX_ACQUISITION_INFLIGHT=25` — sender-side only, ignore receiver capacity |
| 4. Propagated backpressure | 429/503 + Retry-After or credit on the wire | **MISSING** — only emitter is `import_api.rs:557`; nothing honors one |
| 5. Circuit-break | per-upstream breaker opens after N failures | **MISSING** — `PeerHealthRegistry` has data, no accessor |
| 6. Quarantine | eject broken peer from the working set, cooldown + re-admit | `FeedbackSignal::SignalKind::Quarantine` (`elohim/elohim-storage/src/p2p/feedback_signal.rs`) **exists as a B2-attested signal**; no config consumes it |
| 7. Elevate | failed self-heal rides the deterministic ledger + sentinel | `.claude/data/*.jsonl` ledger pattern (existing) |

If the Auto ceiling mis-derives, inbound admission still sheds; if admission is bypassed, the circuit breaker still trips; if the breaker is slow, quarantine still ejects. **No single layer is load-bearing for the invariant** — that is what makes the no-overwhelm property *structural* rather than best-effort. The render path is the only edge that today has even one real shed (layer 1+2 via `try_acquire_owned` → CSR shell, `server/http.rs:2958-2983`).

### (d) Self-protective QUARANTINE at the peer-SET level

A per-peer timeout does NOT fix the freeze: the per-stream 300s timeout (`warm_stream.rs:98`) is itself longer than the liveness window, and even a correct per-peer deadline leaves ~10 peers' aggregate backoff-ladder churn pegging the throttled CPU. The cure must bound **TOTAL upstream work vs the ~210s window**, which decomposes into four moves that DO match the serial mechanism: (i) a **total warm-up CPU/time budget** << the liveness window (e.g. ≤50% of 150s), enforced across the whole peer set; (ii) a **per-upstream circuit-breaker** so the node stops burning the 5× backoff ladder on known-broken peers (open after K consecutive failures, skip them entirely); (iii) **shrink the per-stream timeout** to a fraction of the kill window so one slow peer can never outlast liveness (concrete P0 fix: 300s → e.g. 20–30s); (iv) a **per-tick yield** so warm-up cannot monopolize the runtime. The cpu>1 *generalization* — `warmup_concurrency = max(1, floor(cpu_quota))` to parallelize and fill faster — is a §3 Auto knob, NOT the cpu:1 cure (at cpu:1 concurrency is already 1).

### (e) Per-edge table: one-way today vs needs two-way contract

| Edge | Today | Backpressure | Needs |
|------|-------|--------------|-------|
| app → doorway (front door) | UNBOUNDED accept loop (`server/http.rs:1129`) | none (no 429/503) | inbound admission (layer 2) + shed |
| doorway → upstream (warm-up SSE) | sequential, no budget, no health gate (`warm_stream.rs:269`) | swallowed (300s stall) | **the prime two-way contract**: receiver advertises `warm_credits`, sender restrains + circuit-breaks |
| doorway → storage (`forward_to_storage`) | `reqwest::Client::new()` no timeout/pool/cap (`routes/storage_proxy.rs:112`) | 502 only, no Retry-After | pooled client + deadline + breaker |
| doorway → conductor (zome calls) | 10s timeout, mutex-serialized (`services/zome_caller.rs:40`) | one-way timeout | partial; capacity advertise optional |
| storage inbound HTTP | semaphore=64, `acquire().await` (`http.rs:708`) | QUEUES, never sheds | shed 429 + advertise `available_permits` |
| storage ↔ peer (libp2p req/resp) | timeout + size-cap only (`p2p/behaviour.rs:372-439`) | none | per-stream concurrency + credit |
| storage ↔ conductor | bounded mpsc=100 (`conductor_client.rs:58`) | natural (channel-full blocks) | TWO-WAY-ish already; advertise credit |
| storage ↔ peer (reconcile) | `DispatchBudget` local credit (`reconcile_rails.rs:144`) | local only | advertise `credits_remaining` on the wire |

### (f) EXISTS vs MISSING

**EXISTS:** `compute_budget`-derived `render_semaphore` (the resource-derived + shed precedent); `peer_statuses` + `PeerStatusRecorded` + `record_health_attestation` (advertisement channel); `PullStatusInfo` tri-state; `FeedbackSignal::Quarantine` (B2-attested quarantine primitive); `reconcile_rails` local credit; `peer_selection.rs:138` peer-status consumer (proof the wiring pattern works); storage admission semaphore (64). **MISSING:** doorway inbound admission (0); 429/Retry-After honored anywhere; per-upstream circuit-breaker / `is_open()` accessor; warm-up health gate + total budget; capacity advertisement on any wire; sender restraint to a receiver's window; libp2p connection limits (grep for `ConnectionLimits`/`max_established` returns empty); runtime-mutable knob surface (every knob is hardcoded or restart-env); `route rate_limit_rpm` is **DEAD config-theater** (declared `route_registry.rs:91`, never enforced).

### (g) p2p-design-gate — capacity-attestation, credit/window, quarantine

**Capacity / health attestation** (a peer states "I have N credits / am degraded"):
1. **Class B2** — agent-scoped + attestation (a node attests its own capacity; peers consume it). Extends, does not replace, `record_health_attestation`.
2. **DHT entry type: EXISTING** — rides the health-attestation entry already written by `record_health_attestation`; add a `capacity`/`credits` field, no new entry type.
3. **Identity: agent-composite** — `(agent_pub_key, observation_window)`; the attesting agent is the subject, last-write-wins per window.
4. **Coordinator fn + signal:** existing attestation write path in `services/federation.rs`; projects via `PeerStatusRecorded` → `peer_statuses` (`signals.rs:463`). No new signal needed for the read side.

**Flow-control credit / window** (the live, ephemeral credit a sender holds against a receiver):
1. **Class C** — operational/ephemeral. It is runtime state, never notarized; it expires the instant the connection does.
2. **DHT entry type: NONE** — never hits the DHT. It is a wire field (`credits_remaining` in the request_response reply / a `warm_credits` field on the `/health` view) plus in-memory accounting (`reconcile_rails::DispatchBudget`).
3. **Identity: n/a** (ephemeral) — keyed in memory by `(peer_id, protocol)`.
4. **Coordinator fn + signal: none** — it is a node-local read-model surfaced on `/p2p/status`; no zome involvement. This is the correct rejection of relational-default drift: a credit window is NOT a row.

**Quarantine decision** (this node ejects a broken upstream):
1. **Class B2** — agent-scoped + attestation; the quarantining node attests the decision and its standing impact.
2. **DHT entry type: EXISTING** — `FeedbackSignal` with `SignalKind::Quarantine` + `StandingImpact` (`p2p/feedback_signal.rs`). Do NOT mint a new entry.
3. **Identity: agent-composite** — `(quarantining_agent, quarantined_peer, opened_at)`; content-addressed via the signal payload CID.
4. **Coordinator fn + signal:** the `flood_feedback` path that already encodes `FeedbackSignal` (`p2p_iroh/dual_publish`); projects as a feedback signal consumers already decode.

### (h) Quarantine = bounded / audited / revocable REA action

Quarantine is not a kill switch — it is an REA action with a lifecycle. **Bounded:** opens only after K consecutive failures; affects exactly the named upstream. **Audited:** every open/close is a `FeedbackSignal::Quarantine` payload with provenance (the attesting agent, the evidence — `error_streak`, `last_good`). **Revocable:** auto-expires on a cooldown; a **half-open re-admission probe** (one trial stream) closes the circuit if the peer responds healthily. **Anti-self-partition guard (NEVER violate):** quarantine must NEVER reduce the working upstream set below a floor that would partition the node off the network — if quarantining the next peer would leave zero healthy upstreams, the action is REFUSED and a finding is ELEVATED instead. Concrete files: `doorway/doorway-service/src/projection/warm_stream.rs`, `elohim/elohim-compute/src/peers.rs` (add `is_open`/`record_outcome`), `elohim/elohim-storage/src/p2p/feedback_signal.rs`.

## 3. Resource-aware Auto defaults

The "Auto" preset is the foundation: it sets the credit budget that §2 enforces.

### (a) The resource-probe

Detection primitives already exist in `elohim/elohim-storage/src/services/system_metrics.rs`: `cpu_count()` (line 119, cgroup-aware via `available_parallelism`), `total_memory_bytes()` (lines 99-115), `load_average()` (135), `filesystem_capacity_bytes()` (statvfs). **NOTHING wires these into a default at runtime** — they feed only the compute dashboard (`api/compute.rs:269-344`). **Two preconditions before deriving:** (i) `cpu_count()`/`available_parallelism()` returns an integer floor — it cannot see a fractional `cpu:1.5` or the actual `cpu.max` quota/period; the probe must read `/sys/fs/cgroup/cpu.max` directly so the plane can explain "workers=4 because quota=1.0". (ii) **`total_memory_bytes()` reads HOST RAM, not the cgroup limit** (`/sys/fs/cgroup/memory.max` is never read) — a `mem:512Mi` pod sees host GBs, so ANY mem-derived default is UNSAFE until `cgroup_mem_limit_bytes()` is added. This is a precondition, not an afterthought. The probe would live as one `detect_resources() -> ResourceSnapshot` in `system_metrics.rs`, called at boot in both `doorway/doorway-service/src/main.rs` and `elohim/elohim-storage/src/main.rs`.

### (b) The deterministic derivation — a PURE FUNCTION

```
fn derive(snapshot: ResourceSnapshot) -> DerivedConfig
// snapshot = { cpu_quota, mem_limit (cgroup, not host), disk_free, observed_peer_count }
```

Deterministic, side-effect-free, re-runnable. Outputs (each clamped to a safe floor):

| Output | Derivation | Today (hardcoded) |
|--------|-----------|-------------------|
| `worker_threads` | `max(4, ceil(cpu_quota)×k)` from `cpu.max`, NOT `available_parallelism()` | `DEFAULT_WORKER_THREADS=4` (`main.rs:47`) |
| `warmup_total_budget` | `≤ 0.5 × liveness_window` | none |
| `warmup_per_stream_timeout` | `< liveness_window − margin` | `300s` (`warm_stream.rs:98`) |
| `warmup_concurrency` | `max(1, floor(cpu_quota))` | sequential (effectively 1) |
| `warmup_retry_cap` | `f(cpu_quota, peer_count)` | `MAX_WARMUP_RETRIES=5` |
| `render_isolates` | `min(cpu_total, ceiling, allocation)` | **already derived** (`main.rs:557`) |
| `FALLBACK_CANDIDATE_CAP` | `f(observed_peer_count)` | `8` (`reconcile/custody.rs:88`) |
| `storage_admission` | `floor(cpu_quota × per_req)` | `MAX_CONCURRENT_REQUESTS=64` (`http.rs:132`) |
| `pool_worker_count` / `pool_queue` | `f(cpu_quota)` / `f(mem)` | `4` / `1000` (`worker/pool.rs:51,54`) |
| reconcile/custody/sync cadences | `f(peer_count, churn)` | hardcoded literals (`p2p/mod.rs:2202-2243`) |
| cache sizes (`BLOB_CACHE_MAX_GB`, `shard_cache_max_bytes`) | `f(disk_free, mem_limit)` | `1 GB`, env (`cache/tiered.rs:128`, `delivery_relay.rs:72`) |
| **flow-control credit budget** | **= the Auto ceiling** | none |

### (c) Precedence

**operator override > Auto-derived > safe floor.** Every existing env knob (`DOORWAY_WORKER_THREADS`, `WORKER_COUNT`, `SHARD_MAX_CONCURRENT_FETCHES`, `DOORWAY_RENDER_OVERRIDE`) stays as the operator escape hatch for testing/ops and ALWAYS wins. Auto fills the gap when no override is set. The safe floor is the last-resort clamp (never 0 workers / 0 credit / 0 isolates).

### (d) Re-derivation when limits change

`derive()` runs at boot AND is re-runnable on an explicit signal (a cgroup-limit-change event or an agent commitment). It is **never** re-run silently on every cgroup read — that silent re-derivation is exactly the scar the worker_threads comment warns against. Re-derivation is cooldown-gated so a flapping cgroup limit cannot thrash the runtime.

### (e) `worker_threads(4)` — the prime fix, and how Auto prevents the incident

The hardcode (`main.rs:47`, comment 30-46) deliberately refuses naive cgroup-derivation because a naive read gave **1 worker on cpu:1 → the original freeze**. The corrected rule keeps that scar's lesson: `worker_threads = max(4, ceil(cpu.max_quota)×k)` — floor 4 preserves the single-blocked-await wedge antidote (≥2 is the structural minimum; pin the doc-wide floor at **4**, the existing default), while the derived term lets a cpu:8 host scale UP (today it caps at 4 forever). **How Auto would have prevented the live incident:** with the resource snapshot wired, doorway at cpu:1 would have (i) sized `warmup_total_budget` to <50% of the liveness window, (ii) set `warmup_per_stream_timeout` < the kill window (not 300s), (iii) derived the credit budget that caps total upstream work — so the aggregate backoff churn could never peg the core inside the ~210s window. The freeze is the absence of stance 3 feeding stances 1+2.

### (f) Exposing detected resources + derived values + the "why"

A single read-model: `{ resources: {cpu_quota, mem_limit, disk_free, observed_peers}, derived: {...}, overrides: {...}, reasons: ["worker_threads=4: floor (cpu quota 1.0)", "warmup_timeout=25s: < liveness 150s"] }`. The "why" string is first-class — the agent/UI/operator must see the DERIVATION, not just the value. Surface at `/admin/auto-preset` (new) alongside the existing `/admin/capability` and `/health?level=trace`.

### (g) p2p-class

**Cat C — node-local read-model.** Resource snapshot and derived-config are operational, node-local, never notarized: no DHT entry type, no CID, no coordinator fn. Each node serves its OWN snapshot read-only. This correctly rejects the relational default of "store config in a table" — it is a projection of the live container, computed fresh.

## 4. The control surface — taxonomy of the OTHER primitives

| Class | Primitives (files) | Observable today? | Actuatable today? | Auto-derivable? | Edge | Gap |
|-------|--------------------|-------------------|--------------------|-----------------|------|-----|
| Rate-limiters / backpressure | storage sem=64 (`http.rs:132`); render sem (`main.rs:594`); pool sem+queue=1000 (`worker/pool.rs:54`); gossip token-bucket (`conductor_agent_info_gossip.rs:307`); signing limiter (`signing/service.rs:122`); `KICK_FETCH_PER_PEER_PER_MINUTE`; `rate_limit_rpm` (DEAD) | partial (render via `/admin/capability`; storage via `/health?trace`); most no | **only via env restart**; none at runtime | YES for all but signing | mostly one-way (queue/swallow); render sheds | no 429 honored; no runtime mutation; admission swallows not sheds |
| Cooldowns / backoff / retry | warm-up 5×10→120s (`warm_stream.rs:20`); subscriber reconnect 5s→60s (`subscriber.rs:232`); storage-events backoff (`storage_events_subscriber.rs:81`); conductor_client FIXED 5s no-jitter (`conductor_client.rs:57`); EPR refresh 30s (`main.rs:695`); zome reconnect budget (`zome_caller.rs:168`) | partial (WarmupState `/health/startup`; reconnect → `/status`); cadences/backoff-state NOT surfaced | no | partial (retry budgets yes; SLAs no) | one-way; conductor_client = thundering-herd risk (no jitter) | no per-upstream backoff state readable; no "which peers am I backing off" |
| Batch / sweep cadences | custody 120s; projection-reconcile 300s; p2p block (status 30/sync 60/verify 300/replication 60, `p2p/mod.rs:2202`); tending 300s; provide 60s; **inventory_broadcast (archetype-derived, the template)** (`inventory_broadcaster.rs:2256`) | counts on `/p2p/status`; **cadence/next-tick/skip-count NOT surfaced** | NO pause/resume/run-now anywhere; `P2PCommand::ReconfigureCadence` is a TODO (`p2p/mod.rs:2249`) | YES (vs peer-count); inventory_broadcast already does | internal | no lifecycle commands; 15 of 16 cadences hardcoded |
| Restart / lifecycle | livenessProbe/startupProbe/readiness (`alpha.yaml:247-268`); `is_transport_error` clear-on-error (`zome_caller.rs:59`); reconnect budget; render shed-to-CSR | probes k8s-only; readiness via `/ready`; health blind-200 | redeploy only; no subsystem reset | no (probes operator-owned) | one-way | no bounded subsystem restart (reset-conductor / abort-warmup / drain-SSR) without pod kill; liveness can't report internal pressure |
| Race-guards / idempotency | `GapTracker` never-immediate-requeue (`reconcile_rails.rs:13`); `DispatchBudget` credit (`reconcile_rails.rs:142`); `DedupLru` panic-on-poison (`p2p/dedup.rs`); `WriteThroughState` recover-on-poison + admin override (`write_through.rs:233`); `upsert in_scope_of` guard (`db/rea_commitments.rs:729`); Automerge merge (`sync/doc_store.rs`); signal out-of-order tolerance (`signals.rs:1899`) | `/p2p/status` counts; `WriteThroughState` `admin_snapshot()` | **`WriteThroughState` is the ONE runtime-mutable** (typed, RwLock, poison-tolerant); rest no | `DispatchBudget`/caps YES; the GUARDS are correctness, NEVER tune | mixed; `DispatchBudget` two-way | NEVER-touch guards (poison policies divergent BY DESIGN; `in_scope_of`; never-requeue) must be excluded from actuation |

## 5. The actuation model (REA-native)

**An actuation IS a `Mishpat::Commitment` with the `delegates-compute` action** — not an admin key. The commitment grants a bounded, scoped, revocable authority; the act of changing a knob, opening a quarantine, or running a sweep is the *fulfillment* of that commitment, recorded and projectable.

**Full p2p-design-gate (the actuation command / knob-change record):**
1. **Class A — notarized.** An actuation that changes node behavior is a real economic event with on-chain standing; it must be auditable and revocable, so it is notarized, not operational.
2. **DHT entry type: EXISTING** — `Commitment` in `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` with action discriminator `delegates-compute` (already parsed at `elohim/elohim-storage/src/mishpat_projection.rs:223`, payload carries `scope`, `provider`, `recipient`). No new entry type — the actuation scope rides the commitment's `scope` field (e.g. `scope: "doorway.warmup.timeout"`).
3. **Identity: content-derived (CID) = `entry_hash`.** Per gospel (`project_mishpat_commitment_cid_is_entry_hash`), the commitment CID is the `entry_hash`, NOT the action_hash — every bounds-gate/revoke/fetch keys on it. Returning action_hash silently breaks bounds-gating.
4. **Coordinator fn + signal:** the mishpat `create_commitment` path (post_commit hook); projects via `MishpatSignal::CommitmentCommitted` → `mishpat_commitments` (`elohim/elohim-storage/src/main.rs:922`). The actuation's *result* (the new knob value taking effect) projects as a Cat-C node-local read-model change.

**Grant issue / scope / revoke:** the operator (or a parent agent) issues a `delegates-compute` commitment scoping the agent to exactly a set of knobs (`scope: "doorway.warmup.*"`, `bounded_by: <budget>`). The agent's **blast radius = exactly its granted, revocable commitment scope** — it cannot touch a knob outside scope, and revoking the commitment (a Mishpat revocation keyed on the CID) instantly removes all authority. **Audit trail:** every actuation is the fulfillment of a commitment, so the DHT carries the full who/what/when/why; reads project to the control-plane view.

**The typed / MCP-tool surface** the runtime's AI agent calls (authority IS the granted commitment, validated per-call against the scope):
```
tune_knob(knob_id, value)            // bounded to scope + floor; meta-cooldown enforced
pause_sweep / resume_sweep / run_now(sweep_id)
reset_connection(target)             // bounded subsystem restart, not pod kill
quarantine_peer / readmit_peer(peer_id)   // → FeedbackSignal::Quarantine, cooldown
re_derive_auto()                     // re-run derive(), cooldown-gated
adjust_flow_window(edge, credits)    // clamped to [min_credit, Auto ceiling]
```
Template this surface on `WriteThroughState`'s admin override (`write_through.rs:233,307-324`) — typed, `RwLock`-live, poison-tolerant, status-observable — the cleanest existing actuation pattern in the tree.

## 6. Read model — the observable surface

Existing views to compose (no parallel scheme):
- **`/p2p/status`** (`elohim/elohim-storage/src/main.rs:1204`) — projection-reconcile counts, custody state, gap-tracker counts.
- **`/admin/capability`** + **`/admin/render-stats`** — render budget, isolates, SSR fetch SLA, shed counts.
- **`/api/v1/peer-statuses`** + `PeerStatusRecorded` projection — peer health/`last_seen`.
- **`/health?level=trace`** (`http.rs:1481`) — `concurrencyLimit`/`semaphorePermits`.
- **`/health/startup`** — `WarmupState` (in_progress/attempts/last_error).
- **`WriteThroughState::admin_snapshot()`** — live override state.

**New projections needed:**
1. `/admin/auto-preset` — the §3f resource snapshot + derived-config + reasons (Cat C, node-local).
2. **Per-edge flow-control state** — for each edge: advertised capacity, credits in flight, shed count, circuit state (open/half-open/closed), quarantined peers (Cat C node-local + the B2 quarantine attestation for the decision).
3. **Timer registry** — last_run / next_tick / in_flight / skipped_ticks / enabled per sweep (closes the §4 cadence-observability gap).
4. **Upstream health from the consumer's POV** — "which upstreams am I streaming from, their `error_streak`/`last_good`, am I trimming any" (joins `peer_statuses` + the new circuit state).

**Per-agent scoping:** reads are projections and freely readable by any consumer serving its OWN node's state; the *actuation* surface is scoped per the granting commitment. An agent sees the full read model but can only drive the knobs its `delegates-compute` scope names — and a node always serves its own snapshot, never another node's private actuation state.

## 7. Safety rails

This system can wedge itself, wrongly quarantine a healthy peer, mis-derive defaults, or deadlock a too-tight window. Every rail is mandatory:

- **Meta-cooldowns** — rate-limit changes TO the rate-limiters. Each knob has a minimum interval between actuations so an agent cannot flap a limit during a storm (the `re_derive_auto` and `tune_knob` paths both enforce it).
- **NEVER-touch guards** — refuse any actuation that violates a correctness invariant: the divergent poison policies (`DedupLru` panic vs `WriteThroughState` recover) must NOT be harmonized (each is correct for its invariant); the `upsert in_scope_of` guard (`rea_commitments.rs:729`) must never be bypassed (it caused EprRouter-empties); `GapTracker`'s never-immediate-requeue (`reconcile_rails.rs:85`) must never be "drained faster"; `WriteThroughState`'s integrity-kind short-circuit (`write_through.rs:336`) cannot be overridden off. These are read-only to the agent.
- **Quarantine cooldown + re-admission + anti-self-partition** — quarantine auto-expires; a half-open probe re-admits a recovered peer; and quarantine MUST NEVER reduce the working set to a self-partition (refuse + elevate if the last healthy upstream would be ejected).
- **Auto safe-floor + override-wins** — `derive()` never returns 0 workers / 0 credit / 0 isolates / 0 concurrency; the floor (worker_threads ≥ 4) is the last-resort clamp; operator env override ALWAYS wins over Auto.
- **Flow-window anti-deadlock** — a credit window must always grant a MINIMUM credit (`min_credit ≥ 1`); a receiver can never advertise 0 capacity in a way that deadlocks a sender, and `adjust_flow_window` clamps to `[min_credit, Auto ceiling]`.
- **Bounded blast radius + reversibility** — every actuation is scoped to its `delegates-compute` commitment, is reversible (revoke the commitment / restore the prior value), and is audited on the DHT.
- **Elevate-on-failure** — a failed self-heal does NOT silently retry. It writes a finding to the existing deterministic ledger (`.claude/data/*.jsonl`) and rides the sentinel pattern (the flag→agent→canon→stasis automation, ref `feedback_deterministic_flag_agent_canon_stasis_pattern`) so a human or supervising agent picks it up. This is the closed loop's final arm: detect → recover → verify → **elevate**.

## 8. Knob promotion plan

| Knob | Current | Auto-derivation (resource→value) | Operator override | Edge (1-way/2-way) | Promote-to | Risk | Who may actuate |
|------|---------|----------------------------------|-------------------|--------------------|-----------|------|-----------------|
| `worker_threads` | hardcoded 4 (`main.rs:47`) | `max(4, ceil(cpu.max)×k)` | `DOORWAY_WORKER_THREADS` | n/a | restart-knob + auto-derive | M (naive→1=freeze; floor 4) | operator; agent re-derive |
| warm-up per-stream timeout | hardcoded 300s (`warm_stream.rs:98`) | `< liveness_window − margin` (~25s) | new env | doorway→upstream (1-way → 2-way) | runtime + auto | H (300s>kill window=freeze) | operator/UI/agent/node-self |
| warm-up total budget | none | `≤ 0.5 × liveness_window` | new env | doorway→upstream (1-way → 2-way) | new runtime knob | H (the freeze cure) | node-self/agent |
| `MAX_WARMUP_RETRIES` / backoff | hardcoded 5 / 10→120s | `f(cpu, peer_count)` + jitter | new env | doorway→upstream (1-way → 2-way) | runtime + auto | M | agent/node-self |
| storage admission | hardcoded 64 (`http.rs:132`) | `floor(cpu_quota × per_req)`; shed 429 | new env | storage inbound (1-way → 2-way) | runtime + auto + shed | M (overcommit on cpu:1) | operator/agent |
| doorway inbound admission | NONE (`server/http.rs:1129`) | `floor(cpu × per_req)`, floor min | new env | app→doorway (1-way → 2-way) | NEW knob + shed | H (the missing layer-1) | operator/agent |
| pool worker_count / queue | env 4 / hardcoded 1000 | `f(cpu)` / `f(mem)` | `WORKER_COUNT` | doorway→conductor (1-way) | runtime + auto | M (1000-deep=latency sink) | operator/agent |
| render isolates | derived (`main.rs:557`) | already `min(cpu,ceil,alloc)` | `DOORWAY_RENDER_OVERRIDE` | doorway→app (1-way, degraded-serve) | **make runtime-resizable** (watch) | L | operator/agent |
| `FALLBACK_CANDIDATE_CAP` | hardcoded 8 (`custody.rs:88`) | `f(observed_peers)` | new env | storage↔peer (1-way) | runtime + auto | L | agent |
| `DispatchBudget.max_inflight` | constructor const (`reconcile_rails.rs:144`) | `f(cpu,mem,peers)`; **raising re-creates cpu starvation** | new knob | storage↔peer (2-way, local credit) | runtime + auto | H on raise / L on lower | agent (lower); operator (raise) |
| sweep cadences (custody/reconcile/EPR/p2p) | hardcoded/env | `f(peer_count, churn)` via inventory_broadcast pattern | env | internal (n/a) | runtime + lifecycle cmds | M | operator/UI/agent |
| `shard_cache_max_bytes` / blob cache | hardcoded 1GB / env | `f(disk_free, mem_limit)` | env | n/a | runtime + auto | L | operator/agent |
| upstream quarantine | NONE | breaker opens after K; cooldown | new (env escape) | doorway→upstream / storage↔peer (1-way → 2-way) | `FeedbackSignal::Quarantine` | H (mis-quarantine→partition) | node-self/agent (anti-partition guard) |
| `WriteThroughState` override | runtime (`write_through.rs:233`) | n/a (the template) | admin endpoint | n/a | already promoted | L (integrity short-circuit NEVER off) | operator/agent |

## 9. Ranked opportunity map (leverage-per-effort)

| # | Capability | Type | Leverage | Effort | Incident/need | p2p-class |
|---|-----------|------|----------|--------|---------------|-----------|
| 1 | Warm-up per-stream timeout < kill window | knob-promotion | **H** | **S** | the freeze (300s>210s) | C node-local |
| 2 | Warm-up total budget + per-tick yield (bound total work vs window) | flow-control | **H** | S | the freeze (serial CPU churn) | C node-local |
| 3 | Wire warm-up + blob-fetch to consume `peer_statuses` (skip unhealthy) | wiring | **H** | **S** | the freeze; copies `peer_selection.rs:138` | reuse existing |
| 4 | Per-upstream circuit-breaker (`is_open()` on `PeerHealthRegistry`) | self-protection | **H** | M | the freeze; stance 1 | B2 (FeedbackSignal) |
| 5 | `worker_threads = max(4, ceil(cpu.max)×k)` + cgroup cpu reader | auto-config | **H** | S | the prime fix; cpu>1 scale-up | C node-local |
| 6 | `detect_resources()` snapshot + `/admin/auto-preset` view | auto-config | **H** | M | stance 3 foundation; observability | C node-local |
| 7 | Doorway inbound admission semaphore + SHED 429/Retry-After | flow-control | **H** | M | missing layer-1; storm survival | C + wire |
| 8 | `derive()` pure function (the Auto preset engine) | auto-config | **H** | M | stance 3 core; feeds all knobs | C node-local |
| 9 | `forward_to_storage` pooled client + deadline + breaker | flow-control | M | S | uneven defense (no timeout) | C + wire |
| 10 | Storage admission sheds 429 + advertises `available_permits` | flow-control | M | M | bilateral contract seed | B2 + wire |
| 11 | Sweep lifecycle commands (pause/resume/run-now; impl `ReconfigureCadence`) | new-primitive | M | M | incident triage quiescing | A (commitment) |
| 12 | REA actuation model (`delegates-compute` grant/revoke + typed tool surface) | new-primitive | **H** | L | stance 4; the whole actuation spine | A (Mishpat::Commitment) |
| 13 | cgroup MEMORY reader (`/sys/fs/cgroup/memory.max`) | wiring | M | S | precondition for mem-derived knobs | C node-local |
| 14 | Bilateral credit window on reconcile/warm-up wire | flow-control | M | L | full two-way contract | C ephemeral + wire |
| 15 | cgroup-change re-derivation + elevate-on-failure arm | auto-loop | M | M | runtime adaptivity; closed loop | A + ledger |
| 16 | conductor_client backoff jitter (no thundering-herd) | knob-promotion | L | S | recovering-conductor herd | C node-local |
| 17 | render_semaphore runtime-resizable (watch-fed) | knob-promotion | L | M | CPU-pressure shrink no-restart | C node-local |

## 10. Phased path

### P0 — the live-incident cure, composing all stances (highest leverage / lowest effort first)
1. **Warm-up timeout + total budget** (#1, #2): shrink per-stream timeout to `< liveness_window − margin`, add a total warm-up CPU/time budget ≤ 0.5× the window, add a per-tick yield. Pure mechanism match for the serial CPU-churn freeze; smallest diff with the largest effect. (`warm_stream.rs:98,20-26`)
2. **Consume `peer_statuses` + circuit-break upstreams** (#3, #4): wire warm-up and `blob_fetch` to skip `unreachable`/stale peers (copy `peer_selection.rs:138`), add `is_open()`/`record_outcome()` to `PeerHealthRegistry`, open after K failures. Stops burning the backoff ladder on ~10 broken peers. (stances 1)
3. **`worker_threads` Auto + resource snapshot** (#5, #6, #13): read `cpu.max` (and add `memory.max` reader), set `worker_threads = max(4, ceil(quota)×k)`, expose `/admin/auto-preset` with the resource snapshot + derived values + reasons. (stance 3 + the observable surface)
4. **Doorway inbound admission + shed** (#7): the missing defense-in-depth layer-1 — a global request semaphore that SHEDS 429/Retry-After (never throttles `/health` itself). (stance 2)

### P1 — actuation model + bilateral contracts + Auto preset
1. **REA actuation spine** (#12): `delegates-compute` commitment grant/scope/revoke + the typed/MCP tool surface (`tune_knob`, `pause_sweep`, `quarantine_peer`, …), templated on `WriteThroughState`. Human operator + future UI consumers land here. (stance 4)
2. **Full `derive()` Auto preset** (#8): the complete pure function feeding every knob in §8, with precedence (override > Auto > floor) and cooldown-gated re-derivation.
3. **Bilateral capacity advertise/restraint on key edges** (#9, #10, #14): `forward_to_storage` deadline+breaker; storage advertises `available_permits` + sheds 429; warm-up reads each upstream's advertised `warm_credits` and restrains. Health-negotiation back-off becomes real on the edges we own both ends of.
4. **Sweep lifecycle commands** (#11): implement `P2PCommand::ReconfigureCadence` + pause/resume/run-now + the timer registry view.

### P2 — agent-driven self-debug loop + full defense-in-depth
1. **Agent self-debug closed loop + elevate arm** (#15): detect → recover (scoped actuation) → verify (read model) → elevate (deterministic ledger + sentinel on failure). The AI agent self-debugging FOR the runtime on the user's device — the real target.
2. **Runtime re-derivation on cgroup change** (#15): re-run `derive()` on a limit-change signal, cooldown-gated, never silent.
3. **Defense-in-depth across ALL remaining edges** (#14, #17, libp2p connection limits): negotiated credit windows on every wire, runtime-resizable render semaphore, libp2p `ConnectionLimits` (currently absent), conductor_client jitter (#16). The invariant holds at every edge, not just the warm-up locus.

## 11. Open questions / eyes-on verification

1. **Cross-reference against the wf1 user-agency map** (not supplied to this synthesis) — reconcile this control plane's actuation taxonomy with the prior self-healing/user-agency map; confirm no contradiction on which primitives are wired vs dark.
2. **Wiring verification:** `render_semaphore ← compute_budget` is the ONLY resource-derived path verified-wired in the inventory. Do NOT assert any other Auto-derived path is live without inventory/render evidence. `pnpm look` / `/admin/capability` on `https://doorway-alpha.elohim.host` can confirm render budget reflects the actual cgroup at runtime.
3. **`k` multiplier for `worker_threads`** — what `k` in `max(4, ceil(cpu.max)×k)`? Needs a soak on a cpu>1 host to confirm scale-up doesn't reintroduce contention; the floor 4 is settled, the multiplier is not.
4. **`min_credit` floor value** — the anti-deadlock minimum credit per window needs an empirical floor that admits liveness traffic without re-opening overcommit; verify against the ~210s window.
5. **Memory derivation safety** — confirm `cgroup_mem_limit_bytes()` reads `/sys/fs/cgroup/memory.max` (v2) AND the v1 path before any mem-derived knob ships; a host-RAM read here silently over-sizes caches.
6. **Liveness window vs warm-up budget coupling** — the manifest liveness/startup numbers (`alpha.yaml:247-268`) and the derived warm-up budget must stay coherent across redeploys; consider a startup-probe that reflects warm-up pressure so the LB sheds (readiness 503) while liveness stays 200, avoiding a restart storm. Eyes-on: watch a real warm-up at cpu:1 and confirm the per-stream timeout never approaches the kill window.
7. **`delegates-compute` scope vocabulary** — define the canonical `scope` string namespace for knobs (`doorway.warmup.*`, `storage.admission`, …) so commitment grants are unambiguous and the agent's blast radius is precisely bounded.

## 12. The memory axis — bounded-resource extension (added 2026-06-13)

The CPU-throttle freeze is the *request/peer* overwhelm worked example. Live evidence from the same incident surfaces the **memory** worked example, and it threads through all four stances identically — so the control plane must treat memory as a first-class bounded resource, not just a cache-sizing afterthought.

**The evidence (Prometheus working-set, 2026-06-13):** loaded storage/embedded-conductor nodes show a **load-correlated monotonic memory climb**, not a transient spike. james (largest corpus, content=3654) OOM-flapped against 3Gi every ~9 min for hours during the ~13:05 storage churn; an operator 8Gi bump only **moved the OOM ceiling** (james back to ~3.3GB in 15 min; matthew/jessica climbing ~+1.5–2GB/hr toward 8Gi). The quiet node plateaus ~2.2GB; loaded nodes don't. **Open question (do NOT assume leak):** leak vs slow DHT-sync convergence that eventually plateaus. **Decisive cheap test:** watch matthew/jessica — plateau < 8Gi = heavy-but-bounded; reach 8Gi = leak. No Pyroscope is wired; the code-level cause-hunt runs separately (workflow weq8593tf). See `project_storage_node_memory_climb`.

**The design bounds the CLASS regardless of which way that resolves** — if convergence, the budgets+backpressure smooth the climb; if leak, the elevate arm catches it and the per-structure bound contains the blast radius until the code fix lands.

### Memory through the four stances

- **Stance 3 (Auto) — bound the STRUCTURE, not the container.** The cgroup memory reader (§9 #13 / §11 #5 — the precondition; `total_memory_bytes()` reads host RAM today) lets `derive()` compute the mem ceiling. The key move beyond cache-sizing (§3b row): derive a **per-structure memory budget** as a *share* of that ceiling for each growing structure — gossip/sync buffers, projection/observation buffers, blob/shard cache, in-flight reconcile/acquisition state. The operator's reframe is the design principle: *"8GB per peer" breaks the laptop floor; the fix is to find what grows unbounded under churn × large-corpus and cap it.* A leak means no container size is ever enough — so the bound must live on the structure.
- **Stance 2 (defense-in-depth) — memory-pressure is the 8th layer.** OOM is being overwhelmed by your own retained allocations. Add a **working-set watermark monitor** to the layered table (§2c): on approach to the Auto mem ceiling, escalate in order — (i) drop reclaimable caches, (ii) throttle intake via inbound admission (the same semaphore as CPU defense), (iii) refuse new gossip-sync/provide work (advertise reduced capacity — see stance 1), (iv) elevate. Never silently climb to the kill ceiling. This reuses the exact admission/shed machinery the CPU axis builds.
- **Stance 1 (self-protection / bilateral) — memory pressure lowers advertised capacity.** A node's advertised `warm_credits` / `available_permits` (§2b) becomes a function of memory headroom too: as working-set climbs toward the ceiling, the node advertises *less* capacity, so peers back off and stop feeding its growth. Memory pressure flows into the same two-way contract — a memory-pressured receiver is just a low-capacity receiver.
- **Stance 4 (agent-actuatable) — the detect→elevate loop watches the node's OWN curve.** The runtime version of what was done by hand at 1am: the node monitors its own working-set trajectory and **auto-elevates a finding on monotonic-climb-toward-ceiling-despite-caps = the leak signal** (rides the same deterministic ledger + sentinel, §7). Bounded actuations the agent gains: `trim_caches`, `trigger_compaction`, `re_derive_auto` (re-budget structures), `shed_provider_role` (temporarily stop being a heavy provider). Each scoped to a `delegates-compute` commitment.

### Ranked additions (slot into §9 / §10)

| # | Capability | Type | Leverage | Effort | Need | p2p-class |
|---|-----------|------|----------|--------|------|-----------|
| M1 | cgroup MEMORY reader (`/sys/fs/cgroup/memory.max` v2 + v1) | wiring | **H** | S | precondition; the node can't see its ceiling (= §9 #13) | C node-local |
| M2 | Working-set watermark monitor + memory-pressure shed (layer 8) | self-protection | **H** | M | the climb → OOM; defense-in-depth for memory | C node-local |
| M3 | Per-structure memory budgets in `derive()` (gossip/projection/cache/reconcile shares of mem ceiling) | auto-config | **H** | M | bound the structure not the container; laptop floor | C node-local |
| M4 | Working-set-trajectory → elevate (leak signal) | auto-loop | **H** | S | auto-file what the operator found by hand | A + ledger |
| M5 | Memory headroom feeds advertised capacity | flow-control | M | M | bilateral back-off on a memory-pressured node | C + wire |

**Phasing:** M1 + M4 are P0-adjacent (S effort, high leverage — the reader is the gate, the elevate arm makes the climb self-reporting). M2 + M3 ride P1 alongside the `derive()` build. M5 rides P1's bilateral-contract work.

**The stakes (why this is existential, not hygiene):** even the *bounded* case (4GB+ per loaded node) strains `project_hub_optional_floor` (laptop-as-full-participant); the *leak* case is a hard blocker to it. Bounding memory-to-container isn't stability polish — it's a precondition for the household-device vision. And: do not declare a memory incident "healed" on a boot-time reading (the 8Gi "massive headroom" was a boot snapshot; the node had already re-climbed to 3.3GB). Confirm a plateau on a long-lived loaded node.

### Arc-shrink — the substrate-native instrument for the corpus axis (added 2026-06-13)

A parallel thread sealed `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md` (commit 79e9ef506, born-linked to the tiered-quilt design) that diagnoses the climb precisely — it corrects and completes this section:

1. **It is NOT our Rust.** A 4-agent code-hunt across elohim-storage found no byte-path that can produce a GB step-jump (all disk-backed / dead-wired / byte-trickle; three minor unbounded maps fixed as hygiene, none fixes the OOM). The single structure matching the signature is the **embedded conductor's DHT authority-arc working set** (conductor + storage share one cgroup, so container metrics measure both). The "bound our structures" move above stays as hygiene but is NOT the dominant lever.
2. **The dominant lever is `network.target_arc_factor`.** Every node runs at full arc (1.0 — set *nowhere*, so it defaults to full) → per-node RAM ∝ *total corpus*, growing forever on every node. Arc-shrink (`<1.0`) holds a **bounded shard**; as peer count N rises each arc shrinks toward ~1/N while *coverage stays whole across the mesh*. This is the tiered-quilt data-plane sharding applied to the conductor working set — and the precondition for the laptop-is-a-full-participant floor (a lean device holds a shard; it cannot hold a growing whole-corpus arc).

**This is the corpus-axis completion of the four pillars:**
- **Pillar 3 (Auto):** `target_arc_factor` is the corpus-axis Auto knob — `f(mem_ceiling, device_archetype, observed_peer_count)`. The cgroup MEM reader (M1 / §9 #13) is its precondition: you cannot size an arc to a ceiling you can't read. "How much of the DHT should I hold?" *is* the Auto memory budget, expressed as an arc.
- **Pillars 1+2 (negotiation + no-overwhelm, with a COVERAGE invariant):** arc-shrink bounds corpus-memory overwhelm, but carries the *dual* of the anti-self-partition guard — a **coverage invariant**: the sum of arcs across peers must keep the DHT fully covered, so a node may self-protect by shrinking only as far as collective coverage allows. Arc-factor is therefore a *negotiated* quantity (peers coordinate so no keyspace gap opens) — Pillar 1 applied to storage topology.
- **Pillar 4 (deliberate REA actuation):** the spec is emphatic — arc-factor is a resilience↔memory **trade, never a silent flip**. That is exactly the bounded/audited/deliberate REA actuation §5 prescribes (not an admin toggle), with the coverage invariant as its NEVER-violate guard.

**Open question, refined:** the leak-vs-convergence framing resolves to **leak-vs-bounded-large**, the code-hunt having cleared our Rust. Decisive cheap confirm (operator-side, no Pyroscope): a **per-process RSS split** in one `elohim-node` container (`ps -o rss,comm`: the `holochain` child vs the `elohim-storage` parent) — conductor dominates ⇒ arc diagnosis holds. The durable fix is the **arc, not the RAM limit**; stop the bump treadmill and design the per-archetype arc-factor policy aligned with tiered-quilt. Canonical memory: `project_per_node_memory_is_conductor_authority_arc`.
