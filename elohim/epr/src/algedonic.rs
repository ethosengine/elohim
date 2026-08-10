//! Algedonic feedback signals — pain as a **viability** signal, orthogonal to judgment valence.
//!
//! See `genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md`
//! (§4b "Design canon" is the do/do-not list every construct here anchors).
//!
//! # The loop
//!
//! A **producer** (the `declarer` — a self-reporting node, storage validator, or the operator's
//! elohim-counsel) measures a **stock** (`evidence.stock`) against a **limit**
//! (`evidence.limit`, declared by the bounding commitment/manifest at `evidence.bound_ref`) and
//! emits one signal to the **consumer** — the `target`, the CID of the promise threatened. Two
//! kinds close the loop: [`AlgedonicKind::Approach`] fires at the band edge (`threshold_pct` of
//! the limit — the signal *before* the line) and [`AlgedonicKind::Breach`] fires once the bound
//! is crossed. A node in pain accuses no one; it reports its own sensed state against a limit it
//! declared, which is why `standing_impact` is fixed at [`STANDING_IMPACT`] (`advisory`) and why
//! no valence field exists anywhere in this module.
//!
//! # Invariants, and where each one lives
//!
//! - **No valence.** There is no agree/disagree/position/vote field — unrepresentable, not
//!   validated. Algedonic is a third feedback type, not an overload of judgment valence.
//! - **Evidence is mandatory.** `stock`, `limit` and `bound_ref` are non-optional fields of
//!   [`AlgedonicEvidence`]: a signal without them cannot be constructed, and refuses to
//!   deserialize.
//! - **`Approach` requires `threshold_pct`; `Breach` does not.** Carried by the evidence enum's
//!   own variants, so the two shapes cannot be confused — and the kind is *derived from* the
//!   evidence ([`AlgedonicEvidence::kind`]), so a signal whose `signal_kind` disagrees with its
//!   evidence is unrepresentable in [`AlgedonicSignal`] and refused at the wire boundary.
//! - **One open signal per `(declarer, target, kind)`.** [`open_signal_key`] is that key; pain is
//!   a held state, not a stream. [`should_emit`] is the hysteresis predicate that enforces it.
//! - **Evidence is minted, never assembled.** The limit a signal reports is a bound on a
//!   *promise*, so evidence is derived by the REA fold that owns the bound
//!   (`elohim_epr_rea::fold::fulfillment`, whose `bound_ref` is the bounding commitment's CID) —
//!   the single evidence constructor. That is what keeps the extra-key edge theoretical: the
//!   schemas permit unlisted evidence keys, so a stray `threshold_pct` on a breach payload would
//!   deserialize as `Approach` and be refused by the coherence check below; nothing on the minted
//!   path can build one, because the variant is chosen from the crossing.
//!
//! # Wire contract
//!
//! `elohim/sdk/schemas/v1/feedback-signals/algedonic-{approach,breach}.schema.json`. Those
//! schemas are the **contract only** — the source of truth is the DHT `FeedbackSignal` entry
//! (elohim zome, `content_store_integrity/src/feedback_signal.rs`), extended by `signal_kind`
//! vocabulary rather than by a new entry type. Field names here are **snake_case**, matching
//! those schemas exactly; that is deliberately unlike the crate's camelCase ts-rs types, and is
//! why nothing in this module derives `TS`: the schemas already own the projection.

use crate::error::{EprError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The fixed standing impact of every algedonic signal.
///
/// Pain never debits standing by itself — consequences flow only through the witnessed
/// governance path (spec §3). It is a **constant, not a field**: the wire schemas declare
/// `additionalProperties: false` and do not list `standing_impact`, so emitting it would fail
/// validation. Consumers read it from here.
pub const STANDING_IMPACT: &str = "advisory";

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

/// The two algedonic signal kinds.
///
/// Wire form is the full `signal_kind` const from the schemas (`algedonic-approach` /
/// `algedonic-breach`) — these are members of the DHT `FeedbackSignal` kind whitelist, not a
/// crate-local shorthand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlgedonicKind {
    /// Band edge crossed — the stock has passed `threshold_pct` of the limit but the bound
    /// still holds. The signal before the line.
    #[serde(rename = "algedonic-approach")]
    Approach,
    /// The bound has been crossed, or a self-heal mechanism is exhausted.
    #[serde(rename = "algedonic-breach")]
    Breach,
}

impl AlgedonicKind {
    /// The exact `signal_kind` wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            AlgedonicKind::Approach => "algedonic-approach",
            AlgedonicKind::Breach => "algedonic-breach",
        }
    }
}

/// How loudly the producer is reporting. Advisory only — severity never touches standing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    /// The schema default.
    #[default]
    Warn,
    Critical,
}

// ---------------------------------------------------------------------------
// Evidence — the producer/stock/limit half of the loop
// ---------------------------------------------------------------------------

/// The measured evidence a signal carries. **Every field is non-optional**: evidence-free pain is
/// unrepresentable, not merely rejected.
///
/// The variants ARE the per-kind requirement: `threshold_pct` exists on `Approach` and nowhere
/// else, so "Breach must not require a band edge" needs no runtime rule. Serialization is
/// untagged — the wire evidence object carries no discriminator, because the kind is already at
/// the top level as `signal_kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AlgedonicEvidence {
    /// Band-edge evidence. Ordered first so an object carrying `threshold_pct` never falls
    /// through to `Breach`.
    Approach {
        /// The measured stock level at time of the signal.
        stock: f64,
        /// The bound being approached.
        limit: f64,
        /// CID of the bounding commitment/manifest that declared the limit.
        bound_ref: String,
        /// The band edge as a **percentage of the limit** (`85.0` means 85%), per the
        /// limit-governor stub's elevate step.
        threshold_pct: f64,
    },
    /// Bound-crossed evidence — no band edge, because the line itself is the edge.
    Breach {
        /// The measured stock level at time of breach.
        stock: f64,
        /// The bound that was breached.
        limit: f64,
        /// CID of the bounding commitment/manifest whose limit was breached.
        bound_ref: String,
    },
}

impl AlgedonicEvidence {
    /// The kind this evidence *is*. The signal's `signal_kind` is derived from here, never
    /// stored alongside it.
    pub const fn kind(&self) -> AlgedonicKind {
        match self {
            AlgedonicEvidence::Approach { .. } => AlgedonicKind::Approach,
            AlgedonicEvidence::Breach { .. } => AlgedonicKind::Breach,
        }
    }

    /// The measured quantity.
    pub const fn stock(&self) -> f64 {
        match self {
            AlgedonicEvidence::Approach { stock, .. } | AlgedonicEvidence::Breach { stock, .. } => {
                *stock
            }
        }
    }

    /// The declared bound.
    pub const fn limit(&self) -> f64 {
        match self {
            AlgedonicEvidence::Approach { limit, .. } | AlgedonicEvidence::Breach { limit, .. } => {
                *limit
            }
        }
    }

    /// CID of the commitment/manifest that declared the bound.
    pub fn bound_ref(&self) -> &str {
        match self {
            AlgedonicEvidence::Approach { bound_ref, .. }
            | AlgedonicEvidence::Breach { bound_ref, .. } => bound_ref,
        }
    }

    /// The stock level at which this kind's line is crossed: `limit × threshold_pct / 100` for an
    /// approach, the `limit` itself for a breach. Ceiling-bound semantics at v1 (a floor bound —
    /// pain for a stock falling *below* a line — is not modeled).
    pub fn band_edge(&self) -> f64 {
        match self {
            AlgedonicEvidence::Approach {
                limit,
                threshold_pct,
                ..
            } => limit * threshold_pct / 100.0,
            AlgedonicEvidence::Breach { limit, .. } => *limit,
        }
    }

    /// Whether the measured stock has actually reached this kind's line.
    pub fn crossed(&self) -> bool {
        self.stock() >= self.band_edge()
    }
}

// ---------------------------------------------------------------------------
// The signal
// ---------------------------------------------------------------------------

/// One algedonic signal: a producer reporting a stock against a limit, to the promise it threatens.
///
/// There is **no `signal_kind` field** — the kind is derived from [`Self::evidence`], so a signal
/// that claims `algedonic-approach` while carrying breach evidence cannot exist. The wire form
/// carries `signal_kind` (the schemas require it); the coherence check runs once, at the
/// deserialization boundary, where untrusted bytes arrive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(into = "AlgedonicWire", try_from = "AlgedonicWire")]
pub struct AlgedonicSignal {
    /// Consumer — CID of the promise (Commitment; `cid = entry_hash`) threatened.
    pub target: String,
    /// Producer — CID or agent identifier of the node that declared the pain.
    pub declarer: String,
    /// Stock, limit and bound — mandatory by construction.
    pub evidence: AlgedonicEvidence,
    /// Advisory loudness; never a standing consequence.
    pub severity: Severity,
    /// When the producer signed the report.
    pub signed_at: DateTime<Utc>,
}

impl AlgedonicSignal {
    /// The kind, derived from the evidence.
    pub const fn kind(&self) -> AlgedonicKind {
        self.evidence.kind()
    }

    /// Always [`STANDING_IMPACT`] — pain informs, it does not withdraw standing.
    pub const fn standing_impact(&self) -> &'static str {
        STANDING_IMPACT
    }
}

/// The wire projection: `signal_kind` restored to the top level for the schemas' benefit.
///
/// Private on purpose — it is the only shape in which kind and evidence can disagree, and it
/// exists for exactly as long as one `TryFrom` takes to reject that.
#[derive(Serialize, Deserialize)]
struct AlgedonicWire {
    signal_kind: AlgedonicKind,
    target: String,
    declarer: String,
    evidence: AlgedonicEvidence,
    /// Optional on the wire — the schemas omit it from `required` and declare
    /// `"default": "warn"`. Without `default` here, serde would reject a
    /// contract-valid document that simply didn't say how loud the pain is.
    #[serde(default)]
    severity: Severity,
    signed_at: DateTime<Utc>,
}

impl From<AlgedonicSignal> for AlgedonicWire {
    fn from(s: AlgedonicSignal) -> Self {
        AlgedonicWire {
            signal_kind: s.evidence.kind(),
            target: s.target,
            declarer: s.declarer,
            evidence: s.evidence,
            severity: s.severity,
            signed_at: s.signed_at,
        }
    }
}

impl TryFrom<AlgedonicWire> for AlgedonicSignal {
    type Error = EprError;

    fn try_from(w: AlgedonicWire) -> Result<Self> {
        if w.signal_kind != w.evidence.kind() {
            return Err(EprError::Algedonic(format!(
                "signal_kind {} disagrees with its evidence ({})",
                w.signal_kind.as_str(),
                w.evidence.kind().as_str()
            )));
        }
        Ok(AlgedonicSignal {
            target: w.target,
            declarer: w.declarer,
            evidence: w.evidence,
            severity: w.severity,
            signed_at: w.signed_at,
        })
    }
}

// ---------------------------------------------------------------------------
// Dedupe key + emission bounds
// ---------------------------------------------------------------------------

/// The dedupe key: at most ONE open signal exists per `(declarer, target, kind)`.
///
/// An internal index key, never an address — see the spec's head-plane honesty rule. Because
/// `kind` participates, an open `Approach` never suppresses the `Breach` that escalates it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OpenSignalKey {
    pub declarer: String,
    pub target: String,
    pub kind: AlgedonicKind,
}

/// The dedupe key of a signal. Two signals differing only in their evidence collide here —
/// that is the point: re-measuring the same pain is not new pain.
pub fn open_signal_key(signal: &AlgedonicSignal) -> OpenSignalKey {
    OpenSignalKey {
        declarer: signal.declarer.clone(),
        target: signal.target.clone(),
        kind: signal.kind(),
    }
}

/// Structural validation — the refusals that the type system cannot make unrepresentable.
///
/// Mirrors [`crate::validation::validate_coupling`]: a positive `Ok(())` or a refusal naming the
/// requirement. Everything checked here is a `minLength`/finiteness obligation of the wire
/// schemas; the *shape* obligations (evidence present, `threshold_pct` on approaches only, no
/// valence) are already unrepresentable and are deliberately absent from this function.
pub fn validate(signal: &AlgedonicSignal) -> Result<()> {
    let refuse = |what: &str| -> Result<()> { Err(EprError::Algedonic(what.to_string())) };

    if signal.target.is_empty() {
        return refuse("target (the threatened promise) must be non-empty");
    }
    if signal.declarer.is_empty() {
        return refuse("declarer (the producer) must be non-empty");
    }
    if signal.evidence.bound_ref().is_empty() {
        return refuse("evidence.bound_ref (the bound's declaring CID) must be non-empty");
    }
    if !signal.evidence.stock().is_finite() {
        return refuse("evidence.stock must be a finite measurement");
    }
    if !signal.evidence.limit().is_finite() {
        return refuse("evidence.limit must be a finite bound");
    }
    if let AlgedonicEvidence::Approach { threshold_pct, .. } = signal.evidence {
        if !threshold_pct.is_finite() || threshold_pct <= 0.0 || threshold_pct > 100.0 {
            return refuse("evidence.threshold_pct must be a percentage in (0, 100]");
        }
    }
    Ok(())
}

/// The hysteresis predicate — the answer to "may this signal be emitted now?".
///
/// Two rules, both from the spec's emission-bounds clause:
///
/// 1. **Inside the band, silence.** A signal whose stock has not reached its own line
///    ([`AlgedonicEvidence::crossed`]) is not pain yet. Nothing to report.
/// 2. **One open signal per key.** While a signal with the same [`OpenSignalKey`] is open, no
///    re-fire — pain is a held state, not a stream, and re-tolling it is the anti-pattern this
///    predicate exists to prevent. An open `Approach` does NOT suppress a `Breach` (different
///    kind ⇒ different key ⇒ the escalation gets through).
///
/// `prev` is the currently-open signal for this key, if the caller holds one.
pub fn should_emit(prev: Option<&AlgedonicSignal>, new: &AlgedonicSignal) -> bool {
    if !new.evidence.crossed() {
        return false;
    }
    match prev {
        Some(open) => open_signal_key(open) != open_signal_key(new),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    // ── Harness ──────────────────────────────────────────────────────

    /// The wire contract lives in the repo, not in a constant here — this test reads the same
    /// bytes CI and the seeder read.
    fn load_schema(file: &str) -> Value {
        // CARGO_MANIFEST_DIR = elohim/epr/ ; parent = elohim/
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("parent of epr/ must be elohim/")
            .join("sdk/schemas/v1/feedback-signals")
            .join(file);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read schema {}: {}", path.display(), e));
        serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("failed to parse schema {}: {}", path.display(), e))
    }

    fn assert_conforms(schema_file: &str, instance: &Value) {
        let schema = load_schema(schema_file);
        let validator = jsonschema::validator_for(&schema)
            .unwrap_or_else(|e| panic!("failed to compile schema {}: {}", schema_file, e));
        let errors: Vec<String> = validator
            .iter_errors(instance)
            .map(|e| format!("  - {} (at {})", e, e.instance_path))
            .collect();
        assert!(
            errors.is_empty(),
            "schema validation failed for {}:\n{}\n\ninstance:\n{}",
            schema_file,
            errors.join("\n"),
            serde_json::to_string_pretty(instance).unwrap()
        );
    }

    fn signed_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 10, 12, 0, 0).unwrap()
    }

    fn approach() -> AlgedonicSignal {
        AlgedonicSignal {
            target: "bafy-promise".into(),
            declarer: "bafy-matthew".into(),
            evidence: AlgedonicEvidence::Approach {
                stock: 880.0,
                limit: 1000.0,
                bound_ref: "bafy-bound".into(),
                threshold_pct: 85.0,
            },
            severity: Severity::Warn,
            signed_at: signed_at(),
        }
    }

    fn breach() -> AlgedonicSignal {
        AlgedonicSignal {
            target: "bafy-promise".into(),
            declarer: "bafy-matthew".into(),
            evidence: AlgedonicEvidence::Breach {
                stock: 1200.0,
                limit: 1000.0,
                bound_ref: "bafy-bound".into(),
            },
            severity: Severity::Critical,
            signed_at: signed_at(),
        }
    }

    // ── (a) Evidence is mandatory ─────────────────────────────────────

    /// Evidence-free pain does not deserialize. `stock`/`limit`/`bound_ref` are non-`Option`, so
    /// there is no in-Rust way to omit them at all; the wire is the only place absence can be
    /// attempted, and it is refused there.
    #[test]
    fn evidence_fields_are_mandatory_on_the_wire() {
        for missing in ["stock", "limit", "bound_ref"] {
            let mut wire = serde_json::to_value(breach()).unwrap();
            wire["evidence"]
                .as_object_mut()
                .unwrap()
                .remove(missing)
                .expect("field must have been present to remove");
            let parsed: std::result::Result<AlgedonicSignal, _> = serde_json::from_value(wire);
            assert!(
                parsed.is_err(),
                "a signal missing evidence.{missing} must be refused"
            );
        }
    }

    /// An empty `bound_ref` is present-but-useless evidence: the bound cannot be resolved, so the
    /// pain has no address. `validate` refuses it with the crate's error type.
    #[test]
    fn validate_refuses_evidence_that_names_no_bound() {
        let mut s = breach();
        s.evidence = AlgedonicEvidence::Breach {
            stock: 1200.0,
            limit: 1000.0,
            bound_ref: String::new(),
        };
        let err = validate(&s).expect_err("empty bound_ref must refuse");
        assert!(err.to_string().contains("bound_ref"), "got: {err}");
        assert!(
            matches!(err, EprError::Algedonic(_)),
            "pain has its own refusal class, not the envelope's"
        );

        // …and the valid samples pass, so the refusal is not a blanket one.
        validate(&approach()).expect("valid approach");
        validate(&breach()).expect("valid breach");
    }

    #[test]
    fn validate_refuses_a_signal_with_no_producer_or_consumer() {
        let mut s = approach();
        s.target = String::new();
        assert!(validate(&s).is_err(), "empty target must refuse");

        let mut s = approach();
        s.declarer = String::new();
        assert!(validate(&s).is_err(), "empty declarer must refuse");
    }

    // ── (b) Approach requires threshold_pct; Breach does not ──────────

    /// The per-kind requirement is carried by the evidence variants, so it holds in both
    /// directions: an approach without a band edge is refused, a breach without one is fine.
    #[test]
    fn approach_requires_threshold_pct_and_breach_does_not() {
        let mut wire = serde_json::to_value(approach()).unwrap();
        wire["evidence"]
            .as_object_mut()
            .unwrap()
            .remove("threshold_pct")
            .expect("approach evidence must carry threshold_pct");
        let parsed: std::result::Result<AlgedonicSignal, _> = serde_json::from_value(wire);
        assert!(
            parsed.is_err(),
            "an approach whose evidence lost its band edge must be refused"
        );

        // A breach never carried one, and round-trips clean.
        let wire = serde_json::to_value(breach()).unwrap();
        assert!(
            wire["evidence"].get("threshold_pct").is_none(),
            "breach evidence must not emit a band edge"
        );
        let back: AlgedonicSignal = serde_json::from_value(wire).expect("breach round-trips");
        assert_eq!(back, breach());
    }

    /// The kind is derived from the evidence, so the two cannot drift apart in Rust; the only
    /// place they can disagree is untrusted wire input, and that is refused.
    #[test]
    fn kind_and_evidence_cannot_disagree() {
        assert_eq!(approach().kind(), AlgedonicKind::Approach);
        assert_eq!(breach().kind(), AlgedonicKind::Breach);

        let mut wire = serde_json::to_value(breach()).unwrap();
        wire["signal_kind"] = Value::String("algedonic-approach".into());
        let err = serde_json::from_value::<AlgedonicSignal>(wire)
            .expect_err("mismatched signal_kind must be refused");
        assert!(err.to_string().contains("disagrees"), "got: {err}");
    }

    // ── (c) No valence, ever ──────────────────────────────────────────

    /// Algedonic is a viability signal, not a judgment. The struct simply has no valence field —
    /// this test pins the consequence: nothing valence-shaped reaches the wire, and the emitted
    /// key-set stays inside what the schema declares (`additionalProperties: false`).
    #[test]
    fn no_valence_field_reaches_the_wire() {
        const VALENCE_SHAPED: [&str; 7] = [
            "position", "valence", "vote", "sign", "agree", "disagree", "polarity",
        ];

        for (signal, schema_file) in [
            (approach(), "algedonic-approach.schema.json"),
            (breach(), "algedonic-breach.schema.json"),
        ] {
            let wire = serde_json::to_value(&signal).unwrap();
            let declared = load_schema(schema_file);
            let properties = declared["properties"].as_object().unwrap();

            for key in wire.as_object().unwrap().keys() {
                assert!(
                    !VALENCE_SHAPED.contains(&key.as_str()),
                    "{schema_file}: valence-shaped key `{key}` must not exist"
                );
                assert!(
                    properties.contains_key(key),
                    "{schema_file}: emitted key `{key}` is not declared by the schema"
                );
            }

            // standing_impact is a constant, not a field — emitting it would breach
            // additionalProperties: false.
            assert!(
                wire.get("standing_impact").is_none(),
                "standing_impact must not serialize"
            );
            assert_eq!(signal.standing_impact(), "advisory");
        }
    }

    // ── (d) Wire parity with the schemas on disk ──────────────────────

    /// Serialize a valid sample of each kind and validate it against the schema file itself —
    /// no copied constant, no restated field list.
    #[test]
    fn samples_conform_to_the_wire_schemas() {
        for (signal, schema_file) in [
            (approach(), "algedonic-approach.schema.json"),
            (breach(), "algedonic-breach.schema.json"),
        ] {
            let wire = serde_json::to_value(&signal).unwrap();
            assert_conforms(schema_file, &wire);

            // Every schema-`required` field is actually present (the validator would catch this,
            // but naming it here is what makes a future schema addition fail loudly).
            let declared = load_schema(schema_file);
            for required in declared["required"].as_array().unwrap() {
                let field = required.as_str().unwrap();
                assert!(
                    wire.get(field).is_some(),
                    "{schema_file}: required field `{field}` missing from the emitted signal"
                );
            }
            for required in declared["properties"]["evidence"]["required"]
                .as_array()
                .unwrap()
            {
                let field = required.as_str().unwrap();
                assert!(
                    wire["evidence"].get(field).is_some(),
                    "{schema_file}: required evidence field `{field}` missing"
                );
            }
        }
    }

    #[test]
    fn signal_kind_matches_the_schema_const() {
        for (kind, schema_file) in [
            (AlgedonicKind::Approach, "algedonic-approach.schema.json"),
            (AlgedonicKind::Breach, "algedonic-breach.schema.json"),
        ] {
            let declared = load_schema(schema_file);
            assert_eq!(
                declared["properties"]["signal_kind"]["const"]
                    .as_str()
                    .unwrap(),
                kind.as_str()
            );
            assert_eq!(
                serde_json::to_value(kind).unwrap().as_str().unwrap(),
                kind.as_str()
            );
        }
    }

    #[test]
    fn severity_vocabulary_matches_the_schema() {
        let declared = load_schema("algedonic-approach.schema.json");
        let allowed: Vec<&str> = declared["properties"]["severity"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        for s in [Severity::Info, Severity::Warn, Severity::Critical] {
            let wire = serde_json::to_value(s).unwrap();
            assert!(
                allowed.contains(&wire.as_str().unwrap()),
                "severity {wire} not in schema enum {allowed:?}"
            );
        }
        // The Rust default agrees with the schema's declared default. This asserts the two
        // *declarations* line up — it says nothing about the ingest path, which is
        // `severity_defaults_to_warn_when_the_wire_omits_it`'s job.
        assert_eq!(
            declared["properties"]["severity"]["default"]
                .as_str()
                .expect("the schema must declare a severity default"),
            serde_json::to_value(Severity::default())
                .unwrap()
                .as_str()
                .unwrap()
        );
    }

    /// `severity` is optional on the wire: the schemas leave it out of `required` and declare
    /// `"default": "warn"`. A producer that omits it is emitting a **contract-valid** document —
    /// proved here by running it through the schema validator — so ingest must accept it and
    /// supply the default, not refuse with `missing field 'severity'`.
    #[test]
    fn severity_defaults_to_warn_when_the_wire_omits_it() {
        for (signal, schema_file) in [
            (approach(), "algedonic-approach.schema.json"),
            (breach(), "algedonic-breach.schema.json"),
        ] {
            let mut wire = serde_json::to_value(&signal).unwrap();
            wire.as_object_mut()
                .unwrap()
                .remove("severity")
                .expect("the fixture must have carried a severity to remove");

            // It is still a valid document by the contract on disk…
            assert_conforms(schema_file, &wire);

            // …so it must deserialize, landing on the schema's default rather than refusing.
            let parsed: AlgedonicSignal = serde_json::from_value(wire).unwrap_or_else(|e| {
                panic!("{schema_file}: a schema-valid signal without severity must parse: {e}")
            });
            assert_eq!(
                parsed.severity,
                Severity::Warn,
                "{schema_file}: an omitted severity must default to warn"
            );

            // Everything else survived the round-trip through the default.
            assert_eq!(parsed.evidence, signal.evidence);
            assert_eq!(parsed.kind(), signal.kind());
        }
    }

    // ── (e) One open signal per (declarer, target, kind) ──────────────

    /// Re-measuring the same pain is not new pain: only `(declarer, target, kind)` keys it.
    #[test]
    fn dedupe_key_ignores_evidence() {
        let a = approach();
        let mut b = approach();
        b.evidence = AlgedonicEvidence::Approach {
            stock: 999.0, // a worse measurement…
            limit: 1000.0,
            bound_ref: "bafy-other-bound".into(),
            threshold_pct: 90.0,
        };
        b.severity = Severity::Critical;
        b.signed_at = Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap();

        assert_ne!(a, b, "the signals themselves differ");
        assert_eq!(
            open_signal_key(&a),
            open_signal_key(&b),
            "…but they are the same open pain"
        );
    }

    #[test]
    fn dedupe_key_separates_producer_consumer_and_kind() {
        let base = approach();

        let mut other_declarer = approach();
        other_declarer.declarer = "bafy-john".into();
        assert_ne!(open_signal_key(&base), open_signal_key(&other_declarer));

        let mut other_target = approach();
        other_target.target = "bafy-other-promise".into();
        assert_ne!(open_signal_key(&base), open_signal_key(&other_target));

        // Same declarer + target, escalated kind — a distinct key by design.
        assert_ne!(open_signal_key(&base), open_signal_key(&breach()));
    }

    // ── (f) Hysteresis ────────────────────────────────────────────────

    /// Inside the band there is nothing to report; crossing it emits once and then holds.
    #[test]
    fn should_emit_is_false_inside_the_band_and_true_crossing_it() {
        // 84% of the limit — the 85% band edge has not been reached.
        let mut inside = approach();
        inside.evidence = AlgedonicEvidence::Approach {
            stock: 840.0,
            limit: 1000.0,
            bound_ref: "bafy-bound".into(),
            threshold_pct: 85.0,
        };
        assert!(
            !should_emit(None, &inside),
            "a stock inside the band is not pain yet"
        );

        // 88% — crossed.
        assert!(
            should_emit(None, &approach()),
            "crossing the band edge with nothing open must emit"
        );

        // Breach's band edge is the limit itself.
        let mut under = breach();
        under.evidence = AlgedonicEvidence::Breach {
            stock: 999.0,
            limit: 1000.0,
            bound_ref: "bafy-bound".into(),
        };
        assert!(!should_emit(None, &under), "the bound still holds");
        assert!(should_emit(None, &breach()), "the bound is crossed");
    }

    /// Pain is a held state, not a stream.
    #[test]
    fn should_emit_suppresses_re_fire_while_a_signal_is_open() {
        let open = approach();

        let mut worse = approach();
        worse.evidence = AlgedonicEvidence::Approach {
            stock: 950.0,
            limit: 1000.0,
            bound_ref: "bafy-bound".into(),
            threshold_pct: 85.0,
        };
        assert!(
            !should_emit(Some(&open), &worse),
            "an open signal on the same key must not re-fire"
        );
    }

    /// …but an open approach must never swallow the breach that escalates it, and pain about a
    /// different promise is never suppressed by pain about this one.
    #[test]
    fn should_emit_lets_escalation_and_unrelated_pain_through() {
        let open_approach = approach();
        assert!(
            should_emit(Some(&open_approach), &breach()),
            "an open approach must not suppress its breach"
        );

        let mut other_promise = approach();
        other_promise.target = "bafy-other-promise".into();
        assert!(
            should_emit(Some(&open_approach), &other_promise),
            "an open signal must not suppress a different promise's pain"
        );
    }

    // ── Round-trip ────────────────────────────────────────────────────

    #[test]
    fn signals_round_trip() {
        for s in [approach(), breach()] {
            let j = serde_json::to_string(&s).unwrap();
            let back: AlgedonicSignal = serde_json::from_str(&j).unwrap();
            assert_eq!(s, back);
        }
    }
}
