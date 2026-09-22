//! S3 (2026-09-22 recall-Codex-trail sprint) — prose that lags its value: when `read` or `source`
//! serves a `.json` (or, best-effort, `.yaml`/`.yml`) source, name every place where a comment or
//! string carries a stale numeric claim `key=N` / `key: N` while the document's own field named
//! `key` currently holds a different, unambiguous number — together with the dated `_`-prefixed
//! sibling key (in the SAME object) that recorded when it changed.
//!
//! Motivating case: `genesis/agentic/pool-policy.json`'s `io._comment` says `max_concurrent_heavy=1
//! means one cargo build…`, while `io.max_concurrent_heavy` is `2` and `io._capacity_2026_09_09`
//! is the dated sibling that recorded the widening. A reader of the comment alone would wrongly
//! serialize builds — this is candidate discovery over that drift, never a rewrite: the fixture
//! this station tests against is a LIVE document whose history is deliberately kept, so it is
//! never edited by this code.
//!
//! Deliberately hand-rolled rather than a JSON/YAML AST with source spans (neither `serde_json`
//! nor `serde_yaml` carries byte/line positions without an extra crate this workspace does not
//! depend on) — a bounded, line-tracking character scan, in the same spirit as this seam's other
//! hand-rolled readers (`discovery.rs`'s habit-register line scan).
use super::*;

/// Whether `path`'s extension names a source this station scans, and which grammar to use.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Json,
    Yaml,
}

pub(super) fn kind_of(path: &str) -> Option<Kind> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".json") {
        Some(Kind::Json)
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        Some(Kind::Yaml)
    } else {
        None
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The scan
// ───────────────────────────────────────────────────────────────────────────────────────────────

struct KeyOcc {
    key: String,
    value: f64,
    line: usize,
    object_id: u32,
}

struct StrOcc {
    text: String,
    line: usize,
}

struct DatedKey {
    key: String,
    object_id: u32,
}

/// `YYYY-MM-DD` or `YYYY_MM_DD` anywhere inside `text` — used only against a KEY NAME (a
/// provenance sibling names itself with the date it was decided, e.g. `_capacity_2026_09_09`),
/// never against a value.
fn has_date_token(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() < 10 {
        return false;
    }
    let digit = |b: u8| b.is_ascii_digit();
    for window in bytes.windows(10) {
        if digit(window[0])
            && digit(window[1])
            && digit(window[2])
            && digit(window[3])
            && matches!(window[4], b'-' | b'_')
            && digit(window[5])
            && digit(window[6])
            && matches!(window[7], b'-' | b'_')
            && digit(window[8])
            && digit(window[9])
        {
            return true;
        }
    }
    false
}

/// `key=N` / `key: N` claims embedded in ordinary prose — `key` a bare identifier
/// (`[A-Za-z_][A-Za-z0-9_]*`), `N` a signed integer or decimal. Deliberately permissive (it will
/// also match `group=1` inside `memory.oom.group=1`): a claim naming a key this document does not
/// otherwise declare with a single unambiguous value is simply never looked up successfully below,
/// so a loose match here costs nothing but a discarded candidate.
fn find_claims(text: &str) -> Vec<(String, f64)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            i += 1;
            continue;
        }
        let key_start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let key = &text[key_start..i];
        let mut j = i;
        while j < bytes.len() && bytes[j] == b' ' {
            j += 1;
        }
        if j < bytes.len() && (bytes[j] == b'=' || bytes[j] == b':') {
            let mut k = j + 1;
            while k < bytes.len() && bytes[k] == b' ' {
                k += 1;
            }
            let num_start = k;
            if k < bytes.len() && bytes[k] == b'-' {
                k += 1;
            }
            let digits_start = k;
            while k < bytes.len() && (bytes[k].is_ascii_digit() || bytes[k] == b'.') {
                k += 1;
            }
            if k > digits_start && bytes[digits_start].is_ascii_digit() {
                if let Ok(value) = text[num_start..k].parse::<f64>() {
                    out.push((key.to_string(), value));
                }
            }
        }
    }
    out
}

/// Decode one JSON string literal starting at `bytes[0] == '"'`. Returns the decoded text, the
/// number of bytes consumed (including both quotes) and how many literal newlines it contained.
/// Raw bytes are collected and decoded once via `from_utf8_lossy` so a multibyte UTF-8 character
/// (this repository's prose is full of em dashes) survives intact rather than being mangled
/// byte-by-byte.
fn read_json_string(bytes: &[u8]) -> (String, usize, usize) {
    let mut i = 1usize;
    let mut raw: Vec<u8> = Vec::new();
    let mut newlines = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => {
                match bytes[i + 1] {
                    b'n' => raw.push(b'\n'),
                    b't' => raw.push(b'\t'),
                    other => raw.push(other),
                }
                i += 2;
            }
            b'"' => {
                i += 1;
                break;
            }
            b'\n' => {
                newlines += 1;
                raw.push(b'\n');
                i += 1;
            }
            c => {
                raw.push(c);
                i += 1;
            }
        }
    }
    (String::from_utf8_lossy(&raw).into_owned(), i, newlines)
}

/// A bounded, brace-aware character scan of a JSON document (or a JSON-shaped fragment — an
/// EXCERPT does not carry the whole document's opening/closing braces, and this degrades
/// gracefully rather than panicking: `object_stack` never pops below its root sentinel).
/// `line_offset` is the 1-based line number of `text`'s own first line within the whole file, so a
/// bounded excerpt still reports real file line numbers.
fn scan_json(text: &str, line_offset: usize) -> (Vec<KeyOcc>, Vec<StrOcc>, Vec<DatedKey>) {
    let mut keys = Vec::new();
    let mut strings = Vec::new();
    let mut dated = Vec::new();
    let mut object_stack: Vec<u32> = vec![0];
    let mut next_id: u32 = 1;
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut line = line_offset;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b'{' => {
                object_stack.push(next_id);
                next_id += 1;
                i += 1;
            }
            b'}' => {
                if object_stack.len() > 1 {
                    object_stack.pop();
                }
                i += 1;
            }
            b'"' => {
                let key_line = line;
                let (text_here, consumed, newlines) = read_json_string(&bytes[i..]);
                i += consumed;
                line += newlines;
                let mut j = i;
                while j < bytes.len() && matches!(bytes[j], b' ' | b'\t') {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b':' {
                    let key = text_here;
                    let object_id = *object_stack.last().unwrap_or(&0);
                    if key.starts_with('_') && has_date_token(&key) {
                        dated.push(DatedKey {
                            key: key.clone(),
                            object_id,
                        });
                    }
                    let mut k = j + 1;
                    let mut skip_lines = 0usize;
                    while k < bytes.len() && matches!(bytes[k], b' ' | b'\t' | b'\n' | b'\r') {
                        if bytes[k] == b'\n' {
                            skip_lines += 1;
                        }
                        k += 1;
                    }
                    if k < bytes.len() && bytes[k] == b'"' {
                        let (value_text, vconsumed, vnewlines) = read_json_string(&bytes[k..]);
                        strings.push(StrOcc {
                            text: value_text,
                            line: key_line + skip_lines,
                        });
                        i = k + vconsumed;
                        line += skip_lines + vnewlines;
                    } else if k < bytes.len() && (bytes[k].is_ascii_digit() || bytes[k] == b'-') {
                        let start = k;
                        let mut m = k + 1;
                        while m < bytes.len() && (bytes[m].is_ascii_digit() || bytes[m] == b'.') {
                            m += 1;
                        }
                        if let Ok(value) = text[start..m].parse::<f64>() {
                            keys.push(KeyOcc {
                                key,
                                value,
                                line: key_line + skip_lines,
                                object_id,
                            });
                        }
                        i = m;
                        line += skip_lines;
                    } else {
                        i = k;
                        line += skip_lines;
                    }
                } else {
                    // A bare string VALUE (an array element, or any string not itself followed by
                    // `:`) — still scanned for embedded claims, at whatever object currently holds
                    // it.
                    strings.push(StrOcc {
                        text: text_here,
                        line: key_line,
                    });
                }
            }
            _ => i += 1,
        }
    }
    (keys, strings, dated)
}

/// A best-effort, indentation-tracked scan of a YAML document: siblings at the same indentation
/// under the same opened mapping share one `object_id`. List items (`- …`) are treated
/// transparently (their `- ` marker is stripped, but a new list entry does not open its own
/// object scope) — a deliberate simplification for a format this station only needs to handle "if
/// cheap" (see the module doc); it never produces a FALSE disagreement, only occasionally an
/// over- or under-attributed provenance list.
fn scan_yaml(text: &str, line_offset: usize) -> (Vec<KeyOcc>, Vec<StrOcc>, Vec<DatedKey>) {
    let mut keys = Vec::new();
    let mut strings = Vec::new();
    let mut dated = Vec::new();
    // (indent threshold below which this scope no longer applies, object_id)
    let mut stack: Vec<(usize, u32)> = vec![(0, 0)];
    let mut next_id: u32 = 1;
    for (offset, raw_line) in text.lines().enumerate() {
        let line_no = line_offset + offset;
        let indent = raw_line.len() - raw_line.trim_start().len();
        let trimmed = raw_line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        // Popped BEFORE either the comment or the key below reads `object_id`, so a line that
        // dedents attributes its own comment to the SAME (correct, just-popped-back-to) object as
        // its key — not to whatever deeper scope the previous line left behind.
        while stack.len() > 1 && indent < stack.last().unwrap().0 {
            stack.pop();
        }
        let object_id = stack.last().map(|(_, id)| *id).unwrap_or(0);
        // Whole-line or trailing `#` comment: scanned as prose regardless of what precedes it.
        if let Some(hash) = find_unquoted_hash(trimmed) {
            let comment = trimmed[hash + 1..].trim();
            if !comment.is_empty() {
                strings.push(StrOcc {
                    text: comment.to_string(),
                    line: line_no,
                });
            }
        }
        let code = strip_comment(trimmed);
        let code = code.trim_end();
        if code.is_empty() {
            continue;
        }
        let code = code.strip_prefix("- ").unwrap_or(code);
        let Some((key_raw, rest)) = code.split_once(':') else {
            continue;
        };
        let key = key_raw
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            continue;
        }
        if key.starts_with('_') && has_date_token(&key) {
            dated.push(DatedKey {
                key: key.clone(),
                object_id,
            });
        }
        let value = rest.trim();
        if value.is_empty() {
            // Opens a nested mapping/sequence; its members will indent deeper than this key.
            stack.push((indent + 1, next_id));
            next_id += 1;
            continue;
        }
        if let Some(inner) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
            strings.push(StrOcc {
                text: inner.to_string(),
                line: line_no,
            });
            continue;
        }
        if let Ok(number) = value.trim_end_matches(',').parse::<f64>() {
            keys.push(KeyOcc {
                key,
                value: number,
                line: line_no,
                object_id,
            });
            continue;
        }
        // An unquoted scalar (bool/null/bare word/block-scalar opener) — scanned as prose too, so
        // a stray claim in an unquoted value is not silently dropped.
        strings.push(StrOcc {
            text: value.to_string(),
            line: line_no,
        });
    }
    (keys, strings, dated)
}

/// The index of an unescaped, unquoted `#` in a YAML line — `None` when the whole line carries no
/// comment. A best-effort quote tracker (does not understand YAML's own escape rules inside
/// single- vs double-quoted scalars in full generality), sufficient for "if cheap."
fn find_unquoted_hash(line: &str) -> Option<usize> {
    let mut in_quote: Option<char> = None;
    for (index, ch) in line.char_indices() {
        match in_quote {
            Some(q) if ch == q => in_quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => in_quote = Some(ch),
            None if ch == '#' && (index == 0 || line.as_bytes()[index - 1] == b' ') => {
                return Some(index)
            }
            None => {}
        }
    }
    None
}

fn strip_comment(line: &str) -> &str {
    match find_unquoted_hash(line) {
        Some(index) => &line[..index],
        None => line,
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Effective values, then disagreements
// ───────────────────────────────────────────────────────────────────────────────────────────────

struct Effective {
    value: f64,
    line: usize,
    object_id: u32,
}

/// For each key name, the single value it holds everywhere it appears as a real scalar field —
/// `None` when the same key name carries more than one distinct value anywhere in the document
/// (spec: "exactly one distinct scalar numeric value"), since there is then no ONE effective value
/// a stale claim could be compared against.
fn effective_values(keys: &[KeyOcc]) -> BTreeMap<String, Option<Effective>> {
    let mut by_key: BTreeMap<String, Vec<&KeyOcc>> = BTreeMap::new();
    for occurrence in keys {
        by_key
            .entry(occurrence.key.clone())
            .or_default()
            .push(occurrence);
    }
    by_key
        .into_iter()
        .map(|(key, occurrences)| {
            let mut distinct: Vec<f64> = Vec::new();
            for occurrence in &occurrences {
                if !distinct.contains(&occurrence.value) {
                    distinct.push(occurrence.value);
                }
            }
            let effective = (distinct.len() == 1).then(|| {
                let first = occurrences[0];
                Effective {
                    value: first.value,
                    line: first.line,
                    object_id: first.object_id,
                }
            });
            (key, effective)
        })
        .collect()
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

fn provenance_for(dated: &[DatedKey], object_id: u32) -> Vec<String> {
    let mut names: Vec<String> = dated
        .iter()
        .filter(|d| d.object_id == object_id)
        .map(|d| d.key.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The whole station: scan `text` (already read by the caller, bounded by ITS OWN read budget —
/// see `read_disagreements`/`source_disagreements`), find every prose claim that disagrees with
/// the document's own single, unambiguous effective value for that key, and name its provenance.
fn scan(text: &str, kind: Kind, line_offset: usize) -> Vec<Value> {
    let (keys, strings, dated) = match kind {
        Kind::Json => scan_json(text, line_offset),
        Kind::Yaml => scan_yaml(text, line_offset),
    };
    let effective = effective_values(&keys);
    let mut found = Vec::new();
    for occurrence in &strings {
        for (claim_key, claim_value) in find_claims(&occurrence.text) {
            let Some(Some(effective_value)) = effective.get(&claim_key) else {
                continue;
            };
            if effective_value.value == claim_value {
                continue;
            }
            found.push(json!({
                "key": claim_key,
                "prose_value": format_number(claim_value),
                "prose_line": occurrence.line,
                "effective_value": format_number(effective_value.value),
                "effective_line": effective_value.line,
                "provenance": provenance_for(&dated, effective_value.object_id),
            }));
        }
    }
    found.sort_by(|a, b| {
        a["prose_line"]
            .as_u64()
            .cmp(&b["prose_line"].as_u64())
            .then_with(|| a["key"].as_str().cmp(&b["key"].as_str()))
    });
    found.dedup_by(|a, b| a == b);
    found
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Entry points — `read`'s already-fetched excerpt, `source`'s own small bounded read
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// `read`'s contribution: scan the EXCERPT content it already fetched (no second read at all) —
/// bounded to exactly the lines the reader asked for, so a disagreement is only found here when
/// both the claim and the effective value fall inside that same window.
pub(super) fn from_excerpt(content: &str, path: &str, start_line: usize) -> Option<Value> {
    let kind = kind_of(path)?;
    let found = scan(content, kind, start_line);
    Some(json!({
        "path": path,
        "found": found,
        "scope": "the read excerpt only; a claim and its effective value both had to fall inside the requested lines",
    }))
}

/// `source`'s contribution: one small bounded read of the WHOLE file, capped at the same
/// `scan_bytes` ceiling `outline()` already reads under (never a second, wider read) — charged
/// honestly to `scan_bytes`/`scanned_files` usage. A file bigger than the budget is named as an
/// omission rather than partially scanned and silently under-reported.
pub(super) fn from_source(
    root: &Path,
    contract: &Contract,
    path: &str,
    usage: &mut Value,
) -> FlowResult<Option<Value>> {
    let Some(kind) = kind_of(path) else {
        return Ok(None);
    };
    let source = contained(root, path, &contract.source_roots())?;
    let bound = contract.limit_usize("scan_bytes");
    let mut raw = Vec::new();
    File::open(&source)
        .and_then(|f| f.take(bound as u64 + 1).read_to_end(&mut raw).map(|_| ()))
        .map_err(|error| FlowError::Read {
            path: source.clone(),
            source: error,
        })?;
    let mut omissions: Vec<String> = Vec::new();
    if raw.len() > bound {
        omissions.push(format!(
            "{path} exceeds the disagreement scan's byte budget ({bound}); only a leading window was inspected"
        ));
        raw.truncate(bound);
    }
    add_usage(usage, &json!({"scan_bytes": raw.len(), "scanned_files": 1}));
    let text = String::from_utf8_lossy(&raw);
    let found = scan(&text, kind, 1);
    Ok(Some(json!({
        "path": path,
        "found": found,
        "scope": "the bounded window this station read, up to the same scan_bytes ceiling `source` uses",
        "omissions": omissions,
    })))
}
