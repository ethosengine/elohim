//! Serving eligibility — the REACH and STANDING terms of the four-term fold.
//!
//! A doorway putting a page in front of a stranger holds no authority of its own
//! over what it serves. The right to project THIS record, to THIS person, RIGHT
//! NOW is a privilege the network affords it, derived from records anyone can
//! read back, and it lasts only as long as those records say it does. That
//! privilege is resolved at the moment of the request — never cached alongside
//! the bytes.
//!
//! The invariant this module carries is declared in
//! `doorway/doorway-service/.epr-meta/served-under-standing.habit.md`:
//!
//! > Bytes may be held warm; the fold may not.
//!
//! ## The four terms, and the two this module owns
//!
//! | term      | who decides it                                              |
//! |-----------|-------------------------------------------------------------|
//! | contract  | dispatch / name routing (a mount exists ⇒ we hold one)       |
//! | liveness  | the breaker + shed path (`ShellPlan::Shed`, admission)       |
//! | **reach** | **this module** — the EPR's declared reach, as projected NOW |
//! | **standing** | **this module** — what the requester can show for themselves |
//!
//! Contract and liveness are already decided by the time a request reaches a
//! serve path: a request that reaches [`serve_eligibility`] has matched a live
//! projection row, and a doorway that cannot serve has already shed. What is
//! left — and what no cached-serve path asked before this module existed — is
//! whether the reach the collective declares still admits this requester.
//!
//! ## Never a value cached beside the bytes
//!
//! Every caller reads `head_reach` from the CURRENT projection row (the EPR
//! router's table, rebuilt wholesale by `EprRouter::replace_all` on every
//! reconcile). Nothing in this module reads a reach stored next to a cached
//! body, a warm shell, or a blob. That is the whole point: a collective's
//! ruling that narrows an EPR's reach lands in the router table on the next
//! refresh, and the very next serve folds differently — with no restart and no
//! eviction. The bytes stay warm; the permission does not.
//!
//! ## Never widen
//!
//! Absence of reach information is never read as permission. An undeclared
//! reach refuses everyone (exactly the pre-existing behaviour of the
//! `anon_reach_readable` gate this module replaces), and an unknown rung on the
//! ladder refuses anonymous visitors. Only an explicitly commons reach serves a
//! visitor who has shown nothing.
//!
//! ## The fold says which answer it gave, both ways
//!
//! A refusal NAMES, on its face, which term failed (`refused: "reach"`), the
//! reach that failed it, why, where the visitor can be heard about it, and —
//! when the declaration names one — the COLLECTIVE the reach now admits, said
//! by name. It also points at the ledger records the reason reads back to: the
//! EPR, the REA commitment that backs the projection contract, the route that
//! reads that commitment's reach declaration back, and the route that reads the
//! collective back. So "decided by the fold" is checkable as "the reason it
//! gave reads back to a record". A doorway-local allowlist can name no such
//! record — which is why the naming, not the 403, is the observable.
//!
//! A SERVE says so too: [`stamp_admitted_standing`] puts
//! `x-elohim-standing: admitted;reach=<reach>` on the way out, so the chrome can
//! tell a person what governs the place they walked into instead of inferring
//! "it must have been fine" from a 200.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;

/// Where a refused visitor is pointed so they can be heard about the decision.
///
/// The accountable-correction submission outbox — a real, witnessed route
/// (`POST /api/v1/feedback/operations`, declared by storage's manifest and
/// compiled into the doorway's route registry), not a mailto or a dead link. A
/// challenge that lands in a queue nobody owes anything to is the suggestion
/// box this protocol replaces.
pub const WHERE_TO_BE_HEARD: &str = "/api/v1/feedback/operations";

/// Response header the chrome (and the a2o glue) reads to see the standing the
/// doorway resolved for this request. Shape: `refused;reach=<reach>` on a
/// refusal, `admitted;reach=<reach>` on a serve the fold admitted — the same
/// sentence either way, so a reader never has to infer the fold's answer from
/// the status code.
pub const STANDING_HEADER: &str = "x-elohim-standing";

/// Route prefix a refusal points at so the collective it named reads back to a
/// record. Proxied by every doorway to its own storage, so the path is
/// relative to whichever doorway refused.
const COLLECTIVE_RECORD_ROUTE: &str = "/db/collectives/";

/// Route prefix a refusal points at so the REACH DECLARATION reads back to a
/// record: the REA Commitment that carries this projection's reach and its
/// audience terms.
const REACH_DECLARATION_ROUTE: &str = "/api/v1/commitments/";

/// The reach label used when the projection row carries none. A refusal at this
/// label means "this doorway holds no reach declaration for this record" — not
/// "the record is private", and never "serve it anyway".
pub const REACH_UNDECLARED: &str = "undeclared";

/// The term of the fold a refusal names. Only `reach`/`standing` refusals are
/// minted here; contract and liveness refusals are minted by dispatch and the
/// shed path respectively, and keep their own shapes.
const TERM_REACH: &str = "reach";

/// How a declared reach behaves under the fold.
///
/// Deliberately coarser than the full ladder in
/// [`crate::cache::access_control::REACH_LEVELS`]: where a collective's
/// narrowing LANDS is the collective's to say, and a doorway that only
/// understood a fixed list of rungs would refuse a member the moment a
/// collective declared a rung this binary predates. So every rung that is not
/// explicitly commons and not explicitly beneficiary-only collapses to
/// [`ReachClass::Restricted`] — anonymous refused, standing required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachClass {
    /// `commons` / `public` — the widest rung. Anyone at all, including a
    /// visitor who has shown nothing.
    Commons,
    /// `private` / `invited` — an audience a doorway cannot establish. A
    /// doorway never invents the beneficiary of a private record, so it refuses
    /// rather than guessing.
    BeneficiaryOnly,
    /// Any other declared rung (`local`, `neighborhood`, … and anything this
    /// binary has not heard of). Refuses anonymous; admits a requester whose
    /// standing satisfies whatever audience the reach names.
    Restricted,
    /// No reach declaration at all. Refuses everyone — absence is never
    /// permission.
    Undeclared,
}

/// Classify a projected reach string.
///
/// `None` and the empty string both mean "no declaration", which is NOT the
/// same as a narrow declaration: it refuses everyone, including an
/// authenticated member, because the doorway has nothing to serve under.
#[must_use]
pub fn classify_reach(head_reach: Option<&str>) -> ReachClass {
    match head_reach.map(str::trim) {
        None | Some("") => ReachClass::Undeclared,
        Some("commons" | "public") => ReachClass::Commons,
        Some("private" | "invited") => ReachClass::BeneficiaryOnly,
        Some(_) => ReachClass::Restricted,
    }
}

/// One audience the declared reach names — a membership a requester must hold.
///
/// Read from the projection's `gateHints` entries whose relation is
/// `MembershipPrerequisite`: "a membership the person must hold before access
/// is granted". Using the existing gate-hint surface is deliberate — it is
/// already projected from the substrate and already refreshed on every router
/// reconcile, so a collective that narrows reach to its own membership needs no
/// new entry type, no new column and no new doorway persistence.
///
/// An empty audience list under a restricted reach means "any requester who can
/// show standing at all" — the rung itself is the audience.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudienceTerm {
    /// The ledger identity of the audience (the gate hint's `eprRef`).
    pub id: String,
    /// Human-readable name, when the doorway holds the atom's metadata. This is
    /// what a refusal says out loud, so a visitor who knows nothing of the
    /// protocol reads "members of the Dowell household", not a CID.
    pub label: Option<String>,
}

impl AudienceTerm {
    /// The name to SAY. Falls back to the id when no label is held.
    #[must_use]
    pub fn spoken(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.id)
    }

    /// Whether a membership the requester presented satisfies this term.
    ///
    /// Matches the ledger id exactly, and the spoken label case-insensitively.
    /// The label arm exists because the two sides of this join are still
    /// drifting: a collective row is addressed by id on one surface and by name
    /// on another. Matching both is the non-widening choice — it can only admit
    /// a requester who genuinely holds the named membership, never one who
    /// holds none.
    #[must_use]
    pub fn satisfied_by(&self, membership: &str) -> bool {
        let m = membership.trim();
        if m.is_empty() {
            return false;
        }
        m == self.id
            || self
                .label
                .as_deref()
                .is_some_and(|l| l.trim().eq_ignore_ascii_case(m))
    }
}

/// The contract terms already decided by dispatch, carried so a refusal can
/// point at the records it reads back to.
///
/// This is NOT re-deciding the contract term of the fold — a request that
/// reaches here has already matched a live projection row. It is the
/// traceability half: the reason a refusal gives must read back to a record.
#[derive(Debug, Clone)]
pub struct ContractTerms {
    /// The EPR atom being served.
    pub epr_id: String,
    /// The REA Commitment backing the projection contract.
    pub commitment_id: String,
    /// The audience the declared reach names, if any.
    pub audience: Vec<AudienceTerm>,
}

/// What the requester can show for themselves at the moment of the request.
///
/// The doorway never invents standing and never keeps its own copy to consult
/// later. `authenticated` and the identities come from the verified JWT of THIS
/// request; `memberships` are read live, and only when the fold actually needs
/// them (see [`needs_memberships`]) — a commons serve never pays for a
/// membership read.
#[derive(Debug, Clone, Default)]
pub struct RequesterStanding {
    /// Whether this request carried a credential that verified.
    pub authenticated: bool,
    /// The requester's human id, as the verified credential states it.
    pub human_id: Option<String>,
    /// The requester's agent public key, as the verified credential states it.
    pub agent_pub_key: Option<String>,
    /// Collectives the requester belongs to — ids and names, as read from
    /// their own record at the moment of the request.
    pub memberships: Vec<String>,
}

impl RequesterStanding {
    /// A visitor who has shown nothing.
    #[must_use]
    pub fn anonymous() -> Self {
        Self::default()
    }

    /// Attach the memberships read for this request.
    #[must_use]
    pub fn with_memberships(mut self, memberships: Vec<String>) -> Self {
        self.memberships = memberships;
        self
    }

    /// Whether any membership presented satisfies any term of `audience`.
    #[must_use]
    pub fn admits(&self, audience: &[AudienceTerm]) -> bool {
        audience
            .iter()
            .any(|term| self.memberships.iter().any(|m| term.satisfied_by(m)))
    }
}

/// The request being folded. Path only — everything else that matters is a
/// ledger term.
#[derive(Debug, Clone, Copy)]
pub struct ServeRequest<'a> {
    /// The path being served, for the refusal's log line.
    pub path: &'a str,
}

/// A collective the declared reach names, said the way a refusal must say it:
/// a name a person recognises, and a record anyone can read it back from.
///
/// Every field originates in the CURRENT projection row — `id` and `label`
/// from the reach declaration's own membership gate hint, `record` the route
/// that reads that collective back. None of the three could be produced by a
/// doorway-local allowlist, which is exactly why a refusal carries them: "the
/// collective whose ruling narrowed it" has to be checkable as "the name it
/// gave reads back to a record", not taken on the doorway's word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectiveRef {
    /// The collective's ledger identity (the gate hint's `eprRef`).
    pub id: String,
    /// The name a person would use, when the projection holds it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Where to read this collective back — a real route on the doorway that
    /// refused, so the chrome can follow it without knowing this protocol.
    pub record: String,
}

/// A refusal that names its term, its record and its redress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refusal {
    /// Which of the four terms failed. Always `"reach"` from this module.
    pub refused: &'static str,
    /// The reach as the projection declares it right now.
    pub reach: String,
    /// Why, in the words a friend would use.
    pub reason: String,
    /// Where the visitor can be heard about this decision.
    pub hear: &'static str,
    /// The EPR atom the refusal is about — the record the reason reads back to.
    pub epr: String,
    /// The REA Commitment backing the projection contract.
    pub contract: String,
    /// The collective the declared reach names — in the ordinary case the one
    /// whose ruling narrowed this record. Absent when the reach names no
    /// audience (an undeclared reach, a beneficiary-only record, or a rung
    /// that is its own audience): a doorway that holds no such record says
    /// nothing rather than inventing a name.
    ///
    /// When a reach names SEVERAL audiences this field carries the first — the
    /// one a chrome leads with — while [`Refusal::reason`] says all of them and
    /// `declared_in` points at the record that carries the complete list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collective: Option<CollectiveRef>,
    /// Where to read the reach declaration itself back: the REA Commitment
    /// that carries this projection's reach and its audience terms. This is
    /// the traceability the habit asks for — the named term points at the
    /// ledger record that carries it, dereferenceable in one request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_in: Option<String>,
}

impl Refusal {
    /// The `x-elohim-standing` header value for this refusal.
    ///
    /// The reach is sanitised to the header-safe alphabet before it is spliced
    /// in — the value originates in a projection row, and a row is not a place
    /// to trust bytes from.
    #[must_use]
    pub fn standing_header_value(&self) -> String {
        format!("refused;reach={}", sanitize_reach_label(&self.reach))
    }
}

/// The fold's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServeEligibility {
    /// Reach and standing both admit this requester. Serve as before.
    Serve,
    /// Refused, with the term named on its face.
    ///
    /// Boxed: a refusal carries the whole sentence a person reads plus the
    /// records it reads back to, and the serve arm carries nothing at all — so
    /// the common answer (serve) should not pay for the rare one's payload.
    Refuse(Box<Refusal>),
}

impl ServeEligibility {
    /// The refusal, when this is one.
    #[must_use]
    pub fn refusal(&self) -> Option<&Refusal> {
        match self {
            Self::Serve => None,
            Self::Refuse(r) => Some(r.as_ref()),
        }
    }
}

/// Resolve the reach + standing terms of the fold for one serve.
///
/// PURE — no I/O, no clock, no global state. Every input is either the request,
/// a ledger term read from the CURRENT projection, or the standing this
/// requester presented. That purity is the property that makes this checkable:
/// a doorway-local allowlist could not be threaded through this signature.
///
/// `head_reach` MUST be read from the live projection at serve time. Passing a
/// reach that was stored beside cached bytes reintroduces exactly the defect
/// this module exists to refuse.
#[must_use]
pub fn serve_eligibility(
    request: &ServeRequest<'_>,
    contract: &ContractTerms,
    head_reach: Option<&str>,
    requester_standing: &RequesterStanding,
) -> ServeEligibility {
    let declared = head_reach
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or(REACH_UNDECLARED)
        .to_string();

    let refuse = |reason: String| {
        ServeEligibility::Refuse(Box::new(Refusal {
            refused: TERM_REACH,
            reach: declared.clone(),
            reason,
            hear: WHERE_TO_BE_HEARD,
            epr: contract.epr_id.clone(),
            contract: contract.commitment_id.clone(),
            collective: contract.audience.first().map(collective_ref),
            declared_in: declaration_record(&contract.commitment_id),
        }))
    };

    match classify_reach(head_reach) {
        // The widest rung: anyone at all. An audience term under a commons
        // reach is not a narrowing — commons already admits its members.
        ReachClass::Commons => ServeEligibility::Serve,

        // Absence is never permission.
        ReachClass::Undeclared => refuse(format!(
            "This doorway holds no reach declaration for {}, so it will not \
             project it to anyone. Nothing here says who may read it, and a \
             doorway does not decide that for itself.",
            spoken_record(&contract.epr_id, request.path)
        )),

        // An audience a doorway cannot establish. It refuses rather than
        // guessing — including for an authenticated requester, because the
        // doorway has no way to know whether THIS person is the beneficiary.
        ReachClass::BeneficiaryOnly => refuse(format!(
            "{} is held at {declared} reach — for the person it belongs to, \
             read from their own record. A doorway is not the place that \
             decides who that is, so it will not answer for it here.",
            spoken_record(&contract.epr_id, request.path)
        )),

        ReachClass::Restricted => {
            if !requester_standing.authenticated {
                // The narrowing NAMES someone. A stranger told only "not for
                // you" learns nothing and can do nothing; told who it is now
                // for, they can recognise themselves in it — or know whose
                // decision it was they want to be heard about. The name is
                // read from the declaration's own audience term, never
                // supplied by this doorway, so it is absent (and the sentence
                // falls back) when the record names nobody.
                return refuse(match spoken_audience_opt(&contract.audience) {
                    Some(audience) => format!(
                        "{} is no longer at commons reach. Its stewards narrowed \
                         it to {declared} reach, which admits {audience} and not \
                         a visitor who has shown nothing. If you are one of the \
                         people it now names, sign in and ask again.",
                        spoken_record(&contract.epr_id, request.path)
                    ),
                    None => format!(
                        "{} is no longer at commons reach. Its stewards narrowed \
                         it to {declared}, which does not admit a visitor who has \
                         shown nothing. If you are one of the people it now \
                         names, sign in and ask again.",
                        spoken_record(&contract.epr_id, request.path)
                    ),
                });
            }
            if contract.audience.is_empty() {
                // The rung itself is the audience and standing was shown.
                return ServeEligibility::Serve;
            }
            if requester_standing.admits(&contract.audience) {
                return ServeEligibility::Serve;
            }
            refuse(format!(
                "{} is held at {declared} reach, which admits {}. The standing \
                 you showed names no such membership, so this doorway will not \
                 project it to you.",
                spoken_record(&contract.epr_id, request.path),
                spoken_audience(&contract.audience)
            ))
        }
    }
}

/// Name the record the way a person would — its path if we have one worth
/// saying, else the atom's own address.
fn spoken_record(epr_id: &str, path: &str) -> String {
    if path.is_empty() || path == "/" {
        format!("\"{epr_id}\"")
    } else {
        format!("\"{path}\"")
    }
}

/// The dereferenceable record for one audience term.
fn collective_ref(term: &AudienceTerm) -> CollectiveRef {
    CollectiveRef {
        id: term.id.clone(),
        label: term.label.clone(),
        record: format!(
            "{COLLECTIVE_RECORD_ROUTE}{}",
            urlencoding::encode(term.id.trim())
        ),
    }
}

/// The dereferenceable record for the reach declaration itself. `None` when
/// the projection row carries no commitment id — a refusal never invents a
/// record to point at.
fn declaration_record(commitment_id: &str) -> Option<String> {
    let id = commitment_id.trim();
    if id.is_empty() {
        return None;
    }
    Some(format!(
        "{REACH_DECLARATION_ROUTE}{}",
        urlencoding::encode(id)
    ))
}

/// [`spoken_audience`] when the reach names an audience at all, `None` when it
/// names none — so a sentence can be built that does not pretend otherwise.
fn spoken_audience_opt(audience: &[AudienceTerm]) -> Option<String> {
    if audience.is_empty() {
        return None;
    }
    Some(spoken_audience(audience))
}

/// Say an audience list in a sentence: `a`, `a and b`, `a, b and c`.
fn spoken_audience(audience: &[AudienceTerm]) -> String {
    let names: Vec<&str> = audience.iter().map(AudienceTerm::spoken).collect();
    match names.split_last() {
        None => "a named audience".to_string(),
        Some((last, [])) => format!("members of {last}"),
        Some((last, head)) => format!("members of {} and {}", head.join(", "), last),
    }
}

/// Reduce a reach label to the header-safe alphabet. Anything outside
/// `[A-Za-z0-9_-]` collapses the whole label to [`REACH_UNDECLARED`] rather
/// than being silently stripped — a mangled label in a header is worse than an
/// honest "we could not say".
#[must_use]
pub fn sanitize_reach_label(reach: &str) -> String {
    let trimmed = reach.trim();
    if trimmed.is_empty()
        || trimmed.len() > 64
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return REACH_UNDECLARED.to_string();
    }
    trimmed.to_string()
}

/// Whether the fold will need the requester's memberships to answer.
///
/// The membership read is the only I/O the eligibility path can provoke, so it
/// is gated on the fold actually depending on it: a commons serve, an
/// undeclared refusal, a beneficiary-only refusal and an anonymous refusal all
/// answer without ever asking. In practice that means the overwhelming majority
/// of serves — every commons asset on every page — pay nothing.
#[must_use]
pub fn needs_memberships(
    head_reach: Option<&str>,
    audience: &[AudienceTerm],
    standing: &RequesterStanding,
) -> bool {
    classify_reach(head_reach) == ReachClass::Restricted
        && standing.authenticated
        && !audience.is_empty()
}

/// Render a refusal as the wire contract: `403`, the standing header, and a
/// small JSON body the chrome can read without knowing this protocol.
#[must_use]
pub fn refusal_response(refusal: &Refusal) -> Response<Full<Bytes>> {
    let body = serde_json::to_vec(refusal).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("content-type", "application/json")
        .header(STANDING_HEADER, refusal.standing_header_value())
        // A refusal is this requester's, at this moment, under this reach —
        // never a shared-cacheable answer.
        .header("cache-control", "no-store")
        .body(Full::new(Bytes::from(body)))
        .expect("infallible refusal response")
}

/// The `x-elohim-standing` value for a serve the fold ADMITTED.
///
/// The counterpart of [`Refusal::standing_header_value`], and deliberately the
/// same sentence shape: `admitted;reach=<reach>`. A refusal that names its
/// term while a serve says nothing leaves the chrome unable to tell a person
/// what governs the place they just walked into — it could only infer "it must
/// have been fine" from a 200, which is exactly the reading a cache with a
/// hostname would also produce. Saying the reach out loud on the way IN is what
/// makes the fold visible on both of its answers.
///
/// The reach is sanitised before it is spliced in, for the same reason the
/// refusal's is: the value originates in a projection row.
#[must_use]
pub fn admitted_standing_value(head_reach: &str) -> String {
    format!("admitted;reach={}", sanitize_reach_label(head_reach))
}

/// Stamp [`admitted_standing_value`] on a response the fold admitted.
///
/// Two deliberate restraints:
/// - Only on a response that actually SERVED something (2xx/3xx). A shed, a
///   502 or a 501 is the byte path's own answer about itself, not a statement
///   about this requester's standing, and labelling one "admitted" would be
///   the doorway saying something it did not resolve.
/// - Never overwrites a header already present. A relayed answer carries the
///   HOLDER's standing verbatim ([`crate::services::name_routing`]), and the
///   courier does not restate it in its own voice.
pub fn stamp_admitted_standing<B>(response: &mut hyper::Response<B>, head_reach: &str) {
    let status = response.status();
    if !(status.is_success() || status.is_redirection()) {
        return;
    }
    if response.headers().contains_key(STANDING_HEADER) {
        return;
    }
    if let Ok(value) = hyper::header::HeaderValue::from_str(&admitted_standing_value(head_reach)) {
        response.headers_mut().insert(STANDING_HEADER, value);
    }
}

// ════════════════════════════════════════════════════════════════════════════
// The serve-time wiring
//
// Everything above this line is pure. Below it are the three thin readers that
// turn a live request and a live projection row into the pure function's
// arguments — and nothing else. In particular there is NO store here: no
// memo, no TTL, no "this one was allowed last time". The fold is resolved from
// scratch on every serve, which is the whole invariant.
// ════════════════════════════════════════════════════════════════════════════

use crate::server::AppState;
use elohim_views::projection::{EprProjectionView, GateHintRelation};
use hyper::Request;

/// How long the membership read may take before the fold gives up on it.
///
/// Short on purpose: this sits on the browser hot path for the narrow-reach
/// case. A read that does not answer in time yields NO memberships, which
/// refuses — never serves. A slow substrate must not become an open door.
const MEMBERSHIP_READ_TIMEOUT_SECS: u64 = 3;

/// The audience the declared reach names, read from the CURRENT projection row.
///
/// Source: the projection's `gateHints` entries whose relation is
/// `MembershipPrerequisite`. Every other hint relation (a person who can grant,
/// a payment to offer, …) describes how to SATISFY a gate rather than who the
/// gate admits, so it is not an audience term and is deliberately ignored here.
#[must_use]
pub fn audience_from_projection(projection: &EprProjectionView) -> Vec<AudienceTerm> {
    projection
        .gate_hints
        .iter()
        .filter(|hint| hint.relation == GateHintRelation::MembershipPrerequisite)
        .map(|hint| AudienceTerm {
            id: hint.epr_ref.clone(),
            label: hint.label.clone(),
        })
        .collect()
}

/// The contract terms for a projection — what a refusal points back at.
#[must_use]
pub fn contract_from_projection(projection: &EprProjectionView) -> ContractTerms {
    ContractTerms {
        epr_id: projection.epr_id.clone(),
        commitment_id: projection.commitment_id.clone(),
        audience: audience_from_projection(projection),
    }
}

/// What this request can show for itself, read from the verified credential of
/// THIS request. Never a doorway-held copy of who someone is.
///
/// Memberships are NOT read here — see [`fold_for_projection`], which asks only
/// when the fold depends on the answer.
#[must_use]
pub fn standing_from_request<B>(state: &AppState, req: &Request<B>) -> RequesterStanding {
    match crate::server::http::resolve_verified_claims_from_request(state, req) {
        Some(claims) => RequesterStanding {
            authenticated: true,
            human_id: Some(claims.human_id),
            agent_pub_key: if claims.agent_pub_key.is_empty() {
                None
            } else {
                Some(claims.agent_pub_key)
            },
            memberships: Vec::new(),
        },
        None => RequesterStanding::anonymous(),
    }
}

/// Read the collectives a human currently belongs to, at the moment of the
/// request, from the substrate projection — never from a doorway-local list.
///
/// `GET {storage}/db/participations/{human_id}` is the canonical read (storage
/// `handle_participations_by_human`). A participation with a `departedAt` is
/// past membership and is dropped. Both the collective id and, when present,
/// its name are returned, because the id/name seam between the collective row
/// and a gate hint is still drifting (see [`AudienceTerm::satisfied_by`]).
///
/// Any failure — no storage configured, a non-success status, a timeout, an
/// unparseable body — yields an EMPTY list, which REFUSES. A membership read
/// that cannot be completed is never read as permission.
pub async fn read_memberships(state: &AppState, human_id: &str) -> Vec<String> {
    let Some(storage_url) = state.args.storage_url.as_deref() else {
        tracing::warn!(
            human_id = %human_id,
            "serve eligibility: no storage URL configured — the membership read \
             cannot be made, so no standing is presented"
        );
        return Vec::new();
    };
    let url = format!(
        "{}/db/participations/{}",
        storage_url.trim_end_matches('/'),
        urlencoding::encode(human_id)
    );
    let resp = match state
        .storage_proxy_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(MEMBERSHIP_READ_TIMEOUT_SECS))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            tracing::warn!(human_id = %human_id, status = %r.status(),
                "serve eligibility: membership read non-success — presenting no standing");
            return Vec::new();
        }
        Err(e) => {
            tracing::warn!(human_id = %human_id, error = %e,
                "serve eligibility: membership read failed — presenting no standing");
            return Vec::new();
        }
    };
    let Ok(body) = resp.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    let items = body
        .get("items")
        .and_then(|i| i.as_array())
        .or_else(|| body.as_array());
    let Some(items) = items else {
        return Vec::new();
    };
    let mut memberships = Vec::new();
    for item in items {
        // Past membership is not membership.
        if item.get("departedAt").is_some_and(|d| !d.is_null()) {
            continue;
        }
        for key in ["collectiveId", "collectiveName", "collective_id"] {
            if let Some(v) = item.get(key).and_then(|v| v.as_str()) {
                if !v.is_empty() && !memberships.iter().any(|m| m == v) {
                    memberships.push(v.to_string());
                }
            }
        }
    }
    memberships
}

/// Fold reach + standing for one serve of one projection — the single entry
/// point every cached-serve path calls.
///
/// `projection` MUST be the row the router holds RIGHT NOW (from
/// `EprRouter::dispatch` / `projection_for_epr_id`), never a copy captured
/// alongside cached bytes. The router table is rebuilt wholesale on every
/// reconcile, so reading it here is what makes a collective's ruling reach this
/// doorway with no restart and no eviction.
///
/// On a refusal this also logs a WARN and counts
/// `doorway_serve_refused_total{reach}`.
pub async fn fold_for_projection(
    state: &AppState,
    path: &str,
    projection: &EprProjectionView,
    standing: RequesterStanding,
) -> ServeEligibility {
    let contract = contract_from_projection(projection);
    let head_reach = Some(projection.reach.as_str());

    // The ONE read the eligibility path can provoke, and only when the answer
    // changes the verdict.
    let standing = if needs_memberships(head_reach, &contract.audience, &standing) {
        let human_id = standing.human_id.clone().unwrap_or_default();
        let memberships = read_memberships(state, &human_id).await;
        standing.with_memberships(memberships)
    } else {
        standing
    };

    let verdict = serve_eligibility(&ServeRequest { path }, &contract, head_reach, &standing);

    if let Some(refusal) = verdict.refusal() {
        crate::metrics::inc_serve_refused(&sanitize_reach_label(&refusal.reach));
        tracing::warn!(
            path = %path,
            epr_id = %projection.epr_id,
            reach = %refusal.reach,
            authenticated = standing.authenticated,
            audience = contract.audience.len(),
            "serve refused by the eligibility fold — reach no longer admits this requester"
        );
    }
    verdict
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(audience: Vec<AudienceTerm>) -> ContractTerms {
        ContractTerms {
            epr_id: "community-garden-club".into(),
            commitment_id: "commitment-abc".into(),
            audience,
        }
    }

    fn household() -> AudienceTerm {
        AudienceTerm {
            id: "collective-dowell-household".into(),
            label: Some("the Dowell household".into()),
        }
    }

    fn member(memberships: &[&str]) -> RequesterStanding {
        RequesterStanding {
            authenticated: true,
            human_id: Some("human-matthew".into()),
            agent_pub_key: Some("uhCAkMatthew".into()),
            memberships: memberships.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    fn fold(
        head_reach: Option<&str>,
        contract: &ContractTerms,
        standing: &RequesterStanding,
    ) -> ServeEligibility {
        serve_eligibility(
            &ServeRequest {
                path: "/community-garden-club",
            },
            contract,
            head_reach,
            standing,
        )
    }

    // ── reach classification ────────────────────────────────────────────────

    #[test]
    fn commons_and_public_are_the_widest_rung() {
        assert_eq!(classify_reach(Some("commons")), ReachClass::Commons);
        assert_eq!(classify_reach(Some("public")), ReachClass::Commons);
        assert_eq!(classify_reach(Some(" commons ")), ReachClass::Commons);
    }

    #[test]
    fn absence_is_never_permission() {
        assert_eq!(classify_reach(None), ReachClass::Undeclared);
        assert_eq!(classify_reach(Some("")), ReachClass::Undeclared);
        assert_eq!(classify_reach(Some("   ")), ReachClass::Undeclared);
    }

    /// Where a narrowing LANDS is the collective's to say. A rung this binary
    /// has never heard of must behave like every other narrow rung — refuse
    /// anonymous, admit standing — not like commons and not like a hard deny.
    #[test]
    fn an_unknown_rung_is_restricted_not_commons_and_not_denied() {
        assert_eq!(
            classify_reach(Some("garden-club-only")),
            ReachClass::Restricted
        );
        assert_eq!(classify_reach(Some("local")), ReachClass::Restricted);
        assert_eq!(classify_reach(Some("bioregional")), ReachClass::Restricted);
    }

    // ── the fold ────────────────────────────────────────────────────────────

    #[test]
    fn commons_serves_a_visitor_who_has_shown_nothing() {
        let verdict = fold(
            Some("commons"),
            &contract(vec![]),
            &RequesterStanding::anonymous(),
        );
        assert_eq!(verdict, ServeEligibility::Serve);
    }

    /// SCENARIO 1's assertion, at the unit. The ruling narrows reach; the next
    /// fold refuses an anonymous visitor and NAMES reach as the term.
    #[test]
    fn a_narrowed_reach_refuses_an_anonymous_visitor_naming_reach() {
        let verdict = fold(
            Some("local"),
            &contract(vec![household()]),
            &RequesterStanding::anonymous(),
        );
        let refusal = verdict.refusal().expect("anonymous must be refused");
        assert_eq!(refusal.refused, "reach", "the term that failed is named");
        assert_eq!(refusal.reach, "local", "the declared reach is named");
        assert_eq!(
            refusal.hear, WHERE_TO_BE_HEARD,
            "a way to be heard is offered"
        );
        assert_eq!(
            refusal.epr, "community-garden-club",
            "the reason reads back to the EPR record"
        );
        assert_eq!(
            refusal.contract, "commitment-abc",
            "the reason reads back to the contract record"
        );
        assert!(
            !refusal.reason.is_empty() && !refusal.reason.contains("commitment-abc"),
            "the reason speaks to a person, not in record ids: {}",
            refusal.reason
        );
        // A stranger told only "not for you" learns nothing. The narrowing
        // NAMES someone, and the refusal says who — by the name the
        // declaration carries, in the sentence a person reads.
        assert!(
            refusal.reason.contains("the Dowell household"),
            "an anonymous refusal must name the collective the reach now admits: {}",
            refusal.reason
        );
    }

    /// The traceability half of the same refusal: the collective it named, and
    /// the reach declaration it read, each read back from a record. A
    /// doorway-local list could produce neither.
    #[test]
    fn an_anonymous_refusal_points_at_the_collective_and_the_declaration() {
        let refusal = fold(
            Some("local"),
            &contract(vec![household()]),
            &RequesterStanding::anonymous(),
        )
        .refusal()
        .cloned()
        .expect("anonymous must be refused");

        let collective = refusal
            .collective
            .as_ref()
            .expect("a reach that names an audience must name it as a record");
        assert_eq!(collective.id, "collective-dowell-household");
        assert_eq!(collective.label.as_deref(), Some("the Dowell household"));
        assert_eq!(
            collective.record, "/db/collectives/collective-dowell-household",
            "the named collective is dereferenceable on the doorway that refused"
        );
        assert_eq!(
            refusal.declared_in.as_deref(),
            Some("/api/v1/commitments/commitment-abc"),
            "the refusal points at the reach declaration it actually read"
        );
    }

    /// …and says nothing it holds no record for. A reach that names no
    /// audience yields no collective — the doorway does not invent one to fill
    /// the field, and the sentence falls back to the one that claims less.
    #[test]
    fn a_reach_that_names_nobody_yields_no_collective() {
        let refusal = fold(
            Some("local"),
            &contract(vec![]),
            &RequesterStanding::anonymous(),
        )
        .refusal()
        .cloned()
        .expect("anonymous must be refused");
        assert!(refusal.collective.is_none());
        assert!(
            refusal
                .reason
                .contains("does not admit a visitor who has shown nothing"),
            "the fallback sentence still names reach and still offers the way back in: {}",
            refusal.reason
        );
        // The declaration is still traceable — that half never depends on an
        // audience being declared.
        assert_eq!(
            refusal.declared_in.as_deref(),
            Some("/api/v1/commitments/commitment-abc")
        );
    }

    /// A record id is not a URL path component by construction, so the routes a
    /// refusal points at are encoded rather than spliced.
    #[test]
    fn record_routes_encode_the_ids_they_carry() {
        let odd = AudienceTerm {
            id: "collective/with space".into(),
            label: None,
        };
        assert_eq!(
            collective_ref(&odd).record,
            "/db/collectives/collective%2Fwith%20space"
        );
        assert_eq!(
            declaration_record("   "),
            None,
            "no id, no record to point at"
        );
    }

    /// The fold's answer is stated on BOTH sides: a serve says which reach
    /// admitted it, in the same sentence shape a refusal uses.
    #[test]
    fn an_admitted_serve_states_the_reach_that_admitted_it() {
        assert_eq!(
            admitted_standing_value("household"),
            "admitted;reach=household"
        );
        assert_eq!(
            admitted_standing_value("local\r\nx-injected: 1"),
            "admitted;reach=undeclared",
            "a projection row is not a place to trust bytes from, on this side either"
        );

        let mut served = Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from_static(b"<html>")))
            .unwrap();
        stamp_admitted_standing(&mut served, "household");
        assert_eq!(
            served
                .headers()
                .get(STANDING_HEADER)
                .and_then(|v| v.to_str().ok()),
            Some("admitted;reach=household")
        );

        // A relayed answer already carries the HOLDER's standing: the courier
        // does not restate it in its own voice.
        let mut relayed = Response::builder()
            .status(StatusCode::OK)
            .header(STANDING_HEADER, "admitted;reach=household")
            .body(Full::new(Bytes::from_static(b"<html>")))
            .unwrap();
        stamp_admitted_standing(&mut relayed, "commons");
        assert_eq!(
            relayed
                .headers()
                .get(STANDING_HEADER)
                .and_then(|v| v.to_str().ok()),
            Some("admitted;reach=household"),
            "the holder's own statement survives the hop"
        );

        // A shed is the byte path's answer about itself, not a statement about
        // this requester's standing.
        let mut shed = Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(Full::new(Bytes::from_static(b"{}")))
            .unwrap();
        stamp_admitted_standing(&mut shed, "household");
        assert!(
            shed.headers().get(STANDING_HEADER).is_none(),
            "a doorway does not label its own shed 'admitted'"
        );
    }

    /// SCENARIO 2's pair, at the unit: Matthew and James differ in EXACTLY one
    /// term of the fold, and the fold discriminates rather than having reopened.
    #[test]
    fn a_member_is_served_and_a_non_member_is_refused_under_the_same_reach() {
        let c = contract(vec![household()]);

        let matthew = member(&["collective-dowell-household"]);
        assert_eq!(
            fold(Some("local"), &c, &matthew),
            ServeEligibility::Serve,
            "the standing the reach names must still be served"
        );

        let james = member(&["collective-some-other-group"]);
        let refusal = fold(Some("local"), &c, &james)
            .refusal()
            .cloned()
            .expect("a requester the reach does not admit is refused");
        assert_eq!(refusal.refused, "reach");
        assert_eq!(refusal.reach, "local");
        assert!(
            refusal.reason.contains("the Dowell household"),
            "the refusal says the audience out loud: {}",
            refusal.reason
        );
    }

    #[test]
    fn a_membership_matches_by_ledger_id_or_by_spoken_name() {
        let c = contract(vec![household()]);
        assert_eq!(
            fold(Some("local"), &c, &member(&["collective-dowell-household"])),
            ServeEligibility::Serve
        );
        assert_eq!(
            fold(Some("local"), &c, &member(&["The Dowell Household"])),
            ServeEligibility::Serve,
            "the id/name seam is still drifting; both sides must admit a real member"
        );
        assert!(
            fold(Some("local"), &c, &member(&[""])).refusal().is_some(),
            "an empty membership string admits nothing"
        );
    }

    /// A narrow rung that names no audience is the rung itself: standing shown
    /// is enough. This is what keeps "narrowed to a reach that admits the
    /// household" working before any collective binding is projected.
    #[test]
    fn a_restricted_rung_with_no_audience_admits_any_shown_standing() {
        assert_eq!(
            fold(Some("neighborhood"), &contract(vec![]), &member(&[])),
            ServeEligibility::Serve
        );
        assert!(
            fold(
                Some("neighborhood"),
                &contract(vec![]),
                &RequesterStanding::anonymous()
            )
            .refusal()
            .is_some(),
            "but it still refuses a visitor who has shown nothing"
        );
    }

    /// NEVER WIDEN. An undeclared reach refuses everyone — the authenticated
    /// requester included. This is the behaviour the `anon_reach_readable`
    /// gate had, preserved exactly.
    #[test]
    fn an_undeclared_reach_refuses_everyone() {
        for standing in [RequesterStanding::anonymous(), member(&["anything"])] {
            let refusal = fold(None, &contract(vec![]), &standing)
                .refusal()
                .cloned()
                .expect("absence of reach information is never permission");
            assert_eq!(refusal.reach, REACH_UNDECLARED);
        }
    }

    #[test]
    fn private_and_invited_are_refused_at_the_doorway_even_with_standing() {
        for reach in ["private", "invited"] {
            let refusal = fold(Some(reach), &contract(vec![]), &member(&["anything"]))
                .refusal()
                .cloned()
                .unwrap_or_else(|| panic!("{reach} must not be projected by a doorway"));
            assert_eq!(refusal.reach, reach);
        }
    }

    // ── the membership read is gated on the fold needing it ─────────────────

    #[test]
    fn memberships_are_read_only_when_the_fold_depends_on_them() {
        let audience = vec![household()];
        // commons: never.
        assert!(!needs_memberships(Some("commons"), &audience, &member(&[])));
        // anonymous under a narrow reach: refused without asking anyone.
        assert!(!needs_memberships(
            Some("local"),
            &audience,
            &RequesterStanding::anonymous()
        ));
        // narrow reach, no audience named: the rung is the audience.
        assert!(!needs_memberships(Some("local"), &[], &member(&[])));
        // undeclared / beneficiary-only: refused without asking anyone.
        assert!(!needs_memberships(None, &audience, &member(&[])));
        assert!(!needs_memberships(Some("private"), &audience, &member(&[])));
        // the one case that needs the read.
        assert!(needs_memberships(Some("local"), &audience, &member(&[])));
    }

    // ── the wire contract ───────────────────────────────────────────────────

    #[test]
    fn a_refusal_answers_403_with_the_standing_header_and_the_named_body() {
        let refusal = fold(
            Some("local"),
            &contract(vec![household()]),
            &RequesterStanding::anonymous(),
        )
        .refusal()
        .cloned()
        .unwrap();

        let response = refusal_response(&refusal);
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            response
                .headers()
                .get(STANDING_HEADER)
                .and_then(|v| v.to_str().ok()),
            Some("refused;reach=local")
        );
        assert_eq!(
            response
                .headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok()),
            Some("no-store"),
            "a refusal is this requester's, never shared-cacheable"
        );

        let bytes = serde_json::to_vec(&refusal).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["refused"], "reach");
        assert_eq!(json["reach"], "local");
        assert_eq!(json["hear"], WHERE_TO_BE_HEARD);
        assert!(json["reason"].as_str().is_some_and(|r| !r.is_empty()));
        assert_eq!(json["epr"], "community-garden-club");
        assert_eq!(json["contract"], "commitment-abc");
        // The wire shape a chrome reads: who the reach now names, and where
        // both that collective and the declaration itself read back.
        assert_eq!(json["collective"]["id"], "collective-dowell-household");
        assert_eq!(json["collective"]["label"], "the Dowell household");
        assert_eq!(
            json["collective"]["record"],
            "/db/collectives/collective-dowell-household"
        );
        assert_eq!(json["declaredIn"], "/api/v1/commitments/commitment-abc");
    }

    /// A projection row is not a place to trust bytes from: a reach carrying
    /// CRLF or a header separator must never reach a response header.
    #[test]
    fn a_junk_reach_label_never_reaches_the_header() {
        for junk in [
            "local\r\nx-injected: 1",
            "local;drop",
            "",
            "   ",
            "a-very-long-label-that-should-never-have-been-declared-as-a-reach-rung",
        ] {
            assert_eq!(
                sanitize_reach_label(junk),
                REACH_UNDECLARED,
                "junk: {junk:?}"
            );
        }
        assert_eq!(sanitize_reach_label("commons"), "commons");
        assert_eq!(sanitize_reach_label("garden_club-only"), "garden_club-only");

        let refusal = Refusal {
            refused: TERM_REACH,
            reach: "local\r\nx-injected: 1".into(),
            reason: "…".into(),
            hear: WHERE_TO_BE_HEARD,
            epr: "e".into(),
            contract: "c".into(),
            collective: None,
            declared_in: None,
        };
        let response = refusal_response(&refusal);
        assert_eq!(
            response
                .headers()
                .get(STANDING_HEADER)
                .and_then(|v| v.to_str().ok()),
            Some("refused;reach=undeclared")
        );
    }

    // ── the property the habit exists for ───────────────────────────────────

    // ── the projection readers ──────────────────────────────────────────────

    fn projection(
        reach: &str,
        hints: Vec<elohim_views::projection::GateHintRef>,
    ) -> EprProjectionView {
        use elohim_views::projection::ProjectionMode;
        EprProjectionView {
            commitment_id: "commitment-abc".into(),
            epr_id: "community-garden-club".into(),
            doorway_id: "doorway:beta".into(),
            url_path: "/community-garden-club".into(),
            hostnames: vec![],
            channel: elohim_views::projection::Channel::Converged,
            mode: ProjectionMode::Cached,
            reach: reach.into(),
            base_href: "/community-garden-club/".into(),
            entry_file: "index.html".into(),
            spa_fallback: true,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
            preview_epr_ref: None,
            gate_hints: hints,
            dead_end: false,
            steward_direct_endpoint: None,
            seeded_at: "2026-09-12T00:00:00Z".into(),
            seeded_by: "peer-alpha".into(),
        }
    }

    fn hint(
        epr_ref: &str,
        label: Option<&str>,
        relation: GateHintRelation,
    ) -> elohim_views::projection::GateHintRef {
        elohim_views::projection::GateHintRef {
            epr_ref: epr_ref.into(),
            label: label.map(str::to_string),
            relation,
        }
    }

    /// Only a membership prerequisite is an AUDIENCE. Every other hint relation
    /// describes how to satisfy a gate, not who the gate admits — reading them
    /// as audience terms would let "a payment to offer" silently become a
    /// membership nobody holds, refusing the people the reach does name.
    #[test]
    fn only_membership_prerequisites_are_audience_terms() {
        let p = projection(
            "local",
            vec![
                hint(
                    "collective-dowell-household",
                    Some("the Dowell household"),
                    GateHintRelation::MembershipPrerequisite,
                ),
                hint(
                    "epr-pay",
                    Some("a small fee"),
                    GateHintRelation::PaymentToOffer,
                ),
                hint("epr-ask", None, GateHintRelation::PersonWhoCanGrant),
            ],
        );
        let audience = audience_from_projection(&p);
        assert_eq!(audience.len(), 1);
        assert_eq!(audience[0].id, "collective-dowell-household");
        assert_eq!(audience[0].spoken(), "the Dowell household");
    }

    #[test]
    fn contract_terms_carry_the_records_a_refusal_points_at() {
        let c = contract_from_projection(&projection("commons", vec![]));
        assert_eq!(c.epr_id, "community-garden-club");
        assert_eq!(c.commitment_id, "commitment-abc");
        assert!(c.audience.is_empty());
    }

    /// The end-to-end shape the serve paths get: the SAME projection object,
    /// mutated only in its `reach`, folds differently. Nothing else changed —
    /// no eviction, no new request, no restart.
    #[test]
    fn a_projection_whose_reach_narrows_folds_differently_with_nothing_else_changed() {
        let mut p = projection("commons", vec![]);
        let visitor = RequesterStanding::anonymous();
        let request = ServeRequest {
            path: "/community-garden-club",
        };

        let before = serve_eligibility(
            &request,
            &contract_from_projection(&p),
            Some(p.reach.as_str()),
            &visitor,
        );
        assert_eq!(before, ServeEligibility::Serve);

        // The collective rules. The router's next reconcile replaces the row.
        p.reach = "local".into();

        let after = serve_eligibility(
            &request,
            &contract_from_projection(&p),
            Some(p.reach.as_str()),
            &visitor,
        );
        let refusal = after.refusal().expect("the narrowing must be felt");
        assert_eq!(refusal.refused, "reach");
        assert_eq!(refusal.reach, "local");
    }

    /// THE INVARIANT. Nothing is remembered between folds: the SAME contract
    /// and the SAME requester fold differently the moment the projected reach
    /// changes underneath them. The bytes a caller holds are not an input here,
    /// which is what makes "cache is bytes, never permission" structural rather
    /// than a promise.
    #[test]
    fn the_same_requester_folds_differently_when_the_projected_reach_narrows() {
        let c = contract(vec![household()]);
        let visitor = RequesterStanding::anonymous();

        assert_eq!(
            fold(Some("commons"), &c, &visitor),
            ServeEligibility::Serve,
            "before the ruling"
        );
        assert!(
            fold(Some("local"), &c, &visitor).refusal().is_some(),
            "after the ruling — same visitor, same contract, no eviction, no restart"
        );
    }
}
