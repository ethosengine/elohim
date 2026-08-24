//! Transport-manifest announcer — the iroh plane's "here is where to dial
//! me", published on the gossip fan-out at a slow cadence.
//!
//! Pairs with the receive arm in `p2p::gossip_dispatch::handle_transport_manifest`
//! and the joiner in `gossip_receive`. See `p2p::transport_manifest_gossip`
//! for the why.
//!
//! The announcement is signed with the iroh secret key, so its `iroh_node_id`
//! is self-certifying. `agent_cid` / `libp2p_peer_id` are resolved once at
//! spawn (they do not change for the life of the process) and ride along as
//! routing hints.

use std::sync::Arc;
use std::time::Duration;

use iroh::{Endpoint, Watcher};
use tracing::{debug, info, warn};

use crate::p2p::transport_manifest_gossip::{
    TransportManifestAnnouncement, ANNOUNCE_INTERVAL_ENV, DEFAULT_ANNOUNCE_INTERVAL_SECS,
    TRANSPORT_MANIFEST_TOPIC,
};
use crate::services::gossip_flood::GossipPublisher;

/// Static inputs for the announcer.
pub struct AnnouncerInputs {
    pub endpoint: Endpoint,
    /// The iroh secret key bytes — the announcement's signer.
    pub secret: [u8; 32],
    pub agent_cid: Option<String>,
    pub libp2p_peer_id: Option<String>,
    /// Planes served over iroh (kebab-case, as `peer_map::Plane::as_str`).
    pub planes: Vec<String>,
    /// Where to publish. In dual mode this is the `DualGossipPublisher`, so
    /// the manifest rides libp2p gossipsub (the leg that HAS peers at boot)
    /// and iroh-gossip (useful once the plane has neighbors).
    pub publisher: Arc<dyn GossipPublisher>,
    /// Doorway base URL, when this node has one
    /// ([`super::doorway_bootstrap::doorway_base_url`]). The same signed
    /// announcement is POSTed to the doorway's bounded bulletin board so a
    /// PURE-IROH peer — which can reach no gossip topic until its book is
    /// seeded, and cannot seed its book without gossip — is discoverable at
    /// all. `None` leaves the announcer exactly as it was: gossip only.
    pub doorway_url: Option<String>,
}

/// Cadence from env (`ELOHIM_TRANSPORT_MANIFEST_INTERVAL_SECS`) or default.
pub fn announce_interval() -> Duration {
    let secs = std::env::var(ANNOUNCE_INTERVAL_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(DEFAULT_ANNOUNCE_INTERVAL_SECS);
    Duration::from_secs(secs)
}

/// Build one signed announcement from the endpoint's CURRENT address set.
/// Returns `None` when the endpoint has no address yet (bind still in
/// progress) — the caller just waits for the next tick.
pub fn build_announcement(
    inputs: &AnnouncerInputs,
    now_ms: i64,
) -> Option<TransportManifestAnnouncement> {
    let addr = inputs.endpoint.node_addr().get()?;
    let direct: Vec<String> = addr
        .direct_addresses
        .iter()
        .map(|a| a.to_string())
        .collect();
    let relay = addr.relay_url.as_ref().map(|r| r.to_string());
    if direct.is_empty() && relay.is_none() {
        return None;
    }
    Some(TransportManifestAnnouncement::sign(
        &inputs.secret,
        direct,
        relay,
        inputs.agent_cid.clone(),
        inputs.libp2p_peer_id.clone(),
        inputs.planes.clone(),
        now_ms,
    ))
}

/// Spawn the announcer: wait for the endpoint's first address, then publish
/// immediately and on every `interval` tick. Never panics on publish errors
/// (backpressure/closed channel are logged and retried next tick).
pub fn spawn_transport_manifest_announcer(
    inputs: AnnouncerInputs,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // First address — on loopback this is immediate; with a relay it waits
        // for the home relay to resolve.
        let first = inputs.endpoint.node_addr().initialized().await;
        info!(
            node = %first.node_id,
            direct = ?first.direct_addresses,
            relay = ?first.relay_url,
            interval_secs = interval.as_secs(),
            "transport manifest announcer: endpoint addressed — announcing"
        );
        // bounded-work: one signed announcement per `interval` tick (30s
        // default), one gossip publish per tick, no retry ladder — a failed
        // publish waits for the next tick.
        // Built once: the doorway leg is best-effort and must not add a client
        // build per tick. `None` (no doorway, or no client) keeps it inert.
        let doorway = inputs
            .doorway_url
            .clone()
            .and_then(|url| super::doorway_bootstrap::doorway_client().map(|c| (url, c)));
        if let Some((url, _)) = doorway.as_ref() {
            info!(doorway = %url, "transport manifest announcer: also posting to the doorway bulletin board");
        }
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let now_ms = chrono::Utc::now().timestamp_millis();
            let Some(ann) = build_announcement(&inputs, now_ms) else {
                debug!("transport manifest announcer: no address yet — skipping tick");
                continue;
            };
            match ann.to_bytes() {
                Ok(bytes) => {
                    if let Err(e) = inputs.publisher.publish(TRANSPORT_MANIFEST_TOPIC, bytes) {
                        warn!(error = %e, "transport manifest announcer: publish failed — retry next tick");
                    } else {
                        debug!(
                            addrs = ?ann.iroh_direct_addrs,
                            "transport manifest announcer: published"
                        );
                    }
                }
                Err(e) => warn!(error = %e, "transport manifest announcer: encode failed"),
            }
            // Same announcement, same tick, second channel. Failures are logged
            // at debug inside `post_manifest` and retried next tick — the
            // doorway is a convenience, never a dependency.
            if let Some((url, client)) = doorway.as_ref() {
                super::doorway_bootstrap::post_manifest(client, url, &ann).await;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p_iroh::IrohConfig;
    use std::sync::Mutex;

    struct CapturePublisher(Mutex<Vec<(String, Vec<u8>)>>);
    impl GossipPublisher for CapturePublisher {
        fn publish(
            &self,
            topic: &str,
            payload: Vec<u8>,
        ) -> Result<(), crate::services::gossip_flood::PublishError> {
            self.0.lock().unwrap().push((topic.to_string(), payload));
            Ok(())
        }
    }

    #[tokio::test]
    async fn announcer_publishes_a_verifiable_manifest_for_its_own_endpoint() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = IrohConfig::from_storage_dir(dir.path());
        let secret = crate::p2p_iroh::load_or_generate_secret_key(&cfg.secret_key_path).unwrap();
        let endpoint = crate::p2p_iroh::build_endpoint(&cfg).await.unwrap();
        let publisher = Arc::new(CapturePublisher(Mutex::new(Vec::new())));
        let inputs = AnnouncerInputs {
            endpoint: endpoint.clone(),
            secret: secret.to_bytes(),
            agent_cid: Some("uhCAktest".into()),
            libp2p_peer_id: None,
            planes: vec!["sync".into()],
            publisher: publisher.clone(),
            doorway_url: None,
        };
        let handle = spawn_transport_manifest_announcer(inputs, Duration::from_millis(200));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            if !publisher.0.lock().unwrap().is_empty() {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "no announcement published"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        handle.abort();
        let (topic, bytes) = publisher.0.lock().unwrap()[0].clone();
        assert_eq!(topic, TRANSPORT_MANIFEST_TOPIC);
        let ann = TransportManifestAnnouncement::from_bytes(&bytes).unwrap();
        assert_eq!(ann.verify(), Ok(()));
        assert_eq!(ann.iroh_node_id, endpoint.node_id().to_string());
        assert!(!ann.iroh_direct_addrs.is_empty());
        assert_eq!(ann.agent_cid.as_deref(), Some("uhCAktest"));
        endpoint.close().await;
    }
}
