//! Rule row for view federation.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// View federation relies on the agent identity carried in the request body.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::P2pViewFed, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P9");
