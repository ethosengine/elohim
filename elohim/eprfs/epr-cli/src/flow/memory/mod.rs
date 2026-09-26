//! Bounded local collective-memory composition over governed source files and REA notes.
//! No mutable memory index, latest-head selection, network membership or new EPR kind.
mod affiliate;
pub mod entries;
pub mod footprint;
mod guide;
mod import;
mod index;
pub mod offer;
pub mod recall;
mod validation;

use std::path::Path;
use std::process::ExitCode;

use cid::Cid;
use elohim_epr_rea::{FlowRecord, FlowStore, ReaVerb, SidecarFlowStore};
use eprfs_agent::memory::{
    Affiliation, AffiliationStanding, Contribution, Feedback, FileRef, Graduation, Locality,
    ProjectionRequest,
};
use eprfs_core::BlobCid;
use serde_json::{json, Value};

use super::{body_cid, note, FlowError, FlowResult};
pub(crate) use validation::COLLECTIVE_PATH;
pub use validation::{
    affiliation_line, affiliation_line_signed, affiliation_signing_message, parse_affiliation_line,
    verify_affiliation_line, AffiliationLine, LineSignature, AFFILIATIONS_PATH,
};
use validation::{bounded_text, version, Reader};

fn refused(message: impl Into<String>) -> FlowError {
    FlowError::InvalidArguments(format!("collective memory: {}", message.into()))
}

/// Everything one memory operation may be told, in one place.
///
/// The verbs grew past what four positional parameters can carry honestly (`import` takes a
/// directory and a dry run; `project --index` takes a budget pin and an output path), and a
/// widening `execute(root, op, a, b, c, d, e)` is how a caller ends up passing `--out` where
/// `--input` belongs. Every field is borrowed and defaulted, so an operation reads only the
/// options it declares.
#[derive(Debug, Default, Clone)]
pub struct Options<'a> {
    /// The authored request file an operation reads (`contribute`, `project`, `feedback`, …).
    pub input: Option<&'a str>,
    /// The registered session whose actor claim attributes any write.
    pub session: Option<&'a str>,
    /// The positional argument — today, `import`'s source directory.
    pub target: Option<&'a str>,
    /// Where contribution requests live; defaults to [`import::DEFAULT_CONTRIBUTIONS_DIR`].
    pub contributions: Option<&'a str>,
    /// Plan without writing: no request files, no events, no index.
    pub dry_run: bool,
    /// `project --index` — render the memory index rather than an explicit projection receipt.
    pub index: bool,
    /// The declared bound whose hard watermark refuses an oversized index.
    pub budget: Option<&'a str>,
    /// Where to write the projected index. Absent means write nothing.
    pub out: Option<&'a str>,
    /// `migrate-identity-reserve --basis` — one line the executor stands behind (on whose behalf
    /// it runs), recorded verbatim in the act's reason. Refused by every other operation.
    pub basis: Option<&'a str>,
    /// `affiliate --member` — the participant ref the affiliation line names.
    pub member: Option<&'a str>,
    /// `affiliate --kind person|collective|elohim-agent`.
    pub kind: Option<&'a str>,
    /// `affiliate --role steward|contributor|observer`.
    pub role: Option<&'a str>,
    /// `affiliate --standing fixture` — a test member at Bootstrap stakes. Absent: standing.
    pub standing: Option<&'a str>,
    /// `affiliate --acts-for <ref>` — on whose behalf the member acts (default: the member's
    /// current line's, else the collective's default).
    pub acts_for: Option<&'a str>,
    /// `affiliate --withdraw` — end the member's current affiliation (sponsored, like any line).
    pub withdraw: bool,
    /// This device's key, loaded (never minted) for `affiliate` to sign with when it is enrolled
    /// for the sponsor.
    pub device: Option<&'a crate::device_key::DeviceKey>,
}

/// The operations this shell dispatches, in the order `usage` names them.
const OPERATIONS: [&str; 11] = [
    "collective",
    "affiliate",
    "pin",
    "contribute",
    "project",
    "feedback",
    "graduate",
    "import",
    "migrate-identity-reserve",
    "recall",
    "index",
];

/// What `epr flow memory` offers, printed as an ANSWER rather than a refusal.
pub fn usage() -> String {
    format!(
        "usage: epr flow memory <{}> [--input FILE] [--session ID] [--json] [--root DIR]\n\n  \
         collective    read a declared collective, its Stewards and contracts (--input PATH: \
         the collective of record for PATH)\n  \
         affiliate     add, change or withdraw one member, sponsored by the session's Steward\n                \
                --member REF --kind person|collective|elohim-agent --role steward|contributor|observer\n                \
                [--standing fixture] [--acts-for REF] [--withdraw] --session ID\n  \
         pin           pin one governed source file by reference\n  \
         contribute    file a governed contribution request\n  \
         project       project a contribution receipt, or --index the memory index\n  \
         feedback      file governed feedback on a contribution\n  \
         graduate      rehearse repository locality for a contribution (a distinct Steward approves)\n  \
         import        adopt an authored directory of requests\n  \
         migrate-identity-reserve  rewrite imported.gitAuthor to imported.gitName, one attributed act\n                    \
                   (--session ID [--basis LINE] [--contributions DIR] [--dry-run])\n  \
         recall        the bounded-evidence recall entry \u{2014} `recall --help` for its own surface\n  \
         index         the semantic fold \u{2014} `index fold|status`, `index --help` for its surface\n",
        OPERATIONS.join("|")
    )
}

/// All command inputs are strict versioned objects in explicitly named local files.
pub fn run(args: &[String]) -> FlowResult<ExitCode> {
    // A caller who named no operation, or asked what the operations ARE, gets the list on stdout
    // with a zero exit. Discovering a surface is a question; only a WRONG operation is an error.
    let Some(operation) = args.first() else {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    };
    if operation == "--help" || operation == "-h" {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    }
    // The recall executor owns its whole argument surface (sixteen ceremony operations, paging,
    // evidence keys, measurement scopes). Routing it through this parser would mean teaching the
    // memory shell every recall flag, so the raw tail is handed over intact instead.
    if operation == "recall" {
        return recall::run(&args[1..]);
    }
    // The semantic fold likewise owns its own surface (`fold|status`, a run cap, an embedder).
    if operation == "index" {
        return recall::index::run(&args[1..]);
    }
    let mut root = std::path::PathBuf::from(".");
    let mut opts = Options::default();
    let mut json_output = false;
    let mut i = 1;
    let mut seen = std::collections::BTreeSet::new();
    while i < args.len() {
        let key = args[i].as_str();
        if !key.starts_with("--") {
            if opts.target.is_some() {
                return Err(refused(format!("unexpected second argument {key}")));
            }
            opts.target = Some(key);
            i += 1;
            continue;
        }
        if !seen.insert(key) {
            return Err(refused(format!("duplicate option {key}")));
        }
        match key {
            "--json" => {
                json_output = true;
                i += 1;
                continue;
            }
            "--dry-run" => {
                opts.dry_run = true;
                i += 1;
                continue;
            }
            "--index" => {
                opts.index = true;
                i += 1;
                continue;
            }
            "--withdraw" => {
                opts.withdraw = true;
                i += 1;
                continue;
            }
            _ => {}
        }
        let value = args
            .get(i + 1)
            .ok_or_else(|| refused(format!("{key} needs a value")))?;
        match key {
            "--root" => root = value.into(),
            "--input" => opts.input = Some(value.as_str()),
            "--session" => opts.session = Some(value.as_str()),
            "--contributions" => opts.contributions = Some(value.as_str()),
            "--budget" => opts.budget = Some(value.as_str()),
            "--out" => opts.out = Some(value.as_str()),
            "--basis" => opts.basis = Some(value.as_str()),
            "--member" => opts.member = Some(value.as_str()),
            "--kind" => opts.kind = Some(value.as_str()),
            "--role" => opts.role = Some(value.as_str()),
            "--standing" => opts.standing = Some(value.as_str()),
            "--acts-for" => opts.acts_for = Some(value.as_str()),
            _ => return Err(refused(format!("unknown option {key}"))),
        }
        i += 2;
    }
    // `affiliate` signs with this device's key when one already exists; it never mints one.
    let device = (operation == "affiliate")
        .then(|| crate::device_key::resolve_path().ok())
        .flatten()
        .and_then(|path| crate::device_key::DeviceKey::load(&path).ok());
    opts.device = device.as_ref();
    let value = execute_with(&root, operation, &opts)?;
    if !json_output {
        println!("Collective memory — {operation}. Local scope; no network authority.");
    }
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(ExitCode::SUCCESS)
}

/// The declaration path of the collective of record for `path`: its nearest
/// `.epr-meta/collective.json`, verified (declaration shape, parent pin, Stewards on record).
/// What `epr actor claim --under <path>` binds a session to.
pub fn collective_of_record(root: &Path, path: &str) -> FlowResult<String> {
    let mut reader = Reader::new(root)?;
    let declaration = reader.nearest_declaration(path)?;
    reader.governance(&declaration)?;
    Ok(declaration)
}

/// The four-parameter shape every existing caller and contract test uses, unchanged.
pub fn execute(
    root: &Path,
    operation: &str,
    input: Option<&str>,
    session: Option<&str>,
) -> FlowResult<Value> {
    execute_with(
        root,
        operation,
        &Options {
            input,
            session,
            ..Options::default()
        },
    )
}

/// Public for contract tests and local composition. Writes only existing flow observations.
pub fn execute_with(root: &Path, operation: &str, opts: &Options) -> FlowResult<Value> {
    if operation == "migrate-identity-reserve" {
        return import::migrate_identity_reserve(root, opts);
    }
    if opts.basis.is_some() {
        return Err(refused("--basis belongs to migrate-identity-reserve alone"));
    }
    let affiliate_only = opts.member.is_some()
        || opts.kind.is_some()
        || opts.role.is_some()
        || opts.standing.is_some()
        || opts.acts_for.is_some()
        || opts.withdraw;
    if operation == "affiliate" {
        return affiliate::run(root, opts);
    }
    if affiliate_only {
        return Err(refused(
            "--member, --kind, --role, --standing, --acts-for and --withdraw belong to affiliate alone",
        ));
    }
    if operation == "import" {
        return import::run(root, opts);
    }
    if operation == "project" && opts.index {
        return index::run(root, opts);
    }
    let (input, session) = (opts.input, opts.session);
    let mut reader = Reader::new(root)?;
    if operation == "pin" {
        let file = reader.read(input.ok_or_else(|| refused("pin needs --input"))?)?;
        return Ok(
            json!({"operation":"pin", "resource":file.reference, "flowResourceCid":body_cid(&file.text).to_string(), "usage":reader.usage()}),
        );
    }
    if operation == "collective" {
        // `--input` here names a PATH whose collective of record to inspect; absent, the root.
        let declaration = match input {
            Some(path) => reader.nearest_declaration(path)?,
            None => COLLECTIVE_PATH.to_string(),
        };
        let governance = reader.governance(&declaration)?;
        let registry = reader.registry_report(&governance.declaration);
        return Ok(
            json!({"operation":"collective", "resource":governance.reference, "declaration":governance.declaration,
            "inputGuide":guide::input_guide(&governance),
            "stewards":governance.steward_report(),
            "stewardship":governance.stewardship(),"stewardlessSince":governance.stewardless_since,
            "affiliations":{"current":governance.affiliations.len(),"invalidLines":governance.invalid_lines,"refused":governance.refused,"sidecar":validation::AFFILIATIONS_PATH},
            "registry":registry,
            "standing":"Declared local relationship. Stewards are affiliation records, the local pre-image of Qahal Membership; every line after the genesis Steward is sponsored by an active Steward who is not its member (`affiliate`), and an unsponsored line does not stand. Registered actor claims supply attribution, not authentication or network membership.","usage":reader.usage()}),
        );
    }
    let input = input.ok_or_else(|| refused("operation needs --input"))?;
    let file = reader.read(input)?;
    // Every request names its collective by pinned declaration; that declaration governs it.
    let named: FileRef = serde_json::from_value(
        serde_json::from_str::<Value>(&file.text)?
            .get("collective")
            .cloned()
            .ok_or_else(|| refused("request names no collective"))?,
    )?;
    let governance = reader.governance(&named.path)?;
    let collective_ref = governance.reference.clone();
    let collective = governance.declaration.clone();
    let mut output = match operation {
        "contribute" => {
            let assertion: Contribution = serde_json::from_str(&file.text)?;
            reader.contribution(&assertion, &governance)?;
            // New work is filed under the current charter; lineage covers only what already was.
            reader.require_current(&assertion.collective, &governance)?;
            reader.owned_by(&file.reference.path, &governance)?;
            reader.allowed(&file.reference.path, assertion.reach, &collective)?;
            reader.require_bound(session, &governance)?;
            let event = record(
                &reader,
                &file.reference,
                input,
                session,
                Some(&assertion.author),
                "observation",
                &format!(
                    "Collective contribution {} under {}; unreviewed, no acceptance",
                    file.reference.cid, collective_ref.cid
                ),
            )?;
            json!({"operation":"contribute","resource":file.reference,"flowResourceCid":body_cid(&file.text).to_string(),
                "actorClaim":event["actor_claim"],"author":event["actor"],"steward":assertion.steward,"collectiveOfRecord":{"path":collective_ref.path,"id":collective.id},"event":event,"effectiveReach":assertion.reach,
                "standing":"Local authored contribution; independent judgment remains open. Alternatives are not superseded by arrival."})
        }
        "project" => {
            let request: ProjectionRequest = serde_json::from_str(&file.text)?;
            version(request.version)?;
            reader.require_collective(&request.collective, &governance)?;
            bounded_text(&request.purpose, "purpose", 1000)?;
            reader.allowed(input, request.audience, &collective)?;
            validation::strings(&request.omissions, "omissions", 8, 300)?;
            if request.inputs.is_empty() || request.inputs.len() > 16 {
                return Err(refused("project needs 1..16 explicit inputs"));
            }
            let mut seen = std::collections::BTreeSet::new();
            let mut items = Vec::new();
            for reference in &request.inputs {
                if !seen.insert(&reference.cid) {
                    return Err(refused("duplicate selected contribution"));
                }
                let source = reader.pinned(reference)?;
                let assertion: Contribution = serde_json::from_str(&source.text)?;
                let lineage = reader.contribution(&assertion, &governance)?;
                observed(&mut reader, reference, &source.text, &assertion)?;
                if request.audience > assertion.reach {
                    return Err(refused("projection audience exceeds contribution reach"));
                }
                reader.allowed(&reference.path, request.audience, &collective)?;
                let mut item = json!({"resource":reference,"assertion":assertion,
                    "selectionReason":"Explicit caller selection; no ranking or completeness claim.","standing":"Unreviewed contribution; inclusion confers no acceptance."});
                if let Some(line) = lineage {
                    item["collectivePin"] = json!(line);
                }
                items.push(item);
            }
            let receipt = json!({"version":1,"method":{"id":"collective-memory-explicit-projection","version":1,
                "cid":method_cid()},"collective":collective_ref,"request":file.reference,
                "purpose":request.purpose,"audience":request.audience,"items":items,
                "omissions":request.omissions,"selection":{"selectedCount":request.inputs.len(),"populationCount":null,
                "rule":"Exact supplied versions only; unsupplied alternatives and corpus population are unknown. Declared contradictions remain linked and never resolve by arrival."},
                "retention":"Ephemeral output, not saved by native command. Save exact output before consequential use or feedback; pin saved bytes with memory pin."});
            let bytes = serde_json::to_vec(&receipt)?;
            if bytes.len() > 24576 {
                return Err(refused("projection exceeds 24KiB; select fewer assertions"));
            }
            json!({"operation":"project","receiptCid":BlobCid::compute_raw(&bytes).to_string(),"receipt":receipt,"retained":false})
        }
        "feedback" => {
            let feedback: Feedback = serde_json::from_str(&file.text)?;
            version(feedback.version)?;
            let request_lineage = reader.require_collective(&feedback.collective, &governance)?;
            bounded_text(&feedback.passage, "challenged passage", 1000)?;
            bounded_text(&feedback.reason, "reason", 1000)?;
            reader.allowed(&feedback.target.path, Locality::Private, &collective)?;
            let target = reader.pinned(&feedback.target)?;
            if !target.text.contains(&feedback.passage) {
                return Err(refused(
                    "challenged passage is absent from exact target bytes",
                ));
            }
            // A contribution target's collective pin is checked and, when it is lineage, said.
            let target_lineage = match serde_json::from_str::<Contribution>(&target.text) {
                Ok(assertion) => reader.require_collective(&assertion.collective, &governance)?,
                Err(_) => None,
            };
            reader.require_bound(session, &governance)?;
            let effective_reach = reader
                .policy_locality(input, &collective)?
                .min(reader.effective_locality(&feedback.target, &collective, 0)?);
            let event = record(
                &reader,
                &feedback.target,
                &feedback.target.path,
                session,
                None,
                "correction",
                &format!(
                    "Collective feedback {:?}; request {} {}; reason: {}",
                    feedback.kind, file.reference.path, file.reference.cid, feedback.reason
                ),
            )?;
            let mut out = json!({"operation":"feedback","resource":file.reference,"target":feedback.target,"actorClaim":event["actor_claim"],
                "author":event["actor"],"event":event,"effectiveReach":effective_reach,"standing":"Challenge recorded; target bytes pinned by reference. No source rewritten or judgment discharged."});
            if let Some(line) = request_lineage {
                out["collectivePin"] = json!(line);
            }
            if let Some(line) = target_lineage {
                out["targetCollectivePin"] = json!(line);
            }
            out
        }
        "graduate" => {
            governance.require_stewarded("graduation")?;
            let request: Graduation = serde_json::from_str(&file.text)?;
            version(request.version)?;
            let request_lineage = reader.require_collective(&request.collective, &governance)?;
            if request.audience != Locality::Repository {
                return Err(refused(
                    "only repository-local graduation rehearsal is supported",
                ));
            }
            reader.allowed(input, request.audience, &collective)?;
            let source = reader.pinned(&request.contribution)?;
            let assertion: Contribution = serde_json::from_str(&source.text)?;
            let lineage = reader.contribution(&assertion, &governance)?;
            if assertion.reach < request.audience {
                return Err(refused("contribution restriction forbids repository reach"));
            }
            reader.allowed(&request.contribution.path, request.audience, &collective)?;
            observed(&mut reader, &request.contribution, &source.text, &assertion)?;
            let records = reader.records()?;
            let review = records
                .iter()
                .find_map(|(cid, record)| {
                    if cid.to_string() != request.review {
                        return None;
                    }
                    match record {
                        FlowRecord::Event(e) => Some(e),
                        _ => None,
                    }
                })
                .ok_or_else(|| refused("independent native review is unavailable"))?;
            if review.action != ReaVerb::Cite
                || review.resource != body_cid(&source.text)
                || !review.classified_as.iter().any(|v| v == "run:verdict")
                || !review.classified_as.iter().any(|v| v == "verdict:approved")
                || review.provider.0 == assertion.author
            {
                return Err(refused(
                    "requires an independent approved native verdict on this exact contribution",
                ));
            }
            // Later contrary review is not erased by selecting an older approval.
            let review_pos = records
                .iter()
                .position(|(cid, _)| cid.to_string() == request.review)
                .unwrap_or(0);
            if records[review_pos + 1..].iter().any(|(_, record)| matches!(record, FlowRecord::Event(e)
                if e.resource == review.resource && e.classified_as.iter().any(|v| v == "verdict:changes-requested"))) {
                return Err(refused("a later contrary native verdict requires renewed review"));
            }
            let (affiliation_cid, approver) =
                steward_approval(&governance, &review.provider.0, &assertion.author)?;
            let fixture = approver.standing == AffiliationStanding::Fixture;
            let standing = if fixture {
                "Repository-local locality rehearsal only, approved by a FIXTURE co-steward (a test human at Bootstrap stakes): the primitive ran, but this is not peer validation. No files moved, network publication, peer attestation or experiential acceptance established."
            } else {
                "Repository-local locality rehearsal only; no files moved, network publication, peer attestation or experiential acceptance established."
            };
            let mut out = json!({"operation":"graduate","resource":request.contribution,"review":request.review,
                "audience":"repository","effectiveReach":"repository","allowed":true,
                "approver":{"member":approver.member,"affiliation":affiliation_cid,"role":approver.role,"standing":approver.standing,
                    "sponsor":approver.sponsor,"signature":governance.signature_report(affiliation_cid)["status"]},
                "validatedAt":validation::validated_at(approver.standing),
                "standing":standing,
                "limitations":assertion.uncertainty,"contradictions":assertion.contradicts});
            if let Some(line) = lineage {
                out["collectivePin"] = json!(line);
            }
            if let Some(line) = request_lineage {
                out["requestCollectivePin"] = json!(line);
            }
            out
        }
        // `recall` never reaches here: it is intercepted in `run` before the shared option parser,
        // and it is not an `execute_with` operation because it owns a session lock and a private
        // receipt store rather than an authored request file.
        _ => return Err(refused("unknown memory operation")),
    };
    output["usage"] = reader.usage();
    output["limits"] = json!({"sourceFiles":32,"sourceBytes":262144,"sidecarBytes":33554432,"projectionBytes":24576});
    Ok(output)
}

/// The Steward affiliation an approving verdict rests on, refused unless it is DISTINCT from the
/// contribution's author (concern C1, anti-self-election).
///
/// Distinct means more than a different string: the approver's Steward affiliation must not also
/// stand for the author, so a package-level agent affiliation cannot approve work by another
/// build of its own role.
fn steward_approval<'g>(
    governance: &'g validation::Governance,
    approver: &str,
    author: &str,
) -> FlowResult<(&'g str, &'g Affiliation)> {
    governance.require_stewarded("an approving verdict")?;
    let named = governance.affiliation_of(approver);
    let Some((cid, affiliation)) = named.filter(|(_, a)| a.is_active_steward()) else {
        let held = named.map_or_else(
            || "no affiliation at all".to_string(),
            |(_, a)| format!("a {:?} affiliation", a.role),
        );
        return Err(refused(format!(
            "the approving verdict's provider {approver} holds no Steward affiliation in {} ({held}); \
             graduation needs a distinct Steward's approval",
            governance.declaration.id
        )));
    };
    if affiliation.same_author_as(author) {
        return Err(refused(format!(
            "the approver {approver} is the author: its Steward affiliation ({}) is the same \
             author as {author} (agent refs compare by role); graduation needs a Steward who is \
             not the contribution's author",
            affiliation.member
        )));
    }
    Ok((cid.as_str(), affiliation))
}

fn method_cid() -> String {
    let bytes = [
        include_str!("mod.rs"),
        include_str!("validation.rs"),
        include_str!("guide.rs"),
        include_str!("entries.rs"),
        include_str!("import.rs"),
        include_str!("index.rs"),
        include_str!("../../../../eprfs-agent/src/memory.rs"),
    ]
    .join("\n");
    BlobCid::compute_raw(bytes.as_bytes()).to_string()
}

fn record(
    reader: &Reader,
    expected: &FileRef,
    target: &str,
    session: Option<&str>,
    expected_author: Option<&str>,
    kind: &str,
    reason: &str,
) -> FlowResult<Value> {
    // Bound the existing note reader too, not just projection's cached sidecar fold.
    if reader.root.join(".eprfs/status/flows.jsonl").exists() {
        reader.sidecar(".eprfs/status/flows.jsonl")?;
    }
    // Validate the exact target again immediately before the existing write boundary.
    reader.unchanged(expected)?;
    let session = session.ok_or_else(|| {
        refused("write needs a registered --session; identity remains a local claim")
    })?;
    reader.sidecar(".eprfs/status/actors.jsonl")?;
    let outcome =
        note::note_for_session(&reader.root, target, kind, reason, session, expected_author)?;
    Ok(serde_json::to_value(outcome)?)
}

// A file's author field is an assertion. Projection requires the matching attributed
// contribution act, while acceptance continues to require its independent path.
fn observed(
    reader: &mut Reader,
    reference: &FileRef,
    text: &str,
    assertion: &Contribution,
) -> FlowResult<()> {
    let reason = contribution_reason(&reference.cid, &assertion.collective.cid);
    let direct = reader.records()?.iter().any(|(_, record)| {
        is_contribution_act(record, &reason, &body_cid(text), &assertion.author)
    });
    // Bytes rewritten across the identity reserve are still the author's: the migration act's
    // lineage carries the original contribution act forward (see `ContributionActs`).
    if !direct && !ContributionActs::open(&reader.root)?.holds(text, assertion) {
        return Err(refused(
            "exact contribution has no attributed native contribution observation",
        ));
    }
    Ok(())
}

/// The exact `classified_as` slot [`record`] writes for a contribution act.
///
/// ONE spelling, consulted by the writer's reader (`observed`) and by import's idempotence check
/// alike. Two spellings of this string is how "the record was written but the re-run wrote it
/// again" happens — and the second write is invisible, because both are valid records.
fn contribution_reason(request_cid: &str, collective_cid: &str) -> String {
    format!(
        "{CONTRIBUTION_REASON_PREFIX}{request_cid} under {collective_cid}; unreviewed, no acceptance"
    )
}

/// The invariant head of that slot — what [`ContributionActs`] filters on so the registry holds
/// contribution acts alone rather than every observation the plane has ever carried.
const CONTRIBUTION_REASON_PREFIX: &str = "reason:Collective contribution ";

fn is_contribution_act(record: &FlowRecord, reason: &str, body: &Cid, author: &str) -> bool {
    matches!(record, FlowRecord::Event(e)
        if e.action == ReaVerb::Cite && e.resource == *body && e.provider.0 == author
        && e.classified_as.iter().any(|v| v == "run:observation")
        && e.classified_as.iter().any(|v| v == reason))
}

/// Every attributed contribution act the sidecar holds, read ONCE.
///
/// The read side of import's idempotence and of the index's population gate. Both ask the same
/// question of every contribution in the directory, and the naive shape — re-open the sidecar,
/// re-parse it, scan it — was O(contributions × sidecar): 229 contributions against a 7,800-line
/// plane took `project --index` **113 seconds**, which put it past the PostToolUse hook's 10-second
/// budget and kept the kit's projector wired in. One scan into a set answers all 229 in the same
/// pass, and the structure is what enforces it: there is no per-contribution entry point to call by
/// mistake.
///
/// A repository with no flow plane yet yields an empty registry rather than creating one — asking a
/// question must never leave a trace.
pub(super) struct ContributionActs {
    /// `(resource body CID, provider, reason slot)` — the exact triple
    /// [`is_contribution_act`] matched, so the set answers the identical question the scan did.
    keys: std::collections::BTreeSet<(String, String, String)>,
}

impl ContributionActs {
    pub(super) fn open(root: &Path) -> FlowResult<Self> {
        let mut keys = std::collections::BTreeSet::new();
        let mut lineages = Vec::new();
        if !root.join(".eprfs/status/flows.jsonl").exists() {
            return Ok(Self { keys });
        }
        for (_, record) in SidecarFlowStore::open(root)?.records()? {
            let FlowRecord::Event(event) = record else {
                continue;
            };
            if event.action != ReaVerb::Cite
                || !event.classified_as.iter().any(|v| v == "run:observation")
            {
                continue;
            }
            for slot in &event.classified_as {
                if slot.starts_with(CONTRIBUTION_REASON_PREFIX) {
                    keys.insert((
                        event.resource.to_string(),
                        event.provider.0.clone(),
                        slot.clone(),
                    ));
                } else if slot.starts_with(import::MIGRATION_REASON_PREFIX) {
                    if let Some(lineage) =
                        import::lineage_for(root, slot, &event.resource.to_string())
                    {
                        lineages.push(lineage);
                    }
                }
            }
        }
        // The identity-reserve lineage, in plane order: a contribution act held for the OLD bytes
        // is carried forward to the migrated bytes under the SAME author. It never attributes
        // bytes whose original act is absent, and never changes who authored them — the migration
        // act itself stands in the plane under its own participant.
        for lineage in lineages {
            for moved in lineage.moved {
                let from = (
                    moved.from_body.clone(),
                    moved.author.clone(),
                    contribution_reason(&moved.from_raw, &moved.collective),
                );
                if keys.contains(&from) {
                    keys.insert((
                        moved.to_body,
                        moved.author,
                        contribution_reason(&moved.to_raw, &moved.collective),
                    ));
                }
            }
        }
        Ok(Self { keys })
    }

    /// Whether the attributed contribution act for these EXACT request bytes is already recorded.
    pub(super) fn holds(&self, request_text: &str, contribution: &Contribution) -> bool {
        let raw = BlobCid::compute_raw(request_text.as_bytes()).to_string();
        self.keys.contains(&(
            body_cid(request_text).to_string(),
            contribution.author.clone(),
            contribution_reason(&raw, &contribution.collective.cid),
        ))
    }
}
