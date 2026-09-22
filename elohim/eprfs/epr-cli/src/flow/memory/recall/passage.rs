//! Passage — where, inside a source discovery has already named a candidate, the question's
//! terms land: the section outline (`outline`), the one section a candidate's read should open
//! (`best_section`), and, for a structured file whose author declared no sections (JSON, YAML,
//! Rust, shell), the term-densest line window bounded by `discovery.passage_window_bytes`.
//!
//! Split out of `discovery.rs` (2026-09-22, recall — Codex's trail sprint) when non-markdown
//! candidates made locating a passage its own concern: discovery decides WHICH source, this seam
//! decides WHERE in it. Scan bytes only — a `read` receipt is still the only thing that quotes.
use super::discovery::{question_terms, stem};
use super::*;

/// Bounded table of contents, so the agent chooses a passage before loading it.
/// One section of an outline, with where the question's terms actually land inside it.
///
/// Naming a document is half an answer. A reader that opens a 24,000-byte skill and is handed six
/// headings, none of which mentions its question, closes the file — which is exactly what happened
/// on 2026-09-11: the answer was under "Phase 4 — verify the experience, reconcile and retain
/// learning", a heading that says nothing about re-mining an index.
pub(super) fn outline(args: &Args, contract: &Contract, path: &str) -> FlowResult<Value> {
    outline_with_terms(
        args,
        contract,
        path,
        &question_terms(contract, args.need.trim()),
    )
}

fn outline_with_terms(
    args: &Args,
    contract: &Contract,
    path: &str,
    terms: &[String],
) -> FlowResult<Value> {
    let source = contained(&args.root, path, &contract.source_roots())?;
    let bound = contract.limit_usize("scan_bytes");
    let mut raw = Vec::new();
    File::open(&source)
        .and_then(|f| f.take(bound as u64 + 1).read_to_end(&mut raw).map(|_| ()))
        .map_err(|error| FlowError::Read {
            path: source.clone(),
            source: error,
        })?;
    let complete = raw.len() <= bound;
    let text = String::from_utf8_lossy(&raw[..raw.len().min(bound)]).to_string();
    let lines: Vec<&str> = text.lines().collect();
    let is_code = source.extension().and_then(|e| e.to_str()) == Some("py");
    let mut headings: Vec<Value> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with('#')
            || (is_code && (line.starts_with("def ") || line.starts_with("class ")))
        {
            headings.push(json!({
                "line": index + 1,
                "title": line.trim_start_matches('#').trim(),
            }));
        }
    }
    for index in 0..headings.len() {
        let end = if index + 1 < headings.len() {
            headings[index + 1]["line"].as_u64().unwrap_or(1) - 1
        } else {
            lines.len() as u64
        };
        headings[index]["end_line"] = json!(end);
    }
    // Where the question's terms land, section by section, plus a read range bounded to what one
    // excerpt may return. The bytes counted here are scan bytes; only `read` charges source_bytes.
    let lowered: Vec<String> = lines.iter().map(|line| line.to_lowercase()).collect();
    let scorer = Scorer::new(&lowered, terms);
    let source_bytes = contract.limit_usize("source_bytes");
    let mut oversized_sections = 0usize;
    for heading in headings.iter_mut() {
        let start = heading["line"].as_u64().unwrap_or(1) as usize;
        let end = (heading["end_line"].as_u64().unwrap_or(1) as usize).max(start);
        let section: String = lines[start.saturating_sub(1)..end.min(lines.len())]
            .join("\n")
            .to_lowercase();
        // Same light stemming and word-start matching as the density window, and the same
        // rarity weights: a section is ranked by the question's rare terms it carries together,
        // not by how often it repeats a common one.
        let (score, total, hits) = scorer.score(&section);
        // The emitted range never promises more than one excerpt can carry: lines are taken from
        // the section's start until the byte budget is spent, and a section that does not fit says
        // so rather than handing over a range `read` would refuse.
        let (mut window_end, mut bytes) = (start, 0usize);
        for (offset, line) in lines[start.saturating_sub(1)..end.min(lines.len())]
            .iter()
            .enumerate()
        {
            let next = bytes + line.len() + 1;
            if next > source_bytes && window_end > start {
                break;
            }
            bytes = next;
            window_end = start + offset;
        }
        heading["hits"] = json!(hits);
        heading["hit_total"] = json!(total);
        heading["score"] = json!(score);
        heading["read_lines"] = json!(format!("{start}:{window_end}"));
        heading["window_complete"] = json!(window_end >= end);
        if window_end < end {
            oversized_sections += 1;
        }
    }
    // A structured file (JSON, YAML, Rust, shell, …) has no heading its author meant as a section:
    // YAML's `#` lines are comments, JSON has none at all. Its passage is instead the line window
    // where the question's terms are densest, bounded by the declared
    // `discovery.passage_window_bytes` so the receipt stays one small excerpt. Markdown and Python
    // keep their declared sections; any file whose scan found no heading gets windows too.
    let sectioned = matches!(
        source.extension().and_then(|e| e.to_str()),
        Some("md" | "markdown" | "py")
    );
    if !terms.is_empty() && (!sectioned || headings.is_empty()) {
        let window_bytes = contract
            .value
            .pointer("/discovery/passage_window_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(4096) as usize;
        if let Some(window) = densest_window(&lines, terms, window_bytes) {
            headings = vec![window];
        }
    }
    let truncated = !complete || headings.len() > 40;
    headings.truncate(40);
    let mut omissions: Vec<Value> = Vec::new();
    if truncated {
        omissions.push(json!("outline incomplete; use an explicit range"));
    }
    if oversized_sections > 0 {
        omissions.push(json!(format!(
            "{oversized_sections} section(s) exceed the per-excerpt byte budget; the offered range \
             is the first window of each — continue with an explicit later range"
        )));
    }
    Ok(json!({
        "path": path,
        "headings": headings,
        "line_count": lines.len(),
        "terms": terms,
        "hit_scope": "term occurrences inside each section of this outline; scan bytes, not evidence",
        "omissions": omissions,
        "usage": {"scan_bytes": raw.len(), "scanned_files": 1},
    }))
}

/// The line window, at most `window_bytes` long and starting on a line that carries a term, whose
/// terms score highest by rarity across the file's lines (the earlier window on a tie). Rendered as one outline section, so `best_section` and `read` treat it like any heading.
fn densest_window(lines: &[&str], terms: &[String], window_bytes: usize) -> Option<Value> {
    let lowered: Vec<String> = lines.iter().map(|line| line.to_lowercase()).collect();
    let scorer = Scorer::new(&lowered, terms);
    let mut best: Option<(f64, usize, usize)> = None; // (score, start, end)
    for start in 0..lowered.len() {
        if scorer.score(&lowered[start]).1 == 0 {
            continue;
        }
        let (mut end, mut bytes) = (start, 0usize);
        while end < lowered.len() {
            let next = bytes + lines[end].len() + 1;
            if next > window_bytes && end > start {
                break;
            }
            bytes = next;
            end += 1;
        }
        let (score, _, _) = scorer.score(&lowered[start..end].join("\n"));
        if best.is_none_or(|(s, _, _)| score > s) {
            best = Some((score, start, end));
        }
    }
    let (_, start, end) = best?;
    let (score, total, hits) = scorer.score(&lowered[start..end].join("\n"));
    let first = lines[start..end]
        .iter()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    Some(json!({
        "line": start + 1,
        "end_line": end,
        "title": format!("lines {}–{}: {}", start + 1, end, clip(first, 60)),
        "hits": hits,
        "hit_total": total,
        "score": score,
        "read_lines": format!("{}:{}", start + 1, end),
        "window_complete": true,
        "section_kind": "term-density window",
    }))
}

/// Rarity-weighted term scoring over one file. A term's weight is how rare it is across the
/// file's LINES (inverse line frequency): a word on every other line ranks nothing, a term on three
/// lines is the concern. A passage scores Σ weight × (1 + ln occurrences) over the terms it
/// carries, so several rare terms together outrank one common term repeated. Terms match at a
/// word start, falling back to the light stem when the exact form is absent.
struct Scorer<'a> {
    terms: &'a [String],
    needles: Vec<(String, String)>,
    weights: Vec<f64>,
}

impl<'a> Scorer<'a> {
    fn new(lowered: &[String], terms: &'a [String]) -> Self {
        let needles: Vec<(String, String)> = terms
            .iter()
            .map(|term| {
                let needle = term.to_lowercase();
                let stemmed = stem(&needle).to_string();
                (needle, stemmed)
            })
            .collect();
        let line_count = lowered.len().max(1) as f64;
        let weights = needles
            .iter()
            .map(|n| {
                let df = lowered.iter().filter(|line| count(line, n) > 0).count() as f64;
                ((line_count + 1.0) / (1.0 + df)).ln().max(0.0)
            })
            .collect();
        Scorer {
            terms,
            needles,
            weights,
        }
    }

    /// (score, total occurrences, per-term hits) of one already-lowercased passage.
    fn score(&self, text: &str) -> (f64, usize, Map<String, Value>) {
        let (mut score, mut total, mut hits) = (0.0, 0usize, Map::new());
        for ((term, needle), weight) in self.terms.iter().zip(&self.needles).zip(&self.weights) {
            let found = count(text, needle);
            if found > 0 {
                score += weight * (1.0 + (found as f64).ln());
                total += found;
                hits.insert(term.clone(), json!(found));
            }
        }
        (score, total, hits)
    }
}

/// Word-start occurrences of a term, or of its stem when the exact form is absent.
fn count(text: &str, (needle, stemmed): &(String, String)) -> usize {
    let exact = word_start_matches(text, needle);
    if exact == 0 && stemmed != needle {
        word_start_matches(text, stemmed)
    } else {
        exact
    }
}

/// Occurrences of `needle` that begin a word: the character before it is not alphanumeric. An
/// inflection still matches (`lives` for `live`, `recall-journey` for `journey`), a fragment inside
/// another word does not (`delivery` never counts as `live`).
fn word_start_matches(text: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    text.match_indices(needle)
        .filter(|(at, _)| {
            text[..*at]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric())
        })
        .count()
}

/// The section of `path` whose text carries most of the question's terms, if any does.
pub(super) fn best_section(
    args: &Args,
    contract: &Contract,
    path: &str,
    terms: &[String],
) -> Option<Value> {
    let outline = outline_with_terms(args, contract, path, terms).ok()?;
    let mut best: Option<Value> = None;
    for heading in outline["headings"].as_array()? {
        if heading["hit_total"].as_u64().unwrap_or(0) == 0 {
            continue;
        }
        let score = |h: &Value| h["score"].as_f64().unwrap_or(0.0);
        let better = best
            .as_ref()
            .is_none_or(|current| score(heading) > score(current));
        if better {
            best = Some(heading.clone());
        }
    }
    let mut section = best?;
    section["usage"] = outline["usage"].clone();
    Some(section)
}
