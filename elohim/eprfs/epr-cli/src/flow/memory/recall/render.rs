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
/// `minimal`/`simple`). `Standard` and above are otherwise BYTE-IDENTICAL to the pre-1.2
/// rendering — only the floor line is new — because nothing in that rendering ever dropped a
/// candidate to begin with; the truncation this task adds only bites where a lens actually
/// narrows (`minimal`/`simple`).
pub(super) fn render(view: &Value, lens: &LensView, floor: &RenderFloor) -> String {
    let mut out = String::new();
    let orientation = &view["orientation"];
    let minimal = matches!(lens.level, LensLevel::Minimal | LensLevel::Simple);

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
        // WHO is reading, printed right after the guiding context it bounds — contestable on
        // sight, never buried among the generic key dump below (excluded there explicitly). Only
        // at `standard`+: `minimal`/`simple` fold WHO-is-reading into the floor line's own `lens
        // <cid>` token instead of also carrying this fuller provenance line — the honesty floor
        // asks for one line, not two, when the budget is one command.
        if !view["lens"].is_null() {
            out.push_str(&render_lens(&view["lens"]));
        }
    }

    // The content floor is computed BEFORE the floor line prints, because the floor line's own
    // `omissions` count must include what THIS render pass itself left out under the lens's
    // `choice_count` — a number the view's JSON cannot carry in advance, since it is a property
    // of the rendering, not of the discovery that produced the candidates.
    let (candidates_block, dropped_by_choice) = if minimal {
        match view["first_screen"]["candidates"].as_array() {
            Some(candidates) => render_candidates_bounded(candidates, lens, floor),
            None => (String::new(), 0),
        }
    } else {
        (String::new(), 0)
    };

    out.push_str(&render_floor_line(view, lens, floor, dropped_by_choice));

    if minimal {
        out.push_str(&candidates_block);
        out.push_str("\nLinked choices:\n");
        for choice in view["actions"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .take(lens.choice_count as usize)
        {
            out.push_str(&format!(
                "{}\n  {}\n",
                choice["label"].as_str().unwrap_or_default(),
                choice["command"].as_str().unwrap_or_default()
            ));
        }
        return out;
    }

    // The focused door's first screen comes BEFORE the concern groups, because a reader who named
    // an area asked "what is the shape here and what do I do first", and the stale-edge scan is
    // the answer to a different question.
    if !view["first_screen"].is_null() {
        out.push_str(&render_first_screen(&view["first_screen"]));
    }
    if let Some(map) = view.as_object() {
        for (key, value) in map {
            if matches!(
                key.as_str(),
                "orientation" | "actions" | "execution_method" | "first_screen" | "lens"
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
            if let Some(line) = summary_line(key, value) {
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
    out.push_str("\nLinked choices:\n");
    for choice in view["actions"].as_array().cloned().unwrap_or_default() {
        out.push_str(&format!(
            "{}\n  {}\n",
            choice["label"].as_str().unwrap_or_default(),
            choice["command"].as_str().unwrap_or_default()
        ));
    }
    out
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
/// `first_screen` (the focused door), `concerns` (the whole-scope door) and `source_outline` (a
/// located passage). Each door already bounds and NAMES what it left out (`discovery.rs`,
/// `concerns.rs`); this only totals those named counts for the one-line floor, it never discovers
/// a new omission of its own.
fn nested_omissions_count(view: &Value) -> usize {
    ["first_screen", "concerns", "source_outline"]
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

/// The content floor applied to `minimal`/`simple`: the first screen's candidates, cut to the
/// lens's `choice_count` — except a candidate whose frontmatter `content_class` names one of
/// `floor.unfilterable`, which survives the cut regardless of position, marked `[floor]`. The
/// lens's `density_bytes` is then a SEPARATE soft cap layered on top, dropping ordinary lines
/// (never a `[floor]`-marked one, and never the floor line itself, which this function never
/// touches) once the block would exceed it, and naming exactly how many more exist at a wider
/// lens. Returns the printed block (empty when there is nothing to show) and how many ORDINARY
/// candidates the `choice_count` cut alone left out — the density cut is named in the block's own
/// trailing note instead, a second and separately honest signal rather than one blended number.
fn render_candidates_bounded(
    candidates: &[Value],
    lens: &LensView,
    floor: &RenderFloor,
) -> (String, usize) {
    if candidates.is_empty() {
        return (String::new(), 0);
    }
    let choice_count = lens.choice_count as usize;
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
    if shown.is_empty() {
        return (String::new(), dropped_by_choice);
    }
    let mut out = String::from("\nCandidate sources:\n");
    let mut printed = 0usize;
    let mut over_budget = false;
    let mut dropped_by_density = 0usize;
    for (candidate, is_floor) in &shown {
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
    (out, dropped_by_choice)
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
fn summary_line(key: &str, value: &Value) -> Option<String> {
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
