//! Rule row and address fold for HTTP blob bytes.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status, TierClass};
use crate::blob_reach::BlobReference;
use crate::epr_service::reach_level_index;
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption status for the HTTP blob plane.
pub const STATUS: Status = Status::Pending("P12");

/// HTTP blobs serve on any open or identity-present reference and require a
/// per-reader decision for every more restrictive reference.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::HttpBlob,
    open: Rule::Permit,
    community: Rule::RequireIdentityPresent,
    bound: Rule::Authorize,
    private: Rule::Authorize,
    unknown: Rule::Authorize,
    min_proof: Proof::HopAsserted,
    no_record: NoRecord::Permit,
    records_unavailable: RecordsUnavailable::ServeResidual,
    status: STATUS,
};

/// What the HTTP byte route must do for this caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlobServeVerdict {
    /// Serve because no row claims the bytes or some row already permits them.
    Serve,
    /// Refuse because every row is gated and no identity was presented.
    RequireIdentity {
        /// Least restrictive reach among the referencing rows.
        required_reach: String,
    },
    /// Refuse because every remaining candidate requires per-reader authorization.
    Authorize {
        /// Referencing rows whose standing could permit these bytes.
        candidates: Vec<BlobReference>,
    },
}

/// Whether a reach permits an anonymous HTTP blob read.
pub fn serves_anonymously(reach: &str) -> bool {
    matches!(RULES.rule_for(reach), Rule::Permit)
}

/// Whether a reach needs more than identity presence on the HTTP blob plane.
pub fn requires_authorization(reach: &str) -> bool {
    matches!(
        TierClass::classify(reach),
        TierClass::Bound | TierClass::Private | TierClass::Unknown
    )
}

/// Fold every referencing row using the any-row-permits address rule.
pub fn blob_serve_verdict(refs: &[BlobReference], identity_resolved: bool) -> BlobServeVerdict {
    if refs.is_empty() {
        return BlobServeVerdict::Serve;
    }
    if refs.iter().any(|record| serves_anonymously(&record.reach)) {
        return BlobServeVerdict::Serve;
    }
    if !identity_resolved {
        let required_reach = refs
            .iter()
            .min_by_key(|record| reach_level_index(&record.reach))
            .map(|record| record.reach.clone())
            .unwrap_or_else(|| "restricted".to_string());
        return BlobServeVerdict::RequireIdentity { required_reach };
    }
    if refs
        .iter()
        .any(|record| !requires_authorization(&record.reach))
    {
        return BlobServeVerdict::Serve;
    }
    BlobServeVerdict::Authorize {
        candidates: refs.to_vec(),
    }
}
