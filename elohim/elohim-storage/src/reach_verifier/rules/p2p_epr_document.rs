//! Rule row for EPR protocol document reads.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Document requests use the protocol handler's document lookup behavior.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::P2pEprDocument, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P1");
