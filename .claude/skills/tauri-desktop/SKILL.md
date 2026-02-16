---
name: tauri-desktop
description: Reference for Tauri 2.9 desktop shell development, tauri-plugin-holochain integration, doorway identity handoff, deep linking, key bundle crypto, and IPC commands. Use when someone asks "set up Tauri dev environment", "embed Holochain conductor", "handle deep links", "desktop identity flow", or works on the steward app and native identity.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# Tauri Desktop Reference

The Elohim steward app (`steward/src-tauri/`) is a Tauri 2.9 desktop shell embedding a Holochain conductor via `tauri-plugin-holochain`.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Elohim Steward (Tauri 2.9)             │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │
│  │ Angular App  │  │  IPC Layer  │  │ Tauri Plugins │  │
│  │ (WebView)    │<>│  Commands   │<>│               │  │
│  │              │  │             │  │ holochain     │  │
│  │ Same codebase│  │ doorway_*   │  │ deep-link     │  │
│  │ as web app   │  │             │  │ store         │  │
│  └─────────────┘  └─────────────┘  │ log           │  │
│                                     └───────┬───────┘  │
│                                             │          │
│  ┌──────────────────────────────────────────┘          │
│  │                                                      │
│  │  Holochain Conductor (embedded via plugin)           │
│  │  ├── Lair Keystore (in-process)                      │
│  │  ├── elohim.happ (include_bytes!)                    │
│  │  └── Network: bootstrap + signal via doorway         │
│  │                                                      │
│  │  elohim-storage sidecar (localhost:8090, future)     │
│  │                                                      │
│  └──────────────────────────────────────────────────────┘
└─────────────────────────────────────────────────────────┘
```

---

## Dependencies

```toml
# steward/src-tauri/Cargo.toml
tauri = { version = "2.9", features = ["devtools"] }
tauri-plugin-holochain = { git = "https://github.com/darksoil-studio/tauri-plugin-holochain", branch = "main-0.6" }
tauri-plugin-deep-link = "2"
tauri-plugin-store = "2"
tauri-plugin-log = "2.7"
argon2 = "0.5"                              # Key derivation
chacha20poly1305 = "0.10"                   # Symmetric encryption
ed25519-dalek = { version = "2.1", features = ["rand_core"] }  # Signing
```

---

## Holochain Plugin Setup

### Initialization

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_holochain::async_init(
        vec_to_locked(vec![]),  // No initial apps (installed in setup)
        HolochainPluginConfig::new(holochain_dir(), network_config()),
    ))
    .setup(|app| {
        let handle = app.handle().clone();
        app.handle().listen("holochain://setup-completed", move |_event| {
            tauri::async_runtime::spawn(async move {
                setup(handle.clone()).await.expect("Failed to setup");
                // Build main window after conductor is ready
                handle.holochain()?
                    .main_window_builder("main", false, Some(APP_ID.into()), None)
                    .await?
                    .build()?;
            });
        });
        Ok(())
    })
```

### hApp Installation

The `elohim.happ` is embedded via `include_bytes!`:

```rust
pub fn elohim_happ() -> AppBundle {
    let bytes = include_bytes!("../../workdir/elohim.happ");
    AppBundle::unpack(Cursor::new(bytes)).expect("Failed to unpack elohim happ")
}
```

### Setup Flow

1. `holochain://setup-completed` event fires when conductor is ready
2. Check if app is already installed (`list_apps`)
3. First run: install with doorway identity (if available) or fresh identity
4. Subsequent runs: check for identity mismatch, update coordinators if same

---

## Network Config Priority Chain

```
1. Saved doorway handoff (runtime)     — from doorway_login IPC
2. ELOHIM_BOOTSTRAP_URL env var        — compile-time override
3. ELOHIM_ENV=dev                      — alpha endpoints
4. Default production endpoints        — doorway.elohim.host
5. tauri::is_dev() (cargo tauri dev)   — localhost:8888
```

### Endpoints

| Environment | Bootstrap | Signal |
|-------------|-----------|--------|
| Production | `https://doorway.elohim.host/bootstrap` | `wss://signal.doorway.elohim.host` |
| Alpha/Dev | `https://doorway-alpha.elohim.host/bootstrap` | `wss://signal.doorway-alpha.elohim.host` |
| Local dev | `http://localhost:8888/bootstrap` | `ws://localhost:8888` |

---

## Doorway Identity Handoff

### Flow

```
1. User enters doorway URL + credentials in Angular UI
2. Angular calls doorway_login IPC command
3. Rust: POST /auth/login -> JWT
4. Rust: GET /auth/native-handoff -> identity + network context + encrypted key bundle
5. Rust: Decrypt key bundle with password (Argon2 + ChaCha20Poly1305)
6. Rust: Verify Ed25519 signing key
7. Rust: Save handoff to doorway.json (tauri-plugin-store)
8. Return result with needs_restart: true
9. User restarts app -> conductor installs with doorway agent key
```

### Handoff Data (doorway.json)

| Key | Type | Description |
|-----|------|-------------|
| `doorwayUrl` | string | Doorway base URL |
| `identifier` | string | User identifier (email/handle) |
| `humanId` | string | Human ID in network |
| `agentPubKey` | string | Holochain agent public key |
| `doorwayId` | string | Doorway instance ID |
| `bootstrapUrl` | string? | Bootstrap URL override |
| `signalUrl` | string? | Signal URL override |
| `conductorId` | string? | Conductor ID |
| `installedAppId` | string? | App ID |
| `networkSeed` | string? | Network seed for DNA |
| `keyBundle` | object? | Encrypted key bundle |
| `isSteward` | bool? | Stewardship confirmed |
| `stewardshipAt` | string? | Stewardship timestamp |

---

## Deep Linking

Protocol: `elohim://`

### OAuth Callback Flow

```
Browser: Login at doorway.elohim.host
  -> Redirect: elohim://auth/callback?code=...&state=...
    -> Cold start: Buffer in PendingDeepLinks, drain via get_pending_deep_links
    -> Warm start: Emit 'oauth-callback' event directly to frontend
```

### Implementation

```rust
// Cold start buffering
if handle.get_webview_window("main").is_none() {
    // Buffer for later
    pending.0.push(payload);
    return;
}
// Warm start: emit directly
handle.emit("oauth-callback", payload)?;
```

### Dev Mode Registration

```rust
#[cfg(any(target_os = "linux", target_os = "windows"))]
if tauri::is_dev() {
    app.deep_link().register("elohim")?;
}
```

---

## Key Bundle Crypto

### Encryption Stack

```
Password
  -> Argon2id (salt from bundle) -> 32-byte derived key
  -> ChaCha20Poly1305 (nonce from bundle) -> decrypt
  -> 32-byte Ed25519 signing key
```

### Stewardship Confirmation (Graduation)

```rust
// 1. Re-login for fresh JWT
let login = client.login(&identifier, &password).await?;

// 2. Decrypt key bundle -> signing key
let key_bytes = identity::decrypt_key_bundle(&bundle, &password)?;
let signing_key = SigningKey::from_bytes(&key_bytes);

// 3. Sign human_id
let signature = signing_key.sign(human_id.as_bytes());
let sig_b64 = BASE64.encode(signature.to_bytes());

// 4. POST /auth/confirm-stewardship
let resp = client.confirm_stewardship(&token, &sig_b64).await?;
```

---

## IPC Command Reference

### `doorway_login`

Login to doorway, retrieve identity, save handoff.

```typescript
// From Angular
const result = await invoke('doorway_login', {
  url: 'https://doorway.elohim.host',
  identifier: 'user@example.com',
  password: 'password',
});
// result: { humanId, identifier, agentPubKey, doorwayId, conductorId?, hasKeyBundle, needsRestart: true }
```

### `doorway_confirm_stewardship`

Prove key possession and graduate to full steward.

```typescript
const result = await invoke('doorway_confirm_stewardship', {
  password: 'password',
});
// result: { stewardshipAt: string }
```

### `doorway_status`

Check current doorway connection status.

```typescript
const status = await invoke('doorway_status');
// status: { connected, doorwayUrl?, identifier?, agentPubKey?, hasIdentity }
```

### `doorway_logout`

Clear all saved credentials.

```typescript
await invoke('doorway_logout');
```

### `get_pending_deep_links`

Drain buffered OAuth callbacks from cold start.

```typescript
const pending = await invoke('get_pending_deep_links');
// pending: OAuthCallbackPayload[]
```

---

## Build Commands

```bash
# Development
cd steward
cargo tauri dev

# Production build
cargo tauri build

# With environment targeting
ELOHIM_ENV=dev cargo tauri build         # Alpha endpoints
ELOHIM_BOOTSTRAP_URL=https://... cargo tauri build  # Custom

# Build hApp first (required - embedded via include_bytes!)
cd holochain/dna && ./build-all.sh
hc app pack steward/workdir/
```

---

## Gotchas

1. **`needs_restart: true`** - After `doorway_login`, the app MUST restart for the conductor to use the new identity. The hApp is reinstalled with the doorway-provisioned agent key on next launch.

2. **Identity mismatch** - If the saved doorway agent key differs from the installed app's key, the app is uninstalled and reinstalled with the new identity. This loses local DHT data.

3. **Stale network config** - If `bootstrapUrl`/`signalUrl` exist in store without `doorwayUrl`, they're treated as stale and ignored (falls back to defaults).

4. **hApp via `include_bytes!`** - The `.happ` file is embedded at compile time. Changes to zomes require rebuilding both the hApp and the Tauri app.

5. **Dev mode uses temp directory** - `cargo tauri dev` stores Holochain data in a temp dir that's cleaned on each run. Production uses persistent app data.

6. **Mobile arc factor** - Mobile builds set `target_arc_factor = 0` to avoid holding DHT data (battery/bandwidth).

7. **Store is read pre-init** - `network_config()` reads `doorway.json` directly from filesystem (not via Tauri store API) because it runs before Tauri initializes.

---

## Key Files

| File | Purpose |
|------|---------|
| `steward/src-tauri/src/lib.rs` | Main Tauri setup, IPC commands, deep links |
| `steward/src-tauri/src/identity.rs` | Key bundle decryption (Argon2 + ChaCha20) |
| `steward/src-tauri/src/doorway.rs` | HTTP client for doorway API |
| `steward/src-tauri/Cargo.toml` | Dependencies |
| `steward/src-tauri/tauri.conf.json` | Tauri configuration |
| `steward/workdir/elohim.happ` | Compiled hApp bundle |

## External References

- Tauri 2.x Docs: `https://v2.tauri.app/`
- Tauri Deep Linking: `https://v2.tauri.app/plugin/deep-linking/`
- Tauri Store: `https://v2.tauri.app/plugin/store/`
- tauri-plugin-holochain: `https://github.com/darksoil-studio/tauri-plugin-holochain`
