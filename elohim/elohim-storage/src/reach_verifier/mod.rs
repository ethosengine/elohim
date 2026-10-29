//! Shared reach decisions for every storage egress plane.
//!
//! The policy objects in this module are ephemeral class-C values reconstructed
//! from existing projections. They add no table, DHT entry, wire field, or
//! source of truth. A [`Permit`] owns the exact [`Subject`] it authorizes so a
//! serving function cannot exchange a permit for one subject for another.

use std::sync::{Arc, OnceLock};

use diesel::SqliteConnection;

use crate::db::AppContext;

mod permit;
mod plane;
mod refusal;
pub(crate) mod rules;
mod subject;
mod viewer;

pub use permit::Permit;
#[cfg(feature = "p2p")]
pub use plane::{blob_request, epr_atom_request, epr_request, shard_request, sync_request};
pub use plane::{view_kind, EgressPlane, NotEgress, RequestPlane};
pub use refusal::{Refusal, RefusalDetail};
pub use subject::Subject;
pub use viewer::{Proof, ResolvedViewer, Viewer};

use rules::{NoRecord, RecordsUnavailable, Rule};

/// One candidate record's result after its plane-specific standing check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CandidateVerdict {
    /// This record grants the exact subject.
    Permit,
    /// This record does not grant the subject.
    Deny,
    /// The caller identity is insufficient for this record's standing check.
    IdentityUnresolved,
    /// The records needed to decide could not be read.
    RecordsUnavailable,
    /// Custody standing withheld the private record for this reason.
    CustodyWithheld(crate::private_reach::WithholdReason),
}

/// The uniform result of a reach decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The exact subject may cross the judged plane.
    Permit(Permit),
    /// The subject must not cross the judged plane.
    Refuse(Refusal),
}

impl Verdict {
    /// Whether this verdict permits serving.
    pub fn is_permit(&self) -> bool {
        matches!(self, Self::Permit(_))
    }

    /// Consume the verdict and return its refusal, if any.
    pub fn refusal(self) -> Option<Refusal> {
        match self {
            Self::Permit(_) => None,
            Self::Refuse(refusal) => Some(refusal),
        }
    }
}

/// One request's viewer resolution and content authorizer.
pub struct ReachVerifier {
    viewer: Viewer,
    memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
    resolved: OnceLock<ResolvedViewer>,
    authorizer: OnceLock<crate::epr_service::EprService>,
}

impl std::fmt::Debug for ReachVerifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReachVerifier")
            .field("identity_present", &self.viewer.identity_present())
            .finish_non_exhaustive()
    }
}

impl ReachVerifier {
    /// Construct a verifier for a boundary-minted viewer.
    pub fn new(
        viewer: Viewer,
        memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
    ) -> Self {
        Self {
            viewer,
            memo_store,
            resolved: OnceLock::new(),
            authorizer: OnceLock::new(),
        }
    }

    /// Construct the HTTP verifier from the explicit agent-header extractor.
    pub fn from_http_extractor(
        agent_cid: Option<&str>,
        memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
    ) -> Self {
        Self::new(Viewer::from_http_extractor(agent_cid), memo_store)
    }

    /// Whether this request presented an identity at its boundary.
    pub fn identity_present(&self) -> bool {
        self.viewer.identity_present()
    }

    fn resolved<'a>(&'a self, conn: &mut SqliteConnection) -> &'a ResolvedViewer {
        self.resolved.get_or_init(|| self.viewer.resolve(conn))
    }

    fn authorizer(&self) -> &crate::epr_service::EprService {
        self.authorizer.get_or_init(|| {
            crate::epr_service::EprService::new(
                None,
                None,
                None,
                crate::p2p::trust_cache::PeerTrustCache::new(),
            )
            .with_memo_store(self.memo_store.clone())
        })
    }

    /// Judge a projected content row on one egress plane.
    pub fn judge_row(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        plane: EgressPlane,
        content_id: &str,
        reach: &str,
    ) -> Verdict {
        let subject = Subject::row(content_id, reach);
        self.judge_declared_reach(conn, ctx, plane, &subject, reach)
    }

    fn judge_declared_reach(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        plane: EgressPlane,
        subject: &Subject,
        reach: &str,
    ) -> Verdict {
        let rule = rules::for_plane(plane).rule_for(reach);
        let resolved = self.resolved(conn);
        judge_rule(
            conn,
            ctx,
            subject,
            resolved,
            self.authorizer(),
            rule,
            rules::for_plane(plane).min_proof,
        )
    }

    /// Judge content-addressed bytes by folding every row that references them.
    pub fn judge_address_records(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        plane: EgressPlane,
        hash: &str,
        records: Result<&[crate::blob_reach::BlobReference], ()>,
    ) -> Verdict {
        let subject = Subject::Address {
            hash: hash.to_string(),
        };
        let resolved = self.resolved(conn);
        judge_record_fold(
            plane,
            &subject,
            records,
            resolved.agent().is_some(),
            |record, rule| match rule {
                // The HTTP byte route preserves its fail-closed residual: it
                // resolves no per-reader standing above community.
                Rule::Authorize if plane == EgressPlane::HttpBlob => {
                    CandidateVerdict::IdentityUnresolved
                }
                Rule::Authorize => {
                    if resolved.proof() < rules::for_plane(plane).min_proof {
                        return CandidateVerdict::IdentityUnresolved;
                    }
                    let Some(human) = resolved.human() else {
                        return CandidateVerdict::IdentityUnresolved;
                    };
                    match self.authorizer().authorize_reach_for_human_with_own_trust(
                        conn,
                        ctx,
                        &record.reach,
                        human,
                        &record.content_id,
                    ) {
                        Ok(()) => CandidateVerdict::Permit,
                        Err(_) => CandidateVerdict::Deny,
                    }
                }
                _ => CandidateVerdict::Deny,
            },
        )
    }

    /// Judge a loaded content row after asynchronously pre-fetching custody facts.
    pub async fn judge_pooled(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        plane: EgressPlane,
        row: &crate::db::models::Content,
        custody: &dyn crate::services::custody_standing::CustodyStanding,
    ) -> Verdict {
        let subject = Subject::row(&row.id, &row.reach);
        if rules::for_plane(plane).rule_for(&row.reach) != Rule::Custody {
            return self.judge_declared_reach(conn, ctx, plane, &subject, &row.reach);
        }
        let Some(requester) = self.viewer.custody_requester() else {
            return Verdict::Refuse(Refusal::identity_absent(&row.reach));
        };
        let resolved = self.resolved(conn);
        if resolved.proof() < rules::for_plane(plane).min_proof {
            return Verdict::Refuse(Refusal::identity_unresolved(&row.reach));
        }
        let facts = crate::services::custody_standing::CustodyStanding::facts_for(
            custody,
            &requester,
            &crate::services::custody_standing::RowFacts::from_content(row),
        )
        .await;
        match crate::private_reach::private_serve_verdict(&facts) {
            crate::private_reach::PrivateServeVerdict::Serve(_) => {
                Verdict::Permit(Permit::new(subject))
            }
            crate::private_reach::PrivateServeVerdict::Withhold(
                crate::private_reach::WithholdReason::AuthorityUnavailable,
            ) => Verdict::Refuse(Refusal::records_unavailable()),
            crate::private_reach::PrivateServeVerdict::Withhold(reason) => {
                Verdict::Refuse(Refusal::custody_withheld(&row.reach, reason))
            }
        }
    }

    /// Judge a publication row without pretending a broadcast is anonymous.
    pub fn judge_broadcast(plane: EgressPlane, subject: &Subject) -> Verdict {
        let Some(reach) = subject.reach() else {
            return Verdict::Refuse(Refusal::no_record());
        };
        match rules::for_plane(plane).rule_for(reach) {
            Rule::Permit | Rule::Broadcast => Verdict::Permit(Permit::new(subject.clone())),
            _ => Verdict::Refuse(Refusal::denied(reach, "not admitted on broadcast plane")),
        }
    }
}

/// Judge an HTTP list row whose extractor already resolved the human once.
pub(crate) fn judge_resolved_http_row(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    requester: Option<&crate::db::models::Human>,
    authorizer: &crate::epr_service::EprService,
    subject: &Subject,
) -> Verdict {
    let Some(reach) = subject.reach() else {
        return Verdict::Refuse(Refusal::no_record());
    };
    let resolved = ResolvedViewer::from_http_human(requester);
    let row = rules::for_plane(EgressPlane::HttpList);
    judge_rule(
        conn,
        ctx,
        subject,
        &resolved,
        authorizer,
        row.rule_for(reach),
        row.min_proof,
    )
}

fn judge_rule(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    subject: &Subject,
    viewer: &ResolvedViewer,
    authorizer: &crate::epr_service::EprService,
    rule: Rule,
    min_proof: Proof,
) -> Verdict {
    let reach = subject.reach().unwrap_or("");
    let permit = || Verdict::Permit(Permit::new(subject.clone()));
    if viewer.proof() == Proof::Local {
        return permit();
    }
    match rule {
        Rule::Permit | Rule::Broadcast => permit(),
        Rule::RequireIdentityPresent => {
            if viewer.agent().is_some() {
                permit()
            } else {
                Verdict::Refuse(Refusal::identity_absent(reach))
            }
        }
        Rule::RequireResolvedHuman => {
            if viewer.human().is_some() {
                permit()
            } else if viewer.agent().is_some() {
                Verdict::Refuse(Refusal::identity_unresolved(reach))
            } else {
                Verdict::Refuse(Refusal::identity_absent(reach))
            }
        }
        Rule::Authorize => {
            if viewer.agent().is_none() {
                return Verdict::Refuse(Refusal::identity_absent(reach));
            }
            if viewer.proof() < min_proof {
                return Verdict::Refuse(Refusal::identity_unresolved(reach));
            }
            let Some(human) = viewer.human() else {
                return Verdict::Refuse(Refusal::identity_unresolved(reach));
            };
            match authorizer.authorize_reach_for_human_with_own_trust(
                conn,
                ctx,
                reach,
                human,
                subject.id(),
            ) {
                Ok(()) => permit(),
                Err(reason) => Verdict::Refuse(Refusal::denied(reach, reason)),
            }
        }
        Rule::AuthorOnly => {
            if viewer
                .agent()
                .is_some_and(|agent| subject.signer() == Some(agent))
            {
                permit()
            } else {
                Verdict::Refuse(Refusal::denied(reach, "requester is not the atom author"))
            }
        }
        Rule::Custody => Verdict::Refuse(Refusal::records_unavailable()),
        Rule::Refuse => {
            if viewer.agent().is_some() {
                Verdict::Refuse(Refusal::identity_unresolved(reach))
            } else {
                Verdict::Refuse(Refusal::identity_absent(reach))
            }
        }
    }
}

fn no_record_verdict(plane: EgressPlane, subject: &Subject) -> Verdict {
    match rules::for_plane(plane).no_record {
        NoRecord::Permit => Verdict::Permit(Permit::new(subject.clone())),
        NoRecord::Refuse => Verdict::Refuse(Refusal::no_record()),
    }
}

/// Fold every record naming one address or root; any granting record serves it.
pub(crate) fn judge_record_fold<F>(
    plane: EgressPlane,
    subject: &Subject,
    records: Result<&[crate::blob_reach::BlobReference], ()>,
    identity_present: bool,
    mut decide: F,
) -> Verdict
where
    F: FnMut(&crate::blob_reach::BlobReference, Rule) -> CandidateVerdict,
{
    let rule_set = rules::for_plane(plane);
    let records = match records {
        Ok(records) => records,
        Err(()) => {
            return match rule_set.records_unavailable {
                RecordsUnavailable::ServeResidual => Verdict::Permit(Permit::new(subject.clone())),
                RecordsUnavailable::Refuse => Verdict::Refuse(Refusal::records_unavailable()),
            };
        }
    };
    if records.is_empty() {
        return no_record_verdict(plane, subject);
    }
    if records.iter().any(|record| {
        matches!(
            rule_set.rule_for(&record.reach),
            Rule::Permit | Rule::Broadcast
        )
    }) {
        return Verdict::Permit(Permit::new(subject.clone()));
    }
    let least_reach = || {
        records
            .iter()
            .min_by_key(|record| crate::epr_service::reach_level_index(&record.reach))
            .map(|record| record.reach.clone())
            .unwrap_or_else(|| "restricted".to_string())
    };
    if !identity_present {
        return Verdict::Refuse(Refusal::identity_absent(least_reach()));
    }
    if records.iter().any(|record| {
        matches!(
            rule_set.rule_for(&record.reach),
            Rule::RequireIdentityPresent
        )
    }) {
        return Verdict::Permit(Permit::new(subject.clone()));
    }

    let mut unresolved = false;
    let mut unavailable = false;
    let mut custody_withheld = None;
    for record in records {
        let rule = rule_set.rule_for(&record.reach);
        match decide(record, rule) {
            CandidateVerdict::Permit => {
                return Verdict::Permit(Permit::new(subject.clone()));
            }
            CandidateVerdict::Deny => {}
            CandidateVerdict::IdentityUnresolved => unresolved = true,
            CandidateVerdict::RecordsUnavailable => unavailable = true,
            CandidateVerdict::CustodyWithheld(reason) => custody_withheld = Some(reason),
        }
    }
    if unavailable {
        return Verdict::Refuse(Refusal::records_unavailable());
    }
    let refusal_reach = || {
        if plane == EgressPlane::HttpBlob {
            records
                .first()
                .map(|record| record.reach.clone())
                .unwrap_or_else(|| "restricted".to_string())
        } else {
            least_reach()
        }
    };
    if unresolved {
        return Verdict::Refuse(Refusal::identity_unresolved(refusal_reach()));
    }
    if let Some(reason) = custody_withheld {
        return Verdict::Refuse(Refusal::custody_withheld(refusal_reach(), reason));
    }
    Verdict::Refuse(Refusal::denied(refusal_reach(), "outside standing"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcast_rows_keep_community() {
        let subject = Subject::row("garden", "community");
        for plane in [EgressPlane::ProjectionSync, EgressPlane::P2pSync] {
            assert!(ReachVerifier::judge_broadcast(plane, &subject).is_permit());
        }
    }

    #[test]
    fn self_and_private_are_distinct_rule_classes() {
        assert_eq!(rules::TierClass::classify("self"), rules::TierClass::Bound);
        assert_eq!(
            rules::TierClass::classify("private"),
            rules::TierClass::Private
        );
    }

    #[test]
    fn address_and_root_apply_their_distinct_no_record_rows() {
        let address = Subject::Address {
            hash: "sha256-address".to_string(),
        };
        assert!(no_record_verdict(EgressPlane::HttpBlob, &address).is_permit());

        let root = Subject::Root {
            cid: "bafy-root".to_string(),
        };
        assert!(matches!(
            no_record_verdict(EgressPlane::EprfsTier, &root),
            Verdict::Refuse(Refusal {
                detail: RefusalDetail::NoRecord,
                ..
            })
        ));
    }

    #[test]
    fn identified_http_blob_refusal_reports_the_first_reference() {
        let subject = Subject::Address {
            hash: "sha256-address".to_string(),
        };
        let records = [
            crate::blob_reach::BlobReference::new("private-row", "private"),
            crate::blob_reach::BlobReference::new("familiar-row", "familiar"),
        ];
        let verdict = judge_record_fold(
            EgressPlane::HttpBlob,
            &subject,
            Ok(&records),
            true,
            |_, _| CandidateVerdict::IdentityUnresolved,
        );
        assert_eq!(
            verdict.refusal().and_then(|refusal| refusal.required_reach),
            Some("private".to_string())
        );
    }

    #[test]
    fn community_atoms_keep_author_only_behavior() {
        assert_eq!(
            rules::for_plane(EgressPlane::P2pEprAtom).rule_for("community"),
            Rule::AuthorOnly
        );
    }
}
