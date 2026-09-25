//! `epr flow memory import <dir>` — batch-contribute authored memory entries into the collective.
//!
//! The station-four move: `.claude/memory/*.md` stop being a hand-tended directory the harness
//! loads and become **contributions** — fruit a participant placed into shared space, addressed by
//! the CID of the bytes they actually wrote.
//!
//! ## The privacy line is a gate, not a convention
//!
//! `genesis/docs/architecture/private-thought-governed-fruit.md` §2 is constitutional: a
//! participant's reasoning traces, receipts, transcripts, continuations and scratch are never
//! imported, projected, witnessed or targeted by feedback. That is enforced here, in front of
//! every other check, by [`private_reason`] — before the file is read, before its frontmatter is
//! parsed, before a single byte is pinned. A refusal names WHICH rule refused it, because "import
//! skipped 4 files" is indistinguishable from a bug.
//!
//! ## It contributes through the existing path, it does not fork it
//!
//! Each entry becomes an ordinary [`Contribution`] file and goes through `memory contribute`
//! unchanged — same reach policy, same steward check, same write-boundary re-read, same note. This
//! module writes the request and calls that path; it never appends an event itself. Everything the
//! collective refuses for a hand-authored contribution it still refuses for an imported one.
//!
//! ## Attribution, precisely
//!
//! Three identities meet on one imported entry and the substrate keeps them apart:
//!
//! * `Contribution::author` — the **acting** participant: the session's claim, or this device's
//!   standing human. `contribute`'s note guard refuses any author that is not the acting one. A
//!   human git author CANNOT go here, and no persona is minted to make one fit; inventing
//!   `agent:matthew@human` to satisfy a parser would be the substrate asserting a claim nobody
//!   made. Once written, the author is **frozen**: a re-import of unchanged bytes by another
//!   participant is `skipped`, never re-authored (an edit act for changed bytes is station 3's
//!   acts projection).
//! * `Imported::git_name` — the **provenance** of the pinned bytes: the commit's display name
//!   only, read from the entry's own git history. A source's origin, carrying no standing — and
//!   never the email (the identity reserve, below).
//! * `Contribution::steward` — the party the collective's affiliations act for by default (the
//!   repository agent on the root collective), never a declaration field;
//!   the note's `steward:` slot is resolved by the note leg (the device's standing human's handle
//!   where there is one). Nothing here supplies either, which is why neither can be spoofed here.
//!
//! ## The identity reserve
//!
//! The substrate writes a human into fruit only as the handle they claimed or the name the commit
//! already publishes, never an email or any cross-namespace key. Stores written before the rule
//! carry `imported.gitAuthor: Name <email>`; [`migrate_identity_reserve`] rewrites exactly that
//! one line of each to `imported.gitName: Name`, as ONE act attributed through the standard arm
//! order (the executing session's claim, else this device's standing human), and pins a lineage
//! (old request CID → new request CID) so each author's original contribution act keeps
//! attributing the migrated bytes. Every other byte is untouched.
//!
//! ## Idempotence is by content, checked before the append
//!
//! A contribution's address is the CID of its request bytes, and the note leg is already a no-op
//! for a byte-identical append. That alone would still re-open the store 229 times and would drift
//! the moment HEAD moved (a note is dated by the tree it was written against). So import asks the
//! flow plane first: if the attributed contribution observation for these exact bytes is already
//! recorded, the entry is `skipped` and nothing is opened. Re-running import over a drained
//! directory appends zero events and rewrites zero files.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use elohim_epr_rea::{ActorStore, SidecarActorStore};
use eprfs_agent::memory::{Contribution, FileRef, Imported, Locality, Source};
use eprfs_core::BlobCid;
use serde_json::{json, Value};

use super::validation::Reader;
use super::ContributionActs as Acts;
use super::{entries, refused, Options};
use crate::flow::FlowResult;

/// The default home for the contribution requests import writes.
///
/// It sits under the native sidecar tier rather than beside the entries: a contribution is a
/// record of the flow plane, and putting it inside `.claude/memory` would make the directory its
/// own index — the exact hand-tended-derived-view defect the projector was written to end.
pub const DEFAULT_CONTRIBUTIONS_DIR: &str = ".eprfs/status/memory/contributions";

/// Path shapes that are private records under the privacy line and are never importable.
///
/// Matched as PATH COMPONENTS, not substrings, so `recall-executions/` refuses the receipt store
/// while a document named `recall-executions-design.md` stays importable. Each carries the reason
/// its refusal prints, because the refusal is the only place a caller learns the rule exists.
const PRIVATE_COMPONENTS: &[(&str, &str)] = &[
    (
        "memory-kit",
        "the private report tier (.claude/memory-kit): accumulators, dated audit output and \
         receipts, none of it authored for others",
    ),
    (
        "recall-executions",
        "recall receipts: what a session read while deliberating, private to that session",
    ),
    (
        "recall",
        "recall receipts: what a session read while deliberating, private to that session",
    ),
    (
        "transcripts",
        "a transcript of reasoning; no surface may require one as the price of standing",
    ),
    (
        "continuations",
        "continuation files: context carried between runs, a private working record",
    ),
    (
        "scratch",
        "scratch: working notes, never placed into shared space",
    ),
    (
        "traces",
        "a reasoning trace; only an attestation of the outcome may cross",
    ),
];

/// File-name shapes that are private records regardless of where they sit.
const PRIVATE_SUFFIXES: &[(&str, &str)] = &[
    (
        ".continuation.md",
        "a continuation file: context carried between runs, a private working record",
    ),
    (
        ".transcript.md",
        "a transcript of reasoning; only what was said and done is the governed fruit",
    ),
];

/// Why this path may never be imported, or `None` if the privacy line does not name it.
///
/// The gate is deliberately over-inclusive on `recall`: station five keeps its receipts under
/// `.eprfs/status/recall/`, and a gate that only learned about them after they landed would have
/// been a gate that failed once.
pub fn private_reason(rel: &Path) -> Option<&'static str> {
    for component in rel.components() {
        let Component::Normal(name) = component else {
            continue;
        };
        let name = name.to_string_lossy();
        for (needle, reason) in PRIVATE_COMPONENTS {
            if name == *needle {
                return Some(reason);
            }
        }
    }
    let file = rel.file_name()?.to_string_lossy().to_ascii_lowercase();
    PRIVATE_SUFFIXES
        .iter()
        .find(|(suffix, _)| file.ends_with(suffix))
        .map(|(_, reason)| *reason)
}

/// One entry's outcome. `Refused` carries the named reason; the other three carry the request.
#[derive(Debug)]
enum Outcome {
    Refused {
        reason: String,
        /// Whether the refused entry is one the projected index carries today.
        ///
        /// This is the BLAST RADIUS of the refusal, and it is reported rather than inferred:
        /// refusing an `index: false` entry costs the index nothing, while refusing an indexed one
        /// silently drops a row every session used to load. `None` where the refusal happened
        /// before the frontmatter could be read at all.
        indexed: Option<bool>,
    },
    Planned,
    Skipped,
    Contributed(Value),
}

impl Outcome {
    fn tag(&self) -> &'static str {
        match self {
            Outcome::Refused { .. } => "refused",
            Outcome::Planned => "planned",
            Outcome::Skipped => "skipped",
            Outcome::Contributed(_) => "contributed",
        }
    }

    fn refused(reason: impl Into<String>) -> Self {
        Outcome::Refused {
            reason: reason.into(),
            indexed: None,
        }
    }
}

pub fn run(root: &Path, opts: &Options) -> FlowResult<Value> {
    let dir = opts
        .target
        .ok_or_else(|| refused("import needs a directory: epr flow memory import <dir>"))?;
    let dir_rel = normalized(dir)?;
    let contributions_rel = normalized(opts.contributions.unwrap_or(DEFAULT_CONTRIBUTIONS_DIR))?;

    // The collective is read once, from a reader that reads nothing else: every entry then gets a
    // fresh reader so one oversized entry cannot exhaust the next one's byte budget.
    let mut lead = Reader::new(root)?;
    // The collective of record for the directory being imported: its nearest declaration.
    let declaration = lead.nearest_declaration(&rel_str(&dir_rel))?;
    let governance = lead.governance(&declaration)?;
    let (collective_ref, collective) =
        (governance.reference.clone(), governance.declaration.clone());
    let steward = governance.default_steward();
    let root = lead.root.clone();

    if let Some(reason) = private_reason(&dir_rel) {
        return Err(refused(format!(
            "{} is a private store — {reason}",
            dir_rel.display()
        )));
    }

    // The acting participant, resolved from the SAME registered claim `contribute`'s note guard
    // will check. Deriving it here rather than accepting it as a flag means the two can never
    // disagree, and an unregistered session is refused before a single entry is read.
    let author = acting_author(&root, opts.session)?;

    // ONE scan of the flow plane for the whole batch. Asking per entry was O(entries × sidecar) and
    // is what put the projection past the hook's budget; the registry is read here and consulted
    // 229 times without touching the file again.
    let acts = super::ContributionActs::open(&root)?;

    let source_reach = lead.policy_locality(&rel_str(&dir_rel), &collective)?;
    let request_reach =
        lead.policy_locality(&rel_str(&contributions_rel.join("probe.json")), &collective)?;
    // Never widen: a contribution reaches no further than the narrower of the two policies that
    // govern the bytes it pins and the bytes that record it.
    let reach = source_reach.min(request_reach);

    let mut results = Vec::new();
    // contributed, skipped, refused, appended, refused-and-currently-indexed
    let mut counts = (0usize, 0usize, 0usize, 0usize, 0usize);
    for path in list_entries(&root.join(&dir_rel))? {
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let rel = dir_rel.join(&file);
        let outcome = one(
            &root,
            &rel,
            &file,
            &contributions_rel,
            &collective_ref,
            &steward,
            source_reach,
            reach,
            &author,
            &acts,
            opts,
        )?;
        match &outcome {
            Outcome::Contributed(event) => {
                counts.0 += 1;
                if event["event"]["appended"] == json!(true) {
                    counts.3 += 1;
                }
            }
            Outcome::Skipped => counts.1 += 1,
            Outcome::Refused { indexed, .. } => {
                counts.2 += 1;
                if *indexed == Some(true) {
                    counts.4 += 1;
                }
            }
            Outcome::Planned => counts.0 += 1,
        }
        results.push(json!({
            "entry": rel_str(&rel),
            "state": outcome.tag(),
            "reason": match &outcome { Outcome::Refused { reason, .. } => json!(reason), _ => Value::Null },
            "indexedToday": match &outcome { Outcome::Refused { indexed, .. } => json!(indexed), _ => Value::Null },
            "contribution": match &outcome { Outcome::Contributed(v) => v["resource"].clone(), _ => Value::Null },
        }));
    }

    Ok(json!({
        "operation": "import",
        "directory": rel_str(&dir_rel),
        "contributionsDirectory": rel_str(&contributions_rel),
        "collective": collective_ref,
        "dryRun": opts.dry_run,
        "declaredReach": {"source": source_reach, "request": request_reach, "effective": reach},
        "counts": {
            "entries": results.len(),
            "contributed": counts.0,
            "skipped": counts.1,
            "refused": counts.2,
            "eventsAppended": counts.3,
            // Rows the projected index would LOSE if this import were applied as-is. Zero is the
            // precondition for a live apply; anything higher is frontmatter to heal first.
            "refusedIndexedToday": counts.4,
        },
        "entries": results,
        "standing": "Local authored contributions; unreviewed, no acceptance. Private records are \
                     refused by the privacy line, never silently omitted.",
    }))
}

/// Import one entry. Every refusal is named, and no refusal reads further than it must.
#[allow(clippy::too_many_arguments)]
fn one(
    root: &Path,
    rel: &Path,
    file: &str,
    contributions_rel: &Path,
    collective_ref: &FileRef,
    steward: &str,
    source_reach: Locality,
    reach: Locality,
    author: &str,
    acts: &Acts,
    opts: &Options,
) -> FlowResult<Outcome> {
    if let Some(reason) = private_reason(rel) {
        return Ok(Outcome::refused(format!("private record — {reason}")));
    }

    let bytes = std::fs::read(root.join(rel))?;
    let Ok(text) = String::from_utf8(bytes.clone()) else {
        return Ok(Outcome::refused("entry is not UTF-8"));
    };
    let fm = entries::parse(&text);
    for key in ["name", "description"] {
        if fm.get(key).trim().is_empty() {
            return Ok(Outcome::Refused {
                reason: format!(
                    "missing memory frontmatter `{key}:` — an entry with no {key} was not authored \
                     for others in the shape the collective records"
                ),
                indexed: Some(fm.indexed()),
            });
        }
    }
    let entry_type = fm.entry_type().to_string();
    if entry_type.is_empty() {
        return Ok(Outcome::Refused {
            reason: "missing memory frontmatter `metadata.type:` — the entry declares no kind. \
                     Both dialects are accepted (nested under `metadata:`, or a flat top-level \
                     `type:`); a `type:` that a column-zero key has already closed the `metadata:` \
                     block over is neither"
                .into(),
            indexed: Some(fm.indexed()),
        });
    }

    let row = entries::row_from_frontmatter(file, &fm);
    let name = entries::clean_line(fm.get("name"));
    let contribution = Contribution {
        version: 1,
        collective: collective_ref.clone(),
        // The acting participant, per the actor plane. The human whose commit produced these bytes
        // is recorded as provenance below and as the note's steward slot — never as an author.
        author: author.to_string(),
        steward: steward.to_string(),
        scope: "repository".into(),
        reach,
        concern: bounded(&name, 256),
        claim: bounded(&row.desc, 1000),
        uncertainty: vec![format!(
            "Imported verbatim from {}; the claim is the entry's own one-line description and the \
             entry body is the pinned source, not restated here.",
            rel_str(rel)
        )],
        sources: vec![Source {
            resource: FileRef {
                path: rel_str(rel),
                cid: BlobCid::compute_raw(&bytes).to_string(),
            },
            reach: source_reach,
        }],
        supersedes: vec![],
        contradicts: vec![],
        imported: Some(Imported {
            display: bounded(&row.title, 256),
            file: file.to_string(),
            scope_path: rel_str(rel),
            git_name: git_name(root, rel),
            entry_type,
            indexed: fm.indexed(),
        }),
    };

    let request_rel =
        contributions_rel.join(format!("{}.json", file.strip_suffix(".md").unwrap_or(file)));
    let request_text = format!("{}\n", serde_json::to_string_pretty(&contribution)?);

    // Already recorded? Ask the plane before touching anything. This is what makes a second run a
    // true no-op rather than a dedupe that still opened the store 229 times.
    if acts.holds(&request_text, &contribution) {
        return Ok(Outcome::Skipped);
    }
    // Authorship is frozen. The same bytes, already recorded under the participant who first
    // contributed them, are not re-authored because a different participant ran the import: the
    // only difference would be the author slot, and rewriting it is exactly the defect that buried
    // who authored what (one record's author flipped four times across re-imports).
    if frozen(root, &request_rel, &contribution) {
        return Ok(Outcome::Skipped);
    }
    if opts.dry_run {
        return Ok(Outcome::Planned);
    }

    let absolute = root.join(&request_rel);
    if let Some(parent) = absolute.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Rewrite only on change, so an unchanged request keeps its mtime and its CID stays the same
    // bytes on disk that the record names.
    if std::fs::read_to_string(&absolute).ok().as_deref() != Some(request_text.as_str()) {
        std::fs::write(&absolute, &request_text)?;
    }

    let event = super::execute_with(
        root,
        "contribute",
        &Options {
            input: Some(&rel_str(&request_rel)),
            session: opts.session,
            ..Options::default()
        },
    )?;
    Ok(Outcome::Contributed(event))
}

/// Whether the request already on disk is these same bytes under an EARLIER author — i.e. the
/// only thing a rewrite would change is who authored.
///
/// Decided on the TRACKED bytes alone (ruling R-P18): the prior file equals the candidate apart
/// from `author`. The private flow plane is gitignored, so a fresh checkout has none of it; a
/// freeze that needed the plane's record of the prior author's act would let any fresh checkout
/// re-author the whole store (246 author flips were one such re-import). The plane stays an
/// ADDITIONAL source — `acts.holds` above skips bytes it already recorded — never the sole one.
fn frozen(root: &Path, request_rel: &Path, candidate: &Contribution) -> bool {
    let Ok(existing) = std::fs::read_to_string(root.join(request_rel)) else {
        return false;
    };
    let Ok(prior) = serde_json::from_str::<Contribution>(&existing) else {
        return false;
    };
    // An amended declaration re-pins the collective's CID. The same entry, recorded under the
    // earlier declaration of the SAME collective, stays as it was contributed: re-pinning it here
    // would rewrite it under whoever ran this import, which is re-authoring by another name.
    let repinned = prior.collective.path == candidate.collective.path
        && prior.collective.cid != candidate.collective.cid;
    if prior.author == candidate.author && !repinned {
        return false;
    }
    let mut same = candidate.clone();
    same.author = prior.author;
    same.collective = prior.collective;
    let Ok(text) = serde_json::to_string_pretty(&same) else {
        return false;
    };
    let text = format!("{text}\n");
    text == existing
}

/// Where a migration's lineage manifests live: beside the flow plane that pins them, and
/// gitignored with it (`/.eprfs/status/*`). A manifest is only meaningful to the plane holding the
/// contribution acts it carries forward, and that plane is local.
pub(super) const LINEAGE_DIR: &str = ".eprfs/status/identity-reserve";

/// The head of the migration act's reason slot. [`super::ContributionActs`] filters on it; the
/// manifest's raw CID follows it, up to the first `:`.
pub(super) const MIGRATION_REASON_PREFIX: &str = "reason:Identity-reserve migration ";

/// The retired field and its replacement, spelled once.
const RETIRED_FIELD: &str = "gitAuthor";
const RESERVED_FIELD: &str = "gitName";

/// One contribution's move across the identity reserve, as the lineage manifest records it.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Moved {
    pub path: String,
    pub author: String,
    pub collective: String,
    pub from_body: String,
    pub from_raw: String,
    pub to_body: String,
    pub to_raw: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Lineage {
    pub version: u32,
    pub operation: String,
    pub moved: Vec<Moved>,
}

/// Read the lineage manifest a migration act pins, verifying it IS the bytes the act names.
///
/// `slot` is the act's reason slot; `resource` is the act's resource (the manifest's body CID).
/// Any mismatch — missing file, altered bytes, unparsable manifest — is `None`: a lineage that
/// cannot be verified carries nothing forward.
pub(super) fn lineage_for(root: &Path, slot: &str, resource: &str) -> Option<Lineage> {
    let cid = slot
        .strip_prefix(MIGRATION_REASON_PREFIX)?
        .split(':')
        .next()?
        .trim();
    let text = std::fs::read_to_string(root.join(LINEAGE_DIR).join(format!("{cid}.json"))).ok()?;
    if BlobCid::compute_raw(text.as_bytes()).to_string() != cid
        || crate::flow::body_cid(&text).to_string() != resource
    {
        return None;
    }
    serde_json::from_str(&text).ok()
}

/// `Name <email>` → `Name`; any other value (an honest absence) is already reserved and is kept.
fn reserved_name(value: &str) -> String {
    let trimmed = value.trim_end();
    if let Some(open) = trimmed.rfind(" <") {
        if trimmed.ends_with('>') && trimmed[open..].contains('@') {
            return trimmed[..open].trim_end().to_string();
        }
    }
    value.to_string()
}

/// The migrated bytes for one request, or why it cannot be migrated, or `None` if it needs none.
///
/// The rewrite is a ONE-LINE substitution of the exact retired key/value pair, then proven: the
/// result must parse as a strict contribution AND be byte-identical to the serializer's own output
/// for it. Every other byte therefore provably survives, and a file in any other shape is refused
/// by name rather than normalized.
fn reserve(text: &str) -> Option<Result<String, String>> {
    let value: Value = serde_json::from_str(text).ok()?;
    let retired = value.get("imported")?.get(RETIRED_FIELD)?;
    let Some(retired) = retired.as_str() else {
        return Some(Err(format!("imported.{RETIRED_FIELD} is not a string")));
    };
    let quote = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let needle = format!("\"{RETIRED_FIELD}\": {}", quote(retired));
    if text.matches(&needle).count() != 1 {
        return Some(Err(format!(
            "imported.{RETIRED_FIELD} is not exactly one canonical line"
        )));
    }
    let migrated = text.replacen(
        &needle,
        &format!("\"{RESERVED_FIELD}\": {}", quote(&reserved_name(retired))),
        1,
    );
    let canonical = serde_json::from_str::<Contribution>(&migrated)
        .map_err(|e| e.to_string())
        .and_then(|c| serde_json::to_string_pretty(&c).map_err(|e| e.to_string()))
        .map(|pretty| format!("{pretty}\n"));
    Some(match canonical {
        Ok(canonical) if canonical == migrated => Ok(migrated),
        Ok(_) => Err(
            "the file is not in the serializer's canonical layout; refused rather than \
                      normalized, so no other byte moves"
                .into(),
        ),
        Err(e) => Err(format!("the migrated bytes are not a contribution: {e}")),
    })
}

/// `epr flow memory migrate-identity-reserve [--contributions DIR] --session ID [--basis LINE]
/// [--dry-run]`.
///
/// Station 5 of the participant actor plane: every tracked contribution still carrying
/// `imported.gitAuthor: Name <email>` is rewritten to `imported.gitName: Name`, and nothing else
/// about it moves. The whole rewrite is ONE act — an observation note on the lineage manifest —
/// attributed through the standard arm order ([`acting_author`]): the executing session's claim
/// provides it, and only a session that registered none falls to this device's standing human.
/// The standing human is the act's `steward:` slot either way. Forcing the human as provider
/// would attribute to them an act they did not perform — the misattribution the participant plane
/// exists to end (ruling R-P11). A session with no claim on a device standing for no one is
/// refused before anything is written, because the note leg would otherwise fall back to the
/// commit author — the email this act withdraws. `--basis` is one line the executor stands
/// behind (on whose behalf it runs), recorded verbatim in the act's reason.
///
/// Idempotent by construction: a store with nothing left to migrate writes no manifest and
/// rewrites no file. It appends an act only to CORRECT a prior migration act whose provider is not
/// what the standard order resolves for the session that act names — a corrected act for the same
/// lineage manifest, naming the act it corrects; once one stands, a re-run appends nothing.
pub fn migrate_identity_reserve(root: &Path, opts: &Options) -> FlowResult<Value> {
    let contributions_rel = normalized(opts.contributions.unwrap_or(DEFAULT_CONTRIBUTIONS_DIR))?;
    let session = opts.session.ok_or_else(|| {
        refused(
            "migrate-identity-reserve needs --session: the act names the session that executed it",
        )
    })?;
    if opts
        .basis
        .is_some_and(|b| b.trim().is_empty() || b.contains('\n'))
    {
        return Err(refused("--basis is one non-empty line"));
    }
    let root = std::fs::canonicalize(root)?;
    let dir = root.join(&contributions_rel);

    let mut files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(items) => items
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "json"))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();

    let mut moved = Vec::new();
    let mut writes = Vec::new();
    let mut refusals = Vec::new();
    let mut settled = 0usize;
    for path in &files {
        let rel = rel_str(&contributions_rel.join(path.file_name().unwrap_or_default()));
        let text = std::fs::read_to_string(path)?;
        match reserve(&text) {
            None => settled += 1,
            Some(Err(reason)) => refusals.push(json!({"contribution": rel, "reason": reason})),
            Some(Ok(migrated)) => {
                let prior: Value = serde_json::from_str(&text)?;
                moved.push(Moved {
                    path: rel.clone(),
                    author: prior["author"].as_str().unwrap_or_default().to_string(),
                    collective: prior["collective"]["cid"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    from_body: crate::flow::body_cid(&text).to_string(),
                    from_raw: BlobCid::compute_raw(text.as_bytes()).to_string(),
                    to_body: crate::flow::body_cid(&migrated).to_string(),
                    to_raw: BlobCid::compute_raw(migrated.as_bytes()).to_string(),
                });
                writes.push((path.clone(), migrated));
            }
        }
    }

    let counts = json!({
        "contributions": files.len(),
        "migrated": moved.len(),
        "alreadyReserved": settled,
        "refused": refusals.len(),
    });
    let mut report = json!({
        "operation": "migrate-identity-reserve",
        "contributionsDirectory": rel_str(&contributions_rel),
        "field": {"from": format!("imported.{RETIRED_FIELD}"), "to": format!("imported.{RESERVED_FIELD}")},
        "dryRun": opts.dry_run,
        "counts": counts,
        "refused": refusals,
        "act": Value::Null,
    });
    if opts.dry_run {
        return Ok(report);
    }
    if moved.is_empty() {
        // Nothing moves. A prior act attributed outside the standard order is corrected here —
        // the only act a settled store ever appends.
        let mut corrections = Vec::new();
        for prior in misattributed_migrations(&root)? {
            let provider = acting_author(&root, Some(session))?;
            let reason = migration_reason(&MigrationAct {
                manifest_cid: &prior.manifest_cid,
                count: prior.count,
                directory: &rel_str(&contributions_rel),
                session,
                basis: opts.basis,
                corrects: &prior.acts,
            });
            let act = attributed_act(&root, &prior.manifest_rel, &reason, session, &provider)?;
            corrections.push(json!({
                "lineage": prior.manifest_rel,
                "corrects": prior.acts,
                "act": serde_json::to_value(&act)?,
            }));
        }
        if !corrections.is_empty() {
            report["corrections"] = json!(corrections);
        }
        return Ok(report);
    }

    // Resolved BEFORE anything is written: the refusal must leave the store as it was.
    let provider = acting_author(&root, Some(session))?;

    let lineage = Lineage {
        version: 1,
        operation: "identity-reserve-migration".into(),
        moved,
    };
    let manifest = format!("{}\n", serde_json::to_string_pretty(&lineage)?);
    let manifest_cid = BlobCid::compute_raw(manifest.as_bytes()).to_string();
    let manifest_rel = format!("{LINEAGE_DIR}/{manifest_cid}.json");
    std::fs::create_dir_all(root.join(LINEAGE_DIR))?;
    std::fs::write(root.join(&manifest_rel), &manifest)?;

    let reason = migration_reason(&MigrationAct {
        manifest_cid: &manifest_cid,
        count: lineage.moved.len(),
        directory: &rel_str(&contributions_rel),
        session,
        basis: opts.basis,
        corrects: &[],
    });
    let act = attributed_act(&root, &manifest_rel, &reason, session, &provider)?;

    for (path, migrated) in &writes {
        std::fs::write(path, migrated)?;
    }
    report["act"] = serde_json::to_value(&act)?;
    report["lineage"] = json!(manifest_rel);
    Ok(report)
}

/// The words of a migration act's reason, spelled once for the first act and a correction alike.
struct MigrationAct<'a> {
    manifest_cid: &'a str,
    count: usize,
    directory: &'a str,
    session: &'a str,
    basis: Option<&'a str>,
    /// Prior acts for the same lineage this one corrects; empty on a first migration.
    corrects: &'a [String],
}

/// The marker [`executing_session`] reads back out of a reason slot.
const EXECUTED_IN: &str = "executed in session ";

fn migration_reason(act: &MigrationAct<'_>) -> String {
    let mut reason = format!(
        "{}{}: {} contributions under {} rewritten imported.{RETIRED_FIELD} → \
         imported.{RESERVED_FIELD} (display name only; no email in fruit); every other byte \
         unchanged and each author's contribution act carried forward by this lineage; \
         {EXECUTED_IN}{}",
        MIGRATION_REASON_PREFIX.trim_start_matches("reason:"),
        act.manifest_cid,
        act.count,
        act.directory,
        act.session,
    );
    if let Some(basis) = act.basis {
        reason.push_str(&format!("; basis: {}", basis.trim()));
    }
    if !act.corrects.is_empty() {
        reason.push_str(&format!(
            "; corrects {}, whose provider was not the one the standard arm order resolves for \
             its session",
            act.corrects.join(", ")
        ));
    }
    reason
}

/// The session a migration act's reason names as having executed it.
fn executing_session(slot: &str) -> Option<&str> {
    let (_, tail) = slot.split_once(EXECUTED_IN)?;
    let session = tail.split(';').next()?.trim();
    (!session.is_empty()).then_some(session)
}

/// Append the migration act under `session` and prove it resolved to `provider`, the identity
/// [`acting_author`] resolved for the same session — the note leg and this module must agree on
/// who acted, or nothing is claimed to have happened.
///
/// The agreement is checked BEFORE the append (finding M2): the note leg's own resolver is asked
/// who it would attribute the act to, and a disagreement refuses with nothing appended. The
/// outcome is checked once more after, so the record and the refusal can never disagree.
fn attributed_act(
    root: &Path,
    manifest_rel: &str,
    reason: &str,
    session: &str,
    provider: &str,
) -> FlowResult<crate::flow::note::NoteOutcome> {
    let actor = crate::flow::note::NoteActor {
        as_ref: None,
        session: Some(session.to_string()),
    };
    let resolved = crate::flow::note::resolved_actor(root, &actor)?;
    if resolved.as_deref() != Some(provider) {
        return Err(refused(format!(
            "the migration act would resolve to {resolved:?}, not {provider} — nothing was \
             appended and nothing was rewritten"
        )));
    }
    let act = crate::flow::note::note(
        root,
        manifest_rel,
        "observation",
        reason,
        None,
        None,
        &actor,
    )?;
    if act.actor.as_deref() != Some(provider) {
        return Err(refused(format!(
            "the migration act resolved to {:?}, not {provider} — nothing was rewritten",
            act.actor
        )));
    }
    Ok(act)
}

/// A lineage whose every migration act names a provider other than the one the standard arm
/// order resolves for that act's own executing session.
struct Misattributed {
    manifest_cid: String,
    manifest_rel: String,
    count: usize,
    acts: Vec<String>,
}

fn misattributed_migrations(root: &Path) -> FlowResult<Vec<Misattributed>> {
    use elohim_epr_rea::{FlowRecord, FlowStore, ReaVerb, SidecarFlowStore};
    if !root.join(".eprfs/status/flows.jsonl").exists() {
        return Ok(Vec::new());
    }
    // manifest cid → (count, misattributed act CIDs, any act correctly attributed)
    let mut by_manifest: std::collections::BTreeMap<String, (usize, Vec<String>, bool)> =
        std::collections::BTreeMap::new();
    for (cid, record) in SidecarFlowStore::open(root)?.records()? {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if event.action != ReaVerb::Cite
            || !event.classified_as.iter().any(|v| v == "run:observation")
        {
            continue;
        }
        let Some(slot) = event
            .classified_as
            .iter()
            .find(|v| v.starts_with(MIGRATION_REASON_PREFIX))
        else {
            continue;
        };
        let Some(lineage) = lineage_for(root, slot, &event.resource.to_string()) else {
            continue;
        };
        let Some(manifest_cid) = slot
            .strip_prefix(MIGRATION_REASON_PREFIX)
            .and_then(|s| s.split(':').next())
            .map(|s| s.trim().to_string())
        else {
            continue;
        };
        let expected = executing_session(slot).and_then(|s| acting_author(root, Some(s)).ok());
        let entry =
            by_manifest
                .entry(manifest_cid)
                .or_insert((lineage.moved.len(), Vec::new(), false));
        if expected.as_deref() == Some(event.provider.0.as_str()) {
            entry.2 = true;
        } else {
            entry.1.push(cid.to_string());
        }
    }
    Ok(by_manifest
        .into_iter()
        .filter(|(_, (_, acts, settled))| !settled && !acts.is_empty())
        .map(|(manifest_cid, (count, acts, _))| Misattributed {
            manifest_rel: format!("{LINEAGE_DIR}/{manifest_cid}.json"),
            manifest_cid,
            count,
            acts,
        })
        .collect())
}

/// The identity every imported contribution is authored by: the session's registered claim, or —
/// when the session registered none — this device's standing human (ruling R-P6).
///
/// A human's git identity still cannot stand here: nothing is minted from an email, and
/// `agent:<person>@human` stays a forgery. What CAN stand is a human the device verifiably speaks
/// for — witnessed by a present agent or self-claimed, and signed by this device's key — which is
/// the same standing arm `contribute`'s note resolves, so the author and the note's attribution
/// cannot disagree. The git author is still recorded honestly elsewhere: by display name, as
/// `Imported::git_name` on the bytes they wrote — never by email.
fn acting_author(root: &Path, session: Option<&str>) -> FlowResult<String> {
    acting_author_on(root, session, crate::actor::standing_here(root))
}

/// [`acting_author`] with this device's standing human already resolved.
fn acting_author_on(
    root: &Path,
    session: Option<&str>,
    standing: Option<crate::actor::Standing>,
) -> FlowResult<String> {
    let session = session.ok_or_else(|| {
        refused("import writes contributions and needs a registered --session; identity remains a local claim")
    })?;
    if let Some((_, claim)) = SidecarActorStore::open(root)?.current_for(session)? {
        return Ok(claim.claimed.0);
    }
    if let Some(standing) = standing {
        return Ok(standing.subject);
    }
    Err(refused(format!(
        "session `{session}` registered no actor claim and this device stands for no witnessed \
         human — run `epr actor claim --as agent:<role>@<model> --session {session}` first"
    )))
}

/// The display name (`%an`) of the newest commit touching `rel`, or an honest absence.
///
/// The name only — the identity reserve: the commit already publishes the name, and the email is
/// a cross-namespace key the substrate never copies into fruit. An untracked entry has no git
/// provenance, and saying so is the only truthful answer — falling back to HEAD's author would
/// attribute one person's bytes to whoever happened to commit last.
fn git_name(root: &Path, rel: &Path) -> String {
    let path = rel_str(rel);
    let out = crate::process::build_command(
        "git",
        &["log", "-1", "--format=%an", "--", &path],
        root,
        &[],
    )
    .output();
    match out {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if text.is_empty() {
                "(untracked: no commit carries these bytes)".into()
            } else {
                text
            }
        }
        _ => "(unknown: git could not be consulted)".into(),
    }
}

/// Truncate to a byte bound the collective enforces, on a character boundary.
///
/// Every bound this touches (256 for `concern`, 1000 for `claim`) is far above the render bounds
/// (80 and 200), so a value clipped here renders byte-identically to one that was never clipped.
fn bounded(value: &str, max: usize) -> String {
    if value.len() <= max {
        return value.to_string();
    }
    let mut end = max;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].trim_end().to_string()
}

/// `*.md` in one directory, `MEMORY.md` excluded, in the projector's sort order.
fn list_entries(dir: &Path) -> FlowResult<Vec<PathBuf>> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut seen = BTreeSet::new();
    for item in std::fs::read_dir(dir)? {
        let path = item?.path();
        if !path.is_file() || path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if name == "MEMORY.md" || !seen.insert(name) {
            continue;
        }
        found.push(path);
    }
    found.sort_by_key(|p| entries::sort_key(&p.file_name().unwrap_or_default().to_string_lossy()));
    Ok(found)
}

/// A repository-relative path with no dot, parent or root components — the same shape the
/// collective's reader enforces, refused here so the caller learns from the argument, not the read.
fn normalized(path: &str) -> FlowResult<PathBuf> {
    let candidate = Path::new(path.trim_end_matches('/'));
    if candidate.as_os_str().is_empty()
        || candidate
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(refused(format!(
            "`{path}` must be repository-relative with no dot, parent or root components"
        )));
    }
    Ok(candidate.to_path_buf())
}

fn rel_str(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_privacy_line_refuses_by_component_not_by_substring() {
        assert!(private_reason(Path::new(".claude/memory-kit/recall-executions/a.json")).is_some());
        assert!(private_reason(Path::new(".eprfs/status/recall/session.json")).is_some());
        assert!(private_reason(Path::new("notes/scratch/idea.md")).is_some());
        assert!(private_reason(Path::new("a/b.continuation.md")).is_some());
        // A document ABOUT the private tier is still fruit.
        assert!(private_reason(Path::new(".claude/memory/recall-executions-design.md")).is_none());
        assert!(private_reason(Path::new(".claude/memory/project_scratchpad_rails.md")).is_none());
    }

    #[test]
    fn import_acting_author_falls_back_to_standing_human() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("README.md"), "fixture").unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["add", "README.md"],
            vec!["commit", "-qm", "fixture"],
        ] {
            let out = crate::process::build_command("git", &args, root, &[])
                .env("GIT_AUTHOR_NAME", "Fixture")
                .env("GIT_COMMITTER_NAME", "Fixture")
                .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
                .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
                .output()
                .unwrap();
            assert!(out.status.success());
        }
        let keys = tempfile::tempdir().unwrap();
        let key_file = keys.path().join("ed25519.seed");

        // A bare device: an unclaimed session is refused, exactly as before.
        let err = acting_author_on(
            root,
            Some("import-session"),
            crate::actor::standing_on_device(root, &key_file),
        )
        .expect_err("no claim, no standing");
        assert!(
            err.to_string().contains("registered no actor claim"),
            "{err}"
        );

        let key = crate::device_key::DeviceKey::load_or_generate(&key_file).unwrap();
        crate::actor::witness(
            root,
            "human:matthew",
            "agent:orchestrator@claude-fable-5-1",
            "witness-session",
            "operator of this stewarded device",
            false,
            &key,
        )
        .unwrap();
        let standing = crate::actor::standing_on_device(root, &key_file);
        assert!(standing.is_some());

        // The session registered nothing: the device's standing human authors the import.
        assert_eq!(
            acting_author_on(root, Some("import-session"), standing.clone()).unwrap(),
            "human:matthew"
        );
        // A session's own claim still wins over standing.
        crate::actor::claim(root, "agent:implementer@opus-5.5", "import-session").unwrap();
        assert_eq!(
            acting_author_on(root, Some("import-session"), standing.clone()).unwrap(),
            "agent:implementer@opus-5.5"
        );
        // And a session is still required.
        assert!(acting_author_on(root, None, standing).is_err());
    }

    fn committed_fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("README.md"), "fixture").unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["add", "README.md"],
            vec!["commit", "-qm", "fixture"],
        ] {
            let out = crate::process::build_command("git", &args, root, &[])
                .env("GIT_AUTHOR_NAME", "Fixture")
                .env("GIT_COMMITTER_NAME", "Fixture")
                .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
                .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
                .output()
                .unwrap();
            assert!(out.status.success());
        }
        dir
    }

    #[test]
    fn m2_provider_mismatch_appends_no_note() {
        let dir = committed_fixture();
        let root = dir.path();
        crate::actor::claim(root, "agent:implementer@opus-5.5", "exec-session").unwrap();
        let flows = root.join(".eprfs/status/flows.jsonl");
        let before = std::fs::read_to_string(&flows).unwrap_or_default();

        let err = attributed_act(
            root,
            "README.md",
            "a migration act",
            "exec-session",
            "agent:someone-else@fable-5",
        )
        .expect_err("the note leg would attribute the act to someone else");
        assert!(err.to_string().contains("nothing was appended"), "{err}");
        assert_eq!(
            std::fs::read_to_string(&flows).unwrap_or_default(),
            before,
            "the provider is checked before the note is appended"
        );

        // The agreeing provider appends exactly the one act.
        let act = attributed_act(
            root,
            "README.md",
            "a migration act",
            "exec-session",
            "agent:implementer@opus-5.5",
        )
        .expect("agrees");
        assert_eq!(act.actor.as_deref(), Some("agent:implementer@opus-5.5"));
        assert!(act.appended);
    }

    #[test]
    fn bounded_clips_on_a_character_boundary() {
        let value = "—".repeat(200); // 600 bytes
        let clipped = bounded(&value, 256);
        assert!(clipped.len() <= 256);
        assert_eq!(clipped.len() % 3, 0);
    }
}
