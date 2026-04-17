//! Heartbeat task — periodic policy evaluation + `PeerStatus` publication
//! driven by a lifecycle state machine.
//!
//! Task 12 of Peer-Stewarded Availability Phase 1.
//!
//! The task is split behind two traits so unit tests can inject fakes:
//! - [`Publisher`] — publishes `PeerStatus` (real impl: [`ZomeCallPublisher`],
//!   Task 13).
//! - [`LiveProbe`] — samples runtime `LiveState` (real impl: [`DefaultProbe`],
//!   Task 13).
//!
//! Lifecycle transitions driven by `tick_once`:
//! - Starting → Online on first tick (peer has its first reading)
//! - Online → Degraded when `general_pool_member` drops false
//! - Degraded → Online when `general_pool_member` returns true
//! - Maintenance / Leaving are externally driven (not flipped by tick)

use crate::policy::{evaluate, EvaluatedFlags, LiveState, PolicyConfig};
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct Published {
    pub status: String,
    pub flags: EvaluatedFlags,
    pub archetype_class: Option<String>,
}

#[async_trait::async_trait]
pub trait Publisher: Send + Sync + 'static {
    async fn publish(&self, p: Published) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait LiveProbe: Send + Sync + 'static {
    async fn sample(&self) -> anyhow::Result<LiveState>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum LifecycleState {
    Starting,
    Online,
    Degraded,
    /// Operator-driven maintenance mode. Phase 2 will add a setter for the
    /// co-stewardship directive channel to transition into this state.
    #[allow(dead_code)]
    Maintenance,
    Leaving,
}

impl LifecycleState {
    fn as_str(self) -> &'static str {
        match self {
            LifecycleState::Starting => "starting",
            LifecycleState::Online => "online",
            LifecycleState::Degraded => "degraded",
            LifecycleState::Maintenance => "maintenance",
            LifecycleState::Leaving => "leaving",
        }
    }
}

pub struct HeartbeatTask<P: Publisher, L: LiveProbe> {
    cfg: PolicyConfig,
    publisher: P,
    probe: L,
    archetype_class: Option<String>,
    lifecycle: tokio::sync::Mutex<LifecycleState>,
}

impl<P: Publisher, L: LiveProbe> HeartbeatTask<P, L> {
    pub fn new(cfg: PolicyConfig, publisher: P, probe: L) -> Self {
        Self {
            cfg,
            publisher,
            probe,
            archetype_class: None,
            lifecycle: tokio::sync::Mutex::new(LifecycleState::Starting),
        }
    }

    pub fn with_archetype_class(mut self, class: String) -> Self {
        self.archetype_class = Some(class);
        self
    }

    pub async fn tick_once(&self) -> anyhow::Result<()> {
        let state = self.probe.sample().await?;
        let flags = evaluate(&self.cfg, &state);
        let mut lifecycle = self.lifecycle.lock().await;
        // Transition rules:
        //   Starting -> Online on first tick (peer has its first reading)
        //   Online -> Degraded when general_pool_member drops false
        //   Degraded -> Online when general_pool_member returns true
        //   Maintenance / Leaving are externally driven (not flipped by tick)
        match *lifecycle {
            LifecycleState::Starting => *lifecycle = LifecycleState::Online,
            LifecycleState::Online if !flags.general_pool_member => {
                *lifecycle = LifecycleState::Degraded;
            }
            LifecycleState::Degraded if flags.general_pool_member => {
                *lifecycle = LifecycleState::Online;
            }
            _ => {}
        }
        let status = lifecycle.as_str().to_string();
        drop(lifecycle);
        self.publisher
            .publish(Published {
                status,
                flags,
                archetype_class: self.archetype_class.clone(),
            })
            .await
    }

    pub async fn announce_leaving(&self) -> anyhow::Result<()> {
        let mut lifecycle = self.lifecycle.lock().await;
        *lifecycle = LifecycleState::Leaving;
        drop(lifecycle);
        // Publish one final Leaving status with current state snapshot.
        let state = self.probe.sample().await.unwrap_or(LiveState {
            free_storage_pct: 0,
            conductor_healthy: false,
        });
        let flags = evaluate(&self.cfg, &state);
        self.publisher
            .publish(Published {
                status: "leaving".into(),
                flags,
                archetype_class: self.archetype_class.clone(),
            })
            .await
    }

    pub async fn run(self, mut shutdown: broadcast::Receiver<()>) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        // Skip the first immediate tick — we'll do it explicitly below so the
        // first heartbeat fires immediately at startup rather than after 60s.
        interval.tick().await;
        if let Err(e) = self.tick_once().await {
            tracing::warn!("initial heartbeat tick failed: {e}");
        }
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.tick_once().await {
                        tracing::warn!("heartbeat tick failed: {e}");
                    }
                }
                _ = shutdown.recv() => {
                    if let Err(e) = self.announce_leaving().await {
                        tracing::warn!("leaving announcement failed: {e}");
                    }
                    break;
                }
            }
        }
    }
}

// =============================================================================
// Real Publisher + LiveProbe implementations (Task 13)
// =============================================================================

/// Real [`Publisher`] that commits a `PeerStatus` DHT entry via a signed zome
/// call to the `infrastructure` coordinator (`record_peer_status`).
///
/// The payload is serialized as MessagePack (with named fields) to match the
/// DNA-side `PeerStatus` entry layout. See
/// `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/peer_status.rs`.
pub struct ZomeCallPublisher {
    hc: std::sync::Arc<crate::hc_client::HcClient>,
    agent: holochain_types::prelude::AgentPubKey,
}

impl ZomeCallPublisher {
    pub fn new(
        hc: std::sync::Arc<crate::hc_client::HcClient>,
        agent: holochain_types::prelude::AgentPubKey,
    ) -> Self {
        Self { hc, agent }
    }
}

/// Wire-side mirror of the DNA `PeerLifecycleState` enum.
///
/// MUST match variant names exactly — serde externally-tagged encoding writes
/// the variant name as a MessagePack string, which is what the DNA integrity
/// zome expects when it deserializes the entry.
#[derive(serde::Serialize)]
enum WirePeerLifecycleState {
    Starting,
    Online,
    Degraded,
    Maintenance,
    Leaving,
}

impl WirePeerLifecycleState {
    fn parse(s: &str) -> Self {
        match s {
            "starting" => Self::Starting,
            "degraded" => Self::Degraded,
            "maintenance" => Self::Maintenance,
            "leaving" => Self::Leaving,
            _ => Self::Online,
        }
    }
}

/// Wire-side mirror of the DNA `PeerCapabilityFlags` struct.
#[derive(serde::Serialize)]
struct WirePeerCapabilityFlags {
    general_pool_member: bool,
    accepting_stewardship_reserves: bool,
}

/// Wire-side mirror of the DNA `PeerStatus` entry. Field names and order
/// MUST match the integrity zome definition.
#[derive(serde::Serialize)]
struct WirePeerStatus {
    peer_id: holochain_types::prelude::AgentPubKey,
    status: WirePeerLifecycleState,
    flags: WirePeerCapabilityFlags,
    archetype_class: Option<String>,
    timestamp: holochain_types::prelude::Timestamp,
}

#[async_trait::async_trait]
impl Publisher for ZomeCallPublisher {
    async fn publish(&self, p: Published) -> anyhow::Result<()> {
        let wire = WirePeerStatus {
            peer_id: self.agent.clone(),
            status: WirePeerLifecycleState::parse(&p.status),
            flags: WirePeerCapabilityFlags {
                general_pool_member: p.flags.general_pool_member,
                accepting_stewardship_reserves: p.flags.accepting_stewardship_reserves,
            },
            archetype_class: p.archetype_class,
            timestamp: holochain_types::prelude::Timestamp::now(),
        };

        let payload = rmp_serde::to_vec_named(&wire)
            .map_err(|e| anyhow::anyhow!("encode PeerStatus: {e}"))?;

        self.hc
            .call_zome("infrastructure", "record_peer_status", payload)
            .await
            .map_err(|e| anyhow::anyhow!("record_peer_status zome call failed: {e}"))?;
        Ok(())
    }
}

/// Real [`LiveProbe`] combining a blob store handle (for free-storage sampling)
/// and an [`HcClient`](crate::hc_client::HcClient) handle (for a conductor ping).
///
/// **Free-storage sampling (v1 fallback):** the blob store does not currently
/// expose a disk-utilization API, and no other part of elohim-storage measures
/// filesystem free space yet. Rather than pull in a libc/statvfs dependency
/// in this task, the probe returns an optimistic `free_storage_pct = 100`.
///
/// The policy default for `accept_general_traffic` is `Auto`, which — combined
/// with this value — keeps peers in the general pool. That is the correct
/// behaviour for typical Phase 1 deployments where operators want their node
/// serving traffic by default. The probe MUST be upgraded to a real disk
/// measurement before thin-client deployment in Phase 2, at which point
/// operators with low-disk nodes will rely on the `min_free_storage_pct`
/// threshold to gate their participation.
pub struct DefaultProbe {
    #[allow(dead_code)]
    blob: std::sync::Arc<crate::blob_store::BlobStore>,
    hc: std::sync::Arc<crate::hc_client::HcClient>,
}

impl DefaultProbe {
    pub fn new(
        blob: std::sync::Arc<crate::blob_store::BlobStore>,
        hc: std::sync::Arc<crate::hc_client::HcClient>,
    ) -> Self {
        Self { blob, hc }
    }
}

#[async_trait::async_trait]
impl LiveProbe for DefaultProbe {
    async fn sample(&self) -> anyhow::Result<LiveState> {
        // Free storage %: see struct docs for fallback rationale.
        let free_storage_pct: u8 = 100;

        // Conductor health: a successful `list_apps` via HcClient::ping is a
        // reasonable liveness check — admin websocket is responsive AND the
        // conductor's app manager is answering queries.
        let conductor_healthy = match self.hc.ping().await {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("conductor ping failed: {e}");
                false
            }
        };

        Ok(LiveState {
            free_storage_pct,
            conductor_healthy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::config::{NetworkConfig, PoolConfig, StewardshipConfig};
    use crate::policy::AutoOrBool;

    fn base_cfg() -> PolicyConfig {
        PolicyConfig {
            pool: PoolConfig {
                accept_general_traffic: AutoOrBool::Auto,
                min_free_storage_pct: 20,
                require_conductor_healthy: true,
            },
            stewardship: StewardshipConfig {
                accept_new_reserves: AutoOrBool::Auto,
                max_storage_pct: 80,
            },
            network: NetworkConfig {
                expose_conductor_externally: false,
                conductor_external_bind: "0.0.0.0:4445".into(),
                conductor_internal_port: 4445,
            },
        }
    }

    struct TestPublisher {
        tx: tokio::sync::mpsc::UnboundedSender<Published>,
    }

    #[async_trait::async_trait]
    impl Publisher for TestPublisher {
        async fn publish(&self, p: Published) -> anyhow::Result<()> {
            self.tx.send(p)?;
            Ok(())
        }
    }

    struct TestProbe {
        state: LiveState,
    }

    #[async_trait::async_trait]
    impl LiveProbe for TestProbe {
        async fn sample(&self) -> anyhow::Result<LiveState> {
            Ok(self.state)
        }
    }

    #[tokio::test]
    async fn first_tick_transitions_starting_to_online() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let task = HeartbeatTask::new(
            base_cfg(),
            TestPublisher { tx },
            TestProbe {
                state: LiveState {
                    free_storage_pct: 50,
                    conductor_healthy: true,
                },
            },
        );
        task.tick_once().await.unwrap();
        let p = rx.recv().await.unwrap();
        assert_eq!(p.status, "online");
        assert!(p.flags.general_pool_member);
    }

    #[tokio::test]
    async fn degraded_on_unhealthy_conductor() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let task = HeartbeatTask::new(
            base_cfg(),
            TestPublisher { tx },
            TestProbe {
                state: LiveState {
                    free_storage_pct: 50,
                    conductor_healthy: false,
                },
            },
        );
        task.tick_once().await.unwrap(); // Starting -> Online (first tick always transitions)
        let _ = rx.recv().await.unwrap(); // online
        task.tick_once().await.unwrap(); // Online -> Degraded since flags.general_pool_member=false
        let p = rx.recv().await.unwrap();
        assert_eq!(p.status, "degraded");
        assert!(!p.flags.general_pool_member);
    }

    #[tokio::test]
    async fn announce_leaving_publishes_leaving_status() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let task = HeartbeatTask::new(
            base_cfg(),
            TestPublisher { tx },
            TestProbe {
                state: LiveState {
                    free_storage_pct: 50,
                    conductor_healthy: true,
                },
            },
        );
        task.announce_leaving().await.unwrap();
        let p = rx.recv().await.unwrap();
        assert_eq!(p.status, "leaving");
    }

    #[tokio::test]
    async fn archetype_class_propagates() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let task = HeartbeatTask::new(
            base_cfg(),
            TestPublisher { tx },
            TestProbe {
                state: LiveState {
                    free_storage_pct: 50,
                    conductor_healthy: true,
                },
            },
        )
        .with_archetype_class("home-nuc".into());
        task.tick_once().await.unwrap();
        let p = rx.recv().await.unwrap();
        assert_eq!(p.archetype_class.as_deref(), Some("home-nuc"));
    }
}
