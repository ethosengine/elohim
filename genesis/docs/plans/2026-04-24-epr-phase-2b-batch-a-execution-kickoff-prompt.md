Execute **EPR Phase 2B — Batch A: Identity & controller foundation** (12 tasks, A.1 through A.12). This is implementation, not design — the spec and plan are committed and the design decisions are locked. Follow the plan task-by-task via `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans`.

## Context (self-contained)

Batch A is the first execution slice of the four-batch Phase 2B shape (A/B/C/D mirroring Phase 2C). It lands the pieces every other Phase 2B batch depends on:

- **`AgentPeerBinding`** entry type (Category A — DHT-notarized) in imagodei integrity zome, replacing `StubIdentityMap` with a real peer↔agent binding
- **`ReconcileController`** skeleton in elohim-storage — the k8s-style controller loop that is Principle P1 of the spec (DHT = manifest, libp2p = controller)
- **DNA signal stream** (`imagodei → elohim-storage`) carrying `KeyRotation`, `KeyRevocation`, `AgentPeerBinding`, `RevocationAttestation` signals. **This is the shared coordination surface with Recovery M4.**
- **Two-level verify cache** — per-agent pubkey timeline LRU + per-envelope `verified_at` flag on `epr_atoms`, with eager revocation sweep on observed `KeyRevocation`
- **libp2p identity handshake** + **gossipsub `elohim/identity/binding` topic** for session binding exchange + mid-session rotation propagation

The 12 tasks deliver the hinge between the Recovery epic (producer) and the eventual Phase 3–7 graph surface (consumer). Every subsequent 2B batch reads what Batch A writes.

## Required reading (in this order)

1. **The plan.** `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md` — start at §"Pre-flight" then §"Batch A". All 12 tasks have concrete files, step-by-step test-implementation-commit structure, code snippets.
2. **The spec.** `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md` — §2 (Principle P1) and §3.1–§3.2 (decisions #1 and #2 are Batch A's scope) are load-bearing. §5 (invariants I1–I9) applies to Batch B but anchor-check any Batch A code that might inherit the pattern.
3. **The Batch D addendum.** `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2c-batch-d-completion-addendum.md` — inherited state. Retrospective lessons (lines 28–94) matter: the wire format is locked, Envelope is canonical, EprHead is downgraded to A2 (that's Batch B's work; do not touch here).
4. **Recovery M4 state.** Check `feature/recovery-m4-fast-path-revocation` branch for current M4 progress. If M4 has landed DNA signal emission in imagodei, Task A.3's subscriber side consumes M4's schema; if not, A.3 designs the schema and M4 follows.
5. **Code seams.** The five `TODO(phase-2b)` markers in `elohim/elohim-storage/src/services/epr_store.rs:7,192,221,230,261` — Batch A resolves 2 of these (the identity-resolution seam and the verify-cache seam); the other 3 go to Batch D.

## Session shape

**Skill:** Use `superpowers:subagent-driven-development` — dispatch a fresh subagent per task, review between tasks, commit at task boundary. Pattern per task:

1. Read the task's §"Files" and §"Source of truth:" sections
2. Dispatch subagent with the full task text (the plan's step-by-step structure is subagent-ready)
3. Subagent executes steps 1–N (write test → verify fail → implement → verify pass → commit)
4. Review the commit diff against the plan's expected shape
5. If clean, mark task done and move to next. If drift, course-correct before continuing

**Do not batch tasks in one subagent.** Each task gets a fresh subagent with full plan context — this is the pattern from `subagent-driven-development`.

## Scope

**In scope for Batch A (the 12 tasks):**

- A.1 — JSON schema for `AgentPeerBinding` + `DeviceArchetype`
- A.2 — `AgentPeerBinding` entry type + validators + LinkTypes in imagodei integrity zome + sweettest coverage
- A.3 — DNA signal stream contract (schema + Rust types + trait + stub) — **converges with Recovery M4**
- A.4 — `ReconcileController` skeleton (controller loop routing signals to handlers)
- A.5 — `peer_identity_bindings` table + `HolochainBackedPeerIdentityMap` replacing `StubIdentityMap`
- A.6 — Per-agent pubkey timeline LRU + `PubkeyValidity` derivation from `KeyRotation`+`KeyRevocation` chain
- A.7 — `verified_at` + `verified_signer_fingerprint` columns on `epr_atoms` + resolver-backed Ed25519 verify in ingest
- A.8 — Eager revocation sweep in `ReconcileController::on_key_revocation`
- A.9 — libp2p authentication handshake (new protocol `/elohim/identity/handshake/1.0.0`)
- A.10 — Gossipsub `elohim/identity/binding` topic subscription + publish
- A.11 — Real `HolochainAppSignalStream` connecting to imagodei conductor
- A.12 — End-to-end integration test: rotation propagation + revocation sweep across two peers

**Explicitly OUT of scope (other batches):**

- Projector code (Batch B)
- `EprHead` refactor (Batch B)
- Pillar manifest `projections` schema (Batch B)
- `/api/v1/signal/emit` endpoint (Batch C)
- Signal harness Angular migration (Batch C)
- Write-through flag layers (Batch C)
- Kad `start_providing` on Announce (Batch D)
- Reach-tier fanout routing (Batch D)
- Direct-notify on revocation (Batch D — but flag the affected-peer-list contract in A.8 as a deferral pointer)

If you find yourself reaching for code in Batch B/C/D territory, stop and check whether Batch A genuinely requires it. If yes, note the scope-creep in a commit message. If no, defer.

## Branch & worktree

**Starting point:** `feature/epr-phase-2b-design` @ `ca4ec786` (the head of this session, containing the spec + plan).

**Recommended:** fork to `feature/epr-phase-2b-batch-a` at session start. Keeps the design branch clean and makes Batch A independently reviewable/mergeable.

```bash
git worktree add -b feature/epr-phase-2b-batch-a .claude/worktrees/epr-phase-2b-batch-a feature/epr-phase-2b-design
```

If Recovery M4 lands first on `dev`, rebase the Batch A branch onto dev before continuing — M4's DNA signal emission may already define the schema Batch A task A.3 consumes.

## Convergence protocol with Recovery M4

Task A.3 (DNA signal stream contract) is shared with Recovery M4's fast-path revocation work. Coordination points:

- **If M4 branch has landed signal emission:** Task A.3 reads M4's schema shape and the subscriber code conforms. Do not redesign.
- **If M4 branch has NOT yet landed signal emission:** Task A.3 designs the schema at `elohim/sdk/schemas/v1/dna-signal-stream.schema.json` (+ per-signal sub-schemas). Post a note to M4's branch owner before committing so they can review.
- **If the schemas drift (one branch changes before the other consumes):** resolve at the schema file, which is the single coordination surface per memory `project_epr2b_recovery_m4_convergence.md`.
- **Task D.5 (direct-notify on revocation) is not Batch A's work**, but its dependency on M4's affected-peer list is. Flag this as a deferred pointer when closing Batch A.

**Do not touch** the `feature/recovery-m4-fast-path-revocation` branch directly from this session. Any coordination happens via the shared schema file or by explicit hand-off to the M4 session.

## Build & verification commands

Per project CLAUDE.md — these are the commands Batch A will run frequently:

```bash
# elohim-storage build
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release

# elohim-storage unit + integration tests
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration

# Imagodei DNA pack (after A.2 changes)
cd elohim/holochain && nix develop -c hc dna pack dna/imagodei

# Sweettest (required for A.2 DNA changes per memory feedback_swarm_composition_fresh_tree_build)
cd elohim/holochain/tests/sweettest && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test imagodei_peer_binding

# Schema validation (after A.1 / A.3 / A.9 schema changes)
pnpm run schema:test
pnpm run schema:validate
pnpm run schema:check-dna
pnpm run schema:codegen:ts

# TS type export (if Rust View types change — not expected in Batch A, but check)
cd elohim/elohim-storage && cargo test export_bindings

# Holochain integrity validator constraint check
# Per memory project_hdi_no_get_links_in_validators: A.2 validator must NOT call get_links —
# cross-entity checks (rotation chain freshness) live in coordinator pre-commit gates, not in integrity
```

**Husky pre-push hook** auto-detects changed projects and runs quality gates. Do not bypass (`HUSKY=0 git push` is a red flag — investigate what's breaking rather than skipping).

## Known risks (from plan §"Known risks")

1. **HDI `get_links` constraint on validator.** Per memory `project_hdi_no_get_links_in_validators`, A.2's validator cannot walk links. Cross-entity checks (e.g., "is this agent's key currently rotated?") live in the coordinator pre-commit gate, not in the integrity validator.
2. **Diesel migration ordering.** A.5 (peer_identity_bindings) and A.7 (epr_atoms columns) both add migrations. Ensure timestamp naming keeps them in intended order.
3. **`HolochainAppSignalStream` (A.11) vs sweettest fixtures.** Real signal stream requires a running conductor. Use sweettest harness for integration tests; do not require a full elohim-storage binary to start for unit tests of A.4–A.10.
4. **Batch A independence.** A.9 (handshake) and A.10 (gossip) depend on A.5's table existing. A.11 (real signal stream) depends on A.3's trait + A.2's DNA entry. Plan the task order accordingly.

## Definition of done — Batch A

- [ ] All 12 tasks in the plan's §"Batch A" complete; each has its own commit (or a small number of logically-cohesive commits)
- [ ] `StubIdentityMap` no longer constructed anywhere (grep confirms zero callsites remain at `elohim/elohim-storage/src/p2p/mod.rs:913,988`)
- [ ] `verify_incoming_epr` still does structural-only verify (it's called from the wire handler); the resolver-backed verify happens in `EprService::ingest` before persistence (Task A.7)
- [ ] All existing Batch D Phase 2C integration tests still pass (no regressions in `epr_atom_federation_integration.rs`)
- [ ] Sweettest for `AgentPeerBinding` passes (Task A.2)
- [ ] Integration test (Task A.12) demonstrates two-peer rotation + revocation + sweep end-to-end
- [ ] DNA signal stream schema committed and coordinated with Recovery M4 (either consumed M4's shape or announced the new shape)
- [ ] Branch `feature/epr-phase-2b-batch-a` merged (or ready for merge) to `dev` via orchestrator DAG
- [ ] Next-batch kickoff: Batch B execution prompt drafted at `genesis/docs/plans/YYYY-MM-DD-epr-phase-2b-batch-b-execution-kickoff-prompt.md`

## Constraints & conventions (reminders)

- **Principle P1** (spec §2): elohim-storage is a reconciliation controller. Lazy-accept is not an option for integrity state. Eager sweeps must be index-bounded and observable.
- **Schema-first IoC** (memory `feedback_schema_first_ioc`): for A.1, A.3, A.9 — write the JSON schema first; the Rust/TS types conform.
- **Three-layer truth model** (memory `project_three_layer_truth_model`): DHT = notary (A-class, `AgentPeerBinding`); libp2p data-ops = projection (C-class, `peer_identity_bindings`, verify cache); doorway stays out of this entirely.
- **DHT capacity** (`.claude/skills/p2p-design-gate/SKILL.md`): imagodei is at 28/~100 entry types before A.2; adding `AgentPeerBinding` takes it to 29. Plenty of headroom.
- **`dht_anchor_hash` column naming**: tables that project DHT-notarized entries MUST use this column name per p2p-design-gate convention. A.5's `peer_identity_bindings` table follows this.
- **No agent-scoped attestation (B/B2) entities introduced**: Phase 2B design deliberately skews to A (DHT authority) + C (operational reconciliation). If a task tempts you toward B/B2, pause and check the spec §3.1 rationale.
- **Do not merge implementation commits into `feature/epr-phase-2b-design`.** That branch is the design session's output and should stay clean for future reference. Fork to a batch-specific branch.

## Memories worth checking on start

- `project_principle_p1_reconciliation_controller.md` — the spine of Batch A's architecture
- `project_epr2b_recovery_m4_convergence.md` — the DNA signal stream coordination protocol
- `project_hdi_no_get_links_in_validators.md` — load-bearing constraint on Task A.2
- `project_three_layer_truth_model.md` — DHT / libp2p / doorway split
- `feedback_schema_first_ioc.md` — A.1/A.3/A.9 schema-first order
- `feedback_swarm_composition_fresh_tree_build.md` — run `cargo check` from a clean tree before committing DNA changes
- `project_multi_device_humans.md` — `DeviceArchetype` enum content

## How to handle blockers

If a task fails in a way the plan doesn't anticipate:

1. Report what the plan said (the expected step output) vs what actually happened
2. Identify whether the drift is (a) a plan bug (the step's expected code is wrong), (b) a hidden dependency (something the task sketch didn't flag), or (c) a design-decision gap (a coupling decision the spec didn't resolve)
3. For (a): fix inline, note the plan amendment in the commit message
4. For (b): stop, coordinate (likely with Recovery M4 if DNA-side; likely with main session operator if elohim-storage-side)
5. For (c): stop, do not invent a new design decision — surface it as a blocker to the operator. The spec's §7 (O1–O8) are already-deferred questions; if your blocker is one of those, the operator chooses the resolution.

**Do not bypass tests, lint, or husky to unblock.** If a pre-commit hook fires, investigate and fix the root cause. The hook cooldown for p2p-design-gate is already set through 2026-05-01 for this plan's mechanical word-matches — no further silencing needed.

---

Go.
