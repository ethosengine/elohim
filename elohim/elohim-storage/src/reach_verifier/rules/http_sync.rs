//! Rule row for HTTP Automerge synchronization.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// HTTP sync reads serve documents selected by namespace and document id.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::HttpSync, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P11");
