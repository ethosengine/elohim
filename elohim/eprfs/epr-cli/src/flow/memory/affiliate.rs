//! `epr flow memory affiliate` — the in-band act that adds, changes or withdraws one member of a
//! collective, sponsored by the acting session's Steward.
//!
//! The network may be generous, but participants decide collectively: nobody joins, changes role
//! or leaves a collective by a raw file edit. The sponsor is the session's CLAIMED participant
//! (read from the actor sidecar, never inferred, never an email), and the line is admitted only
//! if the same sponsorship rule the reader folds with ([`Fold::admit`]) admits it now. One line
//! is appended — signed over its record CID by this device when the device is enrolled for the
//! sponsor, else carrying the honest literal `unsigned`.
use std::io::Write;
use std::path::Path;

use elohim_epr_rea::{parse_participant_ref, ActorStore, ParticipantRef, SidecarActorStore};
use eprfs_agent::memory::{Affiliation, AffiliationStanding, MemberKind, MembershipRole};
use serde_json::{json, Value};

use super::validation::{
    affiliation_line_signed, parse_affiliation_line, Fold, Reader, AFFILIATIONS_PATH,
};
use super::{refused, FlowResult, Options};

const ACTORS_PATH: &str = ".eprfs/status/actors.jsonl";

fn member_kind(value: &str) -> FlowResult<MemberKind> {
    match value {
        "person" => Ok(MemberKind::Person),
        "collective" => Ok(MemberKind::Collective),
        "elohim-agent" => Ok(MemberKind::ElohimAgent),
        other => Err(refused(format!(
            "--kind `{other}` is not person|collective|elohim-agent"
        ))),
    }
}

fn role(value: &str) -> FlowResult<MembershipRole> {
    match value {
        "steward" => Ok(MembershipRole::Steward),
        "contributor" => Ok(MembershipRole::Contributor),
        "observer" => Ok(MembershipRole::Observer),
        other => Err(refused(format!(
            "--role `{other}` is not steward|contributor|observer"
        ))),
    }
}

fn standing(value: Option<&str>) -> FlowResult<AffiliationStanding> {
    match value {
        None | Some("standing") => Ok(AffiliationStanding::Standing),
        Some("fixture") => Ok(AffiliationStanding::Fixture),
        Some(other) => Err(refused(format!(
            "--standing `{other}` is not fixture (omit it for a standing member)"
        ))),
    }
}

/// The one affiliation act. Resolves everything first and appends only when every check passed.
pub fn run(root: &Path, opts: &Options) -> FlowResult<Value> {
    let session = opts.session.ok_or_else(|| {
        refused("affiliate needs --session <id>: the sponsor is that session's claimed participant")
    })?;
    let member = opts
        .member
        .ok_or_else(|| refused("affiliate needs --member <participant ref>"))?;

    // The sponsor: the session's claim, exactly as every other epr leg reads it.
    let claim = if root.join(ACTORS_PATH).exists() {
        SidecarActorStore::open(root)?
            .current_for(session)?
            .map(|(_, claim)| claim)
    } else {
        None
    };
    let claim = claim.ok_or_else(|| {
        refused(format!(
            "session {session} has no actor claim: the sponsor is a claimed participant, never \
             inferred — `epr actor claim --as <participant> --session {session}` first"
        ))
    })?;
    let sponsor = claim.claimed.0.clone();
    let declaration = claim.collective_of_record().to_string();

    let mut reader = Reader::new(root)?;
    let governance = reader.governance(&declaration)?;
    let (_, date) = crate::flow::head_commit_provenance(root).ok_or_else(|| {
        refused("cannot date an affiliation: git has no HEAD commit to make it against")
    })?;
    let prior = governance
        .affiliations
        .iter()
        .find(|(_, a)| a.member == member)
        .map(|(_, a)| a);

    let record = if opts.withdraw {
        let prior = prior.filter(|a| a.withdrawn.is_none()).ok_or_else(|| {
            refused(format!(
                "{member} holds no standing affiliation in {} to withdraw",
                governance.declaration.id
            ))
        })?;
        if opts
            .kind
            .map(member_kind)
            .transpose()?
            .is_some_and(|k| k != prior.member_kind)
            || opts
                .role
                .map(role)
                .transpose()?
                .is_some_and(|r| r != prior.role)
            || opts.standing.is_some()
            || opts.acts_for.is_some()
        {
            return Err(refused(
                "--withdraw ends the member's current affiliation as it stands; it takes no \
                 --standing or --acts-for, and a --kind or --role must match the current line",
            ));
        }
        Affiliation {
            version: 1,
            collective: governance.reference.clone(),
            sponsor: Some(sponsor.clone()),
            withdrawn: Some(date),
            ..prior.clone()
        }
    } else {
        let kind =
            member_kind(opts.kind.ok_or_else(|| {
                refused("affiliate needs --kind person|collective|elohim-agent")
            })?)?;
        let role = role(
            opts.role
                .ok_or_else(|| refused("affiliate needs --role steward|contributor|observer"))?,
        )?;
        let standing = standing(opts.standing)?;
        let acts_for = opts
            .acts_for
            .map(str::to_string)
            .or_else(|| prior.and_then(|a| a.acts_for.clone()));
        if let Some(prior) = prior.filter(|a| a.withdrawn.is_none()) {
            if prior.member_kind == kind
                && prior.role == role
                && prior.standing == standing
                && prior.acts_for == acts_for
            {
                return Err(refused(format!(
                    "{member} is already a {role:?} of {} exactly so; nothing to append",
                    governance.declaration.id
                )));
            }
        }
        Affiliation {
            version: 1,
            collective: governance.reference.clone(),
            member: member.to_string(),
            member_kind: kind,
            role,
            sponsor: Some(sponsor.clone()),
            acts_for,
            standing,
            since: date,
            withdrawn: None,
        }
    };

    // The same rule the reader folds with — refused here is refused there.
    let sponsor_line = Fold::from_governance(&governance)
        .admit(&record)
        .map_err(|reason| {
            refused(format!(
                "{sponsor} cannot sponsor this line in {}: {reason}",
                governance.declaration.id
            ))
        })?;

    // Signed only by a device enrolled for a HUMAN sponsor; an agent has no device.
    let device = opts.device.filter(|key| {
        matches!(parse_participant_ref(&sponsor), Ok(ParticipantRef::Human { ref handle })
            if crate::actor::device_enrolled(root, handle, &key.did_key()))
    });
    let line = affiliation_line_signed(&record, device)?;
    let parsed = parse_affiliation_line(&line).map_err(refused)?;

    // Append exactly one line, against the declaration the checks read.
    reader.unchanged(&governance.reference)?;
    let path = root.join(AFFILIATIONS_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let needs_newline = std::fs::read(&path)
        .map(|bytes| bytes.last().is_some_and(|b| *b != b'\n'))
        .unwrap_or(false);
    let mut bytes = Vec::new();
    if needs_newline {
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(line.as_bytes());
    bytes.push(b'\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(&bytes)?;

    let after = Reader::new(root)?.governance(&declaration);
    let signature = match &parsed.signature {
        super::validation::LineSignature::Signed { signer, .. } => {
            json!({"status":"signed","signer":signer})
        }
        super::validation::LineSignature::Unsigned => json!({"status":"unsigned"}),
    };
    Ok(json!({
        "operation":"affiliate",
        "appended":{"cid":parsed.cid,"record":parsed.record},
        "sponsor":sponsor,
        "sponsorLine":sponsor_line,
        "signature":signature,
        "collectiveOfRecord":{"path":governance.reference.path,"id":governance.declaration.id},
        "stewards":after.as_ref().map_or(Value::Null, |g| g.steward_report()),
        "sidecar":AFFILIATIONS_PATH,
        "standing":"Local affiliation sponsored by a Steward on record; the pre-image of Qahal \
            Membership, not network membership."
    }))
}
