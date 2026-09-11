//! The reader lens — WHO is reading, resolved to a bounded rendering budget.
//!
//! Governed-discovery station 1
//! (`genesis/docs/superpowers/specs/2026-09-11-governed-discovery-journey-lens-graduation-design.md`
//! §"Reader lens (a preset per reader)"). `ResolveLens` composes three inputs, each with its own
//! provenance line, in this order:
//!
//! 1. **defaults** — the recipe's own declared floor (`lens_table.defaults`), always present.
//! 2. **stated** — the actor sidecar's `role@model` claim for this session, mapped by the
//!    recipe's `lens_table.stated` table to a starting [`LensLevel`]. A session with no claim,
//!    or a claim whose model the table does not name, falls back to the defaults honestly rather
//!    than inventing a tier nobody declared.
//! 3. **revealed** — this reader's own last three `recall-journey` folds (the four measures
//!    declared `family: recall-journey` in `.claude/epr-meta/measures.yaml`), read from
//!    `.eprfs/status/flows.jsonl` via [`crate::flow::report::read_folds`] and filtered to
//!    `env.reader == <this reader's label>`. Applying `lens_table.revealed_rule`: three clean
//!    journeys (no mistaken assertion) widen the offer one rung; a mistaken assertion anywhere in
//!    the last three narrows it one rung. Fewer than three folds, or no folds at all, is honest
//!    absence — the stated level stands.
//!
//! **`--lens <level>` is never refused.** An explicit request overrides steps 2-3 outright and
//! is named in `provenance.stated[0]` as `requested: <level> (stated <inner>)` — the widening
//! the reader asked for, next to what the recipe would otherwise have offered.
//!
//! **Never refuses — but absent and malformed are NOT the same shape.** [`resolve`] returns a
//! [`LensView`] unconditionally; an unreadable flows sidecar is read as zero revealed evidence
//! rather than an error, for the same reason. But `lens_table` itself splits in two:
//!
//! - **Absent** (a pre-v11 contract, or one that simply never declares it) falls back to
//!   [`builtin_table`] SILENTLY — `provenance.defaults` reads `"builtin"`. This is the legitimate
//!   additive-fallback discipline `Contract::process_spec`/`Contract::bounds` already hold for a
//!   field that genuinely does not exist yet.
//! - **Declared but malformed** (the field is present and fails to parse) is a DIFFERENT case and
//!   is never silent: `Contract::process_spec`/`Contract::bounds` REFUSE outright on this shape —
//!   "a hand-edit error, not a shape to silently paper over with a minted default" (Task 0.6
//!   hardened this after an earlier drift let one through quietly). A lens must not refuse the
//!   same way, so [`resolve`] still falls back to [`builtin_table`] — but SAYS SO: the message is
//!   pushed onto `view["unresolved"]` by the caller (`journey::execute`, via [`LensView::malformed`])
//!   and `provenance.defaults` reads `"builtin (declared lens_table malformed)"` rather than the
//!   silent `"builtin"` the absent case gets. A lens is orientation, not an admission gate — the
//!   honesty floor (every view names its lens, its CID and its provenance, AND names a fallback it
//!   was forced into) is what must never be skipped, not any one input to computing it.
//!
//! **Station 2/3 are not here yet.** `expires_at`/`tended_at` (the TTL and tending cadence the
//! design doc's human lens needs) stay `None`/empty — honest absence, since nothing in this
//! station writes a tending record. `ReaderRef::Human` is declared now so `resolve`'s match is
//! exhaustive against the shape the design doc names, but nothing produces it yet:
//! `reader_from_session` only ever returns `Agent` or `Unknown`.

use std::collections::BTreeMap;

use elohim_epr_rea::{parse_agent_ref, ActorStore, AgentRef, SidecarActorStore};

use super::*;

// ───────────────────────────────────────────────────────────────────────────────────────────────
// LensLevel / Scaffold — the graphos vocabulary, verbatim
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Six rungs, from a scaffold that hands exactly one command to a scaffold that ranks and lets
/// the reader choose among dozens. Spelled exactly as the recipe's `lens_table` and graphos both
/// already do — one vocabulary, not a Rust-side re-spelling of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LensLevel {
    // NOTE: `Copy` matters beyond convenience — `Args` is accessed through `&Args` at every call
    // site, so `args.lens` (an `Option<LensLevel>`) must be `Copy` to read by value there.
    Minimal,
    Simple,
    Standard,
    Detail,
    Debug,
    Trace,
}

impl LensLevel {
    /// Declaration order IS rung order — widening and narrowing walk this array, not a
    /// separately maintained ranking that could drift from it.
    const ALL: [LensLevel; 6] = [
        LensLevel::Minimal,
        LensLevel::Simple,
        LensLevel::Standard,
        LensLevel::Detail,
        LensLevel::Debug,
        LensLevel::Trace,
    ];

    pub(super) fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "minimal" => Some(LensLevel::Minimal),
            "simple" => Some(LensLevel::Simple),
            "standard" => Some(LensLevel::Standard),
            "detail" => Some(LensLevel::Detail),
            "debug" => Some(LensLevel::Debug),
            "trace" => Some(LensLevel::Trace),
            _ => None,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            LensLevel::Minimal => "minimal",
            LensLevel::Simple => "simple",
            LensLevel::Standard => "standard",
            LensLevel::Detail => "detail",
            LensLevel::Debug => "debug",
            LensLevel::Trace => "trace",
        }
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|level| *level == self)
            .unwrap_or(0)
    }

    /// One rung wider; saturates at `trace` rather than wrapping past the declared vocabulary.
    ///
    /// `pub(super)` (not private, like [`Self::prev`]): the density floor's "N more candidate(s)
    /// at a wider lens" note in `render.rs` names the next rung explicitly, so a reader knows
    /// exactly which `--lens` value widens past this render's soft cap.
    pub(super) fn next(self) -> Self {
        Self::ALL[(self.index() + 1).min(Self::ALL.len() - 1)]
    }

    /// One rung narrower; saturates at `minimal`.
    fn prev(self) -> Self {
        Self::ALL[self.index().saturating_sub(1)]
    }
}

/// The two scaffold shapes a level may declare, spelled exactly as `lens_table` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scaffold {
    LocateAndHandOneCommand,
    RankAndLetChoose,
}

impl Scaffold {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "locate-and-hand-one-command" => Some(Scaffold::LocateAndHandOneCommand),
            "rank-and-let-choose" => Some(Scaffold::RankAndLetChoose),
            _ => None,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Scaffold::LocateAndHandOneCommand => "locate-and-hand-one-command",
            Scaffold::RankAndLetChoose => "rank-and-let-choose",
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// ReaderRef — WHO is reading, resolved from the actor sidecar, never inferred
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// WHO is reading. Resolved from the actor sidecar's registered claim for the session — never
/// guessed from output shape, a header, or a flag naming a persona.
pub(super) enum ReaderRef {
    /// A claimed agent identity, `agent:<role>@<model>` — the ordinary case this station wires.
    Agent(AgentRef),
    /// The station-2/3 human lens: an imagodei capability profile (`CapabilityTier` plus the
    /// `@capabilityMaxLens`/stimulus/textuality vocabulary graphos already renders). Declared now
    /// so [`resolve`]'s match is exhaustive against the design this station is one rung of; not
    /// yet produced by [`reader_from_session`] or any other resolver here.
    #[allow(dead_code)]
    Human { agent_cid: String, tier: String },
    /// No claim was found for the session. Honest absence, not a refusal — a session that never
    /// called `epr actor claim` is a normal, common shape.
    Unknown,
}

/// The actor sidecar log, relative to root. Existence is checked before opening — the same
/// read-only discipline `note::claimed_for_session` and `flow::acceptance` already hold — so a
/// bare lens resolution never leaves `.eprfs/status/actors.jsonl` behind in a repository that
/// never had one.
const ACTOR_LOG_REL: &str = ".eprfs/status/actors.jsonl";

/// WHO is reading in `session`: the actor sidecar's current claim, or [`ReaderRef::Unknown`].
pub(super) fn reader_from_session(root: &Path, session: &str) -> ReaderRef {
    if session.trim().is_empty() || !root.join(ACTOR_LOG_REL).is_file() {
        return ReaderRef::Unknown;
    }
    match SidecarActorStore::open(root).and_then(|store| store.current_for(session)) {
        Ok(Some((_record_cid, claim))) => ReaderRef::Agent(claim.claimed),
        Ok(None) | Err(_) => ReaderRef::Unknown,
    }
}

/// The label a `recall-journey` fold's `env.reader` slot must carry to be this reader's own
/// evidence. Only an [`ReaderRef::Agent`] has one today — the human arm's evidence vocabulary
/// (Sophia `Recognition` records) is station 2/3 work.
fn reader_label(reader: &ReaderRef) -> Option<String> {
    match reader {
        ReaderRef::Agent(agent) => Some(agent.0.clone()),
        ReaderRef::Human { .. } | ReaderRef::Unknown => None,
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// LensProvenance / LensView
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Why the resolved level is what it is — printed on every view, never summarized away.
#[derive(Debug, Clone)]
pub(super) struct LensProvenance {
    /// The stated input's account, always exactly one line: `"none"` (no claim), `"<model> →
    /// <level>"` (a declared tier), or — when `--lens` was named — `"requested: <level> (stated
    /// <inner>)"`, where `<inner>` is the line the stated input would otherwise have produced.
    pub stated: Vec<String>,
    /// The revealed input's account. Empty when there is not yet enough of this reader's own
    /// journey history to say anything — never padded with a placeholder line.
    pub revealed: Vec<String>,
    /// WHERE the effective table came from: `"declared"` (the contract's own `lens_table`
    /// parsed cleanly), `"builtin"` (no `lens_table` declared — silent, legitimate fallback), or
    /// `"builtin (declared lens_table malformed)"` (a `lens_table` WAS declared and failed to
    /// parse — see [`LensView::malformed`], which carries the reason).
    pub defaults: String,
}

/// The resolved lens: a bounded rendering budget, its scaffold, its content address, and why.
#[derive(Debug, Clone)]
pub(super) struct LensView {
    pub level: LensLevel,
    pub choice_count: u8,
    pub density_bytes: usize,
    pub scaffold: Scaffold,
    /// `BlobCid::compute_raw` of the canonical JSON of `{level, choice_count, density_bytes,
    /// scaffold, provenance}` — contestable on sight, the design doc's own phrase for it.
    pub cid: String,
    pub provenance: LensProvenance,
    /// The TTL a tended lens expires by. `None` is honest absence: nothing in this station writes
    /// a tending record yet, so there is no clock to have started running.
    pub expires_at: Option<String>,
    /// Tending timestamps, oldest first. Always empty today, for the same reason.
    pub tended_at: Vec<String>,
    /// `Some(message)` exactly when the contract declared a `lens_table` that failed to parse —
    /// the message the caller must push onto `view["unresolved"]`. `None` on every other path
    /// (declared-and-valid, or genuinely absent), which is why it is NOT part of [`to_value`]:
    /// this is a signal for `journey::execute`, not a field of the lens itself.
    ///
    /// [`to_value`]: LensView::to_value
    pub malformed: Option<String>,
}

impl LensView {
    /// The view-shaped projection every operation carries as `view["lens"]`.
    pub(super) fn to_value(&self) -> Value {
        json!({
            "level": self.level.as_str(),
            "choice_count": self.choice_count,
            "density_bytes": self.density_bytes,
            "scaffold": self.scaffold.as_str(),
            "cid": self.cid,
            "provenance": {
                "stated": self.provenance.stated,
                "revealed": self.provenance.revealed,
                "defaults": self.provenance.defaults,
            },
            "expires_at": self.expires_at,
            "tended_at": self.tended_at,
        })
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// RenderFloor — the honesty floor and the content floor, never configurable
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Floors a lens may narrow past but never erase (design doc "Interfaces that guide
/// implementation", rule 2 and rule 3; "Anti-capture invariants" §Filter bubble).
///
/// `unfilterable` is the content-class floor: a candidate whose declared frontmatter
/// `content_class` names one of these renders regardless of the lens's `choice_count`, marked
/// `[floor]` — a correction, a counter-argument, an accountability record or an own-community
/// fact is never the thing a narrow lens quietly drops. `always_printed` is the honesty floor:
/// the five fields `render()` folds into one line at every lens, `minimal` included, never as
/// nothing — see [`super::render::render`].
///
/// Both fields are DECLARED HERE, not read from a flag or the contract: [`RenderFloor::declared`]
/// is the sole constructor, and it is a fixed set of literals, not a table lookup. A floor a
/// caller could widen or narrow by argument would not be a floor.
pub(super) struct RenderFloor {
    pub unfilterable: Vec<&'static str>,
    pub always_printed: [&'static str; 5],
}

impl RenderFloor {
    /// The constant floor every rendering is checked against. Values verbatim from the design
    /// doc and the station 1.2 brief — never read from `Contract`, never a CLI flag.
    pub(super) fn declared() -> Self {
        RenderFloor {
            unfilterable: vec![
                "correction",
                "counter-evidence",
                "accountability",
                "own-community",
            ],
            always_printed: ["recipe", "lens", "selection", "omissions", "receipts"],
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// lens_table — declared in the recipe, with a compiled-in fallback
// ───────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct LevelSpec {
    choice_count: u8,
    density_bytes: usize,
    scaffold: Scaffold,
}

#[derive(Debug)]
struct LensTable {
    defaults_level: LensLevel,
    stated: BTreeMap<String, LensLevel>,
    levels: BTreeMap<LensLevel, LevelSpec>,
}

/// The table this contract version declares, compiled in as the fallback for a contract that
/// predates `lens_table` or carries a malformed one — the same additive-fallback discipline
/// `Contract::process_spec`/`Contract::bounds` already hold. Byte-identical to the JSON
/// `.epr-meta/elohim/algorithms/recall-contract.json` declares as of contract v11.
fn builtin_table() -> LensTable {
    let levels = BTreeMap::from([
        (
            LensLevel::Minimal,
            LevelSpec {
                choice_count: 1,
                density_bytes: 1500,
                scaffold: Scaffold::LocateAndHandOneCommand,
            },
        ),
        (
            LensLevel::Simple,
            LevelSpec {
                choice_count: 3,
                density_bytes: 3000,
                scaffold: Scaffold::LocateAndHandOneCommand,
            },
        ),
        (
            LensLevel::Standard,
            LevelSpec {
                choice_count: 6,
                density_bytes: 6000,
                scaffold: Scaffold::RankAndLetChoose,
            },
        ),
        (
            LensLevel::Detail,
            LevelSpec {
                choice_count: 12,
                density_bytes: 12000,
                scaffold: Scaffold::RankAndLetChoose,
            },
        ),
        (
            LensLevel::Debug,
            LevelSpec {
                choice_count: 12,
                density_bytes: 24000,
                scaffold: Scaffold::RankAndLetChoose,
            },
        ),
        (
            LensLevel::Trace,
            LevelSpec {
                choice_count: 24,
                density_bytes: 32768,
                scaffold: Scaffold::RankAndLetChoose,
            },
        ),
    ]);
    let stated = BTreeMap::from([
        ("claude-haiku-4-5".to_string(), LensLevel::Minimal),
        ("claude-sonnet-5".to_string(), LensLevel::Simple),
        ("claude-opus-5".to_string(), LensLevel::Standard),
        ("claude-fable-5-1".to_string(), LensLevel::Detail),
        ("gpt-5.6-sol".to_string(), LensLevel::Standard),
    ]);
    LensTable {
        defaults_level: LensLevel::Standard,
        stated,
        levels,
    }
}

/// Resolve the effective `lens_table` and where it came from — see the module doc's "Never
/// refuses — but absent and malformed are NOT the same shape" section. Returns `(table,
/// defaults_provenance, malformed_message)`; `malformed_message` is `Some` exactly when a
/// declared `lens_table` failed to parse, and is the exact text [`resolve`] pushes onto
/// `view["unresolved"]` via [`LensView::malformed`].
fn table_from_contract(contract: &Contract) -> (LensTable, String, Option<String>) {
    match contract.value.get("lens_table") {
        None => (builtin_table(), "builtin".to_string(), None),
        Some(raw) => match parse_table(raw) {
            Ok(table) => (table, "declared".to_string(), None),
            Err(reason) => (
                builtin_table(),
                "builtin (declared lens_table malformed)".to_string(),
                Some(format!(
                    "lens_table declared but malformed: {reason}; builtin defaults applied"
                )),
            ),
        },
    }
}

/// Parse a declared `lens_table`, naming exactly what failed rather than collapsing every shape
/// of malformed into one opaque `None` — the message becomes part of the `unresolved` line a
/// reader actually sees.
fn parse_table(raw: &Value) -> Result<LensTable, String> {
    let defaults_level_str = raw
        .pointer("/defaults/level")
        .and_then(Value::as_str)
        .ok_or_else(|| "lens_table.defaults.level missing or not a string".to_string())?;
    let defaults_level = LensLevel::parse(defaults_level_str).ok_or_else(|| {
        format!("lens_table.defaults.level `{defaults_level_str}` is not a recognized level")
    })?;

    let mut levels = BTreeMap::new();
    for level in LensLevel::ALL {
        let name = level.as_str();
        let spec = raw
            .pointer(&format!("/levels/{name}"))
            .ok_or_else(|| format!("lens_table.levels.{name} missing"))?;
        let choice_count = spec
            .get("choice_count")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                format!("lens_table.levels.{name}.choice_count missing or not a number")
            })?;
        let choice_count = u8::try_from(choice_count)
            .map_err(|_| format!("lens_table.levels.{name}.choice_count out of range"))?;
        let density_bytes = spec
            .get("density_bytes")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                format!("lens_table.levels.{name}.density_bytes missing or not a number")
            })?;
        let density_bytes = usize::try_from(density_bytes)
            .map_err(|_| format!("lens_table.levels.{name}.density_bytes out of range"))?;
        let scaffold_str = spec
            .get("scaffold")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("lens_table.levels.{name}.scaffold missing or not a string"))?;
        let scaffold = Scaffold::parse(scaffold_str).ok_or_else(|| {
            format!("lens_table.levels.{name}.scaffold `{scaffold_str}` is not recognized")
        })?;
        levels.insert(
            level,
            LevelSpec {
                choice_count,
                density_bytes,
                scaffold,
            },
        );
    }

    let mut stated = BTreeMap::new();
    for (model, level) in raw
        .get("stated")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if let Some(level) = level.as_str().and_then(LensLevel::parse) {
            stated.insert(model.clone(), level);
        }
    }
    Ok(LensTable {
        defaults_level,
        stated,
        levels,
    })
}

impl LensTable {
    fn spec(&self, level: LensLevel) -> LevelSpec {
        self.levels
            .get(&level)
            .copied()
            .or_else(|| self.levels.get(&self.defaults_level).copied())
            .unwrap_or(LevelSpec {
                choice_count: 6,
                density_bytes: 6000,
                scaffold: Scaffold::RankAndLetChoose,
            })
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Stated — the actor sidecar's claim, mapped by the recipe's table
// ───────────────────────────────────────────────────────────────────────────────────────────────

fn stated_for(reader: &ReaderRef, table: &LensTable) -> (LensLevel, Vec<String>) {
    match reader {
        ReaderRef::Agent(agent) => match parse_agent_ref(&agent.0) {
            Ok((_role, model)) => match table.stated.get(&model) {
                Some(level) => (*level, vec![format!("{model} → {}", level.as_str())]),
                None => (
                    table.defaults_level,
                    vec![format!(
                        "{model} → {} (model not declared in lens_table.stated; recipe default)",
                        table.defaults_level.as_str()
                    )],
                ),
            },
            // A claimed identity the sidecar already validated at write time; this arm exists so
            // a hand-edited or foreign-written log line can never panic a read.
            Err(_) => (table.defaults_level, vec!["none".to_string()]),
        },
        ReaderRef::Human { tier, .. } => (
            table.defaults_level,
            vec![format!(
                "human:{tier} → {} (human lens not yet negotiated; recipe default)",
                table.defaults_level.as_str()
            )],
        ),
        ReaderRef::Unknown => (table.defaults_level, vec!["none".to_string()]),
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Revealed — this reader's own recall-journey folds
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The `family: recall-journey` measure this station reads for revealed evidence
/// (`.claude/epr-meta/measures.yaml`). The other three siblings
/// (`recall-metered-bytes`/`recall-unmetered-bytes`/`recall-screens-to-shape`) measure the
/// journey's COST; this one is its OUTCOME, which is what `revealed_rule` widens or narrows on.
const JOURNEY_OUTCOME_MEASURE: &str = "recall-mistaken-assertions";

/// Apply `lens_table.revealed_rule` to this reader's last three `recall-journey` folds. Returns
/// the offered level (unchanged from `stated` when there is not yet enough evidence) and the
/// provenance lines explaining why.
fn revealed_for(root: &Path, reader: &ReaderRef, stated: LensLevel) -> (LensLevel, Vec<String>) {
    let Some(label) = reader_label(reader) else {
        return (stated, Vec::new());
    };
    // Honest absence on any read failure — a sidecar this leg could not read carries no evidence,
    // not a refusal to resolve a lens.
    let folds = crate::flow::report::read_folds(root).unwrap_or_default();
    let mistaken_values: Vec<f64> = folds
        .iter()
        .filter(|fold| {
            fold.measure.id == JOURNEY_OUTCOME_MEASURE
                && fold.env.get("reader").map(String::as_str) == Some(label.as_str())
        })
        .map(|fold| fold.value)
        .collect();
    // `read_folds` returns records in sidecar append order, which is chronological; the last
    // three are the three most recently appended.
    let last_three: Vec<f64> = mistaken_values.iter().rev().take(3).copied().collect();
    if last_three.is_empty() {
        return (stated, Vec::new());
    }
    if last_three.iter().any(|value| *value > 0.0) {
        let offered = stated.prev();
        return (
            offered,
            vec![format!(
                "revealed: a mistaken assertion in this reader's last {} recall-journey fold(s) \
                 — offered {} instead of {}",
                last_three.len(),
                offered.as_str(),
                stated.as_str()
            )],
        );
    }
    if last_three.len() == 3 {
        let offered = stated.next();
        return (
            offered,
            vec![format!(
                "revealed: this reader's last 3 recall-journey folds reached authority with no \
                 mistaken assertion — offered {} instead of {}",
                offered.as_str(),
                stated.as_str()
            )],
        );
    }
    (
        stated,
        vec![format!(
            "revealed: {} of 3 recall-journey fold(s) recorded for this reader, none mistaken \
             — not enough evidence yet to widen past {}",
            last_three.len(),
            stated.as_str()
        )],
    )
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// resolve — the whole composition
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// `ResolveLens`: defaults, then stated, then revealed, then an explicit `--lens` request that
/// overrides the last two outright. Never refuses — see the module doc.
pub(super) fn resolve(
    reader: &ReaderRef,
    contract: &Contract,
    requested: Option<LensLevel>,
    root: &Path,
) -> LensView {
    let (table, defaults_provenance, malformed) = table_from_contract(contract);
    let (stated_level, stated_lines) = stated_for(reader, &table);
    let (offered_level, revealed_lines) = revealed_for(root, reader, stated_level);

    let level = requested.unwrap_or(offered_level);
    let stated_lines = match requested {
        Some(requested) => {
            let inner = stated_lines
                .first()
                .cloned()
                .unwrap_or_else(|| "none".to_string());
            vec![format!(
                "requested: {} (stated {inner})",
                requested.as_str()
            )]
        }
        None => stated_lines,
    };

    let spec = table.spec(level);
    let provenance = LensProvenance {
        stated: stated_lines,
        revealed: revealed_lines,
        defaults: defaults_provenance,
    };
    let cid = compute_cid(
        level,
        spec.choice_count,
        spec.density_bytes,
        spec.scaffold,
        &provenance,
    );

    LensView {
        level,
        choice_count: spec.choice_count,
        density_bytes: spec.density_bytes,
        scaffold: spec.scaffold,
        cid,
        provenance,
        expires_at: None,
        tended_at: Vec::new(),
        malformed,
    }
}

fn compute_cid(
    level: LensLevel,
    choice_count: u8,
    density_bytes: usize,
    scaffold: Scaffold,
    provenance: &LensProvenance,
) -> String {
    let payload = json!({
        "level": level.as_str(),
        "choice_count": choice_count,
        "density_bytes": density_bytes,
        "scaffold": scaffold.as_str(),
        "provenance": {
            "stated": provenance.stated,
            "revealed": provenance.revealed,
            "defaults": provenance.defaults,
        },
    });
    let raw = serde_json::to_vec(&payload).unwrap_or_default();
    BlobCid::compute_raw(&raw).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_parse_and_display_round_trip_the_graphos_vocabulary() {
        for level in LensLevel::ALL {
            assert_eq!(LensLevel::parse(level.as_str()), Some(level));
        }
        assert_eq!(LensLevel::parse("frobnicate"), None);
    }

    #[test]
    fn next_and_prev_saturate_at_the_vocabulary_edges() {
        assert_eq!(LensLevel::Trace.next(), LensLevel::Trace);
        assert_eq!(LensLevel::Minimal.prev(), LensLevel::Minimal);
        assert_eq!(LensLevel::Simple.next(), LensLevel::Standard);
        assert_eq!(LensLevel::Standard.prev(), LensLevel::Simple);
    }

    #[test]
    fn builtin_table_names_every_declared_level_and_stated_model() {
        let table = builtin_table();
        for level in LensLevel::ALL {
            let _ = table.spec(level);
        }
        assert_eq!(
            table.stated.get("claude-sonnet-5"),
            Some(&LensLevel::Simple)
        );
        assert_eq!(table.defaults_level, LensLevel::Standard);
    }

    #[test]
    fn an_unclaimed_session_resolves_to_the_recipe_default_with_no_evidence() {
        let (level, lines) = stated_for(&ReaderRef::Unknown, &builtin_table());
        assert_eq!(level, LensLevel::Standard);
        assert_eq!(lines, vec!["none".to_string()]);
    }

    #[test]
    fn a_claimed_model_absent_from_the_recipe_table_falls_back_honestly() {
        let reader = ReaderRef::Agent(AgentRef("agent:reader@some-new-model".into()));
        let (level, lines) = stated_for(&reader, &builtin_table());
        assert_eq!(level, LensLevel::Standard);
        assert!(lines[0].contains("some-new-model"));
        assert!(lines[0].contains("recipe default"));
    }

    #[test]
    fn an_absent_lens_table_is_silent_builtin() {
        let mut contract = crate::flow::memory::recall::tests_support::minimal_contract();
        contract.as_object_mut().unwrap().remove("lens_table");
        let (_table, provenance, malformed) =
            table_from_contract(&Contract::from_value(contract).expect("valid contract"));
        assert_eq!(provenance, "builtin");
        assert!(malformed.is_none());
    }

    #[test]
    fn a_declared_lens_table_that_fails_to_parse_is_never_silent() {
        let bad = json!({"levels": "nope"});
        let error = parse_table(&bad).expect_err("malformed table must not parse");
        assert!(error.contains("defaults.level"), "{error}");

        let mut contract = crate::flow::memory::recall::tests_support::minimal_contract();
        contract["lens_table"] = bad;
        let (_table, provenance, malformed) =
            table_from_contract(&Contract::from_value(contract).expect("valid contract"));
        assert_eq!(provenance, "builtin (declared lens_table malformed)");
        let message = malformed.expect("a malformed lens_table must be named, not swallowed");
        assert!(
            message.contains("lens_table declared but malformed"),
            "{message}"
        );
        assert!(message.contains("builtin defaults applied"), "{message}");
    }

    #[test]
    fn a_declared_lens_table_that_parses_wins_over_the_builtin() {
        let mut contract = crate::flow::memory::recall::tests_support::minimal_contract();
        contract["lens_table"]["stated"]["claude-sonnet-5"] = json!("detail");
        let (table, provenance, malformed) =
            table_from_contract(&Contract::from_value(contract).expect("valid contract"));
        assert_eq!(provenance, "declared");
        assert!(malformed.is_none());
        assert_eq!(
            table.stated.get("claude-sonnet-5"),
            Some(&LensLevel::Detail)
        );
    }
}
