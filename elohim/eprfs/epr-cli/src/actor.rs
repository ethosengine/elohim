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
//!
//! THE PARTICIPANT PLANE (`witness`, `contest`, `device`). A human's standing identity at this
//! repository node is established by a present agent WITNESSING them — `epr actor witness` — signed
//! from this device's key (`device_key`), never inferred from git, an email or a workspace
//! namespace. The witness is the genesis row of the handle's roster
//! (`.eprfs/status/participants/<handle>.jsonl`, tracked, public material only); a second device
//! joins through the three-command ceremony `device enroll | authorize | bind`, in which both
//! devices sign the same binding row and no key ever crosses. A witness is contestable
//! (`contest`), and the human may always claim for themselves (`claim --as human:<handle>`, now
//! signed by the device too). None of it is a gate: the signature is evidence a later reader can
//! check, and standing only feeds attribution (`standing_on_device`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cid::Cid;
use elohim_epr_rea::{
    parse_acting_participant, parse_participant_ref, record_signing_message, standing_human_pinned,
    verify_binding, ActorClaim, ActorRecord, ActorStore, ActorWitness, FabricError, ParticipantRef,
    ParticipantRow, RecordSignature, Roster, RosterStanding, RosterVia, SidecarActorStore,
    SidecarRoster, SignatureVerifier,
};
use eprfs_meta::hex_lower;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::device_key::{self, DeviceKey};
use crate::flow::{head_commit_provenance, short_cid};

/// The actor sidecar, relative to the repo root — opened only once it is known to exist on the
/// read paths, because [`SidecarActorStore::open`] creates it.
const ACTOR_LOG_REL: &str = ".eprfs/status/actors.jsonl";

/// Where the tracked participant rosters live, one `<handle>.jsonl` per human handle.
const PARTICIPANTS_REL: &str = ".eprfs/status/participants";

/// The `source` value an attribution records when it comes from this device's standing human
/// (ruling R-P1: one new value beside `claim | unclaimed`, never a credential).
pub const CLAIM_SIGNED_SOURCE: &str = "claim-signed";

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

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// A multi-store act failed part-way and left a named row behind; the message names it.
    #[error("orphan: {0}")]
    Orphan(String),
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
                    "claim needs --as agent:<role>@<model> | human:<handle> — an unnamed claim \
                     names nobody"
                        .into(),
                )
            })?;
            let session = take_opt(&rest, "--session")?.ok_or_else(|| {
                ActorError::InvalidArguments(
                    "claim needs --session <id> — a claim scoped to no run cannot be superseded"
                        .into(),
                )
            })?;
            // Only a human claim is signed, and only the human's own act opens (or first mints)
            // the device key. A key that cannot be opened leaves the claim unsigned — the floor.
            let device = match parse_participant_ref(&claimed) {
                Ok(ParticipantRef::Human { .. }) => match DeviceKey::open() {
                    Ok(key) => Some(key),
                    Err(error) => {
                        eprintln!(
                            "note: no device key ({error}) — the human claim is recorded \
                             unsigned; nothing refuses an unsigned claim"
                        );
                        None
                    }
                },
                _ => None,
            };
            let under = take_opt(&rest, "--under")?;
            let outcome = claim_bound(
                &opts.root,
                &claimed,
                &session,
                device.as_ref(),
                under.as_deref(),
            )?;
            print_outcome(opts.json, &outcome, ClaimOutcome::render)
        }
        "current" => {
            let (opts, rest) = parse_global(&args[1..])?;
            let device_only = rest.iter().any(|a| a == "--device");
            let session = take_opt(&rest, "--session")?;
            let key_file = device_key::resolve_path().ok();
            let outcome = match (device_only, session) {
                // `--device`: the device's standing alone, consulting no session at all — the
                // read a hook makes beside an agent's own claim. A session label would be
                // claimable, and a claimed label would hide the device (finding M8).
                (true, None) => device_current(&opts.root, key_file.as_deref()),
                (true, Some(_)) => {
                    return Err(ActorError::InvalidArguments(
                        "current takes --session <id> or --device, not both".into(),
                    ))
                }
                (false, Some(session)) => {
                    current_on_device(&opts.root, &session, key_file.as_deref())?
                }
                (false, None) => {
                    return Err(ActorError::InvalidArguments(
                        "current needs --session <id> (or --device for the device alone)".into(),
                    ))
                }
            };
            print_outcome(opts.json, &outcome, CurrentOutcome::render)
        }
        "witness" => {
            let (opts, rest) = parse_global(&args[1..])?;
            let subject = required(&rest, "--subject", "witness needs --subject human:<handle>")?;
            let witness_as = required(
                &rest,
                "--as",
                "witness needs --as agent:<role>@<model> — the agent who stands behind it",
            )?;
            let session = required(&rest, "--session", "witness needs --session <id>")?;
            let basis = required(
                &rest,
                "--basis",
                "witness needs --basis \"<one line you stand behind>\"",
            )?;
            let again = rest.iter().any(|a| a == "--again");
            let answers = take_opt(&rest, "--answers")?;
            let key = DeviceKey::open()?;
            let outcome = witness_answering(
                &opts.root,
                &WitnessRequest {
                    subject: &subject,
                    witness_as: &witness_as,
                    session: &session,
                    basis: &basis,
                    again,
                    answers: answers.as_deref(),
                },
                &key,
            )?;
            print_outcome(opts.json, &outcome, WitnessOutcome::render)
        }
        "contest" => {
            let (opts, rest) = parse_global(&args[1..])?;
            let subject = required(&rest, "--subject", "contest needs --subject human:<handle>")?;
            let contest_as = required(&rest, "--as", "contest needs --as <participant>")?;
            let session = required(&rest, "--session", "contest needs --session <id>")?;
            let basis = required(
                &rest,
                "--basis",
                "contest needs --basis \"<one line you stand behind>\"",
            )?;
            let key = DeviceKey::open()?;
            let outcome = contest(&opts.root, &subject, &contest_as, &session, &basis, &key)?;
            print_outcome(opts.json, &outcome, ContestOutcome::render)
        }
        "device" => run_device(&args[1..]),
        other => Err(ActorError::InvalidArguments(format!(
            "unknown actor subcommand `{other}`\n{}",
            usage()
        ))),
    }
}

/// `epr actor device enroll | authorize | bind` — the three-command ceremony (ruling R-P8).
///
/// Every verb prints JSON on stdout (the request and the authorization are the messages carried
/// between the two devices), so `--json` is accepted and implied.
fn run_device(args: &[String]) -> ActorResult<ExitCode> {
    let verb = args.first().map(String::as_str).unwrap_or_default();
    match verb {
        "enroll" => {
            let (opts, rest) = parse_global(&args[1..])?;
            let handle = required(&rest, "--handle", "device enroll needs --handle <handle>")?;
            let key = DeviceKey::open()?;
            let request = device_enroll(&opts.root, &handle, &key)?;
            println!("{}", serde_json::to_string_pretty(&request)?);
            eprintln!(
                "enroll  {handle}: carry this request to a device already in the roster and run \
                 `epr actor device authorize '<request>'` there"
            );
            Ok(ExitCode::SUCCESS)
        }
        "authorize" | "bind" => {
            let input = args
                .get(1)
                .filter(|a| !a.starts_with("--"))
                .ok_or_else(|| {
                    ActorError::InvalidArguments(format!(
                        "device {verb} needs a JSON argument or a path to one"
                    ))
                })?;
            let (opts, _rest) = parse_global(&args[2..])?;
            let key = DeviceKey::open()?;
            if verb == "authorize" {
                let authorization = device_authorize(&opts.root, input, &key)?;
                println!("{}", serde_json::to_string_pretty(&authorization)?);
                eprintln!(
                    "authorize  carry this authorization back to the enrolling device and run \
                     `epr actor device bind '<authorization>'` there"
                );
            } else {
                let outcome = device_bind(&opts.root, input, &key)?;
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            }
            Ok(ExitCode::SUCCESS)
        }
        other => Err(ActorError::InvalidArguments(format!(
            "unknown device verb `{other}` — enroll | authorize | bind\n{}",
            usage()
        ))),
    }
}

/// Print an outcome as JSON or through its renderer.
fn print_outcome<T: Serialize>(json: bool, outcome: &T, render: fn(&T)) -> ActorResult<ExitCode> {
    if json {
        println!("{}", serde_json::to_string_pretty(outcome)?);
    } else {
        render(outcome);
    }
    Ok(ExitCode::SUCCESS)
}

/// A required `--key <value>` option, refused with `why` when absent.
fn required(rest: &[String], key: &str, why: &str) -> ActorResult<String> {
    take_opt(rest, key)?.ok_or_else(|| ActorError::InvalidArguments(why.to_string()))
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
    /// The device (`did:key`) whose `Signed` record stands behind this claim — a human claim made
    /// with a device key in hand. Omitted, never `null`, for an agent claim or an unsigned one, so
    /// an agent's payload is byte-for-byte what it always was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer: Option<String>,
    /// The declaration path of the collective of record this claim binds (the root when the
    /// claim named none).
    pub collective: String,
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
        println!("        collective of record: {}", self.collective);
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
        if let Some(signer) = &self.signer {
            println!("        signed by device: {}", short_did(signer));
        }
        if !self.appended {
            println!("        (already the current claim — no-op)");
        }
    }
}

/// `epr actor claim --as agent:<role>@<model> | human:<handle> --session <id>`, unsigned.
///
/// The library floor every internal caller uses: no device key is opened, so a claim made here is
/// exactly the claim this command always made. The CLI's human claim goes through
/// [`claim_with_device`] instead.
pub fn claim(root: &Path, claimed: &str, session: &str) -> ActorResult<ClaimOutcome> {
    claim_with_device(root, claimed, session, None)
}

/// `epr actor claim …`, with this device's key in hand.
///
/// Two phases, mirroring `flow note`: **Phase 1 resolves everything and appends nothing** — the
/// claimed shape, the tree's date, the package address, the prior claim, and the record's own
/// address — and **Phase 2 performs the appends** only once every resolution succeeded: the
/// claim, and — for a HUMAN claim with a device key — a `Signed` record over the claim's CID
/// (ruling R-P4: a second record, never a field, so the claim's own address is unchanged). An
/// agent claim is never signed: an agent's identity is per session and carries no device.
pub fn claim_with_device(
    root: &Path,
    claimed: &str,
    session: &str,
    device: Option<&DeviceKey>,
) -> ActorResult<ClaimOutcome> {
    claim_bound(root, claimed, session, device, None)
}

/// `epr actor claim … --under <path>`: the claim, bound to the collective of record for `path`
/// (its nearest `.epr-meta/collective.json`). Without `--under` the claim is unbound, which every
/// reader takes as bound to the repository-root collective.
pub fn claim_under(
    root: &Path,
    claimed: &str,
    session: &str,
    under: &str,
) -> ActorResult<ClaimOutcome> {
    claim_bound(root, claimed, session, None, Some(under))
}

fn claim_bound(
    root: &Path,
    claimed: &str,
    session: &str,
    device: Option<&DeviceKey>,
    under: Option<&str>,
) -> ActorResult<ClaimOutcome> {
    // ── Phase 1: resolve. Nothing below this line touches the sidecar until Phase 2. ──

    // Shape first, so a malformed `--as` never even opens the store. The refusal is
    // `elohim-epr-rea`'s own, verbatim: one parser for the shape, in one crate. Either
    // participant kind may claim; only an AI agent has a package build to address.
    let participant = parse_acting_participant(claimed)?;
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

    // A human has no build: honest absence by construction, never a lookup that happens to
    // miss. `ActorClaim::new` refuses a definition on a human claim, so this match is what
    // keeps the CLI from ever asserting one.
    let definition_cid = match &participant {
        ParticipantRef::Agent { role, .. } => definition_cid(root, role),
        // Unreachable past `parse_acting_participant`; a service has no build either.
        ParticipantRef::Human { .. } | ParticipantRef::Service { .. } => None,
    };

    let is_human = matches!(participant, ParticipantRef::Human { .. });
    let mut claim = ActorClaim::new(claimed, session, &claimed_at, definition_cid.clone())?;
    if let Some(path) = under {
        let declaration = crate::flow::memory::collective_of_record(root, path)
            .map_err(|e| ActorError::InvalidArguments(e.to_string()))?;
        claim = claim.bound_to(&declaration)?;
    }
    let collective = claim.collective_of_record().to_string();
    let record = ActorRecord::Claim(claim);
    let record_cid = record.cid()?;
    let signed = match device {
        Some(key) if is_human => Some(sign_record(&record_cid, key)?),
        _ => None,
    };

    let mut store = SidecarActorStore::open(root)?.transaction()?;
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
        signer: device.filter(|_| is_human).map(DeviceKey::did_key),
        collective,
    };

    // ── Phase 2: append. The claim (or nothing), then its signature (once). ──
    if appended {
        store.append(record)?;
    }
    if let Some(signed) = signed {
        // ed25519 is deterministic, so a re-signature of the same claim by the same device is
        // the same record: appended once, however many times the claim is repeated.
        let signed_cid = signed.cid()?;
        if !store.records()?.iter().any(|(cid, _)| *cid == signed_cid) {
            store.append(signed)?;
        }
    }
    Ok(outcome)
}

/// The `Signed` record a device appends over one actor record (a claim or a witness).
fn sign_record(record_cid: &Cid, key: &DeviceKey) -> ActorResult<ActorRecord> {
    let signature = key.sign(&record_signing_message(&record_cid.to_string()));
    Ok(ActorRecord::Signed(RecordSignature::new(
        record_cid,
        &key.did_key(),
        &signature,
    )?))
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
    /// The declaration path of the session's collective of record (the root when unbound).
    pub collective: String,
}

/// The machine-facing result of one `current` read.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentOutcome {
    /// The session read, or `null` for a `--device` read, which consults no session.
    pub session: Option<String>,
    /// `None` — serialized as an explicit `null`, never an omitted key — when the session
    /// registered nothing. A caller distinguishes "nobody claimed" from "could not read" by the
    /// exit code, so this key must always be present for the first case to be readable at all.
    pub claim: Option<CurrentClaim>,
    /// When the session registered nothing: the human this DEVICE stands for (ruling R-P6 —
    /// standing is per device, not per session), or `null` when the device is unwitnessed.
    /// Always `null` beside a present claim: the session's own claim is the answer then.
    pub standing: Option<Standing>,
    /// When the device would stand but the roster's chain root differs from the root this
    /// device pinned (ruling R-P15): the handle and both roots. Omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contested: Option<RootContest>,
}

impl CurrentOutcome {
    pub fn render(&self) {
        let session = self.session.as_deref().unwrap_or("(device read)");
        match &self.claim {
            Some(claim) => {
                println!(
                    "actor   session {session} → {}  {}",
                    claim.claimed,
                    short_cid_str(&claim.record_cid)
                );
                println!("        claimed at: {}", claim.claimed_at);
                if let Some(cid) = &claim.definition_cid {
                    println!("        definition: {cid}");
                }
            }
            None => {
                println!("actor   session {session} → (no claim registered)");
                match (&self.standing, &self.contested) {
                    (Some(standing), _) => println!("        {}", standing.describe()),
                    (None, Some(contest)) => {
                        println!("        {CONTESTED_ROOT_LINE}");
                        println!("        {}", contest.describe());
                    }
                    (None, None) => println!("        (unwitnessed)"),
                }
            }
        }
    }
}

/// The line `current` (and the SessionStart hook) prints for a root that differs from the pin.
pub const CONTESTED_ROOT_LINE: &str = "contested (roster root differs from this device's pin)";

/// A handle whose roster root differs from this device's pin — contested, never standing.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RootContest {
    /// `human:<handle>`.
    pub subject: String,
    pub handle: String,
    /// The root this device pinned on its first verified read.
    pub pinned: String,
    /// The root the roster in this checkout verifies to now.
    pub found: String,
    /// Where the pin lives on this device.
    pub pin_path: String,
    /// This device, `did:key:…`.
    pub device: String,
}

impl RootContest {
    fn describe(&self) -> String {
        format!(
            "{}: pinned {} here, the roster roots at {} (pin {})",
            self.subject,
            short_did(&self.pinned),
            short_did(&self.found),
            self.pin_path
        )
    }
}

/// `epr actor current --session <id>`, reading the session's claim only (no device).
///
/// Reads the LAST claim appended for the session. An absent claim is an ANSWER (exit 0, `claim:
/// null`); only a store that could not be read at all is a failure (exit 2).
pub fn current(root: &Path, session: &str) -> ActorResult<CurrentOutcome> {
    current_on_device(root, session, None)
}

/// `epr actor current --session <id>` on this device: the session's claim as always, and — when
/// the session registered none — the standing human of the device whose key lives at `key_file`.
///
/// A missing key file is an unwitnessed device, never a reason to mint one: a read never writes a
/// key. It MAY write one thing: the roster root pin, on this device's first verified read of a
/// roster it is a member of (ruling R-P15).
pub fn current_on_device(
    root: &Path,
    session: &str,
    key_file: Option<&Path>,
) -> ActorResult<CurrentOutcome> {
    let session = non_empty(session, "--session")?;
    let store = SidecarActorStore::open(root)?;
    let claim = store
        .current_for(session)?
        .map(|(cid, claim)| CurrentClaim {
            collective: claim.collective_of_record().to_string(),
            claimed: claim.claimed.0,
            session: claim.session,
            claimed_at: claim.claimed_at,
            definition_cid: claim.definition_cid,
            record_cid: cid.to_string(),
        });
    let state = match (&claim, key_file) {
        (None, Some(key_file)) => device_state(root, key_file),
        _ => DeviceState::Unwitnessed,
    };
    let (standing, contested) = state.split();
    Ok(CurrentOutcome {
        session: Some(session.to_string()),
        claim,
        standing,
        contested,
    })
}

/// `epr actor current --device`: this device's standing alone, consulting no session (M8).
pub fn device_current(root: &Path, key_file: Option<&Path>) -> CurrentOutcome {
    let state = key_file.map_or(DeviceState::Unwitnessed, |k| device_state(root, k));
    let (standing, contested) = state.split();
    CurrentOutcome {
        session: None,
        claim: None,
        standing,
        contested,
    }
}

// ---------------------------------------------------------------------------
// standing — which human this device speaks for
// ---------------------------------------------------------------------------

/// The human a device stands for, and the record that makes it so.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Standing {
    /// `human:<handle>`.
    pub subject: String,
    /// The `<handle>` alone.
    pub handle: String,
    /// CID of the standing record — the `Witness`, or the human's own `Claim`; for a standing
    /// read from the roster alone, the founding record a genesis names or the binding row.
    pub record_cid: String,
    /// The agent that witnessed, or `None` when the standing record is the human's own claim (or
    /// the standing was read from the roster alone, which names no agent).
    pub witnessed_by: Option<String>,
    /// The standing record's own date (the tree it was made against); empty for a standing read
    /// from the roster alone, which carries no dates.
    pub claimed_at: String,
    /// This device, `did:key:…`.
    pub device: String,
    /// Set when the standing was derived from the tracked roster alone — this checkout's actor
    /// sidecar holds none of the device's records (a fresh worktree on a witnessed device,
    /// ruling R-P16). Omitted otherwise, so a records-derived payload is what it always was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roster: Option<RosterBasis>,
}

/// Which roster row a roster-derived [`Standing`] rests on.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RosterBasis {
    /// The row's CID.
    pub row_cid: String,
    /// `chain-root` (the handle's genesis) or `bound` (a binding row).
    pub via: String,
}

impl Standing {
    /// The one line `current` prints for it.
    pub fn describe(&self) -> String {
        let on = self.claimed_at.get(..10).unwrap_or(&self.claimed_at);
        let device = short_did(&self.device);
        match (&self.roster, &self.witnessed_by) {
            (Some(basis), _) => format!(
                "standing {} (by the tracked roster, {} device {device})",
                self.subject, basis.via
            ),
            (None, Some(witness)) => format!(
                "standing {} (witnessed by {witness} on {on}, device {device})",
                self.subject
            ),
            (None, None) => format!(
                "standing {} (claimed by themselves on {on}, device {device})",
                self.subject
            ),
        }
    }

    /// The record's CID, parsed.
    pub fn record(&self) -> Option<Cid> {
        self.record_cid.parse().ok()
    }
}

/// What a device's standing read found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    Standing(Standing),
    /// A roster this device is pinned to now roots elsewhere — contested, never standing.
    RootContested(RootContest),
    Unwitnessed,
}

impl DeviceState {
    fn split(self) -> (Option<Standing>, Option<RootContest>) {
        match self {
            DeviceState::Standing(standing) => (Some(standing), None),
            DeviceState::RootContested(contest) => (None, Some(contest)),
            DeviceState::Unwitnessed => (None, None),
        }
    }
}

/// The standing human of THIS device, resolving the key the way every command does
/// (`ELOHIM_DEVICE_KEY_FILE`, then the config home). The attribution arms read this.
pub fn standing_here(root: &Path) -> Option<Standing> {
    standing_on_device(root, &device_key::resolve_path().ok()?)
}

/// The standing human of the device whose key lives at `key_file` — or `None` (unwitnessed, or
/// contested by a root that differs from this device's pin). See [`device_state`].
pub fn standing_on_device(root: &Path, key_file: &Path) -> Option<Standing> {
    match device_state(root, key_file) {
        DeviceState::Standing(standing) => Some(standing),
        _ => None,
    }
}

/// The device whose key lives at `key_file`, read against every roster under `root` and the
/// actor sidecar.
///
/// Read-only and cheap when there is nothing to find: no roster directory means no key is even
/// opened, and a key file that does not exist is an unwitnessed device (a read never mints a
/// key). Every failure along the way is honest absence, never an error: standing enriches
/// attribution and must never veto it.
///
/// Standing is per DEVICE (R-P16): the actor sidecar's signed records are the richer source when
/// this checkout holds them, and when it holds none of this device's records for a handle, the
/// tracked roster alone decides — the chain root on its genesis, a bound device on its binding.
/// Each roster is first checked against the root this device pinned for the handle (R-P15); the
/// first verified read of a roster this device is a member of writes that pin.
pub fn device_state(root: &Path, key_file: &Path) -> DeviceState {
    let handles = roster_handles(root);
    if handles.is_empty() || !key_file.is_file() {
        return DeviceState::Unwitnessed;
    }
    let Ok(key) = DeviceKey::load(key_file) else {
        return DeviceState::Unwitnessed;
    };
    let records = if root.join(ACTOR_LOG_REL).is_file() {
        match SidecarActorStore::open(root).and_then(|store| store.records()) {
            Ok(records) => records,
            Err(_) => return DeviceState::Unwitnessed,
        }
    } else {
        Vec::new()
    };
    device_state_for(root, &handles, &records, &key.did_key(), Some(key_file))
}

/// [`device_state`] over already-read records. `key_file` locates the device's root pins; `None`
/// reads without pins (and writes none).
fn device_state_for(
    root: &Path,
    handles: &[String],
    records: &[(Cid, ActorRecord)],
    did: &str,
    key_file: Option<&Path>,
) -> DeviceState {
    let verifier = verifier();
    let plain: Vec<ActorRecord> = records.iter().map(|(_, r)| r.clone()).collect();
    let mut best: Option<(usize, Standing)> = None;
    let mut contested: Option<RootContest> = None;
    for handle in handles {
        let Ok(roster) = read_roster(root, handle) else {
            continue;
        };
        let Some(chain_root) = roster.chain_root(&verifier) else {
            continue;
        };
        let pin = match key_file.map(|k| device_key::read_root_pin(k, handle)) {
            Some(Ok(pin)) => pin,
            // An unreadable pin is never read as "unpinned": that would let this read pin anew.
            Some(Err(_)) => continue,
            None => None,
        };
        if let (Some(pin), Some(key_file)) = (&pin, key_file) {
            if !roster.chain_root_matches(pin, &verifier) {
                contested.get_or_insert(RootContest {
                    subject: format!("human:{handle}"),
                    handle: handle.clone(),
                    pinned: pin.clone(),
                    found: chain_root.clone(),
                    pin_path: device_key::root_pin_path(key_file, handle)
                        .display()
                        .to_string(),
                    device: did.to_string(),
                });
                continue;
            }
        }
        if pin.is_none() && roster.members(&verifier).contains(did) {
            if let Some(key_file) = key_file {
                // First verified read of a roster this device belongs to: pin its root. A pin
                // that cannot be written leaves the device unpinned, never unstanding.
                let _ = device_key::write_root_pin(key_file, handle, &chain_root);
            }
        }
        let found = if device_holds_records(records, handle, did) {
            // The epr-rea resolver is the authority on WHETHER the device stands; the index
            // below only names WHICH record it stands on.
            standing_human_pinned(&roster, &plain, did, &verifier, pin.as_deref())
                .and_then(|_| standing_index(&roster, records, did))
                .and_then(|index| Some((index + 1, standing_view(handle, &records[index], did)?)))
        } else {
            roster
                .device_standing_row(did, &verifier)
                .map(|row| (0, roster_standing(handle, row, did)))
        };
        if let Some((rank, standing)) = found {
            if best.as_ref().is_none_or(|(r, _)| rank >= *r) {
                best = Some((rank, standing));
            }
        }
    }
    match (best, contested) {
        (Some((_, standing)), _) => DeviceState::Standing(standing),
        (None, Some(contest)) => DeviceState::RootContested(contest),
        (None, None) => DeviceState::Unwitnessed,
    }
}

/// Whether this checkout's actor sidecar holds any human record of `handle` that `did` verifiably
/// signed — i.e. whether the records, rather than the roster alone, decide the device's standing.
fn device_holds_records(records: &[(Cid, ActorRecord)], handle: &str, did: &str) -> bool {
    let verifier = verifier();
    records.iter().any(|(_, record)| match record {
        ActorRecord::Signed(sig) if sig.signer == did && sig.verify(&verifier) => {
            records.iter().any(|(cid, r)| {
                cid.to_string() == sig.claim_cid && human_handle(r).as_deref() == Some(handle)
            })
        }
        _ => false,
    })
}

fn roster_standing(handle: &str, row: RosterStanding, did: &str) -> Standing {
    Standing {
        subject: format!("human:{handle}"),
        handle: handle.to_string(),
        record_cid: row.record_cid,
        witnessed_by: None,
        claimed_at: String::new(),
        device: did.to_string(),
        roster: Some(RosterBasis {
            row_cid: row.row_cid,
            via: match row.via {
                RosterVia::Genesis => "chain-root",
                RosterVia::Binding => "bound",
            }
            .to_string(),
        }),
    }
}

/// The index of the record `standing_human` stands on: the latest human record of the roster's
/// handle that `did` verifiably signed and that no effective contest names (neither the record
/// nor the device's signature over it). Mirrors `elohim_epr_rea::participants::standing_human`.
fn standing_index(roster: &Roster, records: &[(Cid, ActorRecord)], did: &str) -> Option<usize> {
    let verifier = verifier();
    let contested = roster.contested(&verifier);
    records
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, (cid, record))| {
            if human_handle(record)? != roster.handle() {
                return None;
            }
            let record_cid = cid.to_string();
            let signed_cid = signed_by_device(records, &record_cid, did)?;
            if contested.contains(&record_cid) || contested.contains(&signed_cid) {
                return None;
            }
            Some(index)
        })
}

/// The CID of the latest `Signed` record by `did` over `record_cid` that verifies.
fn signed_by_device(records: &[(Cid, ActorRecord)], record_cid: &str, did: &str) -> Option<String> {
    let verifier = verifier();
    records.iter().rev().find_map(|(cid, record)| match record {
        ActorRecord::Signed(sig)
            if sig.claim_cid == record_cid && sig.signer == did && sig.verify(&verifier) =>
        {
            Some(cid.to_string())
        }
        _ => None,
    })
}

/// The handle a human record speaks about — a witness's subject or a human's own claim.
fn human_handle(record: &ActorRecord) -> Option<String> {
    match record {
        ActorRecord::Witness(witness) => witness.handle().ok(),
        ActorRecord::Claim(claim) => match claim.participant().ok()? {
            ParticipantRef::Human { handle } => Some(handle),
            ParticipantRef::Agent { .. } | ParticipantRef::Service { .. } => None,
        },
        ActorRecord::Signed(_) => None,
    }
}

fn standing_view(handle: &str, (cid, record): &(Cid, ActorRecord), did: &str) -> Option<Standing> {
    let (witnessed_by, claimed_at) = match record {
        ActorRecord::Witness(w) => (Some(w.witness.0.clone()), w.claimed_at.clone()),
        ActorRecord::Claim(c) => (None, c.claimed_at.clone()),
        ActorRecord::Signed(_) => return None,
    };
    Some(Standing {
        subject: format!("human:{handle}"),
        handle: handle.to_string(),
        record_cid: cid.to_string(),
        witnessed_by,
        claimed_at,
        device: did.to_string(),
        roster: None,
    })
}

/// Every handle with a roster file under `root`, sorted. Enrolment scratch (`.enroll-*`) and any
/// file whose stem is not a legal handle are skipped.
fn roster_handles(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.join(PARTICIPANTS_REL)) else {
        return Vec::new();
    };
    let mut handles: Vec<String> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()? != "jsonl" {
                return None;
            }
            let stem = path.file_stem()?.to_str()?.to_string();
            Roster::new(&stem).ok().map(|_| stem)
        })
        .collect();
    handles.sort();
    handles
}

/// Read one handle's roster; an absent file is an empty roster (without creating it).
fn read_roster(root: &Path, handle: &str) -> ActorResult<Roster> {
    if !roster_file(root, handle).is_file() {
        return Ok(Roster::new(handle)?);
    }
    Ok(SidecarRoster::open(root, handle)?.read()?)
}

fn roster_file(root: &Path, handle: &str) -> PathBuf {
    root.join(PARTICIPANTS_REL).join(format!("{handle}.jsonl"))
}

fn roster_rel(handle: &str) -> String {
    format!("{PARTICIPANTS_REL}/{handle}.jsonl")
}

/// The device-key verifier the roster and the records are checked with.
fn verifier() -> impl SignatureVerifier {
    |did: &str, message: &[u8], signature: &[u8]| device_key::verify(did, message, signature)
}

// ---------------------------------------------------------------------------
// witness
// ---------------------------------------------------------------------------

/// What a witness did to the handle's roster.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RosterState {
    /// The handle had no roster; this witness is its genesis and this device its chain root.
    Genesis,
    /// The roster exists and this device is already a member of it.
    Joined,
    /// The roster exists and this device is NOT a member — the witness is recorded and signed,
    /// but confers no standing until the device is bound (`epr actor device enroll`).
    Unbound,
}

impl RosterState {
    fn label(self) -> &'static str {
        match self {
            RosterState::Genesis => "genesis",
            RosterState::Joined => "joined",
            RosterState::Unbound => "unbound — bind this device: epr actor device enroll --handle",
        }
    }
}

/// The machine-facing result of one `witness` act.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WitnessOutcome {
    pub subject: String,
    pub witness: String,
    pub session: String,
    pub basis: String,
    pub claimed_at: String,
    /// CID of the `ActorRecord::Witness`.
    pub record_cid: String,
    /// CID of the device's `Signed` record over it.
    pub signed_cid: String,
    /// This device, `did:key:…`.
    pub device: String,
    /// The roster file, repo-relative.
    pub roster_path: String,
    pub roster: RosterState,
    /// The earlier witness of this subject on this device, when `--again` superseded one.
    pub prior: Option<String>,
    /// The roster `Contest` row this re-witness answers (`--answers`). Omitted when none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answers: Option<String>,
}

impl WitnessOutcome {
    pub fn render(&self) {
        println!(
            "witness {} ← {}  {}",
            self.subject,
            self.witness,
            short_cid_str(&self.record_cid)
        );
        println!("        basis: {}", self.basis);
        println!("        session: {}  at {}", self.session, self.claimed_at);
        println!("        device: {}", short_did(&self.device));
        let handle = self.subject.trim_start_matches("human:");
        match self.roster {
            RosterState::Unbound => println!(
                "        roster: {} ({} {handle})",
                self.roster_path,
                self.roster.label()
            ),
            _ => println!(
                "        roster: {} ({})",
                self.roster_path,
                self.roster.label()
            ),
        }
        if let Some(prior) = &self.prior {
            println!("        re-witnesses: {}", short_cid_str(prior));
        }
        if let Some(contest) = &self.answers {
            println!("        answers contest: {}", short_cid_str(contest));
        }
    }
}

/// The arguments of one `witness` act.
#[derive(Debug, Clone, Copy)]
pub struct WitnessRequest<'a> {
    pub subject: &'a str,
    pub witness_as: &'a str,
    pub session: &'a str,
    pub basis: &'a str,
    /// Re-witness a subject this device already witnessed.
    pub again: bool,
    /// The roster `Contest` row a re-witness after a contest answers (ruling R-P19).
    pub answers: Option<&'a str>,
}

/// `epr actor witness --subject human:<h> --as agent:<role>@<model> --session <id> --basis
/// "<line>" [--again]` — a present agent witnesses a human, signed from this device (ruling R-P9).
/// The library form without `--answers`; see [`witness_answering`].
pub fn witness(
    root: &Path,
    subject: &str,
    witness_as: &str,
    session: &str,
    basis: &str,
    again: bool,
    device: &DeviceKey,
) -> ActorResult<WitnessOutcome> {
    witness_answering(
        root,
        &WitnessRequest {
            subject,
            witness_as,
            session,
            basis,
            again,
            answers: None,
        },
        device,
    )
}

/// `epr actor witness … [--again [--answers <contest cid>]]`.
///
/// **Phase 1** resolves and refuses without touching anything: the witness's shape
/// (`ActorWitness::new` refuses a human `--as`, a non-human subject and an empty basis), the
/// `--answers` address, the tree's date, the record and its signature.
///
/// **Phase 2 is one ordered, locked sequence** (finding M1): the actor sidecar's lock, then the
/// handle's roster lock, both held to the end. Under them: refuse a subject this device already
/// witnessed unless `again` — "already" read from the sidecar AND from the roster (a fresh
/// checkout on a witnessed device has the roster and no records, R-P16); refuse a re-witness
/// that does not answer an open contest of this device's standing, or answers something that is
/// not one (R-P19 — the contest row is already in the roster, so the re-witness postdates it by
/// construction); then append the roster's `Genesis` row FIRST when the handle has no roster,
/// and only then the `Witness` and its `Signed` record. A roster that cannot be read or written
/// therefore leaves no signed witness behind; a sidecar append that fails after a genesis names
/// the orphaned genesis row in its error.
///
/// Deliberate, once per device, by the agent who knows who is present; nothing calls this on its
/// own.
pub fn witness_answering(
    root: &Path,
    request: &WitnessRequest<'_>,
    device: &DeviceKey,
) -> ActorResult<WitnessOutcome> {
    let WitnessRequest {
        subject,
        witness_as,
        session,
        basis,
        again,
        answers,
    } = *request;

    // ── Phase 1: resolve. Shape before date, so a malformed witness never runs git. ──
    let session = non_empty(session, "--session")?;
    let shape = ActorWitness::new(subject, witness_as, session, basis, "shape-check")?;
    if let Some(contest) = answers {
        shape.answering(contest)?;
        if !again {
            return Err(ActorError::InvalidArguments(
                "--answers names the contest a RE-witness answers; pass it with --again".into(),
            ));
        }
    }
    let (_steward, claimed_at) = head_commit_provenance(root).ok_or_else(|| {
        ActorError::InvalidArguments(format!(
            "cannot date a witness in `{}`: git has no HEAD commit to make it against",
            root.display()
        ))
    })?;
    let mut witness = ActorWitness::new(subject, witness_as, session, basis, &claimed_at)?;
    if let Some(contest) = answers {
        witness = witness.answering(contest)?;
    }
    let handle = witness.handle()?;
    let record = ActorRecord::Witness(witness.clone());
    let record_cid = record.cid()?;
    let signed = sign_record(&record_cid, device)?;
    let signed_cid = signed.cid()?;
    let did = device.did_key();

    // ── Phase 2: the actor sidecar's lock, then the roster's — held together to the end. ──
    let mut store = SidecarActorStore::open(root)?.transaction()?;
    let mut roster_tx = SidecarRoster::open(root, &handle)?.transaction()?;
    let records = store.records()?;
    let roster = roster_tx.read()?;
    let verifier = verifier();

    let prior =
        prior_witness_on_device(&records, subject, &did).or_else(|| prior_on_roster(&roster, &did));
    if let Some(prior_cid) = &prior {
        if !again {
            return Err(ActorError::InvalidArguments(format!(
                "{subject} is already witnessed on this device ({}, record {}) — witnessing is \
                 once per device; pass --again to re-witness deliberately",
                short_did(&did),
                short_cid_str(prior_cid)
            )));
        }
    }
    let open = open_contests(&roster, &records, &handle, &did);
    match (answers, open.is_empty()) {
        (None, false) if again => {
            return Err(ActorError::InvalidArguments(format!(
                "{subject}'s standing on this device was contested — a re-witness after a \
                 contest must name the contest it answers: --answers <contest cid> (open: {})",
                open.join(", ")
            )))
        }
        (Some(named), _) if !open.iter().any(|c| c == named) => {
            return Err(ActorError::InvalidArguments(format!(
                "--answers {} is not an open contest of {subject}'s standing on this device{}",
                short_cid_str(named),
                if open.is_empty() {
                    " — nothing here is contested".to_string()
                } else {
                    format!(" — open: {}", open.join(", "))
                }
            )))
        }
        _ => {}
    }

    // The roster first: a genesis that cannot be written leaves no signed witness behind.
    let (state, genesis_cid) = if roster.rows().is_empty() {
        let row = signed_row(
            ParticipantRow::Genesis {
                handle: handle.clone(),
                chain_root: did.clone(),
                record_cid: record_cid.to_string(),
                signer: did.clone(),
                signature: String::new(),
            },
            device,
        )?;
        (RosterState::Genesis, Some(roster_tx.append(row)?))
    } else if roster.members(&verifier).contains(&did) {
        (RosterState::Joined, None)
    } else {
        (RosterState::Unbound, None)
    };

    let appended: ActorResult<()> = (|| {
        if !records.iter().any(|(cid, _)| *cid == record_cid) {
            store.append(record)?;
        }
        if !records.iter().any(|(cid, _)| *cid == signed_cid) {
            store.append(signed)?;
        }
        Ok(())
    })();
    if let Err(error) = appended {
        return Err(match genesis_cid {
            Some(genesis) => ActorError::Orphan(format!(
                "the roster genesis row {genesis} ({}) was appended, but the witness record \
                 {record_cid} it names could not be written to the actor sidecar: {error}",
                roster_rel(&handle)
            )),
            None => error,
        });
    }
    drop(roster_tx);
    drop(store);

    Ok(WitnessOutcome {
        subject: subject.to_string(),
        witness: witness.witness.0,
        session: witness.session,
        basis: witness.basis,
        claimed_at,
        record_cid: record_cid.to_string(),
        signed_cid: signed_cid.to_string(),
        device: did,
        roster_path: roster_rel(&handle),
        roster: state,
        prior: prior.filter(|p| *p != record_cid.to_string()),
        answers: witness.answers,
    })
}

/// The latest witness of `subject` this device verifiably signed, as a CID string.
fn prior_witness_on_device(
    records: &[(Cid, ActorRecord)],
    subject: &str,
    did: &str,
) -> Option<String> {
    let verifier = verifier();
    records.iter().rev().find_map(|(cid, record)| {
        let ActorRecord::Witness(witness) = record else {
            return None;
        };
        if witness.subject.0 != subject {
            return None;
        }
        let cid = cid.to_string();
        records
            .iter()
            .any(|(_, r)| {
                matches!(r, ActorRecord::Signed(sig)
                    if sig.claim_cid == cid && sig.signer == did && sig.verify(&verifier))
            })
            .then_some(cid)
    })
}

/// A device already in the handle's roster has been witnessed (as its chain root) or bound —
/// the record it stands on by the roster alone, contested or not (R-P16: `--again` reads the
/// roster too, so a fresh checkout cannot quietly re-witness).
fn prior_on_roster(roster: &Roster, did: &str) -> Option<String> {
    let verifier = verifier();
    if !roster.members(&verifier).contains(did) {
        return None;
    }
    roster.rows().iter().find_map(|(cid, row)| match row {
        ParticipantRow::Genesis {
            chain_root,
            record_cid,
            ..
        } if chain_root == did => Some(record_cid.clone()),
        ParticipantRow::Binding { controller, .. } if controller == did => Some(cid.to_string()),
        _ => None,
    })
}

/// The effective contests of this device's standing for `handle` that no witness in `records`
/// has answered yet: every admissible, effective `Contest` row naming a human record of the
/// handle this device signed (or its signature over it), or the roster row the device stands on
/// by the roster alone.
fn open_contests(
    roster: &Roster,
    records: &[(Cid, ActorRecord)],
    handle: &str,
    did: &str,
) -> Vec<String> {
    let verifier = verifier();
    let mut targets: Vec<String> = Vec::new();
    for (cid, record) in records {
        if human_handle(record).as_deref() != Some(handle) {
            continue;
        }
        let record_cid = cid.to_string();
        if let Some(signed) = signed_by_device(records, &record_cid, did) {
            targets.push(record_cid);
            targets.push(signed);
        }
    }
    for (cid, row) in roster.rows() {
        match row {
            ParticipantRow::Genesis {
                chain_root,
                record_cid,
                ..
            } if chain_root == did => {
                targets.push(record_cid.clone());
                targets.push(cid.to_string());
            }
            ParticipantRow::Binding { controller, .. } if controller == did => {
                targets.push(cid.to_string());
            }
            _ => {}
        }
    }
    let answered: Vec<&str> = records
        .iter()
        .filter_map(|(_, record)| match record {
            ActorRecord::Witness(w) => w.answers.as_deref(),
            _ => None,
        })
        .collect();
    let mut open: Vec<String> = Vec::new();
    for target in targets {
        for contest in roster.effective_contests_of(&target, &verifier) {
            if !answered.contains(&contest.as_str()) && !open.contains(&contest) {
                open.push(contest);
            }
        }
    }
    open
}

/// Fill a row's own signature slot (`signature` on a genesis or a contest) with `device`'s
/// signature over the row's signing message.
fn signed_row(mut row: ParticipantRow, device: &DeviceKey) -> ActorResult<ParticipantRow> {
    let signature = hex_lower(&device.sign(&row.signing_message()?));
    match &mut row {
        ParticipantRow::Genesis {
            signature: slot, ..
        }
        | ParticipantRow::Contest {
            signature: slot, ..
        } => *slot = signature,
        ParticipantRow::Binding { .. } => {
            return Err(ActorError::InvalidArguments(
                "a binding is signed by both devices, never by one".into(),
            ))
        }
    }
    Ok(row)
}

// ---------------------------------------------------------------------------
// contest
// ---------------------------------------------------------------------------

/// The machine-facing result of one `contest` act.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContestOutcome {
    pub subject: String,
    /// Who contested — reported here, never written into the roster (public material only).
    pub contested_as: String,
    pub session: String,
    pub basis: String,
    /// The standing record the contest names.
    pub target_cid: String,
    /// CID of the appended `Contest` row.
    pub row_cid: String,
    /// The contesting device, `did:key:…` — the row's `by`.
    pub device: String,
    pub roster_path: String,
}

impl ContestOutcome {
    pub fn render(&self) {
        println!(
            "contest {} ✕ {}  by {}",
            self.subject,
            short_cid_str(&self.target_cid),
            self.contested_as
        );
        println!("        basis: {}", self.basis);
        println!("        device: {}", short_did(&self.device));
        println!(
            "        roster: {} (row {})",
            self.roster_path,
            short_cid_str(&self.row_cid)
        );
    }
}

/// `epr actor contest --subject human:<h> --as <participant> --session <id> --basis "<line>"` —
/// an adverse attestation against the record this device currently stands on for the handle,
/// appended to the roster as a `Contest` row signed by this device. The contested record stops
/// conferring standing; a re-witness (a new record) stands again.
///
/// `--as` and `--session` name who contested and in which run; the roster row carries only the
/// device's did:key, because the roster is tracked and holds public material only.
pub fn contest(
    root: &Path,
    subject: &str,
    contest_as: &str,
    session: &str,
    basis: &str,
    device: &DeviceKey,
) -> ActorResult<ContestOutcome> {
    let ParticipantRef::Human { handle } = parse_participant_ref(subject)? else {
        return Err(ActorError::InvalidArguments(format!(
            "contest --subject `{subject}` is not a human participant — only a human's standing \
             is contested here"
        )));
    };
    parse_acting_participant(contest_as)?;
    let session = non_empty(session, "--session")?;
    let basis = basis.trim();
    if basis.is_empty() {
        return Err(ActorError::InvalidArguments(
            "a contest needs a basis — one line the contester stands behind".into(),
        ));
    }

    let did = device.did_key();
    let standing = standing_for_handle(root, &handle, &did, device.path())?.ok_or_else(|| {
        ActorError::InvalidArguments(format!(
            "nothing standing for {subject} on this device ({}) to contest",
            short_did(&did)
        ))
    })?;

    let row = signed_row(
        ParticipantRow::Contest {
            handle: handle.clone(),
            target_cid: standing.record_cid.clone(),
            by: did.clone(),
            basis: basis.to_string(),
            signature: String::new(),
        },
        device,
    )?;
    let row_cid = SidecarRoster::open(root, &handle)?.append(row)?;
    Ok(ContestOutcome {
        subject: subject.to_string(),
        contested_as: contest_as.to_string(),
        session: session.to_string(),
        basis: basis.to_string(),
        target_cid: standing.record_cid,
        row_cid: row_cid.to_string(),
        device: did,
        roster_path: roster_rel(&handle),
    })
}

/// The standing record of `did` for one handle, reading the stores without creating them (the
/// roster alone decides when this checkout's sidecar holds none of the device's records).
fn standing_for_handle(
    root: &Path,
    handle: &str,
    did: &str,
    key_file: &Path,
) -> ActorResult<Option<Standing>> {
    if !roster_file(root, handle).is_file() {
        return Ok(None);
    }
    let records = if root.join(ACTOR_LOG_REL).is_file() {
        SidecarActorStore::open(root)?.records()?
    } else {
        Vec::new()
    };
    Ok(
        match device_state_for(root, &[handle.to_string()], &records, did, Some(key_file)) {
            DeviceState::Standing(standing) => Some(standing),
            _ => None,
        },
    )
}

// ---------------------------------------------------------------------------
// device — enroll | authorize | bind (ruling R-P8)
// ---------------------------------------------------------------------------

/// What an enrolling device (B) carries to a device already in the roster (A). Public material
/// only: B's did:key and a nonce B minted and kept.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnrollRequest {
    pub handle: String,
    /// The enrolling device, `did:key:…` (B).
    pub controller: String,
    pub nonce: String,
}

/// What A hands back: the request, the lineage it joins, and A's half of the binding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceAuthorization {
    pub handle: String,
    pub controller: String,
    pub nonce: String,
    /// The handle's chain root, as A's roster knows it.
    pub chain_root: String,
    /// The authorizing device, `did:key:…` (A), already a roster member.
    pub authorized_by: String,
    /// A's signature over the binding row's body, lowercase hex.
    pub sig_a: String,
}

/// The machine-facing result of `bind`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindOutcome {
    pub handle: String,
    pub controller: String,
    pub authorized_by: String,
    pub chain_root: String,
    /// CID of the appended `Binding` row.
    pub row_cid: String,
    pub roster_path: String,
}

/// The pending-enrolment file B keeps its nonce in until `bind` spends it.
fn enroll_file(root: &Path, handle: &str) -> PathBuf {
    root.join(PARTICIPANTS_REL)
        .join(format!(".enroll-{handle}.json"))
}

/// `epr actor device enroll --handle <h>` (on B): mint a nonce, keep it, and return the request.
pub fn device_enroll(root: &Path, handle: &str, device: &DeviceKey) -> ActorResult<EnrollRequest> {
    Roster::new(handle)?;
    let did = device.did_key();
    if read_roster(root, handle)?
        .members(&verifier())
        .contains(&did)
    {
        return Err(ActorError::InvalidArguments(format!(
            "this device ({}) is already a member of {handle}'s roster — nothing to enroll",
            short_did(&did)
        )));
    }
    let mut nonce = [0u8; 16];
    OsRng.fill_bytes(&mut nonce);
    let request = EnrollRequest {
        handle: handle.to_string(),
        controller: did,
        nonce: hex_lower(&nonce),
    };
    let path = enroll_file(root, handle);
    std::fs::create_dir_all(root.join(PARTICIPANTS_REL))?;
    std::fs::write(&path, serde_json::to_string_pretty(&request)?)?;
    Ok(request)
}

/// `epr actor device authorize <request-json-or-path>` (on A): A must already be a member; A
/// signs the binding row's body and returns its half.
pub fn device_authorize(
    root: &Path,
    request: &str,
    device: &DeviceKey,
) -> ActorResult<DeviceAuthorization> {
    let request: EnrollRequest = serde_json::from_str(&json_or_path(request)?)?;
    let roster = read_roster(root, &request.handle)?;
    let verifier = verifier();
    let did = device.did_key();
    let members = roster.members(&verifier);
    if !members.contains(&did) {
        return Err(ActorError::InvalidArguments(format!(
            "this device ({}) is not a member of {}'s roster — only a member authorizes a new \
             device",
            short_did(&did),
            request.handle
        )));
    }
    if members.contains(&request.controller) {
        return Err(ActorError::InvalidArguments(format!(
            "{} is already a member of {}'s roster",
            short_did(&request.controller),
            request.handle
        )));
    }
    let chain_root = roster
        .chain_root(&verifier)
        .ok_or_else(|| ActorError::InvalidArguments("the roster has no chain root".into()))?;
    let body = binding_row(&request, &chain_root, &did, String::new(), String::new());
    let sig_a = hex_lower(&device.sign(&body.signing_message()?));
    Ok(DeviceAuthorization {
        handle: request.handle,
        controller: request.controller,
        nonce: request.nonce,
        chain_root,
        authorized_by: did,
        sig_a,
    })
}

/// `epr actor device bind <authorization-json-or-path>` (on B): the nonce must be the one B
/// minted, A must be a member here too, and B countersigns the same body. Both halves verify or
/// nothing is appended; the nonce is spent only on success.
pub fn device_bind(
    root: &Path,
    authorization: &str,
    device: &DeviceKey,
) -> ActorResult<BindOutcome> {
    let auth: DeviceAuthorization = serde_json::from_str(&json_or_path(authorization)?)?;
    Roster::new(&auth.handle)?;
    let did = device.did_key();
    let pending_path = enroll_file(root, &auth.handle);
    let pending: EnrollRequest = match std::fs::read_to_string(&pending_path) {
        Ok(text) => serde_json::from_str(&text)?,
        Err(_) => {
            return Err(ActorError::InvalidArguments(format!(
                "no pending enrolment for {} on this device — run `epr actor device enroll \
                 --handle {}` here first",
                auth.handle, auth.handle
            )))
        }
    };
    if pending.controller != did || auth.controller != did {
        return Err(ActorError::InvalidArguments(format!(
            "this authorization is for {}, not this device ({}) — no pending enrolment matches",
            short_did(&auth.controller),
            short_did(&did)
        )));
    }
    if pending.nonce != auth.nonce {
        return Err(ActorError::InvalidArguments(
            "nonce mismatch — this authorization answers a different enrolment request".into(),
        ));
    }

    let verifier = verifier();
    let request = EnrollRequest {
        handle: auth.handle.clone(),
        controller: did.clone(),
        nonce: auth.nonce.clone(),
    };
    let unsigned = binding_row(
        &request,
        &auth.chain_root,
        &auth.authorized_by,
        auth.sig_a.clone(),
        String::new(),
    );
    let sig_b = hex_lower(&device.sign(&unsigned.signing_message()?));
    let row = binding_row(
        &request,
        &auth.chain_root,
        &auth.authorized_by,
        auth.sig_a.clone(),
        sig_b,
    );
    verify_binding(&row, &verifier)
        .map_err(|e| ActorError::InvalidArguments(format!("binding refused: {e}")))?;

    let roster = read_roster(root, &auth.handle)?;
    if roster.chain_root(&verifier).as_deref() != Some(auth.chain_root.as_str()) {
        return Err(ActorError::InvalidArguments(format!(
            "{}'s roster in this checkout does not have chain root {} — pull the roster before \
             binding",
            auth.handle,
            short_did(&auth.chain_root)
        )));
    }
    if !roster.members(&verifier).contains(&auth.authorized_by) {
        return Err(ActorError::InvalidArguments(format!(
            "{} is not a member of {}'s roster in this checkout — a binding is authorized by a \
             device already in the lineage",
            short_did(&auth.authorized_by),
            auth.handle
        )));
    }

    let row_cid = SidecarRoster::open(root, &auth.handle)?.append(row)?;
    let _ = std::fs::remove_file(&pending_path);
    Ok(BindOutcome {
        handle: auth.handle.clone(),
        controller: did,
        authorized_by: auth.authorized_by,
        chain_root: auth.chain_root,
        row_cid: row_cid.to_string(),
        roster_path: roster_rel(&auth.handle),
    })
}

fn binding_row(
    request: &EnrollRequest,
    chain_root: &str,
    authorized_by: &str,
    sig_a: String,
    sig_b: String,
) -> ParticipantRow {
    ParticipantRow::Binding {
        handle: request.handle.clone(),
        chain_root: chain_root.to_string(),
        controller: request.controller.clone(),
        authorized_by: authorized_by.to_string(),
        sig_a,
        sig_b,
        nonce: request.nonce.clone(),
    }
}

/// A ceremony message given inline (`{…}`) or as a path to a file holding it.
fn json_or_path(input: &str) -> ActorResult<String> {
    let trimmed = input.trim();
    if trimmed.starts_with('{') {
        Ok(trimmed.to_string())
    } else {
        Ok(std::fs::read_to_string(trimmed)?)
    }
}

/// `did:key:z6Mk…abcd` — enough to recognise a device in a line of output.
fn short_did(did: &str) -> String {
    if did.len() > 24 {
        format!("{}…{}", &did[..16], &did[did.len() - 4..])
    } else {
        did.to_string()
    }
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
            "--again" | "--device" => {
                rest.push(args[i].clone());
                i += 1;
            }
            "--as" | "--session" | "--subject" | "--basis" | "--handle" | "--answers"
            | "--under" => {
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
     claim --as agent:<role>@<model> | human:<handle> --session <id> [--under PATH] [--json] \
     [--root DIR]\n  \
     | current --session <id> | --device [--json] [--root DIR]\n  \
     | witness --subject human:<handle> --as agent:<role>@<model> --session <id> \
     --basis \"<line>\" [--again [--answers <contest-cid>]] [--json] [--root DIR]\n  \
     | contest --subject human:<handle> --as <participant> --session <id> --basis \"<line>\" \
     [--json] [--root DIR]\n  \
     | device enroll --handle <handle> [--root DIR]\n  \
     | device authorize <request-json-or-path> [--root DIR]\n  \
     | device bind <authorization-json-or-path> [--root DIR]\n\
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
        let err = run(&["accept".to_string()]).expect_err("an unknown verb must refuse");
        for verb in [
            "claim",
            "current",
            "witness",
            "contest",
            "device enroll",
            "authorize",
            "bind",
        ] {
            assert!(
                err.to_string().contains(verb),
                "`{verb}` missing from: {err}"
            );
        }
        let err = run(&["device".to_string(), "copy-key".to_string()])
            .expect_err("an unknown device verb must refuse");
        assert!(
            err.to_string().contains("enroll | authorize | bind"),
            "got: {err}"
        );
    }

    #[test]
    fn the_witness_flags_parse_and_again_is_a_bare_switch() {
        let args: Vec<String> = [
            "--subject",
            "human:matthew",
            "--again",
            "--as",
            "agent:orchestrator@fable-5",
            "--basis",
            "present",
            "--session",
            "s1",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let (_opts, rest) = parse_global(&args).expect("parses");
        assert_eq!(
            take_opt(&rest, "--subject").unwrap().unwrap(),
            "human:matthew"
        );
        assert_eq!(take_opt(&rest, "--basis").unwrap().unwrap(), "present");
        assert!(rest.iter().any(|a| a == "--again"));
    }
}
