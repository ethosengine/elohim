//! Shared lifecycle seams. Default methods refuse rather than simulate delivery.
//! Concrete adapters must check grants, durable writes and compatibility themselves.
use crate::{contracts::Answer, Result, SdkError};
use async_trait::async_trait;
pub use elohim_views::app_lifecycle::*;
use elohim_views::{ObservationAcceptedView, ObservationIntentView};

mod sync_demand;
pub use sync_demand::SyncDemandQueue;

/// Resolve elected releases, then ensure an instance under independently checked authority.
#[async_trait]
pub trait AppRuntime: Send + Sync {
    async fn resolve(&self, _app_id: &str, _target: &str) -> Result<Answer<AppReleaseView>> {
        Err(SdkError::Unsupported("app.resolve"))
    }
    /// Replay of operation_id returns the same instance, never another process.
    async fn ensure_instance(&self, _request: &AppOpenInput) -> Result<AppInstanceView> {
        Err(SdkError::Unsupported("app.ensureInstance"))
    }
    async fn observe(&self, _instance_id: &str) -> Result<Answer<AppInstanceView>> {
        Err(SdkError::Unsupported("app.observe"))
    }
}

/// Storage implements verification and existing acquisition pins/holds behind this seam.
#[async_trait]
pub trait AppResidency: Send + Sync {
    async fn acquire(&self, _release: &AppReleaseView) -> Result<AppResidencyView> {
        Err(SdkError::Unsupported("app.acquire"))
    }
    async fn pin(&self, _release_cid: &str, _keep: bool) -> Result<AppResidencyView> {
        Err(SdkError::Unsupported("app.pin"))
    }
    /// Releases only eligible executable bytes. Personal and unsynced state survives.
    async fn archive(&self, _release_cid: &str) -> Result<AppResidencyView> {
        Err(SdkError::Unsupported("app.archive"))
    }
}

/// Checkpoint existing private working state and activate once after readiness.
#[async_trait]
pub trait AppContinuation: Send + Sync {
    async fn prepare(&self, _instance_id: &str) -> Result<AppContinuationView> {
        Err(SdkError::Unsupported("app.prepareContinuation"))
    }
    /// The source stays usable until activation succeeds. No signing-key migration.
    async fn activate(
        &self,
        _target: &AppInstanceView,
        _checkpoint: &AppContinuationView,
        _operation_id: &str,
    ) -> Result<()> {
        Err(SdkError::Unsupported("app.activateContinuation"))
    }
}

/// Doorway's browser accommodation; peer-native execution never depends on it.
#[async_trait]
pub trait AppAttachment: Send + Sync {
    async fn attach(&self, _request: &AppAttachmentInput) -> Result<AppRouteView> {
        Err(SdkError::Unsupported("app.attach"))
    }
    async fn detach(&self, _instance_id: &str) -> Result<()> {
        Err(SdkError::Unsupported("app.detach"))
    }
}

/// Durable pending observations reuse the existing observation intent/receipt.
/// An append result is a private retry handle, never evidence of witnessing.
#[async_trait]
pub trait AppObservationJournal: Send + Sync {
    async fn append(&self, _intent: &ObservationIntentView) -> Result<String> {
        Err(SdkError::Unsupported("observation.append"))
    }
    async fn acknowledge(
        &self,
        _pending_id: &str,
        _receipt: &ObservationAcceptedView,
    ) -> Result<()> {
        Err(SdkError::Unsupported("observation.acknowledge"))
    }
}

/// One demand interface over the existing authorized reconciliation worker.
#[async_trait]
pub trait AppSync: Send + Sync {
    async fn request(&self, _demand: &AppSyncDemandView) -> Result<String> {
        Err(SdkError::Unsupported("sync.request"))
    }
    /// Reads per-device progress toward the fixed request targets.
    async fn progress(&self, _demand_id: &str) -> Result<Vec<AppSyncProgressView>> {
        Err(SdkError::Unsupported("sync.progress"))
    }
    /// Releases demand/priority, not durable queued work.
    async fn release(&self, _demand_id: &str) -> Result<()> {
        Err(SdkError::Unsupported("sync.release"))
    }
}

/// Explicitly unimplemented adapter, useful during staged consumer migration.
pub struct UnsupportedLifecycle;
impl AppRuntime for UnsupportedLifecycle {}
impl AppResidency for UnsupportedLifecycle {}
impl AppContinuation for UnsupportedLifecycle {}
impl AppAttachment for UnsupportedLifecycle {}
impl AppObservationJournal for UnsupportedLifecycle {}
impl AppSync for UnsupportedLifecycle {}

/// C4/C7: unknown verification never promises offline capability.
pub fn available_offline(residency: &AppResidencyView) -> bool {
    residency.artifacts_verified == Some(true) && residency.data_verified == Some(true)
}
