//! The fair-trade receipt — what was exchanged for THIS serve, and who was
//! credited for it.
//!
//! A visit is not free and it is not a favour: someone held the bytes, someone
//! projected them, and the network books that work. The receipt is the chrome
//! telling the visitor so in a sentence they would say themselves, with the
//! ledger records behind it for anyone who asks for the precise form.
//!
//! ## A reading, never a record
//!
//! This module mints NOTHING. The receipt is Category C — a READING of REA
//! Commitments that already exist and were already notarized before this
//! request arrived:
//!
//! | clause | the record it reads | what it says |
//! |--------|---------------------|--------------|
//! | `exchanged` | the `hosting-agreement` commitment | what was given so this could be kept ready |
//! | `held` (credit) | the `project-epr` commitment | who holds the bytes and answered for them |
//! | `projected` (credit) | the `operate-doorway` commitment | who runs the doorway that put it in front of you |
//!
//! Three separate records, which is what makes "credits the holder separately
//! from the doorway" structurally true rather than a label this doorway chose.
//!
//! A per-serve `serve-blob` `EconomicEvent` on the DHT was REFUSED at the
//! design gate: 10⁴–10⁶ heads a year on a plane whose one measured anchor is
//! ~3,469 heads (≈2.5 h to quiesce). The successor is one PERIODIC aggregated
//! `serve-blob` event fulfilling the hosting agreement, sourced from the
//! `infrastructure:blob-served` observations that already exist. So: **the
//! receipt names who was CREDITED, never who was SERVED.** No per-visitor serve
//! record may land on any notarized plane, and nothing in this module writes
//! one.
//!
//! ## Honest absence, never a synthesized number
//!
//! A clause whose record this doorway does not hold is `null` AND its name
//! appears in [`Receipt::unrecorded`]. A receipt that quietly omitted a missing
//! credit would read as "there was nothing to credit"; a receipt that invented
//! one would be the doorway saying something no record backs. Both are the
//! failure this module exists to refuse — "a doorway that credits itself for a
//! holder's work" is the specific shape the story pins.
//!
//! ## The words a friend would use, the protocol form one request away
//!
//! [`Receipt::sentence`] is what the chrome leads with, and it carries no
//! protocol identifier: a person who has never heard of this protocol reads a
//! sentence, not a CID. Every clause additionally carries the route that reads
//! its record back, and [`Receipt::protocol_form`] is the one request that
//! returns the underlying `ReaCommitmentView`s verbatim.

use serde::Serialize;

/// Response header naming where the receipt for a serve can be read.
///
/// Stamped beside `x-elohim-standing: admitted;reach=…` on every serve the
/// fold admitted, so the chrome never has to guess that a visit had an economic
/// account at all. Relayed VERBATIM by a courier doorway (the holder minted it
/// about its own serve), the same rule the standing header follows.
pub const RECEIPT_HEADER: &str = "x-elohim-receipt";

/// Route prefix the receipt is read from. Relative, so it is the asked
/// doorway's own route whichever doorway answered.
pub const RECEIPT_ROUTE_PREFIX: &str = "/api/v1/receipt/";

/// Query that asks for the protocol form — the three underlying
/// `ReaCommitmentView`s verbatim, exactly as the ledger holds them.
pub const PROTOCOL_FORM_QUERY: &str = "form=protocol";

/// Route prefix that reads one REA Commitment back. Proxied by every doorway
/// to its own storage, so the path is relative to whichever doorway answered.
const COMMITMENT_RECORD_ROUTE: &str = "/api/v1/commitments/";

/// The clause names a receipt may report as unrecorded. Closed on purpose: a
/// reader can enumerate what a receipt could possibly be missing without
/// running one.
pub const CLAUSE_EXCHANGED: &str = "exchanged";
/// See [`CLAUSE_EXCHANGED`].
pub const CLAUSE_HELD: &str = "held";
/// See [`CLAUSE_EXCHANGED`].
pub const CLAUSE_PROJECTED: &str = "projected";

/// The lead sentence. Says what happened in words, names nobody's identifier,
/// and is the FIRST thing the receipt says — the checkable half of "written in
/// the words a friend would use".
const LEAD_SENTENCE: &str =
    "Someone kept this ready and someone put it in front of you. Here is who was \
     credited for that, and what they were given in return.";

// ════════════════════════════════════════════════════════════════════════════
// The clause inputs — what a caller read off the ledger
// ════════════════════════════════════════════════════════════════════════════

/// The `hosting-agreement` commitment: what was given so this record could be
/// kept ready. Read from the ledger; never derived from the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangeClause {
    /// The commitment id — the record this clause reads back to.
    pub commitment_id: String,
    /// The party who gives it.
    pub party: String,
    /// The resource classification the commitment carries (`compute`, …).
    /// Spoken through [`spoken_resource`] so a person reads words.
    pub resource: Option<String>,
}

/// The `project-epr` commitment: who holds the bytes and answered for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldClause {
    pub commitment_id: String,
    pub party: String,
    pub party_label: Option<String>,
}

/// The `operate-doorway` commitment: who runs the doorway that projected it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedClause {
    pub commitment_id: String,
    pub party: String,
    pub party_label: Option<String>,
    /// The doorway this operator runs — the one that answered this request.
    pub doorway_id: String,
}

// ════════════════════════════════════════════════════════════════════════════
// The wire shape
// ════════════════════════════════════════════════════════════════════════════

/// Which side of the serve a credit is for.
///
/// Two roles, never one: the holder and the projector are different parties
/// doing different work under different commitments, and collapsing them is
/// exactly how "a doorway credits itself for a holder's work" happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CreditRole {
    /// Held the bytes and answered for them (the `project-epr` contract).
    Held,
    /// Ran the doorway that projected them (the `operate-doorway` binding).
    Projected,
}

/// One credit line: who was credited, for what, and the record it reads back to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credit {
    pub role: CreditRole,
    /// The party credited, as the commitment names them.
    pub party: String,
    /// The name a person would use, when the ledger holds one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party_label: Option<String>,
    /// What this party did, said the way a friend would say it.
    pub sentence: String,
    /// The commitment id this credit reads back to.
    pub commitment: String,
    /// The route that reads that commitment back on the doorway that answered.
    pub record: String,
}

/// What was given in exchange for the serve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exchange {
    /// The thing given, in words (`computing time and storage`, …).
    pub what: String,
    /// The whole clause as a sentence.
    pub sentence: String,
    /// The party who gives it, as the agreement names them.
    pub party: String,
    /// The resource classification the agreement carries, VERBATIM.
    ///
    /// Carried beside the spoken `what` so a courier doorway can restate the
    /// holder's clause in its own sentence without re-deriving the holder's
    /// facts, and so a reader can tell the ledger's word from ours.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    pub commitment: String,
    pub record: String,
}

/// The chrome's plain-language account of the exchange this one serve was.
///
/// Field ORDER is load-bearing: `sentence` is declared first so it is the first
/// thing the serialized receipt says. A receipt that led with an EPR id would
/// fail its own story's assertion before a reader got to the words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    /// The words a friend would use — no identifier, no protocol vocabulary.
    pub sentence: String,
    /// The record this receipt is about.
    pub epr: String,
    /// What was given so this could be kept ready. `None` (and named in
    /// [`Self::unrecorded`]) when this doorway holds no hosting agreement for it.
    pub exchanged: Option<Exchange>,
    /// Who was credited. Never more than one credit per [`CreditRole`], and a
    /// role whose record is missing is ABSENT here and NAMED in
    /// [`Self::unrecorded`] — never folded into the other role.
    pub credits: Vec<Credit>,
    /// The clauses this doorway holds no record for, by name. Empty on a
    /// complete receipt.
    pub unrecorded: Vec<&'static str>,
    /// The one request that returns the underlying commitments verbatim.
    pub protocol_form: String,
    /// The origin that supplied the bytes, when this receipt was merged from a
    /// holder's own answer. Absent when this doorway is the holder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub served_by: Option<String>,
}

impl Receipt {
    /// The credit for one role, when the receipt carries it.
    #[must_use]
    pub fn credit(&self, role: CreditRole) -> Option<&Credit> {
        self.credits.iter().find(|c| c.role == role)
    }

    /// Whether every clause reads back to a record — i.e. nothing is unrecorded.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unrecorded.is_empty()
    }
}

/// How a challenge to a standing was answered. **Closed at birth** — every arm
/// a `POST /api/v1/challenge` can take is named here, so a reader can enumerate
/// the outcomes without running one, and a new arm cannot be added silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChallengeOutcome {
    /// 201 — witnessed. A commitment was minted naming the party that owes the
    /// response and the window it is owed in.
    Witnessed,
    /// 200 — heard, but the contract challenged declares no responsive-reach
    /// term, so there is nobody this doorway may name as owing an answer.
    /// `owed: null` plus the reason. A doorway that invented a party here would
    /// be answering for a collective that never agreed to answer.
    NothingOwed,
    /// 200 — this exact challenge is already witnessed. Ensure-not-create: a
    /// replay mints nothing twice and returns the standing record.
    Replay,
    /// 401 — a challenge needs someone to answer TO. An anonymous challenge is
    /// refused with a named reason rather than witnessed against nobody.
    Anonymous,
    /// 409 — the same challenge id already stands for DIFFERENT words. Two
    /// different challenges may not share one record.
    Conflict,
    /// 503 — the substrate could not witness it right now. The visitor is told
    /// so; the challenge is not silently dropped.
    Shed,
}

impl ChallengeOutcome {
    /// The HTTP status this outcome answers with.
    #[must_use]
    pub fn status(self) -> u16 {
        match self {
            Self::Witnessed => 201,
            Self::NothingOwed | Self::Replay => 200,
            Self::Anonymous => 401,
            Self::Conflict => 409,
            Self::Shed => 503,
        }
    }

    /// Whether a commitment stands on the ledger as a result of this outcome.
    #[must_use]
    pub fn is_witnessed(self) -> bool {
        matches!(self, Self::Witnessed | Self::Replay)
    }

    /// Why, in the words a friend would use.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::Witnessed => {
                "Your challenge is on the record, and the people who made this decision \
                 owe you an answer by the date below."
            }
            Self::NothingOwed => {
                "Your challenge was heard, but the people who made this decision have not \
                 said who answers challenges about it or how soon. Nobody can be named as \
                 owing you a reply until they do, and this doorway will not name one for them."
            }
            Self::Replay => {
                "You have already sent this challenge. It is still on the record — here it is."
            }
            Self::Anonymous => {
                "A challenge needs somebody to answer to. Sign in and send it again, so the \
                 answer has somewhere to go."
            }
            Self::Conflict => {
                "A different challenge is already on the record under this reference. Send \
                 yours again and it will be recorded on its own."
            }
            Self::Shed => {
                "This doorway could not put your challenge on the record just now. Nothing \
                 was lost and nothing was recorded — please try again shortly."
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// The build
// ════════════════════════════════════════════════════════════════════════════

/// Build the receipt for one serve from the clauses a caller read off the
/// ledger.
///
/// PURE — no I/O, no clock, no global state. A clause that is `None` is a
/// clause whose record the caller could not find, and this function's whole
/// judgement is what to say about that: the field stays `null` and its NAME
/// goes in `unrecorded`. Nothing is synthesized, nothing is folded into a
/// neighbouring credit, and a missing holder never quietly becomes the
/// doorway's own credit.
#[must_use]
pub fn build_receipt(
    epr_id: &str,
    exchanged: Option<&ExchangeClause>,
    held: Option<&HeldClause>,
    projected: Option<&ProjectedClause>,
    served_by: Option<&str>,
) -> Receipt {
    let mut unrecorded: Vec<&'static str> = Vec::new();
    let mut credits: Vec<Credit> = Vec::new();

    let exchange = match exchanged {
        Some(clause) => {
            let what = spoken_resource(clause.resource.as_deref());
            Some(Exchange {
                sentence: format!(
                    "{} gives {what} so this can be kept ready for whoever asks — an \
                     agreement made in advance, not a charge to you.",
                    clause.party
                ),
                what,
                party: clause.party.clone(),
                resource: clause.resource.clone(),
                commitment: clause.commitment_id.clone(),
                record: commitment_record(&clause.commitment_id),
            })
        }
        None => {
            unrecorded.push(CLAUSE_EXCHANGED);
            None
        }
    };

    match held {
        Some(clause) => credits.push(Credit {
            role: CreditRole::Held,
            sentence: format!(
                "{} keeps this and handed over the copy you are reading.",
                spoken_party(&clause.party, clause.party_label.as_deref())
            ),
            party: clause.party.clone(),
            party_label: clause.party_label.clone(),
            commitment: clause.commitment_id.clone(),
            record: commitment_record(&clause.commitment_id),
        }),
        None => unrecorded.push(CLAUSE_HELD),
    }

    match projected {
        Some(clause) => credits.push(Credit {
            role: CreditRole::Projected,
            sentence: format!(
                "{} runs the doorway that put it in front of you.",
                spoken_party(&clause.party, clause.party_label.as_deref())
            ),
            party: clause.party.clone(),
            party_label: clause.party_label.clone(),
            commitment: clause.commitment_id.clone(),
            record: commitment_record(&clause.commitment_id),
        }),
        None => unrecorded.push(CLAUSE_PROJECTED),
    }

    Receipt {
        sentence: LEAD_SENTENCE.to_string(),
        epr: epr_id.to_string(),
        exchanged: exchange,
        credits,
        unrecorded,
        protocol_form: format!("{}?{PROTOCOL_FORM_QUERY}", receipt_header_value(epr_id)),
        served_by: served_by
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    }
}

/// The `x-elohim-receipt` value for a serve of `epr_id` — the route the chrome
/// reads the receipt from, relative to whichever doorway answered.
///
/// The id is percent-encoded for the same reason the refusal's records are: it
/// originates in a projection row, and a row is not a place to trust bytes from.
#[must_use]
pub fn receipt_header_value(epr_id: &str) -> String {
    format!(
        "{RECEIPT_ROUTE_PREFIX}{}",
        urlencoding::encode(epr_id.trim())
    )
}

/// Stamp [`RECEIPT_HEADER`] on a response the fold admitted.
///
/// The same two restraints [`crate::services::stamp_admitted_standing`] keeps:
/// only on a response that actually SERVED something (a shed or a 502 had no
/// exchange to account for), and never overwriting a value already present — a
/// relayed answer carries the HOLDER's receipt pointer verbatim, because the
/// holder is the doorway whose serve it accounts for.
pub fn stamp_receipt<B>(response: &mut hyper::Response<B>, epr_id: &str) {
    let status = response.status();
    if !(status.is_success() || status.is_redirection()) {
        return;
    }
    if response.headers().contains_key(RECEIPT_HEADER) {
        return;
    }
    if let Ok(value) = hyper::header::HeaderValue::from_str(&receipt_header_value(epr_id)) {
        response.headers_mut().insert(RECEIPT_HEADER, value);
    }
}

/// Say a resource classification the way a person would.
///
/// An unknown classification is spoken VERBATIM rather than dropped or replaced
/// with a generic phrase: the vocabulary is open (a new `resource_classified_as`
/// entry is a protocol-class extension, not a schema change), and a doorway
/// that silently renamed an unrecognised one would be putting its own word in
/// a collective's mouth.
#[must_use]
pub fn spoken_resource(resource: Option<&str>) -> String {
    match resource.map(str::trim).filter(|r| !r.is_empty()) {
        None => "what it takes to keep this ready".to_string(),
        Some("compute") => "computing time and storage".to_string(),
        Some("stewardship") => "the care of looking after this".to_string(),
        Some("attention") => "attention".to_string(),
        Some("currency") => "credit".to_string(),
        Some(other) => other.to_string(),
    }
}

/// The name to SAY for a party — its label when the ledger holds one, else its
/// own identifier. Never blank: a credit with no name to say is a credit that
/// cannot be read back.
fn spoken_party(party: &str, label: Option<&str>) -> String {
    match label.map(str::trim).filter(|l| !l.is_empty()) {
        Some(l) => l.to_string(),
        None => party.trim().to_string(),
    }
}

/// The dereferenceable record route for one commitment.
fn commitment_record(commitment_id: &str) -> String {
    format!(
        "{COMMITMENT_RECORD_ROUTE}{}",
        urlencoding::encode(commitment_id.trim())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange() -> ExchangeClause {
        ExchangeClause {
            commitment_id: "hosting-agreement-ef5a1e65191a56a9".into(),
            party: "matthew".into(),
            resource: Some("compute".into()),
        }
    }

    fn held() -> HeldClause {
        HeldClause {
            commitment_id: "project-epr-sus-abc123".into(),
            party: "12D3KooWAlphaHolder".into(),
            party_label: None,
        }
    }

    fn projected() -> ProjectedClause {
        ProjectedClause {
            commitment_id: "operate-doorway-beta-01".into(),
            party: "human-matthew-manager".into(),
            party_label: Some("Matthew".into()),
            doorway_id: "apex-elohim-host".into(),
        }
    }

    fn full() -> Receipt {
        build_receipt(
            "community-garden-club",
            Some(&exchange()),
            Some(&held()),
            Some(&projected()),
            Some("http://localhost:8888"),
        )
    }

    // ── the story's own assertions, at the unit ─────────────────────────────

    /// "the receipt names what was given in exchange for the serve".
    #[test]
    fn the_receipt_names_what_was_given_in_exchange() {
        let receipt = full();
        let exchanged = receipt.exchanged.expect("an exchange clause was supplied");
        assert_eq!(exchanged.what, "computing time and storage");
        assert!(
            exchanged.sentence.contains("computing time and storage"),
            "the clause must SAY what was given, not only carry it in a field: {}",
            exchanged.sentence
        );
        assert_eq!(exchanged.commitment, "hosting-agreement-ef5a1e65191a56a9");
    }

    /// "the receipt credits the holder peer that provided the bytes" AND
    /// "…credits the doorway for the projection, SEPARATELY from the holder".
    #[test]
    fn the_holder_and_the_doorway_are_credited_separately() {
        let receipt = full();
        let holder = receipt
            .credit(CreditRole::Held)
            .expect("the holder must be credited");
        let doorway = receipt
            .credit(CreditRole::Projected)
            .expect("the projecting doorway must be credited");
        assert_ne!(
            holder.commitment, doorway.commitment,
            "the two credits must read back to two DIFFERENT records — one record \
             credited twice is the doorway crediting itself for the holder's work"
        );
        assert_ne!(holder.party, doorway.party);
        assert_eq!(receipt.credits.len(), 2, "exactly one credit per role");
    }

    /// "every credit on the receipt reads back to a record on the shared ledger".
    #[test]
    fn every_credit_reads_back_to_the_record_it_names() {
        let receipt = full();
        for credit in &receipt.credits {
            assert_eq!(
                credit.record,
                format!("/api/v1/commitments/{}", credit.commitment),
                "a credit's record route must dereference the commitment it names"
            );
        }
        let exchanged = receipt.exchanged.expect("exchange present");
        assert_eq!(
            exchanged.record,
            format!("/api/v1/commitments/{}", exchanged.commitment)
        );
    }

    /// The checkable half of "written in the words a friend would use, with the
    /// protocol form one request away": no protocol identifier is the FIRST
    /// thing the receipt says, and the precise form is one request away.
    #[test]
    fn the_lead_sentence_names_no_identifier_and_the_protocol_form_is_one_request_away() {
        let receipt = full();
        let json = serde_json::to_string(&receipt).expect("receipt serializes");
        assert!(
            json.starts_with("{\"sentence\":\""),
            "the first thing a receipt says must be the sentence: {json}"
        );
        for identifier in [
            "community-garden-club",
            "12D3KooWAlphaHolder",
            "hosting-agreement",
            "project-epr",
            "operate-doorway",
            "commitment",
            "REA",
        ] {
            assert!(
                !receipt.sentence.contains(identifier),
                "the lead sentence must carry no protocol identifier, found {identifier:?} in {:?}",
                receipt.sentence
            );
        }
        assert_eq!(
            receipt.protocol_form, "/api/v1/receipt/community-garden-club?form=protocol",
            "the protocol form must be reachable in ONE request"
        );
    }

    // ── honest absence ──────────────────────────────────────────────────────

    /// A missing record is `null` AND named — never a synthesized credit, and
    /// never a silent omission that reads as "there was nothing to credit".
    #[test]
    fn a_missing_clause_is_null_and_named_never_invented() {
        let receipt = build_receipt(
            "community-garden-club",
            None,
            Some(&held()),
            Some(&projected()),
            None,
        );
        assert!(receipt.exchanged.is_none());
        assert_eq!(receipt.unrecorded, vec!["exchanged"]);
        assert!(!receipt.is_complete());
        assert_eq!(receipt.credits.len(), 2, "the credits are untouched");
    }

    #[test]
    fn a_missing_holder_never_becomes_the_doorways_own_credit() {
        let receipt = build_receipt(
            "community-garden-club",
            Some(&exchange()),
            None,
            Some(&projected()),
            None,
        );
        assert!(
            receipt.credit(CreditRole::Held).is_none(),
            "an absent holder must stay absent"
        );
        assert_eq!(receipt.unrecorded, vec!["held"]);
        assert_eq!(
            receipt.credit(CreditRole::Projected).map(|c| &c.party),
            Some(&"human-matthew-manager".to_string()),
            "the doorway's own credit must not have absorbed the holder's"
        );
    }

    #[test]
    fn a_receipt_with_nothing_to_say_names_all_three_absences() {
        let receipt = build_receipt("community-garden-club", None, None, None, None);
        assert_eq!(
            receipt.unrecorded,
            vec!["exchanged", "held", "projected"],
            "every clause absent must be named, in a stable order"
        );
        assert!(receipt.credits.is_empty());
        assert!(receipt.exchanged.is_none());
        // Still a receipt, still says the same words: absence is reported, not
        // hidden behind an error.
        assert_eq!(receipt.sentence, LEAD_SENTENCE);
    }

    /// A complete receipt names no absence — the anti-vacuity half, so
    /// `unrecorded` cannot quietly become "always everything".
    #[test]
    fn a_complete_receipt_names_no_absence() {
        assert!(full().is_complete());
        assert!(full().unrecorded.is_empty());
    }

    // ── the spoken forms ────────────────────────────────────────────────────

    #[test]
    fn an_unknown_resource_class_is_spoken_verbatim_not_renamed() {
        assert_eq!(spoken_resource(Some("care-token")), "care-token");
        assert_eq!(
            spoken_resource(Some("compute")),
            "computing time and storage"
        );
        assert_eq!(
            spoken_resource(None),
            "what it takes to keep this ready",
            "no classification must still say something true, never a blank"
        );
        assert_eq!(
            spoken_resource(Some("   ")),
            "what it takes to keep this ready"
        );
    }

    #[test]
    fn a_party_with_a_label_is_said_by_name_and_one_without_by_its_own_id() {
        let receipt = full();
        assert!(receipt
            .credit(CreditRole::Projected)
            .expect("projected")
            .sentence
            .contains("Matthew"));
        assert!(receipt
            .credit(CreditRole::Held)
            .expect("held")
            .sentence
            .contains("12D3KooWAlphaHolder"));
    }

    // ── the header ──────────────────────────────────────────────────────────

    #[test]
    fn the_receipt_header_points_at_this_doorways_own_route() {
        assert_eq!(
            receipt_header_value("community-garden-club"),
            "/api/v1/receipt/community-garden-club"
        );
    }

    #[test]
    fn a_hostile_epr_id_cannot_smuggle_bytes_into_the_header() {
        let value = receipt_header_value("a b\"/../admin");
        assert!(
            !value.contains(' ') && !value.contains('"'),
            "the id is percent-encoded before it reaches a header: {value}"
        );
        assert!(value.starts_with("/api/v1/receipt/"));
    }

    #[test]
    fn a_served_response_is_stamped_and_a_relayed_one_is_left_alone() {
        use hyper::{Response, StatusCode};

        let mut served = Response::builder()
            .status(StatusCode::OK)
            .body(())
            .expect("response builds");
        stamp_receipt(&mut served, "community-garden-club");
        assert_eq!(
            served.headers().get(RECEIPT_HEADER).unwrap(),
            "/api/v1/receipt/community-garden-club"
        );

        // The holder already said it; the courier does not restate it.
        let mut relayed = Response::builder()
            .status(StatusCode::OK)
            .header(RECEIPT_HEADER, "/api/v1/receipt/held-elsewhere")
            .body(())
            .expect("response builds");
        stamp_receipt(&mut relayed, "community-garden-club");
        assert_eq!(
            relayed.headers().get(RECEIPT_HEADER).unwrap(),
            "/api/v1/receipt/held-elsewhere"
        );
    }

    #[test]
    fn a_shed_is_never_labelled_with_a_receipt() {
        use hyper::{Response, StatusCode};
        let mut shed = Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(())
            .expect("response builds");
        stamp_receipt(&mut shed, "community-garden-club");
        assert!(
            shed.headers().get(RECEIPT_HEADER).is_none(),
            "a response that served nothing has no exchange to account for"
        );
    }

    // ── the challenge outcome vocabulary ────────────────────────────────────

    #[test]
    fn every_challenge_outcome_carries_its_own_status_and_reason() {
        for outcome in [
            ChallengeOutcome::Witnessed,
            ChallengeOutcome::NothingOwed,
            ChallengeOutcome::Replay,
            ChallengeOutcome::Anonymous,
            ChallengeOutcome::Conflict,
            ChallengeOutcome::Shed,
        ] {
            assert!(
                (200..=599).contains(&outcome.status()),
                "{outcome:?} must answer with a real status"
            );
            assert!(
                !outcome.reason().is_empty(),
                "{outcome:?} must say WHY in words"
            );
        }
        assert_eq!(ChallengeOutcome::Witnessed.status(), 201);
        assert_eq!(ChallengeOutcome::NothingOwed.status(), 200);
        assert_eq!(ChallengeOutcome::Replay.status(), 200);
        assert_eq!(ChallengeOutcome::Anonymous.status(), 401);
        assert_eq!(ChallengeOutcome::Conflict.status(), 409);
        assert_eq!(ChallengeOutcome::Shed.status(), 503);
    }

    /// The two 200s are NOT the same answer, and the distinction is the
    /// honest-absence one: a replay stands on the ledger, a nothing-owed does
    /// not name anybody as owing.
    #[test]
    fn nothing_owed_is_not_witnessed_and_a_replay_is() {
        assert!(!ChallengeOutcome::NothingOwed.is_witnessed());
        assert!(ChallengeOutcome::Replay.is_witnessed());
        assert!(ChallengeOutcome::Witnessed.is_witnessed());
        assert!(!ChallengeOutcome::Anonymous.is_witnessed());
        assert!(!ChallengeOutcome::Shed.is_witnessed());
    }

    #[test]
    fn challenge_outcomes_serialize_as_words_a_reader_can_match_on() {
        assert_eq!(
            serde_json::to_string(&ChallengeOutcome::NothingOwed).unwrap(),
            "\"nothingOwed\""
        );
        assert_eq!(
            serde_json::to_string(&ChallengeOutcome::Witnessed).unwrap(),
            "\"witnessed\""
        );
    }
}
