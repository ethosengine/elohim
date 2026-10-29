//! Rule row for raw HTTP shard, IPFS, and DAG reads.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Raw HTTP shard, IPFS, and DAG routes serve addressed bytes directly.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::HttpRaw, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P11");
