---
id: self-healing-program-roadmap
status: Draft
cites:
  - self-healing-control-plane-design | the four-pillar control-plane design + structural no-overwhelm invariant this roadmap sequences into a buildable program | sha256:e46a55190a70c79b | path: genesis/docs/superpowers/plans/2026-06-13-self-healing-control-plane-design.md
  - self-healing-opportunity-map | the user-agency axis (see/reset/pause; WANT→GapTracker→REA-Commitment loop) this roadmap sequences alongside the control-plane design | sha256:31400dda6437b0dd | path: genesis/docs/superpowers/plans/2026-06-13-self-healing-opportunity-map.md
---

# Self-Healing Control Plane — Program Roadmap (2026-06-13)

The actuatable self-healing control plane, decomposed into a buildable program: 2 design docs + 5 implementation-ready plans + 1 cross-thread handoff. **All work is plan-only — no code has changed.** Execution is gated on your go and coordination with the parallel (arc) thread.

## Artifacts

**Design** (this directory — working drafts):
- `2026-06-13-self-healing-opportunity-map.md` — the user-agency axis (see / reset / pause; the WANT → GapTracker → REA-Commitment loop; mostly wiring over substrate that already self-heals).
- `2026-06-13-self-healing-control-plane-design.md` — the four-pillar control plane + the structural no-overwhelm invariant; §12 = the memory axis + arc-shrink. **This is the single current source of the design.**

**Plans** (`genesis/docs/superpowers/plans/`):
| Plan | Pillar | What it ships |
|------|--------|---------------|
| `…auto-config-resource-probe-plan.md` | 3 (Auto) | cgroup CPU+MEM readers, `derive()`, `worker_threads` fix, `/admin/auto-preset` — in shared `elohim-compute` |
| `…upstream-self-protection-plan.md` (A) | 1 (self-protect) | shared `CircuitBreaker` + health-gated warm-up + total budget/timeout + anti-self-partition — the durable freeze cure |
| `…inbound-admission-backpressure-plan.md` (B) | 2 (negotiate) | doorway accept-loop shed (503/Retry-After) + storage shed/advertise + bounded `forward_to_storage` |
| `…stability-surface-read-model-plan.md` (C) | surface | `GET /admin/self-healing` aggregate read model (human/UI/agent-consumable) |
| `…elevate-arm-runtime-harvest-plan.md` (D) | loop closure | external `runtime-harvest` poller → findings ledger + `runtime-triage` dispatch (zero Rust) |

**Handoff:** arc-shrink / `target_arc_factor` → the **other thread** (`genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md`). Consumes the Auto plan's cgroup MEM reader. This program does **not** touch conductor config.

## Dependency order / execution sequence

1. **Auto-config** — creates `elohim-compute/src/limits.rs` + the cgroup readers + the `/admin/auto-preset` surface. Everything downstream consumes the probe and the shared-crate home. *Safe in prod: derives the same `4` on the `cpu:1` doorway pod.*
2. **A (Upstream self-protection)** — **defines** the shared `CircuitBreaker` in `elohim-compute/peers.rs`; lands the freeze cure.
3. **B (Inbound admission)** — **pure consumer** of A's `CircuitBreaker`. **A is the single canonical owner**; B does NOT embed a copy — B's Task 1 is "ensure `elohim-compute` exposes it; land verbatim from A's definition only if absent, else verify-only." **A→B is a hard dependency** (two independently-authored copies would be a drift trap that breaks the skip-if-present check).
4. **C (Stability surface)** — aggregates A/B/Auto state; reserves pending keys with `// FOLLOW-ON` seams (self-contained, not hard-blocked).
5. **D (Elevate arm)** — polls C's `/admin/self-healing` (primary) + `/admin/render-stats` (value today); files findings + dispatches triage.
- **Parallel:** arc-shrink (other thread), once the Auto MEM reader lands.

*Rationale:* Auto first (shared home + probe). A before B (A defines the breaker; both idempotent). C after A/B/Auto (consumes their state; self-contained). D after C (primary input; ships render-saturation value before C lands).

## Shared primitives / convergence

- **`elohim-compute` is the shared crate home** both doorway and storage already depend on: Auto adds `limits.rs`; **A is the sole author of `CircuitBreaker`/`CircuitState` in `peers.rs`; B consumes it** (never re-defines it — A→B is a hard dependency, not a coincidence of identical text).
- **`CircuitBreaker`** is pure + tick-injected (unit-testable, no wall-clock): A drives it with a pass-counter tick, B with wall-clock-seconds — the type is unit-agnostic, so both are valid.
- **`/admin/self-healing`** (C) is the one read surface D *and* the runtime AI agent consume.

## Cross-cutting constraints (in every plan)

- **No-runtime-write rule:** runtime Rust NEVER writes `.claude/data`; D is the external poller (reads endpoints, writes the ledger).
- **p2p-class:** every new entity is **Cat C node-local read-model** (no DHT entry, no table, no coordinator fn) — verified against the p2p-design-gate. The REA `delegates-compute` *actuation* (runtime knob mutation — the "recover" step's write side) is a **deferred separate pillar**; this program is **observe + self-protect + elevate**, knobs stay boot/env.
- **Coordination:** A owns `warm_stream.rs`; B isolated its proxy breaker to a new `routes/upstream_health.rs` to avoid colliding with A; the parallel thread owns conductor config / `target_arc_factor` / storage hygiene leak-fixes. No plan touches conductor config. **Execute selective-staged or in an isolated worktree.**
- **Build/test:** per-crate `RUSTFLAGS` (doorway/compute `""`, storage `--cfg getrandom_backend="custom"`), `/tmp` `CARGO_TARGET_DIR` (pool fingerprint ENOENT), plain `cargo test` (no nextest); D is Python stdlib (no pytest → self-running harness).

## NOT in this program (named follow-ons)

- **REA `delegates-compute` actuation** — the recover step's runtime knob mutation; the largest remaining pillar.
- **The Angular stability page** — mount the dark `HealthIndicatorComponent` + `/shefa/health` route + render-verify; C ships the endpoint + typed contract, the UI is a sibling frontend plan.
- **Bilateral credit/window numeric accounting** — B does coarse admit/shed/advertise/honor.
- **Auto-derived admission ceiling** — B uses `DOORWAY_MAX_INFLIGHT` env; wire to Auto's `derive()` once landed.
- **arc-shrink** — other thread.

## Doc-hygiene state — resolution (a) applied (working drafts)

**Done (2026-06-13):** all five plans are now plain working drafts. C's + D's frontmatter stripped to match A/B/Auto; C's auto-created duplicate canonical spec (`…specs/2026-06-13-actuatable-self-healing-control-plane.md`, a verbatim now-stale copy of the root design doc) deleted; C's two body references to it repointed to the design doc. `2026-06-13-self-healing-control-plane-design.md` (this directory) is the single current source of the design (it alone carries the §12 arc-shrink edits).

**Deferred (b) Promote + seal** — when you commit to the program as *the plan*: pick one canonical spec home, sync the root design doc's §12 into it, `cite-gen --seal` all five plans against it. Per advisor guidance, do NOT seal vision-in-motion until you've greenlit execution.

## Execution recommendation

**Isolated-worktree, subagent-driven, in the dependency order** — zero collision with the active arc thread. Or hold until that thread settles. The live freeze is operator-covered; the durable cure (A) is not racing the clock.
