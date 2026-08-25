---
id: doorway-service-gospel
cites:
  - "resilience-protocol-spec | the resilience protocol + account-recovery canon (Parts V/VI) this gateway implements as the web2 projection — patron-CDN, social-recovery, creator-succession | sha256:2c832b517c7204cc | status: stale — target content moved on; re-verify | path: genesis/docs/content/elohim-protocol/resilience/README.md"
  - "elohim-seam-map-concern-routing | the concern-routing atlas — this surface owns the Doorway projection seam (§3.9, Track 4); routes any where-does-this-go? question | sha256:7fd48274fae5e8c5 | status: stale — target content moved on; re-verify | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "doorway-auth-posture-declared-stage | the auth-posture entrypoint for this crate — the declared-stage rule, why DEV_MODE is never a posture, the API_KEY_ADMIN vs API_KEY_SEED distinction, the chaperone exception, and the six questions to answer before adding any gate here | sha256:31cda806d3207fd9 | path: genesis/docs/content/elohim-protocol/architecture/2026-08-25-doorway-auth-posture-declared-stage.md"
  - "doorway-auth-refusal-runbook | the symptom-side companion for this crate — what to do when a doorway refusal actually happens, including the decision tree that stops a 403 on the write path being filed as a dataplane divergence | sha256:0929079216f0c37d | path: genesis/docs/content/elohim-protocol/architecture/2026-08-25-doorway-auth-refusal-runbook.md"
---

# Doorway Service (Rust)

doorway-service is the Rust implementation of the web2 projection of the elohim substrate — it makes the patron-CDN, the storage-stewardship surface, and the social-recovery flows reachable from the traditional internet. This file is the Rust-implementation orientation for the crate; architecture, trust model, routing model, and the no-per-domain-proxy-files discipline live in `../CLAUDE.md`. The deeper "why" — patron-CDN, account-takeover-recovery, creator-succession — lives in `../../genesis/docs/content/elohim-protocol/resilience/README.md` (Parts V and VI).

## Seam map — you are here

This surface owns the **Doorway projection** seam (atlas §3.9, Track 4 — make canonical substrate truth legible to browsers and the web2 world: HTTP, OAuth relying-party, manifest-driven routes, single-target proxy + cache, doorway-to-doorway federation).

Any "where does this go?" concern routes through the concern-routing atlas: `elohim-seam-map-concern-routing`.

Confusion-to-avoid: hub ≠ doorway — the doorway projects **outward** to web2 and is **not a P2P participant**; the hub projects inward to nearby peers (§3.12, `steward/node`).

## Auth posture — read BEFORE touching any gate

**A doorway's auth posture derives from the network's DECLARED operating stage, never from a mode flag.**
The stage is `ELOHIM_NETWORK_STAKES`, resolved once at boot into `AppState::network_stage`
(`routes/freshness.rs`) as the same `seam_contracts::freshness::NetworkStage` elohim-storage declares —
fail-closed to `Bootstrap`, and `Simulacra` reachable only by explicit positive declaration.

If you are about to write `if state.args.dev_mode` in an auth path, stop. `DEV_MODE: "true"` is set on
EVERY deployed manifest, so a gate keyed on it is in practice ungated — that is how the seed and
`/admin/cache/*` routes became reachable with no credential from the open web (closed by `62b658784`),
and how the apex became undeployable when that bypass was correctly removed.

Two credentials answer two DIFFERENT questions, and collapsing them is the recurring defect:
`API_KEY_ADMIN` = *"is this caller MY admin?"* (this doorway's operator identity, deliberately distinct
per doorway); `API_KEY_SEED` = *"may this caller seed?"* (the fleet's deploy authority, uniform by design,
scoped to the seed/admin-cache routes and never to the permission ladder).

The full rule, the stage-to-context ladder, the chaperone exception for hosted humans, the migration to
p2p-derived authority, the open items, and the six questions to answer before adding a gate:
`doorway-auth-posture-declared-stage`.

Debugging a refusal rather than designing one — a red deploy seed leg, a host serving an old bundle at
200, a local seed suddenly demanding a credential — go to `doorway-auth-refusal-runbook`: probes,
per-refusal decision trees, and the test that separates an authorization refusal from a dataplane
divergence (that confusion has cost this project two incidents).

## Build

```bash
RUSTFLAGS="" cargo build --release     # MUST override RUSTFLAGS (Holochain WASM env breaks native builds)
RUSTFLAGS="" cargo test --lib --bins   # Unit tests (331+)
RUSTFLAGS="" cargo clippy -- -D warnings
cargo fmt --check
```

The system sets `RUSTFLAGS=--cfg getrandom_backend="custom"` for Holochain WASM. This breaks native Rust builds. Always override with `RUSTFLAGS=""` for this crate.

## Key Files

| File | Purpose |
|------|---------|
| `src/server/http.rs` | HTTP router — match block dispatches to handlers |
| `src/services/route_registry.rs` | Dynamic route table from three Source types: DNA discovery, peer registration, steward self-registration |
| `src/routes/storage_proxy.rs` | Single canonical `forward_to_storage` — single-target dispatch, with iroh dispatch boundary commented; doorway forwards bytes but does not reconcile them ([[project_doorway_single_target_no_fanout]], [[project_inventory_exchange_not_byte_replication]]) |
| `src/routes/mod.rs` | Route module declarations and re-exports |
| `src/routes/admin.rs` | Admin dashboard endpoints including `GET /admin/routes` |
| `src/routes/collectives.rs` | Collective governance (has business logic — not a simple proxy) |
| `src/routes/elohim_agent.rs` | AI agent sidecar proxy (auth + path rewriting) |
| `src/routes/identity.rs` | DID documents + identity API proxy |
| `src/routes/pkarr_resolver.rs` | Pkarr-backed DID/key resolution HTTP surface |
| `src/services/pkarr_resolver.rs` | Pkarr resolver service (DHT-backed name → key) |
| `src/render/` | Manifest-driven SSR dispatch (V8 + capability + concurrency semaphore) |
| `src/ssr.rs` | SSR entry orchestration and `x-ssr-*` observability headers |
| `src/main.rs` | Startup: creates AppState, self-registers steward storage |
| `src/conductor/registry.rs` + `src/conductor/router.rs` | Conductor pool: agent_pub_key → conductor mapping with per-request routing (identity-hosting axis) |
| `src/routes/admin_conductors.rs` | Hosted-user provisioning (`POST /admin/hosted-users`) + graduation accounting (MongoDB flag-state; source-chain migration not yet built) |
| `src/services/federation.rs` | DHT self-registration as `DoorwayRegistration` + federation peer discovery |
| `src/cache/resolution.rs` | DoorwayResolver: tiered Projection → Conductor → External resolution, exactly the three-layer truth model ([[project_three_layer_truth_model]]); also the write-on-fetch site for the projection cache |
| `src/projection/subscriber.rs` | Signal subscriber: connects to conductor app interface, receives DHT signals |
| `src/services/discovery.rs` | DiscoveryService: conductor DNA introspection (route stubs, future) |

doorway-service consumes view types from the `elohim-views` crate.

## Projection Signal Subscriber

The subscriber (`src/projection/subscriber.rs`) feeds the projection cache — doorway's web2-absorption layer that lets traditional-internet visitors read substrate content without being P2P participants (see `../CLAUDE.md` "Two Scaling Axes"). It connects to each conductor's app WebSocket to receive DHT signals. It uses our own `tokio-tungstenite` connection (supports hostnames/URLs) but the official `holochain_websocket::WireMessage` for auth encoding — byte-identical to what the conductor expects.

**Why the wrapper:** `holochain_client::AppWebsocket::connect()` requires `SocketAddr` (IP:port), not hostnames. k8s is the developer test-bench the wrapper accommodates — conductors are reached via headless service hostnames (e.g., `ws://elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8445`). On the protocol-truth deployment (peers reaching peers directly), this wrapper can be replaced with `AppWebsocket::connect()` directly ([[feedback_k8s_is_dev_substrate_not_protocol]]).

**Dependencies for wire format:** `holochain_websocket` (WireMessage), `holochain_conductor_api` (AppAuthenticationRequest), `holochain_serialized_bytes` (SerializedBytes encoding).

## Adding New Routes

Almost always, you should NOT touch doorway-service when adding a new route. Routes are manifest-driven: a peer's storage declares them, the registry compiles them, doorway serves them. This is why we deleted the 13 identical per-domain proxy files — a doorway is not the author of substrate logic, it is one of many surfaces the substrate is reached through ([[project_doorway_manifest_driven_routes]], [[project_doorway_views_through_not_owned]]).

1. Add the endpoint to elohim-storage
2. Add it to `build_manifest()` in elohim-storage's `http.rs`
3. The route is automatically available through doorway on next boot

The only time you add code to doorway-service is when the route needs **doorway-specific logic** — see `../CLAUDE.md` for the decision criteria.

## Testing

```bash
RUSTFLAGS="" cargo test --lib --bins                              # All tests
RUSTFLAGS="" cargo test --lib --bins route_registry               # Registry tests only
RUSTFLAGS="" cargo test --lib --bins storage_proxy                # Proxy tests only
```

The behaviors this crate answers to are scenario-anchored in the resilience epic: patron-CDN discovery, account-takeover recovery, and creator succession (`../../genesis/docs/content/elohim-protocol/resilience/README.md` Part VI).

## Design Documentation

Sibling design docs:
- `ARCHITECTURE.md` — Bootstrap, signal, gateway, cache, resolver components
- `FEDERATION.md` — Cross-doorway patterns, DID discovery, P2P bootstrap
- `SCALING.md` — Two-axis scaling, graduation flywheel, K8s modeling
- `REACH.md` — Reach enforcement rules, caching, DNA integration
- `RECOVERY-PROTOCOL.md` — Social recovery and shard distribution
- `RECOVERY-SPRINT-PLAN.md` — Recovery protocol implementation phases

Upward anchors:
- `../CLAUDE.md` — Architecture, trust model, the no-per-domain-proxy rule, single-target dispatch
- `../../genesis/docs/content/elohim-protocol/resilience/README.md` — Patron-CDN, social-recovery, creator-succession epics
- `../../genesis/data/stories/` — Canonical stories that name doorway-mediated experiences
