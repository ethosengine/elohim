//! The governed offer this bridge operates under: the collective's gift to `service:jenkins`,
//! authored as `bridges/.epr-meta/offers/jenkins-edge-pipeline.offer.md`.
//!
//! Nothing offer-shaped is defined here. The declaration (terms, the two-sided disclosure) and its
//! validation are the shared, consumer-agnostic [`elohim_epr_rea::external`] shapes; reading the
//! document, addressing it by its body and deriving its standing from Steward verdicts are
//! `elohim_epr_cli::flow::memory::offer` ([`load_offer`], [`offer_cid`], [`offer_intent`],
//! `offer_standing`). This module only names them for the bridge and pins the committed offer.

pub use elohim_epr_cli::flow::memory::offer::{load_offer, offer_cid, offer_intent, parse_offer};
pub use elohim_epr_rea::{OfferDeclaration, OfferDocument};

#[cfg(test)]
mod tests {
    use super::*;

    const REAL: &str = include_str!("../../.epr-meta/offers/jenkins-edge-pipeline.offer.md");

    #[test]
    fn the_real_offer_parses_and_mints_a_stable_intent() {
        let doc = parse_offer("o.md", REAL).expect("the committed offer parses");
        assert_eq!(doc.declaration().provider, "collective:ethosengine/elohim");
        assert_eq!(doc.declaration().receiver, "service:jenkins");
        assert_eq!(offer_cid(&doc).unwrap(), offer_cid(&doc).unwrap());
        // An edit to the text re-addresses the offer: approval is of the exact bytes read.
        let edited = parse_offer("o.md", &format!("{REAL}\nedited\n")).unwrap();
        assert_ne!(offer_cid(&edited).unwrap(), offer_cid(&doc).unwrap());
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
            (
                "crossesNetwork: false",
                "crossesNetwork: false\npayee: human:matthew",
            ),
        ] {
            let text = REAL.replacen(from, to, 1);
            assert_ne!(text, REAL, "fixture must change `{from}`");
            assert!(parse_offer("o.md", &text).is_err(), "`{to}` must refuse");
        }
    }
}
