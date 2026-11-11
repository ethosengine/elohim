use std::collections::BTreeMap;
use std::collections::BTreeSet;

use elohim_epr::verdict::{CheckOutcome, CheckWitness, Decision, Verdict, Witness};
use serde::{Deserialize, Serialize};

use crate::{AxisId, AxisValue, Predicate, Requires};

/// Ephemeral, device-local observations. Reconstruct these at startup and whenever
/// the host changes. This crate neither observes a host nor accepts a host manifest.
///
/// Deliberately implements neither Serialize nor Deserialize. Synthetic vector
/// inputs use a separate test-only wire shape and explicit construction.
///
/// ```compile_fail
/// use host_contract::Provides;
/// fn must_serialize<T: serde::Serialize>() {}
/// must_serialize::<Provides>();
/// ```
///
/// ```compile_fail
/// use host_contract::Provides;
/// fn must_deserialize<T: serde::de::DeserializeOwned>() {}
/// must_deserialize::<Provides>();
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provides {
    observed: BTreeMap<AxisId, AxisValue>,
    unobserved: BTreeSet<AxisId>,
}

impl Provides {
    /// Record an explicit observation, superseding an earlier failed observation.
    pub fn observe(&mut self, axis: AxisId, value: AxisValue) {
        self.unobserved.remove(&axis);
        self.observed.insert(axis, value);
    }

    /// Record an axis the observer knows but could not measure. Invalidates any
    /// previous value rather than retaining a stale capability.
    pub fn mark_unobserved(&mut self, axis: AxisId) {
        self.observed.remove(&axis);
        self.unobserved.insert(axis);
    }

    /// The current observed value, if one exists.
    pub fn observed(&self, axis: &AxisId) -> Option<&AxisValue> {
        self.observed.get(axis)
    }

    /// Whether an axis is recognized but its value is unavailable.
    pub fn is_unobserved(&self, axis: &AxisId) -> bool {
        self.unobserved.contains(axis)
    }
}

/// Typed compatibility failures carried in each failed check's `observed.refusal`.
/// These are local diagnostics, including host values; do not publish the verdict
/// as a device inventory. Consumers disclose only the selected variant or effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Refusal {
    /// An observed value fails the predicate (including a wrong version line/type).
    Unmet {
        /// Required axis.
        axis: AxisId,
        /// Required value with its comparison kind.
        required: Predicate,
        /// Actual host observation.
        host: AxisValue,
        /// Missing features or names for subset/present predicates; empty otherwise.
        missing: BTreeSet<String>,
    },
    /// No observer recognizes this axis.
    UnknownAxis {
        /// Unrecognized axis.
        axis: AxisId,
    },
    /// The observer knows the axis but could not obtain its value.
    Unobserved {
        /// Unmeasured axis.
        axis: AxisId,
    },
}

impl Refusal {
    /// Stable reason for a local diagnostic or a consumer-owned counter.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unmet { .. } => "unmet",
            Self::UnknownAxis { .. } => "unknown-axis",
            Self::Unobserved { .. } => "unobserved",
        }
    }
}

/// Evaluate every effective clause with one witness per clause, in axis order.
///
/// Unknown axes are **Refuse**, not Refer: spec §3.1 rule 3 explicitly says
/// "Unknown means refuse". This is an existing mechanical rule, not a novel
/// situation requiring governance authority. Unobserved axes also fail closed.
/// No clause means no requirement, even when the host knows no axes at all.
///
/// This is compatibility only: it confers no execution grant, reach or capacity.
/// The returned spine is serializable but contains private host diagnostics;
/// callers must keep it local and disclose only its effect (spec §4).
pub fn admit(requires: &Requires, provides: &Provides) -> Verdict {
    let checks = requires
        .clauses()
        .map(|clause| {
            let host = provides.observed(&clause.axis);
            let refusal = match host {
                Some(value) if clause.predicate.satisfied_by(value) => None,
                Some(value) => {
                    let missing = match (&clause.predicate, value) {
                        (
                            Predicate::Subset(required) | Predicate::Present(required),
                            AxisValue::Names(names),
                        ) => required.difference(names).cloned().collect(),
                        _ => BTreeSet::new(),
                    };
                    Some(Refusal::Unmet {
                        axis: clause.axis.clone(),
                        required: clause.predicate.clone(),
                        host: value.clone(),
                        missing,
                    })
                }
                None if provides.is_unobserved(&clause.axis) => Some(Refusal::Unobserved {
                    axis: clause.axis.clone(),
                }),
                None => Some(Refusal::UnknownAxis {
                    axis: clause.axis.clone(),
                }),
            };
            CheckWitness {
                check_id: format!("host-compatibility:{}", clause.axis.0),
                outcome: if refusal.is_some() {
                    CheckOutcome::Failed
                } else {
                    CheckOutcome::Passed
                },
                summary: match &refusal {
                    Some(reason) => format!("{}: {}", clause.axis.0, reason.label()),
                    None => format!("{}: satisfied", clause.axis.0),
                },
                observed: Some(serde_json::json!({
                    "required": clause.predicate, "evidence": clause.evidence,
                    "host": host, "refusal": refusal,
                })),
            }
        })
        .collect::<Vec<_>>();
    let decision = if checks
        .iter()
        .all(|check| check.outcome == CheckOutcome::Passed)
    {
        Decision::Permit
    } else {
        Decision::Refuse
    };
    Verdict {
        axis: "host-compatibility".to_owned(),
        subject: None,
        decision,
        witness: Witness { checks },
        policy_ref: None,
    }
}
