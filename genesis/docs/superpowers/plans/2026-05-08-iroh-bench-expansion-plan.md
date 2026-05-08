# iroh Transport Bench Expansion — Multi-Plane Head-to-Head

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Status note (2026-05-08):** This plan was written **retroactively** after a first agent pass scaffolded six bench files. The first pass landed clean code matching the template; it was paused mid-`cargo build` so it never ran benches or committed. Phase tasks below reflect "scaffold complete, verification + commits pending." The user's explicit process correction was: "pause agent, review what it did, write the plan, then continue." This plan is the result.

**Goal:** Extend the head-to-head transport bench (currently only `bench_blob_perf`) to validate iroh's perf bump on every wire plane the architecture spec marked iroh-canonical or dual-stack permanent. Each new bench mirrors `bench_blob_perf` exactly: release mode, `#[ignore]`, connection-reuse, p50/p95/p99 + throughput, lenient one-size-class-wins assertion, two-render stability check.

**Hard prerequisite (gating, must hold before this plan starts):** The architecture spec at `genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md` exists and has a §"Plane-by-plane verdict and decision rule" section. **Confirmed at plan-write — spec is on dev at `bd5dd817`.**

**Architecture:** Each plane gets its own `tests/bench_<plane>_perf.rs` mirroring `bench_blob_perf.rs`. A `just bench-<plane>` recipe runs each (release + `--ignored` + `--nocapture`). A `just bench-all` umbrella target runs every plane in sequence. Final commit updates `p2p_iroh/README.md` "What works" with combined results table and appends a memory addendum to `project_iroh_parallel_stack_phases3_7_landed.md`.

**Tech stack (do not reinvent):**
- Template: `elohim/elohim-storage/tests/bench_blob_perf.rs` (615 lines) — mirror its shape exactly
- iroh-side workload templates: parity tests in `tests/iroh_*_parity.rs`
- libp2p-side harness model: `tests/harness/mod.rs` + production protocols in `src/p2p/`
- Shared utility: `super::codec::{read_frame_default, write_frame, read_frame_cbor_default, write_frame_cbor}` — same wire format both transports

## In-scope vs out-of-scope (per spec verdict table)

| Plane | Spec verdict | In scope? | Bench file |
|---|---|---|---|
| **Blob** | iroh-canonical, libp2p-fallback | already done (commit `bd0a2f75` in dev) | `bench_blob_perf.rs` |
| **Gossip** | dual-stack permanent | yes | `bench_gossip_perf.rs` |
| **Sync** | dual-stack permanent | yes | `bench_sync_perf.rs` |
| **EPR** | dual-stack permanent | yes | `bench_epr_perf.rs` |
| **EPR-atom** | dual-stack permanent | yes | `bench_epr_atom_perf.rs` |
| **Shard** | dual-stack permanent | yes | `bench_shard_perf.rs` |
| **View-fed** | dual-stack permanent | yes | `bench_view_fed_perf.rs` |
| **Identity-handshake** | dual-stack permanent (corrected `bd5dd817`) | yes | `bench_identity_handshake_perf.rs` |
| **Trust** | dual-stack permanent (corrected `bd5dd817`) | yes | `bench_trust_perf.rs` |
| **Reach-authorization** | n/a — internal service, not a wire plane | NO — feature is canonical, but no wire plane to bench | — |
| **Discovery** | dual-stack pkarr + Kademlia | NO — peer-discovery mechanism, not a workload plane | — |

**Total benches to land: 8 new** (6 from initial scope + 2 from spec correction). 6 of 8 already scaffolded by first agent pass.

## Hard requirements (per bench)

Every bench MUST:

- `#![cfg(all(feature = "p2p", feature = "p2p-iroh"))]` at top
- `#[tokio::test(flavor = "multi_thread", worker_threads = 4)]` + `#[ignore]` on test fn
- Release mode (just recipe enforces it)
- `--nocapture` for output
- Percentile reporting: p50 / p95 / p99 + mean + meaningful throughput unit (rps for request-response, msgs/sec for gossip)
- **Connection-reuse pattern**: open one QUIC `Connection` and one libp2p connection up front, reuse across iterations
- **Seed-deterministic payload generator**: SplitMix-style PRNG matching `bench_blob_perf::build_payloads`
- **Lenient assertion**: gate is "iroh p50 < libp2p p50 on ≥1 size class out of N tested"; same `bump_found` logic as `compare_blob_perf`
- **Two-render stability**: bench runs twice; both must pass the assertion before committing
- Use canonical `endpoint.connect(node_addr, ALPN)` pattern (NOT iroh-blobs Downloader pool)
- Markdown table output with one row per (transport, size class)

## Phasing

| Phase | Scope | Status |
|---|---|---|
| 1 | Initial-scope benches scaffolded (6 files, justfile recipes) | **Done by first agent** (uncommitted) |
| 2 | Build verification, two-render runs, per-bench commits | Pending |
| 3 | Spec-correction benches (Identity-handshake, Trust) | Pending |
| 4 | Umbrella target + README + memory + final commit | Pending |

## Phase 1: Initial-scope bench scaffolding

**Status: COMPLETE (uncommitted) by first agent pass (paused mid-build).**

Each file was written end-to-end mirroring `bench_blob_perf.rs`. Spot-checks confirm template adherence on `bench_sync_perf.rs`, `bench_gossip_perf.rs`, and `bench_shard_perf.rs`.

- [x] **Task 1.1: `tests/bench_gossip_perf.rs`** — iroh-gossip vs libp2p-gossipsub publish→receive latency on `BlobInventoryDelta` payloads. 100ms heartbeat tuning for libp2p side; explicit Subscribed wait before measured publishes. (569 lines, uncommitted)
- [x] **Task 1.2: `tests/bench_sync_perf.rs`** — `/elohim/sync/2.0.0` vs libp2p `SyncCodec` request-response. `SyncRequest::GetHeads` → `SyncResponse::Heads` round-trip; FixedHeadsBackend on iroh side. 4 head-count classes (1, 16, 256, 1024). (647 lines, uncommitted)
- [x] **Task 1.3: `tests/bench_epr_perf.rs`** — `/elohim/epr/2.0.0` vs libp2p `/elohim/epr/1.0.0` MessagePack request-response. (560 lines, uncommitted)
- [x] **Task 1.4: `tests/bench_epr_atom_perf.rs`** — `/elohim/epr-atom/2.0.0` vs libp2p `/elohim/epr-atom/1.0.0` CBOR request-response. (544 lines, uncommitted)
- [x] **Task 1.5: `tests/bench_shard_perf.rs`** — `/elohim/shard/2.0.0` vs libp2p `/elohim/shard/1.0.0` payload-size sweep with FixedShardBackend. (529 lines, uncommitted)
- [x] **Task 1.6: `tests/bench_view_fed_perf.rs`** — `/elohim/view-federation/2.0.0` vs libp2p `/elohim/view-federation/1.0.0`, ViewFederationRequest/Response with topology-edge payloads (256 KiB cap). (562 lines, uncommitted)
- [x] **Task 1.7: `justfile` updates** — `bench-sync`, `bench-epr`, `bench-epr-atom`, `bench-shard`, `bench-view-fed`, `bench-gossip` recipes + `bench-all` umbrella target. (uncommitted)

**Compile/build state at agent pause:** `cargo build --features "p2p p2p-iroh"` was running when agent was killed. Compile state is **unverified** — Phase 2 must verify before committing.

## Phase 2: Build verification + per-bench commits

Each bench must compile clean, run twice (two-render stability), and commit individually. Per-bench commit pattern matches `bench_blob_perf` (`bd0a2f75`): one bench, one commit.

- [ ] **Task 2.1: Verify clean build** — `cd elohim/elohim-storage && just build-iroh`. Must complete with zero warnings on the new bench files (existing dead-code warnings in unrelated modules are pre-existing and not gating).
- [ ] **Task 2.2: Bench gossip** — `just bench-gossip` twice; both must assert. Capture markdown table from each run. Commit: `feat(storage): bench_gossip_perf — head-to-head iroh-gossip vs libp2p-gossipsub`
- [ ] **Task 2.3: Bench sync** — `just bench-sync` twice; both must assert. Commit: `feat(storage): bench_sync_perf — head-to-head iroh vs libp2p sync plane`
- [ ] **Task 2.4: Bench EPR** — `just bench-epr` twice; both must assert. Commit: `feat(storage): bench_epr_perf — head-to-head iroh vs libp2p EPR plane`
- [ ] **Task 2.5: Bench EPR-atom** — `just bench-epr-atom` twice; both must assert. Commit: `feat(storage): bench_epr_atom_perf — head-to-head iroh vs libp2p EPR-atom plane`
- [ ] **Task 2.6: Bench shard** — `just bench-shard` twice; both must assert. Commit: `feat(storage): bench_shard_perf — head-to-head iroh vs libp2p shard plane`
- [ ] **Task 2.7: Bench view-fed** — `just bench-view-fed` twice; both must assert. Commit: `feat(storage): bench_view_fed_perf — head-to-head iroh vs libp2p view-fed plane`

**If a bench fails its assertion** on either render: do NOT skip / mock / disable. Surface BLOCKED with the specific failure mode (which size class showed iroh ≥ libp2p p50, ratios from the markdown table). The user decides whether to:
- Tune the workload (different size classes might surface a clearer win)
- Accept the result and adjust the spec verdict for that plane
- Investigate a code-level issue with the bench

**If a bench fails to compile**: do NOT downgrade dependencies, do NOT modify production protocol code in `src/p2p/` or `src/p2p_iroh/`. Surface BLOCKED.

## Phase 3: Spec-correction benches (Identity-handshake, Trust)

The spec was corrected at `bd5dd817` — Identity-handshake and Trust verdicts moved from "libp2p-canonical, iroh-receive" to "dual-stack permanent" because integrity comes from Track 1 DHT-notarized agent identity + signed wire frames, not from any transport-level security property.

These two planes were not in the first agent's scope. They need fresh scaffolding mirroring the same template.

- [ ] **Task 3.1: `tests/bench_identity_handshake_perf.rs`** — `/elohim/identity-handshake/2.0.0` vs libp2p `IdentityHandshakeProtocol`. Workload classes: handshake roundtrips with varying claim payload sizes. Same template as `bench_sync_perf.rs`.
- [ ] **Task 3.2: `tests/bench_trust_perf.rs`** — `/elohim/trust/2.0.0` vs libp2p `TrustProtocol`. Workload classes: attestation request/response with varying attestation payload sizes.
- [ ] **Task 3.3: justfile recipes** — `bench-identity-handshake` + `bench-trust`; add to `bench-all`.
- [ ] **Task 3.4: Run + commit** — same two-render stability + per-bench commit pattern as Phase 2.

**Source workload patterns:** `tests/iroh_auth_parity.rs` (covers both identity-handshake AND trust); `src/p2p/identity_handshake.rs`, `src/p2p/trust_protocol.rs` for libp2p side.

## Phase 4: Umbrella + README + memory

- [ ] **Task 4.1: `bench-all` umbrella target verified** — `just bench-all` runs all 8 benches in sequence, each prints its markdown table, terminal shows combined report.
- [ ] **Task 4.2: Update `elohim/elohim-storage/src/p2p_iroh/README.md`** — "What works" section gets a "Bench coverage" subsection with combined markdown table of p50/p95/p99 numbers per plane per transport per workload class. From the two-render runs in Phases 2 + 3.
- [ ] **Task 4.3: Update memory** — append surgical addendum to `/projects/.claude-config/projects/-projects-elohim/memory/project_iroh_parallel_stack_phases3_7_landed.md` noting bench coverage is now multi-plane (8 planes), with a one-line summary of the combined verdict.
- [ ] **Task 4.4: Final commit** — `docs(storage): bench-all umbrella + README + memory — multi-plane bench coverage`

## Acceptance criteria

- [ ] All 8 bench files compile clean (`cargo build --features "p2p p2p-iroh" --tests`)
- [ ] Each of the 8 benches passes its lenient assertion in two consecutive runs
- [ ] `just bench-all` runs end-to-end without timeout (each bench is single-digit minutes)
- [ ] `p2p_iroh/README.md` "What works" has a "Bench coverage" table with numbers from the two-render runs
- [ ] Memory file `project_iroh_parallel_stack_phases3_7_landed.md` has the bench-coverage addendum
- [ ] All commits land on `worktree-iroh-parallel-stack` (NOT pushed); merge timing surfaced to user for explicit consent

## Pre-cutover constraint (from memory)

Per `project_iroh_parallel_stack_phases3_7_landed.md` and the user's pipeline-stable rule (`feedback_dev_branch_no_pr.md` says dev is local integration, no PR; the pre-cutover memory says park on worktree until pipeline-stability work is closed):

**Do NOT push.** All bench commits stay on `worktree-iroh-parallel-stack` until the user explicitly consents. The plan's deliverable is the parked commits + a status summary; merge timing is a separate user decision.

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| Loopback noise hides small iroh wins on some planes | Lenient assertion (one size class out of N must show iroh < libp2p p50); two-render stability check; if a plane fails, surface BLOCKED rather than weakening the assertion |
| Some plane's existing iroh-side wire protocol differs in shape from request-response (gossip is one-to-many) | First agent already handled this for `bench_gossip_perf.rs` (publish→receive latency); same template for any other one-to-many planes |
| Build time for compile + 8 benches is multi-hour | Per-bench commits + `bench-all` running them sequentially mean intermediate progress is preserved; an interrupted run can resume at the next bench |
| iroh-blobs / iroh / iroh-gossip pin must NOT be changed | Hard guardrail in dispatch prompts; surface BLOCKED if a bench can't be made to compile against `=0.92 / =0.94 / =0.92` |
| Production protocol code in `src/p2p/*` or `src/p2p_iroh/*` must NOT be modified | Benches consume the existing public API only; surface BLOCKED if a bench requires a protocol change |
| `peer_blob_inventory.blake3_hash` migration must NOT be touched | This plan only adds tests/ files; no migrations affected |

## Subagent-conflict guardrails (per project memory)

Per `feedback_subagent_dep_conflict_supervision.md` and `feedback_subagent_scope_guardrails.md`:

- **Do NOT change iroh pin.** Surface BLOCKED.
- **Do NOT git revert / git reset on pre-existing commits.** Surface BLOCKED if tree state seems wrong.
- **Do NOT modify production protocol code.** Benches live in `tests/`.
- **Do NOT push.** All commits parked on worktree.
- **Do NOT skip / mock / disable a failing assertion.** Surface BLOCKED with the markdown ratios.

## What this plan deliberately does NOT do

- **Does not bench planes the spec marked out-of-scope** (Reach-authorization is an internal service, not a wire plane; Discovery is peer-discovery not a workload plane).
- **Does not introduce new bench shapes** beyond the `bench_blob_perf` template — every plane uses the same template + percentile reporting + lenient assertion.
- **Does not push.** Pre-cutover memory governs merge timing.
- **Does not change spec verdicts.** If a plane's bench shows iroh ≥ libp2p on every class, the response is BLOCKED + spec amendment, not silent rewrite.

## Status (live)

**2026-05-08, post-agent-pause:**
- Phase 1: COMPLETE (6 bench files + justfile recipes scaffolded; uncommitted).
- Phase 2: NOT STARTED. Compile verification is the gating first task.
- Phase 3: NOT STARTED. Identity-handshake + Trust scaffolding pending after Phase 2.
- Phase 4: NOT STARTED.

**Next session:** start at Phase 2 Task 2.1 (compile verification). On clean compile, proceed with Phase 2 Tasks 2.2–2.7 in order, committing each bench individually. On any failure, surface BLOCKED with specific failure mode rather than working around it.

---

## See also

- Spec: `genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md` (the architecture spec this plan executes against)
- Original parallel-stack plan: `genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md` (Phases 1-10 produced the wire transport this plan benches)
- Companion spec: `genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md` (transport context: hub-to-hub federation, where iroh's wins land)
- Bench template: `elohim/elohim-storage/tests/bench_blob_perf.rs` (commit `bd0a2f75` in dev — the canonical shape)
- p2p_iroh README: `elohim/elohim-storage/src/p2p_iroh/README.md` (Phase 11 cutover prerequisites; bench coverage will graduate "What works" section)
