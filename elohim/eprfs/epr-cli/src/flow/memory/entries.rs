//! The memory-entry reading and index-rendering half of station four, held at BYTE PARITY with
//! `.claude/scripts/memory-kit/memory-index-projector.py`.
//!
//! Two parsers are how "the projector accepted it but the importer refused it" happens, so this
//! module transcribes ONE oracle deliberately: `.claude/scripts/_lib/frontmatter.py`. The
//! transcription is exact where the projector reads, and additive only where it does not:
//!
//! * **Top-level scalars only.** The Python key regex is anchored at the start of a line that has
//!   been `rstrip`ped alone, so an INDENTED `title:` never matches. Reading indented keys here
//!   would silently retitle every entry whose `metadata:` block happens to carry one.
//! * **Quote stripping is `strip('"')` then `strip("'")`** — repeated quotes are stripped, not one
//!   pair, because that is what Python's `str.strip(chars)` does.
//! * **Truncation is in CHARACTERS, budgets are in BYTES.** `len(title) > 80` counts characters in
//!   Python 3; `len(text.encode("utf-8"))` counts bytes. This corpus is full of em-dashes and
//!   arrows, so conflating the two would drift the index by a few bytes per row and fail parity
//!   on exactly the entries whose titles the rule exists to bound.
//! * **`metadata:` sub-keys are read here and NOWHERE in the projector.** They gate import
//!   (`metadata.type` is required frontmatter) and never reach a rendered row, so admitting them
//!   carries no parity risk. The block ends at the first line that is not indented.

use std::collections::BTreeMap;

/// The projector's generated-file header, byte-for-byte
/// (`memory-index-projector.py:57` `HEADER`). It is part of the projected bytes, so the byte
/// budget and the unloaded-row offset both start after it.
pub const HEADER: &str = "<!-- GENERATED — do not hand-edit. MEMORY.md is projected from .claude/memory/*.md\n     frontmatter (title: + description:) by memory-index-projector.py --apply. -->\n";

/// `memory-index-projector.py:56` `TITLE_MAX`. A render-time bound, not an import-time one: an
/// overlong title is a violation the projector REPORTS and still renders, truncated.
pub const TITLE_MAX: usize = 80;

/// `memory-index-projector.py:55` `DESC_MAX`.
pub const DESC_MAX: usize = 200;

/// The placeholder the projector renders for an entry with no `description:`
/// (`memory-index-projector.py:118`).
///
/// Native import REFUSES such an entry rather than contributing a placeholder claim, so this row
/// can only be reached by a projection over frontmatter, never by one over contributions. It is
/// transcribed anyway so the parity harness compares like with like if such an entry ever lands.
pub const MISSING_DESC: &str =
    "⚠ no description: frontmatter — add a one-line recall hook (<=160 chars)";

/// The frontmatter subset both legs read: flat top-level scalars plus one `metadata:` sub-map.
#[derive(Debug, Default)]
pub struct Frontmatter {
    pub fields: BTreeMap<String, String>,
    pub metadata: BTreeMap<String, String>,
}

impl Frontmatter {
    /// The Python `Frontmatter.get` contract: a missing key reads as the empty string, never an
    /// error, so "absent" and "declared empty" are one case at every call site.
    pub fn get(&self, key: &str) -> &str {
        self.fields.get(key).map_or("", String::as_str)
    }

    pub fn meta(&self, key: &str) -> &str {
        self.metadata.get(key).map_or("", String::as_str)
    }

    /// The entry's declared kind, across the TWO dialects this corpus actually carries.
    ///
    /// The younger entries nest it (`metadata:` → `type:`); the older ones declare a flat
    /// top-level `type:` with no `metadata:` block at all. Both say the same thing, and 15 of the
    /// 229 live entries would be refused for the wrong reason if only one were admitted — three of
    /// them indexed, which would have silently dropped three rows from the projected index and
    /// failed parity against the very projector this leg replaces. `metadata.type` wins when both
    /// are present; it is the declared home the entry template teaches.
    pub fn entry_type(&self) -> &str {
        let nested = self.meta("type").trim();
        if nested.is_empty() {
            self.get("type").trim()
        } else {
            nested
        }
    }

    /// `index: false` opts an entry out of the projected index (projector `collect_entries`),
    /// case-insensitively, the way `fm.get("index").lower() == "false"` reads it.
    pub fn indexed(&self) -> bool {
        !self.get("index").eq_ignore_ascii_case("false")
    }
}

/// Whether a line is a top-level `key: value` pair — the Python `_KV_RE` shape.
///
/// Returns `None` for an indented line, which is the whole reason this is a function and not an
/// inline `split_once(':')`: `split_once` would happily match `  type: project`.
fn top_level_kv(line: &str) -> Option<(&str, &str)> {
    let (key, rest) = line.split_once(':')?;
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some((key, rest.trim()))
}

/// The same shape one indent level down, for the `metadata:` block alone.
fn indented_kv(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    if trimmed.len() == line.len() {
        return None;
    }
    top_level_kv(trimmed)
}

/// Python's `val.strip('"').strip("'")` — repeated quote characters, both ends, in that order.
fn unquote(value: &str) -> &str {
    value.trim_matches('"').trim_matches('\'')
}

/// Parse the `---`-delimited frontmatter of a memory entry.
///
/// An absent or unterminated block yields empty fields rather than an error, matching the oracle:
/// the projector renders such a file from its stem, and import refuses it for want of `name:`.
pub fn parse(text: &str) -> Frontmatter {
    let lines: Vec<&str> = text.lines().collect();
    let mut fm = Frontmatter::default();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return fm;
    }
    let Some(end) = lines[1..]
        .iter()
        .position(|l| l.trim() == "---")
        .map(|i| i + 1)
    else {
        return fm;
    };

    let mut metadata_open = false;
    for line in &lines[1..end] {
        let stripped = line.trim_end();
        if stripped.is_empty() {
            metadata_open = false;
            continue;
        }
        if let Some((key, value)) = top_level_kv(stripped) {
            // A top-level key always closes the metadata block, including `metadata:` itself
            // reopening it. This is why `project_reach_enum_drift_reconciliation.md` — whose
            // `type:` sits after a column-0 `id:` interrupts the block — reads as having no
            // `metadata.type` and is refused by name rather than silently half-read.
            metadata_open = false;
            if value == "|" {
                // Block scalar: the oracle skips it rather than reading the first line.
                continue;
            }
            if value.is_empty() {
                fm.fields.insert(key.to_string(), String::new());
                metadata_open = key == "metadata";
                continue;
            }
            fm.fields
                .insert(key.to_string(), unquote(value).to_string());
            continue;
        }
        if metadata_open {
            if let Some((key, value)) = indented_kv(stripped) {
                fm.metadata
                    .insert(key.to_string(), unquote(value).to_string());
            }
        }
    }
    fm
}

/// Python `_clean_line`: unescape inner quotes, then collapse every whitespace run to one space.
pub fn clean_line(value: &str) -> String {
    value
        .replace("\\\"", "\"")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Python `value[: max - 1].rstrip() + "…"`, in characters.
///
/// A value at exactly `max` is untouched; only `len > max` truncates. The ellipsis is appended
/// AFTER the right-trim, so a truncation landing on a space yields `word…`, never `word …`.
pub fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let head: String = value.chars().take(max - 1).collect();
    format!("{}…", head.trim_end())
}

/// One rendered index row, already cleaned but NOT yet truncated.
///
/// Truncation is deferred to render time on purpose: it is what the oracle does, and it is what
/// lets a contribution carry a title bounded by the collective's 256-byte `concern` limit and
/// still render the identical 80-character row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRow {
    /// The entry's file name — the link target, never a path.
    pub file: String,
    pub title: String,
    pub desc: String,
}

impl IndexRow {
    pub fn line(&self) -> String {
        format!(
            "- [{}]({}) — {}\n",
            truncate(&self.title, TITLE_MAX),
            self.file,
            truncate(&self.desc, DESC_MAX)
        )
    }
}

/// Sort key: the projector globs with `key=lambda q: q.name.lower()`.
pub fn sort_key(file: &str) -> String {
    file.to_lowercase()
}

/// The projected index bytes: header, then one line per row in the caller's order.
pub fn render(rows: &[IndexRow]) -> String {
    let mut out = String::from(HEADER);
    for row in rows {
        out.push_str(&row.line());
    }
    out
}

/// The rows whose cumulative byte offset runs past `hard` — index lines no session can ever read.
///
/// Transcribed from `count_unloaded`, with the ONE change station one requires: the cap is a
/// parameter resolved from the declared measure registry, never a module constant. That is the
/// difference between a threshold and a bound.
pub fn unloaded(rows: &[IndexRow], hard: usize) -> Vec<String> {
    let mut offset = HEADER.len();
    let mut past = Vec::new();
    for row in rows {
        offset += row.line().len();
        if offset > hard {
            past.push(row.file.clone());
        }
    }
    past
}

/// Build an index row straight from an entry's frontmatter, the way the projector does.
///
/// This is the FRONTMATTER path — the parity oracle's own route. The native index projects from
/// contributions instead (`index.rs`); both converge on [`IndexRow`] so the harness can compare
/// them without a second renderer.
pub fn row_from_frontmatter(file: &str, fm: &Frontmatter) -> IndexRow {
    let stem = file.strip_suffix(".md").unwrap_or(file);
    let mut title = clean_line(fm.get("title"));
    if title.is_empty() {
        title = clean_line(fm.get("name"));
    }
    if title.is_empty() {
        title = stem.to_string();
    }
    let desc = clean_line(fm.get("description"));
    IndexRow {
        file: file.to_string(),
        title,
        desc: if desc.is_empty() {
            MISSING_DESC.to_string()
        } else {
            desc
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indented_keys_never_reach_the_rendered_row() {
        let fm = parse("---\nname: slug\ntitle: Real title\nmetadata:\n  title: Nested decoy\n  type: feedback\n---\nbody\n");
        assert_eq!(fm.get("title"), "Real title");
        assert_eq!(fm.meta("type"), "feedback");
    }

    #[test]
    fn a_column_zero_key_closes_the_metadata_block() {
        // The `project_reach_enum_drift_reconciliation.md` shape: `type:` sits after a column-0
        // `id:` has already closed the block, so it is NOT metadata.type.
        let fm =
            parse("---\nname: slug\nmetadata:\n  node_type: memory\nid: x\n  type: project\n---\n");
        assert_eq!(fm.meta("type"), "");
        assert_eq!(fm.meta("node_type"), "memory");
    }

    #[test]
    fn quotes_are_stripped_the_way_python_strips_them() {
        let fm = parse("---\ndescription: \"\"quoted twice\"\"\n---\n");
        assert_eq!(fm.get("description"), "quoted twice");
    }

    #[test]
    fn truncation_counts_characters_and_the_budget_counts_bytes() {
        let title: String = "—".repeat(90);
        let cut = truncate(&title, TITLE_MAX);
        assert_eq!(cut.chars().count(), TITLE_MAX);
        assert_eq!(cut.len(), TITLE_MAX * 3);
        assert_eq!(truncate("exact", 5), "exact");
    }

    #[test]
    fn index_false_is_case_insensitive_and_absence_means_indexed() {
        assert!(!parse("---\nindex: False\n---\n").indexed());
        assert!(parse("---\nname: x\n---\n").indexed());
    }
}
