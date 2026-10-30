//! Rule row for native iroh-blobs serving.

use super::{RuleSet, Status};
use crate::reach_verifier::EgressPlane;

/// Native iroh blob requests serve addressed bytes through the blob protocol.
pub const RULES: RuleSet = RuleSet::pending_permit(EgressPlane::IrohBlobs, STATUS);
/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P3");
