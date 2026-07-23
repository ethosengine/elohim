//! The verdict spine — the shape every gradient's answer speaks.
//!
//! Protocol-owned. No app may redefine what these mean.
//!
//! The verdict spine — every gradient (reach, epistemic, compute/trust, retention)
//! speaks this shape. Spec: reach-ontology-vocabulary-split-spec §2a.
//!
//! A [`Verdict`] is one axis's answer about one subject, carried with a [`Witness`]
//! of positive per-check evidence. The answer itself is a [`Decision`]: the two
//! mechanical outcomes (`Permit`/`Refuse`) plus the first-class ceiling marker
//! (`Refer`) — a routed "a human/elohim must decide this" that a deterministic
//! substrate is structurally forbidden from collapsing into a `Refuse`.
//!
//! Nothing here is persisted: verdicts are Operational (Category C), reconstructed
//! by re-evaluation over notarized/witnessed inputs. `Verdict.subject` references
//! content by CID; it never becomes a stored status column.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Why a decision escalates to a human/elohim rather than resolving mechanically.
///
/// These three come from the constitution's escalation vocabulary
/// (`constitution.md:662-673`, `FlagForHuman(layer, ambiguity)`) — a bounded,
/// closed set. Do NOT add members without a governance decision; a new escalation
/// reason is a constitutional change, not a code convenience.
///
/// Wire form is **kebab-case** (`novel-situation` / `insufficient-authority` /
/// `contested-evidence`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum ReferReason {
    /// No rule covers this — genuinely novel, needs a human frame before it can be decided.
    NovelSituation,
    /// A rule exists but the evaluator lacks the authority/standing to apply it here.
    InsufficientAuthority,
    /// The evidence itself is disputed — an active contest, not a settled fact.
    ContestedEvidence,
}

/// A routed escalation: which governance layer should receive this, and why.
///
/// Requisite-variety routing (subsidiarity) per `constitution.md:662-673`'s
/// `FlagForHuman(layer, ambiguity)`: `layer` names the smallest governance layer
/// competent to decide. It is deliberately an open `String` — the layer taxonomy
/// is governance-configured, not a protocol-closed enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct ReferQuestion {
    /// The governance layer that should receive the question (subsidiarity routing).
    pub layer: String,
    /// Why the question escalated.
    pub reason: ReferReason,
    /// Optional human-readable context for the receiving layer.
    pub note: Option<String>,
}

/// The three-valued verdict discriminator — the spine every axis narrows onto.
///
/// **Ceiling law.** `Refer` is FIRST-CLASS: a witnessed "this is not decidable by
/// rule; the named governance layer must decide." It is never a fallthrough, never
/// a timeout, never an error — and **no conversion may collapse `Refer` into
/// `Refuse`** (a collapsed refer silently swallows a question a human was owed).
/// Conversions INTO the spine map their "not-yet-decidable" state to `Refer`, never
/// to `Refuse` (see `From<ReachVerdict>` in elohim-storage). `Permit`/`Refuse` are
/// the two mechanical outcomes a deterministic check can reach on its own.
///
/// Serde: internally tagged on `type`, so the wire discriminator is exactly
/// `["permit","refuse","refer"]` (matches `enums/decision.schema.json`). `Refer`
/// flattens its [`ReferQuestion`] alongside the tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "lowercase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum Decision {
    /// The check passed on its own authority.
    Permit,
    /// The check failed on its own authority.
    Refuse,
    /// Not mechanically decidable — routed to a governance layer. Never a fallthrough.
    Refer(ReferQuestion),
}

/// How a single check resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum CheckOutcome {
    Passed,
    Failed,
    Skipped,
}

/// Positive per-check evidence — what was checked, how it came out, and (optionally)
/// the value observed.
///
/// Modeled on `bounds-validation-result-view`, and deliberately stronger than a SHACL
/// validation report: SHACL can only carry violations (negative evidence), whereas a
/// `Passed` [`CheckWitness`] with an `observed` value is positive proof the rule held.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct CheckWitness {
    /// Stable id of the check that ran.
    pub check_id: String,
    /// How the check resolved.
    pub outcome: CheckOutcome,
    /// Human-readable one-line result.
    pub summary: String,
    /// The observed value the check recorded (any JSON, or null) — the positive evidence.
    #[ts(type = "any")]
    pub observed: Option<serde_json::Value>,
}

/// The witness backing a [`Verdict`]: the ordered per-check evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct Witness {
    pub checks: Vec<CheckWitness>,
}

/// One gradient axis's answer about one subject, with its supporting [`Witness`].
///
/// Operational (Category C): never persisted, reconstructed by re-evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct Verdict {
    /// The gradient axis id this verdict answers on (e.g. `"reach"`, `"epistemic"`).
    pub axis: String,
    /// Opaque content reference the verdict is about — a CID string when
    /// content-addressed; `None` when the axis concerns an actor/context, not an artifact.
    pub subject: Option<String>,
    /// The spine answer.
    pub decision: Decision,
    /// Positive evidence backing the decision.
    pub witness: Witness,
    /// Reference to the policy/manifest binding that parameterized the check, if any.
    pub policy_ref: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refer_q() -> ReferQuestion {
        ReferQuestion {
            layer: "community".into(),
            reason: ReferReason::ContestedEvidence,
            note: Some("dispute open".into()),
        }
    }

    /// The exact wire discriminators — the vocabulary another surface (decision.schema.json)
    /// pins to `["permit","refuse","refer"]`.
    #[test]
    fn decision_wire_discriminators() {
        assert_eq!(
            serde_json::to_value(Decision::Permit).unwrap()["type"],
            "permit"
        );
        assert_eq!(
            serde_json::to_value(Decision::Refuse).unwrap()["type"],
            "refuse"
        );
        let refer = serde_json::to_value(Decision::Refer(refer_q())).unwrap();
        assert_eq!(refer["type"], "refer");
        // Refer flattens its ReferQuestion alongside the tag.
        assert_eq!(refer["layer"], "community");
        assert_eq!(refer["reason"], "contested-evidence");
        assert_eq!(refer["note"], "dispute open");
    }

    #[test]
    fn decision_roundtrips() {
        for d in [
            Decision::Permit,
            Decision::Refuse,
            Decision::Refer(refer_q()),
        ] {
            let j = serde_json::to_string(&d).unwrap();
            let back: Decision = serde_json::from_str(&j).unwrap();
            assert_eq!(d, back);
        }
    }

    /// Ceiling law, executable. A `Refer` is structurally distinct from a `Refuse`,
    /// and there is deliberately NO `From`/`Into` path anywhere that maps one to the
    /// other. Conversions INTO the spine map their not-yet-decidable state to `Refer`
    /// (e.g. `ReachVerdict::Pending → Refer`), never to `Refuse`.
    #[test]
    fn refer_never_collapses_to_refuse() {
        let refer = Decision::Refer(refer_q());

        // Pattern-matching a Refer can never land in a Refuse arm.
        let tag = match &refer {
            Decision::Refuse => "refuse",
            Decision::Permit => "permit",
            Decision::Refer(_) => "refer",
        };
        assert_eq!(tag, "refer");

        // Distinct by value and by wire tag.
        assert_ne!(refer, Decision::Refuse);
        assert_ne!(
            serde_json::to_value(&refer).unwrap()["type"],
            serde_json::to_value(Decision::Refuse).unwrap()["type"],
        );
    }

    #[test]
    fn verdict_fields_serialize_camel_case() {
        let v = Verdict {
            axis: "epistemic".into(),
            subject: Some("bafy-subject".into()),
            decision: Decision::Refer(refer_q()),
            witness: Witness {
                checks: vec![CheckWitness {
                    check_id: "contest-ratio".into(),
                    outcome: CheckOutcome::Failed,
                    summary: "dismiss weight over threshold".into(),
                    observed: Some(serde_json::json!({ "ratio": 0.4 })),
                }],
            },
            policy_ref: Some("epr:policy:epistemic:v1".into()),
        };
        let j = serde_json::to_value(&v).unwrap();
        assert!(
            j.get("policyRef").is_some(),
            "policy_ref must serialize as policyRef"
        );
        assert!(j.get("policy_ref").is_none(), "snake_case must not leak");
        assert_eq!(j["witness"]["checks"][0]["checkId"], "contest-ratio");
        assert_eq!(j["witness"]["checks"][0]["outcome"], "failed");
    }
}
