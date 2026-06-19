# HANDOFF 2026-06-17 — Conductor anon-heap leak hunt (the "brake")

> ## ➡️ CURED — 2026-06-19 — H4 (glibc allocator) was the closest hypothesis; this trail down-ranked it wrongly
> Archived investigation trail. The leak was glibc-malloc arena retention in the conductor child — i.e.
> essentially H4's family, which this doc down-ranked ("intercept not slope"). The cure was not a tuning of
> MALLOC_ARENA_MAX but a full allocator swap glibc→jemalloc (whose decay/munmap returns what glibc pinned in
> chained sub-heaps). FLAT ~2.1–2.9GB past the OOM cadence; DNA hash unchanged. H1/H2/H3 (receipt/peer-URL/
> kitsune2-buffer) were not the cause. The smaps instrument + attribution methodology here were RIGHT.
> Truth: .claude/data/conductor-leak-jemalloc-cure-verdict-2026-06-19.md · conductor-leak-rca-native-heap-reframe-2026-06-18.md


**For:** the next agent/operator picking up the conductor-OOM brake.
**Status of the question:** the *attribution* is settled (verdict CONFIRMED). What's open is the *mechanism* inside the conductor — this handoff scopes that hunt and hands you ranked, toolkit-testable hypotheses.
**The instruments you'll use shipped this session** (design-decision toolkit P0/P1, on dev): the `/metrics` Prometheus surface + the per-process / smaps memory-attribution sampler. See the SHARED CONTEXT in the dispatching prompt and `genesis/docs/superpowers/plans/2026-06-17-design-decision-toolkit-plan.md` (P2 doorway leg is the remainder).

---

## 1. The confirmed verdict (one paragraph)

The alpha conductor OOM is an **anonymous-heap leak in the Holochain conductor *child* process** — not the elohim-storage parent (dead-flat ~101 MB), not the SQLite page cache (cgroup `file` ~242–287 MB, flat), not corpus, and **arc-independent** (jessica at `arc=0` holds ~no keyspace yet leaks the same shape; james at `arc=0` is the *worst*, ~6.5 GB). Per-process split on jessica (sampler commit `fa9985436`, image `1.0.0-dev-ea494df1`): conductor child carries ~97% of the anon **and all the growth** (3.37→3.45 GB monotonic), 391 threads (**flat**); storage parent flat at ~101 MB. cgroup reconciliation holds: `anon` 3.55 GB ≈ conductor 3.45 + storage 0.10. The brake is therefore a **conductor/kitsune2 heap-leak hunt** — NOT more RAM (8→16 Gi just lengthens the OOM interval — the slope, not the intercept, is the problem), NOT a SQLite `cache_size` cap, NOT arc sharding, NOT our storage code. Full record + evidence: `genesis/data/timeline/backlog/conductor-memory-attribution-verdict.md`. The live lead: jessica's conductor spams `holochain::core::workflow::validation_receipt_workflow: send_validation_receipts could not find url for peer` (`HolochainP2pError`).

---

## 2. The substrate we're hunting — versions, spawn, config (verified in-tree)

### 2.1 The version picture (RECONCILED — read this, it corrects a stale note)

Two version namespaces that were conflated in the verdict's one-liner. They are NOT the same component:

| Component | Version | Source (verified) |
|---|---|---|
| **Conductor BINARY** (the leaking child) | **Holochain `0.6.0-dev.28`** | `elohim/holochain/edgenode/README.md:153`; base image `ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom` (`elohim/holochain/edgenode/Dockerfile:10` — note `hc0.6.0` in the tag) |
| Conductor's holochain source pin | `holochain-0.6.0` @ rev `a6d4e805a0971ccbc0dcb3f3ed6a9e2fac980a3b` | holonix flake.lock `elohim/holochain/dna/elohim/flake.lock:53-71`; DNA flake `holonix?ref=main-0.6` (`elohim/holochain/dna/elohim/flake.nix:5`) |
| Conductor's **kitsune2** | **`v0.3.2`** @ rev `22de6e42100aa960d05f5f30427a236ad922bd80` | holonix flake.lock `kitsune2.original.ref = "v0.3.2"` (`flake.lock:97-114`) |
| elohim-storage CLIENT crates (talks TO conductor; does NOT compile it) | `holochain_client 0.9.0-dev.22`, `holochain_conductor_api 0.7.0-dev.21`, `holochain_types 0.7.0-dev.21`, `kitsune2_api/core/bootstrap_client 0.4.1` | `elohim/elohim-storage/Cargo.lock` (lines 3436/3461/3667/4887+) |

**The reconciliation:** the verdict doc and **RCA §3** say "holochain 0.6 / kitsune2 0.3.2 / holochain_p2p 0.6.0" (the verdict's own one-liner says only "holochain_p2p"; the `0.6.0` is RCA §3's clamp note). That is **correct for the conductor binary** (holonix-built, kitsune2 v0.3.2 confirmed from the flake.lock ref). It is NOT the storage lockfile — which has **no `holochain_p2p` crate at all** and has kitsune2 **0.4.1** on the *client* side. When you hunt, **the conductor's kitsune2 v0.3.2 is the code you're chasing**, not the storage crate's 0.4.1. (The RCA §3 footnote `holochain_conductor_api-0.7.0-dev.21/.../conductor.rs` paths are the *client* crate's vendored copy of the config struct — fine for reading the `db_max_readers`/`target_arc_factor` knobs, but it is not the conductor's runtime allocator code.)

> **One open verification (do before writing any decision-record that pins a kitsune2 patch):** confirm the conductor binary's allocator (§3.1) and, if you need an exact kitsune2 *patch*, read holochain's own `Cargo.lock` at rev `a6d4e805` (the binary's tree), since `v0.3.2` is the flake ref, not necessarily the resolved transitive patch. The git revs above are the primary source; trust them over any prose.

### 2.2 How the conductor is spawned (THE injection seam — this is what makes profiling feasible)

`elohim/elohim-storage/src/conductor/process_manager.rs::start()` (~line 64): elohim-storage spawns the conductor as a tokio child —
```
Command::new(&self.conductor_binary)
    .arg("--config-path").arg(&self.config_path).arg("--piped")
    .env("HOLOCHAIN_DATA_DIR", &self.data_dir)
    .kill_on_drop(true).spawn()
```
We **spawn but do not compile** the binary. The `.env()` / `Command` builder at ~line 68 is the **universal injection point**: allocator env vars (`MALLOC_ARENA_MAX`, `MALLOC_CONF`) and even a launch wrapper (`heaptrack <binary>`, `LD_PRELOAD=…`) go *there*, child-scoped, **with no holochain recompile and no storage rebuild of the binary**. `child_pid()` (line 170) is what the sampler reads each tick to attribute the child's working set. `restart()` (line 188) re-spawns with the (possibly rewritten) config — the only way to apply a config change (there is no runtime API).

Two flavors of injection, cheapest first:
- **Pod-manifest env var** (`genesis/manifests/humans/<human>.yaml` / the edgenode manifest) → inherited by the child → **zero storage rebuild**. Best for `MALLOC_ARENA_MAX`/`MALLOC_CONF`.
- **`.env()` in `process_manager.rs`** → child-only scoping, needs a storage image rebuild. Use when you must scope strictly to the child or wrap the launch.

### 2.3 Conductor config (where knobs live)

`elohim/holochain/edgenode/conductor-config.yaml`: bootstrap `https://doorway.elohim.host/bootstrap`, signal `wss://signal.doorway.elohim.host`, `enable_mdns: false`, `enable_relaying: true`, WebRTC via tx5/kitsune2, Lair in-proc keystore, `data_root_path: /var/local/lib/holochain`. **Neither `db_max_readers` nor `network.target_arc_factor` is set** (RCA §3 footnote, verified) — both inherit defaults (readers `max(2·cpus,8)`=8 on 4-core; arc=1 full). Per-env deploy rewrites these via sed in `elohim/holochain/Jenkinsfile`.

### 2.4 Where the "could not find url for peer" string lives

It is **upstream holochain core** — `holochain::core::workflow::validation_receipt_workflow`, raised as `HolochainP2pError`. **It is NOT in our source**: grep across `elohim/`, `doorway/`, `steward/` finds zero occurrences (the only in-tree `holochain` dirs are our DNA/edgenode/worktrees, not vendored conductor source). It is observable **only via the conductor's logs** (Loki / pod stderr — `process_manager.rs` inherits stdout/stderr). So the peer-URL failure is visible as a log-rate signal, and the leak is visible as the smaps gauges; you correlate the two, you do not read the conductor's source in-tree.

---

## 3. Profiling a process we don't compile — options ranked by feasibility

**Step 0 (gates everything below): which allocator does the conductor binary link?** `MALLOC_ARENA_MAX` is glibc-only; `MALLOC_CONF` prof is jemalloc-only; both are **inert on musl**. holo-host edgenode images are historically Debian/glibc, but **do not assert it** — check on a live pod: `ldd $(which holochain)` or `grep -iE 'malloc|jemalloc' /proc/<child_pid>/maps`. This one fact decides (c)-vs-(d) and whether H4 can even register. Write the answer into the first decision-record.

| Rank | Option | Cost / risk | Notes |
|---|---|---|---|
| **(a) FIRST — already shipped** | `elohim_node_conductor_smaps_anon_bytes{class=heap\|stack\|other}` + `elohim_node_conductor_anon_mapping_count` + `elohim_node_conductor_largest_anon_bytes` | **zero — read it now** | The per-mapping smaps breakdown is live on alpha. **Localizes the leak before any deploy.** See §3.1 for the (important) class-interpretation caveat. Set by `system_metrics::parse_smaps_anon` → `metrics::set_conductor_smaps` (`elohim/elohim-storage/src/services/system_metrics.rs:443-500`, `metrics.rs:79-107,205`). |
| **(d) cheapest mutation** | `MALLOC_ARENA_MAX=2` env on the conductor spawn (glibc only — Step 0) | **cheap, reversible, repo-side** | 391 threads × per-thread glibc arenas can bloat anon via fragmentation. Pod-manifest env (§2.2) → no rebuild. Watch `{class=other}` + `anon_mapping_count` drop. **Caveat below — likely hits intercept not slope.** |
| **(c)** | `MALLOC_CONF=prof:true,prof_leak:true,…` env (jemalloc only — Step 0) | cheap *if* it links jemalloc | If the binary links jemalloc, this gives a real allocation profile via env alone — no recompile. If it links glibc malloc, inert; skip to (b). |
| **(b) heaviest, highest-fidelity** | attach `heaptrack`/bpf to the child pid at runtime, **or** wrap the spawn (`heaptrack <binary>`) on ONE canary | heavier; do on a canary, not all of alpha | The spawn-wrap variant (§2.2) is clean: no GDB-attach to a prod pid, no recompile — wrap the `Command` on jessica/james only. Highest signal; reserve for after (a)/(d) narrow the class. |

### 3.1 CRITICAL caveat on reading the smaps classes (don't misread the gauge)

`classify_mapping` (`system_metrics.rs:443`) buckets by the smaps pathname field: `[heap]`→Heap, `[stack…]`→Stack, `[anon…]`/no-name→**OtherAnon**, else File. **On post-Linux-4.5 kernels, pthread stacks are unlabeled anonymous mappings → they fall in `class=other`, NOT `stack`.** So `other` conflates: secondary glibc arenas + pthread stacks + jemalloc arenas + plain `mmap` anon. Disambiguate with the count gauges you already ship:
- **`anon_mapping_count` rising** ⇒ more *mappings* (new arenas / new connections) ⇒ points at H3/H4.
- **`largest_anon_bytes` rising with flat count** ⇒ one structure growing in place ⇒ points at H1/H3 (receipt/gossip buffer), not arena multiplication.
- **The 391 threads are FLAT** (verdict) ⇒ arena count and stack count are stable ⇒ the *slope* is almost certainly **not** thread/arena multiplication. This **down-ranks H4 as a slope explanation** (see §4).

---

## 4. Ranked hypotheses — each with a toolkit-driven test

Ranking reflects §3.1: flat threads make arena/connection *multiplication* an unlikely slope driver, so the **in-place growth** hypotheses (H1, H3-buffer) lead.

### H1 — validation-receipt accumulation (LEAD; settles the slope)
**Claim:** the conductor accumulates per-receipt / per-failed-resolution state that is never freed; the `send_validation_receipts could not find url for peer` spam is the symptom of a workflow that keeps retrying and growing.
**Test (toolkit):** correlate the conductor **anon slope** (`rate` of `elohim_node_conductor_smaps_anon_bytes` or `elohim_node_proc_rss_bytes{proc="holochain",kind="anon"}`) against the **validation-receipt error rate** in Loki (`count_over_time({…} |= "send_validation_receipts could not find url for peer" [5m])`) on jessica/james. **If they track**, the leak is receipt/resolution-driven. Expect `largest_anon_bytes` to rise with flat `anon_mapping_count`.
**Watch:** Loki was 502-storming at verdict time (adam 26 GB/day spam) — confirm Loki is healthy first, else the error-rate series is untrustworthy; fall back to the smaps slope alone.

### H2 — peer-URL-resolution retry-state growth (bootstrap-islanding angle; HYPOTHESIS, not established)
**Claim:** failed peer-URL resolution leaves growing retry/pending state. **In-tree cause to consider:** the kitsune2 bootstrap store is **per-pod in-memory and never reconciled** — the genesis pair PUT agent-info into *disjoint* doorway stores (matthew→doorway-A, adam→doorway-B) that never sync, wiped fresh each restart (`genesis/docs/superpowers/plans/2026-06-14-federation-bootstrap-plan.md` §"The mechanism is pinpointed"). Unresolvable peers → the receipt workflow can't find a URL → retry state.
**Caveat (do not overstate):** jessica is a **household node, not the genesis pair** the F-BOOTSTRAP plan addresses. Bootstrap islanding is *one* in-tree cause of unresolvable peer URLs, **not** the proven cause of jessica's receipt spam.
**Test (toolkit):** does the anon slope **flatten once peers resolve**? Two ways to create the natural experiment: (1) observe whether slope correlates with `peerCount` (doorway `/health`) climbing; (2) land the `MongoK2Store` shared-bootstrap fix (federation-bootstrap plan) on a **canary** and watch whether the receipt-error rate AND the anon slope both fall. A flatten-on-resolve is the discriminator that promotes H2 from hypothesis to cause.

### H3 — kitsune2 gossip / per-connection state accumulation
**Claim:** per-connection or per-gossip-round buffers in kitsune2 v0.3.2 accumulate. The 391 threads hint at per-task structure.
**Test (toolkit):** `anon_mapping_count` trend (rising-with-connections ⇒ per-connection mapping growth) and `largest_anon_bytes` (one gossip buffer growing). Correlate against connection/peer churn if a kitsune2 metric is exposed. **But:** flat threads (§3.1) argue against per-connection *multiplication* as the slope — so test the **buffer-growth** flavor (one structure in place), not the connection-count flavor, first.

### H4 — glibc malloc-arena fragmentation (CHEAPEST mutation; likely intercept not slope)
**Claim:** 391 threads × per-thread glibc arenas inflate anon via fragmentation/retained free chunks.
**Test (toolkit):** Step 0 confirms glibc, then set `MALLOC_ARENA_MAX=2` on the conductor spawn (pod-manifest env, §2.2). Watch `elohim_node_conductor_smaps_anon_bytes{class=other}` + `anon_mapping_count` drop.
**Honest expectation (§3.1):** threads are FLAT, so arena *count* is stable — H4 attacks the **baseline/fragmentation intercept**, probably not the unbounded slope. **A clean negative (slope unchanged after the cap) is itself high-information** — it redirects decisively to H1/H3 and is recorded as a falsification. That's why it's still worth running first: cheap, reversible, and informative either way.

---

## 5. The decisive next experiment (recommended)

Run **two cheap things in parallel on jessica + james** (non-anchor leechers — safe; never the matthew/adam genesis pair, whose arc/restart can partition the DHT per RCA §7):

1. **H1 correlation (the slope-settler) — FREE, do first, no deploy.** Plot `rate(elohim_node_conductor_smaps_anon_bytes[15m])` (and `elohim_node_proc_rss_bytes{proc="holochain",kind="anon"}`) against the Loki receipt-error rate.
   **Expected signal if H1:** the two series track; `largest_anon_bytes` climbs with flat `anon_mapping_count`.
   **Rollback:** none — read-only.

2. **H4 arena cap (the cheapest mitigation) — after Step 0 confirms glibc.** `MALLOC_ARENA_MAX=2` via pod-manifest env on jessica.
   **Expected signal if H4:** `{class=other}` and `anon_mapping_count` step down; OOM interval lengthens. **If slope unchanged:** H4 falsified → the leak is in-place growth → all weight to H1/H3.
   **Rollback:** remove the env key, redeploy (coordinator-hot-swap class — no DNA re-key, ~60s).

**Why this pair:** H1 is the high-information experiment (it tells you *whether* the slope is receipt-driven, the verdict's own lead); H4 is the cheapest possible mutation and is informative on **both** outcomes. Together they cost one Loki query + one env flip and either localize the leak or hand you a clean falsification. Reserve heaptrack/jemalloc-prof (§3 b/c) for after these narrow the class.

---

## 6. Upstream vs. our-config — the fork in the road

The leak is in code **we don't own** (holochain 0.6.0-dev.28 / kitsune2 v0.3.2). Once a hypothesis lands, decide the fix's home:

- **First, search upstream.** Check `holochain/holochain` and `holochain/kitsune2` issues/PRs for `validation_receipt`, `send_validation_receipts`, `peer url` / `could not find url`, and `memory`/`leak`/`unbounded` around the 0.6 / kitsune2-0.3.2 line. The receipt-workflow + peer-URL-resolution shape is a plausible known upstream class.
  - **If upstream:** the path is **pin → patch → report**. We consume the conductor via the holonix flake (`elohim/holochain/dna/elohim/flake.lock`, holochain rev `a6d4e805`, kitsune2 rev `22de6e42`); bumping/patching means moving those locked revs (or a `cargo` patch on the conductor build) and re-publishing the edgenode artifact. File/track the upstream issue; record the rev we pin to and why.
- **If the peer-URL failures are caused by OUR bootstrap/transport** (H2 confirmed — islanding starves resolution): the fix is **repo-side**, the federation-bootstrap plan's `MongoK2Store` shared store (`doorway/doorway-service/src/bootstrap/{k2,k2_mongo,store}.rs`; `genesis/orchestrator/manifests/doorway/{alpha,alpha-b}.yaml`). That removes the *trigger* even if the conductor's accumulation-on-failure is an upstream latent bug — fewer failed resolutions ⇒ less retry state to leak.

The likely outcome is **both**: report the unbounded-accumulation-on-failure upstream (defense in depth) *and* fix our bootstrap islanding so resolution succeeds (removes the trigger here and now).

---

## 7. Every hypothesis tested becomes a decision-record

Per `genesis/data/timeline/backlog/decision-record-discipline.md` (toolkit P3): each lever turned or hypothesis confirmed/**falsified** gets one durable record so it isn't re-litigated. **One record per H** in §4, using the template's exact frontmatter (`title` = the conclusion not the question; `status: confirmed|falsified|...`; fields `Lever/hypothesis · Instrument · Measured effect · Verdict · Brake/action · Lineage`). `Instrument` must name the **exact metric/Loki query** so the conclusion is reproducible; `Verdict` must state what it rules **OUT** (the anti-re-litigation field). Add each to the "Records to date" table in the discipline doc. Lineage cites: this handoff, the verdict doc, and `2026-06-17-design-decision-toolkit-plan.md`.

---

## 8. Guardrails (do not skip)

- **Cluster is operator-owned.** Repo manifests (`genesis/manifests/`, `genesis/orchestrator/manifests/`) are the surface; **never `kubectl`**. Read live state via Prometheus/Loki MCP or in-repo manifests.
- **Experiment only on jessica/james** (non-anchor leechers). **Never** flip arc/restart/env on the matthew or adam genesis pair — that risks a DHT partition (RCA §7, the arc-actuator trap; `coverage_admits` is bootstrap-blind).
- **Build envs:** doorway = native (`RUSTFLAGS=""`); elohim-storage = WASM (`RUSTFLAGS='--cfg getrandom_backend="custom"'`). `RUSTC_WRAPPER=""`, /tmp target dirs, plain cargo, gate with `--lib` (pre-existing reds on `--all-targets`).
- **Commit-only on the shift branch; the operator pushes.**

## 9. Key files & docs (all absolute-from-repo-root)

- Verdict: `genesis/data/timeline/backlog/conductor-memory-attribution-verdict.md`
- RCA (§3 tunables, §4.3 doorway instruments): `.claude/data/matthew-edge-resiliency-rca-fanout-2026-06-15.md`
- Decision-record discipline: `genesis/data/timeline/backlog/decision-record-discipline.md`
- Toolkit plan (P2 = the remaining doorway/heap-profiling leg): `genesis/docs/superpowers/plans/2026-06-17-design-decision-toolkit-plan.md`
- Bootstrap islanding (H2/H6 fix home): `genesis/docs/superpowers/plans/2026-06-14-federation-bootstrap-plan.md`
- Spawn seam: `elohim/elohim-storage/src/conductor/process_manager.rs` (`start()` ~64, `.env()` ~68, `child_pid()` 170, `restart()` 188)
- smaps gauges: `elohim/elohim-storage/src/services/system_metrics.rs:443-500`; `elohim/elohim-storage/src/metrics.rs:79-107,205`
- Conductor config: `elohim/holochain/edgenode/conductor-config.yaml`; base image `elohim/holochain/edgenode/Dockerfile:10`; version block `elohim/holochain/edgenode/README.md:153`
- Conductor source pins: `elohim/holochain/dna/elohim/flake.lock` (holochain `:53-71`, kitsune2 `:97-114`); `flake.nix:5`
