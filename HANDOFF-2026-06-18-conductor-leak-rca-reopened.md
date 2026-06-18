# HANDOFF — Conductor off-heap leak: RCA REOPENED (zombie-connection hypothesis FALSIFIED)

**Date:** 2026-06-18
**Status:** The alpha conductor off-heap memory leak / OOM-flapping is **NOT fixed**. A complete, verified-deployed fix for the leading hypothesis (tx5 dead-peer "zombie" PeerConnections) **did not cure it in production**. The hypothesis is falsified. A fresh root-cause investigation is needed — **profile, don't reason from source** (source reasoning produced a fix that tested green and held in unit tests but does nothing to the live leak).

> ## ➡️ UPDATE 2026-06-18 (later) — EMPIRICAL REFRAME landed: `.claude/data/conductor-leak-rca-empirical-reframe-2026-06-18.md`
> Starting from live telemetry (not source) per the meta-lesson below, the leak is now characterized as **Go LIVE-HEAP RESIDENT accumulation** — and sharply narrower than §1–§5 below assume:
> - **Discriminators (live):** `anon_mapping_count` FLAT (~5600, oscillating), `largest_anon_bytes` PINNED at 150.9 MB constant, `proc_threads` FLAT (~486), while `class="other"` anon + cgroup `working_set` climb **monotonically to OOM**. → resident growth in a **stable population** of mappings; genuinely unreclaimable (it OOMs, so NOT MADV_FREE non-return); NOT mapping proliferation; NOT a CGo-thread/`m` leak.
> - **Scaling:** leak rate ∝ traffic (anchors ~6× leechers at flat count/threads) → the leak is **per-operation** (per send/op/gossip/receipt), **NOT per-connection-lifecycle** — which is *why* the zombie-teardown fix (a per-connection fix) couldn't cure it.
> - **Shipped instrumentation to finish localizing it:** (a) Rust smaps enrichment — per-VMA size-bucket histogram + top-N growing-VMA dump to Loki, **ships via the normal edge build, no conductor rebuild** (`elohim/elohim-storage/src/{services/system_metrics.rs,metrics.rs,main.rs}`, unit-tested green); (b) `elohim/tx5/crates/tx5-go-pion-sys/pprof_debug.go` — env-gated `net/http/pprof` + `runtime.MemStats` (gofmt+`go vet` clean), the decisive **live-leak-vs-non-return + allocation-site** signal for the next canary.
> - So §2a (zombie) stays falsified; §2c source-refutations and §4's "profile, don't reason from source" still hold; §4's candidate list is now refined by the profile-candidate fan-out recorded in the reframe doc §6. **Start there.**

---

## 1. The leak we're chasing (unchanged, well-characterized)

- **Symptom:** the alpha edgenode `elohim-node` container (image `elohim-storage`, which spawns the holochain conductor as an embedded child) grows **off-heap anonymous mmap** without bound → OOM at ~8 GB → container restart → climb again. This is the "flapping."
- **Gauge:** `elohim_node_conductor_smaps_anon_bytes{class="other"}` (Prometheus, datasource `prometheus`). Rust `[heap]` class is **flat**; the growth is the **`other` (anonymous-mmap) class** → Go/CGo runtime memory, not the Rust heap.
- **Rates (current, 2026-06-18):** busy anchors **matthew / james ~1–2 GB/h** (the "amplifier"); leechers ~0.2 GB/h floor. Genesis pair OOMs roughly hourly.
- **Only Go/CGo in the conductor is `tx5-go-pion`** (the WebRTC transport, CGo-wrapping vendored pion/webrtc v4.1.3 + pion/sctp v1.8.39). So the working assumption remains: the off-heap growth is in the go-pion / pion Go runtime. **Verify this assumption fresh — it was never directly proven, only inferred.**

## 2. What is RULED OUT — do NOT re-investigate these

### 2a. The zombie-PeerConnection hypothesis — FALSIFIED in production
The RCA (`.claude/data/conductor-leak-rca-tx5-gopion-backpressure-2026-06-17.md`) concluded the leak was dead-peer `PeerConnection`s never torn down, because tx5 0.8.1's `tx5-connection/src/webrtc/go_pion.rs` had `Evt::State(_) => ()` (ignored connection-state changes → no teardown of dead/idle peers). Fix = upstream tx5 **#194 + #199** (handle `Evt::State` → drop on Disconnected/Closed/Failed) + holochain **#5719** (receipt-storm amplifier brake).

**This was built, deployed fleet-wide, verified running — and the leak persisted at the same rate.** So connection-lifecycle/teardown is **not** the leak. Evidence below.

### 2b. The delivery mechanism is NOT at fault (ruled out hard)
- The patched binary built from source (`elohim-0.6` fork + tx5 `[patch]` #194/#199) → `harbor…/elohim-edgenode:latest` (che-devworkspaces `jenkins/Jenkinsfile-elohim-edgenode`).
- The production `elohim-storage` build embeds it via `CONDUCTOR_SOURCE_IMAGE` (`elohim/elohim-storage/Dockerfile`) → `elohim-storage:1.0.0-dev-2af2607e`, deployed to all 14 alpha pods ~00:0x UTC.
- **Binary identity verified:** the edgenode image is usr-merged (`/bin -> usr/bin`), and `/bin/holochain` == `/usr/bin/holochain` == our patched build (sha256 `36ddf7ab…`, 53 MB). Storage extracts `/usr/bin/holochain` → `/usr/local/bin/holochain`. Storage spawns `holochain` (clap default, `process_manager.rs` `Command::new`) → resolves via PATH to that patched binary. **The conductor running on alpha is the patched one. No doubt.**

### 2c. Source-level refutations from the original RCA (already done; may revisit empirically)
- R1 "no backpressure" — refuted (64 KB cap, `BufferedAmountLow`, `onBufferReleased` ACK accounting).
- R2 "unbounded recv" — refuted (`try_send`-then-close).
- R3 "unbounded single-channel SCTP retransmit" — refuted (bufferedAmount counts inflight → per-channel 64 KB cap).
These were **source reasoning**; given source reasoning just failed us, treat them as *unconfirmed-by-profiling*, not gospel.

## 3. The evidence that falsified it (the live cure-verdict check, 2026-06-18)

| pod | pre-deploy (stock conductor) | ~2h post-deploy (PATCHED conductor) |
|---|---|---|
| james | 7.45 GB, near OOM | 6.34 GB, **+1.08 GB/h, climbing** |
| matthew | 7.25 GB, near OOM | 6.11 GB, **+1.22 GB/h, climbing** |
| OOM-flaps | continuous | **continuous** (jessica/daniel/gertrude/pete restarted 2× post-deploy) |

Deploy restart dropped the pair to 3.9 / 4.4 GB at 00:15; by 01:52 they were back to 6.1 / 6.3 GB — same leak trajectory, on the verified-patched binary. **Not cured.** (The tx5 unit tests `conn_dropped_on_peer_connection_state_*` pass with the fix / time out without — so the teardown *works*; it just isn't the leak.)

## 4. Fresh investigation — approach (the reorientation)

**Stop reasoning from source. Profile the running Go runtime.** The single biggest lesson: a source-traced root cause, corroborated by the upstream maintainers' own fix (#194) and proven by unit tests, was still wrong about *what the leak is*. Get empirical.

Concrete first moves (roughly in order):
1. **Confirm the `other` anon IS go-pion's Go heap** — not some other CGo/native allocation. Read `/proc/<conductor-pid>/smaps` on a live alpha pod (via Loki: `target=elohim_storage::memory_attribution` log lines, or the storage `/metrics` per-process attribution) and map the growing anon ranges to the go-pion shared library vs. the Rust binary vs. other.
2. **Get a Go heap/allocs profile of the live conductor under load.** Options: does the holochain conductor / kitsune2 / tx5 expose Go pprof or a debug endpoint? If not, build a conductor with `net/http/pprof` wired into the go-pion-sys CGo lib (`elohim/tx5/crates/tx5-go-pion-sys/*.go`) and capture `inuse_space` over time. The growing allocation site is the answer.
3. **Reproduce in isolation with the tx5 throughput bench.** `elohim/tx5/crates/tx5/benches/throughput.rs` drives sustained WebRTC traffic with no cluster. Run it (needs Go 1.24 + the build env in §6) long enough to see anon growth, with Go pprof attached. If it reproduces → profile there (fast loop). If it does NOT → the leak is specific to the kitsune2/holochain integration traffic pattern, not raw tx5.
4. **Distinguish true leak vs. Go-runtime non-return.** The Go runtime can hold freed arenas (RSS up, no logical leak). Test `GODEBUG=madvdontneed=1` / `GOGC` tuning on a canary; if RSS flattens, it's retention, not a leak — a config fix, not a code fix.

Candidate mechanisms to profile-test (NOT source-argue):
- Per-send `GoBuf` retention — `cgo.NewHandle` pins Go objects off-heap; if a Free path is missed under sustained send, it accumulates. (Source said it's freed on Drop — verify under load.)
- pion SCTP/DTLS buffers or the ICE agent state accumulating under high connection *churn* (distinct from idle-zombie teardown, which is what #194 fixed).
- A non-tx5 source entirely — re-confirm step 1 before assuming go-pion.

## 5. The amplifier vs. floor question (still open)
Pre-deploy: floor ~0.2 GB/h (all 14 nodes, even quiet/0-receipt-error terrance) + amplifier ~2 GB/h (high-fanout matthew/james/jessica). #5719 (receipt-storm brake) shipped too; the amplifier did NOT clearly drop, so either #5719 doesn't gate the amplifier or the amplifier isn't receipt-driven. Treat floor and amplifier as possibly-two-leaks until profiling says otherwise.

## 6. Rails that WORK (don't rebuild these)

- **Deploy pipeline (proven end-to-end):** che-dw `elohim-edgenode` job builds the patched conductor → `elohim-edgenode:latest` → `elohim-storage/Dockerfile` `CONDUCTOR_SOURCE_IMAGE` embeds it → edge build → genesis deploy, fleet-wide, "always patched." **Shipping the real fix is one job re-run + one genesis run.** Rollback = revert the `CONDUCTOR_SOURCE_IMAGE` default to `ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom` + push to dev.
- **Build env (documented):** `.claude/data/conductor-leak-deploy-recipe-2026-06-17.md` — Go 1.24 install, `--no-default-features --features sqlite-encrypted,wasmer_sys,backend-go-pion`, `RUSTFLAGS=""`, `RUSTC_WRAPPER=""`, the `/dev/null` sandbox gotcha. The dev-container can't build the storage image (apt `/dev/null`), but the che-dw buildkit job can.
- **Forks:** `ethosengine/tx5@elohim-0.8.1-zombie-fix`, `ethosengine/holochain@elohim-0.6` (+ submodule refs on dev). The local tx5 builds + tests (the teardown tests work in isolation).
- **Observability:** `elohim_node_conductor_smaps_anon_bytes{class=…}` (heap/stack/other), cadvisor `container_memory_*`, `kube_pod_container_status_restarts_total`, Loki `target=elohim_storage::memory_attribution`. `query_prometheus` works from here.

## 7. Knock-on (the original symptom this was meant to fix)
Genesis seed stages **Seed Substrate / Seed Custody Commitments / Seed REA Commitments** fail with `CellDisabled` + 503 catching-up-shed — these are leak symptoms (a conductor that OOM'd mid-seed). They will keep failing until the leak is actually fixed. Re-running genesis won't clear them while the conductor still OOMs.

## 8. Key files
- Falsified RCA: `.claude/data/conductor-leak-rca-tx5-gopion-backpressure-2026-06-17.md` (read for the mechanism detail + the source map; the *conclusion* is wrong).
- Working deploy recipe: `.claude/data/conductor-leak-deploy-recipe-2026-06-17.md`.
- Original handoff (Task-1 go/no-go, upstream survey): `HANDOFF-2026-06-17-upstream-tx5-transport-pin.md`.
- Upstream comments posted (now overclaim — flagged the off-heap/go-pion attribution; the *teardown* part still merits a release but is NOT the fleet cure): holochain #5664, tx5 #196, tx5 #207.
- Storage embed seam: `elohim/elohim-storage/Dockerfile` (`CONDUCTOR_SOURCE_IMAGE`); `src/conductor/process_manager.rs`; `src/main.rs` (`conductor_binary`, `embedded_conductor`).

## 9. Meta-lesson
The whole chain was rigorous *except the part that mattered*: the leak was attributed to a mechanism (idle-zombie teardown) that the fix demonstrably addresses, but that mechanism was not the leak. Unit tests proved the fix does what it claims; they could not prove the claim is the cause. **Only the live cure-check (smaps slope post-deploy) is ground truth.** Start the next loop with profiling and a live canary measurement, and treat any source-only root cause as a hypothesis until the slope flattens.
