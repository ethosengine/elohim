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
//! * **The ROW is still the projector's; the LAYOUT no longer is.** Rows are grouped under one
//!   heading per [`Holder`] (`applies-to:`, read here and nowhere in the Python), where the
//!   projector emitted one flat list. The kit projector is deleted, so no second leg renders the
//!   layout; what parity remains is per row — title, link, description, truncation — and the
//!   pinned digest in `flow_memory_import.rs`, which moved deliberately when the headings arrived.

use std::collections::BTreeMap;

use eprfs_agent::memory::AppliesTo;

/// The generated-file header. It is part of the projected bytes, so the byte budget and the
/// unloaded-row offset both start after it.
///
/// It NAMES THE VERB THAT REGENERATES IT. Until 2026-09-11 it named `memory-index-projector.py
/// --apply`, a kit script the replacement deleted: every re-projection reproduced an instruction
/// that could not be followed, in the one file a session reads first. The header diverging from
/// the Python original is why `EXPECTED_INDEX_SHA256` in `flow_memory_import.rs` was re-baselined
/// on that date — the RULE the oracle proves is unchanged, only this instruction line moved.
pub const HEADER: &str = "<!-- GENERATED — do not hand-edit. MEMORY.md is projected from .claude/memory/*.md\n     frontmatter (title: + description:) by\n     epr flow memory project --index --budget memory-index-bytes@1 --out .claude/memory/MEMORY.md -->\n";

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
    /// Set when the frontmatter cannot be read without guessing: a block scalar (`key: |`, `>`, …)
    /// whose body is EMPTY, followed by a line at or left of the key's indent. YAML reads that line
    /// as a new key; an author who forgot to indent meant it as the body. Attribution rests on
    /// which, so the entry is refused as malformed rather than read either way.
    pub malformed: Option<String>,
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

    /// The entry's declared `applies-to:` — `Ok(None)` when it declares none, `Err` naming the
    /// frontmatter that cannot be read as a declaration.
    ///
    /// Both dialects are read, top-level and nested under `metadata:`, because the edit-time gate
    /// counts either as present: reading only one would let an author who wrote
    /// `metadata:\n  applies-to: device` pass the gate believing they declared `device`, and then
    /// be imported at repository reach — a widening exactly where they asked for a narrowing.
    ///
    /// There is no precedence between them. When both carry a value and they DISAGREE the entry is
    /// refused: either winner would silently overrule a declaration someone wrote, and one of the
    /// two losers is always the narrower one. An unknown value in either place is refused the same
    /// way. Absence is not an error — it is the common case, and [`Self::effective_applies_to`]
    /// supplies the kind's default. The `Err` text is the frontmatter as written, in backticks, so
    /// the refusal can quote it.
    pub fn applies_to(&self) -> Result<Option<AppliesTo>, String> {
        let read = |value: &str, spelled: &str| -> Result<Option<AppliesTo>, String> {
            if value.is_empty() {
                return Ok(None);
            }
            AppliesTo::parse(value)
                .map(Some)
                .ok_or_else(|| format!("`{spelled}: {value}`"))
        };
        let top_value = self.get(APPLIES_TO_KEY).trim();
        let nested_value = self.meta(APPLIES_TO_KEY).trim();
        let top = read(top_value, APPLIES_TO_KEY)?;
        let nested = read(nested_value, &format!("metadata.{APPLIES_TO_KEY}"))?;
        match (top, nested) {
            (Some(a), Some(b)) if a != b => Err(format!(
                "`{APPLIES_TO_KEY}: {top_value}` and `metadata.{APPLIES_TO_KEY}: {nested_value}`, \
                 which disagree"
            )),
            (top, nested) => Ok(top.or(nested)),
        }
    }

    /// Whom the entry applies to: its declaration, else the default for its declared kind.
    pub fn effective_applies_to(&self) -> Result<AppliesTo, String> {
        Ok(self
            .applies_to()?
            .unwrap_or_else(|| AppliesTo::default_for_type(self.entry_type())))
    }
}

/// The frontmatter key, spelled once for the reader and the refusal that names it.
pub const APPLIES_TO_KEY: &str = "applies-to";

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
    // The open block scalar (`|`, `|-`, `>`, …), at the top level or inside `metadata:`: its key's
    // indent, its key, and whether an indented body line has been seen. Every line indented DEEPER
    // than the key is the scalar's BODY, never a key — without this a body quoting
    // `originSessionId: x` would be read as the entry's own origin, and attribution rests on it.
    // The body ends at the next non-blank line at or left of the key's indent; if the body was
    // EMPTY by then, that line is ambiguous (see [`Frontmatter::malformed`]).
    let mut block: Option<(usize, String, bool)> = None;
    for line in &lines[1..end] {
        let stripped = line.trim_end();
        let indent = stripped.len() - stripped.trim_start().len();
        if let Some((key_indent, key, seen_body)) = block.take() {
            if stripped.is_empty() {
                block = Some((key_indent, key, seen_body));
                continue;
            }
            if indent > key_indent {
                block = Some((key_indent, key, true));
                continue;
            }
            if !seen_body && fm.malformed.is_none() {
                fm.malformed = Some(format!(
                    "block scalar `{key}:` has no indented body, and the line after it — \
                     `{}` — could be its unindented body or a new key; refused rather than read \
                     either way",
                    stripped.trim()
                ));
            }
        }
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
            if value.starts_with('|') || value.starts_with('>') {
                block = Some((0, key.to_string(), false));
            }
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
                if value.starts_with('|') || value.starts_with('>') {
                    block = Some((indent, key.to_string(), false));
                    continue;
                }
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

/// Whose memory a group of index rows is: what the collective holds, or one steward's direction.
///
/// This replaces a per-row `[steward of record: X]` prefix. The prefix said who STOOD for a row;
/// it could not say whether the row was shared knowledge or that person's own preference, and a
/// second developer's first session needs exactly that distinction. One heading per holder says
/// it once instead of on every line, which also returns the prefix's bytes to the budget.
///
/// The variant order IS the render order: the collective's shared memory first, then each named
/// steward by name, then direction no steward stands for.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Holder {
    /// `applies-to` repository. Device entries never get a row in the tracked index at all: their
    /// reach stops at the workspace, so `index.rs` counts them and leaves them out.
    Collective,
    /// Steward-bound rows one named participant stands for.
    Steward(String),
    /// Steward-bound rows whose record names no steward — an agent wrote them and nobody stood
    /// for them. Named as such rather than credited to the repository, which directs nothing.
    Unstewarded,
}

impl Holder {
    /// The heading line, newline included: its bytes count against the budget like a row's.
    pub fn heading(&self) -> String {
        match self {
            Holder::Collective => "## Held by the collective\n".to_string(),
            Holder::Steward(name) => format!("## Direction from {name} (theirs, on record)\n"),
            Holder::Unstewarded => "## Direction with no steward on record\n".to_string(),
        }
    }
}

/// One holder's rows, under its heading, in [`sort_key`] order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexGroup {
    pub holder: Holder,
    pub rows: Vec<IndexRow>,
}

/// Gather rows under their holders: holders in [`Holder`] order, rows within each by [`sort_key`].
///
/// A holder with no rows has no heading — an empty section would spend budget saying nothing.
pub fn group(rows: Vec<(Holder, IndexRow)>) -> Vec<IndexGroup> {
    let mut by_holder: BTreeMap<Holder, Vec<IndexRow>> = BTreeMap::new();
    for (holder, row) in rows {
        by_holder.entry(holder).or_default().push(row);
    }
    by_holder
        .into_iter()
        .map(|(holder, mut rows)| {
            rows.sort_by_key(|row| sort_key(&row.file));
            IndexGroup { holder, rows }
        })
        .collect()
}

/// The projected index bytes: header, then each group's heading and its rows, in order.
pub fn render(groups: &[IndexGroup]) -> String {
    let mut out = String::from(HEADER);
    for group in groups {
        out.push_str(&group.holder.heading());
        for row in &group.rows {
            out.push_str(&row.line());
        }
    }
    out
}

/// The rows whose cumulative byte offset runs past `hard` — index lines no session can ever read.
///
/// Transcribed from `count_unloaded`, with the ONE change station one requires: the cap is a
/// parameter resolved from the declared measure registry, never a module constant. That is the
/// difference between a threshold and a bound.
///
/// Headings advance the offset exactly as [`render`] emits them, so a row's position here is its
/// position in the file; a heading is never itself reported, since it is not a row anyone lost.
pub fn unloaded(groups: &[IndexGroup], hard: usize) -> Vec<String> {
    let mut offset = HEADER.len();
    let mut past = Vec::new();
    for group in groups {
        offset += group.holder.heading().len();
        for row in &group.rows {
            offset += row.line().len();
            if offset > hard {
                past.push(row.file.clone());
            }
        }
    }
    past
}

/// How many rows the groups carry — the index's `entries` count, headings excluded.
pub fn row_count(groups: &[IndexGroup]) -> usize {
    groups.iter().map(|g| g.rows.len()).sum()
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

    #[test]
    fn applies_to_defaults_by_kind_and_a_declaration_overrides_it() {
        let of = |kind: &str, extra: &str| {
            parse(&format!(
                "---\nname: x\n{extra}metadata:\n  type: {kind}\n---\n"
            ))
        };
        for (kind, want) in [
            ("user", AppliesTo::Steward),
            ("feedback", AppliesTo::Steward),
            ("project", AppliesTo::Repository),
            ("reference", AppliesTo::Repository),
        ] {
            let fm = of(kind, "");
            assert_eq!(fm.applies_to(), Ok(None), "{kind}: nothing declared");
            assert_eq!(
                fm.effective_applies_to(),
                Ok(want),
                "{kind}: the kind's default"
            );
        }
        // The flat `type:` dialect defaults the same way.
        assert_eq!(
            parse("---\ntype: feedback\n---\n").effective_applies_to(),
            Ok(AppliesTo::Steward)
        );
        let declared = of("feedback", "applies-to: repository\n");
        assert_eq!(declared.applies_to(), Ok(Some(AppliesTo::Repository)));
        assert_eq!(declared.effective_applies_to(), Ok(AppliesTo::Repository));
        assert_eq!(
            of("project", "applies-to: \"device\"\n").effective_applies_to(),
            Ok(AppliesTo::Device)
        );
    }

    #[test]
    fn an_unknown_or_miscased_applies_to_is_an_error_never_a_default() {
        for value in ["everyone", "Steward", "repo"] {
            let fm = parse(&format!(
                "---\napplies-to: {value}\nmetadata:\n  type: feedback\n---\n"
            ));
            assert_eq!(fm.applies_to(), Err(format!("`applies-to: {value}`")));
            assert!(fm.effective_applies_to().is_err());
            let nested = parse(&format!(
                "---\nmetadata:\n  type: feedback\n  applies-to: {value}\n---\n"
            ));
            assert_eq!(
                nested.applies_to(),
                Err(format!("`metadata.applies-to: {value}`"))
            );
        }
    }

    #[test]
    fn a_nested_applies_to_is_a_declaration_and_two_that_disagree_are_refused() {
        // The edit-time gate counts the nested key as present, so the reader must too.
        let nested = parse("---\nmetadata:\n  type: project\n  applies-to: device\n---\n");
        assert_eq!(nested.applies_to(), Ok(Some(AppliesTo::Device)));
        // The same value in both places is one declaration.
        let both = parse(
            "---\napplies-to: device\nmetadata:\n  type: project\n  applies-to: device\n---\n",
        );
        assert_eq!(both.applies_to(), Ok(Some(AppliesTo::Device)));
        // Different values have no winner: either would overrule what someone wrote.
        let split = parse(
            "---\napplies-to: repository\nmetadata:\n  type: project\n  applies-to: device\n---\n",
        );
        assert_eq!(
            split.applies_to(),
            Err(
                "`applies-to: repository` and `metadata.applies-to: device`, which disagree"
                    .to_string()
            )
        );
    }

    fn row(file: &str) -> IndexRow {
        IndexRow {
            file: file.into(),
            title: format!("T {file}"),
            desc: "d".into(),
        }
    }

    #[test]
    fn rows_group_collective_first_then_stewards_by_name_then_the_unstewarded() {
        let groups = group(vec![
            (Holder::Unstewarded, row("u.md")),
            (Holder::Steward("human:ruth".into()), row("r.md")),
            (Holder::Collective, row("b.md")),
            (Holder::Steward("human:matthew".into()), row("m2.md")),
            (Holder::Collective, row("A.md")),
            (Holder::Steward("human:matthew".into()), row("m1.md")),
        ]);
        let text = render(&groups);
        let body: Vec<&str> = text[HEADER.len()..].lines().collect();
        assert_eq!(
            body,
            [
                "## Held by the collective",
                "- [T A.md](A.md) — d",
                "- [T b.md](b.md) — d",
                "## Direction from human:matthew (theirs, on record)",
                "- [T m1.md](m1.md) — d",
                "- [T m2.md](m2.md) — d",
                "## Direction from human:ruth (theirs, on record)",
                "- [T r.md](r.md) — d",
                "## Direction with no steward on record",
                "- [T u.md](u.md) — d",
            ]
        );
        assert_eq!(row_count(&groups), 6);
        assert!(group(Vec::new()).is_empty(), "no rows, no headings");
    }

    #[test]
    fn unloaded_counts_heading_bytes_where_render_puts_them() {
        let groups = group(vec![
            (Holder::Collective, row("a.md")),
            (Holder::Steward("human:matthew".into()), row("m.md")),
        ]);
        let text = render(&groups);
        // A cap at exactly the whole file loads every row; one byte short loses the last one.
        assert!(unloaded(&groups, text.len()).is_empty());
        assert_eq!(unloaded(&groups, text.len() - 1), ["m.md"]);
        // A cap that ends inside the SECOND heading loses the row after it, even though the
        // first group's bytes alone fit: the heading is what pushed it past.
        let first = HEADER.len() + Holder::Collective.heading().len() + row("a.md").line().len();
        assert_eq!(unloaded(&groups, first), ["m.md"]);
        assert_eq!(unloaded(&groups, first - 1), ["a.md", "m.md"]);
    }
}
