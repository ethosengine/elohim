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
//! * **Writing is opt-in.** With no `--out` the command reports and writes no index. A projection
//!   that silently overwrote its target every time it was asked a question would be the same defect
//!   in a new place.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eprfs_agent::memory::Contribution;
use serde_json::{json, Value};

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
    let (collective_ref, _collective) = lead.collective()?;
    let root = lead.root.clone();

    let contributions_rel = Path::new(
        opts.contributions
            .unwrap_or(import::DEFAULT_CONTRIBUTIONS_DIR),
    )
    .to_path_buf();

    let (rows, population) = collect(&root, &contributions_rel)?;
    let text = entries::render(&rows);
    let bytes = text.len();

    // Resolve the cap first: both the unloaded count and the refusal are denominated in it.
    let named = opts.budget.is_some();
    let bound = match opts.budget {
        Some(pin) => Some(resolve(&root, pin)?),
        None => resolve(&root, DEFAULT_BYTES_MEASURE).ok(),
    };
    let hard = bound.as_ref().and_then(|b| b.hard);

    let unloaded = hard.map(|cap| entries::unloaded(&rows, cap as usize));

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

    let mut wrote = Value::Null;
    if named && state == "over-hard" {
        let b = bound.as_ref().expect("over-hard implies a resolved bound");
        return Err(refused(format!(
            "projected index is {bytes} bytes, {} the hard watermark {} declared by `{}` \
             ({}) — refusing to write. Consolidate entries (umbrella or graduate) rather than \
             raising the bound.",
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
        "entries": rows.len(),
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
fn collect(root: &Path, dir: &Path) -> FlowResult<(Vec<entries::IndexRow>, Value)> {
    let absolute = root.join(dir);
    let mut rows = Vec::new();
    let (mut total, mut unattributed, mut opted_out) = (0usize, 0usize, 0usize);
    if !absolute.is_dir() {
        return Ok((
            rows,
            json!({"contributions": 0, "unattributed": 0, "optedOut": 0}),
        ));
    }
    // ONE scan of the flow plane for the whole directory: see `ContributionActs`. Asking per
    // contribution is what made this projection a 113-second command. Opened after the early
    // return, so a repository with no contributions never reads the plane at all.
    let acts = super::ContributionActs::open(root)?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(&absolute)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    for path in files {
        let text = std::fs::read_to_string(&path)?;
        let Ok(contribution) = serde_json::from_str::<Contribution>(&text) else {
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
        rows.push(entries::IndexRow {
            file: provenance.file,
            title: contribution.imported.map(|i| i.display).unwrap_or_default(),
            desc: contribution.claim,
        });
    }
    rows.sort_by_key(|row| entries::sort_key(&row.file));
    Ok((
        rows,
        json!({"contributions": total, "unattributed": unattributed, "optedOut": opted_out}),
    ))
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
