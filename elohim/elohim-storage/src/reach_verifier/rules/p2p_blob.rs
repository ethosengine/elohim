//! Rule row for libp2p blob serving.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// The blob protocol serves requested addresses without a row-level check.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::P2pBlob, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P2");
