//! Record only the effect of host admission. The contract's diagnostic verdict
//! includes host values and must never be copied into an ark record.

use std::collections::BTreeSet;

use cid::Cid;
use elohim_epr::verdict::{CheckOutcome, CheckWitness, Decision, Verdict, Witness};
use host_contract::{admit, names::axis, Clause, ClauseSource, Provides, Requires};
use serde::{Deserialize, Serialize};

/// Ephemeral compatibility effect, reconstructed from an artifact's requirements
/// and local observations. Safe for a caller to record as fulfilment evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostAdmission {
    /// Exact executable/variant CID, never a platform label.
    pub executable: Cid,
    /// Public effect only: required clauses and reasons, never the host inventory.
    pub verdict: Verdict,
    /// Nonempty only for a permit under the own-build rule; names EVERY waiver.
    pub own_build_rule: Vec<Clause>,
}

/// Admit independently resolved artifact requirements. No process authority is
/// conferred. Only failed, observed, extracted x86 ISA clauses can be waived;
/// unknown and unobserved remain refusals even for this device's own builds.
pub fn admit_artifact(
    executable: Cid,
    requires: Option<&Requires>,
    provides: &Provides,
    own_build: bool,
    withdrawn_artifacts: &BTreeSet<Cid>,
) -> HostAdmission {
    let mut checks = Vec::new();
    let mut waived = Vec::new();
    if let Some(requires) = requires {
        // Check original evidence independently: a satisfied declared floor must
        // not hide an unmet extracted clause, nor may a waiver erase a failed
        // declared/proven floor on the same merged axis.
        for clause in requires.clauses().flat_map(|merged| merged.evidence()) {
            let mut single = Requires::default();
            // A single clause cannot conflict with another clause.
            let inserted = single.insert(clause.clone()).is_ok();
            let passed = inserted && admit(&single, provides).decision == Decision::Permit;
            let reason = if provides.is_unobserved(&clause.axis) {
                "unobserved"
            } else if provides.observed(&clause.axis).is_none() {
                "unknown-axis"
            } else {
                "unmet"
            };
            let waiver = !passed
                && reason == "unmet"
                && own_build
                && !withdrawn_artifacts.contains(&executable)
                && clause.axis.0 == axis::ISA_X86
                && clause.source == ClauseSource::Extracted;
            if waiver {
                waived.push(clause.clone());
            }
            checks.push(CheckWitness {
                check_id: format!("host-compatibility:{}", clause.axis.0),
                outcome: if passed || waiver {
                    CheckOutcome::Passed
                } else {
                    CheckOutcome::Failed
                },
                summary: if passed {
                    "satisfied".into()
                } else if waiver {
                    "permit-under-own-build-rule".into()
                } else {
                    reason.into()
                },
                // Allow-list artifact evidence; never transform or serialize the
                // contract verdict, whose observed field includes private values.
                observed: Some(serde_json::json!({ "required": clause })),
            });
        }
    } else {
        checks.push(CheckWitness {
            check_id: "host-compatibility:env:requires".into(),
            outcome: CheckOutcome::Failed,
            summary: "missing-requirement".into(),
            observed: None,
        });
    }
    let permitted = checks
        .iter()
        .all(|check| check.outcome == CheckOutcome::Passed);
    if !permitted {
        // There is no own-build permit when any non-waivable clause failed.
        waived.clear();
        for check in &mut checks {
            if check.summary == "permit-under-own-build-rule" {
                check.outcome = CheckOutcome::Failed;
                check.summary = "unmet".into();
            }
        }
    }
    HostAdmission {
        executable,
        verdict: Verdict {
            axis: "host-compatibility".into(),
            subject: Some(executable.to_string()),
            decision: if permitted {
                Decision::Permit
            } else {
                Decision::Refuse
            },
            witness: Witness { checks },
            policy_ref: None,
        },
        own_build_rule: waived,
    }
}
