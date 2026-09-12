//! The measure registry reader — `.claude/epr-meta/measures.yaml` and `.claude/epr-meta/policies.yaml`
//! read as *declared bounds* rather than as two unrelated YAML files.
//!
//! The replacement pattern this module serves is stated in
//! `genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`:
//! **bound declared in epr-meta → outcome witnessed by the native report → history as folds in
//! the flows sidecar → headline as a projection.** The memory kit hides ~35 thresholds as
//! constants inside Python scripts and keeps each accumulator's history in a private JSON file.
//! Neither the constant nor the history is addressable, so nothing can cite them and nothing can
//! verify them. Declaring the same threshold as a registry row makes it a pinned dependency
//! (`<id>@<version>`, never recency — the discipline both registry headers already state), and
//! the fold that satisfies it becomes a content-addressed record.
//!
//! **This module reads; it never writes.** The rows are authored by whoever owns the registry —
//! the two files are outside this crate's write set on purpose — so every read here is
//! deliberately tolerant of keys it does not know and deliberately intolerant of a row that
//! *claims* to be a bound while naming no measure. A row that names no measure is not a bound
//! and is skipped in silence; a row that names a measure the registry does not declare is the
//! caller's refusal to make, not this module's to paper over.
//!
//! **Two row homes, one bound shape.** `measures.yaml`'s `lenses:` section and `policies.yaml`'s
//! `class: measure` ceiling rows are the governance-plane bindings that consume a measure and
//! declare watermarks over it. They differ in provenance (a lens is a reading context; a ceiling
//! is an enforcement row) and not in arithmetic, so both project onto one [`Bound`] and the
//! report evaluates them identically. The distinction survives in [`Bound::source`] so a reader
//! can still tell which home declared it.
//!
//! **Method pinning.** Every [`Registry`] carries the raw-codec CID (`bafkrei…`) of the exact
//! bytes it parsed. A report that names its inputs by content address can be re-derived; one that
//! names them by path can only be re-run, and a re-run against edited bytes is a different
//! measurement wearing the same name.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use eprfs_core::BlobCid;
use serde_yaml::Value;

use super::{FlowError, FlowResult};

/// The measure registry's canonical home, relative to the repository root.
pub const MEASURES_REL: &str = ".claude/epr-meta/measures.yaml";

/// The policy registry's canonical home, relative to the repository root.
pub const POLICIES_REL: &str = ".claude/epr-meta/policies.yaml";

/// The declared home of the measure rows under `root`.
pub fn default_measures(root: &Path) -> PathBuf {
    root.join(MEASURES_REL)
}

/// The declared home of the ceiling rows under `root`.
pub fn default_policies(root: &Path) -> PathBuf {
    root.join(POLICIES_REL)
}

/// A pinned reference to one measure: `<id>@<version>`.
///
/// The version is REQUIRED and a bare id is refused. Both registry headers state the rule the
/// refusal enforces — "a `<id>@<version>` reference is a DECLARED DEPENDENCY, never recency" —
/// and a bare id is exactly the recency-shaped reference that rule forbids. Accepting one here
/// would let an observation silently re-target when a new version row lands, which is the
/// failure the pin exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeasureRef {
    pub id: String,
    pub version: u64,
}

impl MeasureRef {
    /// Parse `<id>@<version>`, naming the registry in every refusal so the caller learns *where*
    /// the legal set lives rather than merely that its argument was wrong.
    pub fn parse(raw: &str, registry: &Path) -> FlowResult<Self> {
        let trimmed = raw.trim();
        let Some((id, version)) = trimmed.rsplit_once('@') else {
            return Err(FlowError::InvalidArguments(format!(
                "measure `{trimmed}` needs a version pin — write `<id>@<version>` \
                 (a bare id is recency-shaped; the declared set lives in {})",
                registry.display()
            )));
        };
        let id = id.trim();
        if id.is_empty() {
            return Err(FlowError::InvalidArguments(format!(
                "measure `{trimmed}` has an empty id — the declared set lives in {}",
                registry.display()
            )));
        }
        let version: u64 = version.trim().parse().map_err(|_| {
            FlowError::InvalidArguments(format!(
                "measure `{trimmed}` has a non-numeric version pin — write `<id>@<version>` \
                 (the declared set lives in {})",
                registry.display()
            ))
        })?;
        Ok(Self {
            id: id.to_string(),
            version,
        })
    }
}

impl fmt::Display for MeasureRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.id, self.version)
    }
}

/// Which registry home declared a bound. Carried so a reader can tell a reading context (lens)
/// from an enforcement row (ceiling) even though the report evaluates the two identically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundSource {
    /// A `lenses:` row in `measures.yaml`.
    Lens,
    /// A `class: measure` row in `policies.yaml`.
    Ceiling,
}

impl BoundSource {
    pub fn as_str(self) -> &'static str {
        match self {
            BoundSource::Lens => "lens",
            BoundSource::Ceiling => "ceiling",
        }
    }
}

/// How a row wants its watermark compared.
///
/// The kit's thresholds are not all the same shape, and flattening them was a real inversion in
/// two different directions.
///
/// **Magnitude, ceiling-shaped.** A SIZE ceiling ("8 MB is the cap") is crossed by exceeding it, so
/// 8 is fine and 8.1 is not. A TRIGGER COUNT ("re-mine when a surface file has changed") fires AT
/// its number: the kit's `mempalace: ⚠ 1 surface file(s) changed` is a warning at exactly 1, and a
/// strict `>` reading of `hard: 1` reported that same 1 as passing — the native surface saying
/// "fine" where the kit says "due".
///
/// **Magnitude, floor-shaped.** Some rows are a MINIMUM: `agent-description-floor@1` with
/// `hard: 80` means "fewer than 80 characters is the finding". Read as a ceiling it says the exact
/// opposite — a 40-character description passes and a thorough 200-character one fails — which is
/// the same inversion wearing the other sign. A floor is not a smaller ceiling and cannot be
/// expressed as one; it needs its own direction.
///
/// The comparator is declared on the row because only the row knows which kind of number it holds.
/// The default stays `above` so every existing row keeps its meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compare {
    /// `observed > watermark` — a ceiling whose watermark is the last acceptable value.
    #[default]
    Above,
    /// `observed >= watermark` — a ceiling whose watermark is itself a breach.
    AtOrAbove,
    /// `observed < watermark` — a floor whose watermark is the last acceptable value.
    Below,
    /// `observed <= watermark` — a floor whose watermark is itself a breach.
    AtOrBelow,
}

impl Compare {
    /// Parse a row's `compare:` value; an unrecognized spelling warns and keeps the default rather
    /// than refusing, because the row belongs to another owner and a typo there must not take the
    /// headline down with it.
    fn parse(raw: &str, row: &str) -> Self {
        match raw.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "above" | "greater-than" | "gt" => Compare::Above,
            "at-or-above" | "gte" | "greater-or-equal" => Compare::AtOrAbove,
            "below" | "less-than" | "lt" => Compare::Below,
            "at-or-below" | "lte" | "less-or-equal" => Compare::AtOrBelow,
            other => {
                eprintln!(
                    "report: bound `{row}` declares unknown compare `{other}` — \
                     keeping the default `above`"
                );
                Compare::Above
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Compare::Above => "above",
            Compare::AtOrAbove => "at-or-above",
            Compare::Below => "below",
            Compare::AtOrBelow => "at-or-below",
        }
    }

    /// Whether this comparator reads its watermark as a MINIMUM rather than a maximum.
    ///
    /// Only the rendering needs to know: "8.3 is past the cap" and "40 is under the floor" are the
    /// same arithmetic event described from opposite sides, and a summary that said "past" about a
    /// floor would misreport which way the number needs to move.
    pub fn is_floor(self) -> bool {
        matches!(self, Compare::Below | Compare::AtOrBelow)
    }

    /// The verb phrase for a crossed watermark, so the summary states which reading produced it.
    pub fn crossed_phrase(self) -> &'static str {
        match self {
            Compare::Above => "is past",
            Compare::AtOrAbove => "has reached",
            Compare::Below => "is under",
            Compare::AtOrBelow => "has fallen to",
        }
    }

    /// The verb phrase for an uncrossed watermark.
    pub fn within_phrase(self) -> &'static str {
        if self.is_floor() {
            "clears"
        } else {
            "within"
        }
    }

    /// Whether `observed` has crossed `watermark` under this comparator.
    pub fn crossed(self, observed: f64, watermark: f64) -> bool {
        match self {
            Compare::Above => observed > watermark,
            Compare::AtOrAbove => observed >= watermark,
            Compare::Below => observed < watermark,
            Compare::AtOrBelow => observed <= watermark,
        }
    }
}

/// How a lens computes its value when the value is an ACCUMULATION rather than a reading.
///
/// Most bounds read a number somebody measured. A few are counts of *events since the last time
/// somebody drained them* — the kit kept these in private JSON accumulators (`placement-drift.json`
/// and its four siblings) that only their own script could read or reset, which is exactly the
/// unaddressable history this replacement exists to end. Declaring `derive:` moves the
/// accumulation into the fold plane: the events are already content-addressed observations, the
/// reset is one more observation, and the count is arithmetic over records anyone can re-derive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Derive {
    /// How many contributing folds were appended since the reset.
    CountSinceReset,
    /// How many DISTINCT SUBJECTS the contributing folds name, counted per consumed measure and
    /// summed across them.
    ///
    /// Per-measure, not globally, because that is what `cleanup-pressure.py::activity()` does: it
    /// sums `len()` over each accumulator file in turn (`cleanup-pressure.py:53-56`), so one path
    /// drifting in two accumulators is two items while the same path drifting twice in one
    /// accumulator is one. Matching the kit's arithmetic exactly is the point of the parity — a
    /// tidier global-distinct rule would report a different number than the gate it replaces.
    DistinctSubjectsSinceReset,
    /// How many files under the declared surfaces are NEWER than a marker file's recorded epoch.
    ///
    /// The first derive that reads the TREE rather than the fold plane, and it is a different kind
    /// of thing on purpose. The other two accumulate acts somebody recorded; this one measures a
    /// standing condition of the filesystem — "how far has the surface moved since the index was
    /// built" — which nobody records because nobody performs it. The kit computed it by walking the
    /// same surfaces in `mempalace-currency.py`; moving the walk here is what lets that script go.
    ///
    /// **A missing marker is `skipped`, never zero.** The kit falls back to the palace config's
    /// mtime and then to epoch 0, and epoch 0 makes EVERY file newer — so the kit's own failure
    /// mode is "everything is stale", not "nothing is". Neither reading is a measurement: with no
    /// marker nobody knows when the index was built, and the honest answer is that this was not
    /// measured. The config fallback is kept (it is the kit's, and it is a real timestamp); only
    /// the epoch-0 arm becomes a refusal.
    FilesNewerThan,
    /// The fraction of a ROLLING TIME WINDOW's folds whose value is positive — a rate, not a count
    /// since a reset.
    ///
    /// `recall-journey-window-ceiling@1` is the first consumer: the per-journey ceilings
    /// (`recall-mistaken-assertions-ceiling`, `recall-unmetered-bytes-ceiling`, …) each read the
    /// LATEST journey alone, so one clean journey right after a bad one reads green with no memory
    /// of the bad one. This derive is the mishpat's actual claim — "the last quarter's journeys are
    /// mostly clean" — read over every consumed measure's folds whose `occurred_at` falls in the
    /// declared `window_days` (default 91, a quarter). No reset exists for a window: the population
    /// ages out on its own as folds fall past the cutoff, so there is nothing to drain.
    ///
    /// **Fewer than 3 folds in the window is `skipped`, never a rate.** A percentage over one or
    /// two journeys is noise wearing a number — the same three-valued discipline the other derives
    /// hold, moved from "no reset yet" to "not enough population yet".
    RateOverWindow,
}

impl Derive {
    fn parse(raw: &str, row: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "count-since-reset" => Some(Derive::CountSinceReset),
            "distinct-subjects-since-reset" => Some(Derive::DistinctSubjectsSinceReset),
            "files-newer-than" => Some(Derive::FilesNewerThan),
            "rate-over-window" => Some(Derive::RateOverWindow),
            other => {
                eprintln!(
                    "report: bound `{row}` declares unknown derive `{other}` — \
                     reading the bound as a plain measurement instead"
                );
                None
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Derive::CountSinceReset => "count-since-reset",
            Derive::DistinctSubjectsSinceReset => "distinct-subjects-since-reset",
            Derive::FilesNewerThan => "files-newer-than",
            Derive::RateOverWindow => "rate-over-window",
        }
    }

    /// Whether this derive reads the filesystem instead of the fold plane.
    ///
    /// The distinction is load-bearing at two call sites: a tree-reading derive admits no folds
    /// (so `admits_measure` must not gate it), and it can never be `skipped` for want of a reset.
    pub fn reads_tree(self) -> bool {
        matches!(self, Derive::FilesNewerThan)
    }

    /// Whether this derive reads a bounded TIME WINDOW of folds rather than an accumulation since
    /// a reset. Load-bearing at the same call site `reads_tree` is: it routes `evaluate` to the
    /// window-reading evaluator instead of the reset-reading one, and it has no `reset:` measure.
    pub fn reads_window(self) -> bool {
        matches!(self, Derive::RateOverWindow)
    }
}

/// The surfaces, marker and tolerance a [`Derive::FilesNewerThan`] bound walks.
///
/// Declared on the row rather than compiled in, for the same reason every watermark is: the kit
/// hid `SURFACE`, `GRACE_SECONDS` and the marker path as module constants only that script could
/// read, and the whole replacement is about making them addressable.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceWalk {
    /// The file whose contents are an epoch float — the "last built" stamp.
    pub marker: String,
    /// A second stamp consulted when the marker is absent; its MTIME is the epoch. The kit's
    /// `.mempalace/config.json` fallback, declared rather than hidden.
    pub marker_fallback: Option<String>,
    /// Repo-relative directories to walk.
    pub surfaces: Vec<String>,
    /// File suffix counted. Defaults to `.md`, which is the kit's filter.
    pub suffix: String,
    /// Skew tolerated between a write and a build, in seconds.
    pub grace_seconds: f64,
}

/// One declared bound: a subject, a measure, watermarks, and the env it is keyed by.
///
/// Keyed `subject × measure@version × env`, which is the guidestar's memoization key — the same
/// triple a fold must match for the report to consider it evidence of *this* bound rather than of
/// a differently-scoped run that happened to use the same measure.
#[derive(Debug, Clone)]
pub struct Bound {
    /// `<id>@<version>` — the bound's own pinned identity.
    pub id: String,
    /// The bare row id, so `--bound <id>` matches without the caller retyping the version.
    pub bare_id: String,
    /// The PRIMARY measure this bound consumes — its identity anchor: the headline slot derives
    /// from this id, the unit comes from its row, and a `skipped` outcome names it.
    pub measure: MeasureRef,
    /// Every measure the row consumes, primary first. Identical to `[measure]` unless the row
    /// declares several, which only a derived bound does.
    pub consumes: Vec<MeasureRef>,
    /// How the value is computed. `None` means the ordinary reading: the latest matching fold.
    pub derive: Option<Derive>,
    /// The PRIMARY measure's declared unit, filled in by the registry that declared it.
    ///
    /// A derived bound needs this: it accumulates folds from several measures with several units
    /// (`documents`, `seeds`, `edits`), and reporting whichever one happened to come first would
    /// label a pressure score `edits`. The lens is denominated in its own measure's unit, always.
    pub unit: Option<String>,
    /// The measure whose observation DRAINS a derived accumulation. Declared `reset:`, else
    /// `<primary-measure-id>-reset@1` — so `cleanup-pressure@1`'s reset is `cleanup-pressure-reset@1`,
    /// which is the command a person types.
    pub reset: MeasureRef,
    /// The subject the bound is declared over. `None` means "whatever subject the fold names" —
    /// a row may legitimately bind a measure that has exactly one subject in practice.
    pub subject: Option<String>,
    /// Env keys the fold must carry (subset match; a fold may carry more).
    pub env: BTreeMap<String, String>,
    /// The nudge watermark. Crossing it is `passed` with a warn summary, never a failure.
    pub soft: Option<f64>,
    /// The breach watermark. Crossing it is `failed`.
    pub hard: Option<f64>,
    /// How the watermarks are compared. Declared on the row; defaults to [`Compare::Above`].
    pub compare: Compare,
    /// An explicit headline slot, overriding the id-derived default.
    pub headline: Option<String>,
    /// The row's `binding:` — its rung on the Precedent ladder
    /// (`constitutional`/`binding-network`/`binding-local`/`persuasive`/`observation`).
    ///
    /// Carried through to the outcome unchanged rather than interpreted here: the binding says what
    /// a breach COSTS, and this crate is the floor that measures. Reading it would be this module
    /// quietly acquiring teeth the registry gave to whoever consumes the outcome.
    pub binding: Option<String>,
    /// The surfaces a tree-reading derive walks. `Some` exactly when `derive` reads the tree.
    pub walk: Option<SurfaceWalk>,
    /// `window_days:` (or `window-days:`) — how far back a `rate-over-window` derive looks.
    /// `None` when undeclared; the evaluator's own default (91, a quarter) applies then, kept out
    /// of this crate's parsing so the declared-vs-defaulted distinction stays visible to a reader
    /// of the row rather than being baked into a silently-filled field.
    pub window_days: Option<f64>,
    /// The row's `status:`. `superseded` takes the bound OUT of evaluation — see [`Bound::retired`].
    pub status: Option<String>,
    /// `superseded_by:` — what replaced it. The registry's never-delete rule: a version with live
    /// consumers is never removed, only superseded, and the lineage stays readable.
    pub superseded_by: Option<String>,
    /// `superseded_reason:` — why, in the words a headline slot renders.
    pub superseded_reason: Option<String>,
    pub source: BoundSource,
}

impl Bound {
    /// Whether this bound has been RETIRED by supersession, and is therefore not a measurement.
    ///
    /// A superseded bound is not `skipped` — `skipped` means "nobody measured this", which is a gap
    /// somebody should close. A superseded bound means "this is no longer measured, on purpose, and
    /// here is what replaced it". Rendering the second as the first is how a deliberate retirement
    /// reads as neglect, which is exactly the false signal the three-valued vocabulary exists to
    /// prevent. So it leaves evaluation entirely and is reported in its own list.
    pub fn retired(&self) -> bool {
        self.status
            .as_deref()
            .is_some_and(|s| s.trim().eq_ignore_ascii_case("superseded"))
    }

    /// Whether `filter` names this bound, by bare id or by full pin.
    pub fn matches(&self, filter: &str) -> bool {
        let filter = filter.trim();
        self.id == filter || self.bare_id == filter
    }

    /// Whether a fold of `measure` is evidence for this bound.
    ///
    /// A plain bound admits its PRIMARY measure only, so a row that happens to list several
    /// `consumes:` keeps the meaning it had before this key existed. A derived bound admits any
    /// consumed measure, because accumulating across measures is the whole point of declaring one.
    pub fn admits_measure(&self, measure: &MeasureRef) -> bool {
        if self.derive.is_some() {
            self.consumes.contains(measure)
        } else {
            self.measure == *measure
        }
    }
}

/// One parsed registry file plus the content address of the exact bytes parsed.
pub struct Registry {
    pub path: PathBuf,
    /// Raw-codec CID (`bafkrei…`) of the bytes read — the method pin every report carries.
    pub bytes_cid: String,
    doc: Value,
}

impl Registry {
    /// Read and parse `path`, or refuse naming it.
    pub fn open(path: &Path) -> FlowResult<Self> {
        let bytes = std::fs::read(path).map_err(|source| FlowError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let doc: Value = serde_yaml::from_slice(&bytes).map_err(|source| FlowError::Yaml {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            bytes_cid: BlobCid::compute_raw(&bytes).to_string(),
            doc,
        })
    }

    /// Read `path` when it exists, `None` when it does not.
    ///
    /// A missing registry is absence, not corruption: a repository that has declared lenses but no
    /// ceilings (or the reverse) is a legitimate state, and the report says so by emitting a null
    /// method CID rather than by failing. An *unreadable* or malformed file still refuses — that
    /// is a broken declaration, and reporting over it would be reporting over a guess.
    pub fn open_optional(path: &Path) -> FlowResult<Option<Self>> {
        if path.exists() {
            Self::open(path).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Every declared `<id>@<version>` under `measures:`, in file order.
    pub fn measure_pins(&self) -> Vec<MeasureRef> {
        rows(&self.doc, "measures")
            .filter_map(|row| {
                let id = string_at(row, "id")?;
                let version = number_at(row, "version").unwrap_or(1.0) as u64;
                Some(MeasureRef { id, version })
            })
            .collect()
    }

    /// Whether the registry declares this exact pin.
    pub fn declares(&self, want: &MeasureRef) -> bool {
        self.measure_pins().iter().any(|pin| pin == want)
    }

    /// The `unit:` a measure row declares, when it declares one.
    pub fn declared_unit(&self, want: &MeasureRef) -> Option<String> {
        rows(&self.doc, "measures").find_map(|row| {
            let id = string_at(row, "id")?;
            let version = number_at(row, "version").unwrap_or(1.0) as u64;
            if id == want.id && version == want.version {
                string_at(row, "unit")
            } else {
                None
            }
        })
    }

    /// Bounds declared by this registry's `lenses:` rows.
    pub fn lens_bounds(&self) -> Vec<Bound> {
        rows(&self.doc, "lenses")
            .filter_map(|row| bound_from(row, BoundSource::Lens))
            .map(|bound| self.with_declared_unit(bound))
            .collect()
    }

    /// Denominate a bound in its PRIMARY measure's unit when this registry declares it.
    fn with_declared_unit(&self, mut bound: Bound) -> Bound {
        bound.unit = self.declared_unit(&bound.measure);
        bound
    }

    /// Bounds declared by this registry's `class: measure` policy rows that carry a watermark.
    ///
    /// The watermark requirement is what keeps today's two live ceiling rows
    /// (`source-file-loc-ceiling`, `capability-governance`) from becoming permanently-skipped
    /// noise in the headline: they declare LoC watermarks but consume no measure, so `bound_from`
    /// drops them for want of a measure reference — the honest answer, since neither has a fold
    /// plane to be evaluated against.
    pub fn ceiling_bounds(&self) -> Vec<Bound> {
        rows(&self.doc, "policies")
            .filter(|row| string_at(row, "class").as_deref() == Some("measure"))
            .filter_map(|row| bound_from(row, BoundSource::Ceiling))
            .map(|bound| self.with_declared_unit(bound))
            .filter(|bound| bound.soft.is_some() || bound.hard.is_some())
            .collect()
    }
}

/// Project one registry row onto a [`Bound`], or `None` when the row names no measure.
///
/// The measure reference is accepted from `consumes:` (the established lens key, scalar or
/// sequence) or from a `measure:` block's `consumes:`/`id:`. Watermarks are read row-level first,
/// then from the `measure:` block, and the legacy `loc-soft`/`loc-hard` spelling is accepted last
/// so an existing ceiling row that later grows a `consumes:` key starts evaluating without being
/// rewritten.
fn bound_from(row: &Value, source: BoundSource) -> Option<Bound> {
    let bare_id = string_at(row, "id")?;
    let version = number_at(row, "version").unwrap_or(1.0) as u64;
    let measure_raw = first_string(row, "consumes")
        .or_else(|| get(row, "measure").and_then(|m| first_string(m, "consumes")))
        .or_else(|| get(row, "measure").and_then(|m| string_at(m, "id")))?;
    // A malformed pin in a row this crate does not own is a declaration bug, not a report bug:
    // drop the row with a notice rather than refusing the whole report over someone else's typo.
    let measure = match MeasureRef::parse(&measure_raw, Path::new(MEASURES_REL)) {
        Ok(pin) => pin,
        Err(err) => {
            eprintln!("report: skipping bound `{bare_id}` — {err}");
            return None;
        }
    };

    let block = get(row, "measure");
    let soft = number_at(row, "soft")
        .or_else(|| block.and_then(|m| number_at(m, "soft")))
        .or_else(|| block.and_then(|m| number_at(m, "loc-soft")));
    let hard = number_at(row, "hard")
        .or_else(|| block.and_then(|m| number_at(m, "hard")))
        .or_else(|| block.and_then(|m| number_at(m, "loc-hard")));

    let subject = string_at(row, "subject").or_else(|| block.and_then(|m| string_at(m, "subject")));
    let env = map_at(row, "env")
        .or_else(|| block.and_then(|m| map_at(m, "env")))
        .unwrap_or_default();
    let headline =
        string_at(row, "headline").or_else(|| block.and_then(|m| string_at(m, "headline")));

    let compare = string_at(row, "compare")
        .or_else(|| block.and_then(|m| string_at(m, "compare")))
        .map(|raw| Compare::parse(&raw, &bare_id))
        .unwrap_or_default();

    let derive = string_at(row, "derive")
        .or_else(|| block.and_then(|m| string_at(m, "derive")))
        .and_then(|raw| Derive::parse(&raw, &bare_id));

    // Every consumed pin, primary first. A malformed one is DROPPED with a notice rather than
    // taking the row down: a derived bound with four good sources and one typo should still count
    // the four, and say so, instead of vanishing from the headline.
    let mut consumes: Vec<MeasureRef> = Vec::new();
    for raw in all_strings(row, "consumes")
        .or_else(|| block.and_then(|m| all_strings(m, "consumes")))
        .unwrap_or_default()
    {
        match MeasureRef::parse(&raw, Path::new(MEASURES_REL)) {
            Ok(pin) => {
                if !consumes.contains(&pin) {
                    consumes.push(pin);
                }
            }
            Err(err) => eprintln!("report: bound `{bare_id}` skips a consumed measure — {err}"),
        }
    }
    if consumes.is_empty() {
        consumes.push(measure.clone());
    }

    // `reset:` when declared, else `<primary-measure-id>-reset@1` — the spelling the documented
    // reset command uses, so nobody has to look up a second name to drain an accumulator.
    let reset = string_at(row, "reset")
        .or_else(|| block.and_then(|m| string_at(m, "reset")))
        .and_then(
            |raw| match MeasureRef::parse(&raw, Path::new(MEASURES_REL)) {
                Ok(pin) => Some(pin),
                Err(err) => {
                    eprintln!("report: bound `{bare_id}` declares an unreadable reset — {err}");
                    None
                }
            },
        )
        .unwrap_or_else(|| MeasureRef {
            id: format!("{}-reset", measure.id),
            version: 1,
        });

    // The walk is parsed only for a tree-reading derive, and a tree-reading derive with no walk is
    // DROPPED back to a plain reading rather than silently walking nothing: a bound that declared a
    // surface scan and scanned no surfaces would report a confident zero.
    let walk = derive.filter(|d| d.reads_tree()).and_then(|_| {
        let marker =
            string_at(row, "marker").or_else(|| block.and_then(|m| string_at(m, "marker")));
        let surfaces = all_strings(row, "surfaces")
            .or_else(|| block.and_then(|m| all_strings(m, "surfaces")))
            .unwrap_or_default();
        match marker {
            Some(marker) if !surfaces.is_empty() => Some(SurfaceWalk {
                marker,
                marker_fallback: string_at(row, "marker-fallback")
                    .or_else(|| block.and_then(|m| string_at(m, "marker-fallback"))),
                surfaces,
                suffix: string_at(row, "suffix")
                    .or_else(|| block.and_then(|m| string_at(m, "suffix")))
                    .unwrap_or_else(|| ".md".to_string()),
                grace_seconds: number_at(row, "grace-seconds")
                    .or_else(|| block.and_then(|m| number_at(m, "grace-seconds")))
                    .unwrap_or(0.0),
            }),
            _ => {
                eprintln!(
                    "report: bound `{bare_id}` declares `derive: files-newer-than` without both \
                     `marker:` and `surfaces:` — reading the bound as a plain measurement instead"
                );
                None
            }
        }
    });
    let derive = if derive.is_some_and(Derive::reads_tree) && walk.is_none() {
        None
    } else {
        derive
    };

    // Accepted as `window_days:` (the spelling `recall-journey-window-ceiling@1` declares) or
    // `window-days:` (this registry's own hyphenated house style) — the row's owner should not
    // have to remember which convention the parser insists on.
    let window_days = number_at(row, "window_days")
        .or_else(|| number_at(row, "window-days"))
        .or_else(|| block.and_then(|m| number_at(m, "window_days")))
        .or_else(|| block.and_then(|m| number_at(m, "window-days")));

    Some(Bound {
        id: format!("{bare_id}@{version}"),
        bare_id,
        measure,
        consumes,
        derive,
        unit: None,
        reset,
        subject,
        env,
        soft,
        hard,
        compare,
        headline,
        binding: string_at(row, "binding"),
        walk,
        window_days,
        status: string_at(row, "status"),
        superseded_by: string_at(row, "superseded_by").or_else(|| string_at(row, "superseded-by")),
        superseded_reason: string_at(row, "superseded_reason")
            .or_else(|| string_at(row, "superseded-reason")),
        source,
    })
}

/// Every element of a sequence-or-scalar field.
fn all_strings(value: &Value, key: &str) -> Option<Vec<String>> {
    let field = value.get(key)?;
    if let Some(seq) = field.as_sequence() {
        Some(
            seq.iter()
                .filter_map(Value::as_str)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        )
    } else {
        field
            .as_str()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|s| vec![s])
    }
}

// ---------------------------------------------------------------------------
// Loose YAML walking — the registries are authored elsewhere, so unknown keys
// are normal and a strict deserialize would refuse a perfectly good row.
// ---------------------------------------------------------------------------

fn rows<'a>(doc: &'a Value, section: &str) -> impl Iterator<Item = &'a Value> {
    doc.get(section)
        .and_then(Value::as_sequence)
        .map(|s| s.iter())
        .unwrap_or_else(|| [].iter())
}

fn get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get(key)
}

fn string_at(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// A number, accepting the YAML string spelling too (`soft: "20000"` is a declaration, not a typo
/// worth refusing over).
fn number_at(value: &Value, key: &str) -> Option<f64> {
    let field = value.get(key)?;
    field
        .as_f64()
        .or_else(|| field.as_i64().map(|v| v as f64))
        .or_else(|| field.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
}

/// The first element of a sequence-or-scalar field.
fn first_string(value: &Value, key: &str) -> Option<String> {
    let field = value.get(key)?;
    if let Some(seq) = field.as_sequence() {
        seq.first()
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        field
            .as_str()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }
}

fn map_at(value: &Value, key: &str) -> Option<BTreeMap<String, String>> {
    let mapping = value.get(key)?.as_mapping()?;
    let mut out = BTreeMap::new();
    for (k, v) in mapping {
        let (Some(k), Some(v)) = (k.as_str(), scalar_string(v)) else {
            continue;
        };
        out.insert(k.trim().to_string(), v);
    }
    Some(out)
}

fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry(text: &str) -> Registry {
        Registry {
            path: PathBuf::from(MEASURES_REL),
            bytes_cid: BlobCid::compute_raw(text.as_bytes()).to_string(),
            doc: serde_yaml::from_str(text).unwrap(),
        }
    }

    #[test]
    fn a_bare_measure_id_is_refused_with_the_registry_named() {
        let err = MeasureRef::parse("memory-index-bytes", Path::new(MEASURES_REL)).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("version pin"), "{message}");
        assert!(message.contains(MEASURES_REL), "{message}");
    }

    #[test]
    fn a_lens_row_projects_onto_a_bound_with_both_watermarks() {
        let reg = registry(
            "measures:\n  - id: memory-index-bytes\n    version: 1\n    unit: bytes\n\
             lenses:\n  - id: memory-index-budget\n    version: 1\n    \
             consumes: [memory-index-bytes@1]\n    subject: .claude/memory/MEMORY.md\n    \
             soft: 20000\n    hard: 24000\n",
        );
        let bounds = reg.lens_bounds();
        assert_eq!(bounds.len(), 1);
        assert_eq!(bounds[0].id, "memory-index-budget@1");
        assert_eq!(bounds[0].measure.to_string(), "memory-index-bytes@1");
        assert_eq!(bounds[0].soft, Some(20000.0));
        assert_eq!(bounds[0].hard, Some(24000.0));
        assert!(reg.declares(&MeasureRef {
            id: "memory-index-bytes".into(),
            version: 1
        }));
        assert_eq!(
            reg.declared_unit(&MeasureRef {
                id: "memory-index-bytes".into(),
                version: 1
            })
            .as_deref(),
            Some("bytes")
        );
    }

    #[test]
    fn a_ceiling_row_naming_no_measure_is_not_a_bound() {
        // Exactly today's `source-file-loc-ceiling` shape: watermarks, no `consumes:`.
        let reg = registry(
            "policies:\n  - id: source-file-loc-ceiling\n    version: 1\n    class: measure\n    \
             measure:\n      kind: level\n      loc-soft: 3000\n      loc-hard: 7000\n",
        );
        assert!(reg.ceiling_bounds().is_empty());
    }

    #[test]
    fn a_ceiling_row_that_consumes_a_measure_reads_its_legacy_watermarks() {
        let reg = registry(
            "policies:\n  - id: gospel-ceiling\n    version: 2\n    class: measure\n    \
             consumes: gospel-bytes@1\n    subject: CLAUDE.md\n    \
             measure:\n      kind: level\n      loc-hard: 12000\n",
        );
        let bounds = reg.ceiling_bounds();
        assert_eq!(bounds.len(), 1);
        assert_eq!(bounds[0].id, "gospel-ceiling@2");
        assert_eq!(bounds[0].hard, Some(12000.0));
        assert_eq!(bounds[0].soft, None);
        assert_eq!(bounds[0].source, BoundSource::Ceiling);
        assert!(bounds[0].matches("gospel-ceiling"));
        assert!(bounds[0].matches("gospel-ceiling@2"));
    }
}
