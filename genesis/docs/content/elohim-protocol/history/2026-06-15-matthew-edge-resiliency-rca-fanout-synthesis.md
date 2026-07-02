---
title: "Matthew Edge Resiliency — RCA Fanout Synthesis (2026-06-15)"
id: matthew-edge-resiliency-rca-fanout-synthesis
type: history-gotcha
status: noted
tier: history
created: 2026-06-15
topic: [conductor-oom, doorway-watchdog, arc-factor, matthew-edge, rca, alpha]
---

# Matthew Edge Resiliency — RCA Fanout Synthesis (2026-06-15)

Seven-lens fanout + an operator-verified household-topology investigation, reconciled into one decision-grade report. Live evidence captured against ns `elohim-alpha` (Prometheus uid `prometheus`, Loki uid `loki`) ~16:10–18:12Z 2026-06-15. Source claims verified in-tree at write time (see §3 footnote).

---

## 1. Verdict (one paragraph)

**The matthew-edge "flap" is two distinct, only-loosely-coupled failures wearing one alarm, and the lever everyone reached for could never touch either.** (A) The **conductor OOM**: matthew + james are full-DHT-authority-arc anchors (`network.target_arc_factor` defaults to **1** — set in NO deployed config, inherited from the kitsune2 default; only the Tauri mobile build sets 0 at `steward/device/src-tauri/src/lib.rs:139-141`). Full arc means each node tries to hold the *entire* Lamad keyspace, so the conductor working set scales with corpus-held and sawtooths into the **8 GiB** limit and OOMs (live: james **8.46 GB at its cap**, matthew 3.45 GB post-reset and climbing; everyone else 1.4–3.0 GB). The decisive natural experiment: **james authors and endorses ZERO yet is the worst OOM victim AND the top SQLite saturator (~520/sec vs matthew ~290/sec)** — proving the driver is *corpus-held × arc*, not authorship, endorsement, or request volume. (B) The **doorway restart**: matthew's doorway is killed by its own **liveness watchdog** (`:8079`, stale>15s ⇒ wedged; liveness period 10s × failureThreshold 3 = 30s budget) — `last_terminated_reason="Error"` at 75–94 MB working set rules out doorway OOM. Its main tokio runtime parks because it is **co-located on `ethosengine` next to the two memory-climbing conductors**, while the A/B control doorway-alpha-b (adam, on `shem`) reconnect-storms *just as hard* (~2.7k vs ~3.5k/hr) with **0 restarts** — co-location, not `CONDUCTOR_URLS` breadth (identical, both `env: alpha`), is the differentiator. The dominant symptom — `holochain_sqlite … kind=Dht … Database read connection is saturated. Util 225–500%` at ~300/sec, 73% of log volume — is **real but benign on the read path** (`info!`-level queue-depth gauge at `access.rs:188-195`; **zero `DatabaseError::Timeout` lines live**), and it saturates because the DHT read pool is pinned at **8 readers** (`max(2·num_cpus, 8)` with `num_cpus` cgroup-aware → 4-core matthew = 8). The shipped `STORAGE_DB_POOL_SIZE=20` resizes the **diesel `content.db` r2d2 pool** (`db/mod.rs:264`), a different file/crate entirely — **wrong pool**.

---

## 2. Theory Portfolio (ranked, deduped, bold theories preserved)

Plausibility = HIGH/MED/LOW after stress-test. Leverage = all / partial / marginal on the *durable* fix.

| # | Theory | Lens(es) | Plaus. | Leverage | Risk | Deciding experiment |
|---|--------|----------|--------|----------|------|---------------------|
| 1 | **Full authority arc (=1) × corpus-held = conductor OOM sawtooth.** matthew/james hold whole keyspace; RSS ∝ corpus. james (0 authored/endorsed) OOMs worst → driver is holding, not authoring. | authority-arc, corpus-shape, conductor-sqlite (T2), contrarian (T4b), household-topology (AUTHORITATIVE) | HIGH | **all** (the OOM) | high to act (genesis anchor can't shrink) | `target_arc_factor: 0` on a **non-anchor** loaded node (jessica/james-as-test); memory slope flattens ⇒ bounded working set, not leak |
| 2 | **DHT read pool pinned at 8 (`db_max_readers` unset, cgroup-CPU-derived).** Explains 73%-of-log saturation; explains why CPU bumps 1→2→4 were placebos (pool stays 8 until ≥5 cores). | conductor-sqlite (T1), k8s-substrate (T1), contrarian (T1), authority-arc (T2) | HIGH | **partial** (saturation + Loki cost; likely NOT the flap) | low, instant rollback | Set `db_max_readers: 16` on james (worst, 2-core) in conductor-config; saturation collapses ⇒ confirmed |
| 3 | **Doorway liveness watchdog self-kills under conductor-co-location runtime park.** Restart cadence regular/accelerating (3–8 min); A/B split (adam 0 restarts, same image, same URLs) explained only by node placement. | contrarian (T3), k8s-substrate (T4), session-reconnect (implicit) | HIGH | **partial→all** (the doorway restart) | low (probe) / med (masks a real wedge) | Add `doorway_watchdog_wedged_total`; then raise liveness `failureThreshold` 3→12 OR add podAntiAffinity off ethosengine — restarts stop ⇒ confirmed |
| 4 | **`db_sync_strategy: Resilient` fsync/checkpoint stalls the read pool (I/O-bound, not CPU).** Low CFS throttle (~4–7%) + huge read-queue depth is I/O/lock-wait, not CPU. | conductor-sqlite (T3), k8s-substrate (T3), read-amplification (T1 diesel variant) | MED (as explanation), LOW (as action) | marginal | **high** (Fast on authority DB risks corruption) | Correlate node disk fsync p99 vs saturation rate (read-only). DO NOT flip sync on a genesis DB |
| 5 | **Saturation is benign noise; the flap is doorway-only (watchdog + co-location).** `info!`-level; 0 read timeouts live; conductor 0 restarts on current gen. | contrarian (T5), read-amplification (T2 corollary) | MED-HIGH | marginal on flap, real on log cost | trivial (log-level) | `hc_db_read_timeout_total` counter — if it stays 0, saturation is noise (adjudicates T2 vs T5) |
| 6 | **Doorway session multiplication (~28-way fan-out) DDoSes the conductor's readers.** Per-conductor pools (2 workers × N humans) + app + admin + subscriber. | session-reconnect (T2), authority-arc (T4), k8s-substrate (T5), corpus-shape (T3) | LOW (differentiator falsified) | marginal | low (one env flip) | `DOORWAY_PER_CONDUCTOR_WORKERS=0` on matthew. Predicts NO change (adam fans out identically, 0 restarts) |
| 7 | **Reconnect↔read self-amplifying loop** (reconnect → warm/re-subscribe → corpus re-read → saturate → conductor drops session → reconnect). | session-reconnect (T1), read-amplification (T3), corpus-shape (T3) | LOW (falsified by timing) | marginal | low | Saturation is **flat & reconnect-independent** (james ~520/sec steady w/o restart storm) ⇒ already falsified |
| 8 | **`send_authenticate` returns `Ok(())` without verifying the auth ack** (worker path; subscriber path does verify). Silent auth-reject possible. | session-reconnect (T3), read-amplification (T3) | MED (real defect, prob. secondary) | partial | low (correctness fix) | Session-duration histogram: mass <100ms ⇒ auth-reject; mass 1–9s ⇒ busy-conductor close |
| 9 | **No app-ws keepalive on the worker-pool path → idle sessions reaped at ~10s conductor idle timeout** (STABLE_SESSION_THRESHOLD=10s is suspiciously equal). | session-reconnect (T4) | MED | partial | very low (mirror subscriber ping) | Session-lifetime modal ≈10s ⇒ idle-reap; add `ping_interval` to PoolConfig |
| 10 | **Hot-anchor fan-out** (scenario tag anchor 2,690 links) makes projections O(corpus). | corpus-shape (T1) | MED | partial | med (reseed to shard) | Per-anchor `links_returned` histogram on a zero-traffic remote node; if it saturates w/o requests ⇒ gossip not projection |
| 11 | **matthew-as-universal-endorser validation hot spot** (endorses 1,893). | corpus-shape (T4) | LOW (refuted) | marginal | low | **james endorses 0 yet saturates 2× matthew** ⇒ already refuted |
| 12 | **conductorGroup-tiered over-seeding** (cg=0 gets whole 3,369-node corpus incl. james-a-minor-child). Potential energy for T1. | corpus-shape (T2) | HIGH | partial (seed-scope) | med (reseed) | Trim a cg=0 node's seed scope; memory floor drops |
| 13 | **BOLD: stop fighting it — matthew = 16 GiB + arc-1, new `device-genesis-anchor` archetype.** Anchor can't shrink (binary lever + coverage gate); size for the job. | contrarian (T4b), authority-arc | HIGH (as decision) | all (the OOM) | med (capacity: 2× 8 GiB anchors already crowd ethosengine) | corpus↔RSS scatter to size deterministically; separate anchors onto distinct nodes |
| 14 | **BOLD: arc-actuator-as-trap.** The dormant auto-policy can only no-op (Auto never shrinks the climbers) or actuate arc=0 on a bootstrap anchor (coverage gate is bootstrap-blind) → OOM→partition. | authority-arc (T5) | MED (latent, not today's cause) | high on NOT-making-it-worse | high (process) | Add `is_bootstrap_anchor` refusal to `coverage_admits`; test `coverage_refuses_leecher_on_bootstrap_anchor` |
| 15 | **BOLD: centralized-authoring topology is the root** — one agent owning a corpus-sized source chain is an anti-pattern. Sharpened: the killer is corpus-*holding* (T1), not chain length (james has a tiny chain, OOMs hardest). | corpus-shape (T5) | HIGH (direction), MISTARGETED (mechanism) | all (durable, reframed as seed-scope+arc+placement) | highest cost / lowest reversibility | Source-chain-length-per-agent gauge ⇒ james tiny, OOMs ⇒ holding not chain |

---

## 3. Tunables to Surface

The "wrong pool" is resolved decisively: the saturating pool is **`holochain_sqlite`'s per-DB read pool**, sized by **`ConductorConfig.db_max_readers`** (`holochain_conductor_api-0.7.0-dev.21/src/config/conductor.rs:118-119`, default `calculate_default_db_max_readers(num_cpus::get())` = `max(2·cpus, 8)` at `:151-156`; test `:1188` confirms ≤4 CPU → 8). **The knob is REAL and reachable** — settable as a top-level key in `conductor-config.yaml` — contradicting the "must fork the vendored crate" framing. The shipped `STORAGE_DB_POOL_SIZE` controls a *different* pool (the diesel r2d2 pool over `content.db`, `elohim/elohim-storage/src/db/mod.rs:264`).

| Env/config field | Where wired | Default + range | What it ACTUALLY controls | tune→document→report |
|---|---|---|---|---|
| **`db_max_readers`** (THE right knob) | `elohim/holochain/edgenode/conductor-config.yaml` (top-level); placeholder in `_edgenode-consolidated.template.yaml`; sed in `elohim/holochain/Jenkinsfile` deploy path, derived `max(2·ceil(cpuLimitCores), 8)` | unset⇒8 on a 4-core cgroup; set 16, range [8, 64] | `holochain_sqlite` per-DB read semaphore (`pool.rs:71`, `access.rs:319` short=4/long=4 split) — **the saturating pool** | Rust `#[test]` asserting rendered YAML carries `db_max_readers` ≥ `incoming_request_concurrency_limit + 3`; a2o `@regression` `node-resource-tunables.feature` "DHT read concurrency matches CPU budget not host"; shape dim `dbMaxReaders` vs `cpuLimitCores` |
| **`incoming_request_concurrency_limit`** | same as above (`conductor.rs:135-136`, default `db_max_readers−3`) | unset⇒5 on 4-core; set 13, range [4,128] | inbound authority-response admission. **Must move WITH `db_max_readers`** (readers ≥ concurrency+3) or authority responses drop silently | `#[test]` invariant `readers ≥ concurrency+3`; same a2o feature; shape dim `incomingConcurrency` |
| **`network.target_arc_factor`** (THE memory lever) | `conductor-config.yaml` `network:` block; written today only by `arc_actuator.rs:191` `render_conductor_arc_factor` (commitment-gated) | unset⇒1 (full); actuatable set **{0,1} ONLY** (`arc_actuator.rs:33-34` `FACTOR_LEECHER=0`/`FACTOR_FULL=1`) | conductor DHT authority working set (the OOM driver). **Fractional BLOCKED upstream** (kitsune2 0.3.2/0.4.1, holochain_p2p 0.6.0 hard-clamp {0,1}; `arc_actuator.rs:8-13`) | `#[test]` rendered config contains requested factor + `coverage_admits` refuses anchor→leecher; a2o "genesis node holds bounded arc working set"; shape dims `arc_factor`, `corpus_nodes_held` |
| `db_sync_strategy` | `conductor-config.yaml` (`conductor.rs:108`) | unset⇒`Resilient`; **leave at default on DHT DB** | SQLite synchronous level. `Fast` only lets *wipeable* DBs go `Off` (`pool.rs:40-50`) — would NOT blanket-disable DHT fsync; corruption risk on genesis content | record as "knob intentionally NOT tuned + why" (a shape-report first-class row) |
| `STORAGE_DB_POOL_SIZE` (already shipped — WRONG pool) | `elohim/elohim-storage/src/db/mod.rs:264` | 20 | diesel r2d2 pool over `content.db` — **not** the saturating pool | document as mis-aimed prior fix; keep for warm_stream/diesel cost, not the saturation |
| `CACHE_STREAM_BATCH_PACE_MS` + `CACHE_STREAM_BATCH_SIZE` | `elohim/elohim-storage/src/cache_stream.rs:34` | pace 0 (rec 5ms), batch 500 [50–2000] | producer-side pacing of cache SSE (diesel) — the missing twin of shipped `WARMUP_PACE_MS`. Hygiene only; does NOT touch `kind=Dht` | `#[test]` ≤ceil(N/batch) diesel acquisitions; a2o regression; shape dim next to `WARMUP_PACE_MS` |
| `DOORWAY_LIVENESS_WEDGE_THRESHOLD_MS` + liveness `failureThreshold` | `doorway/.../server/http.rs:984` + `genesis/orchestrator/manifests/doorway/alpha.yaml:329-335` | wedge 15000; failureThreshold 3 (30s budget); raise to 12 (120s) | doorway self-kill threshold — the proximate restart trigger | `#[test]` on `watchdog_liveness_response` (200 below / 503 above); a2o "rides out N-sec conductor stall"; shape dim `watchdog503SecondsBeforeRestart` |
| `DOORWAY_PER_CONDUCTOR_WORKERS` | `doorway/.../main.rs:418` (literal `2`) | 2 [0–4]; **0 disables fan-out** | per-conductor app-ws session count. Diagnostic, not differentiator | `#[test]` total sessions = `app+admin+pcw·len(URLS)+1`; a2o "bounded sessions per edge"; shape dim `doorwayConductorSessionsTotal` |
| `DOORWAY_POOL_PING_INTERVAL_MS` | `doorway/.../worker/conductor.rs` (add ping branch mirroring `subscriber.rs:616`) | 0(off)→rec 5000 [0–30000] | worker-pool app-ws keepalive (currently absent → idle-reap suspect) | `#[test]` pool sends ping within interval; a2o "idle session survives idle timeout"; shape dim `poolKeepaliveMs` |
| `edgenodeMemoryLimit` + new `device-genesis-anchor` archetype | `genesis/orchestrator/data/deployments.json` | matthew/james 8Gi → eval 16Gi by corpus model | conductor RAM ceiling — only honest lever if arc can't shrink | `expected_full_arc_working_set_bytes(corpus_docs)` model `#[test]`; a2o "anchor RSS ≥ 2× peak"; shape dim `conductorRssPeakVsLimitRatio` |
| `holochain_sqlite::db::access=warn` (EnvFilter) | conductor tracing config (storage `main.rs` EnvFilter build) | info→warn | suppresses the 73%-of-volume saturation `info!` spam (Loki relief); keep `Timeout` at error | shape dim `dhtReadUtilPct` from a gauge, not log-count |

---

## 4. Log-Levels / Instrumentation to Add

The current logs cannot decide the RCA because three pivotal signals are missing or thrown away:

1. **`hc_db_read_timeout_total{kind,dna}` counter** — file: the `DatabaseError::Timeout` branch in `holochain_sqlite-0.7.0-dev.17/src/db/access.rs:160-169` (or, absent a crate patch, an *alert on the appearance* of that already-existing error line). **Why:** the saturation log only says the queue is deep; it never says a read *failed*. This single counter adjudicates Theory 2 (pool too small, harmful) vs Theory 5 (benign noise). Live value is currently **0** — strongly favoring "benign."

2. **Per-process RSS + thread + RssAnon/RssFile split** — file: `elohim/elohim-storage/src/conductor/process_manager.rs` (it owns the conductor `Child`; read `/proc/<child_pid>/status` every 60s) emitting `elohim_node_child_rss_bytes{proc="holochain"|"elohim-storage"}`, `elohim_node_thread_count{proc}`, and `RssAnon` vs `RssFile`. **Why:** the consolidated `elohim-node` container fuses conductor child + storage parent into one cgroup, so `container_memory_working_set_bytes` cannot attribute the 8 GB. This gates the operator's open **leak-vs-bounded** question (anon-heap ⇒ DHT/leak; file-mmap ⇒ SQLite page cache), and thread-count-tracks-RSS ⇒ blocking-pool exhaustion. **The single highest-value cheapest add.**

3. **Doorway watchdog + close-reason instrumentation** — file: `doorway/.../server/http.rs:~1036` (`doorway_watchdog_wedged_total` + `warn!(heartbeat_age_ms, threshold_ms, "watchdog WEDGED")` at the 503 branch) AND `doorway/.../worker/conductor.rs:533` (the worker path *does* log the frame; promote to a counter split `doorway_conductor_reconnect{reason="tcp_refused|accept_then_drop|close_frame"}` + WS close *code*; the **subscriber** path at `subscriber.rs:636` drops the frame — fix it). Also surface `session_len` (already computed at `conductor.rs:311`, thrown away) as a `doorway_conductor_session_duration_seconds` histogram. **Why:** today we cannot distinguish conductor-absent (T3 OOM-restart) vs conductor-shedding-while-up (admission) vs auth-reject (T8) — the close reason logs as `None` on the subscriber path and is un-aggregated on the worker path.

4. **Conductor boot-time pool-size log** — file: `process_manager.rs` (read `/sys/fs/cgroup/cpu.max`, log `db_max_readers=N num_cpus=M cpu_quota_cores=Q`). **Why:** the conductor logs *saturation* 300×/sec but never logs *what it sized the pool to* — one line proves-or-kills the host-vs-cgroup question without a source dive.

5. **Corpus↔RSS + DHT-authority-set gauges** — file: `elohim/elohim-storage` metrics (`conductor_corpus_docs` × `conductor_rss_peak_bytes`, `dht_authority_set_entry_count`, `sqlite_page_cache_bytes`). **Why:** ends the RAM-bump-chain guesswork (fits GB-per-1000-docs) and is the operator's own asked-for working-set-vs-leak split.

6. **Saturation-span caller tag** — file: the `holochain_sqlite` Dht-pool span (`access.rs:191`), tag with caller-class (zome-call vs gossip vs validation) + `target_anchor`/`links_returned`. **Why:** the plateau (flat, request-independent, present on zero-traffic remotes) implies gossip/arc-serving, not request-time projection — but we can't prove the *driver* of the reads today. Splits "doorway loop drives it" from "arc gossip drives it."

---

## 5. Staged Experiment Plan (safe → risky)

Each marked **[repo]** (we can do), **[operator]** (kubectl/limits/reschedule — manifest-surface only), or **[upstream]** (Holochain/kitsune2).

| # | Change | Expected signal if theory right | Rollback | Surface |
|---|--------|--------------------------------|----------|---------|
| 1 | Add `hc_db_read_timeout_total` alert (or alert on the existing `DatabaseError::Timeout` line) | Stays 0 ⇒ saturation benign (T5); >0 ⇒ pool harmful (T2) | n/a (read-only) | **[repo]** alert / log-level |
| 2 | Add per-process RSS/thread/anon-file split sampler in `process_manager.rs` | Conductor child dominates climb, anon-heavy, threads flat ⇒ bounded arc working set (T1, not leak) | remove sampler | **[repo]** |
| 3 | Add doorway `watchdog_wedged_total` + close-reason split + session-duration histogram; fix subscriber close-frame log | WEDGED precedes each restart ⇒ T3; close reason classifies T3/T8 | remove | **[repo]** |
| 4 | Conductor boot log of effective `db_max_readers` + `num_cpus` + cpu quota | Prints `db_max_readers=8 num_cpus=24 cpu_quota_cores=4` ⇒ host-vs-cgroup pin confirmed | remove | **[repo]** |
| 5 | Set `holochain_sqlite::db::access=warn` EnvFilter | 73% of log volume disappears; Loki 502 storms ease; read path unchanged | restore info | **[repo]** config |
| 6 | `DOORWAY_PER_CONDUCTOR_WORKERS=0` on matthew | Predicts **NO** restart change (adam fans out identically, 0 restarts) — falsifies T6 cheaply | env revert | **[repo]** manifest env |
| 7 | Set `db_max_readers: 16` + `incoming_request_concurrency_limit: 13` on **james** (worst saturator, 2-core, non-bootstrap) | Saturation rate collapses; **flap likely UNchanged** ⇒ pool was a red herring, redirect to T1/T3 | delete 2 YAML lines, redeploy (coordinator hot-swap class, no DNA re-key) | **[repo]** conductor-config + Jenkins sed |
| 8 | Raise doorway liveness `failureThreshold` 3→12 (30s→120s) on matthew, after step 3 confirms WEDGED-before-restart | Restarts stop, pod stays serving (heartbeat age comes back down) ⇒ recoverable park, T3 | restore 3 | **[operator]** probe budget in `doorway/alpha.yaml` |
| 9 | Add `podAntiAffinity` separating matthew/james conductors AND the matthew doorway off `ethosengine` | Doorway restarts stop without touching probes; conductor OOM-crowding eases | remove block, reconcile | **[operator]** manifest (reconciled by pipeline; never kubectl) |
| 10 | **BOLD:** `target_arc_factor: 0` (leecher) on a **non-anchor** loaded household node (NOT matthew/adam/genesis pair) | Memory slope flattens ⇒ bounded working set confirmed, arc is the lever; coverage gate must still admit (`observed_n−1 ≥ r_floor`) | flip to 1, restart (~60s, no re-key) | **[repo]** actuate path / config; **operator** restart |
| 11 | **BOLD:** matthew → 16 GiB + `device-genesis-anchor` archetype (only if anchors can't be spread) | OOM cycle ends; sawtooth tops below new ceiling | revert limit | **[operator]** deployments.json + capacity call |
| 12 | Harden `coverage_admits` with `is_bootstrap_anchor` refusal before any auto-arc rollout | Test `coverage_refuses_leecher_on_bootstrap_anchor` passes; auto-policy can't island the genesis pair | additive gate, revert PR | **[repo]** |
| 13 | **BOLD/UPSTREAM:** push fractional `target_arc_factor` (arc sharding) to kitsune2 so anchors hold a *share* | A genesis anchor at arc 0.3 holds ~30% keyspace, RSS bounded, mesh still covered | upstream revert | **[upstream]** kitsune2/holochain_p2p |

**Do first:** steps 1–4 (read-only instrumentation, can't make it worse) collapse most of the four-way ambiguity in one observation window. Then step 7 as the bold diagnostic that is *expected to fail to fix the flap* — high information value, single-node blast radius, instant rollback.

---

## 6. Resilience Card + caleb/daniel

**These are NOT downstream of the OOM/saturation root** — they are independent and must not be conflated with it (per the AUTHORITATIVE household-topology findings).

- **The DARK resilience card** (Stewarding 0 · Commitment-backed 0 · Diversity 0% · Placement gaps 1) is a **structural empty-JOIN**, not a missing-data problem. `humans.agent_pub_key` is **NULL in production by design** (the seeder sends `agentPubKey: null`; only `household_id` is ever backfilled in `reconcile/controller.rs:1107-1114`). The snapshot's INNER JOINs are `humans::agent_pub_key.eq(rea_commitments::provider)` (`household_resilience.rs:172-174`) and `humans::agent_pub_key.eq(shard_locations::peer_id)` (`:74`, `:448-450`) — both are structurally empty regardless of how many active commitments exist. **Unblock = populate `humans.agent_pub_key`** by extending `reconcile/controller.rs::on_membership_projected` to stamp it from the DHT human projection it already reads. Compounding key-vocabulary bug: the runtime provide-loop writes `provider = self_cid` (a libp2p `12D3Koo…` peer id) while the join wants the holochain `uhCAk…` key — only the seeder path uses `uhCAk…` on both sides; reconcile both. **This is display-honesty only — it moves ZERO bytes of stored content.**

- **caleb + daniel** edge-deploy failure (#1081 UNSTABLE; StatefulSets never Ready, 5 restarts each) are **newly-added humans on shem** and are a **separate deploy/scheduling problem** (not the matthew memory plane). They share only the word "resilience" with the card. Triage independently.

---

## 7. Open Questions / Risks

- **Leak vs bounded (still formally unconfirmed):** james-authors-nothing-yet-OOMs is strong circumstantial proof of *bounded working set*, but only the per-process RSS/anon-file split (§4.2) settles it. 3 known minor leaks are hygiene-only.
- **Capacity for the 16 GiB path:** `ethosengine` already hosts matthew + james + jessica + doorway. Two 16 GiB anchors may not fit — the better-supported move is **spreading the anchors onto distinct nodes** (which *also* fixes the doorway co-location park), not doubling RAM on a crowded box.
- **The arc-actuator trap:** the dormant auto-policy can only no-op or actuate arc=0 on a bootstrap anchor (coverage gate is bootstrap-blind). Do NOT authorize auto-arc until `is_bootstrap_anchor` refusal lands. Manually shrinking matthew/adam (the genesis pair) risks a DHT partition.
- **`db_max_readers` interacts with memory:** more readers ⇒ more SQLite page cache ⇒ faster climb to 8 GiB. The pool fix must NOT ship to an anchor alone; pair with arc/RAM.
- **Numbers in the original evidence packet drifted:** "0 conductor restarts" was a snapshot (the conductor restarts on a ~2.5–3h OOM cycle); matthew saturation is ~280–290/sec not ~158; **james is the real outlier (~520/sec)**. Qualitative conclusions survive and are stronger; anyone tuning to the old numbers will mis-size.
- **Auth-verify change is behavior-changing:** making `send_authenticate` await the ack could convert a tolerated session into a hard refusal if the alpha app interface is auth-optional — test on the b-edge first.

---

## 8. Strategic Frame — The Two Planes

The incident's deepest lesson: **household resilience is being paid for in the wrong plane.**

- **Plane A — Conductor DHT authority arc** (`network.target_arc_factor`). All-or-nothing per node on this substrate (`{0,1}`, fractional hard-clamped upstream). Governs the OOM **and** the SQLite read saturation. This is the plane that is crashing matthew/james. It replicates the whole keyspace to every full node — which is *why* memory ∝ corpus. Sharded/bounded participation here is **upstream-blocked**.
- **Plane B — Dwelling-hub RS-sharded STORAGE plane** (RS-encoded blobs, `mishpat_projection.rs:166-407`; spec `2026-05-28-mutual-storage-replication-dwelling-hub-design.md`). **This is bounded by design** — each node holds a *share*, not everything. `replicates-content` / `replicates-commons` commitments live here. A commitment lights the resilience card but changes **zero bytes** of conductor DHT state.

**What "household resilience" should mean operationally, given fractional arc is upstream-blocked:** the bounded "each node holds a SHARE" property the operator actually wants **already exists** — on Plane B (dwelling-hub RS sharding), not Plane A. The durable architecture is to make household durability a **Plane-B** property (storage-plane RS shards across household nodes) and let Plane A run **leecher (arc=0) on the lighter household nodes** with a small number of full anchors, rather than every household node attempting full DHT authority.

**Three durable paths (weighed):**
- **(a) Push fractional arc upstream to kitsune2** — the *correct* long-term fix (anchors hold a share, RSS bounded, mesh covered), but it is an upstream feature, not a flag; slowest to land.
- **(b) Move household resilience onto the dwelling-hub storage plane** — uses the design that *already exists*, bounded by construction, decouples durability from the OOMing authority plane; the most architecturally coherent near-term path. Requires lighting Plane B (commitments + `humans.agent_pub_key`).
- **(c) Raise matthew RAM + set lighter household nodes to leecher** — immediately available with the `{0,1}` lever + RAM, but a band-aid: it sizes for the anti-pattern rather than fixing it, and capacity-constrains on the shared node.

**THREE distinct work items — joined ONLY by the word "replication," do not conflate:**
1. **Relieve matthew = arc-shrink.** Fractional is upstream-blocked; available now is `{0,1}` actuation (leecher on non-anchors) + the tactical doorway/SQLite/RAM fixes. *Plane A.*
2. **Light the resilience card = populate `humans.agent_pub_key`.** Repo-only, display-honesty, **moves zero bytes**. *Snapshot JOIN.*
3. **caleb/daniel deploy failures.** A separate scheduling/StatefulSet problem. *Deploy plane.*

---

*§3 footnote — verified in-tree 2026-06-15 at write time:* `db_max_readers` knob + `max(2·cpus,8)` default + ≤4-CPU→8 test (`holochain_conductor_api-0.7.0-dev.21/src/config/conductor.rs:118-156,1188`); deployed `conductor-config.yaml` sets neither `db_max_readers` nor `target_arc_factor`; arc lever binary `{0,1}` (`arc_actuator.rs:33-34`) + coverage-gate leecher refusal (`:152-168`); resilience JOINs on `humans.agent_pub_key` (`household_resilience.rs:74,172-174,448-450`); `arc_policy::derive` (`arc_policy.rs:138`) wired only to `GET /api/v1/status/arc-policy` (`http.rs:1003`) + `POST /admin/arc-policy/actuate` (`http.rs:1009`); `STORAGE_DB_POOL_SIZE` is the diesel pool (`db/mod.rs:264`); doorway watchdog/backoff constants (`worker/conductor.rs:42,50,320`; `main.rs:418` per-conductor `worker_count: 2`).
