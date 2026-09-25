//! # External claims, and the offers that govern them
//!
//! How anything from OUTSIDE the network enters it, and the governed gift that lets it in. Two
//! shapes, both pure (no I/O — reading an offer document from disk and deriving its standing from
//! Steward verdicts belong to `elohim-epr-cli`, which owns the sidecar and the affiliations):
//!
//! - [`ExternalClaim`] — the envelope an outside observation carries onto the observation plane
//!   (`Process` / `FlowEvent`), encoded as prefixed slots on `classified_as`.
//! - [`OfferDeclaration`] / [`OfferDocument`] — the collective's offer to a consumer, a
//!   TWO-SIDED DISCLOSURE, minted as a VF [`Intent`] whose CID every claim it admits is scoped to.
//!
//! **Who uses this envelope.** Jenkins (`bridges/jenkins`) is the first consuming app. The same
//! envelope is the entry for Requests & Offers (hREA / VF via `bridges/valueflows`: an exchange
//! observed on another network enters as a claim scoped to the offer that allowed it) and for an
//! elohim agent authoring OUTSIDE a peer runtime (its act arrives unwitnessed by a conductor, so it
//! enters as a claim at the lowest reach and earns standing inside). Two consumers in-tree today —
//! the Jenkins translation and `epr flow`'s offer standing — which is what admitted the module
//! under this crate's guidestar (§3a: a second independent reader of the same distinction).
//!
//! ## The rules the envelope carries
//!
//! - **Legible without permission.** A claim is a spec plus a CID; anyone may read it.
//! - **No standing travels in.** The source is provenance ([`ParticipantRef::Service`]), never an
//!   actor: it cannot claim, author or attest, and it is never a payee.
//! - **Evaluated on entry, at the lowest reach.** Every claim enters at [`Reach::Private`] and
//!   takes up governance attention only when a participant with standing brings it forward.
//!   [`ExternalClaim::read`] refuses any other reach slot.
//! - **Scoped to the governance that allowed it.** `in_scope_of` is the governing offer's Intent
//!   CID, so every flow traces to the offer a distinct Steward approved.
//!
//! ## The slot encoding
//!
//! The envelope rides on `classified_as` AFTER the established tag-then-subject pair, so no
//! existing positional reader moves: `source:<service>`, `provenance:<where to re-read it>`,
//! `signature:unattested | signature:signed:<did>`, `reach:private`, `in-scope-of:<offer cid>`,
//! in that order. A consumer appends its own prefixed slots after these (a build's `build:`,
//! `url:`, `sha:`).
//!
//! ## The offer
//!
//! Beside what the network gives (`offered`), an offer records what the consumer brings from
//! outside: `inflows` (external value, compute, money — named to UNCLAIMED presences, never
//! payees), `heldCapabilities` (what the consumer can do that the network did not grant) and
//! `externalities`. It creates no claim for either party and implies no settlement:
//! [`OfferDeclaration::validate`] refuses terms that say otherwise, because a conversion to token
//! or fiat is a separate settlement Agreement under its own distinct-Steward verdict. Whether an
//! offer is ACTIVE is never written in it — it is derived from the verdicts on record.

use cid::Cid;
use elohim_epr::reach::Reach;
use elohim_epr::witness::ReaVerb;
use serde::{Deserialize, Serialize};

use crate::actor::{parse_acting_participant, parse_participant_ref, ParticipantRef};
use crate::error::{FabricError, Result};
use crate::model::{AgentRef, Intent, PinnedRef, ResourceSpec};

/// The envelope's slot prefixes, in encoding order.
pub const SOURCE_SLOT: &str = "source:";
pub const PROVENANCE_SLOT: &str = "provenance:";
pub const SIGNATURE_SLOT: &str = "signature:";
pub const REACH_SLOT: &str = "reach:";
pub const IN_SCOPE_OF_SLOT: &str = "in-scope-of:";

fn claim_err(message: impl Into<String>) -> FabricError {
    FabricError::InvalidExternalClaim(message.into())
}

fn offer_err(message: impl Into<String>) -> FabricError {
    FabricError::InvalidOffer(message.into())
}

/// Whether the source signed what it claims. `unattested` is honest absence, never a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureStatus {
    /// Signed by the named DID (verification is the reader's, on entry).
    Signed { did: String },
    /// Nothing signed it — the Jenkins archive today.
    Unattested,
}

impl SignatureStatus {
    /// The value after `signature:`.
    pub fn slot_value(&self) -> String {
        match self {
            SignatureStatus::Signed { did } => format!("signed:{did}"),
            SignatureStatus::Unattested => "unattested".to_string(),
        }
    }

    /// The inverse of [`Self::slot_value`].
    pub fn parse_slot_value(value: &str) -> Result<Self> {
        let status = match value.strip_prefix("signed:") {
            Some(did) => SignatureStatus::Signed {
                did: did.to_string(),
            },
            None if value == "unattested" => SignatureStatus::Unattested,
            None => {
                return Err(claim_err(format!(
                    "signature `{value}` is neither `unattested` nor `signed:<did>`"
                )))
            }
        };
        status.validate()?;
        Ok(status)
    }

    fn validate(&self) -> Result<()> {
        if let SignatureStatus::Signed { did } = self {
            let method = did.strip_prefix("did:").unwrap_or("");
            if method.is_empty() || did.chars().any(char::is_whitespace) {
                return Err(claim_err(format!(
                    "signer `{did}` is not a `did:<method>:<id>`"
                )));
            }
        }
        Ok(())
    }
}

/// The external-claim envelope every observation from outside carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalClaim {
    /// The provenance participant, `service:<name>`.
    source: String,
    /// Where the claim can be re-read at its source (a URL or archive path).
    provenance: String,
    signature: SignatureStatus,
    /// The governing offer's Intent CID.
    in_scope_of: Cid,
}

impl ExternalClaim {
    /// The sole constructor: the source must parse as a SERVICE — a human or agent acting from
    /// outside is a different entry (an authored act under its own claim), not this envelope.
    pub fn new(
        source: &str,
        provenance: &str,
        signature: SignatureStatus,
        in_scope_of: Cid,
    ) -> Result<Self> {
        match parse_participant_ref(source)? {
            ParticipantRef::Service { .. } => {}
            _ => {
                return Err(claim_err(format!(
                    "source `{source}` is not a `service:<name>` — this envelope carries \
                     provenance only"
                )))
            }
        }
        if provenance.trim().is_empty() {
            return Err(claim_err(
                "an external claim needs provenance: where it can be re-read",
            ));
        }
        signature.validate()?;
        Ok(Self {
            source: source.to_string(),
            provenance: provenance.to_string(),
            signature,
            in_scope_of,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn provenance(&self) -> &str {
        &self.provenance
    }

    pub fn signature(&self) -> &SignatureStatus {
        &self.signature
    }

    pub fn in_scope_of(&self) -> Cid {
        self.in_scope_of
    }

    /// The reach an external claim enters at: always the lowest. Standing is earned inside.
    pub fn reach(&self) -> Reach {
        Reach::Private
    }

    fn reach_word() -> String {
        serde_json::to_value(Reach::Private)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "private".into())
    }

    /// The envelope's prefixed `classified_as` slots, in a fixed order. They go AFTER the
    /// record's tag-then-subject pair.
    pub fn slots(&self) -> Vec<String> {
        vec![
            format!("{SOURCE_SLOT}{}", self.source),
            format!("{PROVENANCE_SLOT}{}", self.provenance),
            format!("{SIGNATURE_SLOT}{}", self.signature.slot_value()),
            format!("{REACH_SLOT}{}", Self::reach_word()),
            format!("{IN_SCOPE_OF_SLOT}{}", self.in_scope_of),
        ]
    }

    /// `[tag, subject, envelope…, extra…]` — the whole `classified_as` of a record this claim
    /// carries, in the one order every reader expects.
    pub fn classify(
        &self,
        tag: impl Into<String>,
        subject: impl Into<String>,
        extra: impl IntoIterator<Item = String>,
    ) -> Vec<String> {
        let mut slots = vec![tag.into(), subject.into()];
        slots.extend(self.slots());
        slots.extend(extra);
        slots
    }

    /// Read the envelope back off a record's `classified_as` (positions after tag and subject).
    ///
    /// `Ok(None)` when the record carries no envelope (it is not an external claim). A partial
    /// envelope, a non-service source or any reach but the lowest is refused: the record claims to
    /// be external and says so wrongly.
    pub fn read(classified_as: &[String]) -> Result<Option<Self>> {
        let slots = classified_as.get(2..).unwrap_or_default();
        let find = |prefix: &str| slots.iter().find_map(|s| s.strip_prefix(prefix));
        let Some(source) = find(SOURCE_SLOT) else {
            return Ok(None);
        };
        let missing =
            |prefix: &str| claim_err(format!("envelope from `{source}` has no `{prefix}` slot"));
        let provenance = find(PROVENANCE_SLOT).ok_or_else(|| missing(PROVENANCE_SLOT))?;
        let signature = SignatureStatus::parse_slot_value(
            find(SIGNATURE_SLOT).ok_or_else(|| missing(SIGNATURE_SLOT))?,
        )?;
        let reach = find(REACH_SLOT).ok_or_else(|| missing(REACH_SLOT))?;
        if reach != Self::reach_word() {
            return Err(claim_err(format!(
                "an external claim enters at the lowest reach, not `{reach}` — standing is earned \
                 inside"
            )));
        }
        let scope = find(IN_SCOPE_OF_SLOT).ok_or_else(|| missing(IN_SCOPE_OF_SLOT))?;
        let in_scope_of = Cid::try_from(scope)
            .map_err(|e| claim_err(format!("in-scope-of `{scope}` is not a CID: {e}")))?;
        Self::new(source, provenance, signature, in_scope_of).map(Some)
    }
}

// ── The offer ────────────────────────────────────────────────────────────────────────────────

/// The only offer declaration version this crate reads.
pub const OFFER_VERSION: u32 = 1;

/// The one value `terms.claims` and `terms.settlement` may hold.
pub const NONE_TERM: &str = "none";

/// An offer's authored declaration (its document's frontmatter). Serde-strict: an unknown key is a
/// refusal, never a silently ignored term.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfferDeclaration {
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

/// The recipe an offer pins — strict, unlike [`PinnedRef`], so a stray key is refused.
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
    /// The capability id a recipe's `exercises:` binding names (`deploy:kube-credentials`).
    pub id: String,
    pub what: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Externalities {
    pub positive: Vec<String>,
    pub negative: Vec<String>,
}

fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '/')
}

impl OfferDeclaration {
    /// Validate the declaration. Every refusal names the term it refuses; a term that implies a
    /// claim, a settlement or a payee never validates.
    pub fn validate(&self) -> Result<()> {
        if self.offer_version != OFFER_VERSION {
            return Err(offer_err(format!(
                "offerVersion {} is unknown",
                self.offer_version
            )));
        }
        if !is_slug(&self.id) {
            return Err(offer_err(format!("id `{}` is not a slug", self.id)));
        }
        if !self.collective.ends_with(".epr-meta/collective.json") {
            return Err(offer_err(format!(
                "collective `{}` is not a collective declaration path",
                self.collective
            )));
        }
        if !self
            .provider
            .strip_prefix("collective:")
            .is_some_and(is_slug)
        {
            return Err(offer_err(format!(
                "provider `{}` is not a collective — only a collective offers a gift",
                self.provider
            )));
        }
        match parse_participant_ref(&self.receiver) {
            Ok(ParticipantRef::Service { .. }) => {}
            _ => {
                return Err(offer_err(format!(
                    "receiver `{}` is not a `service:<name>` consumer",
                    self.receiver
                )))
            }
        }
        parse_acting_participant(&self.author).map_err(|e| offer_err(format!("author: {e}")))?;
        if self.terms.claims != NONE_TERM {
            return Err(offer_err(
                "terms.claims must be `none` — an offer creates no claim for either party",
            ));
        }
        if self.terms.settlement != NONE_TERM {
            return Err(offer_err(
                "terms.settlement must be `none` — converting a flow to token or fiat is a \
                 separate settlement Agreement under a distinct-Steward verdict, never implied",
            ));
        }
        if !self.terms.optional || !self.terms.withdrawable || self.terms.exclusive {
            return Err(offer_err(
                "a gift is optional, withdrawable and non-exclusive — refusing it must be free",
            ));
        }
        for inflow in &self.disclosure.inflows {
            if !inflow
                .presence
                .strip_prefix("presence:")
                .is_some_and(is_slug)
            {
                return Err(offer_err(format!(
                    "inflow `{}` must name an unclaimed `presence:<slug>`, never a payee",
                    inflow.presence
                )));
            }
        }
        for cap in &self.disclosure.held_capabilities {
            if !cap.id.contains(':') || cap.id.trim() != cap.id {
                return Err(offer_err(format!(
                    "held capability `{}` is not a `<kind>:<name>` id",
                    cap.id
                )));
            }
        }
        Ok(())
    }

    /// The recipe pin, as the observation plane spells it.
    pub fn spec(&self) -> PinnedRef {
        PinnedRef {
            id: self.recipe.id.clone(),
            version: self.recipe.version,
        }
    }

    /// The capability ids the consumer disclosed holding.
    pub fn held(&self) -> Vec<String> {
        self.disclosure
            .held_capabilities
            .iter()
            .map(|c| c.id.clone())
            .collect()
    }

    /// The Intent this offer mints: provider (`raised_by`) the collective, the receiver and terms
    /// in slots, scoped to `body` — the offer document's own body CID, so an approval is an
    /// approval of the exact text a Steward read and an edit re-addresses the offer.
    pub fn intent(&self, body: Cid) -> Intent {
        Intent {
            action: ReaVerb::Produce,
            resource_spec: ResourceSpec {
                classified_as: vec![
                    "offer:gift".to_string(),
                    self.id.clone(),
                    format!("receiver:{}", self.receiver),
                    format!("recipe:{}@{}", self.recipe.id, self.recipe.version),
                    "terms:optional,non-exclusive,withdrawable,no-claims,no-settlement".into(),
                ],
                quantity: None,
            },
            in_scope_of: body,
            raised_by: AgentRef(self.provider.clone()),
        }
    }
}

/// An offer document: its path, the exact text an approval is of, and the validated declaration.
/// Constructed only through [`OfferDocument::new`], so a held document is always a valid one.
#[derive(Debug, Clone, PartialEq)]
pub struct OfferDocument {
    path: String,
    text: String,
    declaration: OfferDeclaration,
}

impl OfferDocument {
    /// The frontmatter of an offer document (between the opening `---` and the next `---`).
    /// Decoding it is the caller's: this crate carries no YAML reader.
    pub fn frontmatter<'a>(path: &str, text: &'a str) -> Result<&'a str> {
        text.strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .map(|(front, _)| front)
            .ok_or_else(|| offer_err(format!("{path}: an offer opens with `---` frontmatter")))
    }

    /// Hold a decoded declaration with the text it came from, refusing an invalid one (the
    /// refusal names the path).
    pub fn new(path: &str, text: &str, declaration: OfferDeclaration) -> Result<Self> {
        declaration.validate().map_err(|e| match e {
            FabricError::InvalidOffer(why) => offer_err(format!("{path}: {why}")),
            other => other,
        })?;
        Ok(Self {
            path: path.to_string(),
            text: text.to_string(),
            declaration,
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn declaration(&self) -> &OfferDeclaration {
        &self.declaration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::atom_cid;

    fn scope() -> Cid {
        elohim_epr::cid::compute_cid(b"offer")
    }

    #[test]
    fn only_a_service_can_be_an_external_claim_source() {
        assert!(
            ExternalClaim::new("service:jenkins", "u", SignatureStatus::Unattested, scope())
                .is_ok()
        );
        for actor in [
            "human:matthew",
            "agent:scribe@opus-5",
            "repo:ethosengine/elohim",
        ] {
            assert!(
                ExternalClaim::new(actor, "u", SignatureStatus::Unattested, scope()).is_err(),
                "{actor} must not enter as an external claim source"
            );
        }
        assert!(
            ExternalClaim::new("service:jenkins", " ", SignatureStatus::Unattested, scope())
                .is_err()
        );
        assert!(ExternalClaim::new(
            "service:jenkins",
            "u",
            SignatureStatus::Signed { did: "z6Mk".into() },
            scope()
        )
        .is_err());
    }

    #[test]
    fn a_claim_enters_at_the_lowest_reach_with_its_signature_status() {
        let claim = ExternalClaim::new(
            "service:jenkins",
            "https://x/1/",
            SignatureStatus::Signed {
                did: "did:key:z6Mk".into(),
            },
            scope(),
        )
        .unwrap();
        assert_eq!(claim.reach().openness(), 1);
        let slots = claim.slots();
        assert!(slots.contains(&"reach:private".to_string()));
        assert!(slots.contains(&"signature:signed:did:key:z6Mk".to_string()));
        assert!(slots.contains(&format!("in-scope-of:{}", scope())));
    }

    #[test]
    fn the_envelope_reads_back_off_classified_as_and_refuses_a_raised_reach() {
        let claim = ExternalClaim::new(
            "service:jenkins",
            "https://x/1/",
            SignatureStatus::Unattested,
            scope(),
        )
        .unwrap();
        let slots = claim.classify("result:success", "Build", ["build:1".to_string()]);
        assert_eq!(&slots[..2], ["result:success", "Build"]);
        assert_eq!(slots.last().unwrap(), "build:1");
        assert_eq!(ExternalClaim::read(&slots).unwrap(), Some(claim));

        // No envelope: not an external claim, and not an error.
        assert_eq!(
            ExternalClaim::read(&["gap:open".into(), "x".into()]).unwrap(),
            None
        );
        // The tag/subject pair is never read as envelope.
        assert_eq!(
            ExternalClaim::read(&["source:service:jenkins".into(), "x".into()]).unwrap(),
            None
        );
        let raised: Vec<String> = slots
            .iter()
            .map(|s| s.replace("reach:private", "reach:public"))
            .collect();
        assert!(ExternalClaim::read(&raised).is_err());
        let partial: Vec<String> = slots
            .iter()
            .filter(|s| !s.starts_with(IN_SCOPE_OF_SLOT))
            .cloned()
            .collect();
        assert!(ExternalClaim::read(&partial).is_err());
    }

    fn declaration() -> OfferDeclaration {
        OfferDeclaration {
            offer_version: 1,
            id: "ci-offer".into(),
            collective: ".epr-meta/collective.json".into(),
            provider: "collective:ethosengine/elohim".into(),
            receiver: "service:jenkins".into(),
            author: "agent:implementer@claude-opus-5-5".into(),
            recipe: RecipePin {
                id: "edge-pipeline".into(),
                version: 1,
            },
            offered: vec!["drift".into()],
            terms: Terms {
                optional: true,
                exclusive: false,
                withdrawable: true,
                claims: "none".into(),
                settlement: "none".into(),
            },
            crosses_network: false,
            disclosure: Disclosure {
                inflows: vec![Inflow {
                    presence: "presence:jenkins-project".into(),
                    brings: "the server".into(),
                }],
                held_capabilities: vec![HeldCapability {
                    id: "deploy:kube-credentials".into(),
                    what: "kubectl".into(),
                }],
                externalities: Externalities {
                    positive: vec![],
                    negative: vec![],
                },
            },
        }
    }

    #[test]
    fn an_offer_refuses_a_claim_a_settlement_or_a_payee() {
        assert!(declaration().validate().is_ok());
        let mut claims = declaration();
        claims.terms.claims = "provider".into();
        let mut settles = declaration();
        settles.terms.settlement = "fiat".into();
        let mut payee = declaration();
        payee.disclosure.inflows[0].presence = "human:matthew".into();
        let mut exclusive = declaration();
        exclusive.terms.exclusive = true;
        let mut service_author = declaration();
        service_author.author = "service:jenkins".into();
        let mut human_receiver = declaration();
        human_receiver.receiver = "human:adam".into();
        for (what, bad) in [
            ("claims", claims),
            ("settlement", settles),
            ("payee", payee),
            ("exclusive", exclusive),
            ("service author", service_author),
            ("human receiver", human_receiver),
        ] {
            assert!(
                matches!(bad.validate(), Err(FabricError::InvalidOffer(_))),
                "{what} must refuse"
            );
            assert!(OfferDocument::new("o.md", "", bad).is_err(), "{what}");
        }
    }

    #[test]
    fn an_unknown_term_is_refused_and_the_intent_is_scoped_to_the_body() {
        let mut json = serde_json::to_value(declaration()).unwrap();
        json["terms"]["refundable"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<OfferDeclaration>(json).is_err());

        let a = declaration().intent(scope());
        let b = declaration().intent(elohim_epr::cid::compute_cid(b"edited"));
        assert_eq!(a.raised_by.0, "collective:ethosengine/elohim");
        assert_ne!(atom_cid(&a).unwrap(), atom_cid(&b).unwrap());

        assert!(OfferDocument::frontmatter("o.md", "no frontmatter").is_err());
        assert_eq!(
            OfferDocument::frontmatter("o.md", "---\nid: x\n---\nbody").unwrap(),
            "id: x"
        );
    }
}
