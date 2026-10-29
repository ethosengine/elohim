//! Rule row for HTTP EPR head reads.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// HTTP EPR head reads serve every locally resolved head.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::HttpEprHead, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P11");
