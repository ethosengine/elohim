# Doorway Service (Rust)

See `../CLAUDE.md` for architecture, routing model, trust model, and the route registry anti-pattern rules.

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
| `src/services/route_registry.rs` | Dynamic route table: DNA discovery + peer registration + steward self-registration |
| `src/routes/storage_proxy.rs` | Single canonical `forward_to_storage` function |
| `src/routes/mod.rs` | Route module declarations and re-exports |
| `src/routes/admin.rs` | Admin dashboard endpoints including `GET /admin/routes` |
| `src/routes/collectives.rs` | Collective governance (has business logic — not a simple proxy) |
| `src/routes/elohim_agent.rs` | AI agent sidecar proxy (auth + path rewriting) |
| `src/routes/identity.rs` | DID documents + identity API proxy |
| `src/main.rs` | Startup: creates AppState, self-registers steward storage |
| `src/cache/resolution.rs` | DoorwayResolver: tiered content resolution (Projection -> Conductor -> External) |
| `src/projection/subscriber.rs` | Signal subscriber: connects to conductor app interface, receives DHT signals |
| `src/services/discovery.rs` | DiscoveryService: conductor DNA introspection (route stubs, future) |

## Projection Signal Subscriber

The subscriber (`src/projection/subscriber.rs`) connects to each conductor's app WebSocket to receive DHT signals for the projection cache. It uses our own `tokio-tungstenite` connection (supports hostnames/URLs) but the official `holochain_websocket::WireMessage` for auth encoding — byte-identical to what the conductor expects.

**Why the wrapper:** `holochain_client::AppWebsocket::connect()` requires `SocketAddr` (IP:port), not hostnames. In k8s, conductors are reached via headless service hostnames (e.g., `ws://elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8445`). When deployment moves to native P2P (not k8s-simulated), this wrapper can be replaced with `AppWebsocket::connect()` directly.

**Dependencies for wire format:** `holochain_websocket` (WireMessage), `holochain_conductor_api` (AppAuthenticationRequest), `holochain_serialized_bytes` (SerializedBytes encoding).

## Adding New Routes

Almost always, you should NOT touch doorway-service when adding a new route. Instead:

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

## Design Documentation

These files in this directory describe doorway's design in detail:
- `ARCHITECTURE.md` — Bootstrap, signal, gateway, cache, resolver components
- `FEDERATION.md` — Cross-doorway patterns, DID discovery, P2P bootstrap
- `SCALING.md` — Two-axis scaling, graduation flywheel, K8s modeling
- `REACH.md` — Reach enforcement rules, caching, DNA integration
- `RECOVERY-PROTOCOL.md` — Social recovery and shard distribution
- `RECOVERY-SPRINT-PLAN.md` — Recovery protocol implementation phases
