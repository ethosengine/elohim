//! `RootReachGate`: the serving side of reach for eprfs trees (interfaces spec §2.A.3, §2.A.6).
//!
//! `blob_reach::blob_serve_verdict` answers "may this caller read these bytes?" from the content
//! rows that reference a blob. This gate asks the same question of **root records**: the records
//! this peer holds that declare a CID as the root of a tree (a release record, a head-plane slug
//! → {root, reach}, a content row whose bytes are a tree) or, for an object outside every root,
//! that reference the object itself. Tree nodes inherit their root's reach (spec §12.3 rule 2),
//! so a `TierEndpoint` asks [`ReachGate::admit_root`] once per batch and checks membership itself.
//!
//! The rule is `blob_serve_verdict`'s, unchanged — a CID is servable to a caller iff SOME record
//! naming it is servable to that caller — with three differences, each stated:
//!
//! 1. **An unknown record refuses (`NoRecord`).** The `/blob` route serves an unreferenced blob
//!    (honest absence, because shard bytes and pre-row uploads have no audience); a tree root with
//!    no record has an audience this peer cannot see, and serving it is exactly the A1 defect. The
//!    gate never fetches a record to decide (spec §2.A.3: the gate is synchronous over held
//!    records).
//! 2. **`private` records are judged by custody** (`private_reach::private_serve_verdict`: the
//!    ward, a spool custodian of the ward, or a custodian of this digest), in place of the generic
//!    authorizer, which the `/db/content` route uses for the other tiers above `community`.
//! 3. **Every non-admission is a typed refusal** (`ReachRefusal`), and a failure to read authority
//!    is `AuthorizerUnavailable`, never laundered into `OutsideStanding` (C4). These are local
//!    diagnostic reasons: a serving endpoint must obscure object-specific refusal details before
//!    returning them to a requester outside reach (spec §12.3 rule 2).
//!
//! No reach vocabulary is canonized here: the strings are the records' own, judged through
//! `blob_reach`'s helpers; [`ReachGate::root_level`] maps them through the protocol schema's
//! `elohim_epr::Reach` and leaves any other word unknown, which an `AskPolicy` reads narrowly.

#[cfg(test)]
use eprfs_core::ReachReason;
use eprfs_core::{BlobCid, ReachGate, ReachLevel, ReachRefusal, Requester};

use crate::blob_reach::BlobReference;
use crate::private_reach::{is_private, private_serve_verdict, PrivateServeFacts, WithholdReason};
use crate::reach_verifier::{CandidateVerdict, EgressPlane, Subject, Verdict};

/// The held records could not be read (a pool checkout, a query or the conductor failed). Never
/// read as "no record": the gate answers `AuthorizerUnavailable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordsUnavailable;

/// The records a serving peer already holds, as the gate reads them. Implemented by the host over
/// its projections (content rows, release records, the head plane, `rea_commitments`). The
/// compatibility adapter consumes custody facts pre-resolved by its host; pooled transport paths
/// use `ReachVerifier::judge_pooled`, whose custody pre-fetch is asynchronous.
pub trait ReachRecords: Send + Sync {
    /// Every held record that declares `root` as the root of a tree, with its reach.
    fn root_records(&self, root: &BlobCid) -> Result<Vec<BlobReference>, RecordsUnavailable>;
    /// Every held record that references `cid` as an object of its own (outside every root).
    fn object_records(&self, cid: &BlobCid) -> Result<Vec<BlobReference>, RecordsUnavailable>;
    /// The content route's Layer 1.5 authorizer for a record above `community` that is not
    /// `private`: does it grant `who` against `record`?
    fn authorize(
        &self,
        who: &Requester,
        record: &BlobReference,
    ) -> Result<bool, RecordsUnavailable>;
    /// The custody facts for a `private` record and this requester (the ward, spool custody of
    /// the ward, blob custody of `subject`), resolved from held commitments. `authority_available`
    /// false when they could not be read.
    fn private_facts(
        &self,
        who: &Requester,
        subject: &BlobCid,
        record: &BlobReference,
    ) -> PrivateServeFacts;
}

/// `ReachGate` over a peer's held records.
pub struct RootReachGate<R> {
    records: R,
}

impl<R: ReachRecords> RootReachGate<R> {
    /// Construct a root gate over held reach records.
    pub fn new(records: R) -> Self {
        Self { records }
    }

    /// Return the held-record adapter used by this gate.
    pub fn records(&self) -> &R {
        &self.records
    }

    fn judge(
        &self,
        who: &Requester,
        subject: Subject,
        cid: &BlobCid,
        records: Result<Vec<BlobReference>, RecordsUnavailable>,
    ) -> Result<(), ReachRefusal> {
        let refs = records.as_deref().map_err(|_| ());
        match crate::reach_verifier::judge_record_fold(
            EgressPlane::EprfsTier,
            &subject,
            refs,
            who.agent().is_some(),
            |record, rule| match rule {
                crate::reach_verifier::rules::Rule::Authorize => {
                    match self.records.authorize(who, record) {
                        Ok(true) => CandidateVerdict::Permit,
                        Ok(false) => CandidateVerdict::Deny,
                        Err(RecordsUnavailable) => CandidateVerdict::RecordsUnavailable,
                    }
                }
                crate::reach_verifier::rules::Rule::Custody if is_private(&record.reach) => {
                    match private_serve_verdict(&self.records.private_facts(who, cid, record)) {
                        crate::private_reach::PrivateServeVerdict::Serve(_) => {
                            CandidateVerdict::Permit
                        }
                        crate::private_reach::PrivateServeVerdict::Withhold(
                            WithholdReason::AuthorityUnavailable,
                        ) => CandidateVerdict::RecordsUnavailable,
                        crate::private_reach::PrivateServeVerdict::Withhold(reason) => {
                            CandidateVerdict::CustodyWithheld(reason)
                        }
                    }
                }
                _ => CandidateVerdict::Deny,
            },
        ) {
            Verdict::Permit(_) => Ok(()),
            Verdict::Refuse(refusal) => Err(refusal.into_eprfs()),
        }
    }
}

/// A record's reach as `Reach::openness()`, when the word is one of the protocol schema's eight.
fn openness(reach: &str) -> Option<ReachLevel> {
    serde_json::from_value::<elohim_epr::Reach>(serde_json::Value::String(reach.to_string()))
        .ok()
        .map(|r| ReachLevel(r.openness()))
}

impl<R: ReachRecords> ReachGate for RootReachGate<R> {
    fn admit_root(&self, who: &Requester, root: &BlobCid) -> Result<(), ReachRefusal> {
        self.judge(
            who,
            Subject::Root {
                cid: root.to_string(),
            },
            root,
            self.records.root_records(root),
        )
    }

    fn admit_object(&self, who: &Requester, cid: &BlobCid) -> Result<(), ReachRefusal> {
        self.judge(
            who,
            Subject::Address {
                hash: cid.to_string(),
            },
            cid,
            self.records.object_records(cid),
        )
    }

    /// The widest reach among the root's records (any one record makes the root that open, the
    /// same "some" as the verdict). Words outside the protocol schema are skipped, so they can
    /// only narrow the answer; no record, unreadable records, or no known word is `None`.
    fn root_level(&self, root: &BlobCid) -> Option<ReachLevel> {
        self.records
            .root_records(root)
            .ok()?
            .iter()
            .filter_map(|r| openness(&r.reach))
            .max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eprfs_core::KeyCustody;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Held {
        roots: HashMap<BlobCid, Vec<BlobReference>>,
        objects: HashMap<BlobCid, Vec<BlobReference>>,
        /// (agent, record id) pairs the authorizer grants.
        grants: Vec<(String, String)>,
        /// Agents that are the ward of every private record.
        wards: Vec<String>,
        records_down: bool,
        authorizer_down: bool,
        custody_down: bool,
    }

    impl ReachRecords for Held {
        fn root_records(&self, root: &BlobCid) -> Result<Vec<BlobReference>, RecordsUnavailable> {
            if self.records_down {
                return Err(RecordsUnavailable);
            }
            Ok(self.roots.get(root).cloned().unwrap_or_default())
        }
        fn object_records(&self, cid: &BlobCid) -> Result<Vec<BlobReference>, RecordsUnavailable> {
            if self.records_down {
                return Err(RecordsUnavailable);
            }
            Ok(self.objects.get(cid).cloned().unwrap_or_default())
        }
        fn authorize(
            &self,
            who: &Requester,
            record: &BlobReference,
        ) -> Result<bool, RecordsUnavailable> {
            if self.authorizer_down {
                return Err(RecordsUnavailable);
            }
            Ok(who.agent().is_some_and(|a| {
                self.grants
                    .iter()
                    .any(|(g, r)| g == a && *r == record.content_id)
            }))
        }
        fn private_facts(
            &self,
            who: &Requester,
            _subject: &BlobCid,
            record: &BlobReference,
        ) -> PrivateServeFacts {
            let resolved = who.agent().is_some();
            PrivateServeFacts {
                reach: record.reach.clone(),
                requester_resolved: resolved,
                requester_is_ward: who
                    .agent()
                    .is_some_and(|a| self.wards.iter().any(|w| w == a)),
                custody_for_ward: false,
                custody_for_digest: false,
                ward_resolved: true,
                authority_available: !self.custody_down,
            }
        }
    }

    fn root(name: &str) -> BlobCid {
        BlobCid::compute(name.as_bytes())
    }

    fn rec(id: &str, reach: &str) -> BlobReference {
        BlobReference::new(id, reach)
    }

    fn agent(a: &str) -> Requester {
        Requester::authenticated(a, KeyCustody::DeviceHeld)
    }

    fn reason(r: Result<(), ReachRefusal>) -> ReachReason {
        r.expect_err("expected a refusal").reason
    }

    fn gate_with(roots: &[(&BlobCid, Vec<BlobReference>)]) -> RootReachGate<Held> {
        let mut held = Held::default();
        for (c, refs) in roots {
            held.roots.insert((*c).clone(), refs.clone());
        }
        RootReachGate::new(held)
    }

    #[test]
    fn a_public_root_serves_anyone() {
        let r = root("public-app");
        let gate = gate_with(&[(&r, vec![rec("release", "public")])]);
        assert!(gate.admit_root(&Requester::anonymous(), &r).is_ok());
        assert_eq!(
            gate.root_level(&r),
            Some(ReachLevel(elohim_epr::Reach::Public.openness()))
        );
    }

    /// Unknown root authority refuses locally with `NoRecord` (§2.A.3); the serving endpoint
    /// owns the outside-requester privacy boundary and must not expose this distinction.
    #[test]
    fn an_unknown_root_refuses_rather_than_serving() {
        let gate = gate_with(&[]);
        assert_eq!(
            reason(gate.admit_root(&Requester::anonymous(), &root("never-heard-of"))),
            ReachReason::NoRecord
        );
        assert_eq!(gate.root_level(&root("never-heard-of")), None);
    }

    #[test]
    fn a_gated_root_asks_an_anonymous_caller_for_identity() {
        let r = root("garden-club");
        let gate = gate_with(&[(&r, vec![rec("club", "community")])]);
        let refused = gate.admit_root(&Requester::anonymous(), &r).unwrap_err();
        assert_eq!(refused.reason, ReachReason::IdentityRequired);
        assert_eq!(refused.needs_standing.as_deref(), Some("community"));
        assert!(gate.admit_root(&agent("uhCAk-member"), &r).is_ok());
    }

    #[test]
    fn above_community_runs_the_authorizer_and_serves_iff_any_record_grants() {
        let r = root("love-map");
        let mut gate = gate_with(&[(&r, vec![rec("map", "intimate"), rec("map-2", "trusted")])]);
        gate.records
            .grants
            .push(("uhCAk-partner".into(), "map-2".into()));
        assert!(gate.admit_root(&agent("uhCAk-partner"), &r).is_ok());
        let refused = gate.admit_root(&agent("uhCAk-stranger"), &r).unwrap_err();
        assert_eq!(refused.reason, ReachReason::OutsideStanding);
        assert_eq!(
            refused.needs_standing.as_deref(),
            Some("trusted"),
            "the least restricted"
        );
    }

    #[test]
    fn private_records_are_judged_by_custody_not_the_authorizer() {
        let r = root("death-witness");
        let mut gate = gate_with(&[(&r, vec![rec("witness", "private")])]);
        // The generic authorizer would grant; custody does not, so the private record refuses.
        gate.records
            .grants
            .push(("uhCAk-stranger".into(), "witness".into()));
        gate.records.wards.push("uhCAk-ward".into());
        assert_eq!(
            reason(gate.admit_root(&agent("uhCAk-stranger"), &r)),
            ReachReason::OutsideStanding
        );
        assert!(gate.admit_root(&agent("uhCAk-ward"), &r).is_ok());
        assert_eq!(
            reason(gate.admit_root(&Requester::anonymous(), &r)),
            ReachReason::IdentityRequired
        );
    }

    #[test]
    fn unreadable_authority_is_never_laundered_into_absent_standing() {
        let r = root("intimate-tree");
        let mut gate = gate_with(&[(&r, vec![rec("t", "intimate")])]);
        gate.records.authorizer_down = true;
        assert_eq!(
            reason(gate.admit_root(&agent("uhCAk-x"), &r)),
            ReachReason::AuthorizerUnavailable
        );

        let p = root("private-tree");
        let mut gate = gate_with(&[(&p, vec![rec("p", "private")])]);
        gate.records.custody_down = true;
        assert_eq!(
            reason(gate.admit_root(&agent("uhCAk-x"), &p)),
            ReachReason::AuthorizerUnavailable
        );

        let mut gate = gate_with(&[]);
        gate.records.records_down = true;
        assert_eq!(
            reason(gate.admit_root(&Requester::anonymous(), &r)),
            ReachReason::AuthorizerUnavailable
        );
        assert_eq!(gate.root_level(&r), None);
    }

    /// Content addressing deduplicates: a root named by a public record and a private one is
    /// public through the public record, and its level is the widest.
    #[test]
    fn a_root_any_public_record_names_is_public() {
        let r = root("shared");
        let gate = gate_with(&[(&r, vec![rec("secret", "intimate"), rec("open", "commons")])]);
        assert!(gate.admit_root(&Requester::anonymous(), &r).is_ok());
        assert_eq!(
            gate.root_level(&r),
            Some(ReachLevel(elohim_epr::Reach::Commons.openness()))
        );
    }

    #[test]
    fn an_unrecognized_reach_fails_closed_and_is_unknown_to_ask_policy() {
        let r = root("new-tier");
        let gate = gate_with(&[(&r, vec![rec("x", "brand-new-tier")])]);
        assert_eq!(
            reason(gate.admit_root(&Requester::anonymous(), &r)),
            ReachReason::IdentityRequired
        );
        assert_eq!(gate.root_level(&r), None, "an unknown word is not a level");
        let mixed = root("mixed");
        let gate = gate_with(&[(
            &mixed,
            vec![rec("x", "brand-new-tier"), rec("y", "intimate")],
        )]);
        assert_eq!(
            gate.root_level(&mixed),
            Some(ReachLevel(elohim_epr::Reach::Intimate.openness())),
            "an unknown word only narrows"
        );
    }

    #[test]
    fn an_object_outside_every_root_is_judged_by_its_own_records() {
        let leaf = BlobCid::compute_raw(b"a leaf");
        let mut held = Held::default();
        held.objects
            .insert(leaf.clone(), vec![rec("row", "public")]);
        let gate = RootReachGate::new(held);
        assert!(gate.admit_object(&Requester::anonymous(), &leaf).is_ok());
        assert_eq!(
            reason(gate.admit_object(&Requester::anonymous(), &BlobCid::compute_raw(b"other"))),
            ReachReason::NoRecord
        );
        // Its object records say nothing about it as a root.
        assert_eq!(
            reason(gate.admit_root(&Requester::anonymous(), &leaf)),
            ReachReason::NoRecord
        );
    }

    /// The gate plugs into eprfs-serve's `TierEndpoint`, which asks it once per batch.
    #[test]
    fn the_gate_is_an_eprfs_reach_gate() {
        fn takes(_: &dyn ReachGate) {}
        takes(&gate_with(&[]));
    }
}
