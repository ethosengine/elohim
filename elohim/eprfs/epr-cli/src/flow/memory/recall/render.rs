//! Rendering — the human screen an agent can act from.
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
pub(super) fn render(view: &Value) -> String {
    let mut out = String::new();
    let orientation = &view["orientation"];
    out.push_str(&format!(
        "Intent: {}\n",
        orientation["intent"].as_str().unwrap_or_default()
    ));
    out.push_str(&format!(
        "Scope: {}\n",
        orientation["scope"].as_str().unwrap_or_default()
    ));
    out.push_str(&format!(
        "Worthwhile finish: {}\n",
        orientation["worthwhile_finish"]
            .as_str()
            .unwrap_or_default()
    ));
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
                "orientation" | "actions" | "execution_method" | "first_screen"
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
