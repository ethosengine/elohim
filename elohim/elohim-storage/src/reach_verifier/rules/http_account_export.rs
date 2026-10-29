//! Rule row for account package export.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Account export relies on the route's direct-caller identity checks.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::HttpAccountExport, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P11");
