mod doorway;
mod identity;

use serde::Serialize;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Listener, Manager};
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
    let doorway_url = read_doorway_store_value("doorwayUrl");
    let doorway_bootstrap = read_doorway_store_value("bootstrapUrl");
    let doorway_signal = read_doorway_store_value("signalUrl");

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
        // Local development mode (cargo tauri dev): use localhost
        network_config.bootstrap_url = url2::Url2::parse("http://localhost:8888/bootstrap");
        network_config.signal_url = url2::Url2::parse("ws://localhost:8888");
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

/// Read a string value from the doorway store file (pre-Tauri init)
///
/// This reads the store file directly since network_config() is called
/// before the Tauri app is initialized.
fn read_doorway_store_value(key: &str) -> Option<String> {
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
    data.get(key)?.as_str().map(|s| s.to_string())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
        .invoke_handler(tauri::generate_handler![
            doorway_login,
            doorway_confirm_stewardship,
            doorway_status,
            doorway_logout,
            get_pending_deep_links,
        ])
        .plugin(tauri_plugin_holochain::async_init(
            vec_to_locked(vec![]),
            HolochainPluginConfig::new(holochain_dir(), network_config()),
        ))
        .setup(|app| {
            // Set up deep link handler for OAuth callbacks
            setup_deep_link_handler(app)?;

            let handle = app.handle().clone();
            app.handle()
                .listen("holochain://setup-completed", move |_event| {
                    let handle = handle.clone();
                    tauri::async_runtime::spawn(async move {
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
        log::info!("Deep link received: {:?}", event.urls());
        for url in event.urls() {
            handle_deep_link_url(&handle, url);
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
        let agent_key_str = read_doorway_store_value("agentPubKey");
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
        let network_seed = read_doorway_store_value("networkSeed");

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

        Ok(())
    } else {
        // Subsequent runs: check for identity mismatch (doorway switch)
        let saved_key_str = read_doorway_store_value("agentPubKey");
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
            let network_seed = read_doorway_store_value("networkSeed");

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
            // Same identity — update coordinators if necessary
            log::info!("Checking for coordinator updates...");
            handle
                .holochain()?
                .update_app_if_necessary(String::from(APP_ID), elohim_happ())
                .await?;
        }

        Ok(())
    }
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

    // Step 4: Save to store
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    store.set("doorwayUrl", serde_json::json!(url));
    store.set("identifier", serde_json::json!(handoff.identifier));
    store.set("humanId", serde_json::json!(handoff.human_id));
    store.set("agentPubKey", serde_json::json!(handoff.agent_pub_key));
    store.set("doorwayId", serde_json::json!(handoff.doorway_id));

    if let Some(ref bootstrap) = handoff.bootstrap_url {
        store.set("bootstrapUrl", serde_json::json!(bootstrap));
    }
    if let Some(ref signal) = handoff.signal_url {
        store.set("signalUrl", serde_json::json!(signal));
    }
    if let Some(ref conductor_id) = handoff.conductor_id {
        store.set("conductorId", serde_json::json!(conductor_id));
    }
    if let Some(ref app_id) = handoff.installed_app_id {
        store.set("installedAppId", serde_json::json!(app_id));
    }
    if let Some(ref seed) = handoff.network_seed {
        store.set("networkSeed", serde_json::json!(seed));
    }

    // Save key bundle for future decryption (still encrypted)
    if let Some(ref bundle) = handoff.key_bundle {
        store.set(
            "keyBundle",
            serde_json::to_value(bundle).unwrap_or(serde_json::Value::Null),
        );
    }

    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    log::info!(
        "Doorway handoff saved for {} (restart needed for identity)",
        identifier
    );

    // Step 5: Return summary
    Ok(DoorwayLoginResult {
        human_id: handoff.human_id,
        identifier: handoff.identifier,
        agent_pub_key: handoff.agent_pub_key,
        doorway_id: handoff.doorway_id,
        conductor_id: handoff.conductor_id,
        has_key_bundle,
        needs_restart: true, // Always true — conductor must reinit with new network config
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

    // Read saved handoff data
    let doorway_url = store
        .get("doorwayUrl")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or("No doorway URL saved — run doorway_login first")?;
    let identifier = store
        .get("identifier")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or("No identifier saved — run doorway_login first")?;
    let human_id = store
        .get("humanId")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or("No human ID saved — run doorway_login first")?;

    // Read key bundle
    let key_bundle_value = store
        .get("keyBundle")
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

    // Step 5: Update store
    store.set("isSteward", serde_json::json!(true));
    store.set("stewardshipAt", serde_json::json!(resp.stewardship_at));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    log::info!(
        "Stewardship confirmed for {} — conductor cell retired",
        identifier
    );

    Ok(resp)
}

/// Check doorway connection status
#[tauri::command]
async fn doorway_status(app: AppHandle) -> Result<DoorwayStatus, String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    let doorway_url = store
        .get("doorwayUrl")
        .and_then(|v| v.as_str().map(String::from));
    let identifier = store
        .get("identifier")
        .and_then(|v| v.as_str().map(String::from));
    let agent_pub_key = store
        .get("agentPubKey")
        .and_then(|v| v.as_str().map(String::from));

    let connected = doorway_url.is_some() && identifier.is_some();
    let has_identity = agent_pub_key.is_some();

    Ok(DoorwayStatus {
        connected,
        doorway_url,
        identifier,
        agent_pub_key,
        has_identity,
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

/// Clear doorway credentials and saved identity
#[tauri::command]
async fn doorway_logout(app: AppHandle) -> Result<(), String> {
    let store = app
        .store(DOORWAY_STORE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    store.clear();
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    log::info!("Doorway credentials cleared");
    Ok(())
}
