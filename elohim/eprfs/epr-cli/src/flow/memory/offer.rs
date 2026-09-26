//! The standing of a GOVERNED OFFER — a gift the collective makes to a consumer, active only while
//! a Steward who did not author it has approved it.
//!
//! An offer is a VF `Intent` in the flow sidecar, addressed by its atom CID. This module does not
//! mint it (the offer's own tooling does); it reads the verdict notes written against that CID
//! (`epr flow note --on <offer-cid> --kind verdict --verdict approved|changes-requested`) and resolves
//! them through the SAME distinct-Steward rule graduation uses ([`super::steward_approval`]):
//!
//! - an approving verdict counts only from an active Steward who is not the offer's author
//!   (anti-self-election, C1) — every other approving verdict is listed as ignored, never silent;
//! - a later `changes-requested` verdict from any active Steward withdraws it — the collective
//!   withdraws by the same process it approves;
//! - an offer that crosses the network (a consumer the collective does not own, or any settlement)
//!   additionally needs two distinct NON-fixture Stewards ([`requires_non_fixture_stewards`]); a
//!   fixture co-steward's approval reads `bootstrap (fixture co-steward)` wherever it is shown.
//!
//! An offer creates no claim and implies no settlement; its standing is a read, never a grant.
//!
//! The offer's DECLARATION (terms, the two-sided disclosure) and the external-claim envelope it
//! governs are shared, pure shapes in [`elohim_epr_rea::external`]; this module adds only what
//! needs the repository: reading an offer document off disk ([`load_offer`]), addressing it by its
//! body ([`offer_intent`], [`offer_cid`]) and deriving its standing from the verdicts on record.

use std::path::Path;

use cid::Cid;
use elohim_epr_rea::{atom_cid, FlowRecord, Intent, OfferDeclaration, OfferDocument};
use eprfs_agent::memory::{requires_non_fixture_stewards, Affiliation, AffiliationStanding};
use serde::Serialize;

use super::validation::{validated_at, Reader};
use super::{refused, steward_approval};
use crate::flow::{FlowError, FlowResult};

/// Where an offer stands, derived from the verdicts on record — never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OfferState {
    /// No distinct Steward has approved it.
    Proposed,
    /// A distinct Steward's approval stands, uncontradicted.
    Active,
    /// A Steward's later contrary verdict withdrew it.
    Withdrawn,
}

/// The approval an active offer rests on.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferApproval {
    /// The verdict note's record CID.
    pub verdict: String,
    pub approver: String,
    /// The approver's Steward affiliation line CID.
    pub affiliation: String,
    pub standing: AffiliationStanding,
    /// `local (steward on record)` or `bootstrap (fixture co-steward)`.
    pub validated_at: &'static str,
}

/// The derived standing of one offer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferStanding {
    pub offer: String,
    pub collective: String,
    pub state: OfferState,
    /// Present while Active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<OfferApproval>,
    /// The withdrawing verdict's record CID, while Withdrawn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdrawn_by: Option<String>,
    /// Verdicts on this offer that count for nothing, each with the reason — visible, not silent.
    pub ignored: Vec<String>,
    /// What an operator must do to activate it, while it is not Active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<String>,
    /// The collective has no active Steward: the offer can never be active until it is
    /// re-founded. Serialized only when true.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub stewardless: bool,
}

impl OfferStanding {
    pub fn is_active(&self) -> bool {
        self.state == OfferState::Active
    }

    /// The one-line stakes reading every surface prints beside the offer.
    pub fn stakes_line(&self) -> String {
        match (&self.state, &self.approval) {
            (OfferState::Active, Some(a)) => format!(
                "offer {} active — approved by {} · validated-at: {}",
                self.offer, a.approver, a.validated_at
            ),
            (OfferState::Withdrawn, _) => format!(
                "offer {} withdrawn by verdict {}",
                self.offer,
                self.withdrawn_by.as_deref().unwrap_or("?")
            ),
            _ if self.stewardless => format!(
                "offer {} proposed — collective {} is stewardless; no Steward can approve it",
                self.offer, self.collective
            ),
            _ => format!(
                "offer {} proposed — no distinct Steward's approval",
                self.offer
            ),
        }
    }
}

fn slot(classified_as: &[String], value: &str) -> bool {
    classified_as.iter().any(|v| v == value)
}

/// Parse an offer document: its frontmatter decoded as an [`OfferDeclaration`] (serde-strict, so
/// an unknown term refuses) and validated — a claim, a settlement or a payee never parses.
pub fn parse_offer(path: &str, text: &str) -> FlowResult<OfferDocument> {
    let front = OfferDocument::frontmatter(path, text)?;
    let declaration: OfferDeclaration = serde_yaml::from_str(front).map_err(|e| {
        FlowError::InvalidArguments(format!("{path}: offer declaration is malformed: {e}"))
    })?;
    Ok(OfferDocument::new(path, text, declaration)?)
}

/// Read and parse the offer document at `path` (repository-relative) under `root`.
pub fn load_offer(root: &Path, path: &str) -> FlowResult<OfferDocument> {
    let file = root.join(path);
    let text = std::fs::read_to_string(&file).map_err(|source| FlowError::Read {
        path: file.clone(),
        source,
    })?;
    parse_offer(path, &text)
}

/// The Intent an offer document mints, scoped to its own body CID: an approval is an approval of
/// the exact text a Steward read, and an edit re-addresses the offer.
pub fn offer_intent(doc: &OfferDocument) -> Intent {
    doc.declaration().intent(crate::flow::body_cid(doc.text()))
}

/// The offer's address — its Intent's atom CID, the value every claim it admits is scoped to.
pub fn offer_cid(doc: &OfferDocument) -> FlowResult<Cid> {
    Ok(atom_cid(&offer_intent(doc))?)
}

/// Read the standing of `offer` (its declaration) minted at Intent CID `cid`, under the
/// collective its declaration names (a repository-relative `.epr-meta/collective.json`).
///
/// The approving Steward must not be the offer's `author`. An offer that `crossesNetwork` (a
/// consumer the collective does not own) needs two distinct non-fixture Stewards, and a fixture
/// approval can never activate it. An offer whose `provider` is not that collective is refused.
pub fn offer_standing(
    root: &Path,
    offer_declaration: &OfferDeclaration,
    cid: &Cid,
) -> FlowResult<OfferStanding> {
    let offer = cid;
    let author = offer_declaration.author.as_str();
    let crosses_network = offer_declaration.crosses_network;
    let mut reader = Reader::new(root)?;
    let governance = reader.governance(&offer_declaration.collective)?;
    if offer_declaration.provider != governance.declaration.id {
        return Err(refused(format!(
            "the offer's provider {} is not the governing collective {}",
            offer_declaration.provider, governance.declaration.id
        )));
    }
    let records = reader.records()?.to_vec();
    if !records
        .iter()
        .any(|(cid, record)| cid == offer && matches!(record, FlowRecord::Intent(_)))
    {
        return Err(refused(format!(
            "offer {offer} is not an Intent in this sidecar — mint the offer before reading its \
             standing"
        )));
    }

    let mut state = OfferState::Proposed;
    let mut approval: Option<OfferApproval> = None;
    let mut approvals: Vec<Affiliation> = Vec::new();
    let mut withdrawn_by = None;
    let mut ignored = Vec::new();
    for (cid, record) in &records {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if &event.resource != offer
            || event.classified_as.first().map(String::as_str) != Some("run:verdict")
        {
            continue;
        }
        let provider = event.provider.0.as_str();
        if slot(&event.classified_as, "verdict:approved") {
            match steward_approval(&governance, provider, author) {
                Ok((affiliation_cid, affiliation)) => {
                    approvals.push(affiliation.clone());
                    approval = Some(OfferApproval {
                        verdict: cid.to_string(),
                        approver: affiliation.member.clone(),
                        affiliation: affiliation_cid.to_string(),
                        standing: affiliation.standing,
                        validated_at: validated_at(affiliation.standing),
                    });
                    state = OfferState::Active;
                    withdrawn_by = None;
                }
                Err(why) => ignored.push(format!("{cid}: {why}")),
            }
        } else if slot(&event.classified_as, "verdict:changes-requested") {
            match governance.affiliation_of(provider) {
                Some((_, a)) if a.is_active_steward() => {
                    state = OfferState::Withdrawn;
                    approval = None;
                    approvals.clear();
                    withdrawn_by = Some(cid.to_string());
                }
                _ => ignored.push(format!(
                    "{cid}: contrary verdict by {provider}, who holds no Steward affiliation in {}",
                    governance.declaration.id
                )),
            }
        }
    }

    if crosses_network && state == OfferState::Active {
        let held: Vec<&Affiliation> = approvals.iter().collect();
        if let Err(why) = requires_non_fixture_stewards(&held, 2) {
            ignored.push(format!("network-crossing offer: {why}; it stays proposed"));
            state = OfferState::Proposed;
            approval = None;
        }
    }

    // A stewardless collective's offer is never active: every approval above was refused by
    // `steward_approval`, naming the state, and a withdrawal needs no Steward to stand.
    let stewardless = governance.is_stewardless();
    let missing = match state {
        OfferState::Active => None,
        _ if stewardless => governance
            .require_stewarded("activating this offer")
            .err()
            .map(|e| e.to_string()),
        OfferState::Proposed => Some(format!(
            "an approving verdict from a Steward of {} who is not the offer's author ({author}){}: \
             epr flow note --on {offer} --kind verdict --verdict approved --reason \"<what you \
             approve>\" --session <your registered session>",
            governance.declaration.id,
            if crosses_network {
                " — two distinct non-fixture Stewards, because this offer crosses the network"
            } else {
                ""
            }
        )),
        OfferState::Withdrawn => Some(format!(
            "a renewed approving verdict from a distinct Steward of {} after the withdrawal",
            governance.declaration.id
        )),
    };

    Ok(OfferStanding {
        offer: offer.to_string(),
        collective: governance.declaration.id.clone(),
        state,
        approval,
        withdrawn_by,
        ignored,
        missing,
        stewardless,
    })
}
