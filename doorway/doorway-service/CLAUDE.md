# Doorway Service (Rust)

doorway-service is the Rust implementation of the web2 projection of the elohim substrate — it makes the patron-CDN, the storage-stewardship surface, and the social-recovery flows reachable from the traditional internet. This file is the Rust-implementation orientation for the crate; architecture, trust model, routing model, and the no-per-domain-proxy-files discipline live in `../CLAUDE.md`. The deeper "why" — patron-CDN, account-takeover-recovery, creator-succession — lives in `../../genesis/docs/content/elohim-protocol/resilience/README.md` (Parts V and VI).

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
