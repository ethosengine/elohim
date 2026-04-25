Kick off **EPR Phase 2B — Batch C: Producer migration & ramp controls**. Batch B landed end-to-end on `feature/epr-phase-2b-batch-b` and merges to `dev`. Batch C builds on the projector + manifest registry foundation that B left in place.

Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to drive task-by-task.

## What landed in Batch B (preceding work)

| Layer | Capability |
|---|---|
| Schema | `pillar-projection.schema.json`; optional `projections[]` on `app-manifest.schema.json` |
| Manifest | shefa declares `EconomicEvent → economic_events` columnMapping (snake_case + `$cid`/`$signer`/`$issuedAt`/`$state:default`/`$verifiedAt`) |
| elohim-storage projector | `Projector` controller alongside `ReconcileController`; `ManifestRegistry`; `projector_cursor` table; column-mapping evaluator UPSERTs into manifest-declared targets |
| Invariants | I1 (idempotency via cid PK), I2 (manifest-authority guard), I3 (`(signer_cid, issued_at)` ordering), I4 (revocation sweep clears projection rows), I5 (verified-state mirrored), I6 (unmapped kinds remain in `epr_atoms`) — each pinned by tests |
| EprHead | `derive_epr_head` helper centralizes the three production sites (http.rs, p2p/mod.rs); wire format byte-stable; gate-provenance + enrich-pillars params capture the prior site differences |
| Observability | `ProjectorSignal` channel emission on every projection write; `GET /api/v1/status/projector` returns per-(pillar, kind) cursor + lag |

The projector substrate is stable. Decisions #3 (projector) and #4 (EprHead) are closed end-to-end. Batch C addresses Decisions #5 (signal harness) and #6 (write-through flag).

## Carryover items from Batch B (not blockers; track in Batch C)

These were flagged in passing during Batch B and deliberately deferred:

1. **Phase 4 manifest-graph context derivation for EprHead** — the plan's "use projector mapping to derive EprLamadContext/EprShefaContext/EprQahalContext" was scoped to Phase 4 because the contexts are nested typed structs (not flat columns). Batch C should not touch this; it is post-2B work.

2. **Pre-existing schema drift in shefa manifest** — `vocabulary.protocolPrimitives`, `vocabulary.crossPillarCoupling`, and `observation.archetype` are in shefa's manifest but not declared in `app-manifest.schema.json`'s Vocabulary `$defs`. Either tighten manifest to schema or extend schema to authorize these fields. Not blocking; surfaced by accident during B.2's spot-validation run.

3. **No JSON schema for `ProjectorStatusView`/`ProjectorCursorView`/`ProjectorLagView`** — generated from Rust via ts-rs only. The codebase's pattern is "schema → Rust → ts-rs → schema_contract.rs". Status surface was scoped as internal observability so this was deferred. Add the schemas when the surface stabilises into a client contract.

4. **`down.sql` convention** — `migrations/2026-04-25-020000_economic_events_verified_at/down.sql` was initially a no-op (`SELECT 1`); aligned to `ALTER TABLE … DROP COLUMN` per established precedent. Future migrations should follow that pattern by default.

5. **HTTP test server pattern** — codebase has no in-process HTTP test server, so B.8 tested the projector status surface at the builder-function level (`compute_projector_status`). The HTTP handler itself is a 4-line wrapper. Batch C's `/api/v1/signal/emit` handler will need a real HTTP test (the plan sketch uses `app.post(...).json(...)`); evaluate whether to introduce a test-server scaffold or stick with builder-function tests.

6. **`(signer_cid, issued_at)` ordering** — `fetch_epr_atoms_since` was changed to `.order((epr_atoms::signer_cid.asc(), epr_atoms::issued_at.asc()))` to satisfy I3. The compound index from migration `2026-04-25-000000_verified_at_on_epr_atoms` is used. If a future projector pass needs strict-monotonic per-signer cursoring, revisit the cursor key shape.

7. **`pre-existing clippy issues in `tests/peer_status_e2e.rs`, `lib.rs`, `tests/phase4_e2e.rs`, `tests/content_safety_integration.rs`** — surfaced during Batch B test runs; pre-existing and unrelated to the projector work. The `gate_client::testing` items appear to be `#[cfg(test)]`-gated in a way that breaks integration test compilation. Track as a separate cleanup pass.

## Batch C tasks

Read each task in the plan: `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md` §Batch C (lines 1093-1480).

| Task | One-liner |
|---|---|
| C.1 | Conductor signing API contract — schema + zome `sign_for_agent` + `ConductorSigningClient` wrapper |
| C.2 | Signal-intent schema + `POST /api/v1/signal/emit` (composes EPR Envelope, requests signature, ingests) |
| C.3 | Angular signal harness migration — replace direct conductor calls with `/api/v1/signal/emit` (default-OFF, per pillar) |
| C.4 | Write-through flag — manifest default + effective state view (4-layer: archetype default → policy.toml → env/CLI → admin override) |
| C.5 | Wire write-through flag into ingest path (skip projector when flag is OFF; integrity validation always on) |
| C.6 | `policy.toml` + env/CLI + admin override wiring (per memory `project_cadence_archetype_tunable_with_dev_overrides`) |
| C.7 | End-to-end shefa migration test — flag flipped, EconomicEvent flows from Angular harness through `/api/v1/signal/emit` → conductor signing → ingest → projector → row in `economic_events` |

## Required reading before starting

1. Plan §Batch C (lines 1093-1480) — task-by-task spec.
2. Spec sections referenced by the plan: Decisions #5 (signal harness) and #6 (write-through flag) from `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md`.
3. Batch B's `Projector` and `ManifestRegistry` at `elohim/elohim-storage/src/projector/` — Batch C produces atoms that this projector consumes.
4. `EprService::ingest` at `elohim/elohim-storage/src/services/epr_service.rs` — `/api/v1/signal/emit` calls into this after signing.
5. Existing signal harness in Angular: search `app/elohim-app/src/app/elohim/services/` for the current write path you are migrating.
6. Memory pin `project_cadence_archetype_tunable_with_dev_overrides` for the 4-layer flag pattern.

## Build commands (per CLAUDE.md)

```bash
# elohim-storage build + tests
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib

# imagodei zome (for C.1's sign_for_agent)
cd elohim/holochain/dna/imagodei
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown

# Angular harness migration
cd app/elohim-app && pnpm test
```

## Branch & worktree

Create a fresh worktree off `dev` (after Batch B merges):

```bash
git worktree add /projects/elohim/.claude/worktrees/epr-phase-2b-batch-c -b feature/epr-phase-2b-batch-c origin/dev
```

If Batch B has not merged yet, branch from `feature/epr-phase-2b-batch-b` instead and rebase later.

## Done definition (Batch C close)

- [ ] All 7 Batch C tasks have landing commits on this branch
- [ ] `/api/v1/signal/emit` end-to-end test passes (intent → sign → ingest → project)
- [ ] Write-through flag effective at all 4 layers (archetype default → policy.toml → env → admin)
- [ ] Angular signal harness migrated for shefa (lamad migration is C.3 follow-on or separate batch)
- [ ] Pre-push hooks pass; clippy + fmt clean
- [ ] Commit + push; branch ready for merge to `dev` via orchestrator DAG
- [ ] Next-batch kickoff (Batch D — federation/fanout: tiered reach, Kad providers, gossipsub topic structure) drafted

## Memory pins worth re-checking on start

- `project_principle_p1_reconciliation_controller` — projector + reconcile both observe DHT signals; producer migration completes the loop.
- `project_three_layer_truth_model` — `/api/v1/signal/emit` is the storage-layer doorway-into-DHT path; doorway projection of this view is web2 surface, NOT a P2P participant.
- `feedback_schema_first_ioc` — `signal-intent.schema.json` and `conductor-signing.schema.json` are written FIRST; Rust + Angular comply.
- `project_doorway_manifest_driven_routes` — the new endpoint is declared in shefa's manifest if it's a pillar-specific surface; document in C.2.
- `project_cadence_archetype_tunable_with_dev_overrides` — write-through flag is the canonical 4-layer pattern (archetype default → policy.toml → env/CLI → admin override).
- `project_epr2b_recovery_m4_convergence` — Batch B merged M4; Batch C continues building on the merged base.

## How to handle blockers

Same protocol as Batch B:
1. Plan-bug → fix inline + note in commit
2. Hidden dependency (e.g., conductor-signing API doesn't exist yet) → coordinate (likely with main session operator if cross-batch)
3. Design-decision gap → BLOCK and surface to operator; do not invent

Do NOT bypass tests, lint, or husky.

---

Go.
