//! Bounded local collective-memory composition over governed source files and REA notes.
//! No mutable memory index, latest-head selection, network membership or new EPR kind.
pub mod entries;
pub mod footprint;
mod guide;
mod import;
mod index;
pub mod recall;
mod validation;

use std::path::Path;
use std::process::ExitCode;

use cid::Cid;
use elohim_epr_rea::{FlowRecord, FlowStore, ReaVerb, SidecarFlowStore};
use eprfs_agent::memory::{Contribution, Feedback, FileRef, Graduation, ProjectionRequest, Reach};
use eprfs_core::BlobCid;
use serde_json::{json, Value};

use super::{body_cid, note, FlowError, FlowResult};
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
}

/// All command inputs are strict versioned objects in explicitly named local files.
pub fn run(args: &[String]) -> FlowResult<ExitCode> {
    let operation = args.first().ok_or_else(|| {
        refused("needs collective|pin|contribute|project|feedback|graduate|import|recall")
    })?;
    // The recall executor owns its whole argument surface (sixteen ceremony operations, paging,
    // evidence keys, measurement scopes). Routing it through this parser would mean teaching the
    // memory shell every recall flag, so the raw tail is handed over intact instead.
    if operation == "recall" {
        return recall::run(&args[1..]);
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
            _ => return Err(refused(format!("unknown option {key}"))),
        }
        i += 2;
    }
    let value = execute_with(&root, operation, &opts)?;
    if !json_output {
        println!("Collective memory — {operation}. Local scope; no network authority.");
    }
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(ExitCode::SUCCESS)
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
    let (collective_ref, collective) = reader.collective()?;
    if operation == "collective" {
        return Ok(
            json!({"operation":"collective", "resource":collective_ref, "declaration":collective,
            "inputGuide":guide::input_guide(&collective_ref, &collective),
            "relationship":{"memberKind":"Collective","role":"Steward","memberRef":collective.steward},
            "standing":"Declared local relationship. Registered actor claims supply attribution, not authentication or network membership.","usage":reader.usage()}),
        );
    }
    let input = input.ok_or_else(|| refused("operation needs --input"))?;
    let file = reader.read(input)?;
    let mut output = match operation {
        "contribute" => {
            let assertion: Contribution = serde_json::from_str(&file.text)?;
            reader.contribution(&assertion, &collective_ref, &collective)?;
            reader.allowed(&file.reference.path, assertion.reach, &collective)?;
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
                "actorClaim":event["actor_claim"],"author":event["actor"],"steward":collective.steward,"event":event,"effectiveReach":assertion.reach,
                "standing":"Local authored contribution; independent judgment remains open. Alternatives are not superseded by arrival."})
        }
        "project" => {
            let request: ProjectionRequest = serde_json::from_str(&file.text)?;
            version(request.version)?;
            reader.require_collective(&request.collective, &collective_ref)?;
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
                reader.contribution(&assertion, &collective_ref, &collective)?;
                observed(&mut reader, reference, &source.text, &assertion)?;
                if request.audience > assertion.reach {
                    return Err(refused("projection audience exceeds contribution reach"));
                }
                reader.allowed(&reference.path, request.audience, &collective)?;
                items.push(json!({"resource":reference,"assertion":assertion,
                    "selectionReason":"Explicit caller selection; no ranking or completeness claim.","standing":"Unreviewed contribution; inclusion confers no acceptance."}));
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
            reader.require_collective(&feedback.collective, &collective_ref)?;
            bounded_text(&feedback.passage, "challenged passage", 1000)?;
            bounded_text(&feedback.reason, "reason", 1000)?;
            reader.allowed(&feedback.target.path, Reach::Private, &collective)?;
            let target = reader.pinned(&feedback.target)?;
            if !target.text.contains(&feedback.passage) {
                return Err(refused(
                    "challenged passage is absent from exact target bytes",
                ));
            }
            let effective_reach = reader
                .policy_reach(input, &collective)?
                .min(reader.effective_reach(&feedback.target, &collective, 0)?);
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
            json!({"operation":"feedback","resource":file.reference,"target":feedback.target,"actorClaim":event["actor_claim"],
                "author":event["actor"],"event":event,"effectiveReach":effective_reach,"standing":"Challenge recorded; target bytes pinned by reference. No source rewritten or judgment discharged."})
        }
        "graduate" => {
            let request: Graduation = serde_json::from_str(&file.text)?;
            version(request.version)?;
            reader.require_collective(&request.collective, &collective_ref)?;
            if request.audience != Reach::Repository {
                return Err(refused(
                    "only repository-local graduation rehearsal is supported",
                ));
            }
            reader.allowed(input, request.audience, &collective)?;
            let source = reader.pinned(&request.contribution)?;
            let assertion: Contribution = serde_json::from_str(&source.text)?;
            reader.contribution(&assertion, &collective_ref, &collective)?;
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
            json!({"operation":"graduate","resource":request.contribution,"review":request.review,
                "audience":"repository","effectiveReach":"repository","allowed":true,"standing":"Repository-local reach rehearsal only; no files moved, network publication, peer attestation or experiential acceptance established.",
                "limitations":assertion.uncertainty,"contradictions":assertion.contradicts})
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
    if !reader
        .records()?
        .iter()
        .any(|(_, record)| is_contribution_act(record, &reason, &body_cid(text), &assertion.author))
    {
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
