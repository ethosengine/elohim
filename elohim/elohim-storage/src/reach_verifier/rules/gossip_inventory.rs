//! Rule row for inventory gossip.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Inventory snapshots and deltas publish every locally selected hash.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::GossipInventory, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P7");
