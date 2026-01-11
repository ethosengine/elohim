use serde::Serialize;
use std::io::Cursor;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Listener};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_holochain::{
    vec_to_locked, AppBundle, HolochainExt, HolochainPluginConfig, NetworkConfig,
};

const APP_ID: &str = "elohim";

// Build-time environment configuration via environment variables
// Set ELOHIM_BOOTSTRAP_URL and ELOHIM_SIGNAL_URL during build for custom endpoints
// Defaults to production if not specified
const DEFAULT_BOOTSTRAP_URL: &str = "https://doorway.elohim.host/bootstrap";
const DEFAULT_SIGNAL_URL: &str = "wss://signal.doorway.elohim.host";

// Dev environment endpoints (for builds targeting dev/alpha)
const DEV_BOOTSTRAP_URL: &str = "https://doorway-dev.elohim.host/bootstrap";
const DEV_SIGNAL_URL: &str = "wss://signal.doorway-dev.elohim.host";

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

/// Load the Elohim hApp bundle from embedded bytes
pub fn elohim_happ() -> AppBundle {
    let bytes = include_bytes!("../../workdir/elohim.happ");
    AppBundle::unpack(Cursor::new(bytes)).expect("Failed to unpack elohim happ")
}

/// Configure the Holochain network for Elohim
///
/// Network endpoints are determined at compile time:
/// 1. ELOHIM_BOOTSTRAP_URL / ELOHIM_SIGNAL_URL env vars (highest priority)
/// 2. ELOHIM_ENV=dev uses dev endpoints
/// 3. Default to production endpoints
fn network_config() -> NetworkConfig {
    let mut network_config = NetworkConfig::default();

    if tauri::is_dev() {
        // Local development mode (cargo tauri dev): use localhost
        network_config.bootstrap_url = url2::Url2::parse("http://localhost:8888/bootstrap");
        network_config.signal_url = url2::Url2::parse("ws://localhost:8888");
    } else {
        // Built app: use compile-time configured endpoints
        let bootstrap_url = option_env!("ELOHIM_BOOTSTRAP_URL")
            .or_else(|| {
                if option_env!("ELOHIM_ENV") == Some("dev") {
                    Some(DEV_BOOTSTRAP_URL)
                } else {
                    None
                }
            })
            .unwrap_or(DEFAULT_BOOTSTRAP_URL);

        let signal_url = option_env!("ELOHIM_SIGNAL_URL")
            .or_else(|| {
                if option_env!("ELOHIM_ENV") == Some("dev") {
                    Some(DEV_SIGNAL_URL)
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
        .plugin(tauri_plugin_deep_link::init())
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
fn handle_deep_link_url(handle: &AppHandle, url: &url2::Url2) {
    let url_str = url.to_string();
    log::info!("Processing deep link: {}", url_str);

    // Parse as standard URL for query parameter extraction
    let parsed = match url::Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => {
            log::error!("Failed to parse deep link URL: {}", e);
            let _ = handle.emit(
                "deep-link-error",
                DeepLinkError {
                    message: format!("Invalid URL: {}", e),
                    url: url_str,
                },
            );
            return;
        }
    };

    // Only handle auth callbacks
    if parsed.host_str() == Some("auth") && parsed.path() == "/callback" {
        // Extract query parameters
        let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();

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
        // First run: install the app
        log::info!("Installing Elohim hApp for the first time...");
        handle
            .holochain()?
            .install_app(String::from(APP_ID), elohim_happ(), None, None, None)
            .await?;

        Ok(())
    } else {
        // Subsequent runs: update coordinators if necessary
        log::info!("Checking for coordinator updates...");
        handle
            .holochain()?
            .update_app_if_necessary(String::from(APP_ID), elohim_happ())
            .await?;

        Ok(())
    }
}
