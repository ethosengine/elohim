//! Heartbeat task — periodic policy evaluation + `PeerStatus` publication
//! driven by a lifecycle state machine.
//!
//! Task 12 of Peer-Stewarded Availability Phase 1.
//!
//! The task is split behind two traits so unit tests can inject fakes:
//! - [`Publisher`] — publishes `PeerStatus` (real impl: ZomeCall, Task 13).
//! - [`LiveProbe`] — samples runtime `LiveState` (real impl: blob store +
//!   conductor probe, Task 13).
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
