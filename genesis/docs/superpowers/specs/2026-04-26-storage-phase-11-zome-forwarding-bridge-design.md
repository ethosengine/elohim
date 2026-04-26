# Storage Phase 11 — Zome-Forwarding Bridge

**Status:** Design approved (2026-04-26)
**Branch:** `feature/storage-phase-11-zome-forwarding-bridge`
**Cuts from:** `dev` @ `b3cdd753` (M5 merge `a07aaa66` + rustfmt fix)
**Predecessors:** Recovery Protocol Phase 2 — M5 (auth portal convergence + revocation UX + stub defender)
**Successor:** M6 (real elohim-defender + browser-mode write path)

## Why this exists

M5 landed the four `/api/v1/account/*` write endpoints as `503 PHASE_11_PENDING` stubs, with five `@phase11-pending` a2o scenarios pinned to those stubs. The Angular UI surfaces "coming soon" instead of writing through to Holochain. Phase 11 connects the existing `HcClient` to those four routes so the stubs become real working endpoints — with the trust model that matches the protocol.

The trust framing matters: a human using a browser via doorway does not necessarily trust the doorway operator. A self-revocation, a recovery vote, or a portal-host registration is a self-sovereign act. Those writes belong on a peer the human chose to trust — their own device. Phase 11 ships exactly that surface (Tauri-direct mode) and surfaces a clear deferral for browser-via-doorway until M6 settles where browser-mode writes safely execute.

## What already exists (do not rebuild)

- `HcClient` at `elohim/elohim-storage/src/hc_client.rs` — wraps `holochain_client::AppWebsocket`, binds to one `CellId` at connect time, exposes `call_zome(zome_name, fn_name, payload: Vec<u8>) -> Result<Vec<u8>>`.
- `HcClient` instantiation precedent at `main.rs:468` for `role: "infrastructure"` (the heartbeat path). Connect failure is logged and non-fatal — the node keeps serving HTTP.
- All four 503 stubs route through one shared handler `zome_bridge_not_yet_wired` at `account.rs:495`. Routes dispatched at `account.rs:55-71`.
- `extract_agent_key()` at `account.rs:520` resolves the calling human via `X-Agent-Id` header (doorway-injected) or active local session (Tauri).
- M5 Task 10 fully deserializes each handler's HTTP `InputView`. Wire-shape work is done.
- imagodei zome name: `"imagodei"`. Function names confirmed by reading `elohim/holochain/dna/imagodei/zomes/imagodei/src/`:
  - `create_self_revocation(input: CreateSelfRevocationInput) -> KeyRevocationOutput` — `lib.rs:1939`
  - `submit_revocation_vote(input: SubmitRevocationVoteInput) -> RevocationVoteOutput` — `lib.rs:2212`
  - `add_portal_host(input: AddPortalHostInput) -> ActionHash` — `portal_host.rs:100`
  - `remove_portal_host(host_url: String) -> ()` — `portal_host.rs:135`
- All four functions read `agent_info()?.agent_initial_pubkey` at the top — confirming the call must execute in the human's cell.

## The architectural question this spec answers

Storage's existing HcClient signs as the cell-owner of whatever cell it is connected to. When the conductor signs a zome call with admin-issued signing credentials, the call appears to the zome as coming from the cell's owner agent — signing credentials authorize storage to call on the cell's behalf, they do **not** swap the caller identity. So `agent_info()?.agent_initial_pubkey` returns the cell's owner.

Two deployment modes constrain what storage can do:

| Mode | Conductor location | Cell ownership | Storage can forward writes? |
|------|-------------------|----------------|----------------------------|
| **Tauri-direct** | Local sidecar on the human's device | The human owns the cell | **Yes** — `agent_info()` returns the human |
| **Browser-via-doorway** | Hosted somewhere the storage process does not own | Some other peer's cell | **No** — storage has no signing relationship with the human's cell |

Three approaches were weighed in brainstorm:

- **(a)** Per-cell HcClient pool: rejected. To forward writes for arbitrary humans in browser mode, storage would need to hold every human's signing credentials. That inverts the recovery model where the human's key is on their device, not in custody of a peer they happen to be hitting.
- **(b)** Tauri-only now, browser-mode later: **chosen.** Matches the trust model (the human's writes execute on a peer they chose). Smallest blast radius. Gets the four endpoints real for Tauri stewards immediately. Defers the harder browser-mode question to M6 where the hosting model is being designed anyway.
- **(c)** Move write path off storage entirely (doorway → conductor direct): rejected. Conflicts with `elohim-storage/CLAUDE.md`'s "single HTTP API boundary" and would split the API surface across two crates with no clean reunification path.

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│  elohim-storage process (Tauri sidecar OR peer node)       │
│                                                            │
│  ┌──────────────┐   ┌────────────────────────────────────┐ │
│  │ http handlers│   │ HcClientRegistry  (NEW)            │ │
│  │ /api/v1/     │──▶│   infrastructure: Option<Arc<Hc>>  │ │
│  │ account/*    │   │   imagodei:      Option<Arc<Hc>>   │ │
│  └──────────────┘   └─────────────┬──────────────────────┘ │
│                                   │ call_zome(...)         │
└───────────────────────────────────┼────────────────────────┘
                                    ▼
                       ┌────────────────────────┐
                       │ embedded conductor     │
                       │ (Tauri sidecar OR peer)│
                       │  cells: imagodei,      │
                       │  lamad, infrastructure │
                       └────────────────────────┘
```

`HcClientRegistry` is a thin in-process holder of role-keyed `Option<Arc<HcClient>>` connections. Each role-specific HcClient connects independently at startup; failures are logged and non-fatal (consistent with the heartbeat precedent at `main.rs:553`).

## New surface

### `hc_client_registry.rs` (new, ~80 lines)

```rust
pub struct HcClientRegistry {
    pub infrastructure: Option<Arc<HcClient>>,
    pub imagodei: Option<Arc<HcClient>>,
}

impl HcClientRegistry {
    pub async fn connect(args: &Args) -> Self {
        let infrastructure = Self::connect_role(args, "infrastructure").await;
        let imagodei = Self::connect_role(args, "imagodei").await;
        Self { infrastructure, imagodei }
    }

    async fn connect_role(args: &Args, role: &str) -> Option<Arc<HcClient>> {
        let admin_url = args.admin_url.clone()?;
        match HcClient::connect(HcClientConfig {
            admin_url,
            app_url: args.app_url.clone(),
            app_id: args.app_id.clone(),
            role: Some(role.to_string()),
        }).await {
            Ok(hc) => {
                info!(role, "HcClient connected");
                Some(Arc::new(hc))
            }
            Err(e) => {
                warn!(role, error = %e, "HcClient connect failed — routes for this role will return 503");
                None
            }
        }
    }
}
```

The existing `main.rs:468` heartbeat block is refactored to consume `registry.infrastructure` instead of constructing its own HcClient inline. No behavior change for the heartbeat path.

### Forwarder helpers in `account.rs`

One async helper per route, each ~30-40 lines:

```rust
async fn forward_self_revocation(
    body: Bytes,
    hc: &HcClient,
    agent_key: &str,
) -> Result<Response<Full<Bytes>>, StorageError>;

async fn forward_revocation_vote(
    body: Bytes,
    hc: &HcClient,
    agent_key: &str,
    revocation_id: &str,
) -> Result<Response<Full<Bytes>>, StorageError>;

async fn forward_add_portal_host(
    body: Bytes,
    hc: &HcClient,
    agent_key: &str,
) -> Result<Response<Full<Bytes>>, StorageError>;

async fn forward_remove_portal_host(
    hc: &HcClient,
    agent_key: &str,
    url_b64: &str,
) -> Result<Response<Full<Bytes>>, StorageError>;
```

Each helper:
1. Calls `verify_caller_owns_cell(hc, agent_key)` — returns 503 immediately if mismatch (browser-mode).
2. Deserializes the request body into the existing M5 `InputView` types.
3. Translates `InputView` → zome input via existing `From` impls, MessagePack-encodes the payload.
4. Calls `hc.call_zome("imagodei", "<fn>", payload).await`.
5. On `Ok(bytes)`: MessagePack-decodes into the response View type and emits `200 OK` (or `201 Created` for resource-creation routes — `add_portal_host` and `create_self_revocation`).
6. On `Err(StorageError)`: hands off to `map_zome_err_to_http`.

### Mode gate

```rust
fn verify_caller_owns_cell(
    hc: &HcClient,
    agent_key: &str,
) -> Result<(), Response<Full<Bytes>>> {
    let cell_owner = hex::encode(hc.cell_id().agent_pubkey().get_raw_39());
    if cell_owner != agent_key {
        return Err(response_503_browser_write_path_pending());
    }
    Ok(())
}
```

Returns the new 503 contract on mismatch:

```json
{
  "error": "browser write path not yet implemented",
  "code": "BROWSER_WRITE_PATH_PENDING",
  "message": "Self-sovereign writes require a peer the human controls. \
              The browser-via-doorway write path is deferred to M6 where \
              the hosting trust model is settled."
}
```

### Two distinct 503 contracts

Phase 11 introduces two 503 codes, replacing the single M5 `PHASE_11_PENDING`:

| Code | Trigger | Recovery |
|------|---------|----------|
| `BROWSER_WRITE_PATH_PENDING` | Caller's agent key does not match the connected cell's owner (browser-via-doorway path) | Defer to M6 — no in-Phase-11 fix |
| `IMAGODEI_BRIDGE_OFFLINE` | Imagodei HcClient failed to connect at startup (e.g., DNA not installed in dev flow) | Restart storage with the imagodei DNA available |

Both responses share the same JSON envelope shape (`error`, `code`, `message`) and only differ in `code` + `message`. Angular layer should switch on `code`.

The Tauri-direct invariant is asserted (cell-owner matches the resolved human key), and the `extract_agent_key` heuristic is unchanged from M5 — header-or-session, no new logic.

### Route dispatch update

The four route arms in `account.rs:55-71` change from:

```rust
(Method::POST, "/self-revocation") => Ok(zome_bridge_not_yet_wired("self-revocation")),
```

to:

```rust
(Method::POST, "/self-revocation") => {
    handle_self_revocation(req, registry.imagodei.as_deref(), pool).await
}
```

`handle_self_revocation` is a thin wrapper: resolves agent key (via existing `extract_agent_key`), checks `Some(hc)` (returns `IMAGODEI_BRIDGE_OFFLINE` 503 if `None`), reads body, calls `forward_self_revocation`. Same shape for the other three routes.

## Error mapping

One shared function in `account.rs`:

```rust
fn map_zome_err_to_http(err: &StorageError) -> Response<Full<Bytes>> {
    let msg = err.to_string();

    // 403 — gate rejections (defender, EC, ownership)
    if msg.contains("not a configured defender")
        || msg.contains("not an active emergency contact")
        || msg.contains("does not control")
        || msg.contains("does not belong to") {
        return response::forbidden(&msg);
    }

    // 400 — input validation
    if msg.contains("invalid reason")
        || msg.contains("already effective")
        || msg.contains("votes not accepted")
        || msg.contains("attestation cannot be empty")
        || msg.contains("no KeyRevocation with id") {
        return response::bad_request(&msg);
    }

    // 503 — conductor connectivity
    if msg.contains("Connection") || msg.contains("Conductor") {
        return response::service_unavailable(&msg);
    }

    response::internal_server_error(&msg)
}
```

String-matching on well-known zome error prefixes is acknowledged as brittle. It matches what the zome actually returns today; typed errors over the wire are a separate refactor (M6+). Documented in code with a `// PHASE-11-DEBT:` marker so the next maintainer sees the deferral intent.

## Response timing — eventual consistency

Handlers respond on zome `Ok(_)` immediately. Projection-signal echo into SQLite is in-process (sub-millisecond signal stream) but the HTTP handler does not wait on it. Rationale:

- Coupling HTTP write to projection upsert would slow every request by the projection's worst-case latency.
- The Angular layer's signal-driven UI from M5 already handles "list refreshes shortly after submit."
- Eventual-consistency semantics here match the rest of the protocol — the DHT entry is committed on `Ok`; the SQLite projection is a fast index, not a source of truth (`elohim/elohim-storage/CLAUDE.md`).

## Testing

### Unit tests in `account.rs`

**Removed:**
- `zome_bridge_stub_returns_503` — the stub no longer exists.
- `zome_bridge_all_stub_routes_return_503` — same.

**Added:**
- `verify_caller_owns_cell_passes_when_keys_match` — synthesizes an `HcClient`-shaped fixture (or a trait if `HcClient` is hard to mock) and confirms the gate returns `Ok(())`.
- `verify_caller_owns_cell_returns_503_browser_pending_on_mismatch` — confirms the 503 body has `code: "BROWSER_WRITE_PATH_PENDING"`.
- `map_zome_err_to_http_403_for_gate_rejection`
- `map_zome_err_to_http_400_for_invalid_input`
- `map_zome_err_to_http_503_for_connection_error`
- `map_zome_err_to_http_500_for_unknown`

**Mocking strategy — committed:** introduce a small `trait CellOwner { fn agent_key_hex(&self) -> String; }` that `HcClient` implements via its existing `cell_id().agent_pubkey()` accessor, and have `verify_caller_owns_cell` take `&dyn CellOwner` instead of `&HcClient`. Tests use a hand-rolled stub. This is a strict isolation improvement — the gate function only needs the cell-owner key, no other HcClient capability — and removes the question of whether to mock the websocket layer.

### Integration / live verification

No live-conductor integration tests in this sprint. Mocking `holochain_client::AppWebsocket` is heavy and the real verification surface is the four a2o scenarios that flip from `@phase11-pending` to passing in CI.

Per `feedback_shift_measure_jenkins` (Eclipse Che lacks the runtime), pre-push runs:

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cargo test --lib
```

Live a2o verification happens on Jenkins.

### A2O tag removal

Five `@phase11-pending` tags across four feature files (kickoff §4):

| File | Tag count |
|------|-----------|
| `genesis/a2o/features/auth/recovery/recovery-m5-self-revoke.feature` | 1 |
| `genesis/a2o/features/auth/recovery/recovery-m5-vote-as-emergency-contact.feature` | 2 |
| `genesis/a2o/features/auth/recovery/recovery-m5-portal-host-discovery.feature` | 1 |
| `genesis/a2o/features/auth/recovery/recovery-m5-defender-role-gate.feature` | 1 |

Step definitions in `genesis/a2o/steps/ui/account-m5.steps.ts` already drive the live HTTP path; no step changes expected. If a step fails because it asserts the 503 message specifically, that's a step-update task to capture in plan execution.

## Out of scope

- **Defender path (`submit_specialist_revocation`)** — kickoff confirms only the four account routes. Defender wiring (gate-client bridge from elohim-defender → zome) is M6.
- **Browser-mode write path** — Phase 11 ships the clearer 503 contract change. Actual browser-mode forwarding is a separate sprint after M6's hosting-model decisions.
- **HA / connection pool / reconnect** — single HcClient connection per role; if it drops, the registry holds the existing `Arc` and zome calls fail with a connectivity error mapped to 503 by `map_zome_err_to_http`. Reconnect logic is M6+.
- **Hashcash / rate limiting on POST** — M6+.
- **Typed error contracts over the conductor wire** — M6+; Phase 11 lives with string-matching.
- **Schema-first wire types** — no new types cross any boundary in this sprint. The four zome inputs/outputs are existing structures; the four HTTP InputViews already exist from M5. Per `feedback_schema_first_ioc`, no schema work is required because no contract is being introduced.

## P2P design gate

Runs trivially. No new entities. The `HcClientRegistry` is **Category C — operational** (in-process connection cache, not on the DHT, not signed, not federated). No new content addressing. No new identity surface. No new sync-message types.

The four zome functions and their entries (`KeyRevocation`, `RevocationVote`, `PortalHost`) are pre-existing and were classified during M3/M4/M5.

## Memory references

- `feedback_serde_json_value_breaks_zome_boundary` — pre-stringification convention. Phase 11 does not unstub `submit_specialist_revocation`, so this only matters when that path lands in M6. The four Phase 11 routes don't carry `Value` payloads.
- `project_m5_is_plumbing_sprint` — Phase 11 inherits the same plumbing-not-polish ethos.
- `feedback_a2o_is_human_experience_not_dev_bugs` — no new feature files; only `@phase11-pending` tag removal.
- `project_three_layer_truth_model` — supports the rejection of approach (c). Doorway is the web2 projection, not a P2P participant; the write path lives on the storage spine.
- `project_principle_p1_reconciliation_controller` — storage as actuator: Phase 11 makes storage actuate the DHT writes that M5 only described.
- `feedback_schema_first_ioc` — no new wire contracts; not invoked.
- `feedback_dev_branch_no_pr` — feature → dev = local merge.
- `feedback_swarm_composition_fresh_tree_build` — HcClient is conductor-side, not swarm — no fresh-tree build prerequisite, but pre-push still runs the full crate build.
- `feedback_subagent_scope_guardrails` — plan execution dispatches must explicitly forbid `git revert/reset` on pre-existing commits and require BLOCKED reports instead of silent cleanup.

## Acceptance

Phase 11 is done when:

1. `feature/storage-phase-11-zome-forwarding-bridge` cleanly merges into `dev`.
2. Pre-push gate passes locally (`cargo build`, `cargo clippy -- -D warnings`, `cargo test --lib`, `cargo fmt --check`).
3. The four `/api/v1/account/*` routes call the imagodei coordinator zome via the registered HcClient when the cell-owner matches the resolved agent key.
4. The four routes return `503 BROWSER_WRITE_PATH_PENDING` when the resolved agent key does not match the connected cell's owner.
5. The four routes return `503 IMAGODEI_BRIDGE_OFFLINE` when the imagodei HcClient failed to connect at startup.
6. The five `@phase11-pending` tags are removed and the corresponding scenarios pass on Jenkins (this is the live verification surface for #3 above).
7. The two stub unit tests in `account.rs` are removed and the new mode-gate + error-mapping tests pass.

## Risk register

| Risk | Likelihood | Mitigation |
|------|-----------|-----------|
| imagodei DNA installed under a different `app_id` than `infrastructure` | Low-Mid | Verify during plan execution by reading conductor config / `Args` handling. If different, add `--imagodei-app-id` CLI arg; one-line registry change. |
| Mocking HcClient for unit tests is too heavy | Mid | Introduce `trait CellOwner` (small refactor), test against the trait. Spec already calls this out. |
| Step definitions assert 503 message text and break when the route returns 200/201 | Low-Mid | Plan task explicitly inspects each step before tag removal; updates assertions if needed. |
| `agent_info()?.agent_initial_pubkey` does NOT in fact equal the cell's owner under signing credentials | Low | If verification during plan execution shows otherwise, the spec's central premise fails and we BLOCK back for re-scoping. The brainstorm reviewed `holochain_client` source enough to be confident, but a runtime probe in the plan's first task verifies. |
| `cargo test --lib` runs zero conductor — no end-to-end coverage in pre-push | High (intended) | Acknowledged. Live verification is a2o on Jenkins. |
