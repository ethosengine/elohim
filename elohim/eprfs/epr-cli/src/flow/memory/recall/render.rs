//! Rendering — the human screen an agent can act from.
use cid::Cid;

use super::lens::{LensLevel, LensView, RenderFloor};
use super::*;

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Rendering
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The human rendering: one screen an agent can act from.
///
/// The rule is that every accounting structure gets exactly ONE line. Before 2026-09-11 this
/// function pretty-printed `continuation`, `cumulative`, `frontier`, `measurement` and `usage` as
/// inline JSON, which put roughly 8 KB of nested objects between the reader and the only part of
/// the view that is executable — the Linked choices. `--json` still carries every field; a human
/// gets the magnitudes and the commands.
///
/// Governed-discovery station 1.2 adds two floors, checked here and never elsewhere:
/// `floor.always_printed` (the honesty floor — recipe CID, lens CID, selection rule, omissions
/// and receipts, collapsed onto one `render_floor_line`, at EVERY lens including `minimal`) and
/// `floor.unfilterable` (the content floor — a candidate whose frontmatter `content_class` names
/// one of `floor.unfilterable` survives the lens's own `choice_count` cut, marked `[floor]`, at
/// `minimal`/`simple`).
///
/// Fix round 2 (a fresh reader's finding on the station-1 binary, ruled on by the controller):
/// density bounds candidate LISTS and secondary blocks only — never an operation's own primary
/// result. `open`/`resume`/`adopt`'s result IS a candidate list (`first_screen`'s ranked sources,
/// or the whole-scope door's stale-edge groups), so it stays density-bounded at `minimal`/
/// `simple` exactly as station 1.2 shipped it — [`is_open_shaped`] is what tells the two apart.
/// EVERY OTHER operation's result — `read`'s excerpt, `source`'s outline, `history`'s findings,
/// `finish`'s outcome, `context`/`select`'s standing block, `measure`'s measurement line, and by
/// the same principle anything else the generic key dump below already knew how to print — now
/// renders in full at EVERY lens, `minimal` included: before this fix `minimal`'s early return
/// skipped straight from the floor line to Linked choices, so a reader who had just run `read`
/// literally could not see what it read without asking for a wider lens. The `lens:` provenance
/// line moves the same way, unconditional now: rule 3 is about the honesty floor's five fields,
/// never a license to hide WHO is reading at a narrow lens too.
pub(super) fn render(view: &Value, lens: &LensView, floor: &RenderFloor) -> String {
    let mut out = String::new();
    let orientation = &view["orientation"];
    let minimal = matches!(lens.level, LensLevel::Minimal | LensLevel::Simple);
    let is_open_shaped = is_open_shaped(view);
    let operation = view["operation"].as_str().unwrap_or_default();

    out.push_str(&format!(
        "Intent: {}\n",
        orientation["intent"].as_str().unwrap_or_default()
    ));
    if !minimal {
        out.push_str(&format!(
            "Scope: {}\n",
            orientation["scope"].as_str().unwrap_or_default()
        ));
    }
    out.push_str(&format!(
        "Worthwhile finish: {}\n",
        orientation["worthwhile_finish"]
            .as_str()
            .unwrap_or_default()
    ));
    if !minimal {
        for value in orientation["guiding_context"]
            .as_array()
            .cloned()
            .unwrap_or_default()
        {
            out.push_str(&format!(
                "Guiding context: {} — {}\n",
                value["value"].as_str().unwrap_or_default(),
                value["source"].as_str().unwrap_or_default()
            ));
        }
    }
    // WHO is reading — every lens (fix round 2): contestable on sight, never buried among the
    // generic key dump below (excluded there explicitly), and never gated behind `standard`+ —
    // the honesty floor's own compact `lens <cid>` token is a cross-reference to this line, never
    // a replacement for it.
    if !view["lens"].is_null() {
        out.push_str(&render_lens(&view["lens"]));
    }

    // The content floor's SELECTION is computed BEFORE the floor line prints, because the floor
    // line's own `omissions` count must include what THIS render pass itself left out under the
    // lens's `choice_count` — a number the view's JSON cannot carry in advance, since it is a
    // property of the rendering, not of the discovery that produced the candidates. Rendering the
    // candidate BLOCK (the density cap) and choosing the Linked choices both read this same
    // `shown` selection, so the two never disagree about which candidates the reader was actually
    // offered — fix round 1's finding: a `[floor]` candidate past `choice_count` must be one
    // command away too, and a dropped candidate's command must never leak in as a "choice." Only
    // meaningful for an `is_open_shaped` view; `first_screen.candidates` is absent everywhere
    // else, so this is a harmless empty selection there.
    let empty_candidates: Vec<Value> = Vec::new();
    let candidates = if minimal {
        view["first_screen"]["candidates"]
            .as_array()
            .unwrap_or(&empty_candidates)
    } else {
        &empty_candidates
    };
    let (shown, dropped_by_choice) =
        select_shown_candidates(candidates, lens.choice_count as usize, floor);

    out.push_str(&render_floor_line(view, lens, floor, dropped_by_choice));
    if let Some(line) = render_bootstrap_line(view) {
        out.push_str(&line);
    }

    if minimal && is_open_shaped {
        // Fix round 3: a resumed session's recovered state (`continuation`) is the READER'S OWN
        // findings/evidence/questions, never a candidate list — it renders here even though the
        // candidate block below it stays density-bounded. `open` always recovers an empty
        // continuation (a brand-new ceremony), so it keeps the one-line summary only; `resume`/
        // `adopt` additionally get the latest finding and latest unresolved question, since those
        // are exactly what the reader asked to recover.
        out.push_str(&render_continuation_summary(view, operation));
        // `open`/`resume`/`adopt` ONLY: the candidate-shaped result stays density-bounded, and
        // Linked choices are built from the same `shown` selection (fix round 1) rather than the
        // view's raw action list.
        out.push_str(&render_candidate_block(&shown, lens));
        let actions = view["actions"].as_array().cloned().unwrap_or_default();
        let chosen = select_linked_choices(&actions, &shown, lens.choice_count as usize);
        let limit = chosen.len();
        out.push_str(&render_linked_choices(&chosen, limit));
        return out;
    }

    // Every other view's own primary result, at every lens (fix round 2) — the focused door's
    // first screen (never reached when `minimal && is_open_shaped` already returned above) comes
    // BEFORE the concern groups, because a reader who named an area asked "what is the shape here
    // and what do I do first", and the stale-edge scan is the answer to a different question.
    if !view["first_screen"].is_null() {
        out.push_str(&render_first_screen(&view["first_screen"]));
    }
    if let Some(map) = view.as_object() {
        for (key, value) in map {
            if matches!(
                key.as_str(),
                "orientation"
                    | "actions"
                    | "execution_method"
                    | "first_screen"
                    | "lens"
                    | "bootstrap"
                    | "projection"
            ) {
                continue;
            }
            if key == "source_outline" {
                out.push_str(&render_outline(value));
                continue;
            }
            if key == "concerns" {
                out.push_str(&render_concerns(value));
                continue;
            }
            if let Some(line) = summary_line(key, value, operation) {
                out.push_str(&line);
                continue;
            }
            let mut heading = key.replace('_', " ");
            if let Some(first) = heading.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            if let Some(text) = value.as_str() {
                out.push_str(&format!("\n{heading}: {text}\n"));
            } else if value.is_object() || value.is_array() {
                out.push_str(&format!(
                    "\n{heading}:\n{}\n",
                    serde_json::to_string_pretty(value).unwrap_or_default()
                ));
            } else {
                out.push_str(&format!("\n{heading}: {value}\n"));
            }
        }
    }
    // Linked choices: bounded to `choice_count` at `minimal`/`simple` for a non-`open`-shaped
    // view (a plain `.take` — there is no candidate floor to reconcile against here, unlike
    // `select_linked_choices` above), unbounded at `standard`+ exactly as before this task.
    let actions = view["actions"].as_array().cloned().unwrap_or_default();
    let limit = if minimal {
        lens.choice_count as usize
    } else {
        actions.len()
    };
    out.push_str(&render_linked_choices(&actions, limit));
    out
}

/// `open`/`resume`/`adopt`'s result is shaped like a candidate list — `first_screen` (the focused
/// door) or `concerns` (the whole-scope door), or both at once. Everything else this executor can
/// return (`read`'s excerpt, `source`'s outline, `history`'s findings, `finish`'s outcome, a
/// `context`/`select` standing block, `measure`'s measurement, …) carries neither key.
fn is_open_shaped(view: &Value) -> bool {
    !view["first_screen"].is_null() || !view["concerns"].is_null()
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The honesty floor and the content floor
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// A CID string, shortened for the floor line — honest about a fingerprint's existence without
/// paying 59 characters for it twice on the same screen (the detailed `lens:` line already pays
/// that cost at `standard`+; `minimal`/`simple` pay it exactly once, here).
fn short(raw: &str) -> String {
    raw.parse::<Cid>()
        .map(|parsed| crate::flow::short_cid(&parsed))
        .unwrap_or_default()
}

/// The honesty floor, one line, every lens: `floor.always_printed`'s five labels folded together
/// rather than spelled out as five separate lines — `minimal` has no budget for five, and rule 3
/// requires the same five fields be named at every lens, not a subset. The labels are read FROM
/// `floor.always_printed` (index order: recipe, lens, selection, omissions, receipts) rather than
/// hand-duplicated as string literals, so the declared constant actually drives what prints —
/// not merely document it. `extra_omissions` is what THIS render pass left out under the lens's
/// own `choice_count` (always 0 at `standard`+, where nothing is cut); [`nested_omissions_count`]
/// adds what discovery itself already reported as unshown.
fn render_floor_line(
    view: &Value,
    lens: &LensView,
    floor: &RenderFloor,
    extra_omissions: usize,
) -> String {
    let recipe = short(
        view["execution_method"]["method"]
            .as_str()
            .unwrap_or_default(),
    );
    let lens_cid = short(&lens.cid);
    let selection = clip(&selection_rule_for(view), 120);
    let omissions = nested_omissions_count(view) + extra_omissions;
    let receipts = receipts_count(view);
    let [recipe_label, lens_label, selection_label, omissions_label, receipts_label] =
        floor.always_printed;
    format!(
        "{recipe_label} {recipe} · {lens_label} {lens_cid} · {selection_label}: {selection} · \
         {omissions_label}: {omissions} · {receipts_label}: {receipts}\n"
    )
}

/// Governed-discovery station 2.1: `open --purpose bootstrap` names its top red right after the
/// honesty floor line, before the first-screen/habit lines it points into — one line, every lens,
/// the same "print it once, as a line" rule the floor itself follows. `None` for every other view
/// (`view["projection"]["purpose"]` absent from a plain `open`/`resume`/`select`/… view indexes to
/// `Value::Null`, so this is a no-op there, not a panic).
///
/// Fix round 1, finding 1: `projection.omissions` render as `· <omission>` bullets directly under
/// the `Bootstrap:` line — they were previously carried in `--json` only, invisible to a human
/// reader at every lens even though the same omissions now also count toward the floor line's own
/// `omissions: N` ([`nested_omissions_count`]).
fn render_bootstrap_line(view: &Value) -> Option<String> {
    if view["projection"]["purpose"].as_str() != Some("bootstrap") {
        return None;
    }
    let id = view["bootstrap"]["id"].as_str().unwrap_or("none");
    let check = view["bootstrap"]["check"]
        .as_str()
        .unwrap_or("no red habit; orient");
    let mut line = format!("Bootstrap: top red: {id} — {}\n", clip(check, 120));
    for omission in view["projection"]["omissions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        if let Some(text) = omission.as_str() {
            line.push_str(&format!("· {}\n", clip(text, 160)));
        }
    }
    Some(line)
}

/// Fix round 3: at `minimal`/`simple`, an `open`/`resume`/`adopt` view's recovered `continuation`
/// state is never density-bounded — it is the READER'S OWN findings/evidence/questions, not a
/// candidate list, even though the candidate block it sits beside stays bounded. Prints the SAME
/// one-line summary [`summary_line`]'s `"continuation"` case prints (deliberately without that
/// case's optional `Selected:` line — provenance for a chosen concern edge, not part of "what did
/// I recover"), plus [`continuation_recovery_lines`]. `""` when the view carries no `continuation`
/// at all (a harmless no-op, not a panic). Only reached for an `is_open_shaped` view at `minimal`/
/// `simple` — [`render`]'s early return there is exactly where `summary_line`'s `"continuation"`
/// case (which ALSO calls `continuation_recovery_lines`, so a `resume`/`adopt` view gets the same
/// finding/question lines regardless of which of the two code paths it takes) would otherwise
/// have printed it, had that path not returned before reaching the generic key dump.
fn render_continuation_summary(view: &Value, operation: &str) -> String {
    let continuation = &view["continuation"];
    if continuation.is_null() {
        return String::new();
    }
    let counts = &continuation["counts"];
    let mut out = format!(
        "Continuation: {} finding(s) · {} evidence · {} question(s) · next: {}\n",
        count_of(counts, "findings"),
        count_of(counts, "evidence"),
        count_of(counts, "questions"),
        continuation["next_action"]
            .as_str()
            .unwrap_or("choose a concern"),
    );
    out.push_str(&continuation_recovery_lines(continuation, operation));
    out
}

/// The latest finding's claim and the latest unresolved question from a `continuation` block,
/// each clipped to 160 chars on its own line (mirroring `render_bootstrap_line`'s bullet
/// treatment) — ONLY for `resume`/`adopt`, where there is a PRIOR session's state to recover;
/// never a fresh `open`, whose continuation is always freshly empty. `""` when the operation
/// isn't `resume`/`adopt`, or when a slot is honestly absent (fewer than one finding/question
/// retained yet). Shared by [`render_continuation_summary`] (the `minimal`/`simple`
/// `is_open_shaped` early return) and `summary_line`'s `"continuation"` case (every other path,
/// at every lens) — a resumed reader's recovered state must be visible whichever of the two
/// `render()` paths their view takes, not just one of them.
fn continuation_recovery_lines(continuation: &Value, operation: &str) -> String {
    if !matches!(operation, "resume" | "adopt") {
        return String::new();
    }
    let mut out = String::new();
    if let Some(finding) = continuation["findings"][0]["claim"].as_str() {
        out.push_str(&format!("Finding: {}\n", clip(finding, 160)));
    }
    if let Some(question) = continuation["unresolved_questions"][0].as_str() {
        out.push_str(&format!("Question: {}\n", clip(question, 160)));
    }
    out
}

/// The rule this view actually ranked candidates by, from whichever door produced them — the
/// focused door's `first_screen.ranking` (itself `discover_scored`'s own `selection` text) when a
/// question named an area, else the whole-scope door's `concerns.selection_rule`. Honest absence
/// (neither door ranked anything) when this view is a `recipe`/`source`/`history` projection with
/// no candidate list of its own.
fn selection_rule_for(view: &Value) -> String {
    if let Some(rule) = view["first_screen"]["ranking"].as_str() {
        return rule.to_string();
    }
    if let Some(rule) = view["concerns"]["selection_rule"].as_str() {
        return rule.to_string();
    }
    "no candidates ranked in this view".to_string()
}

/// Omissions already named by discovery itself, summed across every door this view carries —
/// `first_screen` (the focused door), `concerns` (the whole-scope door), `source_outline` (a
/// located passage) and `projection` (`--purpose bootstrap`'s own omissions — fix round 1, finding
/// 1: these previously named an absent `flows.jsonl` in `view["projection"]["omissions"]` but
/// never counted toward this floor, so the honesty floor's own `omissions: N` undercounted a
/// bootstrap view). Each door already bounds and NAMES what it left out (`discovery.rs`,
/// `concerns.rs`); this only totals those named counts for the one-line floor, it never discovers
/// a new omission of its own.
fn nested_omissions_count(view: &Value) -> usize {
    ["first_screen", "concerns", "source_outline", "projection"]
        .iter()
        .map(|key| {
            view[*key]["omissions"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0)
        })
        .sum()
}

/// What this view is standing on, receipt-wise — the session's accumulated evidence count when
/// this operation carries a `continuation` (`open`/`resume`/`adopt`), else whichever
/// operation-specific receipt list this view carries (`source`'s `receipt_keys`, `finish`'s
/// `outcome.receipts`, `context`'s `evidence`). Honest zero for a view with none of these
/// (`recipe`, `search`, `measure`).
fn receipts_count(view: &Value) -> usize {
    if let Some(n) = view["continuation"]["counts"]["evidence"].as_u64() {
        return n as usize;
    }
    if let Some(items) = view["receipt_keys"].as_array() {
        return items.len();
    }
    if let Some(items) = view["outcome"]["receipts"].as_array() {
        return items.len();
    }
    if let Some(items) = view["evidence"].as_array() {
        return items.len();
    }
    0
}

/// The content floor's SELECTION, applied to `minimal`/`simple`: the first screen's candidates,
/// cut to `choice_count` — except a candidate whose frontmatter `content_class` names one of
/// `floor.unfilterable`, which survives the cut regardless of position, marked `is_floor`. Kept
/// separate from RENDERING (see [`render_candidate_block`]) so both the candidate block and
/// [`select_linked_choices`] read the exact same set — fix round 1's finding: the two must never
/// disagree about which candidates the reader was actually offered. Order is preserved from
/// `candidates`' own rank order. Returns `(shown, dropped_by_choice)` — the latter counts only
/// ORDINARY candidates the cut left out, and feeds the floor line's `omissions`.
fn select_shown_candidates<'a>(
    candidates: &'a [Value],
    choice_count: usize,
    floor: &RenderFloor,
) -> (Vec<(&'a Value, bool)>, usize) {
    let mut shown: Vec<(&Value, bool)> = Vec::new();
    let mut dropped_by_choice = 0usize;
    for (index, candidate) in candidates.iter().enumerate() {
        let is_floor = candidate["content_class"]
            .as_str()
            .map(|class| floor.unfilterable.contains(&class))
            .unwrap_or(false);
        if index < choice_count || is_floor {
            shown.push((candidate, is_floor));
        } else {
            dropped_by_choice += 1;
        }
    }
    (shown, dropped_by_choice)
}

/// The content floor's RENDERING: the `shown` candidates (see [`select_shown_candidates`]), one
/// line each, `[floor]`-marked where `is_floor`. `lens.density_bytes` is a SEPARATE soft cap
/// layered on top of the selection, dropping ordinary lines (never a `[floor]`-marked one, and
/// never the floor line itself, which this function never touches) once the block would exceed
/// it, and naming exactly how many more exist at a wider lens. Empty string when `shown` is
/// empty — no header printed over nothing.
fn render_candidate_block(shown: &[(&Value, bool)], lens: &LensView) -> String {
    if shown.is_empty() {
        return String::new();
    }
    let mut out = String::from("\nCandidate sources:\n");
    let mut printed = 0usize;
    let mut over_budget = false;
    let mut dropped_by_density = 0usize;
    for (candidate, is_floor) in shown {
        let marker = if *is_floor { " [floor]" } else { "" };
        let line = format!(
            "  {}. {} — {}{marker}\n",
            printed + 1,
            candidate["path"].as_str().unwrap_or_default(),
            clip(candidate["title"].as_str().unwrap_or_default(), 100),
        );
        if !is_floor && (over_budget || out.len() + line.len() > lens.density_bytes) {
            over_budget = true;
            dropped_by_density += 1;
            continue;
        }
        out.push_str(&line);
        printed += 1;
    }
    if dropped_by_density > 0 {
        out.push_str(&format!(
            "  · {dropped_by_density} more candidate(s) at a wider lens (--lens {})\n",
            lens.level.next().as_str()
        ));
    }
    out
}

/// The `--path <p>` an action's `argv` names, or `None` for an action that does not reference a
/// candidate path at all (a session action: recipe/history/resume/measure, or an edge `select`,
/// which names `--edge` instead). The action-building side (`journey.rs`'s `action()`) always
/// spells the flag this way (`format!("--{}", key.replace('_', "-"))` on the `"path"` option key),
/// so this is a read of that same convention, not a re-derivation of it.
fn action_path(action: &Value) -> Option<&str> {
    let argv = action["argv"].as_array()?;
    let index = argv
        .iter()
        .position(|item| item.as_str() == Some("--path"))?;
    argv.get(index + 1)?.as_str()
}

/// The first action in `actions` whose `--path` names exactly `path` — `None` for an empty path
/// or no match, rather than guessing. Linear scan: the action lists this walks are single-digit
/// to low-dozens long (one action per candidate plus a handful of session actions), never a hot
/// path worth indexing.
fn action_for_path<'a>(actions: &'a [Value], path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return None;
    }
    actions
        .iter()
        .find(|action| action_path(action) == Some(path))
}

/// `minimal`/`simple` ONLY: the Linked choices actually offered, built from `shown` (see
/// [`select_shown_candidates`]) rather than the view's raw unbounded action list — fix round 1's
/// finding. Order: EVERY `[floor]`-marked shown candidate's command first, unconditionally (the
/// anti-capture invariant: floor content is one command away at every lens, never bumped by rank
/// or by the choice budget); then ordinary shown candidates' commands while the running total is
/// still under `choice_count`; then the view's own session actions (recipe/history/resume/measure
/// — anything with no `--path`, so a dropped candidate's command can never leak in as a "session"
/// filler) in their existing order, filling whatever budget remains. A command naming a candidate
/// the content floor did NOT keep never appears here, by construction: only `shown`'s own
/// candidates are ever looked up.
fn select_linked_choices(
    actions: &[Value],
    shown: &[(&Value, bool)],
    choice_count: usize,
) -> Vec<Value> {
    let mut chosen: Vec<Value> = Vec::new();
    for (candidate, _) in shown.iter().filter(|(_, is_floor)| *is_floor) {
        let path = candidate["path"].as_str().unwrap_or_default();
        if let Some(action) = action_for_path(actions, path) {
            chosen.push(action.clone());
        }
    }
    for (candidate, _) in shown.iter().filter(|(_, is_floor)| !*is_floor) {
        if chosen.len() >= choice_count {
            break;
        }
        let path = candidate["path"].as_str().unwrap_or_default();
        if let Some(action) = action_for_path(actions, path) {
            chosen.push(action.clone());
        }
    }
    for action in actions {
        if chosen.len() >= choice_count {
            break;
        }
        if action_path(action).is_none() {
            chosen.push(action.clone());
        }
    }
    chosen
}

/// The Linked choices block: `label\n  command\n` for each of the first `limit` actions — the
/// literal printing loop shared by `minimal`/`simple` (called with [`select_linked_choices`]'s
/// already-bounded list and its own length — "print everything selected") and `standard`+
/// (called with the view's full unbounded action list and its own length — today's "print
/// everything" behaviour, unchanged). Fix round 1's deferred-duplication finding.
fn render_linked_choices(actions: &[Value], limit: usize) -> String {
    let mut out = String::from("\nLinked choices:\n");
    for choice in actions.iter().take(limit) {
        out.push_str(&format!(
            "{}\n  {}\n",
            choice["label"].as_str().unwrap_or_default(),
            choice["command"].as_str().unwrap_or_default()
        ));
    }
    out
}

/// WHO is reading, resolved and contestable on sight: the level, its stated and revealed
/// provenance, and the recipe default it fell back to. One line, however many provenance
/// entries there are — the honesty floor requires the lens be NAMED on every view, not that it
/// be spelled out in full; `--json` carries every provenance line for a reader who wants them.
fn render_lens(lens: &Value) -> String {
    let level = lens["level"].as_str().unwrap_or_default();
    let stated = joined_or(&lens["provenance"]["stated"], "none");
    let revealed = joined_or(&lens["provenance"]["revealed"], "no evidence yet");
    let defaults = lens["provenance"]["defaults"].as_str().unwrap_or_default();
    let cid = lens["cid"]
        .as_str()
        .and_then(|s| s.parse::<Cid>().ok())
        .map(|parsed| crate::flow::short_cid(&parsed))
        .unwrap_or_default();
    // A wall-clock `renew by <date>` would break this line's digest daily; real expiry arrives
    // with station 4's tending record, and until then the honest answer is that there is none.
    let renew = match lens["expires_at"].as_str() {
        Some(date) => format!("renew by {date}"),
        None => "renew: none (no tending record)".to_string(),
    };
    format!(
        "lens: {level} · stated {stated} · revealed {revealed} · {defaults} · cid {cid} · {renew}\n"
    )
}

/// Every string in a JSON array, joined for one line; `fallback` when the array is empty or
/// absent.
fn joined_or(value: &Value, fallback: &str) -> String {
    let items: Vec<&str> = value
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if items.is_empty() {
        fallback.to_string()
    } else {
        items.join("; ")
    }
}

/// The focused door: the area's habits with their last delta, then the competing sources.
fn render_first_screen(screen: &Value) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\nArea: {}\n",
        screen["area"].as_str().unwrap_or_default()
    ));
    let habits = screen["habits"].as_array().cloned().unwrap_or_default();
    if habits.is_empty() {
        out.push_str("Habits: none in the register match this question's terms\n");
    }
    for habit in &habits {
        out.push_str(&format!(
            "Habit: {} [{}] — {}\n",
            habit["id"].as_str().unwrap_or_default(),
            habit["status"].as_str().unwrap_or_default(),
            habit["delta"]
                .as_str()
                .unwrap_or("no evidence ledger entry")
        ));
    }
    let candidates = screen["candidates"].as_array().cloned().unwrap_or_default();
    if candidates.is_empty() {
        out.push_str("Candidate sources: none in scope matched this question's terms\n");
    } else {
        out.push_str(&format!(
            "Candidate sources ({}, {}):\n",
            screen["provider"].as_str().unwrap_or("local"),
            screen["ranking"]
                .as_str()
                .unwrap_or("term overlap over declared metadata; not authority")
        ));
        for (index, candidate) in candidates.iter().enumerate() {
            let kinds: Vec<&str> = candidate["match"]
                .as_array()
                .map(|items| items.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let matched = if kinds.is_empty() {
                String::new()
            } else {
                format!(" [matched in {}]", kinds.join(", "))
            };
            out.push_str(&format!(
                "  {}. {} — {}{matched}\n",
                index + 1,
                candidate["path"].as_str().unwrap_or_default(),
                truncate(candidate["title"].as_str().unwrap_or_default(), 100)
            ));
            if let (Some(title), Some(lines)) = (
                candidate["best_section"]["title"].as_str(),
                candidate["best_section"]["lines"].as_str(),
            ) {
                out.push_str(&format!(
                    "       § {title} ({lines}) {}\n",
                    render_hits(&candidate["best_section"]["hits"])
                ));
            }
        }
    }
    for message in screen["omissions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .chain(
            screen["unresolved"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter(),
        )
    {
        out.push_str(&format!(
            "  · {}\n",
            bullet(message.as_str().unwrap_or_default())
        ));
    }
    out
}

/// One line per section: where it starts, where the question's terms land, and what to read.
fn render_outline(value: &Value) -> String {
    let mut out = format!(
        "\nOutline: {} ({} lines)\n",
        value["path"].as_str().unwrap_or_default(),
        value["line_count"].as_u64().unwrap_or(0)
    );
    for heading in value["headings"].as_array().cloned().unwrap_or_default() {
        let hits = render_hits(&heading["hits"]);
        out.push_str(&format!(
            "  {} {}{}{}\n",
            heading["read_lines"].as_str().unwrap_or_default(),
            truncate(heading["title"].as_str().unwrap_or_default(), 110),
            if hits.is_empty() {
                String::new()
            } else {
                format!("  {hits}")
            },
            if heading["window_complete"].as_bool() == Some(false) {
                "  (first window only)"
            } else {
                ""
            }
        ));
    }
    for message in value["omissions"].as_array().cloned().unwrap_or_default() {
        out.push_str(&format!(
            "  · {}\n",
            bullet(message.as_str().unwrap_or_default())
        ));
    }
    out
}

/// The whole-scope door: the stale edges grouped by shared source, with its accounting in lines.
fn render_concerns(value: &Value) -> String {
    let mut out = String::from("\nConcerns:\n");
    let counts = &value["counts"];
    out.push_str(&format!(
        "Measured scope: {} stale · {} dangling · {} group(s) · {} edge(s) indexed\n",
        count_of(counts, "stale"),
        count_of(counts, "dangling"),
        count_of(counts, "groups"),
        count_of(counts, "total_edges"),
    ));
    let page = &value["page"];
    out.push_str(&format!(
        "Page: {} shown from offset {} · {} omitted\n",
        count_of(page, "returned_edges"),
        count_of(page, "offset"),
        count_of(page, "omitted_edges"),
    ));
    let mut number = 0;
    for group in value["groups"].as_array().cloned().unwrap_or_default() {
        out.push_str(&format!(
            "Shared source: {}\n",
            group["source"].as_str().unwrap_or_default()
        ));
        for edge in group["edges"].as_array().cloned().unwrap_or_default() {
            number += 1;
            out.push_str(&format!(
                "  {number}. {} [{}, {}] — {}\n",
                edge["slot"]["from"].as_str().unwrap_or_default(),
                edge["verdict"].as_str().unwrap_or_default(),
                edge["slot"]["plane"].as_str().unwrap_or_default(),
                edge["slot"]["description"]
                    .as_str()
                    .unwrap_or("Purpose unstated; inspect source")
            ));
        }
    }
    out.push_str(&format!(
        "Selection: {}\n",
        value["selection_rule"]
            .as_str()
            .unwrap_or("native fixture selection")
    ));
    let omissions = value["omissions"].as_array().cloned().unwrap_or_default();
    if !omissions.is_empty() {
        out.push_str("Omissions:\n");
        for message in omissions {
            out.push_str(&format!(
                "  · {}\n",
                bullet(message.as_str().unwrap_or_default())
            ));
        }
    }
    out
}

/// The compact line for one accounting structure, or `None` when the key has no summary shape.
/// `operation` is only consulted by the `"continuation"` case (fix round 3 —
/// [`continuation_recovery_lines`]); every other case ignores it.
fn summary_line(key: &str, value: &Value, operation: &str) -> Option<String> {
    match key {
        "continuation" => {
            let counts = &value["counts"];
            let mut line = format!(
                "\nContinuation: {} finding(s) · {} evidence · {} question(s) · next: {}\n",
                count_of(counts, "findings"),
                count_of(counts, "evidence"),
                count_of(counts, "questions"),
                value["next_action"].as_str().unwrap_or("choose a concern"),
            );
            if let Some(selected) = value["selected"]["slot"]["from"].as_str() {
                line.push_str(&format!("Selected: {selected}\n"));
            }
            // Fix round 3: a resumed reader's recovered state must be visible at every lens —
            // `resume`/`adopt` views that carry `concerns`/`first_screen` (so `render()` takes
            // the `minimal`/`simple` early return) get these same lines from
            // `render_continuation_summary` instead; this arm covers every other path.
            line.push_str(&continuation_recovery_lines(value, operation));
            Some(line)
        }
        "cumulative" => {
            let totals = &value["totals"];
            let mut parts = vec![format!("attempt {}", count_of(value, "attempts"))];
            parts.push(format!("{} source bytes", count_of(totals, "source_bytes")));
            if number_of(totals, "source_files") > 0.0 {
                parts.push(format!("{} file(s)", count_of(totals, "source_files")));
            }
            if number_of(totals, "native_raw_bytes") > 0.0 {
                parts.push(format!(
                    "{} native bytes",
                    count_of(totals, "native_raw_bytes")
                ));
            }
            parts.push(format!("{:.1}s", number_of(totals, "elapsed_seconds")));
            parts.push(format!(
                "unmetered {}",
                count_of(value, "unmetered_attempts")
            ));
            Some(format!("\nAccounting: {}\n", parts.join(" · ")))
        }
        "frontier" => {
            let latest = value["latest"].as_array().cloned().unwrap_or_default();
            let newest = latest
                .first()
                .and_then(|item| item["question"].as_str().or_else(|| item.as_str()))
                .map(|text| format!(" · latest: {}", bullet(text)))
                .unwrap_or_default();
            Some(format!(
                "\nFrontier: {} unresolved question(s){newest}\n",
                count_of(value, "total")
            ))
        }
        "measurement" => {
            if value["observed"].is_null() {
                return Some(
                    "\nMeasurement: original baseline retained; resuming does not resample it\n"
                        .into(),
                );
            }
            Some(format!(
                "\nMeasurement: {} {} B over {} file(s) sampled; receipt {} B{}\n",
                value["phase"].as_str().unwrap_or("baseline"),
                count_of(&value["observed"], "bytes"),
                count_of(&value["observed"], "files"),
                count_of(&value["evidence"], "bytes"),
                if value["complete"].as_bool() == Some(true) {
                    ""
                } else {
                    " (incomplete)"
                },
            ))
        }
        "usage" => {
            let Some(map) = value.as_object() else {
                return Some("\nUsage: none charged\n".into());
            };
            if map.is_empty() {
                return Some("\nUsage: none charged\n".into());
            }
            let parts: Vec<String> = map
                .iter()
                .filter_map(|(name, item)| {
                    numeric(item).map(|number| format!("{name} {}", thousands(number)))
                })
                .collect();
            Some(format!("\nUsage: {}\n", parts.join(" · ")))
        }
        "unresolved" => {
            let items = value.as_array().cloned().unwrap_or_default();
            if items.is_empty() {
                return Some("\nUnresolved: none named by this view\n".into());
            }
            let mut line = String::from("\nUnresolved:\n");
            for item in items {
                line.push_str(&format!(
                    "  · {}\n",
                    bullet(item.as_str().unwrap_or_default())
                ));
            }
            Some(line)
        }
        _ => None,
    }
}

/// `[hits: mempalace ×3, re-mine ×1]`, or nothing when the section carries none of the terms.
pub(super) fn render_hits(hits: &Value) -> String {
    let Some(map) = hits.as_object().filter(|m| !m.is_empty()) else {
        return String::new();
    };
    let mut rows: Vec<(&String, u64)> = map
        .iter()
        .map(|(term, count)| (term, count.as_u64().unwrap_or(0)))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    format!(
        "[hits: {}]",
        rows.iter()
            .map(|(term, count)| format!("{term} ×{count}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// A bounded bullet: one line, cut at a word boundary, with the cut named by an ellipsis.
fn bullet(text: &str) -> String {
    clip(&one_line(text), 160)
}

/// Cut `text` to `limit` characters at a word boundary, naming the cut with an ellipsis.
pub(super) fn clip(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let cut: String = text.chars().take(limit).collect();
    let trimmed = match cut.rfind(' ') {
        Some(index) if index > limit / 2 => cut[..index].to_string(),
        _ => cut,
    };
    format!("{trimmed}…")
}

fn number_of(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(numeric).unwrap_or(0.0)
}

fn count_of(value: &Value, key: &str) -> String {
    thousands(number_of(value, key))
}

/// Group a magnitude in threes. Byte counts are the whole point of this view, and `3377184` and
/// `337718` read identically at a glance.
fn thousands(number: f64) -> String {
    if number.fract().abs() > f64::EPSILON {
        return format!("{number}");
    }
    let negative = number < 0.0;
    let digits = format!("{}", number.abs() as u64);
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}
