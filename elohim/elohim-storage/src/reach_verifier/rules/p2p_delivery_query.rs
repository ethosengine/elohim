//! Rule row for delivery-capability queries.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Delivery queries reveal whether the requested blob is locally warm.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::P2pDeliveryQuery, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P1");
