//! Rule row for the HTTP EPR atom API.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P10");

/// HTTP EPR reads serve atoms admitted by their route-local predicate.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::HttpEprApi, STATUS);
