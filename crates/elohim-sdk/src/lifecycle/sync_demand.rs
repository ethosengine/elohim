//! Bounded in-memory demand coordination, not a persistence or transport queue.
//! A worker uses one scope entry for all consumers; each demand freezes its targets.
use super::{AppSyncDemandView, AppSyncProgressView, AppSyncUrgency};
use crate::contracts::demand::{DemandLeases, DemandRefusal};
use crate::contracts::Answer;
use crate::{Result, SdkError};
use std::collections::BTreeMap;

/// Operational demand leases. The owner releases them on completion, failure or cancellation.
/// No timer, network call or observation write occurs here.
pub struct SyncDemandQueue {
    max_targets: usize,
    demands: DemandLeases<String, AppSyncDemandView>,
}

impl SyncDemandQueue {
    pub fn new(max_demands: usize, max_targets: usize) -> Self {
        Self {
            max_targets,
            demands: DemandLeases::new(max_demands),
        }
    }

    /// Freeze the caller's observed ends. Identical scopes share worker demand,
    /// but distinct leases prevent one consumer's cancellation releasing another's priority.
    pub fn request(&mut self, demand: AppSyncDemandView) -> Result<u64> {
        if demand.targets.len() > self.max_targets {
            return Err(SdkError::BackpressureFull(100));
        }
        if demand.scope.is_empty() || demand.targets.is_empty() {
            return Err(SdkError::Config(
                "sync demand requires scope and observed targets".into(),
            ));
        }
        let mut keys = std::collections::BTreeSet::new();
        for target in &demand.targets {
            if target.device_id.is_empty()
                || target.stream_id.is_empty()
                || target.epoch.is_empty()
                || !keys.insert((&target.device_id, &target.stream_id))
            {
                return Err(SdkError::Config(
                    "sync target must name one device/stream/epoch".into(),
                ));
            }
            if let Some(end) = &target.end_position {
                if end.is_empty()
                    || !end.bytes().all(|c| c.is_ascii_digit())
                    || end.parse::<u64>().is_err()
                {
                    return Err(SdkError::Config(
                        "sync position must be a decimal u64".into(),
                    ));
                }
            }
        }
        self.demands
            .insert(
                demand.scope.clone(),
                demand.urgency == AppSyncUrgency::Foreground,
                demand,
            )
            .map_err(|reason| match reason {
                DemandRefusal::Capacity => SdkError::BackpressureFull(100),
                DemandRefusal::HandleExhausted => {
                    SdkError::Config("sync demand handle exhausted".into())
                }
            })
    }

    /// At most one worker entry per scope, regardless of the number of dashboard requests.
    /// Resource/power admission is still the resident runtime's decision.
    pub fn scopes(&self) -> BTreeMap<&str, AppSyncUrgency> {
        self.demands
            .work()
            .into_iter()
            .map(|(scope, foreground)| {
                (
                    scope.as_str(),
                    if foreground {
                        AppSyncUrgency::Foreground
                    } else {
                        AppSyncUrgency::Background
                    },
                )
            })
            .collect()
    }

    /// Reads the original immutable targets; subsequent remote arrivals cannot extend them.
    pub fn demand(&self, id: u64) -> Option<&AppSyncDemandView> {
        self.demands.target(id)
    }

    /// Reads fixed-target progress without collapsing unknown into behind.
    pub fn progress(
        &self,
        id: u64,
        mut local: impl FnMut(&str, &str) -> Answer<(String, u64)>,
        mut remote_reachable: impl FnMut(&str) -> Option<bool>,
    ) -> Option<Vec<AppSyncProgressView>> {
        Some(
            self.demands
                .target(id)?
                .targets
                .iter()
                .map(|target| {
                    let position = match local(&target.device_id, &target.stream_id) {
                        Answer::Present(position) => Some(position),
                        Answer::Absent | Answer::Unreachable => None,
                    };
                    let caught_up = position.as_ref().and_then(|(epoch, position)| {
                        if epoch != &target.epoch {
                            return None;
                        }
                        target
                            .end_position
                            .as_ref()?
                            .parse::<u64>()
                            .ok()
                            .map(|end| *position >= end)
                    });
                    AppSyncProgressView {
                        target: target.clone(),
                        reachable: remote_reachable(&target.device_id),
                        local_epoch: position.as_ref().map(|(epoch, _)| epoch.clone()),
                        local_position: position.as_ref().map(|(_, position)| position.to_string()),
                        caught_up,
                    }
                })
                .collect(),
        )
    }

    /// C4: the callback returns an observed local (epoch, position), or None if unknown.
    /// Empty/unknown targets never constitute a successful refresh.
    pub fn caught_up(
        &self,
        id: u64,
        mut local: impl FnMut(&str, &str) -> Option<(String, u64)>,
    ) -> bool {
        self.progress(
            id,
            |device, stream| {
                local(device, stream)
                    .map(Answer::Present)
                    .unwrap_or(Answer::Unreachable)
            },
            |_| None,
        )
        .is_some_and(|progress| progress.iter().all(|p| p.caught_up == Some(true)))
    }

    /// Safe to repeat. This removes urgency only, never observations or persisted sync work.
    pub fn release(&mut self, id: u64) {
        self.demands.release(id);
    }
}
