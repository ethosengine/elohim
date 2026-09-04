//! `epr flow stocks` — the READ-ONLY equilibrium fold over the development system's stocks
//! (Spec B,
//! `genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md`;
//! harness-borrows plan Task 2).
//!
//! Every gate we run today reads a **level** — 560 unfulfilled commitments, cleanup pressure
//! 210/120, `MEMORY.md` over cap — and a level says nothing about controllability. Meadows'
//! correction is the whole of this leg: *"deforestation is indicated not when the forest is
//! gone, but when the rate of harvest first exceeds the rate of regrowth"*
//! (`elohim_epr_rea::stock`, module header). So the measure here is **two rates over one
//! declared window**, and equilibrium is `outflow >= inflow`.
//!
//! # This leg writes nothing
//!
//! It opens the sidecar, reads `records()`, folds, and prints — the read pattern
//! [`crate::flow::walk::status`] already uses. The derived [`FlowEvent`]s it builds are an
//! in-memory *view*, never appended: a stock is a pure fold, and a stored level would be a
//! second home for a number that already has one (`stock.rs:15-20`).
//!
//! # The whole risk is the derived view, not the arithmetic
//!
//! The raw sidecar records do not fold correctly as they stand, and the reason is a **sign
//! inversion arriving through the data**. `epr flow fulfill` mints a discharging fulfillment as
//! `ReaVerb::Produce` (`fulfill.rs:336-352`) — because from the *artifact's* point of view a
//! green run produced something. Fed to [`stock_over_window_within`] raw, every fulfillment
//! would land in **inflow**, and a stock that is draining fastest would read as the one filling
//! fastest. The fold's `match` arms read only `Produce`/`Consume` and silently drop everything
//! else (`stock.rs:423-437`), so nothing downstream would ever notice.
//!
//! [`classify_record`] is therefore the load-bearing function in this file, and it is a small
//! pure predicate with its own tests for exactly that reason. Against the *commitments* stock:
//!
//! - a **`Commitment` record that is open** (`Proposed | Active`) is the stock's **inflow** — a
//!   promise entered the stock when it was minted;
//! - an **event that names a commitment in `fulfills`** is the stock's **outflow** — the promise
//!   was discharged. The event's own `ReaVerb` is deliberately NOT consulted: `fulfills` is the
//!   fulfillment edge (the DHT spells it `bounded_by`) and it is the same field the level
//!   predicate upstream reads, so keying on it is what keeps the two in agreement;
//! - everything else moves this stock in neither direction, with the reason named
//!   ([`NotCounted`]) rather than dropped.
//!
//! # Two divergences from Spec B §3, both grounded in what the tree actually does
//!
//! Spec B §3 states the outflow as *"(fulfilled + dismissed + revoked) per week"*. Read against
//! the code, two of those three arms do not exist in the shape the spec assumed, and inventing
//! them would corrupt the level rather than complete it:
//!
//! 1. **`ReaVerb::Dismiss` is a REGRESSION marker here, not a commitment dismissal.**
//!    `fulfill.rs:360-393` mints one only when a commitment that was **already discharged** goes
//!    red — with `fulfills: []`, unit `red-run`, against the *artifact's* resource. Counting it
//!    as outflow would drain a promise this fold has already drained once, pushing the level
//!    below the `unfulfilled_total` every other reader quotes, and it would do so in the wrong
//!    direction: a regression is the opposite of a drain. It is [`NotCounted::DischargesNothing`].
//! 2. **`CommitmentState::Revoked` is structurally unreachable** (Spec B Q-C): no projection path
//!    mints a commitment in any state but `Active` (`project.rs:559`, `:596`). The revoke arm is
//!    therefore **absence, not a counted zero** — a non-open commitment contributes to neither
//!    flow, and if one ever appears its `NotOpenAtMint` classification is what a reader will see.
//!
//! The consequence is stated rather than hidden: **the outflow arm of the commitments stock is
//! fulfillment-only today**, and it is named in the rendered basis so a reader can disagree with
//! it.
//!
//! # A note is neither inflow nor outflow (Spec A/B Q2)
//!
//! Task 1's `epr flow note` mints `ReaVerb::Cite` events with `fulfills: []` and unit `run-note`.
//! A run-scale observation about a promise does not move the promise: it neither creates a new
//! obligation nor discharges an existing one. So a note falls out through
//! [`NotCounted::DischargesNothing`] on the classifier, and would fall out a *second* time on the
//! unit filter even if the classifier were wrong — the belt-and-braces is deliberate, because
//! `run-note` was chosen as a distinct unit precisely to keep notes out of every unit-keyed fold
//! by construction. Both exclusions are pinned by tests.
//!
//! # Three preconditions this leg states rather than assumes
//!
//! - **Lexicographic time.** `Window::contains` compares RFC3339 strings (`stock.rs:75-79`),
//!   which is correct only for a **uniform UTC** format: an event stamped `+02:00` sorts by its
//!   local wall clock, not its instant. Git's `%aI` emits the author's own offset, so this leg
//!   **normalizes every timestamp to `…Z`** on the way into the view rather than trusting that
//!   the tree happens to be UTC. An unparseable timestamp is honest absence, counted and
//!   reported, never coerced to a bound.
//! - **Exact-string units.** `count_in` admits only `Magnitude::Count { unit }` matching the
//!   requested unit and silently excludes the rest (`stock.rs:470-475`). One constant
//!   ([`COMMITMENT_UNIT`]) mints the view AND names the fold, and the count that the fold
//!   excluded is **derived from the fold's own output** and reported — a unit typo must surface
//!   as a refusal, never as a drained stock.
//! - **Turnover time is information, never a predicate.** `Level ÷ Rate` returns a bare `Level`
//!   whose period survives only in the `basis` string (`stock.rs:194-207`, spec Q15) — `3.0` is
//!   three weeks or three years depending on a divisor the type no longer holds. It is printed
//!   with its basis attached and never reaches [`equilibrium_verdict`].

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use cid::Cid;
// Re-exported: `parse_period` returns one and `parse_window` takes one, so a caller that uses
// this module's own API cannot name the type it needs without it.
pub use elohim_epr::measure::Period;
use elohim_epr_rea::{
    atom_cid, stock_over_window_within, AgentRef, AlgedonicKind, Commitment, CommitmentState,
    FlowEvent, FlowRecord, FlowStore, Magnitude, PinnedRef, ReaVerb, SidecarFlowStore, Stock,
    StockError, Window, Within,
};
use serde::Serialize;

use super::project::WIP_FENCE_UNIT;
use super::{
    producing_commit, repo_agent, repo_scope_atom, FlowError, FlowResult, Labels, REPO_AGENT,
};

/// The slot-0 tag on the WIP fence commitment `project` mints. One constant names the promise
/// and finds it again, so the minting leg and the reading leg cannot drift on the spelling.
const WIP_FENCE_TAG: &str = "register:wip-fence";

/// The unit every derived commitment-stock event is counted in.
///
/// ONE constant mints the view and names the fold. `count_in` matches units by exact string
/// (`stock.rs:470-475`), so two spellings of this word — here and at the call site — would
/// silently fold an empty stock and report it as drained. The single binding is the mechanical
/// guarantee that cannot happen.
pub const COMMITMENT_UNIT: &str = "commitment";

/// The stable id of the synthetic resource the commitments stock is folded on.
///
/// A stock needs *one* resource to fold over, and "the set of open promises" is not a file. The
/// id is minted the way [`crate::flow::repo_scope_atom`] mints the repo scope — a
/// [`PinnedRef`], deterministic and version-pinned — so two runs over the same sidecar address
/// the same stock with no coordination.
const COMMITMENT_STOCK_ID: &str = "stock:commitments";

// ---------------------------------------------------------------------------
// The classifier — the decision surface of this leg
// ---------------------------------------------------------------------------

/// What one sidecar record contributes to the **commitments** stock.
///
/// Direction, and only direction. Whether the referenced promise is one this stock actually
/// holds open, and whether it has already been drained once, are **identity** questions the view
/// builder answers with the record set in hand ([`derive_commitment_view`]); keeping them out of
/// here is what lets this predicate be a pure, exhaustively-tested function rather than a fold
/// with a lookup table inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitmentFlow {
    /// A promise entered the stock — projected as one `Produce`. **Inflow.**
    Minted,
    /// A promise left the stock — projected as one `Consume`. **Outflow.**
    Discharged,
    /// The record moves this stock in neither direction, for the named reason.
    Neither(NotCounted),
}

/// Why a record moves the commitments stock in neither direction.
///
/// Named rather than elided: "we did not count it" and "there was nothing to count" are the two
/// states this whole vocabulary exists to keep apart, and a reader auditing an unexpected level
/// needs to know which one a record landed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotCounted {
    /// A commitment that is not open at mint (`Fulfilled` / `Revoked`). Structurally unreachable
    /// on today's projection paths (Spec B Q-C) — kept as an explicit arm so that if one ever
    /// appears it reads as absence rather than being silently counted as an inflow that never
    /// drains.
    NotOpenAtMint,
    /// An event that discharges no promise: an artifact `Produce`, a red-run `Dismiss`
    /// regression marker, a `Cite` run-note. It may be a perfectly good observation about a
    /// commitment — it just does not move the commitment out of the stock, and `fulfills` being
    /// empty is precisely how the rest of this system already says so.
    DischargesNothing,
    /// An `Intent`, `Process`, `Spec` or `Edge` — the plan and knowledge planes. Real records,
    /// simply not flows of *this* stock.
    NotAFlowOfThisStock,
}

/// Classify one sidecar record against the commitments stock.
///
/// See the module header for why `fulfills` — not the event's `ReaVerb` — decides the outflow
/// arm. A `Produce` fulfillment and a `Consume` fulfillment are the same act to this stock; a
/// `Produce` that fulfills nothing is not an act of this stock at all.
pub fn classify_record(record: &FlowRecord) -> CommitmentFlow {
    match record {
        FlowRecord::Commitment(c) => match c.state {
            CommitmentState::Proposed | CommitmentState::Active => CommitmentFlow::Minted,
            CommitmentState::Fulfilled | CommitmentState::Revoked => {
                CommitmentFlow::Neither(NotCounted::NotOpenAtMint)
            }
        },
        FlowRecord::Event(e) => {
            if e.fulfills.is_empty() {
                CommitmentFlow::Neither(NotCounted::DischargesNothing)
            } else {
                CommitmentFlow::Discharged
            }
        }
        FlowRecord::Intent(_)
        | FlowRecord::Process(_)
        | FlowRecord::Spec(_)
        | FlowRecord::Edge(_) => CommitmentFlow::Neither(NotCounted::NotAFlowOfThisStock),
    }
}

// ---------------------------------------------------------------------------
// The derived view
// ---------------------------------------------------------------------------

/// The in-memory event view one stock is folded over, plus everything the derivation could not
/// resolve.
///
/// The unresolved counts are fields rather than log lines because they are the honest edge of
/// this measure: an undated mint is present in the level and absent from the rate, which
/// *understates inflow* — precisely the direction that would make a filling stock look calm.
/// A reader has to be able to see it.
#[derive(Debug)]
pub struct DerivedView {
    /// The synthetic resource every derived event is against.
    pub resource: Cid,
    /// `Produce` (mint) and `Consume` (discharge) events, in derivation order.
    pub events: Vec<FlowEvent>,
    /// Open commitments whose mint instant no git history could date. Counted in the level,
    /// absent from the inflow rate.
    pub undated_mints: usize,
    /// Discharge edges whose event carried no parseable `occurred_at`. Counted in the level,
    /// absent from the outflow rate.
    pub undated_discharges: usize,
    /// Discharge edges naming a commitment this stock does not hold open (an unknown CID, or one
    /// whose record is not `Proposed`/`Active`). Counted here rather than dropped, because a
    /// growing number would mean the fulfillment edge and the commitment set have drifted apart.
    pub discharges_of_unheld: usize,
    /// Discharge edges beyond the **chronologically first** one for a given commitment — the
    /// re-fulfillment path after a regression (`fulfill.rs`'s `refulfilled` arm). A promise can
    /// only leave the stock once, so these are excluded from the outflow; without the dedup the
    /// level would fall below the `unfulfilled_total` every other reader quotes. *Chronologically*
    /// first, never first-appended — see the sort in [`derive_commitment_view`].
    pub redundant_discharges: usize,
}

/// The synthetic commitments-stock resource CID.
pub fn commitment_stock_resource() -> FlowResult<Cid> {
    Ok(atom_cid(&PinnedRef {
        id: COMMITMENT_STOCK_ID.to_string(),
        version: 1,
    })?)
}

/// Project the sidecar's records into the commitments stock's flow view.
///
/// One pass classifies; a second pass over the (much smaller) commitment map dates the mints.
/// Mint instants are resolved through [`producing_commit`] on the artifact the promise is about
/// — the same history-derived clock every other timestamp on this path uses, never `now()` —
/// and memoized per repo-relative path, because 161 scenario commitments over ~50 gap documents
/// would otherwise spawn one `git log` per commitment instead of one per distinct artifact.
pub fn derive_commitment_view(
    root: &Path,
    labels: &Labels,
    records: &[(Cid, FlowRecord)],
) -> FlowResult<DerivedView> {
    let resource = commitment_stock_resource()?;
    let scope = repo_scope_atom()?;

    // Pass 1 — the open promise set, in append order, and the discharge edges.
    let mut open: Vec<(Cid, &Commitment)> = Vec::new();
    let mut open_set: HashSet<Cid> = HashSet::new();
    let mut discharges: Vec<(Cid, &str)> = Vec::new(); // (commitment cid, occurred_at)
    for (cid, record) in records {
        match (classify_record(record), record) {
            (CommitmentFlow::Minted, FlowRecord::Commitment(c)) => {
                if open_set.insert(*cid) {
                    open.push((*cid, c));
                }
            }
            (CommitmentFlow::Discharged, FlowRecord::Event(e)) => {
                for commitment in &e.fulfills {
                    discharges.push((*commitment, e.occurred_at.as_str()));
                }
            }
            _ => {}
        }
    }

    // Order the discharge edges by WHEN THEY HAPPENED, not by where they sit in the log.
    //
    // This crate has already shipped the append-order-vs-`occurredAt` bug once — `a83b35079`
    // (2026-07-29), whose own commit body reads *"the exact 8a1c531dc bug, reintroduced via
    // replay"* — and this pass is a second site with the same shape: `fulfill`'s re-fulfillment
    // arm appends a SECOND `fulfills` edge for one promise, and a delayed or backfilled report
    // can append an OLDER discharge after a newer one. The dedup below keeps the first edge it
    // sees, so keying on position would date a drain by whichever report happened to land first
    // and could file it in the wrong window entirely.
    //
    // Undated edges sort LAST (`Option`'s `None < Some` inverted by the `is_none()` key), so a
    // discharge we can place always wins the slot over one we cannot; the sort is stable, so
    // append order still breaks genuine ties — the same `(timestamp, then arrival)` shape the
    // zome selector settled on.
    discharges.sort_by_cached_key(|(_, occurred_at)| {
        let normalized = normalize_utc(occurred_at);
        (normalized.is_none(), normalized)
    });

    let mut events = Vec::with_capacity(open.len() + discharges.len());
    let mut undated_mints = 0usize;

    // Pass 2 — date each mint from the artifact's own history.
    //
    // An undatable mint is emitted with an EMPTY `occurred_at` rather than dropped, and the
    // distinction is load-bearing. The fold skips events at-or-after `window.end` but counts
    // everything before it in the level, and `""` sorts below every RFC3339 string — so an empty
    // timestamp lands in the level and in no window's rate. That is exactly the honest reading
    // (*the promise exists; we cannot say when it arrived*), and it is the same spelling
    // `project.rs:487` already uses for a resource whose history is unreadable. Dropping the
    // event instead — the first shape this took — silently pushed the level BELOW the
    // `unfulfilled_total` every other reader quotes, which is the one number this fold must
    // agree with. Caught by `an_undatable_mint_is_reported_not_guessed`.
    let mut when: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (cid, commitment) in &open {
        let occurred_at = match mint_instant(root, labels, commitment, &mut when) {
            Some(instant) => instant,
            None => {
                undated_mints += 1;
                String::new()
            }
        };
        events.push(stock_event(
            ReaVerb::Produce,
            resource,
            scope,
            occurred_at,
            mint_label(commitment, cid),
        ));
    }

    // Pass 3 — the discharges, deduped: a promise leaves the stock exactly once.
    let mut drained: HashSet<Cid> = HashSet::new();
    let (mut undated_discharges, mut discharges_of_unheld, mut redundant_discharges) = (0, 0, 0);
    for (commitment, occurred_at) in discharges {
        if !open_set.contains(&commitment) {
            discharges_of_unheld += 1;
            continue;
        }
        if !drained.insert(commitment) {
            redundant_discharges += 1;
            continue;
        }
        // Same rule as the mints above: an undatable discharge drains the level (the promise IS
        // discharged — `fulfills` says so with no reference to a clock) and is absent from the
        // outflow rate.
        let ts = match normalize_utc(occurred_at) {
            Some(ts) => ts,
            None => {
                undated_discharges += 1;
                String::new()
            }
        };
        events.push(stock_event(
            ReaVerb::Consume,
            resource,
            scope,
            ts,
            format!("discharge:{commitment}"),
        ));
    }

    Ok(DerivedView {
        resource,
        events,
        undated_mints,
        undated_discharges,
        discharges_of_unheld,
        redundant_discharges,
    })
}

/// One derived event. `fulfills`/`satisfies` stay empty and `process` stays `None`: this record
/// is never appended, and inventing edges on an in-memory projection would be inviting someone
/// to persist it later.
fn stock_event(
    action: ReaVerb,
    resource: Cid,
    in_scope_of: Cid,
    occurred_at: String,
    label: String,
) -> FlowEvent {
    FlowEvent {
        action,
        provider: AgentRef(REPO_AGENT.to_string()),
        receiver: repo_agent(),
        resource,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: COMMITMENT_UNIT.to_string(),
        },
        process: None,
        in_scope_of,
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        classified_as: vec![COMMITMENT_STOCK_ID.to_string(), label],
        occurred_at,
    }
}

fn mint_label(commitment: &Commitment, cid: &Cid) -> String {
    match commitment.resource_spec.classified_as.as_slice() {
        [tag, subject, ..] => format!("{tag}:{subject}"),
        [tag] => tag.clone(),
        [] => format!("mint:{cid}"),
    }
}

/// When did this promise enter the stock?
///
/// `Commitment::valid_from` is the declared answer and is honored first — but **both** mint sites
/// leave it `None` (`project.rs:558`, `:594`), so in practice the instant comes from the
/// artifact the promise is about, through the same `producing_commit` helper `derive_process_doc`
/// already uses:
///
/// - a **scenario** commitment carries its feature path at `classified_as[1]`
///   (`project.rs:590`), so its mint instant resolves exactly;
/// - a **gap-item claim** carries `["gap:claimed", item.id]` (`project.rs:529-532`) and no path
///   at all, so it resolves only as coarse as its gap *document*, reached through the label index
///   on `in_scope_of`. That coarseness biases such a mint toward the doc's own add date and is
///   Spec B's open question Q-B; it is recorded here rather than papered over with a narrower
///   number than the input supports.
///
/// `None` — no `valid_from`, no resolvable path, or no git history for it — is honest absence and
/// is counted, never substituted.
fn mint_instant(
    root: &Path,
    labels: &Labels,
    commitment: &Commitment,
    cache: &mut BTreeMap<String, Option<String>>,
) -> Option<String> {
    if let Some(declared) = commitment.valid_from.as_deref() {
        if let Some(ts) = normalize_utc(declared) {
            return Some(ts);
        }
    }
    let rel = match commitment.resource_spec.classified_as.as_slice() {
        [tag, subject, ..] if tag == "a2o:scenario-green" => subject.clone(),
        _ => labels.get(&commitment.in_scope_of.to_string())?.clone(),
    };
    if let Some(hit) = cache.get(&rel) {
        return hit.clone();
    }
    // Only the instant is wanted here: a mint is dated by the artifact's arrival, and who else
    // was named on that commit says nothing about when the promise started.
    let resolved = producing_commit(root, &rel).and_then(|p| normalize_utc(&p.occurred_at));
    cache.insert(rel, resolved.clone());
    resolved
}

/// Normalize an RFC3339 timestamp to a uniform `…Z` UTC spelling.
///
/// [`Window::contains`] compares timestamps **lexicographically**, which is right for uniform
/// UTC and silently wrong for mixed offsets — `2026-08-13T01:00:00+02:00` sorts *after*
/// `2026-08-13T00:30:00Z` by string even though it is the earlier instant. Git's `%aI` emits the
/// author's own offset, so normalizing here is what makes the window predicate mean what it
/// says. An unparseable or empty value returns `None` (honest absence) rather than a guess.
fn normalize_utc(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = chrono::DateTime::parse_from_rfc3339(trimmed).ok()?;
    Some(
        parsed
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    )
}

// ---------------------------------------------------------------------------
// Window construction
// ---------------------------------------------------------------------------

/// Parse `--per`. The vocabulary is [`Period`]'s, minus the two whose length is not fixed.
///
/// `month` and `year` are **refused, not approximated**. A month is 28-31 days and a year is
/// 365 or 366, so a `periods` count for either is a calendar assumption buried in a helper —
/// exactly the shape `MeasureKind::divide` refuses when it declines to convert a per-day rate to
/// a per-year one (`measure.rs:78-80`). The same window expressed in days or weeks is exact.
pub fn parse_period(raw: &str) -> FlowResult<Period> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "second" => Ok(Period::Second),
        "minute" => Ok(Period::Minute),
        "hour" => Ok(Period::Hour),
        "day" => Ok(Period::Day),
        "week" => Ok(Period::Week),
        other @ ("month" | "year") => Err(FlowError::InvalidArguments(format!(
            "--per {other} is refused: a {other} has no fixed length, so the period count would \
             be a calendar guess this leg will not make — express the same window in days or \
             weeks"
        ))),
        other => Err(FlowError::InvalidArguments(format!(
            "unknown period `{other}` — the vocabulary is second|minute|hour|day|week"
        ))),
    }
}

fn period_seconds(per: Period) -> i64 {
    match per {
        Period::Second => 1,
        Period::Minute => 60,
        Period::Hour => 3_600,
        Period::Day => 86_400,
        Period::Week => 604_800,
        // Unreachable: `parse_period` refuses both. Kept total rather than `unreachable!()` so a
        // future caller constructing a Window directly cannot panic this helper.
        Period::Month => 30 * 86_400,
        Period::Year => 365 * 86_400,
    }
}

/// Build the one declared [`Window`] from `--window START..END` and `--per`.
///
/// `Window::periods` is declared by the caller and never inferred by the type
/// (`stock.rs:70-73`) — *"it forces the author to state the denominator their rate is actually
/// per"*. The CLI **is** that caller, and `--per` is where the author states it; the number of
/// periods is then arithmetic over the two declared endpoints, computed once, in the open, and
/// printed in the basis so a reader can disagree with it. The window is not defaulted: a weekly
/// equilibrium claim that hides its window is not a claim, and a default would be a wall-clock
/// read on a path whose whole discipline is history-derived time.
///
/// Endpoints accept a bare `YYYY-MM-DD` (read as midnight UTC) or a full RFC3339 timestamp, and
/// both are normalized to the uniform `…Z` spelling the lexicographic `contains` requires.
pub fn parse_window(spec: &str, per: Period) -> FlowResult<Window> {
    let (raw_start, raw_end) = spec.trim().split_once("..").ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "--window wants START..END (e.g. 2026-08-06..2026-08-13), got `{spec}`"
        ))
    })?;
    let start = parse_bound(raw_start, "start")?;
    let end = parse_bound(raw_end, "end")?;
    if end <= start {
        return Err(FlowError::InvalidArguments(format!(
            "--window end ({end}) must be after start ({start}) — a window that spans no time \
             cannot carry a rate"
        )));
    }
    let span = (end - start).num_seconds();
    let periods = span as f64 / period_seconds(per) as f64;
    if !periods.is_finite() || periods <= 0.0 {
        return Err(FlowError::InvalidArguments(format!(
            "--window spans {periods} {per:?}-periods — a rate needs a positive denominator"
        )));
    }
    Ok(Window {
        start: start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        end: end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        per,
        periods,
    })
}

fn parse_bound(raw: &str, side: &str) -> FlowResult<chrono::DateTime<chrono::Utc>> {
    let text = raw.trim();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(text) {
        return Ok(dt.with_timezone(&chrono::Utc));
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Ok(chrono::DateTime::from_naive_utc_and_offset(
            date.and_hms_opt(0, 0, 0).expect("midnight is a valid time"),
            chrono::Utc,
        ));
    }
    Err(FlowError::InvalidArguments(format!(
        "--window {side} `{text}` is neither YYYY-MM-DD nor an RFC3339 timestamp"
    )))
}

// ---------------------------------------------------------------------------
// The verdict
// ---------------------------------------------------------------------------

/// What `--check` decides about one stock over one window.
///
/// Three outcomes, and the third is the one that matters: **a fold that produced no comparable
/// pair is a refusal, never a pass.** "We cannot see the drain" and "the drain is adequate" are
/// the two states this entire vocabulary exists to keep apart (`stock.rs:200`,
/// `stock.rs:315-319`), and a check that greens because it measured nothing is the over-claim the
/// habits covenant names in its own text.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "verdict", rename_all = "kebab-case")]
pub enum EquilibriumVerdict {
    /// `outflow >= inflow` over a window that actually observed events. Equilibrium.
    Draining,
    /// `inflow > outflow`. The stock is filling; the named stock is the finding.
    Filling,
    /// No verdict was reachable. Fail-closed.
    Refused { reason: RefusalReason },
    /// A BOUNDED stock inside its declared band. The analog of [`Self::Draining`] for a stock
    /// whose level is projected rather than folded: nothing is flowing to compare, and the
    /// bound's question — "is the level admissible" — is answered yes.
    WithinBound,
    /// The band edge is crossed and the limit is not: the signal BEFORE the line. It warns and
    /// does not refuse, because a fence that failed on approach would be a fence at the band
    /// edge, which is not the number anyone declared.
    AtBandEdge,
    /// The limit itself is crossed.
    OverBound,
}

/// Why no verdict was reachable — typed, so `--check`'s non-zero exit can say which absence it
/// hit rather than emitting a bare failure.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum RefusalReason {
    /// The window contained no events of this stock's unit at all. Either nothing happened or
    /// the unit does not match what the view minted; both are absence, and absence is not green.
    NoObservations,
    /// [`Stock::new`] or the fold refused. `kind` is the [`StockError`] variant's stable
    /// discriminant so a caller can branch without string-matching a `Display` message.
    FoldRefused { kind: &'static str, error: String },
    /// A rate came back `NaN`. `divide` mints NaN exactly where an interval declares the answer
    /// unknowable (`stock.rs:315-319`), specifically so no comparison silently passes it — this
    /// arm is that intent honored at the comparison site.
    NotComparable,
}

impl RefusalReason {
    fn from_stock_error(error: &StockError) -> Self {
        let kind = match error {
            StockError::LevelIsNotALevel(_) => "level-is-not-a-level",
            StockError::FlowIsNotARate { .. } => "flow-is-not-a-rate",
            StockError::MismatchedPeriods { .. } => "mismatched-periods",
            StockError::UndefinedDivision(_, _) => "undefined-division",
            StockError::EmptyWindow(_) => "empty-window",
        };
        RefusalReason::FoldRefused {
            kind,
            error: error.to_string(),
        }
    }
}

impl EquilibriumVerdict {
    /// Does this verdict let `--check` exit 0? Only `Draining` does.
    /// What `--check` passes on. A bounded stock passes inside its band AND at the band edge —
    /// an approach is a warning, and the number that refuses is the limit.
    pub fn is_equilibrium(&self) -> bool {
        matches!(
            self,
            EquilibriumVerdict::Draining
                | EquilibriumVerdict::WithinBound
                | EquilibriumVerdict::AtBandEdge
        )
    }

    pub fn word(&self) -> &'static str {
        match self {
            EquilibriumVerdict::Draining => "DRAINING",
            EquilibriumVerdict::Filling => "FILLING",
            EquilibriumVerdict::Refused { .. } => "REFUSED",
            EquilibriumVerdict::WithinBound => "WITHIN-BOUND",
            EquilibriumVerdict::AtBandEdge => "AT-BAND-EDGE",
            EquilibriumVerdict::OverBound => "OVER-BOUND",
        }
    }
}

/// **The** verdict function: drain ≥ inflow, over one declared window, or a typed refusal.
///
/// Takes the fold's `Result` rather than a `Stock`, because the refusals are half the semantics:
/// a `MismatchedPeriods` from [`Stock::new`] must produce a non-zero exit and **no partial
/// verdict**, and a signature that could only accept a successful fold would force the call site
/// to invent that behaviour (or forget to).
///
/// `observed_in_window` is passed in rather than recomputed from the `Stock`, because the two
/// distinguishable zero-drain states — *nothing happened* and *everything was excluded by unit* —
/// both arrive as `outflow == 0.0` and only the observation count tells them apart from a
/// genuinely balanced window.
///
/// Turnover time is deliberately absent from this signature (spec Q15).
pub fn equilibrium_verdict(
    folded: &Result<Stock, StockError>,
    observed_in_window: usize,
) -> EquilibriumVerdict {
    let stock = match folded {
        Ok(stock) => stock,
        Err(error) => {
            return EquilibriumVerdict::Refused {
                reason: RefusalReason::from_stock_error(error),
            }
        }
    };
    if observed_in_window == 0 {
        return EquilibriumVerdict::Refused {
            reason: RefusalReason::NoObservations,
        };
    }
    if stock.inflow.value.is_nan() || stock.outflow.value.is_nan() {
        return EquilibriumVerdict::Refused {
            reason: RefusalReason::NotComparable,
        };
    }
    if stock.outflow.value >= stock.inflow.value {
        EquilibriumVerdict::Draining
    } else {
        EquilibriumVerdict::Filling
    }
}

// ---------------------------------------------------------------------------
// The leg
// ---------------------------------------------------------------------------

/// The closed set of stocks this leg can fold.
///
/// `commitments` is the only one at birth, and Spec B §6's later stocks (memory-index bytes,
/// cleanup pressure) arrive **through this flag** as authored `run:observation` events folded by
/// the same code over the same `Window` — never as a second measurement path. An unknown
/// `--stock` is refused naming the legal set, the way `NoteKind` refuses a fourth kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StockName {
    Commitments,
    /// The habits the register declares `active: true`, judged against the WIP fence's Bound.
    ///
    /// A different KIND of stock from `commitments`, and the difference is the point: its level
    /// is a PROJECTION read from the register at read time, not a fold over events. So it has no
    /// inflow and no outflow to compare, and the question it answers is not "is the drain
    /// adequate" but "is the level admissible" — which is what a bound is for.
    ActiveHabits,
}

impl StockName {
    pub fn parse(raw: &str) -> FlowResult<Self> {
        match raw.trim() {
            "commitments" => Ok(StockName::Commitments),
            "active-habits" => Ok(StockName::ActiveHabits),
            other => Err(FlowError::InvalidArguments(format!(
                "unknown stock `{other}` — the set is: commitments|active-habits"
            ))),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            StockName::Commitments => "commitments",
            StockName::ActiveHabits => "active-habits",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            StockName::Commitments => COMMITMENT_UNIT,
            StockName::ActiveHabits => WIP_FENCE_UNIT,
        }
    }
}

/// The window, rendered for a reader who has to be able to disagree with it.
#[derive(Debug, Clone, Serialize)]
pub struct WindowView {
    pub start: String,
    pub end: String,
    pub per: String,
    pub periods: f64,
}

/// The three numbers, each with the basis the fold produced for it.
#[derive(Debug, Clone, Serialize)]
pub struct StockMeasures {
    pub level: f64,
    pub inflow: f64,
    pub outflow: f64,
    pub net: Option<f64>,
    pub level_basis: String,
    pub inflow_basis: String,
    pub outflow_basis: String,
    /// Turnover time — **information only**. `NaN` when the drain band admits zero, which is
    /// honest absence rather than `+∞`.
    pub turnover_time: Option<f64>,
    /// The basis carrying the period `turnover_time` is denominated in — which the type does not
    /// (spec Q15). Never read a bare turnover number without it.
    pub turnover_basis: Option<String>,
}

/// One stock's readout.
#[derive(Debug, Clone, Serialize)]
pub struct StockReport {
    pub stock: String,
    pub unit: String,
    pub verdict: EquilibriumVerdict,
    /// `None` exactly when the **fold itself** refused (`Stock::new` never returned a stock), so
    /// a `MismatchedPeriods` red carries no half-numbers beside it — there are none to carry.
    ///
    /// A `NoObservations` refusal is the other case and deliberately KEEPS its measures: the fold
    /// succeeded, the level is a real cumulative count, and the `0.000 / 0.000` rates beside a
    /// `REFUSED` verdict are the evidence for the refusal rather than a partial verdict. Blanking
    /// them would hide the very thing a reader needs — that the stock is real and the window is
    /// where the silence is.
    pub measures: Option<StockMeasures>,
    pub observed_in_window: usize,
    /// Events inside the window the fold excluded because their unit did not match — derived
    /// from the fold's own output, not from a second copy of `count_in`.
    pub excluded_by_unit: usize,
    pub undated_mints: usize,
    pub undated_discharges: usize,
    pub discharges_of_unheld: usize,
    pub redundant_discharges: usize,
    /// A projected level judged against a declared [`Bound`]. `None` for every event-derived
    /// stock, and skipped on the wire, so the `commitments` payload is byte-identical to the one
    /// emitted before bounded stocks existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound: Option<BoundReading>,
}

/// A level read from a register, and the band it was judged in.
///
/// Every field a reader needs to disagree with the verdict is here — the level, the limit, the
/// band edge, the promise the limit came from, and where the level itself was read. A bound
/// verdict with no basis is an assertion.
#[derive(Debug, Clone, Serialize)]
pub struct BoundReading {
    pub level: f64,
    pub limit: f64,
    pub band_edge: f64,
    pub unit: String,
    /// The fence commitment whose `Bound` this level was judged against — the promise, by
    /// address, so the verdict can be traced to the thing that declared it.
    pub bound_ref: String,
    /// `algedonic-approach` at the band edge, `algedonic-breach` past the limit, absent inside
    /// the band. The vocabulary is [`elohim_epr_rea::AlgedonicKind`]'s, not a second one.
    pub signal: Option<String>,
    /// Where the level came from. Stated because it is NOT event-derived, and a reader who
    /// assumed it was would draw a rate out of it.
    pub level_basis: String,
}

/// The whole readout.
#[derive(Debug, Clone, Serialize)]
pub struct StocksOutcome {
    pub window: WindowView,
    pub stocks: Vec<StockReport>,
    /// True only when **every** stock's verdict passes: `Draining` for a folded stock, inside
    /// the band (or at its edge) for a bounded one. This is what `--check` exits on.
    pub equilibrium: bool,
}

impl StocksOutcome {
    pub fn render(&self) {
        println!("epr flow stocks");
        println!(
            "  window: {} .. {} ({} {}-periods)",
            self.window.start, self.window.end, self.window.periods, self.window.per
        );
        for stock in &self.stocks {
            println!("  stock: {} (unit \"{}\")", stock.stock, stock.unit);
            match &stock.measures {
                Some(m) => {
                    println!("    level:    {:.0} {}", m.level, stock.unit);
                    println!("    inflow:   {:.3} per {}", m.inflow, self.window.per);
                    println!("    outflow:  {:.3} per {}", m.outflow, self.window.per);
                    match m.net {
                        Some(net) => {
                            println!("    net:      {:+.3} per {}", net, self.window.per)
                        }
                        None => println!("    net:      unknown (period mismatch)"),
                    }
                    println!("    verdict:  {}", stock.verdict.word());
                    if let EquilibriumVerdict::Refused { reason } = &stock.verdict {
                        println!("    reason:   {}", refusal_line(reason));
                    }
                    if let (Some(t), Some(basis)) = (m.turnover_time, m.turnover_basis.as_ref()) {
                        // Never a predicate: the period this number is denominated in survives
                        // only in the basis printed beside it (spec Q15).
                        println!("    turnover: {t:.2} (information only — basis: {basis})");
                    }
                    println!("    basis:");
                    println!("      level:   {}", m.level_basis);
                    println!("      inflow:  {}", m.inflow_basis);
                    println!("      outflow: {}", m.outflow_basis);
                }
                None => {
                    println!("    verdict:  {}", stock.verdict.word());
                    if let EquilibriumVerdict::Refused { reason } = &stock.verdict {
                        println!("    reason:   {}", refusal_line(reason));
                    }
                }
            }
            println!(
                "    observed in window: {} · excluded by unit: {}",
                stock.observed_in_window, stock.excluded_by_unit
            );
            if stock.undated_mints > 0 || stock.undated_discharges > 0 {
                println!(
                    "    undated: {} mints, {} discharges (present in the level, absent from \
                     the rate — inflow is understated by exactly this much)",
                    stock.undated_mints, stock.undated_discharges
                );
            }
            if stock.discharges_of_unheld > 0 || stock.redundant_discharges > 0 {
                println!(
                    "    discharge edges not counted: {} name no open promise, {} re-discharge \
                     one already drained",
                    stock.discharges_of_unheld, stock.redundant_discharges
                );
            }
            if let Some(bound) = &stock.bound {
                println!("    level:    {:.0} {}", bound.level, bound.unit);
                println!(
                    "    band:     limit {:.0}, band edge {:.0} (ceiling — the fabric breaches \
                     at level >= limit)",
                    bound.limit, bound.band_edge
                );
                println!("    verdict:  {}", stock.verdict.word());
                match &bound.signal {
                    Some(signal) => println!("    signal:   {signal}"),
                    None => println!("    signal:   none (inside the band)"),
                }
                println!("    bound:    {}", bound.bound_ref);
                println!("    basis:    {}", bound.level_basis);
            }
            println!(
                "    outflow arm: fulfillment only — Dismiss is a regression marker (empty \
                 `fulfills`) and no path mints a Revoked commitment"
            );
        }
        println!(
            "  equilibrium (every stock draining): {}",
            if self.equilibrium { "yes" } else { "no" }
        );
    }
}

fn refusal_line(reason: &RefusalReason) -> String {
    match reason {
        RefusalReason::NoObservations => {
            "no events of this stock's unit fell inside the declared window — absence, not a \
             drained stock"
                .to_string()
        }
        RefusalReason::FoldRefused { kind, error } => format!("{kind}: {error}"),
        RefusalReason::NotComparable => {
            "a rate came back NaN — the comparison is refused rather than silently passed"
                .to_string()
        }
    }
}

/// `epr flow stocks --window START..END --per week [--stock commitments] [--check]`.
///
/// Read-only end to end: opens the sidecar, reads `records()`, folds, returns. Nothing is
/// appended, so this is safe to run against a shared worktree while other work is in flight.
pub fn stocks(root: &Path, window: &Window, names: &[StockName]) -> FlowResult<StocksOutcome> {
    let store = SidecarFlowStore::open(root)?;
    let records = store.records()?;
    let labels = load_labels(root);

    let mut reports = Vec::with_capacity(names.len());
    for name in names {
        let report = match name {
            StockName::Commitments => {
                let view = derive_commitment_view(root, &labels, &records)?;
                report(*name, &view, window, name.unit())
            }
            // Not folded. Its level is read, and the window is irrelevant to it — which the
            // reading says out loud rather than leaving a reader to infer from a missing rate.
            StockName::ActiveHabits => active_habits_report(root, &records),
        };
        reports.push(report);
    }

    Ok(StocksOutcome {
        window: WindowView {
            start: window.start.clone(),
            end: window.end.clone(),
            per: format!("{:?}", window.per),
            periods: window.periods,
        },
        equilibrium: reports.iter().all(|r| r.verdict.is_equilibrium()),
        stocks: reports,
    })
}

/// The WIP fence reading: a PROJECTED level from the habit register, judged against the Bound
/// the fence commitment declares.
///
/// Two refusals, and both are the same principle this whole leg exists for — an absence must
/// never read as a zero. An unreadable register is not a repository with no active habits, and a
/// missing fence commitment is not a repository with no limit; both are "we cannot see it", and
/// `--check` must go non-zero on either.
///
/// The bound is READ from the commitment rather than restated here. The promise is the authority
/// on its own limit; a constant in this function would be a second declaration of it, free to
/// drift from the one the projection minted.
fn active_habits_report(root: &Path, records: &[(Cid, FlowRecord)]) -> StockReport {
    let name = StockName::ActiveHabits;
    let refused = |kind: &'static str, error: String| StockReport {
        stock: name.label().to_string(),
        unit: name.unit().to_string(),
        verdict: EquilibriumVerdict::Refused {
            reason: RefusalReason::FoldRefused { kind, error },
        },
        measures: None,
        observed_in_window: 0,
        excluded_by_unit: 0,
        undated_mints: 0,
        undated_discharges: 0,
        discharges_of_unheld: 0,
        redundant_discharges: 0,
        bound: None,
    };

    let habits = match super::registers::read_habits(root) {
        Ok(habits) => habits,
        Err(error) => {
            return refused(
                "register-unreadable",
                format!(
                    "{error} — an unreadable register is not a repository with zero active habits"
                ),
            )
        }
    };
    let level = habits.iter().filter(|habit| habit.active).count() as f64;

    let fence = records.iter().find_map(|(cid, record)| match record {
        FlowRecord::Commitment(commitment)
            if commitment.state == CommitmentState::Active
                && commitment
                    .resource_spec
                    .classified_as
                    .first()
                    .map(String::as_str)
                    == Some(WIP_FENCE_TAG) =>
        {
            commitment.bound.as_ref().map(|bound| (*cid, bound))
        }
        _ => None,
    });
    let Some((fence_cid, bound)) = fence else {
        return refused(
            "no-fence-commitment",
            "no active `register:wip-fence` commitment carries a bound — run `epr flow project` \
             to mint it from the habits covenant"
                .to_string(),
        );
    };

    let (verdict, signal) = if bound.breached_by(level) {
        (
            EquilibriumVerdict::OverBound,
            Some(AlgedonicKind::Breach.as_str().to_string()),
        )
    } else if bound.approached_by(level) {
        (
            EquilibriumVerdict::AtBandEdge,
            Some(AlgedonicKind::Approach.as_str().to_string()),
        )
    } else {
        (EquilibriumVerdict::WithinBound, None)
    };

    StockReport {
        stock: name.label().to_string(),
        unit: name.unit().to_string(),
        verdict,
        // Deliberately absent. A projected level has no inflow and no outflow, and a zero in
        // either slot would be read as a measured rate of nothing happening.
        measures: None,
        observed_in_window: 0,
        excluded_by_unit: 0,
        undated_mints: 0,
        undated_discharges: 0,
        discharges_of_unheld: 0,
        redundant_discharges: 0,
        bound: Some(BoundReading {
            level,
            limit: bound.limit,
            band_edge: bound.band_edge(),
            unit: bound.unit.clone(),
            bound_ref: fence_cid.to_string(),
            signal,
            level_basis: format!(
                "projected level: {} of {} habits in {} carry `active: true`, counted at read \
                 time — not event-derived, so the declared window does not bear on it",
                level,
                habits.len(),
                super::registers::HABITS_REGISTER_REL
            ),
        }),
    }
}

/// Fold one derived view and build its report.
///
/// `Within::Anywhere` is the right selection here and is stated rather than defaulted: the
/// synthetic stock resource IS the whole subject, so containment is not the question this fold
/// asks (`stock.rs:363-375`).
///
/// `unit` is an explicit parameter rather than being read from `name` inside the body, and the
/// production call site passes `name.unit()`. The reason is testability of the one failure this
/// function is responsible for detecting: `count_in` excludes a mismatched unit **silently**
/// (`stock.rs:470-475`), so the only way to prove that a typo surfaces as a refusal rather than
/// as a drained stock is to fold a real view with a wrong unit through *this* code path — the
/// one that computes `excluded_by_unit`. A parameter the test can vary is what makes the
/// exclusion accounting a claim rather than an assertion about code that is never run that way.
pub fn report(name: StockName, view: &DerivedView, window: &Window, unit: &str) -> StockReport {
    let folded =
        stock_over_window_within(&view.resource, Within::Anywhere, &view.events, window, unit);

    // Derived from the fold's OWN output rather than a second copy of `count_in`: the rates are
    // counts divided by `periods`, so multiplying back recovers exactly what the fold admitted.
    // Anything inside the window by timestamp that the fold did not admit was excluded by unit.
    let in_window_by_time = view
        .events
        .iter()
        .filter(|e| window.contains(&e.occurred_at))
        .count();
    let (observed_in_window, excluded_by_unit) = match &folded {
        Ok(stock) => {
            let counted =
                ((stock.inflow.value + stock.outflow.value) * window.periods).round() as usize;
            (counted, in_window_by_time.saturating_sub(counted))
        }
        // A refused fold admitted nothing, so every in-window event is unaccounted for. Reporting
        // the timestamp count here (rather than 0) keeps the refusal from reading as an empty
        // sidecar.
        Err(_) => (0, in_window_by_time),
    };

    let verdict = equilibrium_verdict(&folded, observed_in_window);
    let measures = folded.ok().map(|stock| {
        let net = stock.net_change().ok();
        let turnover = stock.turnover_time().ok();
        StockMeasures {
            level: stock.level.value,
            inflow: stock.inflow.value,
            outflow: stock.outflow.value,
            net: net.as_ref().map(|q| q.value),
            level_basis: stock.level.confidence.basis.clone(),
            inflow_basis: stock.inflow.confidence.basis.clone(),
            outflow_basis: stock.outflow.confidence.basis.clone(),
            turnover_time: turnover.as_ref().map(|q| q.value),
            turnover_basis: turnover.map(|q| q.confidence.basis),
        }
    });

    StockReport {
        stock: name.label().to_string(),
        unit: unit.to_string(),
        verdict,
        measures,
        observed_in_window,
        excluded_by_unit,
        undated_mints: view.undated_mints,
        undated_discharges: view.undated_discharges,
        discharges_of_unheld: view.discharges_of_unheld,
        redundant_discharges: view.redundant_discharges,
        // A folded stock is judged by its rates, not by a declared ceiling.
        bound: None,
    }
}

/// The sidecar's label index (`cid string -> repo-relative path`).
///
/// A local reader rather than a shared one: `walk`'s copy is private to that module, and the
/// alternative — widening its visibility — would touch a file outside this task's boundary for
/// five lines. Absence degrades to an empty map, which costs gap-item claims their mint instant
/// and is reported as `undated_mints` rather than failing the whole readout.
fn load_labels(root: &Path) -> Labels {
    let path = root.join(".eprfs").join("status").join("labels.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr::measure::{Confidence, Interval, MeasureKind, Quantity};
    use elohim_epr_rea::{Intent, ResourceSpec};

    fn cid_of(s: &str) -> Cid {
        atom_cid(&s.to_string()).expect("cid")
    }

    fn commitment(state: CommitmentState) -> FlowRecord {
        FlowRecord::Commitment(Commitment {
            action: ReaVerb::Produce,
            provider: AgentRef("tool:a2o".into()),
            receiver: repo_agent(),
            resource_spec: ResourceSpec {
                classified_as: vec!["a2o:scenario-green".into(), "f.feature".into()],
                quantity: None,
            },
            in_scope_of: cid_of("scope"),
            valid_from: None,
            valid_until: None,
            state,
            satisfies: Vec::new(),
            bound: None,
        })
    }

    fn event(action: ReaVerb, unit: &str, fulfills: Vec<Cid>) -> FlowRecord {
        FlowRecord::Event(FlowEvent {
            action,
            provider: AgentRef("ci".into()),
            receiver: repo_agent(),
            resource: cid_of("artifact"),
            quantity: Magnitude::Count {
                value: 1.0,
                unit: unit.into(),
            },
            process: None,
            in_scope_of: cid_of("scope"),
            fulfills,
            satisfies: Vec::new(),
            classified_as: Vec::new(),
            occurred_at: "2026-08-08T00:00:00Z".into(),
        })
    }

    /// The leg's central correctness risk, at the predicate that decides it: a fulfillment is
    /// minted as `ReaVerb::Produce` (`fulfill.rs:336-352`), and reading its verb instead of its
    /// `fulfills` edge would file every discharge as INFLOW — inverting the sign of the finding
    /// through the data rather than through a caller's mistake.
    #[test]
    fn a_produce_fulfillment_is_outflow_not_inflow() {
        let commit = cid_of("commitment");
        assert_eq!(
            classify_record(&event(ReaVerb::Produce, "green-run", vec![commit])),
            CommitmentFlow::Discharged,
            "a Produce that fulfills a promise DRAINS the commitments stock"
        );
        // …and the same verb with nothing fulfilled is not a flow of this stock at all.
        assert_eq!(
            classify_record(&event(ReaVerb::Produce, "artifact", vec![])),
            CommitmentFlow::Neither(NotCounted::DischargesNothing)
        );
    }

    /// `Dismiss` is a REGRESSION marker here, not a commitment dismissal: `fulfills` is empty by
    /// construction (`fulfill.rs:367-382`) precisely so a red run cannot discharge anything.
    /// Counting it as outflow — as a plain reading of "fulfilled + dismissed + revoked" would —
    /// drains a promise this fold has already drained once.
    #[test]
    fn a_red_run_dismiss_is_neither_inflow_nor_outflow() {
        assert_eq!(
            classify_record(&event(ReaVerb::Dismiss, "red-run", vec![])),
            CommitmentFlow::Neither(NotCounted::DischargesNothing)
        );
    }

    /// Task 1's run-notes: `ReaVerb::Cite`, empty `fulfills`, unit `run-note`. A note about a
    /// promise does not move the promise (Spec A/B Q2) — excluded at the classifier, and
    /// excluded a second time at the unit filter even if the classifier were wrong.
    #[test]
    fn a_run_note_moves_the_commitments_stock_in_neither_direction() {
        assert_eq!(
            classify_record(&event(ReaVerb::Cite, "run-note", vec![])),
            CommitmentFlow::Neither(NotCounted::DischargesNothing)
        );
        assert_ne!(
            COMMITMENT_UNIT, "run-note",
            "the second, mechanical exclusion: notes carry a unit no commitments fold names"
        );
    }

    #[test]
    fn an_open_commitment_is_inflow_and_a_closed_one_is_absence() {
        assert_eq!(
            classify_record(&commitment(CommitmentState::Active)),
            CommitmentFlow::Minted
        );
        assert_eq!(
            classify_record(&commitment(CommitmentState::Proposed)),
            CommitmentFlow::Minted
        );
        // Structurally unreachable today (Spec B Q-C) — and if it ever appears it must read as
        // absence, never as an inflow that can never drain.
        assert_eq!(
            classify_record(&commitment(CommitmentState::Revoked)),
            CommitmentFlow::Neither(NotCounted::NotOpenAtMint)
        );
        assert_eq!(
            classify_record(&commitment(CommitmentState::Fulfilled)),
            CommitmentFlow::Neither(NotCounted::NotOpenAtMint)
        );
    }

    #[test]
    fn plan_and_knowledge_plane_records_are_not_flows_of_this_stock() {
        let intent = FlowRecord::Intent(Intent {
            action: ReaVerb::Produce,
            resource_spec: ResourceSpec {
                classified_as: vec!["gap:open".into(), "id".into()],
                quantity: None,
            },
            in_scope_of: cid_of("scope"),
            raised_by: AgentRef("tool:decompose".into()),
        });
        assert_eq!(
            classify_record(&intent),
            CommitmentFlow::Neither(NotCounted::NotAFlowOfThisStock)
        );
    }

    // ── window construction ──────────────────────────────────────────────────────────────

    #[test]
    fn a_week_window_declares_one_period_and_normalizes_to_utc() {
        let w = parse_window("2026-08-06..2026-08-13", Period::Week).unwrap();
        assert_eq!(w.start, "2026-08-06T00:00:00Z");
        assert_eq!(w.end, "2026-08-13T00:00:00Z");
        assert_eq!(w.periods, 1.0);
        // The same span stated per-day declares seven, which is the point of `--per`.
        let d = parse_window("2026-08-06..2026-08-13", Period::Day).unwrap();
        assert_eq!(d.periods, 7.0);
    }

    #[test]
    fn an_offset_bound_is_normalized_so_the_lexicographic_contains_means_what_it_says() {
        let w = parse_window("2026-08-06T02:00:00+02:00..2026-08-13", Period::Day).unwrap();
        assert_eq!(w.start, "2026-08-06T00:00:00Z");
        assert!(w.contains("2026-08-06T00:30:00Z"));
    }

    #[test]
    fn a_backwards_or_malformed_window_is_refused() {
        assert!(parse_window("2026-08-13..2026-08-06", Period::Week).is_err());
        assert!(parse_window("2026-08-06", Period::Week).is_err());
        assert!(parse_window("not-a-date..2026-08-13", Period::Week).is_err());
        assert!(parse_window("2026-08-06..2026-08-06", Period::Week).is_err());
    }

    #[test]
    fn month_and_year_are_refused_rather_than_approximated() {
        for raw in ["month", "year"] {
            let err = parse_period(raw).expect_err("a calendar guess is refused");
            assert!(err.to_string().contains("no fixed length"), "got: {err}");
        }
        assert!(parse_period("week").is_ok());
        assert!(parse_period("fortnight").is_err());
    }

    #[test]
    fn the_stock_vocabulary_is_closed() {
        assert_eq!(
            StockName::parse("commitments").unwrap(),
            StockName::Commitments
        );
        let err = StockName::parse("memory-bytes").expect_err("a second stock is a spec change");
        assert!(err.to_string().contains("commitments"));
    }

    // ── the verdict ──────────────────────────────────────────────────────────────────────

    fn rate(value: f64, per: Period) -> Quantity {
        Quantity {
            value,
            kind: MeasureKind::Rate { per },
            confidence: Confidence::witnessed(Interval::exact(value), "fixture"),
        }
    }

    fn level(value: f64) -> Quantity {
        Quantity {
            value,
            kind: MeasureKind::Level,
            confidence: Confidence::witnessed(Interval::exact(value), "fixture"),
        }
    }

    #[test]
    fn drain_at_or_above_inflow_is_equilibrium_and_below_it_is_filling() {
        let draining = Stock::new(
            level(10.0),
            rate(2.0, Period::Week),
            rate(5.0, Period::Week),
        );
        assert_eq!(
            equilibrium_verdict(&draining, 7),
            EquilibriumVerdict::Draining
        );

        let balanced = Stock::new(
            level(10.0),
            rate(3.0, Period::Week),
            rate(3.0, Period::Week),
        );
        assert_eq!(
            equilibrium_verdict(&balanced, 6),
            EquilibriumVerdict::Draining,
            "zero net change IS the target state, not a failure"
        );

        let filling = Stock::new(
            level(10.0),
            rate(9.0, Period::Week),
            rate(1.0, Period::Week),
        );
        assert_eq!(
            equilibrium_verdict(&filling, 10),
            EquilibriumVerdict::Filling
        );
    }

    /// An empty window must NOT green. This is the whole fail-closed clause: a check that reads
    /// green because it measured nothing is an over-claim, and a balanced-looking 0-vs-0 is the
    /// exact shape it takes.
    #[test]
    fn a_window_that_observed_nothing_is_refused_not_passed() {
        let empty = Stock::new(
            level(10.0),
            rate(0.0, Period::Week),
            rate(0.0, Period::Week),
        );
        assert!(
            empty
                .as_ref()
                .is_ok_and(|s| s.outflow.value >= s.inflow.value),
            "the naive comparison would pass — which is exactly why the count is a second input"
        );
        let verdict = equilibrium_verdict(&empty, 0);
        assert_eq!(
            verdict,
            EquilibriumVerdict::Refused {
                reason: RefusalReason::NoObservations
            }
        );
        assert!(!verdict.is_equilibrium());
    }

    /// Two flows of different tempo are refused by `Stock::new` before any arithmetic happens
    /// (`stock.rs:137-141`), and the verdict carries that refusal through as a typed reason with
    /// **no partial verdict** beside it.
    #[test]
    fn mismatched_periods_refuse_with_a_typed_reason_and_no_verdict() {
        let mismatched = Stock::new(level(10.0), rate(2.0, Period::Week), rate(5.0, Period::Day));
        assert!(matches!(
            mismatched,
            Err(StockError::MismatchedPeriods { .. })
        ));
        let verdict = equilibrium_verdict(&mismatched, 99);
        match &verdict {
            EquilibriumVerdict::Refused {
                reason: RefusalReason::FoldRefused { kind, error },
            } => {
                assert_eq!(*kind, "mismatched-periods");
                assert!(error.contains("different tempo"), "got: {error}");
            }
            other => panic!("expected a typed fold refusal, got {other:?}"),
        }
        assert!(!verdict.is_equilibrium());
    }

    #[test]
    fn a_nan_rate_is_refused_rather_than_silently_compared() {
        let nan = Stock::new(
            level(10.0),
            rate(f64::NAN, Period::Week),
            rate(1.0, Period::Week),
        );
        assert_eq!(
            equilibrium_verdict(&nan, 4),
            EquilibriumVerdict::Refused {
                reason: RefusalReason::NotComparable
            }
        );
    }

    /// The window path cannot itself mint a period mismatch — both rates inherit `window.per`
    /// from one `Window` (`stock.rs:440-450`). Pinned so that a future refactor introducing a
    /// second period source has to break this assertion rather than the verdict silently.
    #[test]
    fn the_window_path_gives_both_rates_one_period() {
        let window = parse_window("2026-08-06..2026-08-13", Period::Week).unwrap();
        let resource = cid_of("stock");
        let scope = cid_of("scope");
        let events = vec![
            stock_event(
                ReaVerb::Produce,
                resource,
                scope,
                "2026-08-07T00:00:00Z".into(),
                "mint".into(),
            ),
            stock_event(
                ReaVerb::Consume,
                resource,
                scope,
                "2026-08-08T00:00:00Z".into(),
                "discharge".into(),
            ),
        ];
        let stock = stock_over_window_within(
            &resource,
            Within::Anywhere,
            &events,
            &window,
            COMMITMENT_UNIT,
        )
        .expect("one window gives one period");
        assert_eq!(stock.inflow.kind, stock.outflow.kind);
        assert_eq!(
            equilibrium_verdict(&Ok(stock), 2),
            EquilibriumVerdict::Draining
        );
    }

    /// A unit typo excludes everything (`count_in` is exact-string, `stock.rs:470-475`) and the
    /// stock must then read as REFUSED, never as drained. `"doc"` vs `"docs"` is the canonical
    /// shape of this mistake.
    #[test]
    fn a_unit_typo_reads_as_refused_never_as_a_drained_stock() {
        let window = parse_window("2026-08-06..2026-08-13", Period::Week).unwrap();
        let resource = cid_of("stock");
        let scope = cid_of("scope");
        let mut minted = stock_event(
            ReaVerb::Produce,
            resource,
            scope,
            "2026-08-07T00:00:00Z".into(),
            "mint".into(),
        );
        minted.quantity = Magnitude::Count {
            value: 1.0,
            unit: "doc".into(),
        };
        let events = vec![minted];
        let folded =
            stock_over_window_within(&resource, Within::Anywhere, &events, &window, "docs")
                .expect("the fold succeeds — it simply admitted nothing");
        assert_eq!(folded.inflow.value, 0.0);
        assert_eq!(folded.outflow.value, 0.0);
        assert_eq!(
            equilibrium_verdict(&Ok(folded), 0),
            EquilibriumVerdict::Refused {
                reason: RefusalReason::NoObservations
            },
            "0-vs-0 from a unit typo must never read as equilibrium"
        );
    }
}
