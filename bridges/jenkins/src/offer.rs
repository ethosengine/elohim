//! The governed offer: the collective's gift to a consumer, authored as a document and minted as a
//! VF `Intent` whose CID every observation is scoped to.
//!
//! An offer is a TWO-SIDED DISCLOSURE. Beside what the network gives (`offered`), it records what
//! the consumer brings from outside: `inflows` (external value, compute, money — named to
//! UNCLAIMED presences, never payees), `heldCapabilities` (what the consumer can do that the
//! network did not grant), and `externalities`. It creates no claim for either party and implies no
//! settlement: the parser refuses terms that say otherwise, because a conversion to token or fiat
//! is a separate settlement Agreement under its own distinct-Steward verdict.
//!
//! Whether the offer is ACTIVE is never written here: it is derived from the verdicts on record by
//! `elohim_epr_cli::flow::memory::offer::offer_standing` (Slice 0's distinct-Steward path).
//!
//! The Intent's `in_scope_of` is the offer document's body CID, so an approval is an approval of
//! the exact text a Steward read: an edit to the offer re-addresses it and needs a fresh verdict.

use std::path::Path;

use cid::Cid;
use elohim_epr_rea::{
    atom_cid, parse_participant_ref, AgentRef, Intent, ParticipantRef, PinnedRef, ReaVerb,
    ResourceSpec,
};
use serde::{Deserialize, Serialize};

use crate::translate::Refusal;

/// The offer's authored declaration (its YAML frontmatter). Serde-strict: an unknown key is a
/// refusal, never a silently ignored term.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Offer {
    pub offer_version: u32,
    pub id: String,
    /// The governing collective's declaration path (repository-relative).
    pub collective: String,
    /// `collective:<id>` — must be the governing declaration's own id.
    pub provider: String,
    /// The consumer, `service:<name>`.
    pub receiver: String,
    /// Who drafted it; the approving Steward must be someone else.
    pub author: String,
    pub recipe: RecipePin,
    pub offered: Vec<String>,
    pub terms: Terms,
    /// True for a consumer the collective does not own: two non-fixture Stewards are then owed.
    pub crosses_network: bool,
    pub disclosure: Disclosure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipePin {
    pub id: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Terms {
    pub optional: bool,
    pub exclusive: bool,
    pub withdrawable: bool,
    /// Always `none`: an offer creates no claim for either party.
    pub claims: String,
    /// Always `none`: settlement is a separate governance act.
    pub settlement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Disclosure {
    pub inflows: Vec<Inflow>,
    pub held_capabilities: Vec<HeldCapability>,
    pub externalities: Externalities,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Inflow {
    /// An UNCLAIMED presence (`presence:<slug>`) — recognition collects under commons
    /// stewardship and redirects if that entity ever attests. Never a payee.
    pub presence: String,
    pub brings: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeldCapability {
    /// The capability id the recipe's `exercises:` binding names (`deploy:kube-credentials`).
    pub id: String,
    pub what: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Externalities {
    pub positive: Vec<String>,
    pub negative: Vec<String>,
}

fn refuse(message: impl Into<String>) -> Refusal {
    Refusal(message.into())
}

fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '/')
}

/// A parsed offer document: the declaration plus the text its body CID is taken over.
#[derive(Debug, Clone)]
pub struct OfferDoc {
    pub path: String,
    pub text: String,
    pub offer: Offer,
}

impl OfferDoc {
    /// Parse and validate an offer document. Every refusal names the term it refuses.
    pub fn parse(path: &str, text: &str) -> Result<Self, Refusal> {
        let front = text
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .map(|(front, _)| front)
            .ok_or_else(|| refuse(format!("{path}: an offer opens with `---` frontmatter")))?;
        let offer: Offer = serde_yaml::from_str(front)
            .map_err(|e| refuse(format!("{path}: offer declaration is malformed: {e}")))?;
        offer.validate(path)?;
        Ok(Self {
            path: path.to_string(),
            text: text.to_string(),
            offer,
        })
    }

    pub fn read(root: &Path, path: &str) -> Result<Self, Refusal> {
        let text = std::fs::read_to_string(root.join(path))
            .map_err(|e| refuse(format!("cannot read offer {path}: {e}")))?;
        Self::parse(path, &text)
    }

    /// The Intent this offer mints: provider (raised_by) the collective, the receiver and terms
    /// in slots, scoped to the offer document's own body CID.
    pub fn intent(&self) -> Intent {
        let o = &self.offer;
        Intent {
            action: ReaVerb::Produce,
            resource_spec: ResourceSpec {
                classified_as: vec![
                    "offer:gift".to_string(),
                    o.id.clone(),
                    format!("receiver:{}", o.receiver),
                    format!("recipe:{}@{}", o.recipe.id, o.recipe.version),
                    "terms:optional,non-exclusive,withdrawable,no-claims,no-settlement".into(),
                ],
                quantity: None,
            },
            in_scope_of: elohim_epr_cli::flow::body_cid(&self.text),
            raised_by: AgentRef(o.provider.clone()),
        }
    }

    pub fn cid(&self) -> Result<Cid, Refusal> {
        atom_cid(&self.intent()).map_err(|e| refuse(e.to_string()))
    }

    pub fn spec(&self) -> PinnedRef {
        PinnedRef {
            id: self.offer.recipe.id.clone(),
            version: self.offer.recipe.version,
        }
    }

    pub fn held(&self) -> Vec<String> {
        self.offer
            .disclosure
            .held_capabilities
            .iter()
            .map(|c| c.id.clone())
            .collect()
    }
}

impl Offer {
    fn validate(&self, path: &str) -> Result<(), Refusal> {
        let at = |why: String| refuse(format!("{path}: {why}"));
        if self.offer_version != 1 {
            return Err(at(format!(
                "offerVersion {} is unknown",
                self.offer_version
            )));
        }
        if !is_slug(&self.id) {
            return Err(at(format!("id `{}` is not a slug", self.id)));
        }
        if !self.collective.ends_with(".epr-meta/collective.json") {
            return Err(at(format!(
                "collective `{}` is not a collective declaration path",
                self.collective
            )));
        }
        if !self
            .provider
            .strip_prefix("collective:")
            .is_some_and(is_slug)
        {
            return Err(at(format!(
                "provider `{}` is not a collective — only a collective offers a gift",
                self.provider
            )));
        }
        match parse_participant_ref(&self.receiver) {
            Ok(ParticipantRef::Service { .. }) => {}
            _ => {
                return Err(at(format!(
                    "receiver `{}` is not a `service:<name>` consumer",
                    self.receiver
                )))
            }
        }
        elohim_epr_rea::parse_acting_participant(&self.author)
            .map_err(|e| at(format!("author: {e}")))?;
        if self.terms.claims != "none" {
            return Err(at(
                "terms.claims must be `none` — an offer creates no claim for either party".into(),
            ));
        }
        if self.terms.settlement != "none" {
            return Err(at(
                "terms.settlement must be `none` — converting a flow to token or fiat is a \
                 separate settlement Agreement under a distinct-Steward verdict, never implied"
                    .into(),
            ));
        }
        if !self.terms.optional || !self.terms.withdrawable || self.terms.exclusive {
            return Err(at(
                "a gift is optional, withdrawable and non-exclusive — refusing it must be free"
                    .into(),
            ));
        }
        for inflow in &self.disclosure.inflows {
            if !inflow
                .presence
                .strip_prefix("presence:")
                .is_some_and(is_slug)
            {
                return Err(at(format!(
                    "inflow `{}` must name an unclaimed `presence:<slug>`, never a payee",
                    inflow.presence
                )));
            }
        }
        for cap in &self.disclosure.held_capabilities {
            if !cap.id.contains(':') || cap.id.trim() != cap.id {
                return Err(at(format!(
                    "held capability `{}` is not a `<kind>:<name>` id",
                    cap.id
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL: &str = include_str!("../../.epr-meta/offers/jenkins-edge-pipeline.offer.md");

    #[test]
    fn the_real_offer_parses_and_mints_a_stable_intent() {
        let doc = OfferDoc::parse("o.md", REAL).expect("the committed offer parses");
        assert_eq!(doc.offer.provider, "collective:ethosengine/elohim");
        assert_eq!(doc.offer.receiver, "service:jenkins");
        assert_eq!(doc.cid().unwrap(), doc.cid().unwrap());
        // An edit to the text re-addresses the offer: approval is of the exact bytes read.
        let edited = OfferDoc::parse("o.md", &format!("{REAL}\nedited\n")).unwrap();
        assert_ne!(edited.cid().unwrap(), doc.cid().unwrap());
    }

    #[test]
    fn terms_that_imply_a_claim_or_settlement_or_a_payee_are_refused() {
        for (from, to) in [
            ("claims: none", "claims: provider"),
            ("settlement: none", "settlement: fiat"),
            ("exclusive: false", "exclusive: true"),
            (
                "presence: presence:jenkins-project",
                "presence: human:matthew",
            ),
            ("receiver: service:jenkins", "receiver: human:adam"),
            (
                "provider: collective:ethosengine/elohim",
                "provider: service:jenkins",
            ),
            (
                "author: agent:implementer@claude-opus-5-5",
                "author: service:jenkins",
            ),
        ] {
            let text = REAL.replacen(from, to, 1);
            assert_ne!(text, REAL, "fixture must change `{from}`");
            assert!(
                OfferDoc::parse("o.md", &text).is_err(),
                "`{to}` must refuse"
            );
        }
    }
}
