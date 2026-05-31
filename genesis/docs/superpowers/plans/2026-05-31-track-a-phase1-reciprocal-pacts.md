# Track A — Phase 1: Reciprocal Dwelling Pacts (Gap 0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`. Steps use `- [ ]`. **Verify every signature against live code before editing — Sprint-3 code drifts.**

**Goal:** Author the first reciprocal dwelling pacts (Gertrude↔Dowell + Adam↔Matthew) so real pledges exist and the resilience/capacity bars light — closing **Gap 0** (the commitment writer exists but nothing calls it; per `2026-05-29-close-the-gaps-HANDOFF.md:29-30`).

**Architecture:** The specialized writer `create_replicates_dwelling_commitment` (donut + bounds validation) is dead code. Add a caller path: (1) ensure `POST /api/v1/commitments` with `action='replicates-dwelling'` dispatches to the *specialized* writer with a fetched `ProviderPledgedState`; (2) a genesis seeder step authors the 4 commitments; (3) implement the `@wip` `household-resiliency-handshake.feature` step defs. **No new DHT entry types.**

**Tech Stack:** Rust (`elohim-storage` http.rs handler + services), TypeScript (`genesis/seeder/src/seed-commitments.ts`), Gherkin (a2o).

---

## Verbatim signatures (verify before editing)

- `create_replicates_dwelling_commitment(conn: &mut SqliteConnection, ctx: &AppContext, input: CreateReplicatesDwellingInput, provider_state: &ProviderPledgedState) -> Result<ReaCommitment, CreateCommitmentError>` — `replicates_dwelling_service.rs:90`
- `CreateReplicatesDwellingInput { provider: String, receiver: String, payload: ReplicatesDwellingPayload, dht_anchor_hash: Option<String> }` — `replicates_dwelling_service.rs:55`
- `ReplicatesDwellingPayload { action, provider_dwelling_hub_id, recipient_dwelling_hub_id, provider_role: ProviderRole, via_collective_hub_id: Option<String>, capacity_bytes: u64, scope_filter: ScopeFilter, valid_from, valid_until, grace_period_days: u32, rotation_ttl_days: u32, ratio_attestation: RatioAttestation }` — `elohim-views/src/replicates_dwelling.rs:7`
- `ScopeFilter { epr_kinds: Option<Vec<String>>, bytes_per_blob_max: Option<u64>, requires_attestations: Option<Vec<String>>, kinds_excluded: Option<Vec<String>> }`
- `RatioAttestation { commons_pct, dwelling_pct, collective_pct, free_pct, effective_ratio_cid }` — auto-populated via `constitutional_ratio_registry::effective_ratios()`
- Helpers: `build_payload()` (`:148`), `fresh_provider_state()` (`:204`) — reuse the construction pattern.

## The pacts (hub_id = `householdId` from `genesis/data/humans/humans.json`)

| Pact | provider_dwelling_hub_id | recipient_dwelling_hub_id |
|------|--------------------------|----------------------------|
| A (Gertrude↔Dowell) | `household-gertrude` (shem) | `household-matthew` (on-prem) |
| A counter | `household-matthew` | `household-gertrude` |
| B (Adam↔Matthew) | `household-adam` (shem) | `household-matthew` |
| B counter | `household-matthew` | `household-adam` |

`provider_role = steward_mutual` (`collective_steward` is explicitly rejected by the validator). These three hubs (Dowell on-prem + Gertrude + Adam on shem) are the first hub-quality replica-holders on the path to the **HA = 5 hub-quality replicas** target (full HA for landing+core arrives via the commons floor in Phase 4).

## Uncertainties to resolve in Task 1 (verify live, do not assume)

1. **Dispatch path:** does `POST /api/v1/commitments` (`http.rs:~9599` → `create_commitment` handler) route `action='replicates-dwelling'` to the *specialized* `create_replicates_dwelling_commitment` (donut validation + `ProviderPledgedState`), or only to the generic `rea_commitment_service` (which bypasses donut validation)? If the latter, **wire the specialized dispatch.**
2. **Hub-id representation (T6 convergence):** confirm whether `replication_prioritizer` / `hub_summary` match on `household-matthew` (the `household_id`) or `hub:{collective_cid}`. Use the form they match, consistently, or pledges won't bind to inventory.
3. **ProviderPledgedState** must be fetched (`compute_peer_capacity`) *before* the write or the ceiling check rejects it.

---

## Tasks

### Task 1: Resolve + wire the dispatch so the specialized writer runs from HTTP

**Files:** `elohim/elohim-storage/src/http.rs` (create_commitment handler), `.../services/replicates_dwelling_service.rs`, `.../api/peer_capacity.rs` · **Test:** `elohim/elohim-storage/tests/` (new integration test)

- [ ] Read the `create_commitment` handler and trace `action='replicates-dwelling'`. Record whether it reaches the specialized writer.
- [ ] **Write the failing test:** POST a well-formed `replicates-dwelling` payload to `/api/v1/commitments`; assert 200, a `rea_commitments` row with `action='replicates-dwelling'`, and `replication_prioritizer::active_commitments_for_provider(provider)` returns it.
- [ ] Run → expect FAIL (no dispatch / no row).
- [ ] Implement: in the handler, on `action='replicates-dwelling'`, fetch `ProviderPledgedState` via `compute_peer_capacity`, then call `create_replicates_dwelling_commitment`. (Skip if already wired.)
- [ ] Run → PASS. Commit.

### Task 2: Seeder authors the 4 commitments

**Files:** `genesis/seeder/src/seed-commitments.ts`, `genesis/seeder/src/doorway-client.ts`

- [ ] Add a `seedReplicatesDwellingPacts()` step that POSTs the 4 rows in the pacts table above (broad `scope_filter` = each hub backs the other's dwelling; `capacity_bytes` a sensible pledge; `ratio_attestation` auto).
- [ ] **Test:** after seeding, `GET /api/v1/hub/household-matthew/capacity` shows pledged dwelling bytes incoming from *both* `household-gertrude` and `household-adam`; `GET /api/v1/hub/household-gertrude/capacity` shows the counter.
- [ ] Run → PASS. Commit.

### Task 3: Implement the `household-resiliency-handshake.feature` step defs

**Files:** `genesis/a2o/features/storage/household-resiliency-handshake.feature` (exists, `@wip`), `genesis/a2o/steps/storage/*.ts` (new)

- [ ] Implement steps: *"Given {family} promises to keep {N} GB … safe"* → POST `replicates-dwelling`; *"Then {family} sees shared content marked protected"* → `GET /api/v1/resilience/{content_id}/household`; *"Then network names them gently"* → `GET /api/v1/diagnostics/mutuality-audit?hub={hub_id}` assert `reciprocity_status`.
- [ ] Scenario "both families reciprocate" → green (`Matched` once Phase 2 lands; until then assert both pledges present).
- [ ] Scenario "one never reciprocates" → tag the *signal* assertion `@wip-until-phase-2` (real `emit_reciprocity_imbalance` is Phase 2); assert `Pending`/`Breached` classification in the audit log meanwhile.
- [ ] Run the feature → green (minus the @wip-until-phase-2 assertion). Commit.

### Task 4: End-to-end resiliency-bar verification

- [ ] Seed → query `/api/v1/hub/{id}/capacity` + `/api/v1/resilience/{content}/household` for the 3 hubs; confirm both pacts visible and pledged bars non-zero each direction.
- [ ] Record the hub-quality replica count for a commons EPR (landing/core) across {Dowell, Gertrude, Adam} as the baseline toward **HA=5** (Phase 4 commons floor closes the gap to 5).

---

## Done when

The two reciprocal pacts are authored, the capacity/resilience bars light for all three hubs, and `household-resiliency-handshake.feature` passes (signal-on-breach assertion deferred to Phase 2). This makes the Sprint-3 mechanism *observable* for the first time — the prerequisite for Phases 2–5.
