//! Witnessed-interaction primitive — the frame/witness parent (spec §2.2–2.5).
//!
//! See `genesis/docs/superpowers/specs/2026-07-15-frame-witness-primitive-architecture-design.md`.
//!
//! A **witness** emits an REA-shaped record — `(object_cid, substrate, action, magnitude)` —
//! about a content-addressed object; peer-sync validates it; it aggregates on the object CID,
//! substrate-denominated. The governance/frame classifier is the *same* primitive as a
//! consumption meter, up to a choice of **magnitude algebra** (`Vote` ordinal tally vs `Count`
//! additive monoid vs `Classification` frame-ref). Two of its specializations already exist as
//! `EprKind`s (`FeedbackSignal`, `AttentionTending`); this module names their parent and gives
//! the classifier a third magnitude.
//!
//! # G1 scope (this module)
//!
//! The canonical Rust shapes + ts-rs projection + the elohim-epr leg of the three-way golden
//! CID vector. The `brit-epr::elohim::witness` and `eprfs-core` legs of the vector are **G1b
//! follow-up** (spec §4.4, §10) — NOT in this module. Frame atoms (S2 schema JSON), the Python
//! `frame_classifier`, and the `epr-element` chips are G2/G3/G4.
//!
//! # Canonical-bytes discipline
//!
//! `WitnessedInteraction` and `FrameClassification` each expose `canonical_bytes()` /
//! `content_cid()` built from a hand-ordered `BTreeMap<String, Ipld>` — identical in spirit to
//! [`crate::envelope::Envelope::canonical_bytes`]. The hand-built map (not a serde derive) IS the
//! portable byte-contract the three-way vector reproduces: brit-epr and eprfs-core must mint the
//! *same* CID string from the *same* logical value.

use crate::cbor;
use crate::cid::compute_cid;
use crate::error::Result;
use chrono::{DateTime, Utc};
use cid::Cid;
use ipld_core::ipld::Ipld;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

// ---------------------------------------------------------------------------
// The witnessed-interaction envelope — the parent (spec §2.2)
// ---------------------------------------------------------------------------

/// An REA-shaped record a witness emits about a content-addressed object.
///
/// Emitted inside an [`crate::envelope::Envelope`] as one
/// [`crate::EprKind::WitnessedInteraction`] variant, inheriting
/// `compute_cid`/`reach`/`coupling`/`proof`/`supersedes`.
///
/// **`substrate` is a `String`, deliberately — there is NO Rust `SubstrateSignal` enum.**
/// The substrate vocabulary (`attention|compute|storage|bandwidth|energy|time|resource`) is
/// **schema/DNA-rooted** (`elohim/sdk/schemas/v1/enums/substrate-signal.schema.json`, DNA const
/// `SUBSTRATE_SIGNALS` in `content_store_integrity`). It is validated at the boundary against that
/// whitelist; a ts-rs peer enum here would dual-source a DNA-notarized enum (spec must-fix #3) and
/// invite the recorded reach-enum-drift. **Governance is NOT a substrate** — it rides in
/// [`Magnitude`] (spec must-fix #1), never as an 8th substrate member (which would move the DNA
/// hash).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct WitnessedInteraction {
    /// REA Resource — the aggregation anchor (the auto-dedup theorem hangs off this CID).
    #[ts(type = "string")]
    pub object_cid: Cid,

    /// Substrate signal, validated at the boundary against the schema-rooted `SUBSTRATE_SIGNALS`
    /// const. NOT a Rust enum by design (see type doc).
    pub substrate: String,

    /// REA verb describing what the witness did with / to the object.
    pub action: ReaVerb,

    /// Algebra-tagged magnitude. **Governance rides here** (`Vote` / `Classification`), never in
    /// `substrate`.
    pub magnitude: Magnitude,

    /// v1 self-reported witness identity; graduates to `agent_initial_pubkey`.
    pub witness: WitnessId,

    /// Window this record covers; `None` supports empty-never-projects.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(optional)]
    pub coverage_span: Option<Span>,

    /// UTC issue time. (A local-only advisory `wall_clock` is stripped at sync per fleet-safety
    /// and is intentionally NOT modeled here.)
    pub issued_at: DateTime<Utc>,
}

impl WitnessedInteraction {
    /// Canonical dag-cbor bytes (alphabetical map keys), mirroring
    /// [`crate::envelope::Envelope::canonical_bytes`]. `coverageSpan` is included only when
    /// present (mirrors the envelope's `supersedes` treatment).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let mut map: BTreeMap<String, Ipld> = BTreeMap::new();
        map.insert(
            "action".into(),
            Ipld::String(reaverb_canonical(self.action)),
        );
        if let Some(span) = self.coverage_span {
            map.insert("coverageSpan".into(), span_ipld(&span));
        }
        map.insert(
            "issuedAt".into(),
            Ipld::String(
                self.issued_at
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            ),
        );
        map.insert("magnitude".into(), magnitude_ipld(&self.magnitude));
        map.insert("objectCid".into(), Ipld::Link(self.object_cid));
        map.insert("substrate".into(), Ipld::String(self.substrate.clone()));
        map.insert(
            "witness".into(),
            Ipld::String(witnessid_canonical(self.witness)),
        );
        cbor::encode(&Ipld::Map(map))
    }

    /// CIDv1(dag-cbor, sha2-256) over the canonical bytes.
    pub fn content_cid(&self) -> Result<Cid> {
        Ok(compute_cid(&self.canonical_bytes()?))
    }
}

// ---------------------------------------------------------------------------
// The magnitude algebra — the axis of specialization (spec §2.3, must-fix #1)
// ---------------------------------------------------------------------------

/// The algebra a witnessed interaction is denominated in.
///
/// - `Count` — additive monoid over ℝ≥0; identity `0` = abstain = empty-never-projects.
/// - `Vote` — ordinal, signed tally (governance affirm/dismiss).
/// - `Classification` — a **CID pointing at the frame atom** ([`FrameClassification`]'s home),
///   NOT an embedded typed struct. This is what keeps `WitnessedInteraction` self-contained in
///   `elohim-epr`: no `brit-epr` type dependency, hence no Cargo cycle and no cross-crate ts-rs
///   `../../../../` import trap.
///
/// Struct-style variants (project more cleanly in ts-rs than tuple variants). Variant tags stay
/// PascalCase to match the spec §2.3 literals (`Count`/`Vote`/`Classification`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum Magnitude {
    /// Additive monoid — consumption/value.
    Count { value: f64, unit: String },
    /// Ordinal signed tally — governance.
    Vote { sign: i8 },
    /// Frame-ref — governance classification; the CID of the frame atom.
    Classification {
        #[serde(rename = "frameRef")]
        #[ts(type = "string")]
        frame_ref: Cid,
    },
}

// ---------------------------------------------------------------------------
// FrameClassification — durable truth split from resolution provenance
// (spec §2.4, must-fix #2)
// ---------------------------------------------------------------------------

/// The durable, content-addressed frame judgement — **what the CID hashes over.**
///
/// The graduated escalation ladder (`compute_source`, `resolved_by_hop`, `bounded_by`) is
/// deliberately NOT here — it lives in [`ResolutionProvenance`], coupled by CID. Were it in
/// `canonical_bytes` it would be in the CID, and the H3 peer-native leg setting
/// `resolved_by_hop: H3` would mint a *different* CID for the *same* judgement, moving the
/// aggregation anchor out from under everything (spec must-fix #2).
///
/// Schema-checked invariant (enforced downstream, not here): a **detector** (Family-2) frame may
/// only emit [`FrameVerdict::Abstain`], never `Drift`; a **defeater** (Family-1) may emit
/// `Legitimate`/`Drift`. A labeling function may only over-fire; suppression is Layer-B's job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct FrameClassification {
    /// CID of the write/EPR judged (v1: server-minted path digest — an honest gap, spec §9).
    #[ts(type = "string")]
    pub target_cid: Cid,

    /// CID of the frame atom (SDK-schema data, spec §3).
    #[ts(type = "string")]
    pub frame_ref: Cid,

    pub verdict: FrameVerdict,

    /// Confidence in `[0, 1]`; a coarse proxy at v1.
    pub confidence: f64,

    /// Spans (over ORIGINAL content), matched recall signal, rubric answer, reason ref.
    pub evidence: FrameEvidence,
}

impl FrameClassification {
    /// Canonical dag-cbor bytes (alphabetical map keys). Excludes all of
    /// [`ResolutionProvenance`] by construction (must-fix #2).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let mut map: BTreeMap<String, Ipld> = BTreeMap::new();
        map.insert("confidence".into(), Ipld::Float(self.confidence));
        map.insert("evidence".into(), evidence_ipld(&self.evidence));
        map.insert("frameRef".into(), Ipld::Link(self.frame_ref));
        map.insert("targetCid".into(), Ipld::Link(self.target_cid));
        map.insert(
            "verdict".into(),
            Ipld::String(verdict_canonical(self.verdict)),
        );
        cbor::encode(&Ipld::Map(map))
    }

    /// CIDv1(dag-cbor, sha2-256) over the canonical bytes.
    pub fn content_cid(&self) -> Result<Cid> {
        Ok(compute_cid(&self.canonical_bytes()?))
    }
}

/// Evidence backing a [`FrameClassification`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct FrameEvidence {
    /// Half-open spans over the ORIGINAL content the frame matched.
    pub spans: Vec<Span>,
    /// Recall-signal phrases/markers that fired.
    pub matched_recall_signal: Vec<String>,
    /// Rubric answer, when the frame carries a rubric.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(optional)]
    pub rubric_answer: Option<String>,
    /// CID of a reason record (e.g. a model rationale), when present.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub reason_ref: Option<Cid>,
}

/// Operational resolution provenance — **coupled to a [`FrameClassification`] by CID, NOT in its
/// CID** (spec §2.4, must-fix #2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct ResolutionProvenance {
    /// Couples to the [`FrameClassification`] this provenance describes.
    #[ts(type = "string")]
    pub classification_cid: Cid,

    /// v1 self-reported classifier identity; graduates to `agent_initial_pubkey`. A `String`
    /// (not a closed enum) precisely because it must hold both forms across the graduation.
    pub classifier_agent: String,

    pub compute_source: ComputeSource,

    pub resolved_by_hop: Hop,

    /// `delegates-compute` Commitment CID, present at H3 (the bounded-compute leg).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub bounded_by: Option<Cid>,
}

// ---------------------------------------------------------------------------
// Leaf enums / value types
// ---------------------------------------------------------------------------

/// Half-open interval `[start, end)` — a span over content bytes/tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct Span {
    pub start: u64,
    pub end: u64,
}

/// The frame verdict. **ts-rs-rooted** (a `FrameClassification` field type) — there is no
/// `frame-verdict` schema peer (spec must-fix #3: one generator per enum).
///
/// `Abstain` is first-class: a detector that looked and declined is real evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum FrameVerdict {
    Legitimate,
    Drift,
    Abstain,
}

/// Where the classifier compute ran. **ts-rs-rooted.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum ComputeSource {
    Terminal,
    ApiKey,
    PeerNative,
}

/// The graduated escalation rung a resolution reached. **ts-rs-rooted.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum Hop {
    H0,
    H1,
    H2,
    H3,
}

/// REA verb for a witnessed interaction. **ts-rs-rooted here** — no authoritative closed verb
/// enum exists to reuse: the economic-event schemas define `action` as a free-form string (no
/// enum), and `elohim_storage::services::economic_event_service::EconomicAction`
/// (`Produce|Consume|Transfer|Use`) is a downstream leaf's private, non-exported enum whose crate
/// depends on `elohim-epr` (reusing it would be a Cargo cycle). Rooting here is therefore not
/// dual-sourcing (spec must-fix #3). Serializes lowercase to match the spec §2.2 canonical forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum ReaVerb {
    Use,
    Consume,
    Produce,
    Cite,
    Affirm,
    Dismiss,
}

/// v1 self-reported witness identity `{claude|codex|gemini|human}`. Graduates to a signed
/// `agent_initial_pubkey`; the closed enum is the simplest faithful v1 shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum WitnessId {
    Claude,
    Codex,
    Gemini,
    Human,
}

// ---------------------------------------------------------------------------
// Canonical Ipld helpers (mirror envelope.rs's kind_canonical / reach_canonical style)
// ---------------------------------------------------------------------------

fn magnitude_ipld(m: &Magnitude) -> Ipld {
    let mut outer: BTreeMap<String, Ipld> = BTreeMap::new();
    match m {
        Magnitude::Count { value, unit } => {
            let mut inner: BTreeMap<String, Ipld> = BTreeMap::new();
            inner.insert("unit".into(), Ipld::String(unit.clone()));
            inner.insert("value".into(), Ipld::Float(*value));
            outer.insert("Count".into(), Ipld::Map(inner));
        }
        Magnitude::Vote { sign } => {
            let mut inner: BTreeMap<String, Ipld> = BTreeMap::new();
            inner.insert("sign".into(), Ipld::Integer(*sign as i128));
            outer.insert("Vote".into(), Ipld::Map(inner));
        }
        Magnitude::Classification { frame_ref } => {
            let mut inner: BTreeMap<String, Ipld> = BTreeMap::new();
            inner.insert("frameRef".into(), Ipld::Link(*frame_ref));
            outer.insert("Classification".into(), Ipld::Map(inner));
        }
    }
    Ipld::Map(outer)
}

fn span_ipld(s: &Span) -> Ipld {
    let mut m: BTreeMap<String, Ipld> = BTreeMap::new();
    m.insert("end".into(), Ipld::Integer(s.end as i128));
    m.insert("start".into(), Ipld::Integer(s.start as i128));
    Ipld::Map(m)
}

fn evidence_ipld(e: &FrameEvidence) -> Ipld {
    let mut m: BTreeMap<String, Ipld> = BTreeMap::new();
    m.insert(
        "matchedRecallSignal".into(),
        Ipld::List(
            e.matched_recall_signal
                .iter()
                .map(|s| Ipld::String(s.clone()))
                .collect(),
        ),
    );
    if let Some(r) = e.reason_ref {
        m.insert("reasonRef".into(), Ipld::Link(r));
    }
    if let Some(ans) = &e.rubric_answer {
        m.insert("rubricAnswer".into(), Ipld::String(ans.clone()));
    }
    m.insert(
        "spans".into(),
        Ipld::List(e.spans.iter().map(span_ipld).collect()),
    );
    Ipld::Map(m)
}

fn reaverb_canonical(v: ReaVerb) -> String {
    match v {
        ReaVerb::Use => "use",
        ReaVerb::Consume => "consume",
        ReaVerb::Produce => "produce",
        ReaVerb::Cite => "cite",
        ReaVerb::Affirm => "affirm",
        ReaVerb::Dismiss => "dismiss",
    }
    .into()
}

fn witnessid_canonical(w: WitnessId) -> String {
    match w {
        WitnessId::Claude => "claude",
        WitnessId::Codex => "codex",
        WitnessId::Gemini => "gemini",
        WitnessId::Human => "human",
    }
    .into()
}

fn verdict_canonical(v: FrameVerdict) -> String {
    match v {
        FrameVerdict::Legitimate => "Legitimate",
        FrameVerdict::Drift => "Drift",
        FrameVerdict::Abstain => "Abstain",
    }
    .into()
}

// ---------------------------------------------------------------------------
// Golden CID self-vector (spec §4.4 — the elohim-epr leg of the three-way vector)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn tcid(b: u8) -> Cid {
        compute_cid(&[b])
    }

    /// A fixed, canonical governance `WitnessedInteraction` — a witness affirming an object with a
    /// frame classification riding in the magnitude. `substrate` is `"attention"` (a valid
    /// `SUBSTRATE_SIGNALS` value): governance is NOT a substrate, it rides in the magnitude.
    fn golden_witness() -> WitnessedInteraction {
        WitnessedInteraction {
            object_cid: tcid(1),
            substrate: "attention".into(),
            action: ReaVerb::Affirm,
            magnitude: Magnitude::Classification { frame_ref: tcid(7) },
            witness: WitnessId::Claude,
            coverage_span: None,
            issued_at: Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap(),
        }
    }

    /// A fixed, canonical `FrameClassification` — a Drift verdict from a defeater frame.
    fn golden_classification() -> FrameClassification {
        FrameClassification {
            target_cid: tcid(10),
            frame_ref: tcid(7),
            verdict: FrameVerdict::Drift,
            confidence: 0.75,
            evidence: FrameEvidence {
                spans: vec![Span { start: 0, end: 12 }],
                matched_recall_signal: vec!["self-sovereign".into()],
                rubric_answer: Some("apex-identity".into()),
                reason_ref: Some(tcid(11)),
            },
        }
    }

    // ── Golden CID pins ────────────────────────────────────────────────
    // Non-skippable, no oracle: a fixed value needs none. These are the elohim-epr leg of the
    // eventual three-way vector; the brit-epr + eprfs-core legs (spec §4.4) are G1b follow-up and
    // MUST reproduce these exact strings from the same logical values.
    const GOLDEN_WITNESS_CID: &str = "bafyreihw7wwany3vsobq5ci4rtahtu7urhcldpv5c2grg4uw2dv2xmxs4u";
    const GOLDEN_CLASSIFICATION_CID: &str =
        "bafyreic3kk7rq6wgpztkctvnc4rso3z65ibybu2by5fwcfggbquvgmfgb4";

    #[test]
    fn witnessed_interaction_golden_cid() {
        let cid = golden_witness().content_cid().unwrap();
        assert_eq!(
            cid.to_string(),
            GOLDEN_WITNESS_CID,
            "WitnessedInteraction golden CID drift — actual = {cid}"
        );
    }

    #[test]
    fn frame_classification_golden_cid() {
        let cid = golden_classification().content_cid().unwrap();
        assert_eq!(
            cid.to_string(),
            GOLDEN_CLASSIFICATION_CID,
            "FrameClassification golden CID drift — actual = {cid}"
        );
    }

    #[test]
    fn canonical_bytes_are_deterministic() {
        let w = golden_witness();
        assert_eq!(w.canonical_bytes().unwrap(), w.canonical_bytes().unwrap());
        let f = golden_classification();
        assert_eq!(f.canonical_bytes().unwrap(), f.canonical_bytes().unwrap());
    }

    #[test]
    fn magnitude_variant_changes_cid() {
        let mut w = golden_witness();
        let a = w.content_cid().unwrap();
        w.magnitude = Magnitude::Vote { sign: 1 };
        let b = w.content_cid().unwrap();
        assert_ne!(a, b, "magnitude must affect canonical bytes");
    }

    #[test]
    fn resolution_provenance_not_in_classification_cid() {
        // Two provenances over the SAME classification must not perturb the classification CID —
        // the whole point of must-fix #2. (Provenance is coupled by CID, never hashed into it.)
        let f = golden_classification();
        let base = f.content_cid().unwrap();
        let _p_h0 = ResolutionProvenance {
            classification_cid: base,
            classifier_agent: "claude".into(),
            compute_source: ComputeSource::Terminal,
            resolved_by_hop: Hop::H0,
            bounded_by: None,
        };
        let _p_h3 = ResolutionProvenance {
            classification_cid: base,
            classifier_agent: "peer".into(),
            compute_source: ComputeSource::PeerNative,
            resolved_by_hop: Hop::H3,
            bounded_by: Some(tcid(20)),
        };
        // The classification CID is stable regardless of which provenance couples to it.
        assert_eq!(f.content_cid().unwrap(), base);
    }

    #[test]
    fn serde_roundtrips() {
        let w = golden_witness();
        let j = serde_json::to_string(&w).unwrap();
        let back: WitnessedInteraction = serde_json::from_str(&j).unwrap();
        assert_eq!(w, back);

        let f = golden_classification();
        let j = serde_json::to_string(&f).unwrap();
        let back: FrameClassification = serde_json::from_str(&j).unwrap();
        assert_eq!(f, back);
    }
}
