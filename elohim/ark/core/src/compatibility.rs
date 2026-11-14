//! Record only the effect of host admission. The contract's diagnostic verdict
//! includes host values and must never be copied into an ark record.

use std::collections::BTreeSet;

use cid::Cid;
use elohim_epr::verdict::{CheckOutcome, CheckWitness, Decision, Verdict, Witness};
use host_contract::{
    admit, names::axis, AxisId, Clause, ClauseSource, Predicate, Provides, Requires,
};
use serde::{Deserialize, Serialize};

/// Features emitted by epr-cli's extract_cpuid::cpu_name whose usable state ark
/// cannot establish. Privileged/MSR modes, device setup, obsolete modes and CET
/// need more than CPUID. AMX also needs per-process Linux XCOMP permission; an
/// envelope snapshot cannot promise that a newly exec'd child has requested it.
/// Never turn these into an absence-based own-build waiver.
pub const UNPROBED_X86: &[&str] = &[
    "msr",
    "ibt",
    "user_shstk",
    "monitor",
    "mwaitx",
    "mpx",
    "smap",
    "smx",
    "vmx",
    "svm",
    "skinit",
    "invpcid",
    "enqcmd",
    "wbnoinvd",
    "amx_bf16",
    "amx_tile",
    "amx_int8",
    "amx_fp16",
    "amx_complex",
    "ia64",
    "lwp",
    "pconfig",
    "sev_es",
    "sev_snp",
    "invlpgb",
    "fred",
    "lkgs",
    "wrmsrns",
    "sgx1",
    "ace_en",
    "phe_en",
    "pmm_en",
    "rng_en",
];

/// Local evidence claim, not a transferable authority or a record deserializer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnProduce {
    produce: Cid,
    executable: Cid,
}

impl OwnProduce {
    /// Only the caller that looked up this executable's local Produce and verified
    /// that THIS device recorded it may construct this claim. A sibling's Produce
    /// or a publisher assertion does not qualify. Core performs no record lookup.
    pub fn from_local_lookup(produce: Cid, executable: Cid) -> Self {
        Self {
            produce,
            executable,
        }
    }
}

/// Debug view of ark's known axes only; never formats the underlying inventory.
/// The contract currently has no axis iterator, so unknown extension axes are omitted.
pub struct HostAxesDebug<'a>(pub &'a Provides);

impl std::fmt::Debug for HostAxesDebug<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let axes = [
            axis::ARCH,
            axis::OS,
            axis::ISA_X86,
            axis::ABI_LIBC,
            axis::ABI_LOADER,
            axis::KERNEL_LINUX,
        ];
        f.debug_set()
            .entries(axes.into_iter().filter(|name| {
                let id = AxisId((*name).into());
                self.0.observed(&id).is_some() || self.0.is_unobserved(&id)
            }))
            .finish()
    }
}

fn unprobed(clause: &Clause) -> bool {
    clause.axis.0 == axis::ISA_X86
        && match &clause.predicate {
            Predicate::Subset(names) | Predicate::Present(names) => names
                .iter()
                .any(|name| UNPROBED_X86.contains(&name.as_str())),
            Predicate::Equal(name) => UNPROBED_X86.contains(&name.as_str()),
            Predicate::AtLeast(_) => false,
        }
}

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
    /// This device's Produce CID, present only when the whole admission permits
    /// under the own-build rule. Never a host inventory or a bare boolean.
    pub own_produce: Option<Cid>,
}

/// Admit independently resolved artifact requirements. No process authority is
/// conferred. Only failed, observed, extracted x86 ISA clauses can be waived;
/// unknown and unobserved remain refusals even for this device's own builds.
pub fn admit_artifact(
    executable: Cid,
    requires: Option<&Requires>,
    provides: &Provides,
    own_produce: Option<&OwnProduce>,
    withdrawn_artifacts: &BTreeSet<Cid>,
) -> HostAdmission {
    let own_build = own_produce.is_some_and(|claim| claim.executable == executable)
        && !withdrawn_artifacts.contains(&executable);
    let (mut verdict, own_build_rule) = judge_requirements(requires, provides, own_build);
    verdict.subject = Some(executable.to_string());
    let own_produce = own_produce
        .filter(|_| !own_build_rule.is_empty())
        .map(|p| p.produce);
    if let Some(produce) = own_produce {
        for check in &mut verdict.witness.checks {
            if check.summary == "permit-under-own-build-rule" {
                if let Some(serde_json::Value::Object(evidence)) = &mut check.observed {
                    evidence.insert("ownProduce".into(), serde_json::json!(produce));
                }
            }
        }
    }
    HostAdmission {
        executable,
        verdict,
        own_build_rule,
        own_produce,
    }
}

/// Judge a requirement as recorded, against the observed host. The own-build
/// rule never applies here: it needs a verified local Produce (`OwnProduce`),
/// which a bare requirement document cannot supply. No artifact identity or
/// host inventory is fabricated.
#[must_use]
pub fn admit_recorded_requirements(requires: Option<&Requires>, provides: &Provides) -> Verdict {
    judge_requirements(requires, provides, false).0
}

/// Shared judgement; `own_build` is derived only from a verified `OwnProduce`
/// in `admit_artifact`, so it is never an externally assertable flag.
fn judge_requirements(
    requires: Option<&Requires>,
    provides: &Provides,
    own_build: bool,
) -> (Verdict, Vec<Clause>) {
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
            let passed = !unprobed(clause)
                && inserted
                && admit(&single, provides).decision == Decision::Permit;
            let reason = if unprobed(clause) || provides.is_unobserved(&clause.axis) {
                "unobserved"
            } else if provides.observed(&clause.axis).is_none() {
                "unknown-axis"
            } else {
                "unmet"
            };
            let waiver = !passed
                && reason == "unmet"
                && own_build
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
        for required_axis in [axis::ARCH, axis::OS] {
            if !requires
                .clauses()
                .any(|clause| clause.axis().0 == required_axis)
            {
                checks.push(CheckWitness {
                    check_id: format!("host-compatibility:{required_axis}"),
                    outcome: CheckOutcome::Failed,
                    summary: "missing-requirement".into(),
                    observed: None,
                });
            }
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
                if let Some(serde_json::Value::Object(evidence)) = &mut check.observed {
                    evidence.remove("ownProduce");
                }
            }
        }
    }
    (
        Verdict {
            axis: "host-compatibility".into(),
            subject: None,
            decision: if permitted {
                Decision::Permit
            } else {
                Decision::Refuse
            },
            witness: Witness { checks },
            policy_ref: None,
        },
        waived,
    )
}
