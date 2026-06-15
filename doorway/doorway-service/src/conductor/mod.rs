//! Conductor pool management
//!
//! Maps agent public keys to conductor instances for multi-conductor routing.
//! Every doorway instance knows the conductor pool and can answer
//! "which conductor hosts this agent?" — no special mode required.

pub mod chaperone;
pub mod pool_map;
pub mod provisioner;
pub mod registry;
pub mod router;
pub mod typed_admin;
pub mod typed_app;

pub use pool_map::{ConductorPoolMap, ConductorPoolStatus};
pub use provisioner::{AgentProvisioner, ProvisionedAgent};
pub use registry::{ConductorEntry, ConductorInfo, ConductorRegistry};
pub use router::ConductorRouter;
pub use typed_admin::TypedAdminClient;
pub use typed_app::TypedAppClient;

use std::net::SocketAddr;
use std::time::Duration;

/// Default DNS-resolution timeout for conductor connects (ms). A failing resolve
/// must return within this bound rather than parking the caller.
const CONDUCTOR_DNS_TIMEOUT_MS: u64 = 5_000;

/// Resolve a conductor address (`ws://host:port`, `wss://host:port`, or bare
/// `host:port`) to a concrete `SocketAddr` using tokio's ASYNC resolver, so the
/// lookup never parks a core worker thread.
///
/// WHY (alpha doorway crashloop RCA, 2026-06-15): `holochain_client`'s
/// `AdminWebsocket`/`AppWebsocket::connect` call `std::net::ToSocketAddrs::
/// to_socket_addrs()` — a bare, synchronous libc `getaddrinfo` — at the top of
/// their async fns (`admin_websocket.rs:112`, `app_websocket_inner.rs:45`), on
/// whatever tokio worker polls the connect future. Under a flapping/NXDOMAIN
/// conductor that blocking syscall parks the worker for the full resolver timeout
/// (zero CPU, and uncancellable by `tokio::time::timeout` because it has no
/// `.await` yield point); enough concurrent parks from the per-conductor
/// reconnect storm starve the shared-runtime `/health` task off the scheduler →
/// liveness SIGKILL. `tokio::net::lookup_host` dispatches `getaddrinfo` to
/// tokio's BLOCKING pool (not a core worker) and is `.await`-cancellable; handing
/// the resulting `SocketAddr` to the connect makes the dep's `to_socket_addrs()`
/// a no-op (the std impl for `SocketAddr` yields the address without a syscall).
pub(crate) async fn resolve_host_port(addr: &str) -> Result<SocketAddr, String> {
    // Tolerate a ws://|wss:// prefix so every caller is safe whether it passes a
    // full URL or a bare host:port.
    let host_port = addr
        .trim()
        .trim_start_matches("wss://")
        .trim_start_matches("ws://")
        .to_string();
    tokio::time::timeout(
        Duration::from_millis(CONDUCTOR_DNS_TIMEOUT_MS),
        tokio::net::lookup_host(host_port.clone()),
    )
    .await
    .map_err(|_| {
        format!("DNS resolve for '{host_port}' timed out after {CONDUCTOR_DNS_TIMEOUT_MS}ms")
    })?
    .map_err(|e| format!("DNS resolve for '{host_port}' failed: {e}"))?
    .next()
    .ok_or_else(|| format!("DNS resolve for '{host_port}' returned no addresses"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolve_host_port_handles_ip_literal() {
        let addr = resolve_host_port("127.0.0.1:4444")
            .await
            .expect("ip literal resolves");
        assert_eq!(addr.port(), 4444);
        assert!(addr.ip().is_loopback());
    }

    #[tokio::test]
    async fn resolve_host_port_strips_ws_scheme() {
        let addr = resolve_host_port("ws://127.0.0.1:8445")
            .await
            .expect("ws:// ip resolves");
        assert_eq!(addr.port(), 8445);
    }

    #[tokio::test]
    async fn resolve_host_port_errors_without_network_on_malformed() {
        // No port ⇒ lookup_host fails to parse, no network I/O, deterministic.
        assert!(resolve_host_port("missing-port").await.is_err());
    }
}
