I want to plan and implement **elohim-storage Phase 11 — wire the four account 503 stubs to the existing HcClient**. Once it lands, the four `503 PHASE_11_PENDING` routes from M5 (and the five `@phase11-pending` a2o scenarios) become real working endpoints.

> **Revision note (2026-04-26):** the original kickoff scoped this as "build HcClient from scratch." Investigation post-M5-merge showed `HcClient` already exists at `elohim/elohim-storage/src/hc_client.rs` and is used by `heartbeat.rs`, `import_handler.rs`, `node_registry_api.rs`, `content_server.rs`, `main.rs:468`. Scope shrunk and refocused on the **multi-tenant cell-routing question** that the existing HcClient does NOT yet solve.

## Context (self-contained)

Recovery Protocol Phase 2 finished M5 (auth portal convergence + revocation UX + stub defender). M5 is **merged to dev** as of `a07aaa66`. Phase 11 cuts a fresh branch from current `dev` HEAD.

### What already exists (do NOT rebuild)

- `HcClient` at `elohim-storage/src/hc_client.rs` — wraps `holochain_client::AppWebsocket` with admin-issued signing credentials. Method: `call_zome(zome_name, fn_name, payload: Vec<u8>) -> Result<Vec<u8>>` (MessagePack-encoded bytes both directions).
- `HcClient` instantiation pattern at `main.rs:429-555` (the infrastructure heartbeat path).
- All four 503 stubs route through ONE shared handler at `account.rs:506` (`zome_bridge_not_yet_wired`). The four routes are dispatched at `account.rs:55-71`.
- `extract_agent_key()` at `account.rs:534-552` already resolves the calling human (X-Agent-Id header from doorway JWT, or active local session for Tauri).
- Each handler's HTTP InputView is fully deserialized in M5 Task 10. The wire-shape work is done.
- imagodei coordinator zome name: `"imagodei"` (verified during M5).
- For `submit_specialist_revocation`: zome wants `anomaly_attestation_json: String`, NOT structured Value. Pre-stringify before forwarding (memory: `feedback_serde_json_value_breaks_zome_boundary`).

### The four routes to unstub

```
POST   /api/v1/account/self-revocation       → imagodei::create_self_revocation
POST   /api/v1/account/recovery/:id/vote     → imagodei::submit_revocation_vote
POST   /api/v1/account/portal-hosts          → imagodei::add_portal_host
DELETE /api/v1/account/portal-hosts/:url_b64 → imagodei::remove_portal_host
```

### THE central question (what brainstorming must resolve)

**Storage's existing HcClient signs as ITSELF, not as the calling human.**

All existing HcClient consumers (`heartbeat`, `import_handler`, `node_registry_api`, `content_server`) are storage acting on its own behalf — service-bot pattern. Phase 11 introduces a NEW pattern: **storage forwarding a zome call ON BEHALF OF a human**, where the imagodei zome's `agent_info()?.agent_initial_pubkey` MUST equal the human's key (otherwise self-revocation semantics break — "I revoked my own key" requires the zome to see the human as caller).

Two deployment modes constrain the answer:

| Mode | Conductor | Cells | Can storage's existing HcClient work? |
|------|-----------|-------|---------------------------------------|
| **Tauri-direct** | Local sidecar | ONE (the human's) | Likely yes — provenance defaults to the cell's owner agent. Verify. |
| **Browser-via-doorway** | Hosted, multi-tenant | MANY (one per human) | No — storage has no credentials for arbitrary humans' cells. |

Three candidate approaches to weigh in brainstorm:

a) **Per-cell HcClient pool in storage.** Maintain a `HashMap<AgentPubKey, HcClient>`. On first call for a human, attach app interface, authorize signing credentials, cache the HcClient. Browser+doorway compatible. Storage holds many auth tokens.

b) **Phase-11A Tauri-only, Phase-11B browser later.** Tauri uses the existing single-cell HcClient (verify provenance behavior). Browser writes get a 503 for now with a clearer "browser write path not yet implemented" message. Smaller blast radius; gets the auth loop demonstrable for Tauri stewards immediately.

c) **Move the write path off storage entirely.** Doorway already has a conductor connection (projection subscriber). Browser writes go doorway → conductor directly; storage only handles GETs. Re-routes the four POST/DELETE endpoints to live on doorway, not storage. Conflicts with the "one HTTP API" boundary in `elohim/elohim-storage/CLAUDE.md`.

Brainstorm should pick a primary direction (likely b → a graduation, but let the gate process surface the answer).

### Other gaps to resolve in brainstorm

1. **Provenance behavior in Tauri-mode** — when storage's HcClient calls a zome on a single-cell conductor, does `agent_info()` in the zome return the human's pubkey or storage's signing credential pubkey? Verify experimentally or by reading holochain_client source. This determines whether option (b) works at all.
2. **Error-to-HTTP mapping** — gate rejection → 403, validator rejection → 400, system errors → 500. Define one shared mapping function for all four handlers.
3. **Projection-signal echo timing** — when self-revocation commits, post_commit emits a signal, projector upserts. HTTP handler can either (i) wait briefly for projection update before responding (consistent reads), or (ii) respond immediately on zome `Ok` (faster, eventual consistency). Pick one.
4. **InputView ↔ zome-input shape** — for `submit_specialist_revocation`, where does the `anomalyAttestation: Value` → `anomaly_attestation_json: String` conversion happen — in the InputView's `From` impl, or inline in the handler? Pick the convention that generalizes.

Run `p2p-design-gate` early — Phase 11 introduces no new entities (the `HcClient` per-cell pool is Category C operational), so the gate runs quickly.

## Scope

### In scope

1. Pick the deployment-mode answer (a / b / c above).
2. If new types (e.g., per-cell pool) are needed, write them under `elohim-storage/src/services/` (or extend `hc_client.rs`).
3. Replace `zome_bridge_not_yet_wired` with real forwarding for the four routes. Each handler:
   - Resolves agent (already done via `extract_agent_key`).
   - Routes the call to the right cell (per the chosen approach).
   - Translates HTTP InputView → zome input (using existing `From` impls; `anomaly_attestation_json` requires pre-stringify).
   - Maps zome `Err` → HTTP per the agreed mapping.
   - Maps zome `Ok` → HTTP 200/201 with the entity's projection (after deciding (3) above on echo timing).
4. Remove `@phase11-pending` tags from:
   - `genesis/a2o/features/auth/recovery/recovery-m5-self-revoke.feature` (1 tag)
   - `genesis/a2o/features/auth/recovery/recovery-m5-vote-as-emergency-contact.feature` (2 tags)
   - `genesis/a2o/features/auth/recovery/recovery-m5-portal-host-discovery.feature` (1 tag)
   - `genesis/a2o/features/auth/recovery/recovery-m5-defender-role-gate.feature` (1 tag)
   The step definitions in `genesis/a2o/steps/ui/account-m5.steps.ts` should already work once the bridge is real.
5. Delete the `zome_bridge_stub_returns_503` and `zome_bridge_all_stub_routes_return_503` unit tests in `account.rs` (they test the stub that no longer exists).

### Out of scope (deferred)

- Real elohim-defender detection — M6+. Phase 11 unstubs the `submit_specialist_revocation` HTTP path; the defender STILL doesn't actually call it (gate-client bridge is also stubbed).
- Hashcash / rate limiting on POST endpoints — M6+.
- Conductor-connection HA / failover — Phase 11 ships single-connection; HA pool is later.
- If approach (b) is picked: the browser write path (option a graduation) — separate sprint.

## How to run this session

1. Sanity-check at session start (5 min):
   - [ ] On a fresh branch off `dev` HEAD (currently `a07aaa66`).
   - [ ] Read `elohim-storage/src/api/account.rs` lines 1-80 (route dispatch) and lines 480-640 (stub + tests).
   - [ ] Read `elohim-storage/src/hc_client.rs` lines 100-260 (connect + call_zome).
   - [ ] Read `elohim-storage/src/main.rs` lines 425-560 (existing HcClient instantiation pattern).
   - [ ] Confirm zome function names by reading `elohim/holochain/dna/imagodei/zomes/imagodei/src/{portal_host,submit_specialist_revocation}.rs`.

2. Invoke `/superpowers:brainstorming` on Phase 11 with focus on the central question + 4 gaps. Run `p2p-design-gate` as part of step 3.

3. Once approach is locked, invoke `/superpowers:writing-plans` to produce:
   - Design spec: `genesis/docs/superpowers/specs/2026-04-26-storage-phase-11-zome-forwarding-bridge-design.md`
   - Plan: `genesis/docs/superpowers/plans/2026-04-26-storage-phase-11-zome-forwarding-bridge.md`

4. Execute via `/superpowers:subagent-driven-development`. Likely 4-6 tasks: provenance probe (option b verify), bridge wiring per approach, four endpoint un-stubs (likely grouped), a2o tag removal + stub-test removal.

5. Pre-push gate locally before pushing. Memory `feedback_swarm_composition_fresh_tree_build` — fresh-tree build before commit if anything touches the swarm composition (HcClient is conductor-side, not swarm — likely fine, but verify).

## Constraints & conventions

- Working branch: cut `feature/storage-phase-11-zome-forwarding-bridge` from `dev` HEAD `a07aaa66`.
- Build commands:
  - elohim-storage: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release && cargo test --lib`
  - imagodei DNA (no changes expected): `cd /projects/elohim/elohim/holochain/dna/imagodei && just check`
  - sweettest verification: CI only (Eclipse Che lacks the env per memory `feedback_shift_measure_jenkins`).
- Schema-first IoC per `feedback_schema_first_ioc` — if any new wire types emerge, schemas first.
- Subagent dispatches MUST include explicit scope guardrails per `feedback_subagent_scope_guardrails`. Orchestrator scans SHA range post-dispatch.
- Pre-push gate runs `cargo fmt + clippy + tests`. ~25-30 minutes on cold cache.
- Per `feedback_a2o_is_human_experience_not_dev_bugs` — Phase 11 is dev plumbing. Don't write new feature files; just remove the @phase11-pending tags.

## Memories worth checking on start

- `feedback_serde_json_value_breaks_zome_boundary` — pre-stringify convention for structured payloads.
- `project_m5_is_plumbing_sprint` — keeps Phase 11 narrow.
- `feedback_a2o_is_human_experience_not_dev_bugs` — minimal a2o footprint.
- `project_three_layer_truth_model` — informs option (c) tradeoff.
- `project_principle_p1_reconciliation_controller` — storage as actuator, not just observer.
- `feedback_schema_first_ioc` — schemas first if new types cross boundaries.
- `feedback_dev_branch_no_pr` — feature → dev = local merge.

Go.
