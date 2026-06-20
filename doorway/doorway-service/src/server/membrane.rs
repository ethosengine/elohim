//! Doorway membrane policy stage — per-source rate-limit and admission verdict.
//!
//! Pure logic lives in `elohim-peer-fabric::guard` (feature = "edge-defense").
//! This module provides:
//!   - `client_ip_from_xff`  — rightmost-untrusted IP from X-Forwarded-For
//!   - `derive_source`       — authenticated agent key or IP fallback
//!   - `EdgeClock`           — wall-clock `Clock` impl for the runtime
//!   - `EdgeGuardStore`      — evicting in-memory `GuardStore` for the edge
//!   - `is_static_asset`     — exempts static files from membrane scoring
//!   - `edge_guard_config`   — interim edge `GuardConfig` with env overrides
//!
//! `apply_membrane` lives in `http.rs` where `to_boxed`, `resolve_agent_cid_from_request`,
//! and `admission_exempt` are already in scope.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;

use elohim_peer_fabric::guard::{Clock, GuardConfig, GuardStore};

// ─── Clock ────────────────────────────────────────────────────────────────────

/// Wall-clock seconds — runtime supplier for `assess`.
pub struct EdgeClock;

impl Clock for EdgeClock {
    fn now_secs(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

// ─── EdgeGuardStore ───────────────────────────────────────────────────────────

/// Per-source hit ring + ban map.  Evicts hits older than `window_secs` on every
/// `record` call for THAT source.  Calls `sweep_idle` opportunistically to evict
/// sources whose VecDeque has gone empty (idle eviction so the HashMap doesn't grow
/// without bound across distinct IPs).
struct SourceState {
    /// Timestamps (secs) of recent hits — oldest at front.
    hits: VecDeque<u64>,
    /// Ban expiry (`Some(until_secs)`) or `None` when not banned.
    ban_until: Option<u64>,
}

pub struct EdgeGuardStore {
    sources: HashMap<String, SourceState>,
    /// Window in seconds — used to prune stale hits on `record`.
    window_secs: u64,
    /// Epoch-secs of the last idle sweep.
    last_sweep_secs: u64,
    /// Active ban count — kept in sync with `sources` by `record`/`ban_until`/sweep.
    pub ban_count: i64,
}

impl EdgeGuardStore {
    pub fn new(window_secs: u64) -> Self {
        Self {
            sources: HashMap::new(),
            window_secs,
            last_sweep_secs: 0,
            ban_count: 0,
        }
    }

    /// Remove sources that have no in-window hits AND no active ban.  Called
    /// opportunistically (at most once per window) from `record` — keeps the
    /// HashMap bounded for high-IP-diversity traffic.
    pub fn sweep_idle(&mut self, now_secs: u64) {
        let cutoff = now_secs.saturating_sub(self.window_secs);
        let mut removed_bans = 0i64;
        self.sources.retain(|_, s| {
            // Prune expired hits first.
            while s.hits.front().is_some_and(|&t| t < cutoff) {
                s.hits.pop_front();
            }
            // Drop entry if no hits remain and ban has expired (or never set).
            let ban_active = s.ban_until.is_some_and(|until| until > now_secs);
            let keep = !s.hits.is_empty() || ban_active;
            if !keep && s.ban_until.is_some() {
                removed_bans += 1;
            }
            keep
        });
        self.ban_count -= removed_bans;
    }

    /// Opportunistically sweep once per window and update the Prometheus gauge.
    pub fn maybe_sweep(&mut self, now_secs: u64) {
        if now_secs.saturating_sub(self.last_sweep_secs) >= self.window_secs {
            self.sweep_idle(now_secs);
            self.last_sweep_secs = now_secs;
            crate::metrics::set_membrane_bans_active(self.ban_count);
        }
    }
}

impl GuardStore for EdgeGuardStore {
    fn record(&mut self, source: &str, ts_secs: u64) {
        let cutoff = ts_secs.saturating_sub(self.window_secs);
        let state = self.sources.entry(source.to_string()).or_insert_with(|| SourceState {
            hits: VecDeque::new(),
            ban_until: None,
        });
        // Prune in-window on record — bound the VecDeque for high-rate sources.
        while state.hits.front().is_some_and(|&t| t < cutoff) {
            state.hits.pop_front();
        }
        state.hits.push_back(ts_secs);
    }

    fn count_since(&self, source: &str, since_secs: u64) -> u32 {
        self.sources
            .get(source)
            .map_or(0, |s| s.hits.iter().filter(|&&t| t >= since_secs).count() as u32)
    }

    fn is_banned(&self, source: &str, now_secs: u64) -> bool {
        self.sources
            .get(source)
            .and_then(|s| s.ban_until)
            .is_some_and(|until| until > now_secs)
    }

    fn ban_until(&mut self, source: &str, until_secs: u64) {
        let state = self.sources.entry(source.to_string()).or_insert_with(|| SourceState {
            hits: VecDeque::new(),
            ban_until: None,
        });
        let was_banned = state.ban_until.is_some_and(|u| u > 0);
        state.ban_until = Some(until_secs);
        if !was_banned {
            self.ban_count += 1;
        }
    }
}

// ─── XFF / source key ─────────────────────────────────────────────────────────

/// Extract the client IP from `X-Forwarded-For` given the number of trusted
/// proxy hops.  The rightmost `trusted_proxy_hops` entries are set by the
/// infrastructure (ingress/load-balancer); the one to the LEFT of those is the
/// true client IP.
///
/// Falls back to `peer_addr` when the header is absent or too short.
pub fn client_ip_from_xff(xff: Option<&str>, peer_addr: &SocketAddr, trusted_proxy_hops: usize) -> String {
    let header = match xff {
        Some(h) if !h.is_empty() => h,
        _ => return peer_addr.ip().to_string(),
    };
    let parts: Vec<&str> = header.split(',').map(str::trim).collect();
    if parts.len() <= trusted_proxy_hops {
        // Header too short — fall back to peer addr (conservative).
        return peer_addr.ip().to_string();
    }
    let idx = parts.len() - trusted_proxy_hops - 1;
    parts[idx].to_string()
}

/// Build the rate-limit source key.  Authenticated requests use the human_id
/// (so multi-IP sessions share a single budget); unauthenticated use the IP.
pub fn derive_source(human_id: Option<&str>, client_ip: &str) -> String {
    match human_id {
        Some(id) if !id.is_empty() => format!("agent:{id}"),
        _ => format!("ip:{client_ip}"),
    }
}

// ─── Static asset exemption ───────────────────────────────────────────────────

/// Returns `true` for paths that should skip the membrane stage entirely.
///
/// Static assets are a very-high-request-rate class (every page load fetches
/// ~dozens of JS/CSS chunks), and their rate is entirely browser-driven rather
/// than adversarial.  Charging them against the per-session budget would
/// challenge or ban legitimate users on first page-load.
///
/// # TODO(calibrate): measure real page-load request count before deploy
/// Interim: exempt by extension (.js/.css/.woff2/.png) + bootstrap asset paths.
pub fn is_static_asset(path: &str) -> bool {
    // Bootstrap SPA chunks (Angular build output).
    if path.starts_with("/assets/") || path.starts_with("/chunk-") || path.starts_with("/main.") {
        return true;
    }
    // Extension-based exemption — covers hashed filenames Angular emits.
    let ext_start = path.rfind('.').map(|i| i + 1).unwrap_or(path.len());
    matches!(&path[ext_start..], "js" | "css" | "woff2" | "woff" | "png" | "ico" | "svg" | "map")
}

// ─── Edge GuardConfig ─────────────────────────────────────────────────────────

/// Build the edge `GuardConfig` from environment overrides with interim defaults.
///
/// Interim thresholds (calibrated for a single-ingress alpha topology):
/// - window 60s, shape 300, challenge 600, ban 1200, ban_secs 900, shape_delay 250ms
///
/// # TODO(calibrate): measure real page-load request count before deploy
pub fn edge_guard_config() -> GuardConfig {
    fn env_u64(key: &str, default: u64) -> u64 {
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
    fn env_u32(key: &str, default: u32) -> u32 {
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
    GuardConfig {
        window_secs: env_u64("DOORWAY_MEMBRANE_WINDOW_SECS", 60),
        shape_threshold: env_u32("DOORWAY_MEMBRANE_SHAPE_THRESHOLD", 300),
        challenge_threshold: env_u32("DOORWAY_MEMBRANE_CHALLENGE_THRESHOLD", 600),
        ban_threshold: env_u32("DOORWAY_MEMBRANE_BAN_THRESHOLD", 1200),
        ban_secs: env_u64("DOORWAY_MEMBRANE_BAN_SECS", 900),
        shape_delay_ms: env_u64("DOORWAY_MEMBRANE_SHAPE_DELAY_MS", 250),
    }
}

/// Parse `DOORWAY_TRUSTED_PROXY_HOPS` (default 1 for single-ingress topology).
pub fn trusted_proxy_hops_from_env() -> usize {
    std::env::var("DOORWAY_TRUSTED_PROXY_HOPS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn peer(ip: [u8; 4]) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::from(ip)), 12345)
    }

    // ── client_ip_from_xff ──────────────────────────────────────────────────

    #[test]
    fn xff_no_header_returns_peer() {
        assert_eq!(client_ip_from_xff(None, &peer([10, 0, 0, 1]), 1), "10.0.0.1");
    }

    #[test]
    fn xff_single_ingress_extracts_leftmost_of_rightmost() {
        // "1.2.3.4, 10.10.0.1" — ingress is rightmost; client is 1.2.3.4.
        assert_eq!(
            client_ip_from_xff(Some("1.2.3.4, 10.10.0.1"), &peer([10, 0, 0, 1]), 1),
            "1.2.3.4"
        );
    }

    #[test]
    fn xff_too_short_falls_back_to_peer() {
        // Header has 1 entry but trusted_hops=1 → too short.
        assert_eq!(
            client_ip_from_xff(Some("10.10.0.1"), &peer([10, 0, 0, 1]), 1),
            "10.0.0.1"
        );
    }

    #[test]
    fn xff_two_hops_extracts_correct_position() {
        // "1.2.3.4, 172.16.0.1, 10.0.0.1" — 2 trusted hops → client is 1.2.3.4.
        assert_eq!(
            client_ip_from_xff(Some("1.2.3.4, 172.16.0.1, 10.0.0.1"), &peer([10, 0, 0, 1]), 2),
            "1.2.3.4"
        );
    }

    // ── derive_source ───────────────────────────────────────────────────────

    #[test]
    fn derive_source_authenticated_uses_agent_prefix() {
        assert_eq!(derive_source(Some("human-abc"), "1.2.3.4"), "agent:human-abc");
    }

    #[test]
    fn derive_source_unauthenticated_uses_ip_prefix() {
        assert_eq!(derive_source(None, "1.2.3.4"), "ip:1.2.3.4");
    }

    #[test]
    fn derive_source_empty_human_id_falls_back_to_ip() {
        assert_eq!(derive_source(Some(""), "5.6.7.8"), "ip:5.6.7.8");
    }

    // ── is_static_asset ─────────────────────────────────────────────────────

    #[test]
    fn static_asset_js_css_woff2_exempted() {
        assert!(is_static_asset("/main.abc123.js"));
        assert!(is_static_asset("/styles.css"));
        assert!(is_static_asset("/fonts/inter.woff2"));
        assert!(is_static_asset("/assets/logo.png"));
    }

    #[test]
    fn api_and_html_not_exempted() {
        assert!(!is_static_asset("/api/v1/content"));
        assert!(!is_static_asset("/lamad"));
        assert!(!is_static_asset("/health"));
        assert!(!is_static_asset("/index.html"));
    }

    // ── EdgeGuardStore ──────────────────────────────────────────────────────

    #[test]
    fn record_and_count_within_window() {
        let mut store = EdgeGuardStore::new(60);
        store.record("ip:1.2.3.4", 1000);
        store.record("ip:1.2.3.4", 1010);
        assert_eq!(store.count_since("ip:1.2.3.4", 990), 2);
    }

    #[test]
    fn count_excludes_hits_before_since() {
        let mut store = EdgeGuardStore::new(60);
        store.record("ip:1.2.3.4", 900); // old
        store.record("ip:1.2.3.4", 1000); // recent
        // since=950 → only hit at 1000 counts.
        assert_eq!(store.count_since("ip:1.2.3.4", 950), 1);
    }

    #[test]
    fn ban_and_is_banned() {
        let mut store = EdgeGuardStore::new(60);
        store.ban_until("ip:bad", 2000);
        assert!(store.is_banned("ip:bad", 1999));
        assert!(!store.is_banned("ip:bad", 2001));
    }

    #[test]
    fn sweep_idle_removes_empty_sources() {
        let mut store = EdgeGuardStore::new(60);
        // Record a hit 70 secs ago (out of window) and no ban.
        store.record("ip:old", 930);
        // At t=1000, cutoff=940, so 930 < 940 → hit is stale.
        store.sweep_idle(1000);
        assert_eq!(store.count_since("ip:old", 0), 0);
        assert!(!store.sources.contains_key("ip:old"), "idle source must be evicted");
    }

    #[test]
    fn sweep_idle_keeps_active_ban() {
        let mut store = EdgeGuardStore::new(60);
        store.ban_until("ip:banned", 5000);
        store.sweep_idle(1000);
        assert!(store.sources.contains_key("ip:banned"), "banned source must survive sweep");
    }

    #[test]
    fn ban_count_tracks_active_bans() {
        let mut store = EdgeGuardStore::new(60);
        assert_eq!(store.ban_count, 0);
        store.ban_until("ip:a", 5000);
        assert_eq!(store.ban_count, 1);
        store.ban_until("ip:b", 5000);
        assert_eq!(store.ban_count, 2);
        // Banning the same source again does NOT double-count.
        store.ban_until("ip:a", 6000);
        assert_eq!(store.ban_count, 2);
        // Sweep at t=10000 → both bans expired → both entries removed.
        store.sweep_idle(10000);
        assert_eq!(store.ban_count, 0);
    }

    // ── EdgeClock ───────────────────────────────────────────────────────────

    #[test]
    fn edge_clock_returns_nonzero_secs() {
        let clk = EdgeClock;
        assert!(clk.now_secs() > 0, "clock must return a positive unix timestamp");
    }
}
