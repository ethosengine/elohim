//! Rule row for peer announcements.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Announced and response-cached heads are admitted without a reach check.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::GossipAnnounce, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P6");
