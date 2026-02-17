# Elohim Steward - Desktop P2P Client

Tauri 2.x desktop application that runs the full Elohim P2P stack locally: cryptographic identity (Holochain conductor), content storage (elohim-storage sidecar), and the Angular learning platform. The steward makes a user a direct participant in the network's web of mutual accountability - holding their own keys, validating peers, and contributing to the shared infrastructure that the community depends on.

## Role in the Protocol

The Elohim Protocol builds toward **socially-resilient stewardship** - technology that helps humans understand the complex interdependencies of relationships that make thriving possible, and makes individual limits natural and intuitive. Progressive stewardship is the path through which users deepen their participation and responsibility within this relational fabric:

```
Visitor → Hosted User → App Steward → App + Node Steward → Doorway Host
                             ▲
                        YOU ARE HERE
```

- **Hosted User**: Doorway holds custodial keys, runs conductor on their behalf. The user learns and contributes, but the infrastructure is managed for them
- **App Steward** (this app): Self-custodied keys, local conductor + storage. Full participation - the user is a complete peer in the network, validating others, holding DHT shards, contributing to shared resilience
- **App + Node Steward**: Steward app + always-on elohim-node daemon. Both are peers stewarded by the same person, providing internal resilience - when the laptop closes, the node carries the load, and vice versa
- **Doorway Host**: A node steward who goes above and beyond, running a federated gateway that provides Web 2.0 conveniences (HTTP access, custodial keys, bootstrap/signal, account recovery) for users still graduating - and a portal to recover when disaster happens

The steward app is full participation on its own - it is a complete node. The practical reality is that laptops close. An elohim-node is a peer stewarded by the same person, not completing the steward but backing it up. A doorway is not a separate category; it's the most generous expression of node stewardship, offering on-ramps to the wider community.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  Elohim Steward (Tauri 2.x)                                 │
│                                                              │
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────────┐  │
│  │  Angular UI   │  │  Tauri IPC    │  │  Holochain       │  │
│  │  (webview)    │◄─┤  Commands     │  │  Conductor       │  │
│  │              │  │              │  │  (identity &     │  │
│  │  lamad       │  │  doorway_*   │  │   provenance)    │  │
│  │  imagodei    │  │  identity    │  │  admin: 4444     │  │
│  │  shefa       │  │  handoff     │  │  app:   4445     │  │
│  │  qahal       │  │              │  │                  │  │
│  └──────┬───────┘  └───────────────┘  └──────────────────┘  │
│         │                                                    │
│         │ HTTP fetch (same API as browser mode)              │
│         ▼                                                    │
│  ┌──────────────────────────────────────────────────────────┐│
│  │  elohim-storage (localhost:8090)                         ││
│  │  Content DB + blob storage + sessions + mastery          ││
│  └──────────────────────────────────────────────────────────┘│
│                                                              │
│  ┌──────────────────────────────────────────────────────────┐│
│  │  WebSocket Interceptor (initialization_script)           ││
│  │  Redirects doorway WS → local conductor                  ││
│  └──────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────┘
         │
         │ P2P (DHT gossip, bootstrap, signal)
         ▼
    ┌──────────┐
    │  Network  │  Other stewards, doorway edge nodes, elohim-nodes
    └──────────┘
```

### What each layer provides

| Layer | Role | Why it matters |
|-------|------|----------------|
| **Holochain conductor** | Cryptographic identity, source chains, DHT participation | Every action is signed by the user's agent key; peers validate each other |
| **elohim-storage** | Content DB (SQLite), blob storage, session management, mastery records | Where content actually lives; Holochain DHT is for provenance, not bulk content |
| **Angular UI** | Learning (lamad), identity (imagodei), economics (shefa), governance (qahal) | Same app as browser mode, just wired to local services |
| **Tauri shell** | Window management, deep links, IPC, OS integration | Bridges native OS and the web-based UI |

### Key Insight: No FFI, Same HTTP API

The Tauri app does NOT use Rust FFI or direct SQLite bindings. It makes standard HTTP fetch calls to the same `elohim-storage` endpoints that doorway proxies in browser mode. This means one API boundary, one set of types, one behavior across all deployment modes.

---

## Source Files

| File | Purpose |
|------|---------|
| `src-tauri/src/lib.rs` | App setup, conductor init, window builder, WebSocket interceptor, deep link handler, IPC commands |
| `src-tauri/src/doorway.rs` | HTTP client for doorway auth (login, native-handoff, confirm-stewardship) |
| `src-tauri/src/identity.rs` | Key bundle decryption (Argon2id + ChaCha20-Poly1305, mirrors doorway crypto) |
| `src-tauri/src/main.rs` | Entry point, calls `steward_lib::run()` |
| `src-tauri/Cargo.toml` | Dependencies (tauri 2.9, tauri-plugin-holochain main-0.6) |
| `src-tauri/tauri.conf.json` | Tauri config (deep-link schemes, dev URL, bundle targets) |
| `flake.nix` | Nix devShell (holonix + tauri-plugin-holochain + just) |
| `package.json` | npm scripts: `tauri:dev`, `tauri:build` |

---

## Identity & Doorway Handoff Flow

Doorways are federated thin-client gateways - node stewards who provide Web 2.0 conveniences (HTTP access, custodial keys, bootstrap/signal, account recovery) so users can participate before running their own infrastructure, and have a portal to recover when disaster happens. The steward can operate in two modes:

**Standalone**: Fresh cryptographic identity generated locally. The user participates in the P2P network with a new agent key. No doorway needed.

**Connected (via doorway handoff)**: The user logs into a doorway instance, imports their existing identity + network config, and the local conductor joins the same DHT space as that doorway's community. This is how hosted users graduate to direct participation - their node becomes part of the network that others depend on.

```
1. User clicks "Connect to Doorway" in UI
2. doorway_login IPC command:
   - POST /auth/login → JWT token
   - GET /auth/native-handoff → identity + network config + encrypted key bundle
   - Decrypt key bundle with password (Argon2id + ChaCha20)
   - Save to doorway.json store (tauri-plugin-store)
3. App restart required (conductor must reinit with new network config)
4. setup() reads saved agentPubKey → installs hApp with doorway identity
5. Both conductors (doorway + steward) share same DHT identity
```

### Stewardship Graduation

`doorway_confirm_stewardship` proves key possession via Ed25519 signature, allowing the doorway to retire its custodial conductor cell. The custodial training wheels come off, but the interdependencies with the community deepen - the steward's node now participates directly in peer validation and DHT gossip, contributing to the resilience of the shared infrastructure. The doorway that hosted them is freed to serve others still graduating.

---

## IPC Commands

| Command | Purpose |
|---------|---------|
| `doorway_login` | Authenticate with doorway, save identity + network config (v2 multi-account) |
| `doorway_confirm_stewardship` | Prove key possession, complete graduation to direct stewardship |
| `doorway_status` | Check active account's doorway identity status |
| `doorway_unlock` | Decrypt key bundle with password (local auth, no network) |
| `doorway_logout` | Remove active account from store |
| `doorway_list_accounts` | List all saved accounts (multi-account) |
| `doorway_switch_account` | Switch active account by humanId (requires restart) |
| `doorway_lock` | Soft lock: delete session, keep identity (unlock with password) |
| `doorway_remove_account` | Remove a specific account by humanId |
| `doorway_reset` | Clear all accounts and session data |
| `doorway_deregister` | Revoke identity from doorway + remove locally |
| `doorway_emergency_wipe` | Delete all local data (store, storage, holochain) and exit |
| `get_pending_deep_links` | Drain buffered OAuth callbacks (cold-start) |

---

## Deep Link Handling

Scheme: `elohim://auth/callback?code=...&state=...`

- **Warm start**: `on_open_url` handler emits `oauth-callback` event to frontend
- **Cold start**: Callbacks buffered in `PendingDeepLinks`, drained via `get_pending_deep_links` after Angular init
- **Dev mode**: Deep link scheme registered at runtime on Linux/Windows

---

## Network Configuration Priority

1. Saved doorway handoff data (runtime, from `doorway.json`)
2. `ELOHIM_BOOTSTRAP_URL` / `ELOHIM_SIGNAL_URL` env vars (compile time)
3. `ELOHIM_ENV=dev` selects alpha endpoints
4. Default: production endpoints

---

## Nix DevShell

The steward flake provides the combined toolchain for the full local stack:

```bash
nix develop ./steward --accept-flake-config
```

**Packages**: nodejs_22, pnpm, imagemagick, patchelf, just

**Shell hook**: Deduplicates `NIX_CFLAGS_COMPILE` and `NIX_LDFLAGS` to avoid "Argument list too long" from combined devshells (holonix + tauri-plugin-holochain repeat `-isystem` paths hundreds of times).

**Important**: Use `nix develop`, not `nix develop --command bash`. The latter skips `shellHook`, causing the deduplication to not run, which leads to build failures.

---

## Development

```bash
# Enter nix shell (required for all steward work)
nix develop ./steward --accept-flake-config

# Dev mode (hot-reload, connects to Angular dev server at localhost:4200)
just steward-dev

# Production build
just steward-build

# Or via npm directly
cd steward && npm run tauri:dev
```

### Prerequisites

- elohim.happ must be built and placed in `workdir/` (CI does this, or `just dna-build`)
- Angular UI must be built into `ui/` for production (CI does this, or `just app-build`)
- Dev mode uses Angular dev server at `localhost:4200` (start with `just app-dev`)

---

## Common Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| "Argument list too long" during build | Skipped shellHook (used `--command`) | Use `nix develop` without `--command` |
| hApp install fails silently | Missing `workdir/elohim.happ` | Run `just dna-build` |
| WebSocket connections go to doorway | Interceptor not loaded | Check `initialization_script` in lib.rs |
| OAuth callback lost on cold start | Deep link arrived before window | Check `get_pending_deep_links` is called by Angular |
| "Coordinator update failed, reinstalling" | hApp roles changed (new DNA added) | Expected behavior, reinstall is automatic |
| Content not loading in Tauri mode | elohim-storage not running or wrong port | Check storage sidecar is up on port 8090 |
