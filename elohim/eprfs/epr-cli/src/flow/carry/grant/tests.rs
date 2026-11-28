use chrono::TimeDelta;

use super::*;

struct Case {
    commitment: Option<Commitment>,
    words: Option<String>,
    clause_status: GrantClauseStatus,
    envelope: Result<bool, String>,
    runtime: Result<Option<String>, String>,
    current_device: Result<String, String>,
    revoked: Result<bool, String>,
    now: DateTime<Utc>,
}

impl Case {
    fn verified() -> Self {
        let valid_from = Utc::now().fixed_offset();
        let valid_until = valid_from + TimeDelta::hours(1);
        let clause = GrantClause {
            cid: "bafygrant".into(),
            session: "grant-session".into(),
            party: "human:matthew".into(),
            runner: "did:key:zRunner".into(),
            projects: vec!["eprfs".into(), "orchestrator".into()],
            refs: vec!["self/*".into()],
            valid_from,
            valid_until,
        };
        let commitment = Commitment {
            action: ReaVerb::Use,
            provider: AgentRef(clause.party.clone()),
            receiver: AgentRef("repo:ethosengine/elohim".into()),
            resource_spec: ResourceSpec {
                quantity: None,
                classified_as: vec![
                    GRANT_CLASS.into(),
                    format!("{RUNNER_PREFIX}{}", clause.runner),
                    format!("{PROJECT_PREFIX}eprfs"),
                    format!("{PROJECT_PREFIX}orchestrator"),
                    format!("{REF_PREFIX}self/*"),
                    format!("{WORDS_PREFIX}{}", clause.cid),
                ],
            },
            in_scope_of: elohim_epr::cid::compute_raw_cid(b"test repository scope"),
            valid_from: Some(valid_from.to_rfc3339()),
            valid_until: Some(valid_until.to_rfc3339()),
            state: CommitmentState::Active,
            satisfies: Vec::new(),
            bound: None,
        };
        Self {
            commitment: Some(commitment),
            words: Some(clause.cid.clone()),
            clause_status: GrantClauseStatus::Verified(clause),
            envelope: Ok(true),
            runtime: Ok(Some("claude-code".into())),
            current_device: Ok("did:key:zRunner".into()),
            revoked: Ok(false),
            now: Utc::now(),
        }
    }

    fn clause_mut(&mut self) -> &mut GrantClause {
        match &mut self.clause_status {
            GrantClauseStatus::Verified(clause) => clause,
            _ => panic!("test case must have a verified grant clause"),
        }
    }

    fn commitment_mut(&mut self) -> &mut Commitment {
        self.commitment.as_mut().expect("test commitment")
    }

    fn audit(&self) -> GrantVerification {
        audit(AuditInput {
            commitment_cid: "bafycommitment",
            commitment: self.commitment.as_ref(),
            words: self.words.as_deref(),
            clause_status: &self.clause_status,
            envelope: self.envelope.clone(),
            runtime: self.runtime.clone(),
            current_device: self.current_device.clone(),
            revoked: self.revoked.clone(),
            now: self.now,
        })
    }
}

fn assert_verified(case: &Case) {
    let result = case.audit();
    assert_eq!(result.status, "verified", "{:?}", result.checks);
    assert!(result.checks.iter().all(|check| check.passed));
}

fn assert_first_failure(case: &Case, expected: &str) {
    let result = case.audit();
    let failed = result
        .checks
        .iter()
        .find(|check| !check.passed)
        .unwrap_or_else(|| panic!("expected `{expected}` to fail: {:?}", result.checks));
    assert_eq!(failed.check, expected, "{:?}", result.checks);
    assert_eq!(result.status, "failed");
}

#[test]
fn verify_grant_accepts_a_fully_consistent_grant() {
    assert_verified(&Case::verified());
}

#[test]
fn verify_grant_checks_commitment_exists() {
    let mut case = Case::verified();
    case.commitment = None;
    assert_first_failure(&case, "commitment exists");
}

#[test]
fn verify_grant_checks_commitment_class() {
    let mut case = Case::verified();
    case.commitment_mut()
        .resource_spec
        .classified_as
        .retain(|slot| slot != GRANT_CLASS);
    assert_first_failure(&case, "commitment is carry-grant@1");
}

#[test]
fn verify_grant_checks_words_clause_is_named() {
    let mut case = Case::verified();
    case.words = None;
    assert_first_failure(&case, "words clause is named");
}

#[test]
fn verify_grant_checks_clause_exists() {
    let mut case = Case::verified();
    case.clause_status = GrantClauseStatus::Absent("absent fixture clause".into());
    assert_first_failure(&case, "clause exists");
}

#[test]
fn verify_grant_checks_clause_verifies() {
    let mut case = Case::verified();
    case.clause_status = GrantClauseStatus::Unverifiable("bad fixture signature".into());
    assert_first_failure(&case, "clause verifies");
}

#[test]
fn verify_grant_checks_clause_kind() {
    let mut case = Case::verified();
    case.clause_status = GrantClauseStatus::VerifiedOther {
        kind: "declaresPurpose".into(),
        party: "human:matthew".into(),
        reason: "fixture clause has the wrong kind".into(),
    };
    assert_first_failure(&case, "clause is grantsCarry");
}

#[test]
fn verify_grant_checks_provider_matches_party() {
    let mut case = Case::verified();
    case.commitment_mut().provider = AgentRef("human:someone-else".into());
    assert_first_failure(&case, "provider matches clause party");
}

#[test]
fn verify_grant_checks_session_envelope() {
    let mut case = Case::verified();
    case.envelope = Ok(false);
    assert_first_failure(&case, "clause session has an envelope on this device");
}

#[test]
fn verify_grant_checks_claude_code_mooring() {
    let mut case = Case::verified();
    case.runtime = Ok(Some("codex".into()));
    assert_first_failure(&case, "clause session was moored here as claude-code");
}

#[test]
fn verify_grant_checks_runner_matches_commitment() {
    let mut case = Case::verified();
    let runner = case
        .commitment_mut()
        .resource_spec
        .classified_as
        .iter_mut()
        .find(|slot| slot.starts_with(RUNNER_PREFIX))
        .expect("runner classification");
    *runner = format!("{RUNNER_PREFIX}did:key:zOther");
    assert_first_failure(&case, "clause runner matches commitment runner");
}

#[test]
fn verify_grant_checks_runner_is_current_device() {
    let mut case = Case::verified();
    case.current_device = Ok("did:key:zOther".into());
    assert_first_failure(&case, "clause runner is this device");
}

#[test]
fn verify_grant_checks_projects_as_sets() {
    let mut case = Case::verified();
    case.clause_mut().projects.push("app".into());
    assert_first_failure(&case, "projects match as sets");
}

#[test]
fn verify_grant_checks_refs_as_sets() {
    let mut case = Case::verified();
    case.clause_mut().refs.push("feature/*".into());
    assert_first_failure(&case, "refs match as sets");
}

#[test]
fn verify_grant_checks_valid_until_matches() {
    let mut case = Case::verified();
    let changed = case.clause_mut().valid_until + TimeDelta::minutes(1);
    case.clause_mut().valid_until = changed;
    assert_first_failure(&case, "validUntil matches");
}

#[test]
fn verify_grant_checks_twenty_four_hour_window() {
    let mut case = Case::verified();
    let changed = case.clause_mut().valid_from + TimeDelta::hours(25);
    case.clause_mut().valid_until = changed;
    case.commitment_mut().valid_until = Some(changed.to_rfc3339());
    assert_first_failure(&case, "validity is at most 24 hours after validFrom");
}

#[test]
fn verify_grant_checks_expiry() {
    let mut case = Case::verified();
    let changed = Utc::now().fixed_offset() - TimeDelta::minutes(1);
    case.clause_mut().valid_from = changed - TimeDelta::hours(1);
    case.clause_mut().valid_until = changed;
    case.commitment_mut().valid_from = Some((changed - TimeDelta::hours(1)).to_rfc3339());
    case.commitment_mut().valid_until = Some(changed.to_rfc3339());
    assert_first_failure(&case, "grant has not expired");
}

#[test]
fn verify_grant_checks_revocation() {
    let mut case = Case::verified();
    case.revoked = Ok(true);
    assert_first_failure(&case, "no revokesClause names the grant clause");
}
