mod doorway;
mod identity;
mod storage;

use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Listener, Manager};
use tokio::sync::Mutex as TokioMutex;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_holochain::{
    vec_to_locked, AgentPubKey, AppBundle, HolochainExt, HolochainPluginConfig, NetworkConfig,
};
use tauri_plugin_store::StoreExt;

const APP_ID: &str = "elohim";

// Build-time environment configuration via environment variables
// Set ELOHIM_BOOTSTRAP_URL and ELOHIM_SIGNAL_URL during build for custom endpoints
// Defaults to production if not specified
const DEFAULT_BOOTSTRAP_URL: &str = "https://doorway.elohim.host/bootstrap";
const DEFAULT_SIGNAL_URL: &str = "wss://signal.doorway.elohim.host";

// Dev environment endpoints (for builds targeting dev/alpha)
const ALPHA_BOOTSTRAP_URL: &str = "https://doorway-alpha.elohim.host/bootstrap";
const ALPHA_SIGNAL_URL: &str = "wss://signal.doorway-alpha.elohim.host";

/// OAuth callback payload emitted to frontend when deep link is received
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthCallbackPayload {
    /// Authorization code from doorway
    pub code: String,
    /// State parameter for CSRF protection
    pub state: Option<String>,
    /// Full callback URL for debugging
    pub url: String,
}

/// Error payload for deep link handling failures
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepLinkError {
    pub message: String,
    pub url: String,
}

/// Buffered deep link callbacks for cold-start OAuth.
///
/// When the app is launched via deep link (e.g. `elohim://auth/callback?code=...`),
/// the main window doesn't exist yet. Callbacks are buffered here and drained
/// by the frontend via `get_pending_deep_links` after Angular initializes.
#[derive(Default)]
struct PendingDeepLinks(Vec<OAuthCallbackPayload>);

/// Load the Elohim hApp bundle from embedded bytes
pub fn elohim_happ() -> AppBundle {
    let bytes = include_bytes!("../../workdir/elohim.happ");
    AppBundle::unpack(Cursor::new(bytes)).expect("Failed to unpack elohim happ")
}

/// Saved doorway handoff data stored in doorway.json
const DOORWAY_STORE: &str = "doorway.json";

/// Configure the Holochain network for Elohim
///
/// Network endpoints are determined by priority:
/// 1. Saved doorway handoff data (runtime — from doorway login)
/// 2. ELOHIM_BOOTSTRAP_URL / ELOHIM_SIGNAL_URL env vars (compile time)
/// 3. ELOHIM_ENV=dev uses dev endpoints
/// 4. Default to production endpoints
///
/// First launch is always standalone (no doorway data yet).
/// After doorway_login + app restart, saved URLs take effect.
fn network_config() -> NetworkConfig {
    let mut network_config = NetworkConfig::default();

    // Check for saved doorway handoff data (runtime override)
    // Validate that the store isn't stale: bootstrap/signal URLs are only valid
    // if doorwayUrl is also present (a complete handoff). Partial data (e.g.
    // bootstrap URL without doorwayUrl) indicates a corrupted or partially-cleared store.
    let doorway_url = read_active_account_value("doorwayUrl");
    let doorway_bootstrap = read_active_account_value("bootstrapUrl");
    let doorway_signal = read_active_account_value("signalUrl");

    if doorway_url.is_some() && (doorway_bootstrap.is_some() || doorway_signal.is_some()) {
        // Use doorway-provided URLs (takes priority over compile-time defaults)
        if let Some(ref url) = doorway_bootstrap {
            log::info!("Using doorway bootstrap URL: {}", url);
            network_config.bootstrap_url = url2::Url2::parse(url);
        }
        if let Some(ref url) = doorway_signal {
            log::info!("Using doorway signal URL: {}", url);
            network_config.signal_url = url2::Url2::parse(url);
        }
    } else if doorway_bootstrap.is_some() || doorway_signal.is_some() {
        log::warn!("Stale network config detected (URLs without doorwayUrl), using defaults");
    } else if tauri::is_dev() {
        // Development mode: use alpha environment for p2p discovery
        network_config.bootstrap_url = url2::Url2::parse(ALPHA_BOOTSTRAP_URL);
        network_config.signal_url = url2::Url2::parse(ALPHA_SIGNAL_URL);
    } else {
        // Built app: use compile-time configured endpoints
        let bootstrap_url = option_env!("ELOHIM_BOOTSTRAP_URL")
            .or_else(|| {
                if option_env!("ELOHIM_ENV") == Some("dev") {
                    Some(ALPHA_BOOTSTRAP_URL)
                } else {
                    None
                }
            })
            .unwrap_or(DEFAULT_BOOTSTRAP_URL);

        let signal_url = option_env!("ELOHIM_SIGNAL_URL")
            .or_else(|| {
                if option_env!("ELOHIM_ENV") == Some("dev") {
                    Some(ALPHA_SIGNAL_URL)
                } else {
                    None
                }
            })
            .unwrap_or(DEFAULT_SIGNAL_URL);

        network_config.bootstrap_url = url2::Url2::parse(bootstrap_url);
        network_config.signal_url = url2::Url2::parse(signal_url);
    }

    // Mobile devices don't hold DHT data (reduces battery/bandwidth)
    if cfg!(mobile) {
        network_config.target_arc_factor = 0;
    }

    network_config
}

/// Read a string value from the active account in doorway.json (pre-Tauri init).
///
/// Supports both v1 (flat) and v2 (multi-account) store formats.
/// In v2, reads `accounts[activeHumanId][key]`.
fn read_active_account_value(key: &str) -> Option<String> {
    let app_dir = app_dirs2::app_root(
        app_dirs2::AppDataType::UserData,
        &app_dirs2::AppInfo {
            name: "elohim-steward",
            author: "Ethos Engine",
        },
    )
    .ok()?;

    let store_path = app_dir.join(DOORWAY_STORE);
    let content = std::fs::read_to_string(store_path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;

    // v2: multi-account structure
    if data.get("version").and_then(|v| v.as_u64()) == Some(2) {
        let active_id = data.get("activeHumanId")?.as_str()?;
        return data
            .get("accounts")?
            .get(active_id)?
            .get(key)?
            .as_str()
            .map(|s| s.to_string());
    }

    // v1: flat structure (fallback)
    data.get(key)?.as_str().map(|s| s.to_string())
}

/// Migrate doorway.json from v1 (flat) to v2 (multi-account) format.
///
/// Called at filesystem level before Tauri init. If the store has no `version` key,
/// wraps existing flat data into `accounts[humanId]` structure. Backs up original.
fn migrate_doorway_store() {
    let app_dir = match app_dirs2::app_root(
        app_dirs2::AppDataType::UserData,
        &app_dirs2::AppInfo {
            name: "elohim-steward",
            author: "Ethos Engine",
        },
    ) {
        Ok(dir) => dir,
        Err(_) => return,
    };

    let store_path = app_dir.join(DOORWAY_STORE);
    let content = match std::fs::read_to_string(&store_path) {
        Ok(c) => c,
        Err(_) => return, // No store file yet
    };

    let data: serde_json::Value = match serde_json::from_str(&content) {
        Ok(d) => d,
        Err(_) => return,
    };

    // Already v2 or empty
    if data.get("version").is_some() {
        return;
    }

    // Need humanId to create account entry
    let human_id = match data.get("humanId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return, // No identity saved, nothing to migrate
    };

    // Backup original
    let backup_path = app_dir.join("doorway.json.bak");
    let _ = std::fs::copy(&store_path, &backup_path);

    // Build v2 structure
    let mut accounts = serde_json::Map::new();
    accounts.insert(human_id.clone(), data);

    let v2 = serde_json::json!({
        "version": 2,
        "activeHumanId": human_id,
        "requireUnlock": true,
        "accounts": accounts,
    });

    if let Ok(v2_str) = serde_json::to_string_pretty(&v2) {
        let _ = std::fs::write(&store_path, v2_str);
        log::info!("Migrated doorway.json from v1 to v2 (multi-account)");
    }
}

/// Get the Holochain data directory
fn holochain_dir() -> PathBuf {
    if tauri::is_dev() {
        // Development: use temp directory (clean state each run)
        let tmp_dir =
            tempdir::TempDir::new("elohim-steward").expect("Could not create temporary directory");
        tmp_dir.into_path()
    } else {
        // Production: use persistent app data directory
        app_dirs2::app_root(
            app_dirs2::AppDataType::UserData,
            &app_dirs2::AppInfo {
                name: "elohim-steward",
                author: "Ethos Engine",
            },
        )
        .expect("Could not get app root")
        .join("holochain")
    }
}

const STORAGE_PORT: u16 = 8090;

/// Spawn elohim-storage as a managed sidecar process.
///
/// - Skips if port is already healthy (dev guard — supports `just storage-start`)
/// - Non-fatal: logs warning on failure, app continues in degraded mode
/// - Stores handle in managed state so the child lives as long as the app
async fn spawn_storage_sidecar(handle: &AppHandle) {
    // Dev guard: check if storage is already running (e.g. manual `just storage-start`)
    let health_url = format!("http://localhost:{}/health", STORAGE_PORT);
    if let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1))
        .build()
    {
        if let Ok(resp) = client.get(&health_url).send().await {
            if resp.status().is_success() {
                log::info!(
                    "elohim-storage already running on port {}, using existing instance",
                    STORAGE_PORT
                );
                return;
            }
        }
    }

    // Find binary
    let binary_path = match storage::find_binary() {
        Ok(path) => {
            log::info!("Found elohim-storage binary: {}", path.display());
            path
        }
        Err(e) => {
            log::warn!("elohim-storage binary not found: {} (app will run without local storage)", e);
            return;
        }
    };

    // Determine storage directory
    let storage_dir = if tauri::is_dev() {
        PathBuf::from("/tmp/elohim-storage")
    } else {
        app_dirs2::app_root(
            app_dirs2::AppDataType::UserData,
            &app_dirs2::AppInfo {
                name: "elohim-steward",
                author: "Ethos Engine",
            },
        )
        .expect("Could not get app root")
        .join("storage")
    };

    log::info!(
        "Starting elohim-storage on port {} (storage_dir: {})",
        STORAGE_PORT,
        storage_dir.display()
    );

    let config = storage::StorageConfig {
        binary_path,
        port: STORAGE_PORT,
        storage_dir,
        enable_content_db: true,
    };

    match storage::StorageProcess::spawn(config).await {
        Ok(process) => {
            log::info!("elohim-storage sidecar ready on port {}", process.port);
            // Store handle in managed state — keeps child alive until app exits
            let state = handle.state::<TokioMutex<Option<storage::StorageProcess>>>();
            *state.lock().await = Some(process);
        }
        Err(e) => {
            log::warn!(
                "Failed to start elohim-storage sidecar: {} (app will run without local storage)",
                e
            );
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Migrate doorway.json v1 → v2 before Tauri plugin-store loads it
    migrate_doorway_store();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .level_for("tracing::span", log::LevelFilter::Error)
                .level_for("holochain_sqlite", log::LevelFilter::Error)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_deep_link::init())
        .manage(Mutex::new(PendingDeepLinks::default()))
        .manage(TokioMutex::new(Option::<storage::StorageProcess>::None))
        .invoke_handler(tauri::generate_handler![
            doorway_login,
            doorway_confirm_stewardship,
            doorway_status,
            doorway_unlock,
            doorway_logout,
            doorway_list_accounts,
            doorway_switch_account,
            doorway_lock,
            doorway_remove_account,
            doorway_reset,
            doorway_deregister,
            doorway_emergency_wipe,
            get_pending_deep_links,
        ])
        .plugin(tauri_plugin_holochain::async_init(
            vec_to_locked(vec![]),
            HolochainPluginConfig::new(holochain_dir(), network_config())
                .admin_port(4444)
                .enable_mdns_discovery(),
        ))
        .setup(|app| {
            // Set up deep link handler for OAuth callbacks
            setup_deep_link_handler(app)?;

            let handle = app.handle().clone();
            app.handle()
                .listen("holochain://setup-completed", move |_event| {
                    let handle = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        spawn_storage_sidecar(&handle).await;
                        setup(handle.clone()).await.expect("Failed to setup");

                        handle
                            .holochain()
                            .expect("Failed to get holochain")
                            .main_window_builder(
                                String::from("main"),
                                false,
                                Some(APP_ID.into()),
                                None,
                            )
                            .await
                            .expect("Failed to build window")
                            .initialization_script(
                                r#"
                                // Redirect doorway WebSocket connections to local conductor
                                // and strip Node.js-only options from @holochain/client
                                (function() {
                                    const _WS = window.WebSocket;
                                    window.WebSocket = function(url, protocols) {
                                        let actual = (typeof url === 'string') ? url : url.toString();
                                        if (actual.includes('doorway') || actual.includes('elohim.host')) {
                                            console.log('[STEWARD] Redirecting WS:', actual, '-> ws://localhost:4444');
                                            actual = 'ws://localhost:4444';
                                        }
                                        // Only pass protocols if valid (string or array).
                                        // @holochain/client passes { origin: '...' } which is
                                        // a Node.js ws option, not a browser WebSocket protocol.
                                        if (typeof protocols === 'string' || Array.isArray(protocols)) {
                                            return new _WS(actual, protocols);
                                        }
                                        return new _WS(actual);
                                    };
                                    window.WebSocket.prototype = _WS.prototype;
                                    window.WebSocket.CONNECTING = _WS.CONNECTING;
                                    window.WebSocket.OPEN = _WS.OPEN;
                                    window.WebSocket.CLOSING = _WS.CLOSING;
                                    window.WebSocket.CLOSED = _WS.CLOSED;
                                    console.log('[STEWARD] WebSocket interceptor installed');
                                })();
                                "#,
                            )
                            .build()
                            .expect("Failed to open main window");
                    });
                });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Set up the deep link handler for OAuth callbacks
///
/// Handles elohim://auth/callback?code=...&state=... URLs from doorway OAuth flow.
/// Emits 'oauth-callback' event to frontend with the authorization code.
fn setup_deep_link_handler(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Check for URLs that launched the app (cold start)
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        log::info!("App launched with deep link URLs: {:?}", urls);
        for url in urls {
            handle_deep_link_url(app.handle(), &url);
        }
    }

    // Register handler for URLs received while app is running (warm start)
    let handle = app.handle().clone();
    app.deep_link().on_open_url(move |event| {
        let urls = event.urls();
        log::info!("Deep link received: {:?}", urls);
        for url in urls {
            handle_deep_link_url(&handle, &url);
        }
    });

    // On Linux/Windows in dev mode, register the deep link at runtime
    // This ensures deep links work without requiring app installation
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if tauri::is_dev() {
        log::info!("Registering elohim:// deep link scheme for development");
        if let Err(e) = app.deep_link().register("elohim") {
            log::warn!("Failed to register deep link scheme: {}", e);
        }
    }

    Ok(())
}

/// Parse and handle a single deep link URL
///
/// Parses OAuth callback URLs (elohim://auth/callback?code=...&state=...)
/// and emits the appropriate event to the frontend.
fn handle_deep_link_url(handle: &AppHandle, url: &url::Url) {
    let url_str = url.to_string();
    log::info!("Processing deep link: {}", url_str);

    // Only handle auth callbacks
    if url.host_str() == Some("auth") && url.path() == "/callback" {
        // Extract query parameters
        let params: std::collections::HashMap<_, _> = url.query_pairs().collect();

        if let Some(code) = params.get("code") {
            let payload = OAuthCallbackPayload {
                code: code.to_string(),
                state: params.get("state").map(|s| s.to_string()),
                url: url_str,
            };

            log::info!(
                "OAuth callback received, emitting event (code length: {})",
                code.len()
            );

            // If main window doesn't exist yet (cold start), buffer for later retrieval
            if handle.get_webview_window("main").is_none() {
                log::info!("Main window not ready, buffering OAuth callback");
                if let Ok(mut pending) = handle.state::<Mutex<PendingDeepLinks>>().lock() {
                    pending.0.push(payload);
                }
                return;
            }

            if let Err(e) = handle.emit("oauth-callback", payload) {
                log::error!("Failed to emit oauth-callback event: {}", e);
            }
        } else {
            log::warn!("OAuth callback missing 'code' parameter");
            let _ = handle.emit(
                "deep-link-error",
                DeepLinkError {
                    message: "Missing 'code' parameter in OAuth callback".to_string(),
                    url: url_str,
                },
            );
        }
    } else {
        log::info!("Ignoring non-auth deep link: {}", url_str);
    }
}

/// Setup the Holochain app on first run or update coordinators on subsequent runs
///
/// If a doorway identity is saved (from doorway_login), installs with the
/// doorway-provisioned agent key so both conductors share the same identity on DHT.
async fn setup(handle: AppHandle) -> anyhow::Result<()> {
    let admin_ws = handle.holochain()?.admin_websocket().await?;

    let installed_apps = admin_ws
        .list_apps(None)
        .await
        .map_err(|err| tauri_plugin_holochain::Error::ConductorApiError(err))?;

    // Check if app is already installed
    if installed_apps
        .iter()
        .find(|app| app.installed_app_id.as_str().eq(APP_ID))
        .is_none()
    {
        // First run: check for saved doorway identity
        let agent_key_str = read_active_account_value("agentPubKey");
        let agent_key: Option<AgentPubKey> =
            agent_key_str
                .as_deref()
                .and_then(|key_str| match AgentPubKey::try_from(key_str) {
                    Ok(key) => {
                        log::info!(
                            "Installing Elohim hApp with doorway agent key: {}...",
                            &key_str[..key_str.len().min(12)]
                        );
                        Some(key)
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to parse doorway agent key, using fresh identity: {}",
                            e
                        );
                        None
                    }
                });

        if agent_key.is_none() {
            log::info!("Installing Elohim hApp with fresh identity (standalone)...");
        }

        // Network seed from doorway (if any)
        let network_seed = read_active_account_value("networkSeed");

        handle
            .holochain()?
            .install_app(
                String::from(APP_ID),
                elohim_happ(),
                None,      // roles_settings
                agent_key, // doorway-provisioned agent identity (or None for fresh)
                network_seed,
            )
            .await?;
    } else {
        // Subsequent runs: check for identity mismatch (doorway switch)
        let saved_key_str = read_active_account_value("agentPubKey");
        let installed_app = installed_apps
            .iter()
            .find(|app| app.installed_app_id.as_str().eq(APP_ID));

        let needs_reinstall = match (&saved_key_str, installed_app) {
            (Some(saved), Some(app)) => {
                let installed_key_str = app.agent_pub_key.to_string();
                if *saved != installed_key_str {
                    log::warn!(
                        "Doorway identity mismatch: saved={}... installed={}... — reinstalling",
                        &saved[..saved.len().min(12)],
                        &installed_key_str[..installed_key_str.len().min(12)]
                    );
                    true
                } else {
                    false
                }
            }
            _ => false,
        };

        if needs_reinstall {
            // Uninstall old app and reinstall with new identity
            admin_ws
                .uninstall_app(String::from(APP_ID), false)
                .await
                .map_err(|err| tauri_plugin_holochain::Error::ConductorApiError(err))?;

            let agent_key: Option<AgentPubKey> = saved_key_str
                .as_deref()
                .and_then(|key_str| AgentPubKey::try_from(key_str).ok());
            let network_seed = read_active_account_value("networkSeed");

            handle
                .holochain()?
                .install_app(
                    String::from(APP_ID),
                    elohim_happ(),
                    None,      // roles_settings
                    agent_key, // new doorway identity
                    network_seed,
                )
                .await?;
        } else {
            // Same identity — try coordinator update, reinstall if hApp structure changed
            log::info!("Checking for coordinator updates...");
            let update_result = handle
                .holochain()?
                .update_app_if_necessary(String::from(APP_ID), elohim_happ())
                .await;

            if let Err(e) = update_result {
                // Update fails when hApp roles changed (e.g. new DNA added).
                // Coordinator updates can't add roles — must reinstall.
                log::warn!(
                    "Coordinator update failed ({}), reinstalling hApp with new structure...",
                    e
                );
                admin_ws
                    .uninstall_app(String::from(APP_ID), false)
                    .await
                    .map_err(|err| tauri_plugin_holochain::Error::ConductorApiError(err))?;

                let agent_key: Option<AgentPubKey> = saved_key_str
                    .as_deref()
                    .and_then(|key_str| AgentPubKey::try_from(key_str).ok());
                let network_seed = read_active_account_value("networkSeed");

                handle
                    .holochain()?
                    .install_app(
                        String::from(APP_ID),
                        elohim_happ(),
                        None,
                        agent_key,
                        network_seed,
                    )
                    .await?;
            }
        }
    }

    // After hApp install/update, ensure local session exists in elohim-storage.
    // Handles the restart-after-login case where doorway_login saved identity
    // to doorway.json but the session may not have persisted across restart.
    ensure_local_session().await;

    Ok(())
}

/// Ensure a local session exists in elohim-storage from saved doorway.json data.
///
/// Called after setup() completes (both first-run and restart cases) to handle
/// the restart-after-login scenario where the previous session may have been
/// in a temp directory or the storage sidecar wasn't running yet.
async fn ensure_local_session() {
    let human_id = read_active_account_value("humanId");
    let agent_key = read_active_account_value("agentPubKey");
    let doorway_url = read_active_account_value("doorwayUrl");
    let identifier = read_active_account_value("identifier");

    // Only attempt if we have the minimum required identity data
    let (human_id, agent_key, doorway_url, identifier) =
        match (human_id, agent_key, doorway_url, identifier) {
            (Some(h), Some(a), Some(d), Some(i)) => (h, a, d, i),
            _ => return, // No saved identity, nothing to do
        };

    let storage_url = "http://localhost:8090";
    let http = reqwest::Client::new();

    // Check if a session already exists
    match http.get(format!("{}/session", storage_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            log::info!("Local session already exists, skipping creation");
            return;
        }
        Ok(_) => {} // 404 or other — create session
        Err(e) => {
            log::warn!("Cannot reach elohim-storage to check session: {}", e);
            return;
        }
    }

    // Create session from doorway.json data
    let doorway_id = read_active_account_value("doorwayId");
    let bootstrap_url = read_active_account_value("bootstrapUrl");

    let session_body = serde_json::json!({
        "humanId": human_id,
        "agentPubKey": agent_key,
        "doorwayUrl": doorway_url,
        "doorwayId": doorway_id,
        "identifier": identifier,
        "bootstrapUrl": bootstrap_url,
    });

    match http
        .post(format!("{}/session", storage_url))
        .json(&session_body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            log::info!("Local session created from doorway.json on restart");
        }
        Ok(resp) => {
            log::warn!(
                "Failed to create local session on restart (status {})",
                resp.status()
            );
        }
        Err(e) => {
            log::warn!("Failed to create local session on restart: {}", e);
        }
    }
}

// =============================================================================
// Store Helpers — v2 multi-account access via Tauri plugin-store
// =============================================================================

/// Read a value from the active account in the Tauri plugin-store.
///
/// Supports both v2 (multi-account) and v1 (flat, legacy) formats.
fn store_get_active_account_value(
    store: &tauri_plugin_store::Store<tauri::Wry>,
    key: &str,
) -> Option<serde_json::Value> {
    if store.get("version").and_then(|v| v.as_u64()) == Some(2) {
        let active_id = store
            .get("activeHumanId")
            .and_then(|v| v.as_str().map(String::from))?;
        store
            .get("accounts")
            .and_then(|v| v.get(&active_id)?.get(key).cloned())
    } else {
        store.get(key)
    }
}

/// Read a string value from the active account.
fn store_get_active_str(
    store: &tauri_plugin_store::Store<tauri::Wry>,
    key: &str,
) -> Option<String> {
    store_get_active_account_value(store, key).and_then(|v| v.as_str().map(String::from))
}

/// Set a value on the active account in the Tauri plugin-store (v2 format).
fn store_set_active_account_value(
    store: &tauri_plugin_store::Store<tauri::Wry>,
    key: &str,
    value: serde_json::Value,
) {
    if store.get("version").and_then(|v| v.as_u64()) == Some(2) {
        if let Some(active_id) = store
            .get("activeHumanId")
            .and_then(|v| v.as_str().map(String::from))
        {
            if let Some(mut accounts) = store
                .get("accounts")
                .and_then(|v| v.as_object().cloned())
            {
                if let Some(account) = accounts
                    .get_mut(&active_id)
                    .and_then(|v| v.as_object_mut())
                {
                    account.insert(key.to_string(), value);
                    store.set("accounts", serde_json::json!(accounts));
                    return;
                }
            }
        }
    }
    // Fallback: flat write
    store.set(key, value);
}

// =============================================================================
// Tauri IPC Commands — Doorway Identity
// =============================================================================

/// Response from doorway_login command
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorwayLoginResult {
    pub human_id: String,
    pub identifier: String,
    pub agent_pub_key: String,
    pub doorway_id: String,
    pub conductor_id: Option<String>,
    pub has_key_bundle: bool,
    pub needs_restart: bool,
    /// Doorway says user has already confirmed stewardship
    pub is_steward: bool,
}

/// Response from doorway_status command
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorwayStatus {
    pub connected: bool,
    pub doorway_url: Option<String>,
    pub identifier: Option<String>,
    pub agent_pub_key: Option<String>,
    pub has_identity: bool,
    /// Whether user has confirmed stewardship (from doorway.json)
    pub is_steward: bool,
    /// Whether an encrypted key bundle is saved (enables local unlock)
    pub has_key_bundle: bool,
}

/// Login to doorway, retrieve identity + network context, decrypt key, save to store.
///
/// Flow:
/// 1. Login to doorway → get JWT
/// 2. Call native-handoff → get identity + network context + encrypted key bundle
/// 3. Decrypt key bundle with password → verify signing key
/// 4. Save handoff data to doorway.json store
/// 5. Return summary (user must restart for hApp install with identity)
#[tauri::command]
async fn doorway_login(
    app: AppHandle,
    url: String,
    identifier: String,
    password: String,
) -> Result<DoorwayLoginResult, String> {
    log::info!("Doorway login: {} → {}", identifier, url);

    // Step 1: Login
    let client = doorway::DoorwayClient::new(url.clone());
    let login_resp = client.login(&identifier, &password).await?;

    // Step 2: Native handoff
    let handoff = client.native_handoff(&login_resp.token).await?;

    // Step 3: Decrypt key bundle (if present) to verify password works
    let has_key_bundle = if let Some(ref bundle) = handoff.key_bundle {
        match identity::decrypt_key_bundle(bundle, &password) {
            Ok(_key_bytes) => {
                log::info!("Key bundle decrypted successfully for {}", identifier);
                true
            }
            Err(e) => {
                log::warn!("Key bundle decryption failed: {}", e);
                false
            }
        }
    } else {
        false
    };

    // Step 4: Save to v2 multi-account store
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    let is_steward = handoff.is_steward.unwrap_or(false);

    // Build account data object
    let mut account = serde_json::json!({
        "doorwayUrl": url,
        "identifier": handoff.identifier,
        "humanId": handoff.human_id,
        "agentPubKey": handoff.agent_pub_key,
        "doorwayId": handoff.doorway_id,
        "isSteward": is_steward,
        "addedAt": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default(),
    });

    if let Some(ref bootstrap) = handoff.bootstrap_url {
        account["bootstrapUrl"] = serde_json::json!(bootstrap);
    }
    if let Some(ref signal) = handoff.signal_url {
        account["signalUrl"] = serde_json::json!(signal);
    }
    if let Some(ref conductor_id) = handoff.conductor_id {
        account["conductorId"] = serde_json::json!(conductor_id);
    }
    if let Some(ref app_id) = handoff.installed_app_id {
        account["installedAppId"] = serde_json::json!(app_id);
    }
    if let Some(ref seed) = handoff.network_seed {
        account["networkSeed"] = serde_json::json!(seed);
    }
    if let Some(ref bundle) = handoff.key_bundle {
        account["keyBundle"] = serde_json::to_value(bundle).unwrap_or(serde_json::Value::Null);
    }
    if let Some(ref name) = handoff.display_name {
        account["displayName"] = serde_json::json!(name);
    }

    // Load or init v2 accounts map
    let mut accounts: serde_json::Map<String, serde_json::Value> = store
        .get("accounts")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    accounts.insert(handoff.human_id.clone(), account);

    store.set("version", serde_json::json!(2));
    store.set("activeHumanId", serde_json::json!(handoff.human_id));
    store.set("requireUnlock", serde_json::json!(true));
    store.set("accounts", serde_json::json!(accounts));

    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    log::info!(
        "Doorway handoff saved for {} (restart needed for identity, is_steward={})",
        identifier,
        is_steward
    );

    // Step 4b: Create local session in elohim-storage (sidecar)
    // This ensures the session exists before restart so TauriAuthService finds it.
    let storage_url = "http://localhost:8090";
    let session_body = serde_json::json!({
        "humanId": handoff.human_id,
        "agentPubKey": handoff.agent_pub_key,
        "doorwayUrl": url,
        "doorwayId": handoff.doorway_id,
        "identifier": handoff.identifier,
        "displayName": handoff.display_name,
        "profileImageHash": handoff.profile_image_hash,
        "bootstrapUrl": handoff.bootstrap_url,
    });

    let http = reqwest::Client::new();
    match http
        .post(format!("{}/session", storage_url))
        .json(&session_body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            log::info!("Local session created in elohim-storage");
        }
        Ok(resp) => {
            log::warn!(
                "Failed to create local session (status {}): storage may not be ready",
                resp.status()
            );
        }
        Err(e) => {
            log::warn!(
                "Failed to create local session (storage may not be running): {}",
                e
            );
        }
    }

    // Step 5: Return summary
    Ok(DoorwayLoginResult {
        human_id: handoff.human_id,
        identifier: handoff.identifier,
        agent_pub_key: handoff.agent_pub_key,
        doorway_id: handoff.doorway_id,
        conductor_id: handoff.conductor_id,
        has_key_bundle,
        needs_restart: true, // Always true — conductor must reinit with new network config
        is_steward,
    })
}

/// Confirm stewardship (graduation) — prove key possession and retire conductor cell.
///
/// Flow:
/// 1. Read saved handoff data from doorway.json store
/// 2. Re-login to get fresh JWT (token may have expired)
/// 3. Decrypt key bundle with password → 32-byte signing key
/// 4. Sign human_id with Ed25519 signing key
/// 5. Call POST /auth/confirm-stewardship with base64 signature
/// 6. Update store: isSteward: true, stewardshipAt: timestamp
///
/// NOT auto-triggered — requires explicit user action (graduation is destructive).
#[tauri::command]
async fn doorway_confirm_stewardship(
    app: AppHandle,
    password: String,
) -> Result<doorway::StewardshipConfirmedResponse, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use ed25519_dalek::{Signer, SigningKey};

    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    // Read saved handoff data from active account
    let doorway_url = store_get_active_str(&store, "doorwayUrl")
        .ok_or("No doorway URL saved — run doorway_login first")?;
    let identifier = store_get_active_str(&store, "identifier")
        .ok_or("No identifier saved — run doorway_login first")?;
    let human_id = store_get_active_str(&store, "humanId")
        .ok_or("No human ID saved — run doorway_login first")?;

    // Read key bundle from active account
    let key_bundle_value = store_get_active_account_value(&store, "keyBundle")
        .ok_or("No key bundle saved — run doorway_login first")?;
    let key_bundle: doorway::KeyExportFormat = serde_json::from_value(key_bundle_value.clone())
        .map_err(|e| format!("Invalid key bundle in store: {}", e))?;

    log::info!(
        "Confirming stewardship for {} at {}",
        identifier,
        doorway_url
    );

    // Step 1: Re-login to get fresh JWT
    let client = doorway::DoorwayClient::new(doorway_url.clone());
    let login_resp = client.login(&identifier, &password).await?;

    // Step 2: Decrypt key bundle → 32-byte signing key
    let signing_key_bytes = identity::decrypt_key_bundle(&key_bundle, &password)?;
    let signing_key = SigningKey::from_bytes(&signing_key_bytes);

    // Step 3: Sign human_id
    let signature = signing_key.sign(human_id.as_bytes());
    let signature_base64 = BASE64.encode(signature.to_bytes());

    // Step 4: Call confirm-stewardship
    let resp = client
        .confirm_stewardship(&login_resp.token, &signature_base64)
        .await?;

    // Step 5: Update active account in store
    store_set_active_account_value(&store, "isSteward", serde_json::json!(true));
    store_set_active_account_value(&store, "stewardshipAt", serde_json::json!(resp.stewardship_at));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    log::info!(
        "Stewardship confirmed for {} — conductor cell retired",
        identifier
    );

    Ok(resp)
}

/// Check doorway connection status (reads from active account)
#[tauri::command]
async fn doorway_status(app: AppHandle) -> Result<DoorwayStatus, String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    let doorway_url = store_get_active_str(&store, "doorwayUrl");
    let identifier = store_get_active_str(&store, "identifier");
    let agent_pub_key = store_get_active_str(&store, "agentPubKey");

    let connected = doorway_url.is_some() && identifier.is_some();
    let has_identity = agent_pub_key.is_some();
    let is_steward = store_get_active_account_value(&store, "isSteward")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_key_bundle = store_get_active_account_value(&store, "keyBundle").is_some();

    Ok(DoorwayStatus {
        connected,
        doorway_url,
        identifier,
        agent_pub_key,
        has_identity,
        is_steward,
        has_key_bundle,
    })
}

/// Drain buffered deep link callbacks (cold-start OAuth).
///
/// Called by Angular after event listeners are registered.
/// Returns any OAuth callbacks that arrived before the main window existed.
#[tauri::command]
fn get_pending_deep_links(
    state: tauri::State<'_, Mutex<PendingDeepLinks>>,
) -> Vec<OAuthCallbackPayload> {
    match state.lock() {
        Ok(mut pending) => std::mem::take(&mut pending.0),
        Err(_) => vec![],
    }
}

/// Response from doorway_unlock command
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorwayUnlockResult {
    pub identifier: String,
    pub is_steward: bool,
}

/// Unlock local identity by decrypting stored key bundle with password.
///
/// P2P-native auth: proves the user holds the secret without any network calls.
/// The encrypted key bundle in doorway.json is the auth layer — if you can
/// decrypt it, you own the identity.
///
/// Used by returning users who already have doorway.json from a previous login.
#[tauri::command]
async fn doorway_unlock(app: AppHandle, password: String) -> Result<DoorwayUnlockResult, String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    // Read key bundle from active account
    let key_bundle_value = store_get_active_account_value(&store, "keyBundle")
        .ok_or("No key bundle saved — login required")?;
    let key_bundle: doorway::KeyExportFormat = serde_json::from_value(key_bundle_value.clone())
        .map_err(|e| format!("Invalid key bundle: {}", e))?;

    // Decrypt locally — proves identity without network
    identity::decrypt_key_bundle(&key_bundle, &password)
        .map_err(|_| "Invalid password".to_string())?;

    // Read identity info from active account
    let is_steward = store_get_active_account_value(&store, "isSteward")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let identifier = store_get_active_str(&store, "identifier").unwrap_or_default();

    log::info!("Local unlock successful for {} (is_steward={})", identifier, is_steward);

    Ok(DoorwayUnlockResult {
        identifier,
        is_steward,
    })
}

/// Remove the active account from the store (or clear all if last account).
#[tauri::command]
async fn doorway_logout(app: AppHandle) -> Result<(), String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    if store.get("version").and_then(|v| v.as_u64()) == Some(2) {
        if let Some(active_id) = store
            .get("activeHumanId")
            .and_then(|v| v.as_str().map(String::from))
        {
            let mut accounts = store
                .get("accounts")
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();

            accounts.remove(&active_id);

            if accounts.is_empty() {
                store.clear();
            } else {
                // Switch to next available account
                let next_id = accounts.keys().next().cloned().unwrap_or_default();
                store.set("activeHumanId", serde_json::json!(next_id));
                store.set("accounts", serde_json::json!(accounts));
            }
        }
    } else {
        store.clear();
    }

    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    log::info!("Doorway credentials cleared");
    Ok(())
}

// =============================================================================
// Multi-Account Management
// =============================================================================

/// Summary of a saved account (returned to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub human_id: String,
    pub identifier: String,
    pub doorway_url: String,
    pub display_name: Option<String>,
    pub is_steward: bool,
    pub is_active: bool,
}

/// List all saved accounts in the multi-account store.
#[tauri::command]
async fn doorway_list_accounts(app: AppHandle) -> Result<Vec<AccountSummary>, String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    // v1 stores have at most one account
    if store.get("version").and_then(|v| v.as_u64()) != Some(2) {
        let human_id = store
            .get("humanId")
            .and_then(|v| v.as_str().map(String::from));
        if let Some(human_id) = human_id {
            return Ok(vec![AccountSummary {
                identifier: store
                    .get("identifier")
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                doorway_url: store
                    .get("doorwayUrl")
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                display_name: store
                    .get("displayName")
                    .and_then(|v| v.as_str().map(String::from)),
                is_steward: store
                    .get("isSteward")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                is_active: true,
                human_id,
            }]);
        }
        return Ok(vec![]);
    }

    let active_id = store
        .get("activeHumanId")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    let accounts = store
        .get("accounts")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    let mut result: Vec<AccountSummary> = accounts
        .iter()
        .map(|(id, acct)| AccountSummary {
            human_id: id.clone(),
            identifier: acct
                .get("identifier")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            doorway_url: acct
                .get("doorwayUrl")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            display_name: acct
                .get("displayName")
                .and_then(|v| v.as_str().map(String::from)),
            is_steward: acct
                .get("isSteward")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            is_active: *id == active_id,
        })
        .collect();

    // Active account first
    result.sort_by(|a, b| b.is_active.cmp(&a.is_active));

    Ok(result)
}

/// Switch active account by humanId. Requires app restart.
#[tauri::command]
async fn doorway_switch_account(
    app: AppHandle,
    human_id: String,
) -> Result<serde_json::Value, String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    // Verify account exists
    let accounts = store
        .get("accounts")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    if !accounts.contains_key(&human_id) {
        return Err(format!("Account {} not found", human_id));
    }

    store.set("activeHumanId", serde_json::json!(human_id));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    log::info!("Switched active account to {}", human_id);

    // Conductor must reinit with new agent key
    Ok(serde_json::json!({ "needsRestart": true }))
}

// =============================================================================
// Account Lifecycle
// =============================================================================

/// Soft lock — delete session from storage, return to lock screen.
/// Does NOT clear doorway.json. User can unlock again with password.
#[tauri::command]
async fn doorway_lock(_app: AppHandle) -> Result<(), String> {
    let storage_url = "http://localhost:8090";
    let http = reqwest::Client::new();

    match http
        .delete(format!("{}/session", storage_url))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() || resp.status() == 404 => {
            log::info!("Session deleted (lock)");
        }
        Ok(resp) => {
            log::warn!("Failed to delete session on lock (status {})", resp.status());
        }
        Err(e) => {
            log::warn!("Failed to delete session on lock: {}", e);
        }
    }

    Ok(())
}

/// Remove a specific account from the multi-account store.
#[tauri::command]
async fn doorway_remove_account(app: AppHandle, human_id: String) -> Result<(), String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    let mut accounts = store
        .get("accounts")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    if !accounts.contains_key(&human_id) {
        return Err(format!("Account {} not found", human_id));
    }

    accounts.remove(&human_id);

    if accounts.is_empty() {
        store.clear();
    } else {
        store.set("accounts", serde_json::json!(accounts));

        // If we removed the active account, switch to next
        let active_id = store
            .get("activeHumanId")
            .and_then(|v| v.as_str().map(String::from));
        if active_id.as_deref() == Some(&human_id) {
            let next_id = accounts.keys().next().cloned().unwrap_or_default();
            store.set("activeHumanId", serde_json::json!(next_id));
        }
    }

    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    // Delete session if it belongs to the removed account
    let storage_url = "http://localhost:8090";
    let http = reqwest::Client::new();
    let _ = http
        .delete(format!("{}/session", storage_url))
        .send()
        .await;

    log::info!("Account {} removed", human_id);
    Ok(())
}

/// Reset all accounts — clears store completely.
#[tauri::command]
async fn doorway_reset(app: AppHandle) -> Result<(), String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    store.clear();
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    // Delete session
    let storage_url = "http://localhost:8090";
    let http = reqwest::Client::new();
    let _ = http
        .delete(format!("{}/session", storage_url))
        .send()
        .await;

    log::info!("All accounts reset");
    Ok(())
}

/// Deregister identity from doorway (revoke). Stub if endpoint doesn't exist.
#[tauri::command]
async fn doorway_deregister(app: AppHandle, password: String) -> Result<(), String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    let doorway_url = store_get_active_str(&store, "doorwayUrl")
        .ok_or("No doorway URL saved")?;
    let identifier = store_get_active_str(&store, "identifier")
        .ok_or("No identifier saved")?;
    let human_id = store_get_active_str(&store, "humanId")
        .ok_or("No human ID saved")?;

    // Re-login to get fresh JWT
    let client = doorway::DoorwayClient::new(doorway_url.clone());
    let login_resp = client.login(&identifier, &password).await?;

    // Attempt deregister (may not exist yet on doorway)
    let http = reqwest::Client::new();
    let deregister_url = format!("{}/auth/deregister", doorway_url);
    match http
        .post(&deregister_url)
        .bearer_auth(&login_resp.token)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            log::info!("Identity deregistered from doorway for {}", identifier);
        }
        Ok(resp) if resp.status() == 404 => {
            log::warn!("Deregister endpoint not found (stub) — removing locally only");
        }
        Ok(resp) => {
            log::warn!("Deregister returned status {}", resp.status());
        }
        Err(e) => {
            log::warn!("Deregister failed (removing locally): {}", e);
        }
    }

    // Remove account locally
    doorway_remove_account(app, human_id).await?;

    Ok(())
}

/// Emergency wipe — delete all local data and exit.
#[tauri::command]
async fn doorway_emergency_wipe(app: AppHandle) -> Result<(), String> {
    // 1. Clear store
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;
    store.clear();
    let _ = store.save();

    // 2. Delete session
    let storage_url = "http://localhost:8090";
    let http = reqwest::Client::new();
    let _ = http
        .delete(format!("{}/session", storage_url))
        .send()
        .await;

    // 3. Delete storage data directory
    let app_dir = app_dirs2::app_root(
        app_dirs2::AppDataType::UserData,
        &app_dirs2::AppInfo {
            name: "elohim-steward",
            author: "Ethos Engine",
        },
    );
    if let Ok(dir) = app_dir {
        let storage_dir = dir.join("storage");
        if storage_dir.exists() {
            let _ = std::fs::remove_dir_all(&storage_dir);
            log::info!("Deleted storage directory: {}", storage_dir.display());
        }

        let holochain_dir = dir.join("holochain");
        if holochain_dir.exists() {
            let _ = std::fs::remove_dir_all(&holochain_dir);
            log::info!("Deleted holochain directory: {}", holochain_dir.display());
        }
    }

    log::info!("Emergency wipe complete — app should exit");

    // 4. Exit app
    app.exit(0);

    Ok(())
}
