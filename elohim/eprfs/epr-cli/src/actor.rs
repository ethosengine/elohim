//! `epr actor` — register and read the in-flight identity claim of whoever is acting.
//!
//! WHY THIS EXISTS. Attribution in this harness has always been written *about* an agent, by
//! whoever dispatched it, after the run: a trailer roster assembled from memory. That form
//! cannot be corrected by the party it names, cannot be revised mid-run when a session switches
//! personas, and is only as good as the dispatcher's recollection. `epr actor claim` inverts it
//! — the actor registers for itself, while the work is happening — and every downstream leg
//! (`flow note`, `govern`) then has one place to ask "who is acting in this session?" instead of
//! inventing its own answer.
//!
//! CONTRACT, mirroring `govern`. The answer lives in the PAYLOAD, never in the exit code:
//!
//!   exit 0 — the command ran; read the payload (`current` prints `claim: null` when a session
//!            registered nothing, which is a normal answer and not a failure)
//!   exit 2 — the command did NOT run (bad arguments, no HEAD to date a claim against, an
//!            unreadable store)
//!
//! A client must be able to tell "nobody claimed" from "I could not find out", because those
//! demand opposite responses: the first is an answer to record as `unclaimed`, the second is an
//! absence to route on. Collapsing them is how a stamp ends up asserting `unclaimed` every time
//! its sidecar breaks.
//!
//! THE CLAIM NEVER BLOCKS ANYTHING. This leg writes a record and returns; no decision anywhere
//! is gated on a claim existing. That is the floor the honor-system design rests on — an
//! identity claim that could block work would be a credential, and nothing here can verify one.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use elohim_epr_rea::{
    parse_agent_ref, ActorClaim, ActorRecord, ActorStore, FabricError, SidecarActorStore,
};
use eprfs_meta::hex_lower;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::flow::{head_commit_provenance, short_cid};

/// Where an agent's own package definition lives, relative to the repo root. The role segment
/// of the claimed identity names the file; nothing else about the claim touches the filesystem.
const AGENT_PACKAGE_DIR: &str = ".epr-meta/elohim/packages/agents";

/// Errors surfaced by the `actor` command family.
///
/// A local family rather than the crate-wide [`crate::error::Error`], for the same reason
/// `flow` carries [`crate::flow::FlowError`]: this leg owns `elohim-epr-rea` records, and a
/// [`FabricError`] mapped onto `InvalidArguments` would report an integrity failure as a
/// user typo.
#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("fabric: {0}")]
    Fabric(#[from] FabricError),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type ActorResult<T> = std::result::Result<T, ActorError>;

/// Options shared by the actor subcommands (the `flow` family's `--root`/`--json` idiom).
#[derive(Debug)]
struct GlobalOpts {
    root: PathBuf,
    json: bool,
}

/// Entry point invoked by `main` for `epr actor …`.
pub fn run(args: &[String]) -> ActorResult<ExitCode> {
    let Some(sub) = args.first().map(String::as_str) else {
        return Err(ActorError::InvalidArguments(usage()));
    };

    match sub {
        "claim" => {
            let (opts, rest) = parse_global(&args[1..])?;
            let claimed = take_opt(&rest, "--as")?.ok_or_else(|| {
                ActorError::InvalidArguments(
                    "claim needs --as agent:<role>@<model> — an unnamed claim names nobody".into(),
                )
            })?;
            let session = take_opt(&rest, "--session")?.ok_or_else(|| {
                ActorError::InvalidArguments(
                    "claim needs --session <id> — a claim scoped to no run cannot be superseded"
                        .into(),
                )
            })?;
            let outcome = claim(&opts.root, &claimed, &session)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                outcome.render();
            }
            Ok(ExitCode::SUCCESS)
        }
        "current" => {
            let (opts, rest) = parse_global(&args[1..])?;
            let session = take_opt(&rest, "--session")?.ok_or_else(|| {
                ActorError::InvalidArguments("current needs --session <id>".into())
            })?;
            let outcome = current(&opts.root, &session)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                outcome.render();
            }
            Ok(ExitCode::SUCCESS)
        }
        other => Err(ActorError::InvalidArguments(format!(
            "unknown actor subcommand `{other}`\n{}",
            usage()
        ))),
    }
}

// ---------------------------------------------------------------------------
// claim
// ---------------------------------------------------------------------------

/// The prior claim this one displaces — named so a reader of a single outcome can see the
/// switch without walking the sidecar.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SupersededClaim {
    pub claimed: String,
    pub record_cid: String,
}

/// The machine-facing result of one `claim` act (`--json` consumers read this).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimOutcome {
    pub claimed: String,
    pub session: String,
    /// Git HEAD's author date (RFC3339) — the tree the claim was made against.
    pub claimed_at: String,
    /// `sha256:<hex>` of the agent package, or `null` when no package could be read.
    pub definition_cid: Option<String>,
    /// The atom CID of the `ActorRecord::Claim`.
    pub record_cid: String,
    /// `false` when this exact claim was already current for the session and the append was a
    /// no-op.
    pub appended: bool,
    /// The claim that was current for this session beforehand, when it differs from this one.
    pub superseded: Option<SupersededClaim>,
}

impl ClaimOutcome {
    pub fn render(&self) {
        println!(
            "actor   {} → session {}  {}",
            self.claimed,
            self.session,
            short_cid_str(&self.record_cid)
        );
        println!("        claimed at: {}", self.claimed_at);
        match &self.definition_cid {
            Some(cid) => println!("        definition: {cid}"),
            None => println!("        definition: (no package on disk — honest absence)"),
        }
        if let Some(prior) = &self.superseded {
            println!(
                "        supersedes: {} ({})",
                prior.claimed,
                short_cid_str(&prior.record_cid)
            );
        }
        if !self.appended {
            println!("        (already the current claim — no-op)");
        }
    }
}

/// `epr actor claim --as agent:<role>@<model> --session <id>`.
///
/// Two phases, mirroring `flow note`: **Phase 1 resolves everything and appends nothing** — the
/// claimed shape, the tree's date, the package address, the prior claim, and the record's own
/// address — and **Phase 2 performs the single append** only once every resolution succeeded.
pub fn claim(root: &Path, claimed: &str, session: &str) -> ActorResult<ClaimOutcome> {
    // ── Phase 1: resolve. Nothing below this line touches the sidecar until Phase 2. ──

    // Shape first, so a malformed `--as` never even opens the store. The refusal is
    // `elohim-epr-rea`'s own, verbatim: one parser for the shape, in one crate.
    let (role, _model) = parse_agent_ref(claimed)?;
    let session = non_empty(session, "--session")?;

    // A claim is dated by the tree it was made against, never by wall clock — so a tree with no
    // HEAD is refused rather than dated with `now()`. The provenance call yields the git author
    // email too; the steward slot that carries it is the note leg's, not this record's.
    let (_steward, claimed_at) = head_commit_provenance(root).ok_or_else(|| {
        ActorError::InvalidArguments(format!(
            "cannot date a claim in `{}`: git has no HEAD commit to make it against — \
             a claim is dated by the tree it was made against, never by wall clock",
            root.display()
        ))
    })?;

    let definition_cid = definition_cid(root, &role);

    let claim = ActorClaim::new(claimed, session, &claimed_at, definition_cid.clone())?;
    let record = ActorRecord::Claim(claim);
    let record_cid = record.cid()?;

    let mut store = SidecarActorStore::open(root)?;
    let prior = store.current_for(session)?;

    // Idempotence is scoped to "is this ALREADY what's current for the session", not to "does
    // this CID appear anywhere in the log". In an append-only log whose read rule is
    // latest-wins, position carries meaning: re-claiming an identity that was superseded within
    // the same session at the same HEAD is a real act — it makes that identity current again —
    // and skipping it because its CID had been seen before would leave the wrong actor current
    // while reporting success.
    let appended = prior.as_ref().map(|(cid, _)| *cid) != Some(record_cid);
    let superseded = prior
        .filter(|(cid, _)| *cid != record_cid)
        .map(|(cid, claim)| SupersededClaim {
            claimed: claim.claimed.0,
            record_cid: cid.to_string(),
        });

    let outcome = ClaimOutcome {
        claimed: claimed.to_string(),
        session: session.to_string(),
        claimed_at,
        definition_cid,
        record_cid: record_cid.to_string(),
        appended,
        superseded,
    };

    // ── Phase 2: append. One record, or none. ──
    if appended {
        store.append(record)?;
    }
    Ok(outcome)
}

/// Content address of the agent package the role names: `sha256:<hex>` over its RAW BYTES.
///
/// The same `evaluator_identity()` discipline `govern` applies to its own binary, pointed at a
/// persona build instead. It is honest-narrow on purpose — it identifies the DEFINITION this
/// claim was made from, never the instance that made it, so two runs of one persona share it.
///
/// `None` when the file cannot be read, and that is **honest absence** rather than an error:
/// agents legitimately run without a package on disk (a fresh worktree, an unpackaged role),
/// and refusing the whole claim over a missing definition would make the identity plane
/// depend on the packaging plane. A placeholder would be worse still — it would read as a real
/// address and compare unequal to every genuine one.
fn definition_cid(root: &Path, role: &str) -> Option<String> {
    let path = root.join(AGENT_PACKAGE_DIR).join(format!("{role}.json"));
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("sha256:{}", hex_lower(&hasher.finalize())))
}

// ---------------------------------------------------------------------------
// current
// ---------------------------------------------------------------------------

/// The claim `current` found, flattened for readers that do not want to know about records.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentClaim {
    pub claimed: String,
    pub session: String,
    pub claimed_at: String,
    pub definition_cid: Option<String>,
    pub record_cid: String,
}

/// The machine-facing result of one `current` read.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentOutcome {
    pub session: String,
    /// `None` — serialized as an explicit `null`, never an omitted key — when the session
    /// registered nothing. A caller distinguishes "nobody claimed" from "could not read" by the
    /// exit code, so this key must always be present for the first case to be readable at all.
    pub claim: Option<CurrentClaim>,
}

impl CurrentOutcome {
    pub fn render(&self) {
        match &self.claim {
            Some(claim) => {
                println!(
                    "actor   session {} → {}  {}",
                    self.session,
                    claim.claimed,
                    short_cid_str(&claim.record_cid)
                );
                println!("        claimed at: {}", claim.claimed_at);
                if let Some(cid) = &claim.definition_cid {
                    println!("        definition: {cid}");
                }
            }
            None => println!("actor   session {} → (no claim registered)", self.session),
        }
    }
}

/// `epr actor current --session <id>`.
///
/// Reads the LAST claim appended for the session. An absent claim is an ANSWER (exit 0, `claim:
/// null`); only a store that could not be read at all is a failure (exit 2).
pub fn current(root: &Path, session: &str) -> ActorResult<CurrentOutcome> {
    let session = non_empty(session, "--session")?;
    let store = SidecarActorStore::open(root)?;
    let claim = store
        .current_for(session)?
        .map(|(cid, claim)| CurrentClaim {
            claimed: claim.claimed.0,
            session: claim.session,
            claimed_at: claim.claimed_at,
            definition_cid: claim.definition_cid,
            record_cid: cid.to_string(),
        });
    Ok(CurrentOutcome {
        session: session.to_string(),
        claim,
    })
}

// ---------------------------------------------------------------------------
// argument handling
// ---------------------------------------------------------------------------

/// Reject a blank flag value, naming the flag.
fn non_empty<'a>(value: &'a str, flag: &str) -> ActorResult<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ActorError::InvalidArguments(format!(
            "{flag} needs a value — a blank one is not an identifier"
        )));
    }
    Ok(trimmed)
}

/// Pull a `--key <value>` option out of `rest`, erroring on a missing value.
fn take_opt(rest: &[String], key: &str) -> ActorResult<Option<String>> {
    let mut i = 0;
    while i < rest.len() {
        if rest[i] == key {
            let value = rest
                .get(i + 1)
                .ok_or_else(|| ActorError::InvalidArguments(format!("{key} needs a value")))?;
            return Ok(Some(value.clone()));
        }
        i += 1;
    }
    Ok(None)
}

/// Pull `--root` and `--json` out of `args`, refusing any flag this family does not know.
///
/// The unknown-flag refusal matters more here than in a read-only leg: [`take_opt`] scans for a
/// `--key value` pair and ignores what it does not recognise, so a typo'd `--sesion` would
/// otherwise read as "no session given" — and on the `claim` path that is a record written under
/// the wrong scope, or none at all.
fn parse_global(args: &[String]) -> ActorResult<(GlobalOpts, Vec<String>)> {
    let mut root = PathBuf::from(".");
    let mut json = false;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => {
                json = true;
                i += 1;
            }
            "--root" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| ActorError::InvalidArguments("--root needs a path".into()))?;
                root = PathBuf::from(value);
                i += 2;
            }
            "--as" | "--session" => {
                let value = args.get(i + 1).ok_or_else(|| {
                    ActorError::InvalidArguments(format!("{} needs a value", args[i]))
                })?;
                rest.push(args[i].clone());
                rest.push(value.clone());
                i += 2;
            }
            other => {
                return Err(ActorError::InvalidArguments(format!(
                    "unknown actor argument `{other}`"
                )))
            }
        }
    }
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    Ok((GlobalOpts { root, json }, rest))
}

fn short_cid_str(cid: &str) -> String {
    match cid.parse::<cid::Cid>() {
        Ok(parsed) => short_cid(&parsed),
        Err(_) => cid.to_string(),
    }
}

pub fn usage() -> String {
    "usage: epr actor <\n  \
     claim --as agent:<role>@<model> --session <id> [--json] [--root DIR]\n  \
     | current --session <id> [--json] [--root DIR]\n\
     >"
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_flag_is_refused_rather_than_silently_dropped() {
        let args = vec!["--sesion".to_string(), "s1".to_string()];
        let err = parse_global(&args).expect_err("a typo'd flag must never read as absence");
        assert!(err.to_string().contains("--sesion"), "got: {err}");
    }

    #[test]
    fn the_global_parser_keeps_the_family_flags_paired_in_the_remainder() {
        let args = vec![
            "--as".to_string(),
            "agent:scribe@opus-5".to_string(),
            "--json".to_string(),
            "--session".to_string(),
            "s1".to_string(),
        ];
        let (opts, rest) = parse_global(&args).expect("parses");
        assert!(opts.json);
        assert_eq!(
            take_opt(&rest, "--as").unwrap().unwrap(),
            "agent:scribe@opus-5"
        );
        assert_eq!(take_opt(&rest, "--session").unwrap().unwrap(), "s1");
    }

    #[test]
    fn a_flag_value_that_looks_like_a_flag_is_still_taken_as_the_value() {
        // `--as --session` is a user error, but it must surface as a malformed claim rather than
        // as "no --as given", which would be a different (and misleading) refusal.
        let args = vec!["--as".to_string(), "--session".to_string()];
        let (_opts, rest) = parse_global(&args).expect("parses");
        assert_eq!(take_opt(&rest, "--as").unwrap().unwrap(), "--session");
    }

    #[test]
    fn a_blank_value_is_refused_and_names_the_flag() {
        let err = non_empty("  \n ", "--session").expect_err("whitespace is not an identifier");
        assert!(err.to_string().contains("--session"));
        assert_eq!(non_empty("  kept  ", "--session").unwrap(), "kept");
    }

    #[test]
    fn an_unknown_subcommand_names_the_legal_set() {
        let err = run(&["accept".to_string()]).expect_err("a fourth verb must refuse");
        assert!(err.to_string().contains("claim"), "got: {err}");
        assert!(err.to_string().contains("current"), "got: {err}");
    }
}
