//! Host admission owns the grant; serializable records are evidence only.

use std::collections::BTreeSet;

use ark_core::candidate::{AuthenticatedCandidateContent, CandidateRefusal};
use serde::{Deserialize, Serialize};

#[path = "admission_record.rs"]
mod record;
#[path = "recorded_requirement.rs"]
mod recorded;
use cid::Cid;
use elohim_epr::verdict::{CheckOutcome, CheckWitness, Decision, Verdict, Witness};
use host_contract::{admit, names::axis, Clause, ClauseSource, Predicate, Provides, Requires};
pub use record::{AdmissionRecord, WaivedClause};
pub use recorded::{resolve_recorded, ResolveGap};

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

/// Local evidence minted only by the artifact reader, never caller assertions.
/// ```compile_fail
/// use ark_supervisor::admission::OwnProduce;
/// let forged = OwnProduce { produce: todo!(), executable: todo!() };
/// ```
/// ```compile_fail
/// let forged: ark_supervisor::admission::OwnProduce = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnProduce {
    produce: Cid,
    executable: Cid,
}

/// Requirement bound to an output by the local artifact reader. No public constructor.
/// ```compile_fail
/// use ark_supervisor::admission::RecordedRequirement;
/// let forged = RecordedRequirement { output: todo!(), requires: todo!(), own_produce: None };
/// ```
/// ```compile_fail
/// let forged: ark_supervisor::admission::RecordedRequirement = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug)]
pub struct RecordedRequirement {
    output: Cid,
    requires: Requires,
    own_produce: Option<OwnProduce>,
}

/// Host admission of authenticated content. A future launch path must require
/// `&AdmittedCandidate`, alongside its other authority checks; no launch is wired
/// here. This proves compatibility at observation time, not delegation or freshness.
/// ```compile_fail
/// use ark_supervisor::admission::AdmittedCandidate;
/// let forged = AdmittedCandidate { authenticated: todo!(), record: todo!() };
/// ```
/// ```compile_fail
/// let forged: ark_supervisor::admission::AdmittedCandidate = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug)]
pub struct AdmittedCandidate {
    authenticated: AuthenticatedCandidateContent,
    record: AdmissionRecord,
}

impl AdmittedCandidate {
    pub fn authenticated(&self) -> &AuthenticatedCandidateContent {
        &self.authenticated
    }
    pub fn executable(&self) -> Cid {
        self.record.executable()
    }
    pub fn verdict(&self) -> &Verdict {
        self.record.verdict()
    }
    pub fn own_build_rule(&self) -> &[WaivedClause] {
        self.record.own_build_rule()
    }
    pub fn own_produce(&self) -> Option<Cid> {
        self.record.own_produce()
    }
    pub fn record(&self) -> AdmissionRecord {
        self.record.clone()
    }
}

/// Recordable refusal details, containing requirements but no host inventory.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "detail")]
pub enum PlatformRefusal {
    Refused(Box<AdmissionRecord>),
    RequirementForOtherExecutable { recorded_executable: Cid },
}

/// Combine authenticated content and locally resolved requirements. Missing
/// documents fail in `resolve_recorded`; missing required axes refuse here.
pub fn admit_candidate(
    authenticated: AuthenticatedCandidateContent,
    recorded: RecordedRequirement,
    host: &Provides,
    withdrawn: &BTreeSet<Cid>,
) -> Result<AdmittedCandidate, CandidateRefusal<PlatformRefusal>> {
    if recorded.output != authenticated.content().candidate.executable {
        return Err(CandidateRefusal::WrongPlatform(
            PlatformRefusal::RequirementForOtherExecutable {
                recorded_executable: recorded.output,
            },
        ));
    }
    let record = admit_artifact(&recorded, host, withdrawn);
    if !record.permits() {
        return Err(CandidateRefusal::WrongPlatform(PlatformRefusal::Refused(
            Box::new(record),
        )));
    }
    Ok(AdmittedCandidate {
        authenticated,
        record,
    })
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

/// Required clause in the recordable evidence form; never a host value.
fn waived_clause(clause: &Clause) -> WaivedClause {
    let source = match serde_json::json!(clause.source) {
        serde_json::Value::String(source) => source,
        other => other.to_string(),
    };
    WaivedClause {
        axis: clause.axis.0.clone(),
        predicate: serde_json::json!(clause.predicate),
        source,
        narrowing: clause.narrowing.as_ref().map(|n| serde_json::json!(n)),
    }
}

/// Inspect recorded artifact requirements. Returns evidence only, never a grant.
pub fn admit_artifact(
    recorded: &RecordedRequirement,
    provides: &Provides,
    withdrawn_artifacts: &BTreeSet<Cid>,
) -> AdmissionRecord {
    let executable = recorded.output;
    let requires = Some(&recorded.requires);
    let own_produce = recorded.own_produce.as_ref();
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
    AdmissionRecord {
        executable,
        verdict,
        own_build_rule: own_build_rule.iter().map(waived_clause).collect(),
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
