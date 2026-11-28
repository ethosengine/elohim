//! Native carry-grant Commitments and their read-only verifier.
//!
//! Issuance appends one existing REA `Commitment`; verification derives a structured answer from
//! the flow store, Brit's signed covenant note, and the durable session envelope.
//! The verifier has no writer. Slice 1b can consume its result to add revocation and observations
//! without changing any check here.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

use chrono::{DateTime, Utc};
use cid::Cid;
use elohim_epr_rea::{
    decode_flow_records, AgentRef, Commitment, CommitmentState, FlowRecord, FlowStore, ReaVerb,
    ResourceSpec, SidecarFlowStore,
};
use serde::Serialize;

use crate::covenant::{grant_clause_status, grant_is_revoked, GrantClause, GrantClauseStatus};
use crate::{device_key, session_envelope};

use crate::flow::{intend, repository_agent, repository_scope, FlowError, FlowResult};

const GRANT_CLASS: &str = "carry-grant@1";
const RUNNER_PREFIX: &str = "runner:";
const PROJECT_PREFIX: &str = "project:";
const REF_PREFIX: &str = "ref:";
const WORDS_PREFIX: &str = "words:";

/// Append the one native Commitment named by a verified grantsCarry clause.
pub(crate) fn write_commitment_once(root: &Path, clause: &GrantClause) -> FlowResult<String> {
    intend::served_store_guard(root)?;
    let receiver = repository_agent(root)?;
    let scope = repository_scope(root)?;
    let mut classified_as = vec![
        GRANT_CLASS.to_string(),
        format!("{RUNNER_PREFIX}{}", clause.runner),
    ];
    classified_as.extend(
        clause
            .projects
            .iter()
            .map(|project| format!("{PROJECT_PREFIX}{project}")),
    );
    classified_as.extend(
        clause
            .refs
            .iter()
            .map(|branch| format!("{REF_PREFIX}{branch}")),
    );
    classified_as.push(format!("{WORDS_PREFIX}{}", clause.cid));
    let commitment = FlowRecord::Commitment(Commitment {
        action: ReaVerb::Use,
        provider: AgentRef(clause.party.clone()),
        receiver,
        resource_spec: ResourceSpec {
            quantity: None,
            classified_as,
        },
        in_scope_of: scope,
        valid_from: Some(clause.valid_from.to_rfc3339()),
        valid_until: Some(clause.valid_until.to_rfc3339()),
        state: CommitmentState::Active,
        satisfies: Vec::new(),
        bound: None,
    });
    let commitment_cid = commitment.cid()?;
    let mut store = SidecarFlowStore::open(root)?.transaction()?;
    let records = store.records()?;
    if let Some(existing) = commitment_for_clause_in(&records, &clause.cid) {
        return Err(FlowError::InvalidArguments(format!(
            "grantsCarry clause {} already has Commitment {existing}",
            clause.cid
        )));
    }
    store.append(commitment)?;
    Ok(commitment_cid.to_string())
}

/// The Commitment, if any, whose resource classification names this clause.
pub(crate) fn commitment_for_clause(root: &Path, clause: &str) -> FlowResult<Option<String>> {
    Ok(commitment_for_clause_in(&read_records(root)?, clause))
}

fn commitment_for_clause_in(records: &[(Cid, FlowRecord)], clause: &str) -> Option<String> {
    let slot = format!("{WORDS_PREFIX}{clause}");
    records.iter().find_map(|(cid, record)| match record {
        FlowRecord::Commitment(commitment)
            if commitment.resource_spec.classified_as.contains(&slot) =>
        {
            Some(cid.to_string())
        }
        _ => None,
    })
}

/// Which identifier form selected a verification.
pub(crate) enum VerifyTarget<'a> {
    Commitment(&'a str),
    Clause(&'a str),
}

/// One named audit result. Order is the audit order and therefore the text result's first failure.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GrantCheck {
    pub check: &'static str,
    pub passed: bool,
    pub detail: String,
}

/// Complete read-only result; slice 1b can consume this without changing the checks.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GrantVerification {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clause: Option<String>,
    pub checks: Vec<GrantCheck>,
}

impl GrantVerification {
    pub(crate) fn verified(&self) -> bool {
        self.status == "verified"
    }

    pub(crate) fn render(&self) {
        match self.status {
            "verified" => println!("verified"),
            "orphan" => println!("orphan"),
            _ => println!(
                "failed: {}",
                self.checks
                    .iter()
                    .find(|check| !check.passed)
                    .map_or("unknown check", |check| check.check)
            ),
        }
    }

    pub(crate) fn exit_code(&self) -> ExitCode {
        if self.verified() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

/// Verify one carry grant from local durable records without writing any record or observation.
pub(crate) fn verify_grant(root: &Path, target: VerifyTarget<'_>) -> FlowResult<GrantVerification> {
    let records = read_records(root)?;
    let (commitment_cid, commitment) = match target {
        VerifyTarget::Commitment(raw) => {
            let cid: Cid = raw.parse().map_err(|_| {
                FlowError::InvalidArguments(format!(
                    "verify-grant `{raw}` is not a content identifier"
                ))
            })?;
            let commitment = records.iter().find_map(|(candidate, record)| {
                (*candidate == cid).then(|| match record {
                    FlowRecord::Commitment(commitment) => Some(commitment.clone()),
                    _ => None,
                })?
            });
            (raw.to_string(), commitment)
        }
        VerifyTarget::Clause(clause) => {
            let Some(commitment_cid) = commitment_for_clause_in(&records, clause) else {
                if matches!(
                    grant_clause_status(root, clause),
                    GrantClauseStatus::Verified(_)
                ) {
                    return Ok(GrantVerification {
                        status: "orphan",
                        commitment: None,
                        clause: Some(clause.to_string()),
                        checks: Vec::new(),
                    });
                }
                return verify_missing_clause_commitment(root, clause);
            };
            let commitment = records.iter().find_map(|(cid, record)| {
                (cid.to_string() == commitment_cid).then(|| match record {
                    FlowRecord::Commitment(commitment) => Some(commitment.clone()),
                    _ => None,
                })?
            });
            (commitment_cid, commitment)
        }
    };

    let words = commitment
        .as_ref()
        .and_then(|commitment| one_slot(&commitment.resource_spec.classified_as, WORDS_PREFIX));
    let clause_status = words.as_deref().map_or_else(
        || GrantClauseStatus::Unavailable("Commitment names no single words: clause CID".into()),
        |cid| grant_clause_status(root, cid),
    );
    let current_device = current_device();
    let (envelope, runtime, revoked) = clause_identity(&clause_status).map_or_else(
        || {
            (
                Err("verified clause names no session".into()),
                Err("verified clause names no session".into()),
                Err("verified clause names no CID".into()),
            )
        },
        |clause| {
            let envelope_runtime =
                session_envelope::envelope_runtime_on_this_device(&clause.session);
            (
                envelope_runtime
                    .as_ref()
                    .map(|runtime| runtime.is_some())
                    .map_err(Clone::clone),
                envelope_runtime,
                grant_is_revoked(root, &clause.cid),
            )
        },
    );
    let mut result = audit(AuditInput {
        commitment_cid: &commitment_cid,
        commitment: commitment.as_ref(),
        words: words.as_deref(),
        clause_status: &clause_status,
        envelope,
        runtime,
        current_device,
        revoked,
        now: Utc::now(),
    });
    result.commitment = Some(commitment_cid);
    result.clause = words;
    Ok(result)
}

fn verify_missing_clause_commitment(
    root: &Path,
    clause_cid: &str,
) -> FlowResult<GrantVerification> {
    let clause_status = grant_clause_status(root, clause_cid);
    let (envelope, runtime, revoked) = clause_identity(&clause_status).map_or_else(
        || {
            (
                Err("verified clause names no session".into()),
                Err("verified clause names no session".into()),
                Err("verified clause names no CID".into()),
            )
        },
        |clause| {
            let envelope_runtime =
                session_envelope::envelope_runtime_on_this_device(&clause.session);
            (
                envelope_runtime
                    .as_ref()
                    .map(|runtime| runtime.is_some())
                    .map_err(Clone::clone),
                envelope_runtime,
                grant_is_revoked(root, &clause.cid),
            )
        },
    );
    let mut result = audit(AuditInput {
        commitment_cid: "no Commitment names this clause",
        commitment: None,
        words: Some(clause_cid),
        clause_status: &clause_status,
        envelope,
        runtime,
        current_device: current_device(),
        revoked,
        now: Utc::now(),
    });
    result.clause = Some(clause_cid.to_string());
    Ok(result)
}

/// Read the flow log as bytes. Verification must not open the sidecar store because opening it
/// advances its verified-prefix cache; slice 1a's verifier is intentionally side-effect free.
fn read_records(root: &Path) -> FlowResult<Vec<(Cid, FlowRecord)>> {
    let path = root.join(".eprfs/status/flows.jsonl");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(FlowError::Read { path, source }),
    };
    Ok(decode_flow_records(&text)?)
}

struct AuditInput<'a> {
    commitment_cid: &'a str,
    commitment: Option<&'a Commitment>,
    words: Option<&'a str>,
    clause_status: &'a GrantClauseStatus,
    envelope: Result<bool, String>,
    runtime: Result<Option<String>, String>,
    current_device: Result<String, String>,
    revoked: Result<bool, String>,
    now: DateTime<Utc>,
}

fn audit(input: AuditInput<'_>) -> GrantVerification {
    let mut checks = Vec::new();
    let push = |checks: &mut Vec<GrantCheck>, check, passed, detail: String| {
        checks.push(GrantCheck {
            check,
            passed,
            detail,
        });
    };
    push(
        &mut checks,
        "commitment exists",
        input.commitment.is_some(),
        input.commitment.map_or_else(
            || format!("{} is absent or is not a Commitment", input.commitment_cid),
            |_| input.commitment_cid.to_string(),
        ),
    );
    let classified = input.commitment.is_some_and(|commitment| {
        commitment
            .resource_spec
            .classified_as
            .iter()
            .any(|slot| slot == GRANT_CLASS)
    });
    push(
        &mut checks,
        "commitment is carry-grant@1",
        classified,
        if classified {
            GRANT_CLASS.into()
        } else {
            "resourceSpec.classifiedAs does not contain carry-grant@1".into()
        },
    );
    push(
        &mut checks,
        "words clause is named",
        input.words.is_some(),
        input.words.unwrap_or("no single words: entry").to_string(),
    );

    let (exists, verifies, kind, party, clause) = match input.clause_status {
        GrantClauseStatus::Verified(clause) => {
            (true, true, true, Some(clause.party.as_str()), Some(clause))
        }
        GrantClauseStatus::VerifiedOther { party, .. } => {
            (true, true, false, Some(party.as_str()), None)
        }
        GrantClauseStatus::Absent(reason) => {
            push(&mut checks, "clause exists", false, reason.clone());
            (false, false, false, None, None)
        }
        GrantClauseStatus::Unverifiable(reason) => {
            push(&mut checks, "clause exists", true, reason.clone());
            (true, false, false, None, None)
        }
        GrantClauseStatus::Unavailable(reason) => {
            push(&mut checks, "clause exists", false, reason.clone());
            (false, false, false, None, None)
        }
    };
    if !matches!(
        input.clause_status,
        GrantClauseStatus::Absent(_)
            | GrantClauseStatus::Unverifiable(_)
            | GrantClauseStatus::Unavailable(_)
    ) {
        push(
            &mut checks,
            "clause exists",
            exists,
            "Brit found the clause CID".into(),
        );
    }
    push(
        &mut checks,
        "clause verifies",
        verifies,
        if verifies {
            "Brit verified the signed covenant note".into()
        } else {
            "Brit did not return a verified clause".into()
        },
    );
    push(
        &mut checks,
        "clause is grantsCarry",
        kind,
        match input.clause_status {
            GrantClauseStatus::Verified(_) => "grantsCarry".into(),
            GrantClauseStatus::VerifiedOther { kind, reason, .. } => {
                format!("kind={kind}: {reason}")
            }
            _ => "no verified clause kind".into(),
        },
    );
    let provider_matches = input
        .commitment
        .zip(party)
        .is_some_and(|(commitment, party)| commitment.provider.0 == party);
    push(
        &mut checks,
        "provider matches clause party",
        provider_matches,
        match (input.commitment, party) {
            (Some(commitment), Some(party)) => {
                format!("provider={} party={party}", commitment.provider.0)
            }
            _ => "provider or verified clause party is unavailable".into(),
        },
    );

    let envelope_passed = input.envelope.as_ref().is_ok_and(|exists| *exists);
    push(
        &mut checks,
        "clause session has an envelope on this device",
        envelope_passed,
        result_detail(&input.envelope, "envelope exists", "envelope is absent"),
    );
    let moored = input
        .runtime
        .as_ref()
        .is_ok_and(|runtime| runtime.as_deref() == Some("claude-code"));
    push(
        &mut checks,
        "clause session was moored here as claude-code",
        moored,
        match &input.runtime {
            Ok(Some(runtime)) => format!("runtime={runtime}"),
            Ok(None) => "the session's envelope on this device records no runtime".into(),
            Err(reason) => reason.clone(),
        },
    );

    let commitment_runner = input
        .commitment
        .and_then(|commitment| one_slot(&commitment.resource_spec.classified_as, RUNNER_PREFIX));
    let runner_matches = commitment_runner
        .as_deref()
        .zip(clause.map(|clause| clause.runner.as_str()))
        .is_some_and(|(commitment, clause)| commitment == clause);
    push(
        &mut checks,
        "clause runner matches commitment runner",
        runner_matches,
        format!(
            "commitment={} clause={}",
            commitment_runner.as_deref().unwrap_or("none"),
            clause.map_or("none", |clause| clause.runner.as_str())
        ),
    );
    let current_matches = input
        .current_device
        .as_ref()
        .ok()
        .zip(clause.map(|clause| clause.runner.as_str()))
        .is_some_and(|(device, runner)| device == runner);
    push(
        &mut checks,
        "clause runner is this device",
        current_matches,
        match (&input.current_device, clause) {
            (Ok(device), Some(clause)) => {
                format!("device={device} runner={}", clause.runner)
            }
            (Err(reason), _) => reason.clone(),
            _ => "verified grant runner is unavailable".into(),
        },
    );

    set_check(
        &mut checks,
        "projects match as sets",
        input.commitment,
        clause.map(|clause| (PROJECT_PREFIX, clause.projects.as_slice())),
    );
    set_check(
        &mut checks,
        "refs match as sets",
        input.commitment,
        clause.map(|clause| (REF_PREFIX, clause.refs.as_slice())),
    );

    let commitment_until = input
        .commitment
        .and_then(|commitment| commitment.valid_until.as_deref())
        .and_then(|time| DateTime::parse_from_rfc3339(time).ok());
    let validity_equal = commitment_until
        .zip(clause.map(|clause| clause.valid_until))
        .is_some_and(|(commitment, clause)| commitment == clause);
    push(
        &mut checks,
        "validUntil matches",
        validity_equal,
        match (commitment_until, clause) {
            (Some(commitment), Some(clause)) => format!(
                "commitment={} clause={}",
                commitment.to_rfc3339(),
                clause.valid_until.to_rfc3339()
            ),
            _ => "one side has no parseable validUntil".into(),
        },
    );
    let window_valid = clause.is_some_and(|clause| {
        let duration = clause.valid_until - clause.valid_from;
        duration >= chrono::TimeDelta::zero() && duration <= chrono::TimeDelta::hours(24)
    });
    push(
        &mut checks,
        "validity is at most 24 hours after validFrom",
        window_valid,
        clause.map_or_else(
            || "verified grant validity is unavailable".into(),
            |clause| {
                format!(
                    "validFrom={} validUntil={}",
                    clause.valid_from.to_rfc3339(),
                    clause.valid_until.to_rfc3339()
                )
            },
        ),
    );
    let unexpired = clause.is_some_and(|clause| clause.valid_until.with_timezone(&Utc) > input.now);
    push(
        &mut checks,
        "grant has not expired",
        unexpired,
        clause.map_or_else(
            || "verified grant validity is unavailable".into(),
            |clause| format!("validUntil={}", clause.valid_until.to_rfc3339()),
        ),
    );
    let not_revoked = input.revoked.as_ref().is_ok_and(|revoked| !*revoked);
    push(
        &mut checks,
        "no revokesClause names the grant clause",
        not_revoked,
        match input.revoked {
            Ok(false) => "no verified revocation names the clause".into(),
            Ok(true) => "a verified revocation names the clause".into(),
            Err(reason) => reason,
        },
    );

    GrantVerification {
        status: if checks.iter().all(|check| check.passed) {
            "verified"
        } else {
            "failed"
        },
        commitment: None,
        clause: None,
        checks,
    }
}

fn clause_identity(status: &GrantClauseStatus) -> Option<&GrantClause> {
    match status {
        GrantClauseStatus::Verified(clause) => Some(clause),
        _ => None,
    }
}

fn one_slot(slots: &[String], prefix: &str) -> Option<String> {
    let mut values = slots.iter().filter_map(|slot| slot.strip_prefix(prefix));
    let first = values.next()?;
    values.next().is_none().then(|| first.to_string())
}

fn slots(slots: &[String], prefix: &str) -> BTreeSet<String> {
    slots
        .iter()
        .filter_map(|slot| slot.strip_prefix(prefix).map(str::to_string))
        .collect()
}

fn set_check(
    checks: &mut Vec<GrantCheck>,
    name: &'static str,
    commitment: Option<&Commitment>,
    clause: Option<(&str, &[String])>,
) {
    let actual = commitment
        .zip(clause.as_ref())
        .map(|(commitment, (prefix, _))| slots(&commitment.resource_spec.classified_as, prefix));
    let expected = clause.map(|(_, values)| values.iter().cloned().collect::<BTreeSet<_>>());
    let passed = actual
        .as_ref()
        .zip(expected.as_ref())
        .is_some_and(|(a, e)| a == e);
    checks.push(GrantCheck {
        check: name,
        passed,
        detail: match (actual, expected) {
            (Some(actual), Some(expected)) => format!("commitment={actual:?} clause={expected:?}"),
            _ => "commitment or verified grantsCarry clause is unavailable".into(),
        },
    });
}

fn result_detail(result: &Result<bool, String>, yes: &str, no: &str) -> String {
    match result {
        Ok(true) => yes.into(),
        Ok(_) => no.into(),
        Err(reason) => reason.clone(),
    }
}

fn current_device() -> Result<String, String> {
    let path = device_key::resolve_path()
        .map_err(|error| format!("cannot resolve device key: {error}"))?;
    device_key::DeviceKey::load(&path)
        .map(|key| key.did_key())
        .map_err(|error| format!("cannot read device key: {error}"))
}

#[cfg(test)]
mod tests;
