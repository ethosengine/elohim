//! Governance modules for layer-aware decision making.
//!
//! - **Layer context**: Understanding which constitutional layer applies
//! - **Escalation**: Moving decisions to higher authority layers
//! - **Subsidiarity**: Keeping decisions at lowest appropriate level

mod escalation;
mod layer_context;
mod subsidiarity;

pub use escalation::{EscalationManager, EscalationRequest, EscalationReason};
pub use layer_context::{LayerContext, LayerContextBuilder};
pub use subsidiarity::{SubsidiarityChecker, SubsidiarityResult};
