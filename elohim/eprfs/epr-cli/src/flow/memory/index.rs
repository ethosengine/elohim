//! `epr flow memory project --index` — `MEMORY.md` as a projection of the collective.
//!
//! The kit's projector proved the principle (a hand-maintained derived view drifts past its own
//! budget) and solved it against a directory of frontmatter. This leg keeps the principle and moves
//! the source of truth one layer in: the rows come from **contributions**, so the index is a
//! projection of what participants placed into shared space, not of whatever `.md` files happen to
//! sit in a directory. The rendering is byte-identical either way — that is what `entries.rs`
//! exists to guarantee and what the parity harness measures.
//!
//! Three properties are load-bearing and each is a deliberate choice:
//!
//! * **The cap is declared, never coded.** The kit carried `HARD_BUDGET_BYTES = 24_000` as a module
//!   constant in three files. Here the watermark is resolved from `.claude/epr-meta/measures.yaml`
//!   through the same [`Bound`] the native report reads, and `--budget` names it by pin. A refusal
//!   quotes the bound that refused, so the number is addressable.
//! * **`index_unloaded` becomes a fold, not a private JSON file.** The rows past the cap are the one
//!   deterministic compaction measure — bytes over generated output, unfakeable — and they land as
//!   a structured observation on `memory-index-drift@1` through the ordinary note verb. The fold's
//!   reason pins the exact unloaded SET, so the same drift appended twice is one record and a
//!   changed drift is a new one.
//! * **Consolidation is governed, never silent.** An umbrella's `supersedes:` folds its members'
//!   rows out only when the fold is EFFECTIVE (see `supersession.rs`): at once for the curator's
//!   own work, else after a distinct Steward's approving verdict. Until then both stay indexed and
//!   the umbrella's row says the fold is pending; the report names the approval command.
//! * **Rows are grouped by whom they apply to.** Each entry's `applies-to:` (declared, else its
//!   kind's default) files it under what the collective holds or under the one steward whose
//!   direction it is (see `holder` below), so an inheriting session can tell shared memory from one
//!   person's preference without reading a per-row label.
//! * **Writing is opt-in.** With no `--out` the command reports and writes no index. A projection
//!   that silently overwrote its target every time it was asked a question would be the same defect
//!   in a new place.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eprfs_agent::memory::{AppliesTo, Contribution, Imported};
use serde_json::{json, Value};

use super::supersession;
use super::validation::Reader;
use super::{entries, import, refused, Options};
use crate::flow::measures::{default_measures, default_policies, Bound, MeasureRef, Registry};
use crate::flow::note::{self, NoteActor};
use crate::flow::{FlowError, FlowResult};

/// The measure every unloaded-row fold is taken under.
const DRIFT_MEASURE: &str = "memory-index-drift@1";

/// The bound consulted for the unloaded-row cap when `--budget` names none.
///
/// Resolving it is best-effort: with no such row declared the command reports and refuses nothing,
/// because "how many rows are past a cap that was never declared" has no honest answer.
const DEFAULT_BYTES_MEASURE: &str = "memory-index-bytes@1";

pub fn run(root: &Path, opts: &Options) -> FlowResult<Value> {
    let mut lead = Reader::new(root)?;
    let collective_ref = lead.collective()?.reference;
    let root = lead.root.clone();

    let contributions_rel = Path::new(
        opts.contributions
            .unwrap_or(import::DEFAULT_CONTRIBUTIONS_DIR),
    )
    .to_path_buf();

    let (groups, population, folds) = collect(&mut lead, &contributions_rel)?;
    let text = entries::render(&groups);
    let bytes = text.len();

    // Resolve the cap first: both the unloaded count and the refusal are denominated in it.
    let named = opts.budget.is_some();
    let bound = match opts.budget {
        Some(pin) => Some(resolve(&root, pin)?),
        None => resolve(&root, DEFAULT_BYTES_MEASURE).ok(),
    };
    let hard = bound.as_ref().and_then(|b| b.hard);

    let unloaded = hard.map(|cap| entries::unloaded(&groups, cap as usize));

    // The fold is appended BEFORE any refusal. A breached budget is exactly when the drift matters,
    // and a report that refuses without recording what it saw teaches nothing twice.
    let fold = match &unloaded {
        Some(files) => Some(observe(&root, files, hard.unwrap_or_default(), opts)?),
        None => None,
    };

    let state = match (&bound, hard) {
        (Some(b), Some(cap)) if b.compare.crossed(bytes as f64, cap) => "over-hard",
        (Some(b), _) => match b.soft {
            Some(soft) if b.compare.crossed(bytes as f64, soft) => "over-soft",
            Some(_) => "under",
            None => "under",
        },
        (None, _) => "undeclared",
    };

    // One line per fold awaiting a Steward's verdict, naming the exact approval command.
    let advisory: Vec<String> = folds
        .iter()
        .filter(|f| !f.effective())
        .map(|f| {
            format!(
                "[supersession] {} folds {} contribution(s) and is pending a Steward's verdict — \
                 its members stay indexed until an active Steward who is not its curator ({}) \
                 approves: {}",
                f.umbrella,
                f.members.len(),
                f.author,
                supersession::approval_command(&f.resource)
            )
        })
        .collect();

    let mut wrote = Value::Null;
    if named && state == "over-hard" {
        let b = bound.as_ref().expect("over-hard implies a resolved bound");
        let pending = if advisory.is_empty() {
            String::new()
        } else {
            format!(" {}", advisory.join(" "))
        };
        return Err(refused(format!(
            "projected index is {bytes} bytes, {} the hard watermark {} declared by `{}` \
             ({}) — refusing to write. Consolidate entries (umbrella or graduate) rather than \
             raising the bound.{pending}",
            b.compare.crossed_phrase(),
            b.hard.unwrap_or_default(),
            b.id,
            default_measures(&root).display(),
        )));
    }
    if let Some(out) = opts.out {
        let target = confined(&root, out)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, &text)?;
        wrote = json!(out);
    }

    Ok(json!({
        "operation": "project",
        "mode": "index",
        "collective": collective_ref,
        "contributionsDirectory": contributions_rel.to_string_lossy(),
        "entries": entries::row_count(&groups),
        "population": population,
        "bytes": bytes,
        "budget": bound.as_ref().map(|b| json!({
            "bound": b.id,
            "measure": b.measure.to_string(),
            "soft": b.soft,
            "hard": b.hard,
            "compare": b.compare.as_str(),
            "named": named,
            "state": state,
        })),
        "indexUnloaded": unloaded.as_ref().map(Vec::len),
        "unloadedRows": unloaded,
        "fold": fold,
        // The rows an EFFECTIVE supersession folds out, and every fold's standing. A pending fold
        // names the one command that makes it effective; its members stay indexed until then.
        "supersededRows": superseded_rows(&folds),
        "supersession": folds.iter().map(supersession::Fold::report).collect::<Vec<_>>(),
        "advisory": advisory,
        "wrote": wrote,
        "standing": "Derived view. The contributions are the record; this index is regenerable \
                     from them and carries no acceptance of any claim it lists.",
    }))
}

/// Every indexed contribution in the directory, as rendered rows plus the population it came from.
///
/// A `.json` that does not parse as a contribution, carries no `imported` provenance, or has no
/// attributed contribution observation in the flow plane is not a contribution — it is a file
/// someone left in a directory, and it does not get to add a row to what every session loads.
fn collect(
    reader: &mut Reader,
    dir: &Path,
) -> FlowResult<(Vec<entries::IndexGroup>, Value, Vec<supersession::Fold>)> {
    let root = reader.root.clone();
    let absolute = root.join(dir);
    let mut rows = Vec::new();
    let (mut total, mut unattributed, mut opted_out, mut superseded) =
        (0usize, 0usize, 0usize, 0usize);
    let (mut device_only, mut unreadable) = (0usize, 0usize);
    if !absolute.is_dir() {
        return Ok((
            Vec::new(),
            json!({"contributions": 0, "unattributed": 0, "optedOut": 0, "superseded": 0,
                   "deviceOnly": 0, "unreadable": 0}),
            Vec::new(),
        ));
    }
    // ONE scan of the flow plane for the whole directory: see `ContributionActs`. Asking per
    // contribution is what made this projection a 113-second command. Opened after the early
    // return, so a repository with no contributions never reads the plane at all.
    let acts = super::ContributionActs::open(&root)?;
    // The supersession folds, from the same scan (it carries the verdicts too).
    let folds = supersession::load(reader, dir, &acts);
    let hidden = supersession::hidden(&folds);
    let pending: std::collections::BTreeSet<String> = folds
        .iter()
        .filter(|f| !f.effective())
        .map(|f| f.request.clone())
        .collect();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&absolute)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    for path in files {
        let text = std::fs::read_to_string(&path)?;
        // Counted, not just skipped: a contribution written by a NEWER binary (a field this one
        // does not know, refused by `deny_unknown_fields`) lands here too, and a silent skip would
        // make a sibling device's row vanish from this device's index with nothing said.
        let Ok(contribution) = serde_json::from_str::<Contribution>(&text) else {
            unreadable += 1;
            continue;
        };
        let Some(provenance) = contribution.imported.clone() else {
            continue;
        };
        total += 1;
        if !acts.holds(&text, &contribution) {
            unattributed += 1;
            continue;
        }
        if !provenance.indexed {
            opted_out += 1;
            continue;
        }
        // A device entry's contribution reaches the workspace at most, and this index is a
        // tracked file every clone receives: rendering its row would carry it past its reach.
        // It stays a contribution (recall on this workspace still sees it) and is counted.
        if provenance.effective_applies_to() == AppliesTo::Device {
            device_only += 1;
            continue;
        }
        // Folded out by an EFFECTIVE supersession: the contribution and its file are untouched,
        // recall and attribution still see it; only the index row is the umbrella's now.
        let request = dir
            .join(path.file_name().unwrap_or_default())
            .to_string_lossy()
            .replace('\\', "/");
        let raw = eprfs_core::BlobCid::compute_raw(text.as_bytes()).to_string();
        if hidden.get(&request).is_some_and(|(cid, _)| *cid == raw) {
            superseded += 1;
            continue;
        }
        let holder = holder(&contribution, &provenance);
        // A fold awaiting a Steward's verdict says so on the umbrella's own row.
        let desc = if pending.contains(&request) {
            format!("{} {}", supersession::PENDING_LABEL, contribution.claim)
        } else {
            contribution.claim
        };
        rows.push((
            holder,
            entries::IndexRow {
                file: provenance.file,
                title: provenance.display,
                desc,
            },
        ));
    }
    Ok((
        entries::group(rows),
        json!({"contributions": total, "unattributed": unattributed, "optedOut": opted_out,
               "superseded": superseded, "deviceOnly": device_only, "unreadable": unreadable}),
        folds,
    ))
}

/// Whose row this is: the heading the index files it under.
///
/// Repository entries are the collective's. A steward-bound entry is the direction of
/// the participant who STANDS for it: first the human a steward-of-record line names (that act
/// writes the human into `author`, and the line is what says they stand rather than wrote), else
/// the contribution's `steward` slot — but only when that slot names a participant. Import fills
/// the slot with the collective's own party (`repo:<owner>/<name>` at the root, the child's
/// `collective:<id>` below it), and crediting a preference to the repository would tell a second
/// developer that nobody's direction is somebody's. Such rows are filed as unstewarded instead.
///
/// A steward-of-record line only names a steward when its `author` is a `human:` ref — only a
/// human stands as steward of record, so a line beside an agent author is a malformed or forged
/// slot, and filing the row under the agent would credit it with direction it cannot give. That
/// row falls through to the `steward` slot rule. Device rows never reach here (see `collect`).
fn holder(contribution: &Contribution, provenance: &Imported) -> entries::Holder {
    match provenance.effective_applies_to() {
        AppliesTo::Repository | AppliesTo::Device => entries::Holder::Collective,
        AppliesTo::Steward => {
            let steward_of_record = contribution
                .uncertainty
                .iter()
                .any(|line| line.starts_with(super::import::STEWARD_OF_RECORD));
            if steward_of_record && has_prefix(&contribution.author, "human:") {
                entries::Holder::Steward(contribution.author.trim().to_string())
            } else if names_a_participant(&contribution.steward) {
                entries::Holder::Steward(contribution.steward.trim().to_string())
            } else {
                entries::Holder::Unstewarded
            }
        }
    }
}

/// Whether a steward slot names a participant rather than nobody or a collective's own party.
///
/// An empty slot names no one; a heading built from it would read `Direction from  (theirs…)`.
fn names_a_participant(slot: &str) -> bool {
    let slot = slot.trim();
    !(slot.is_empty() || has_prefix(slot, "repo:") || has_prefix(slot, "collective:"))
}

/// An ASCII-case-insensitive prefix test, so `Repo:` cannot pass for a participant.
fn has_prefix(value: &str, prefix: &str) -> bool {
    value
        .trim()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// The entry files an EFFECTIVE fold hides — the rows the harness's install guard excuses.
fn superseded_rows(folds: &[supersession::Fold]) -> Vec<String> {
    let mut files: Vec<String> = folds
        .iter()
        .filter(|f| f.effective())
        .flat_map(|f| f.members.iter().filter(|m| m.current))
        .filter_map(|m| m.file.clone())
        .collect();
    files.sort();
    files.dedup();
    files
}

/// Append the unloaded-row fold, once per distinct unloaded SET.
///
/// The measure alone would key the fold by its VALUE — three unloaded rows today and three
/// different ones tomorrow would mint one record and the second drift would vanish. So the reason
/// pins the set by content address. Two runs over the same drift are one record; a changed drift is
/// a new one; and a drained index appends its witnessed zero exactly once.
fn observe(root: &Path, files: &[String], cap: f64, opts: &Options) -> FlowResult<Value> {
    let set = eprfs_core::BlobCid::compute_raw(files.join("\n").as_bytes()).to_string();
    let sample: Vec<&str> = files.iter().take(4).map(String::as_str).collect();
    let tail = if files.len() > sample.len() {
        format!(" … and {} more", files.len() - sample.len())
    } else {
        String::new()
    };
    let reason = format!(
        "{DRIFT_MEASURE} = {} index rows past the {cap}-byte cap — rows no session can ever load. \
         Unloaded set {set}{}{tail}",
        files.len(),
        if sample.is_empty() {
            String::new()
        } else {
            format!(": {}", sample.join(", "))
        },
    );
    let outcome = note::observe(
        root,
        "observation",
        DRIFT_MEASURE,
        note::REPO_SUBJECT,
        files.len() as f64,
        Some("rows"),
        &BTreeMap::new(),
        Some(&reason),
        &NoteActor {
            as_ref: None,
            session: opts.session.map(str::to_string),
        },
        &default_measures(root),
    )?;
    Ok(serde_json::to_value(outcome)?)
}

/// The declared bound `pin` names, by bound id or by the measure it consumes.
///
/// Both registries are consulted because a watermark may be declared as a lens row or as a
/// `class: measure` ceiling row, and a caller naming `memory-index-bytes@1` should not have to know
/// which file its watermark ended up in.
fn resolve(root: &Path, pin: &str) -> FlowResult<Bound> {
    let measures_path = default_measures(root);
    let mut bounds = Registry::open(&measures_path)?.lens_bounds();
    if let Some(policies) = Registry::open_optional(&default_policies(root))? {
        bounds.extend(policies.ceiling_bounds());
    }
    if let Some(found) = bounds.iter().find(|b| b.matches(pin)) {
        return Ok(found.clone());
    }
    let measure = MeasureRef::parse(pin, &measures_path)?;
    bounds
        .into_iter()
        .find(|b| b.measure == measure)
        .ok_or_else(|| {
            refused(format!(
                "no declared bound named `{pin}` and none consuming it — declare a watermark row \
                 in {} or {}",
                measures_path.display(),
                default_policies(root).display(),
            ))
        })
}

/// Keep `--out` inside the repository, so a projection cannot be aimed at an arbitrary path.
fn confined(root: &Path, out: &str) -> FlowResult<PathBuf> {
    let candidate = Path::new(out);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    let parent = joined.parent().unwrap_or(root);
    let anchor = std::fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
    if !anchor.starts_with(root) {
        return Err(FlowError::InvalidArguments(format!(
            "collective memory: --out `{out}` resolves outside the repository"
        )));
    }
    Ok(joined)
}
