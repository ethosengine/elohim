//! Typed Holochain App interface client using the official `holochain_client` crate.
//!
//! Wraps `AppWebsocket` in a thin adapter that:
//! - Accepts URL strings (`ws://hostname:port`), not just `SocketAddr`
//! - Issues its own auth token via admin (one-step connect)
//! - Provides signal subscription
//! - Handles health checking
//!
//! All wire protocol handling — authentication, keepalive, signal routing —
//! is delegated to the official client. Compile-time type safety replaces
//! the runtime format guessing that caused 5+ wrong fixes in the subscriber.
//!
//! ## Why this exists
//!
//! `AppWebsocket::connect()` takes `impl ToSocketAddrs` which resolves hostnames
//! via DNS. The doc examples only show `Ipv4Addr::LOCALHOST`, creating the false
//! impression that hostnames don't work. This wrapper makes the hostname support
//! explicit and provides a single entry point for all app interface connections.
//!
//! When Holochain's docs improve, this wrapper can thin out — but the
//! "connect by URL + issue token in one step" convenience will remain useful.

use holochain_client::{AppWebsocket, ClientAgentSigner};
use holochain_types::signal::Signal;
use std::time::Duration;
use tracing::{debug, info, warn};

use super::typed_admin::TypedAdminClient;

/// Typed Holochain app interface client backed by `holochain_client::AppWebsocket`.
///
/// Provides authenticated connection to the conductor's app interface with
/// signal subscription. All wire protocol details are handled by the official client.
pub struct TypedAppClient {
    app_ws: AppWebsocket,
    admin_url: String,
    app_url: String,
    installed_app_id: String,
}

impl TypedAppClient {
    /// Connect to a conductor's app interface.
    ///
    /// 1. Connects to admin interface to issue an auth token
    /// 2. Connects to app interface with the token
    /// 3. Authenticates (handled by AppWebsocket internally)
    ///
    /// Both `admin_url` and `app_url` accept `ws://hostname:port` format.
    /// Hostnames are resolved via DNS (works with k8s headless services).
    pub async fn connect(
        admin_url: &str,
        app_url: &str,
        installed_app_id: &str,
    ) -> Result<Self, String> {
        // Step 1: Issue auth token via admin
        let admin = TypedAdminClient::connect(admin_url).await?;
        let token = admin
            .issue_app_authentication_token(installed_app_id, 0)
            .await?;

        info!(
            app_url,
            installed_app_id, "Obtained app auth token, connecting to app interface"
        );

        // Step 2: Connect to app interface with token
        // Strip ws:// prefix — ToSocketAddrs expects "host:port"
        let app_addr = strip_ws_prefix(app_url);

        let signer = ClientAgentSigner::default();
        let app_ws = AppWebsocket::connect(
            &*app_addr,
            token,
            signer.into(),
            Some("doorway".to_string()),
        )
        .await
        .map_err(|e| format!("AppWebsocket connect to {app_url} failed: {e}"))?;

        info!(
            agent = %app_ws.cached_app_info().agent_pub_key,
            app_url,
            "Connected and authenticated to conductor app interface"
        );

        Ok(Self {
            app_ws,
            admin_url: admin_url.to_string(),
            app_url: app_url.to_string(),
            installed_app_id: installed_app_id.to_string(),
        })
    }

    /// Subscribe to app signals from this conductor.
    ///
    /// The handler receives `Signal::App { cell_id, zome_name, signal }` events.
    /// The official client filters signals to only this app's cells.
    /// Returns a handle ID that can be used to unsubscribe.
    pub async fn on_signal<F>(&self, handler: F) -> String
    where
        F: Fn(Signal) + 'static + Sync + Send,
    {
        self.app_ws.on_signal(handler).await
    }

    /// Health check — returns true if the connection is alive and the app exists.
    pub async fn is_healthy(&self) -> bool {
        matches!(self.app_ws.app_info().await, Ok(Some(_)))
    }

    /// Get the agent public key for this app connection.
    pub fn agent_pub_key(&self) -> &holochain_client::AgentPubKey {
        &self.app_ws.cached_app_info().agent_pub_key
    }

    /// Get the admin URL used to connect.
    pub fn admin_url(&self) -> &str {
        &self.admin_url
    }

    /// Get the app interface URL.
    pub fn app_url(&self) -> &str {
        &self.app_url
    }

    /// Get the installed app ID.
    pub fn installed_app_id(&self) -> &str {
        &self.installed_app_id
    }
}

/// Strip `ws://` or `wss://` prefix from a URL to get a socket address string.
///
/// `AppWebsocket::connect()` takes `impl ToSocketAddrs` which expects `host:port`.
fn strip_ws_prefix(url: &str) -> String {
    let url = url.trim();
    if let Some(rest) = url.strip_prefix("wss://") {
        rest.to_string()
    } else if let Some(rest) = url.strip_prefix("ws://") {
        rest.to_string()
    } else {
        url.to_string()
    }
}

/// Hold an app connection open with periodic health checks.
///
/// Returns when the connection dies or shutdown signal is received.
/// Caller should reconnect after this returns.
pub async fn hold_connection(
    client: &TypedAppClient,
    health_interval: Duration,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown received, closing app connection");
                break;
            }
            _ = tokio::time::sleep(health_interval) => {
                if client.is_healthy().await {
                    debug!("App connection health check passed");
                } else {
                    warn!("App connection health check failed, reconnecting");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ws_prefix() {
        assert_eq!(strip_ws_prefix("ws://localhost:4445"), "localhost:4445");
        assert_eq!(
            strip_ws_prefix("wss://host.example.com:8445"),
            "host.example.com:8445"
        );
        assert_eq!(
            strip_ws_prefix("ws://elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8445"),
            "elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8445"
        );
        assert_eq!(strip_ws_prefix("localhost:4445"), "localhost:4445");
    }
}
