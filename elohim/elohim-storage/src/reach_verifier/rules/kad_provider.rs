//! Rule row for Kademlia provider publication.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Atom provider publication advertises every selected atom CID.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::KadProvider, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P6");
