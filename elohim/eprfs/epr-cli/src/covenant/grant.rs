//! `grantsCarry` issuance and the covenant half of grant verification.
//!
//! Brit owns the signed clause and its CID. This module adds the device-local admission rules,
//! then hands the verified clause view to the carry module for the native REA Commitment.

use std::path::Path;

use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use serde_json::{json, Value};

use super::{Args, Brit, Clause, CovResult, GRANTS, REVOKES};
use crate::{device_key, flow};

/// The verified fields `verify-grant` compares with the native Commitment.
#[derive(Debug, Clone)]
pub(crate) struct GrantClause {
    pub cid: String,
    pub session: String,
    pub party: String,
    pub runner: String,
    pub projects: Vec<String>,
    pub refs: Vec<String>,
    pub valid_from: DateTime<FixedOffset>,
    pub valid_until: DateTime<FixedOffset>,
}

impl TryFrom<Clause> for GrantClause {
    type Error = String;

    fn try_from(clause: Clause) -> Result<Self, Self::Error> {
        if clause.kind != GRANTS {
            return Err(format!(
                "clause {} is {}, not grantsCarry",
                clause.cid, clause.kind
            ));
        }
        let runner = clause
            .field("runner")
            .ok_or_else(|| format!("grantsCarry clause {} names no runner", clause.cid))?
            .to_string();
        let valid_until = clause
            .valid_until
            .ok_or_else(|| format!("grantsCarry clause {} carries no validUntil", clause.cid))?;
        let projects = clause.strings("projects");
        let refs = clause.strings("refs");
        Ok(Self {
            cid: clause.cid,
            session: clause.session,
            party: clause.party,
            runner,
            projects,
            refs,
            valid_from: clause.valid_from,
            valid_until,
        })
    }
}

/// What Brit could establish about a clause CID, preserving absence vs unverifiability.
#[derive(Debug, Clone)]
pub(crate) enum GrantClauseStatus {
    Verified(GrantClause),
    VerifiedOther {
        kind: String,
        party: String,
        reason: String,
    },
    Absent(String),
    Unverifiable(String),
    Unavailable(String),
}

pub(super) fn payload(
    root: &Path,
    session: &str,
    brit: &Brit,
    party: &str,
    quote: &str,
    args: &Args,
) -> CovResult<Value> {
    let valid_until = args
        .valid_until
        .as_deref()
        .ok_or("too long: grantsCarry requires --valid-until in the next 24 hours")?;
    let valid_until = DateTime::parse_from_rfc3339(valid_until)
        .map_err(|error| format!("--valid-until is not RFC3339: {error}"))?;
    let now = Utc::now();
    let until_utc = valid_until.with_timezone(&Utc);
    if until_utc <= now || until_utc > now + TimeDelta::hours(24) {
        return Err(
            "too long: grantsCarry --valid-until must be in the future and no more than 24 hours from now"
                .into(),
        );
    }
    if args
        .refs
        .iter()
        .any(|pattern| glob_matches(pattern, "dev") || glob_matches(pattern, "main"))
    {
        return Err(
            "shared branch: a grantsCarry --ref may not match dev or main under '*' matching"
                .into(),
        );
    }
    let earlier = brit.list_kind(GRANTS).map_err(|reason| {
        format!("same words: earlier grantsCarry clauses could not be read: {reason}")
    })?;
    if earlier
        .iter()
        .any(|clause| clause.party == party && clause.quote.trim() == quote.trim())
    {
        return Err(
            "same words: grantsCarry renewal requires a quote not used by an earlier grant of this party on this device"
                .into(),
        );
    }
    let runtime = flow::participate::local_session_runtime(root, session).map_err(|reason| {
        format!("not moored here as claude-code: berth status --json could not be read: {reason}")
    })?;
    if runtime.as_deref() != Some("claude-code") {
        return Err(format!(
            "not moored here as claude-code: session `{session}` has runtime {} in berth status --json",
            runtime.as_deref().unwrap_or("none")
        ));
    }
    if args.projects.is_empty() {
        return Err("grantsCarry needs at least one --project".into());
    }
    if args.refs.is_empty() {
        return Err("grantsCarry needs at least one --ref".into());
    }
    Ok(json!({
        "runner": current_device()?,
        "projects": args.projects,
        "refs": args.refs,
    }))
}

pub(super) fn recorded(root: &Path, stored: &Value) -> CovResult<Value> {
    let clause = GrantClause::try_from(Clause::parse(stored)?)?;
    let commitment = flow::carry::grant::write_commitment_once(root, &clause).map_err(|error| {
        format!(
            "stored grantsCarry clause {} but its Commitment write failed: {error}",
            clause.cid
        )
    })?;
    Ok(json!({
        "status": "recorded",
        "cid": clause.cid,
        "commitment": commitment,
        "clause": stored.get("clause").cloned().unwrap_or(Value::Null),
    }))
}

pub(super) fn repair(root: &Path, brit: &Brit, cid: &str) -> CovResult<Value> {
    let clause = GrantClause::try_from(brit.show(cid)?)?;
    if clause.runner != current_device()? {
        return Err(format!(
            "clause {cid} is not a grantsCarry clause of this device: runner is {}",
            clause.runner
        ));
    }
    if let Some(commitment) =
        flow::carry::grant::commitment_for_clause(root, cid).map_err(|error| error.to_string())?
    {
        return Err(format!(
            "grantsCarry clause {cid} already has Commitment {commitment}"
        ));
    }
    let commitment = flow::carry::grant::write_commitment_once(root, &clause).map_err(|error| {
        format!("stored grantsCarry clause {cid} but its Commitment write failed: {error}")
    })?;
    Ok(json!({
        "status": "commitment-recorded",
        "cid": cid,
        "commitment": commitment,
    }))
}

/// Read one signed clause through Brit's verified/unverifiable boundary.
pub(crate) fn grant_clause_status(root: &Path, cid: &str) -> GrantClauseStatus {
    let brit = match Brit::resolve(root, "") {
        Ok(brit) => brit,
        Err(error) => return GrantClauseStatus::Unavailable(error),
    };
    match brit.show(cid) {
        Ok(clause) if clause.kind == GRANTS => match GrantClause::try_from(clause) {
            Ok(clause) => GrantClauseStatus::Verified(clause),
            Err(error) => GrantClauseStatus::Unverifiable(error),
        },
        Ok(clause) => GrantClauseStatus::VerifiedOther {
            reason: format!("clause {} is {}, not grantsCarry", clause.cid, clause.kind),
            kind: clause.kind,
            party: clause.party,
        },
        Err(error) if error.contains("unverifiable:") || error.contains("held:") => {
            GrantClauseStatus::Unverifiable(error)
        }
        Err(error) if error.contains("no covenant clause with cid") => {
            GrantClauseStatus::Absent(error)
        }
        Err(error) => GrantClauseStatus::Unavailable(error),
    }
}

/// Whether any verified revocation clause on this device names `cid`.
pub(crate) fn grant_is_revoked(root: &Path, cid: &str) -> CovResult<bool> {
    let brit = Brit::resolve(root, "")?;
    Ok(brit
        .list_kind(REVOKES)?
        .iter()
        .any(|clause| clause.field("revokes") == Some(cid)))
}

fn current_device() -> CovResult<String> {
    let path = device_key::resolve_path()
        .map_err(|error| format!("cannot resolve device key: {error}"))?;
    device_key::DeviceKey::load(&path)
        .map(|key| key.did_key())
        .map_err(|error| format!("cannot read device key: {error}"))
}

/// `*` is the only metacharacter and spans any run of characters, including `/`.
pub(crate) fn glob_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let (mut pi, mut vi) = (0, 0);
    let (mut star, mut after_star) = (None, 0);
    while vi < value.len() {
        if pi < pattern.len() && pattern[pi] == value[vi] {
            pi += 1;
            vi += 1;
        } else if pi < pattern.len() && pattern[pi] == b'*' {
            star = Some(pi);
            pi += 1;
            after_star = vi;
        } else if let Some(star_at) = star {
            pi = star_at + 1;
            after_star += 1;
            vi = after_star;
        } else {
            return false;
        }
    }
    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }
    pi == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::glob_matches;

    #[test]
    fn carry_ref_glob_has_the_agreed_star_semantics() {
        assert!(glob_matches("*", "dev"));
        assert!(!glob_matches("self/*", "dev"));
        assert!(glob_matches("d*", "dev"));
        assert!(glob_matches("self/*/x", "self/a/b/x"));
        assert!(glob_matches("literal", "literal"));
        assert!(!glob_matches("literal", "literal/x"));
    }
}
