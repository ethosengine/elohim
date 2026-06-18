# Conductor leak — EMPIRICAL RCA reframe (Go live-heap resident growth), 2026-06-18

Picks up `HANDOFF-2026-06-18-conductor-leak-rca-reopened.md` (zombie-PeerConnection
hypothesis falsified in production). Per that handoff's meta-lesson — **profile, don't
reason from source** — this pass started from live telemetry, not source. The result is
a sharply narrower target than the handoff had, plus shipped instrumentation to localize
it the rest of the way.

> **TL;DR:** the leak is **Go LIVE-HEAP resident accumulation** in the go-pion/pion
> runtime — genuinely-resident (OOM-driving), growing within a **stable population** of
> anon mappings, at a rate **proportional to traffic** (per-operation, not per-connection).
> It is NOT mapping proliferation, NOT a CGo-thread/m leak, and NOT Go-runtime non-return.

---

## 1. The discriminators (live alpha Prometheus + cadvisor, datasource `prometheus`)

The previous instrument set already exported `anon_mapping_count` and `largest_anon_bytes`
gauges — **the handoff never read them.** Reading them (plus thread count and cadvisor
working_set) splits the question the scalar `other` anon couldn't:

| Signal | matthew (amplifier) | What it means |
|---|---|---|
| `smaps_anon{class="other"}` | climbing ~1.2 GB/h (6.56 GB, near OOM) | the leak — resident anon grows |
| `anon_mapping_count` | **flat**, oscillating ~5300–5900 (no trend within an instance) | NOT mapping proliferation |
| `largest_anon_bytes` | **pinned at exactly 158 265 344 B = 150.9 MB**, constant all window | the leak is NOT one lone growing mapping |
| `proc_threads{proc=holochain}` | **flat**, 479 → 486 over 2.8 h (plateaued) | NOT a CGo-thread / Go-`m` leak |
| cadvisor `working_set_bytes` | **monotonic 1.3 → 7.35 GB**, prior instance OOM'd at 8.57 GB | genuinely resident / **unreclaimable** |

**count flat + largest flat + threads flat + total climbing** ⇒ resident anon accumulates
**within a stable population of mid-sized mappings** = the Go-heap-arena-resident-growth
signature. `working_set` (the OOM-driving figure) climbing to the limit and OOM-flapping
means the memory is **unreclaimable** — if it were `MADV_FREE` / Go-runtime non-return, the
kernel would reclaim under cgroup pressure and plateau instead of OOMing. So this is a
**true live-heap leak**, not lazy non-return. (Confirm with the memstats instrument in §4.)

## 2. Two populations — the leak scales with TRAFFIC, not connection lifecycle

Snapshot across the 14 `elohim-*-alpha-0` pods:

| | mappings | threads | other-anon | bytes/mapping | leak rate |
|---|---|---|---|---|---|
| high-fanout anchors (matthew/james/jessica) | ~5500 | ~450–490 | 2.7–6.9 GB | 0.5–1.2 MB | ~1–1.2 GB/h |
| low-fanout (adam/terrance/leechers) | ~800–1780 | ~115–140 | 2–4.5 GB | 2.4–3.2 MB | ~0.2 GB/h |

Mapping-count and thread-count track **traffic level** (working-set of connections /
goroutines), and they're stable per node. The **leak** (bytes-up at flat count) rides on
top and is present on every node, ~6× faster on the busy anchors. So the leak is
**per-operation** — per send / per DHT op / per gossip / per validation-receipt — retained
in the live heap. This rules out the per-connection-lifecycle class (which the deployed-and-
falsified zombie fix already addressed) and **re-frames the search**: find a per-operation
Go allocation in the pion/go-pion path that accumulates in the live heap at steady
connection count.

PromQL used (representative):
```
elohim_node_conductor_smaps_anon_bytes{class="other"}
elohim_node_conductor_anon_mapping_count
elohim_node_conductor_largest_anon_bytes{pod="elohim-matthew-alpha-0"}
elohim_node_proc_threads{proc="holochain",pod=~"elohim-(matthew|james)-alpha-0"}
container_memory_working_set_bytes{pod="elohim-matthew-alpha-0",container="elohim-node"}
```

## 3. Ruled out (do not re-investigate)
- **Mapping proliferation** — count is flat.
- **CGo-thread / Go-`m` leak** — thread count is flat.
- **`MADV_FREE` / Go-runtime non-return** — `working_set` climbs to OOM (unreclaimable). (Belt-and-suspenders: the memstats canary in §4 settles it directly.)
- **Dead-peer zombie PeerConnections** — tx5 #194/#199 built, deployed fleet-wide, binary-verified, leak persisted unchanged (the handoff's falsification).

## 4. Shipped this session — the instrumentation that localizes the rest

### 4a. Rust smaps enrichment (ships via the normal edge pipeline — NO conductor rebuild)
`elohim/elohim-storage/src/services/system_metrics.rs` + `metrics.rs` + the sampler in
`main.rs`. One smaps read per 60 s now also produces:
- **`elohim_node_conductor_anon_bucket_{bytes,count}{bucket}`** — per-mapping size-bucket
  histogram (`0-64k … 256m+`). SHAPE: which size band the growth lands in (Go-arena vs
  thread-stack vs small per-op buffer). A band whose `bytes` climb at flat `count` is arena fill.
- **Loki `scope="smaps_hist"`** — the histogram as a one-line summary.
- **Loki `scope="smaps_growth"`** — top-8 anon VMAs by **resident-anon growth since the
  prior sample** (address + size + delta + nearest-file neighbor + NEW flag). This is the
  localizer: at flat count/largest, these are the exact ranges accumulating the leak; diff
  them across samples to see whether the growth clusters (one allocator region) or is
  diffuse (many arenas). Reset on conductor restart (addresses are then meaningless).

Pure parse path (`parse_smaps_vmas` → `SmapsAnonBreakdown::from_vmas`, `anon_size_histogram`,
`top_growing_vmas`, `fmt_vma_deltas`) is unit-tested; the legacy `parse_smaps_anon` and its
test are preserved (delegates to the new path). **This is the must-ship: it requires no Go/nix
and no conductor rebuild — just an elohim-storage edge build.**

### 4b. Go memstats + pprof (operator-gated — needs the Go/nix conductor build + a canary)
`elohim/tx5/crates/tx5-go-pion-sys/pprof_debug.go` (new file; gofmt-clean, `go vet`-clean
against the vendored pion package with Go 1.24). Both knobs OFF unless env-set, stdlib-only
(go.mod/vendor untouched):
- **`TX5_GO_PPROF_ADDR=127.0.0.1:6060`** → serves `net/http/pprof`. `go tool pprof -base`
  two `/debug/pprof/heap` samples an hour apart ⇒ the `inuse_space` growth **names the
  leaking call stack directly.** This is the answer-finder.
- **`TX5_GO_MEMSTATS_SECS=30`** → one `runtime.MemStats` line to stderr (→ pod log → Loki),
  no scrape infra. **The live-leak-vs-non-return verdict:** `heap_inuse` climbing ⇒ true
  live-heap leak; `heap_inuse` flat while `heap_idle`/`heap_released` climb ⇒ runtime
  non-return (a GODEBUG/GOGC fix). `live_objs` (Mallocs−Frees) climbing corroborates a leak.

## 5. Runbook for the operator (the path to the named site)
1. **Cheapest first:** ship 4a via an elohim-storage edge build → read `smaps_growth` /
   `smaps_hist` on a busy anchor (matthew/james). Localizes the growing band + ranges with
   zero conductor rebuild.
2. **Decisive:** build a conductor with 4b (the existing patched-conductor pipeline — see
   `conductor-leak-deploy-recipe-2026-06-17.md`), deploy a canary anchor with
   `TX5_GO_MEMSTATS_SECS=30` + `TX5_GO_PPROF_ADDR=127.0.0.1:6060`. memstats settles
   leak-vs-non-return within an hour; `pprof -base` names the allocation site.
3. **Sanity (free):** a canary with `GODEBUG=madvdontneed=1` — if RSS flattens, it WAS
   non-return (a config fix). The §1 working_set evidence predicts it will NOT flatten.
4. **Repro (fast loop, off-cluster):** `elohim/tx5/crates/tx5/benches/throughput.rs` under
   sustained send with pprof attached. If anon grows there → profile in the fast loop; if
   not → the leak needs the kitsune2 traffic pattern (the per-op driver the anchors do 6× more).

## 6. Profile-candidates (from a 5-layer source fan-out — to PROFILE, not declared causes)
Five independent readers (go-pion-sys CGo, pion/sctp ×2 angles, pion/ice+dtls+stun,
pion/webrtc+kitsune2 driver) ranked per-operation live-heap sites that fit the §1–§2
signature. **These are hypotheses to confirm/refute with the §4b heap profile — not source-
declared causes** (the meta-lesson). Strongest convergence across readers first:

**The per-op driver (why anchors leak 6×):** kitsune2 **op-fetch via `CoreFetch`**, triggered
by the gossip new-ops exchange. `respond.rs:178` queues one `(op_id, peer_url)` per new op;
`core_fetch.rs:309 outgoing_request_task()` issues **one `transport.send_module` per op-id**
→ `transport_tx5/src/lib.rs:336 ep.send(... data.to_vec())` → CGo `NewBuffer` → `CallDataChanSend`
→ `stream.WriteSCTP`. A ~480-peer full-arc anchor does dozens–hundreds of these per round ×
up to 10 concurrent rounds = thousands of SCTP payloads/min; a leecher does a fraction. The
leak rides this send count → the ~6× ratio. **This is the operation to instrument.**

**C1 — SCTP send-path retention (TOP; flagged by 3 of 5 readers).**
- `pion/sctp/pending_queue.go:24-30` `pendingBaseQueue.pop()` does `q.queue = q.queue[1:]` — a
  textbook Go **head-slice leak**: the backing array is never compacted, so it accumulates
  capacity proportional to **cumulative messages ever sent**, reclaimed only on association
  teardown. On Holochain's single long-lived gossip association this grows **monotonically** —
  the cleanest fit to count-flat/total-up.
- `chunkPayloadData.userData []byte` held in the inflight/pending queue **until SACK**; at high
  fanout the aggregate unacked window ∝ send-rate × RTT × peer-count (per-op, steady conn count).
- pprof `inuse_space` tell: `runtime.growslice` ← `sctp.(*pendingBaseQueue).push`, and/or
  `chunkPayloadData`/`userData []byte` dominating live bytes under `pion/sctp` send frames.

**C2 — cgo.Handle table pinning, esp. the nil-event-handler window (the unifying mechanism).**
Every `NewBuffer` (`datachannel.go:79` on inbound msg; `peerconnection.go:136` on ICE cand)
calls `cgo.NewHandle`, pinning the Go object until `.Delete()`. The Rust `GoBuf::Drop`
(`go_buf.rs:138`) frees it on the happy path — **but `main.go:189` drops the event entirely
when `globalEventReg.event_cb == nil`** (any (re)registration / reconnect window), leaking the
buffer handle permanently. This is the code's own `// TODO!!! MEMORY LEAK`. Traffic/churn-
scaled, invisible to mapping & thread count.
- pprof tell: `main.(*Buffer)` / `main.NewBuffer` accumulating; **discriminator** — a cgo.Handle
  leak keeps growing the Handle-table even during idle gaps between bursts (the memstats
  `live_objs` from §4b shows it; pure SCTP retention would track send bursts).

**C3 — ICE task-loop STUN backlog (secondary).** `ice/candidate_base.go:278-282` allocates a
fresh `*stun.Message{Raw: make([]byte,n)}` per inbound STUN datagram, dispatched into the
**single-threaded** ICE task loop; under fanout the loop backlogs and pins ~1–2 KB/message ∝
inbound-rate × loop-latency. pprof tell: `make([]byte)` under `handleInboundPacket → loop.Run`.

**C4 — lower-fit / likely-not-monotonic (rule out, don't chase):** DTLS `handshakeCache` never
cleared (`dtls/handshake_cache.go:37-48`) — per-session FIXED, only grows on renegotiation;
ICE `pendingBindingRequests` capacity ratchet (`agent.go`) — bounded; SCTP `markAsAcked` struct
shells — cwnd-bounded, self-limiting.

**Profile order (matches §5):** with §4b's pprof on a canary, take two `inuse_space` samples an
hour apart and `-base` them. If the top frame is `pion/sctp …growslice/pushNoCheck` → C1; if
`main.NewBuffer` with a climbing Handle/`live_objs` count even at idle → C2; if
`stun.Message`/`handleInboundPacket` → C3. The profile decides — this list only orders the bets.

_(The workflow's telemetry-correlation agent did not return a leak-rate-vs-traffic table this
run; the per-op driver above + the §1–§2 solo telemetry already establish the scaling and name
the operation. A follow-up correlation pass could quantify the send-count↔slope coefficient.)_

## 7. Key files / forks (unchanged, proven)
- Falsified RCA + source map: `.claude/data/conductor-leak-rca-tx5-gopion-backpressure-2026-06-17.md`
- Deploy recipe + build env: `.claude/data/conductor-leak-deploy-recipe-2026-06-17.md`
- Instrumentation: `elohim/elohim-storage/src/services/system_metrics.rs`, `src/metrics.rs`, `src/main.rs`; `elohim/tx5/crates/tx5-go-pion-sys/pprof_debug.go`
- Forks: `ethosengine/tx5@elohim-0.8.1-zombie-fix`, `ethosengine/holochain@elohim-0.6`
- Memory: `project_storage_metrics_surface_and_leak_verdict`
