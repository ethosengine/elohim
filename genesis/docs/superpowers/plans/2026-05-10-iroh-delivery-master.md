# Iroh Delivery Master Plan

> **For agentic workers:** This is the coordinating plan for the iroh integration cutover. It maps inter-plan dependencies, sequences execution waves, and surfaces Decisions Required + Discovery Required items that block sub-plan execution. Sub-plans use the writing-plans format with checkbox steps.
>
> Use `superpowers:subagent-driven-development` to dispatch implementation agents per sub-plan, with the wave order below.

**Goal:** Land all post-Phase-11 cutover gates so iroh becomes a fully-wired transport for production traffic, with EPR Announce, EPR-atom caller identity, recovery, and discovery all flowing through iroh ALPNs without n0-only chokepoints.

**Architecture:** Six sub-plans + this coordinator. Phase 12 (peer_transport_manifest) is foundational; HTTP/blob, gossip, seeder, pkarr, and recovery layer on top. Soaks #6–9 and rollback drill #11 are infra/docs and live in this master.

**Tech Stack:** Rust (elohim-storage, doorway), Diesel migrations, iroh 0.92 + iroh-blobs 0.94 + iroh-gossip 0.92, pkarr 3.10, ts-rs codegen, JSON Schema (schema-first), Cucumber (a2o features).

---

## P2P Design Gate — entity classification (master coordinator)

This document is a coordinator that introduces **zero new storage schemas, zero new HTTP routes, zero new wire formats, and zero new DHT entry types** of its own. Every artifact referenced in this master is owned and classified by exactly one sub-plan; this section declares the source-of-truth pointers so the line-pattern audit has explicit context.

| Artifact mentioned in this master | Owning sub-plan | Category | Source of truth |
|---|---|---|---|
| `peer_transport_manifest` table + Diesel schema | Plan 1 | C — operational projection | `2026-05-10-iroh-phase12-peer-transport-manifest.md` §"P2P Design Gate"; rebuildable from observed identity-handshake arrivals on both stacks |
| `peer-transport-manifest.schema.json` | Plan 1 | C — view schema for the same projection | Same as above |
| `PUT /blob/{hash}` dual-write semantics | Plan 3 | Pre-existing route (Category C operational projection); Plan 3 only changes server-side handler logic | `2026-05-10-iroh-seeder-dual-write.md` §"P2P Design Gate" |
| `GET /blob/{hash}` dual-format read | Plan 2 | Pre-existing route; Plan 2 only changes the backend selection inside the handler | `2026-05-10-iroh-http-blob-graduation.md` §"P2P Design Gate" |
| `/status` JSON shape (`blobs` field) | Plan 2 | C — operator diagnostic JSON, not a typed View | Same as above |
| `GET /pkarr/{public_key}` + `PUT /pkarr/{public_key}` | Plan 5 | C — pkarr.org-compatible relay; signed packets self-validate via embedded Ed25519 | `2026-05-10-iroh-pkarr-resolver.md` §"P2P Design Gate" |
| `discovery-resolvers.schema.json` (federation manifest extension) | Plan 5 | C — federation-manifest fragment, operator-declared | Same as above |
| Gossip topic publish sites (12 catalogued) | Plan 4 | All pre-existing wire types; Plan 4 only adds a second transport with byte-identical payloads | `2026-05-10-iroh-gossip-dual-publish.md` §"Source-of-truth declaration" |
| Recovery cucumber scenarios + step defs | Plan 6 | Test artifacts only; no protocol-level entities introduced | `2026-05-10-iroh-recovery-e2e.md` §"P2P design gate — entity classification" |

**No new DNA entry types.** Lamad DNA capacity stays at ~73/~100; Mishpat at 11/~100.

**Wave-4 work in this master** (gates #6–9, #11) introduces only:
- Jenkins pipeline stages (operational, no entities)
- Operator runbooks (docs)
- A `bench_blob_stress_10k.rs` test harness (test-only, no entities)
- Modifications to the alpha-cluster Kubernetes manifest (operational deploy config, not a protocol entity)

---

## Sub-plan portfolio

| # | Plan | File | Tasks | EPR-critical | Notes |
|---|------|------|-------|---|---|
| 1 | Phase 12 peer_transport_manifest | `2026-05-10-iroh-phase12-peer-transport-manifest.md` | 13 | ✅ unblocks EPR-atom caller identity | Public API consumed by 2, 4, 6 |
| 2 | HTTP /blob dual-format (gate #2) | `2026-05-10-iroh-http-blob-graduation.md` | 7 | — | Blocks on Plan 1 at exec time |
| 3 | Seeder dual-write (gate #3) | `2026-05-10-iroh-seeder-dual-write.md` | 9 | — | Blocks on Plan 2 Task 5 at exec time |
| 4 | Gossip dual-publish (gate #4) | `2026-05-10-iroh-gossip-dual-publish.md` | 9 | ✅ unblocks EPR Announce identity-binding | Blocks on Plan 1 |
| 5 | pkarr resolver (gate #10) | `2026-05-10-iroh-pkarr-resolver.md` | 11 | ✅ unblocks EPR Announce final form | Standalone; pkarr 3.10 already in lockfile |
| 6 | Recovery e2e (gate #5) | `2026-05-10-iroh-recovery-e2e.md` | 10 | — | Blocks on Plans 1 + 4 |

**Total scoped tasks:** 59 across 6 plans. Plus this master's gate #6–9 + #11 work below.

## Inter-plan dependency graph

```
                  ┌─────────────────────────────┐
                  │ Plan 1: peer_transport_     │
                  │ manifest (13 tasks)          │
                  └─┬───────────────┬───────────┘
                    │               │
            ┌───────┘               └────────────┐
            ▼                                    ▼
   ┌─────────────────────┐              ┌─────────────────────┐
   │ Plan 2: HTTP /blob  │              │ Plan 4: Gossip dual-│
   │ dual-format (7)     │              │ publish (9)         │
   └────────┬────────────┘              └─────┬───────────────┘
            │                                 │
            │ Task 5 →                        │
            ▼                                 ▼
   ┌─────────────────────┐              ┌─────────────────────┐
   │ Plan 3: Seeder dual-│              │ Plan 6: Recovery    │
   │ write (9)           │              │ e2e (10)            │
   └─────────────────────┘              └─────────────────────┘

   ┌─────────────────────┐
   │ Plan 5: pkarr       │  ← Standalone, parallel with all
   │ resolver (11)       │
   └─────────────────────┘
```

## Execution waves

**Wave 1 — Foundation + Independent (parallel):**
- Plan 1 — peer_transport_manifest (13 tasks)
- Plan 5 — pkarr resolver (11 tasks)
- (Plan 3's Task 1 — schema-first work — can also start)

**Wave 2 — Manifest consumers (parallel, after Wave 1):**
- Plan 2 — HTTP /blob dual-format (7 tasks)
- Plan 4 — Gossip dual-publish (9 tasks)

**Wave 3 — Cross-plan integrators (parallel, after Wave 2):**
- Plan 3 — Seeder remaining tasks (Tasks 2-9, after Plan 2 Task 5 lands)
- Plan 6 — Recovery e2e (10 tasks)

**Wave 4 — Soaks + rollback (this master):**
- Gates #6–9 (CI parity, alpha-cluster, latency stress, consumer-grade)
- Gate #11 (rollback drill playbook)

## Decisions Required (resolve before Wave 1 dispatch)

### D1 — Plane enum string encoding (Plan 1)
The `Plane` enum is serialized into JSON arrays inside `peer_transport_manifest.libp2p_supports_json` / `iroh_supports_json` and into the schema enum. Plan 1 recommends **kebab-case** strings: `"blob"`, `"gossip"`, `"sync"`, `"epr"`, `"epr-atom"`, `"shard"`, `"view-fed"`, `"identity-handshake"`, `"trust"`. This matches the ALPN naming conventions (`/elohim/epr-atom/2.0.0`).

**Recommendation:** Accept kebab-case. Action: confirm in this doc, no code change needed.

### D2 — `/status` JSON shape change (Plan 2)
Plan 2 Task 5 changes the doorway/operator-visible `/status` response field `blobs` from `int` (total count) to `{ total, iroh_served, libp2p_served }`. This is operator-facing diagnostic JSON, not a typed View. No TypeScript clients consume it.

**Recommendation:** Accept the shape change. It's additive in spirit; total count is preserved as `total`.

### D3 — Dual-write approach (Plan 3)
Plan 3 chose approach (a) — server-side dual-write derives BLAKE3 from validated bytes on a single `PUT /blob/{hash}`. Single byte transfer, atomic dual-write, zero doorway changes.

**Recommendation:** Accept (a). Approach (b) — separate POSTs — is rejected because it doubles network bytes for ZIPs (1-50MB).

### D4 — `self_peer_id` source on `HttpServer` (Plan 3)
Plan 3 BLOCKED #2: Plan 3 needs the daemon's own peer_id to record itself as the producer in `peer_blob_inventory`. Two options:
- **(a) Builder method** `HttpServer::with_self_peer_id(...)` — explicit
- **(b) Query `self_transport_manifest`** — uses Plan 1's manifest as source of truth

**Recommendation:** (b). Plan 1 already establishes `self_transport_manifest` as the canonical source for self-identity at runtime. Avoids a new builder field that would drift from the manifest.

### D5 — pkarr packet content-type (Plan 5)
Plan 5 chose `application/pkarr.org-relays+octet-stream` for the relay endpoint, matching pkarr.org's spec. Lets iroh's stock `PkarrPublisher`/`PkarrResolver` interoperate with no custom client code.

**Recommendation:** Accept. Standards interop > custom contract.

## Discovery Required (Plan 6)

Plan 6 surfaced 3 items that must be acknowledged before recovery e2e tests run, but **none redesign the protocol**:

1. **Share-bytes custody is metadata-only on the DHT.** `KeyStewardship.key_shard_holders` carries agent IDs only; actual Shamir shares have no peer-to-peer custody-transfer protocol in elohim-storage today. Plan 6 stubs share-bytes as opaque blobs over the existing blob plane and relies on the social-attestation half (`HumanityWitness` via `submit_intimate_witness`) for the gate #5 cross-stack assertion.
   - **Action:** Acknowledge. The full share-custody protocol is a separate epic. Gate #5 only requires shares "traverse whichever transport profile each peer supports" — Plan 6 exercises gossip + blob-fetch dual-stack, which satisfies the wording.

2. **No existing two-stack recovery integration test.** `recovery_m4.rs` is single-conductor + `#[ignore]`. Plan 6 creates the libp2p-only baseline alongside the cross-stack tests.
   - **Action:** Already in plan; no decision needed.

3. **No per-share transport observability today.** Plan 6 adds one `tracing::debug!(target = "recovery::transport", ...)` line at the recipient side.
   - **Action:** Already in plan; no decision needed.

## Wave 4 — Soaks and rollback drill (this master)

These gates are infra/docs; they don't fit the per-plan TDD shape and live here.

### Gate #6 — CI parity soak (one week, zero divergence)

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile` — add nightly stage `iroh-parity-nightly` triggering `cargo test --features p2p,p2p-iroh --test 'iroh_*' -- --test-threads=1` on every dev push and at midnight UTC
- Create: `genesis/manifests/RUNBOOK-iroh-parity-soak-2026-05-10.md` — operator runbook for monitoring nightly results, what counts as a divergence, escalation path

- [ ] **Step 1: Add nightly stage to orchestrator**

Insert in the `PIPELINES` map alongside existing entries. Use `cron('0 0 * * *')` trigger plus `UpstreamCause` from any change that touches `elohim/elohim-storage/src/p2p/` or `src/p2p_iroh/`.

- [ ] **Step 2: Define divergence criteria in runbook**

Divergence = any `iroh_*_real_backend` test fails on a build where the corresponding libp2p code path is green, OR any `iroh_*_parity` test fails. Single failure ≠ divergence (could be flake); 2 in a 7-day window = escalate to investigation.

- [ ] **Step 3: Soak start commit**

```bash
git add genesis/orchestrator/Jenkinsfile genesis/manifests/RUNBOOK-iroh-parity-soak-2026-05-10.md
git commit -m "$(cat <<'EOF'
ci(iroh): nightly parity soak — gate #6 of iroh cutover

Adds nightly iroh-parity-nightly stage to orchestrator. Runs all
parity + real-backend tests at midnight UTC and on every dev push.
Soak window opens at this commit; divergence threshold per RUNBOOK.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 4: Track soak progress**

Each nightly run logs to a soak-tracker artifact. After 7 consecutive zero-divergence runs, gate #6 is closed; commit the closure note to the same RUNBOOK.

### Gate #7 — Alpha-cluster soak (one week, 6 peers)

**Files:**
- Modify: `genesis/manifests/alpha-cluster.yaml` (or wherever the alpha topology is defined per memory `project_alpha_topology_bootstrap_pair`) — set `TRANSPORT_BACKEND=dual-stack` for all 6 peers
- Create: `genesis/manifests/RUNBOOK-iroh-alpha-soak-2026-05-10.md`

- [ ] **Step 1: Verify cluster manifests** — `kubectl get -n ethosengine deployment` to enumerate alpha peers; quote actual state in runbook (per memory `feedback_verify_cluster_state_before_runbook`)

- [ ] **Step 2: Flip transport backend** — apply manifest update via `kubectl patch deployment` (per Che workspace pattern — no docker locally per memory `feedback_shift_measure_jenkins`)

- [ ] **Step 3: Define soak success** — every blob fetch served from iroh OR libp2p with operator-visible `/status` showing both counters incrementing; zero "no shared transport" errors in 7 days

- [ ] **Step 4: Commit + open soak window**

### Gate #8 — Latency stress (10k blob round-trips, p99 ≤ libp2p baseline)

**Files:**
- Create: `elohim/elohim-storage/tests/bench_blob_stress_10k.rs` — release-mode `#[ignore]` test; runs 10,000 fresh round-trips on each transport, computes p50/p95/p99
- Modify: `justfile` — add `bench-stress` recipe

- [ ] **Step 1: Write the stress test** — mirror `tests/bench_blob_perf.rs` shape; iterate 10k times per transport; collect histogram; print sorted percentiles

- [ ] **Step 2: Run baseline on libp2p** — capture p50/p95/p99 numbers for comparison

- [ ] **Step 3: Run iroh stress** — assert p99(iroh) ≤ p99(libp2p) within 10% (loopback bench shows iroh wins p50 by 4×–290×; p99 should stay ≤ libp2p without much slack)

- [ ] **Step 4: Commit** with the result table embedded in the test as a doc comment

### Gate #9 — Consumer-grade soak (NEW per spec)

**Files:**
- Create: `genesis/manifests/RUNBOOK-iroh-consumer-soak-2026-05-10.md` — three sub-runbooks for phone/cellular, Chromebook/school-WiFi, residential CGN

Per spec line 518: "**If iroh fails any of these, the affected plane stays libp2p-canonical for that device class permanently.**" This is a structural decision, not a regression test.

- [ ] **Step 1: Identify consumer-grade test devices** — coordinate with operator; document device archetype + network type for each

- [ ] **Step 2: Deploy iroh-only mode to one consumer device per archetype** — with libp2p fallback enabled at the manifest level (any plane that fails iroh will fall through)

- [ ] **Step 3: Run for 7 days** — log every plane attempt: success or fall-through; per-archetype daily report

- [ ] **Step 4: Decision per (plane, device-archetype)** — if iroh succeeded ≥99% of attempts, mark plane iroh-canonical for that archetype; else mark plane libp2p-canonical for that archetype permanently

- [ ] **Step 5: Update `peer_transport_manifest.capability_level`** seed defaults per Plan 1 to reflect the per-archetype decisions

- [ ] **Step 6: Commit decisions** to RUNBOOK + Plan 1's capability_level seed defaults

### Gate #11 — Rollback drill playbook

**Files:**
- Create: `genesis/manifests/RUNBOOK-iroh-rollback-drill-2026-05-10.md`

- [ ] **Step 1: Document the env-flip procedure** — `TRANSPORT_BACKEND=libp2p` env var on elohim-storage; no rebuild required (Phase 1's `TransportBackend` selector)

- [ ] **Step 2: Document what does NOT rollback automatically** — per Plan 4 + Phase 11 prep doc: migrations, on-disk state, peer-map rows, iroh secret key. These persist; only the runtime selector flips.

- [ ] **Step 3: Define drill scenario** — "alpha cluster has been on iroh for 24h; flip to libp2p; verify all blobs served via libp2p; flip back to iroh; verify all blobs served via iroh"

- [ ] **Step 4: Run the drill in alpha cluster** — record latencies + error rates before/during/after each flip

- [ ] **Step 5: Commit drill report** to the RUNBOOK

## Cutover gate closure tracker

| Gate | Plan | Status |
|---|---|---|
| #1 — Backend wiring | Phase 11 (already on dev at 1dc7ed385) | ✅ |
| #2 — HTTP /blob dual-format | Plan 2 | ⏳ |
| #3 — Seeder dual-write | Plan 3 | ⏳ |
| #4 — Gossip dual-publish | Plan 4 | ⏳ |
| #5 — Recovery e2e | Plan 6 | ⏳ |
| #6 — CI parity soak | This master Wave 4 | ⏳ |
| #7 — Alpha-cluster soak | This master Wave 4 | ⏳ |
| #8 — Latency stress | This master Wave 4 | ⏳ |
| #9 — Consumer-grade soak | This master Wave 4 | ⏳ |
| #10 — pkarr resolver | Plan 5 | ⏳ |
| #11 — Rollback drill | This master Wave 4 | ⏳ |
| #12 — Column-drop migration | Per spec line 521: STAYS — no-op rename, not a drop | ✅ (no work) |

## Dispatch shape recommendation

Given the wave structure, **dispatch one implementation agent per Plan in Wave 1 in parallel**, then dispatch Wave 2 agents in parallel after Wave 1 lands, etc. Each agent uses `superpowers:subagent-driven-development` to execute its plan task-by-task.

Per the user pattern (`feedback_dev_branch_no_pr.md`): each plan's commits land directly on dev as a stack of commits. No PR per plan; the integration target is dev.

Per the user pattern (`feedback_subagent_scope_guardrails.md`): every implementation-agent dispatch prompt must explicitly forbid scope creep, dep changes, and destructive git ops, and require BLOCKED report instead of silent cleanup.

**Worktree strategy:** one worktree per plan in Wave 1 (parallel execution = isolated workspaces). Wave 2 worktrees branch off post-Plan-1-landing dev. Single worktree per plan; multiple commits per worktree; ff into dev when each plan completes.

## Self-review

- ✅ Every cutover gate has either a sub-plan or a Wave-4 task block
- ✅ Inter-plan dependencies graphed; wave order honors them
- ✅ All 5 Decisions Required have stated recommendations
- ✅ All 3 Discovery Required items from Plan 6 have actions
- ✅ Soaks have measurable acceptance criteria (zero-divergence, p99 ratio, ≥99% success)
- ✅ Rollback drill defines what does NOT auto-rollback (persistent state)
- ✅ Per-archetype capability decisions feed back into Plan 1's seed defaults (gate #9 → Plan 1)
