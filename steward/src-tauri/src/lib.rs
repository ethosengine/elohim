use std::io::Cursor;
use std::path::PathBuf;
use tauri_plugin_holochain::{HolochainExt, HolochainPluginConfig, NetworkConfig, vec_to_locked, AppBundle};
use tauri::{AppHandle, Listener};

const APP_ID: &str = "elohim";

// Build-time environment configuration via environment variables
// Set ELOHIM_BOOTSTRAP_URL and ELOHIM_SIGNAL_URL during build for custom endpoints
// Defaults to production if not specified
const DEFAULT_BOOTSTRAP_URL: &str = "https://doorway.elohim.host/bootstrap";
const DEFAULT_SIGNAL_URL: &str = "wss://signal.doorway.elohim.host";

// Dev environment endpoints (for builds targeting dev/alpha)
const DEV_BOOTSTRAP_URL: &str = "https://doorway-dev.elohim.host/bootstrap";
const DEV_SIGNAL_URL: &str = "wss://signal.doorway-dev.elohim.host";

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
        let tmp_dir = tempdir::TempDir::new("elohim-steward")
            .expect("Could not create temporary directory");
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
        .plugin(tauri_plugin_holochain::async_init(
            vec_to_locked(vec![]),
            HolochainPluginConfig::new(holochain_dir(), network_config())
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            app.handle().listen("holochain://setup-completed", move |_event| {
                let handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    setup(handle.clone()).await.expect("Failed to setup");

                    handle
                        .holochain()
                        .expect("Failed to get holochain")
                        .main_window_builder(String::from("main"), false, Some(APP_ID.into()), None).await
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

/// Setup the Holochain app on first run or update coordinators on subsequent runs
async fn setup(handle: AppHandle) -> anyhow::Result<()> {
    let admin_ws = handle.holochain()?.admin_websocket().await?;

    let installed_apps = admin_ws
        .list_apps(None)
        .await
        .map_err(|err| tauri_plugin_holochain::Error::ConductorApiError(err))?;

    // Check if app is already installed
    if installed_apps.iter().find(|app| app.installed_app_id.as_str().eq(APP_ID)).is_none() {
        // First run: install the app
        log::info!("Installing Elohim hApp for the first time...");
        handle
            .holochain()?
            .install_app(
                String::from(APP_ID),
                elohim_happ(),
                None,
                None,
                None,
            )
            .await?;

        Ok(())
    } else {
        // Subsequent runs: update coordinators if necessary
        log::info!("Checking for coordinator updates...");
        handle.holochain()?.update_app_if_necessary(
            String::from(APP_ID),
            elohim_happ()
        ).await?;

        Ok(())
    }
}
