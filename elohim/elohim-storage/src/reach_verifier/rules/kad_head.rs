//! Rule row for Kademlia EPR head publication.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Head publication places every selected head into Kademlia.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::KadHead, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P6");
