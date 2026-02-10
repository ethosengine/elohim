# Modeling Human Agency End-to-End

> **Prerequisite**: [Decoupling sprints 0-9](./decoupling-p2p-cicd-complete.md) (all complete)
> **Context**: [doorway/SCALING.md](../../doorway/SCALING.md) — the dual-axis scaling model

## Vision

Model the full human agency journey in code — from anonymous visitor to community doorway steward. Each sprint makes one stage real, building on the infrastructure from Sprints 0-9 (decoupling, P2P activation, doorway separation, dual scaling axes).

## The 5 Agency Stages

```
Stage 1: VISITOR/BROWSER          ← ALREADY WORKING (projection cache, reach enforcement)
    ↓  creates account
Stage 2: HOSTED HUMAN             ← Sprints 10-11 COMPLETE (conductor pool routing, provisioning)
    ↓  logs in from Tauri, gets identity
Stage 3: TAURI APP USER           ← Sprint 12 COMPLETE (bootstrap + identity handoff)
    ↓  graduates, frees conductor capacity      Sprint 13 NEXT (graduation + conductor retirement)
Stage 4: TAURI + NODE STEWARD     ← Sprint 14 COMPLETE (P2P sync, storage replication)
    ↓  serves community
Stage 5: DOORWAY STEWARD          ← Sprint 15 COMPLETE (federation, DHT registration, cross-doorway routing)
```

Each graduation REDUCES the steward's conductor load while INCREASING the network's capacity.

## What Exists (through Sprint 12)

| Component | Location | Added | Status |
|-----------|----------|-------|--------|
| ConductorRegistry | `doorway/src/conductor/registry.rs` | S9 | Agent→conductor DashMap + MongoDB |
| CONDUCTOR_URLS | `doorway/src/config.rs` | S9 | Parsed to Vec, registered on startup |
| Admin API | `doorway/src/routes/admin_conductors.rs` | S9 | `/admin/conductors`, agents, capacity, hosted-users |
| WorkerPool | `doorway/src/worker/pool.rs` | S9 | Single conductor, round-robin workers |
| JWT Claims | `doorway/src/auth/jwt.rs` | S9 | `agent_pub_key` in every authenticated token |
| Custodial Key Vault | `doorway/src/custodial_keys/` | S9 | Ed25519 + Argon2id + ChaCha20, KeyExportFormat |
| Reach Enforcement | `doorway/src/cache/reach_aware_serving.rs` | S9 | 7-level access control |
| P2P Bootstrap/Signal | `doorway/src/bootstrap/`, `signal/` | S9 | Agent discovery + WebRTC relay |
| Projection Engine | `doorway/src/projection/` | S9 | MongoDB + hot cache, signal subscriber |
| AdminClient | `doorway/src/conductor/admin_client.rs` | S11 | Holochain admin WebSocket + MessagePack |
| AgentProvisioner | `doorway/src/conductor/provisioner.rs` | S11 | provision/deprovision agents on conductors |
| UserDoc.conductor_id | `doorway/src/db/schemas/user.rs` | S11 | Tracks which conductor hosts each user |
| NativeHandoffResponse | `doorway/src/routes/auth_routes.rs` | S12 | Identity + network + key_bundle for Tauri |
| signal_url config | `doorway/src/config.rs` | S12 | Signal relay URL in Args |
| DoorwayClient (Tauri) | `steward/src-tauri/src/doorway.rs` | S12 | HTTP client for login + handoff |
| Identity import (Tauri) | `steward/src-tauri/src/identity.rs` | S12 | Argon2id + ChaCha20 key decryption |
| Tauri IPC commands | `steward/src-tauri/src/lib.rs` | S12 | doorway_login, doorway_status, doorway_logout |
| ElohimSwarm (P2P) | `elohim-node/src/p2p/transport.rs` | S14 | libp2p 0.53 mDNS + Kademlia + request-response |
| SyncEngine | `elohim-node/src/sync/merge.rs` | S14 | Automerge CRDT with SQLite WAL persistence |
| SyncCoordinator | `elohim-node/src/sync/coordinator.rs` | S14 | tokio::select! peer sync orchestration |
| ReplicationPolicy | `elohim-node/src/storage/reach.rs` | S14 | Reach-based replication (family/extended/public) |
| Bootstrap client | `elohim-node/src/network/registration.rs` | S14 | Doorway bootstrap MessagePack client |
| ZomeCaller | `doorway/src/services/zome_caller.rs` | S15 | Generic zome call (single-conn, auth, MessagePack) |
| FederationService | `doorway/src/services/federation.rs` | S15 | DHT registration, heartbeat, cross-doorway fetch |
| Federation routes | `doorway/src/routes/federation.rs` | S15 | /api/v1/federation/doorways, /.well-known/doorway-keys |
| DID real keys | `doorway/src/routes/identity.rs` | S15 | Ed25519 multibase key in DID document |

## Sprint Index

| Sprint | Focus | Agency Stage | Status |
|--------|-------|-------------|--------|
| **10** | **Per-Request Conductor Routing** | Stage 2: Hosted Human | **COMPLETE** |
| **11** | **Dynamic Agent Provisioning** | Stage 2: Hosted Human | **COMPLETE** |
| **12** | **Tauri Bootstrap & Identity Handoff** | Stage 2→3 transition | **COMPLETE** |
| **13** | **Graduation Protocol (Conductor Retirement)** | Stage 3: App User | **NEXT** |
| **14** | **P2P Node Operations** | Stage 4: Node Steward | **COMPLETE** |
| **15** | **Doorway Federation** | Stage 5: Doorway Steward | **COMPLETE** |

## Sprint Dependency Graph

```
Sprint 9 (DONE: ConductorRegistry, CONDUCTOR_URLS, read replicas)
    │
    ▼
Sprint 10: Per-Request Conductor Routing ✓
    │
    ▼
Sprint 11: Dynamic Agent Provisioning ✓
    │
    ▼
Sprint 12: Tauri Bootstrap & Identity Handoff ✓
    │
    ▼
Sprint 13: Graduation Protocol (Conductor Retirement) ← NEXT
    │                \
    ▼                 ▼
Sprint 14:        Sprint 15: Doorway Federation
P2P Node Ops         (also depends on 14)
    │
    ▼
Sprint 15: Doorway Federation
```

---

## Sprint 10: Per-Request Conductor Routing

**Goal**: Wire ConductorRegistry into the request path so every authenticated request routes to the conductor hosting that user's agent, with auto-assignment for unknown agents.

### What Ships

1. **ConductorPoolMap** — `doorway/src/conductor/pool_map.rs` (NEW, ~120 lines)
   - `DashMap<String, Arc<WorkerPool>>` mapping conductor_id → per-conductor pool
   - Default pool fallback for unauthenticated requests
   - `healthy_count()` / `total_count()` for health reporting

2. **ConductorRouter** — `doorway/src/conductor/router.rs` (NEW, ~150 lines)
   - `route(agent_pub_key) → Arc<WorkerPool>`: registry lookup → pool selection
   - Auto-assignment: unknown agents → `find_least_loaded()` → `register_agent()` → pool
   - `default_pool()` for unauthenticated requests

3. **AppState + handle_request routing** — `server/http.rs`, `main.rs`
   - `conductor_router: Option<Arc<ConductorRouter>>` on AppState
   - Per-conductor pool creation at startup from CONDUCTOR_URLS
   - JWT extraction → router.route() in request handler

4. **Health + Admin extensions** — `health.rs`, `admin_conductors.rs`
   - `pools_healthy` / `pools_total` in health response
   - `POST /admin/conductors/assign` for manual agent assignment

### Files

| File | Change | ~Lines |
|------|--------|--------|
| `doorway/src/conductor/pool_map.rs` | NEW | 120 |
| `doorway/src/conductor/router.rs` | NEW | 150 |
| `doorway/src/conductor/mod.rs` | MODIFY | +5 |
| `doorway/src/server/http.rs` | MODIFY | +30 |
| `doorway/src/main.rs` | MODIFY | +25 |
| `doorway/src/routes/health.rs` | MODIFY | +15 |
| `doorway/src/routes/admin_conductors.rs` | MODIFY | +40 |

---

## Sprint 11: Dynamic Agent Provisioning — COMPLETE

**Goal**: Doorway creates hosted users on specific conductors via `generateAgentPubKey` + `installApp` admin calls, completing registration with conductor-aware provisioning.

### What Shipped

1. **AdminClient** — `doorway/src/conductor/admin_client.rs` (NEW, ~490 lines)
   - Holochain admin API WebSocket client (short-lived connections, MessagePack)
   - `generate_agent_pub_key()`, `install_app()`, `enable_app()`, `uninstall_app()`
   - Envelope pattern from `projection/app_auth.rs`

2. **AgentProvisioner** — `doorway/src/conductor/provisioner.rs` (NEW, ~290 lines)
   - `provision_agent(identifier) → ProvisionedAgent` (find_least_loaded → generate_key → install → enable → register)
   - `deprovision_agent(agent_key)` → uninstall + deregister
   - Cleanup on failure (uninstall partially provisioned app)

3. **Registration flow update** — `doorway/src/routes/auth_routes.rs` (MODIFY)
   - `POST /auth/register` provisions agent on conductor, uses conductor-generated key
   - Graceful fallback to custodial key generation if provisioning unavailable

4. **Admin hosted users API** — `doorway/src/routes/admin_conductors.rs` (MODIFY)
   - `POST /admin/hosted-users` — manual provisioning
   - `GET /admin/hosted-users` — list with conductor assignments (MongoDB query)
   - `DELETE /admin/hosted-users/{agent_key}` — deprovision

5. **UserDoc conductor_id** — `doorway/src/db/schemas/user.rs` (MODIFY)
   - Added `conductor_id: Option<String>` field + sparse index

---

## Sprint 12: Tauri Bootstrap & Identity Handoff — COMPLETE

**Goal**: When Tauri starts, a human can login to their doorway, receive network context (bootstrap, signal) and their encrypted agent identity, decrypt it locally, and install the hApp with the same agent key the doorway provisioned. Both conductor cells coexist on the DHT — content syncs via gossip.

### What Shipped

**Doorway side:**

1. **signal_url config** — `doorway/src/config.rs` (MODIFY, +4 lines)
   - Added `signal_url: Option<String>` to Args, env `SIGNAL_URL`

2. **Enhanced NativeHandoffResponse** — `doorway/src/routes/auth_routes.rs` (MODIFY)
   - Added: `agent_pub_key`, `signal_url`, `network_seed`, `installed_app_id`, `conductor_id`, `key_bundle: Option<KeyExportFormat>`
   - Handler now: looks up UserDoc from MongoDB, reads conductor registry, exports key bundle inline (non-destructive)

**Tauri side:**

3. **DoorwayClient** — `steward/src-tauri/src/doorway.rs` (NEW, ~160 lines)
   - `login()` → JWT, `native_handoff(token)` → identity + network context
   - Response types mirror doorway: `LoginResponse`, `NativeHandoffResponse`, `KeyExportFormat`

4. **Identity import** — `steward/src-tauri/src/identity.rs` (NEW, ~170 lines)
   - `decrypt_key_bundle()` — Argon2id (64MB, 3 iter, 4 parallel) + ChaCha20-Poly1305
   - Exact crypto parameter match with `doorway/src/custodial_keys/crypto.rs`

5. **Modified setup() + network_config()** — `steward/src-tauri/src/lib.rs` (MODIFY)
   - `network_config()` checks `doorway.json` store for runtime bootstrap/signal URLs
   - `setup()` reads saved agent key for hApp installation
   - First launch = standalone; doorway URLs take effect after login + restart

6. **Tauri IPC commands** — `steward/src-tauri/src/lib.rs` (MODIFY)
   - `doorway_login(url, identifier, password)` — login → handoff → decrypt key → save to store
   - `doorway_status()` — check connection status, has identity
   - `doorway_logout()` — clear saved credentials
   - Registered with `tauri-plugin-store` for persistence

### Note on Scope Shift
Original Sprint 12 was "Source Chain Migration (Graduation Protocol)". Reframed to prioritize Tauri bootstrap — getting both conductors on the DHT first. Graduation (conductor retirement) moved to Sprint 13.

---

## Sprint 13: Graduation Protocol (Conductor Retirement)

**Goal**: After Tauri confirms DHT presence with the doorway-provisioned agent key, the doorway conductor cell is retired — freeing capacity. The doorway remains as bootstrap/signal/recovery point but no longer holds a conductor cell for graduated users.

### What Ships

1. **GraduationService** — `doorway/src/conductor/graduation.rs` (NEW, ~200 lines)
   - `initiate_graduation(agent_pub_key)` — marks user as graduating
   - `verify_dht_presence(agent_pub_key)` — checks Tauri conductor is live on DHT
   - `complete_graduation(agent_pub_key)` — uninstall_app from doorway conductor, deregister, update UserDoc

2. **Graduation endpoints** — `doorway/src/routes/auth_routes.rs` (MODIFY)
   - `POST /auth/confirm-sovereignty` — Tauri calls this after successful hApp install
   - Triggers: verify DHT presence → uninstall from doorway conductor → mark sovereign
   - Existing `GET /auth/export-key` + `POST /auth/confirm-sovereignty` wired to graduation

3. **Recovery metadata** — `doorway/src/conductor/recovery.rs` (NEW, ~100 lines)
   - After graduation: lightweight MongoDB record (no conductor cell)
   - Recovery relationships, last known bootstrap URL
   - Doorway remains DNS/bootstrap/recovery point

4. **Graduation admin dashboard** — `doorway/src/routes/admin_conductors.rs` (MODIFY)
   - `GET /admin/graduation/pending` — users with Tauri connected but not yet graduated
   - `GET /admin/graduation/completed` — freed capacity metrics
   - `POST /admin/graduation/force/{agent_key}` — force-graduate (admin override)

5. **Tauri graduation command** — `steward/src-tauri/src/lib.rs` (MODIFY)
   - `doorway_confirm_sovereignty()` — calls POST /auth/confirm-sovereignty after hApp install succeeds
   - Auto-triggers on first successful setup() with doorway identity

### Depends On
- Sprint 12 (Tauri has identity handoff, both conductors on DHT)

---

## Sprint 14: P2P Node Operations — COMPLETE

**Goal**: elohim-node orchestrates always-on P2P sync between family nodes with relationship-based storage replication.

### What Shipped

1. **P2P transport layer** — `elohim-node/src/p2p/transport.rs` (~363 lines)
   - `ElohimSwarm` with libp2p 0.53 (mDNS discovery, Kademlia DHT, request-response)
   - `#[derive(NetworkBehaviour)]` composing mDNS + Kademlia + request-response
   - `with_codec()` for SyncCodec registration

2. **Sync codec** — `elohim-node/src/p2p/protocols.rs` (~135 lines)
   - `SyncCodec` with length-prefixed MessagePack framing
   - SyncRequest/SyncResponse types for event log sync

3. **Automerge sync engine** — `elohim-node/src/sync/merge.rs` (~250 lines)
   - `SyncEngine` with SQLite (WAL mode) persistence
   - Automerge CRDT document management for conflict-free sync

4. **Sync coordinator** — `elohim-node/src/sync/coordinator.rs` (~264 lines)
   - `SyncCoordinator` orchestrating peer sync with `tokio::select!`
   - Event-driven: handles swarm events, sync requests, periodic sync triggers

5. **Reach-based replication** — `elohim-node/src/storage/reach.rs` (~113 lines)
   - `ReplicationAction` enum + `replication_policy()` function
   - Family (reach 1-2): full blob replication
   - Extended (reach 3-4): metadata only
   - Public (reach 5+): on-demand fetch
   - 4 tests passing

6. **Doorway bootstrap client** — `elohim-node/src/network/registration.rs` (~261 lines)
   - `bootstrap_put/random/now` using rmpv MessagePack
   - Register with doorway's bootstrap service for peer discovery

7. **Main.rs P2P wiring** — `elohim-node/src/main.rs` (~50 lines)
   - Swarm + engine + coordinator startup integration

**Total**: ~1,171 new lines across 11 files, 9 tests passing (5 merge + 4 reach)

---

## Sprint 15: Doorway Federation — COMPLETE

**Goal**: A community doorway steward runs their own doorway, registers it in the infrastructure DNA's DHT, publishes a DID document with real cryptographic keys, and participates in cross-doorway content routing.

### What Shipped

1. **ZomeCaller service** — `doorway/src/services/zome_caller.rs` (NEW, ~280 lines)
   - Generic zome call mechanism following ImportClient's single-connection pattern
   - Auth flow: `issue_app_token()` → connect with token → CallZome envelope (MessagePack)
   - Typed `call<I, O>()` wrapper with rmp_serde serialization
   - Lazy connection init with RwLock + Mutex double-check pattern

2. **Federation service** — `doorway/src/services/federation.rs` (NEW, ~370 lines)
   - `FederationConfig::from_args()` — returns None if doorway_id/url not configured
   - `register_doorway_in_dht()` — registers in infrastructure DNA, falls back to update on "already exists"
   - `spawn_heartbeat_task()` — periodic 60s heartbeat with live metrics from AppState
   - `fetch_from_remote_doorway()` — DHT publisher discovery → DID resolution → HTTP fetch
   - `get_all_doorways()` — query registered doorways for API endpoint
   - Input types match infrastructure zome structs exactly

3. **Federation routes** — `doorway/src/routes/federation.rs` (NEW, ~200 lines)
   - `GET /api/v1/federation/doorways` — list known doorways from DHT (fallback: self-only)
   - `GET /.well-known/doorway-keys` — Ed25519 public key in JWKS format (OKP/Ed25519)
   - Base64url encoding for JWK "x" parameter

4. **DID document real keys** — `doorway/src/routes/identity.rs` (MODIFY)
   - `public_key_multibase`: Ed25519 multicodec prefix (0xed01) + base58btc z-prefix encoding
   - `elohim_holochain_cell_id`: populated from zome_configs DashMap if infrastructure cell discovered

5. **AppState extensions** — `doorway/src/server/http.rs` (MODIFY)
   - `node_verifying_key: Option<ed25519_dalek::VerifyingKey>` on AppState
   - `zome_caller: Option<Arc<ZomeCaller>>` shared by federation + routes

6. **Startup wiring** — `doorway/src/main.rs` (MODIFY)
   - Node Ed25519 key generation via `custodial_keys::crypto::generate_keypair()`
   - ZomeCaller creation (admin + app URLs from config)
   - Federation registration task (5s delay for conductor readiness)
   - Heartbeat task spawn with federation config

7. **Module declarations** — `services/mod.rs`, `routes/mod.rs` (MODIFY)
   - `pub mod federation`, `pub mod zome_caller` + re-exports

### Files

| File | Action | ~Lines |
|------|--------|--------|
| `doorway/src/services/zome_caller.rs` | NEW | 280 |
| `doorway/src/services/federation.rs` | NEW | 370 |
| `doorway/src/routes/federation.rs` | NEW | 200 |
| `doorway/src/routes/identity.rs` | MODIFY | +10 |
| `doorway/src/server/http.rs` | MODIFY | +25 |
| `doorway/src/main.rs` | MODIFY | +40 |
| `doorway/src/services/mod.rs` | MODIFY | +5 |
| `doorway/src/routes/mod.rs` | MODIFY | +5 |
| `doorway/Cargo.toml` | MODIFY | +1 (bs58) |
| **Total** | | **~936** |

---

## Architecture After All Sprints

```
VISITOR (Stage 1 — WORKING)
    Browser → Ingress → Doorway → MongoDB projection → response
    No JWT, no conductor, no identity needed

HOSTED HUMAN (Stage 2 — COMPLETE: Sprints 10-11)
    Browser → Doorway → JWT → ConductorRouter
                                  │
                                  ├─ Registry lookup: agent → conductor-2
                                  └─ WorkerPool[conductor-2] → Conductor-2
    Registration: AgentProvisioner → least-loaded conductor → install_app

TAURI + DOORWAY DUAL-WRITE (Stage 2→3 — COMPLETE: Sprint 12)
    Tauri App → doorway_login → NativeHandoffResponse (identity + network + key_bundle)
            → decrypt key → install hApp with same agent key
            → both conductors on DHT, gossip syncs content
    Doorway conductor cell STILL ACTIVE (dual-write mode)

TAURI SOVEREIGN (Stage 3 — Sprint 13: graduation)
    Tauri confirms DHT presence → doorway retires conductor cell
    Doorway keeps: bootstrap/signal/recovery/projection
    Frees: conductor capacity for new hosted humans

NODE STEWARD (Stage 4 — Sprint 14)
    elohim-node → P2P sync → family cluster replication
    Always-on: participates in DHT, stores shards for family
    Recovery: custodian for other family members

DOORWAY STEWARD (Stage 5 — Sprint 15)
    Community doorway → DID registration → federation
    Hosts users, serves projection, bootstraps agents
    Cross-doorway: content routing, recovery coordination
```
