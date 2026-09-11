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
//! * `Contribution::author` — the **acting** participant: the registered session's agent claim.
//!   The collective validates it with `parse_agent_ref`, and `contribute`'s note guard refuses any
//!   author that is not the session's own claim. A human git author therefore CANNOT go here, and
//!   no persona is minted to make one fit; inventing `agent:matthew@human` to satisfy a parser
//!   would be the substrate asserting a claim nobody made.
//! * `Imported::git_author` — the **provenance** of the pinned bytes, `Name <email>`, read from
//!   the entry's own git history. A source's origin, carrying no standing.
//! * the note's `steward:` slot — the git-signing human answerable for the tree, which
//!   `note_with_options_guard` already derives from HEAD and attaches to every agent-attributed
//!   record. Nothing here supplies it, which is why it cannot be spoofed here.
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
use eprfs_agent::memory::{Collective, Contribution, FileRef, Imported, Reach, Source};
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
    let (collective_ref, collective) = lead.collective()?;
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

    let source_reach = lead.policy_reach(&rel_str(&dir_rel), &collective)?;
    let request_reach =
        lead.policy_reach(&rel_str(&contributions_rel.join("probe.json")), &collective)?;
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
            &collective,
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
    collective: &Collective,
    source_reach: Reach,
    reach: Reach,
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
        steward: collective.steward.clone(),
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
            git_author: git_author(root, rel),
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

/// The registered claim for `session` — the identity every imported contribution is authored by.
///
/// A human's git identity cannot stand here: the collective validates `author` with
/// `parse_agent_ref`, and minting `agent:<person>@human` to get past that parser would be the
/// substrate asserting a persona nobody claimed. The human is recorded twice, honestly and
/// elsewhere: as `Imported::git_author` on the bytes they wrote, and as the note's `steward:` slot.
fn acting_author(root: &Path, session: Option<&str>) -> FlowResult<String> {
    let session = session.ok_or_else(|| {
        refused("import writes contributions and needs a registered --session; identity remains a local claim")
    })?;
    let claim = SidecarActorStore::open(root)?
        .current_for(session)?
        .ok_or_else(|| {
            refused(format!(
                "session `{session}` registered no actor claim — run `epr actor claim --as agent:<role>@<model> --session {session}` first"
            ))
        })?;
    Ok(claim.1.claimed.0)
}

/// `Name <email>` of the newest commit touching `rel`, or an honest absence.
///
/// An untracked entry has no git provenance, and saying so is the only truthful answer — falling
/// back to HEAD's author would attribute one person's bytes to whoever happened to commit last.
fn git_author(root: &Path, rel: &Path) -> String {
    let path = rel_str(rel);
    let out = crate::process::build_command(
        "git",
        &["log", "-1", "--format=%an <%ae>", "--", &path],
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
    fn bounded_clips_on_a_character_boundary() {
        let value = "—".repeat(200); // 600 bytes
        let clipped = bounded(&value, 256);
        assert!(clipped.len() <= 256);
        assert_eq!(clipped.len() % 3, 0);
    }
}
