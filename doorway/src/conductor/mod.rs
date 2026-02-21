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

pub use pool_map::{ConductorPoolMap, ConductorPoolStatus};
pub use provisioner::{AgentProvisioner, ProvisionedAgent};
pub use registry::{ConductorEntry, ConductorInfo, ConductorRegistry};
pub use router::ConductorRouter;
pub use typed_admin::TypedAdminClient;
