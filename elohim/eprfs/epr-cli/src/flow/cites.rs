//! `epr flow cites` — the native cite writer.
//!
//! The doc plane's citation envelopes (`slug | desc | sha256:hex16 [| status: …] [| path: …]`)
//! are TOOL-generated: nobody hand-writes a slug, a fingerprint or a locator. This module is the
//! writer for them, replacing `.claude/scripts/memory-kit/{cite-gen,cite-describe,cite-propagate,
//! cites-migrate}.py`. The reader half already lives next door — [`super::edges`] indexes sealed
//! envelopes and [`super::edges::derive_verdict`] decides whether one drifted — and `stamp`
//! delegates the digest comparison to exactly that function, so the inline `status:` a doc carries
//! in git and the verdict `epr flow concerns` prints are one computation with two renderings.
//!
//! ## Why this module carries its own frontmatter reader
//!
//! [`super::parse_frontmatter`] is the crate's reader and it is deliberately NOT reused for the
//! `cites:` list, on two measured divergences from the Python oracle this writer must reproduce
//! byte-for-byte (measured over the 1101-document corpus, 2026-09-10):
//!
//! * **A `#` comment inside the list closes it** in the oracle (12 live documents); the crate's
//!   reader reads through, by design, because the placement reading must not lose `cites:` entries.
//!   The WRITER drops only the contiguous `- ` run under `cites:`, so a reader that sees more
//!   entries than the writer removes would duplicate every entry below the comment.
//! * **An escaped quote is unescaped** by the oracle (9 live envelopes carry `\"`); the crate's
//!   `trim_matches('"')` leaves the backslash, and re-minting would double it on every pass.
//!
//! Scalars (`id:`, `title:`) parse identically in both, so those go through the crate reader.
//!
//! ## Byte layout
//!
//! Every write reassembles the file from its own lines: the frontmatter block is edited in place
//! (only the `id:` insertion and the `cites:` list block are touched), the body is carried through
//! verbatim, and exactly one trailing newline is guaranteed — the oracle's `_reassemble` contract.
//! Frontmatter is NEVER re-serialized through a YAML round-trip.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cid::Cid;
use elohim_epr_rea::Governor;
use eprfs_core::BlobCid;
use serde::Serialize;

use super::edges::{derive_verdict, EdgePlane, IndexedEdge, SealForm, Verdict};
use super::{canonical_body, parse_frontmatter, FlowError, FlowResult};

/// The envelope segment separator.
const SEP: &str = " | ";

/// Filenames that are index surfaces rather than citing documents (the oracle's `SKIP` set).
const SKIP_NAMES: &[&str] = &[
    "CLAUDE.md",
    "MEMORY.md",
    "INDEX.md",
    "README.md",
    "TRAJECTORY.md",
    "claude.md",
];

/// Directories the gospel walk prunes (vendored/build trees), matching `cite_graph.GOSPEL_EXCLUDE_DIRS`.
const GOSPEL_EXCLUDE_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    ".angular",
    "coverage",
    "target",
    "sophia",
];

/// The raw multicodec (0x55) — the codec every canonical document body carries.
const RAW_CODEC: u64 = 0x55;

/// Suffixes that make a `|`-less cite string a path rather than a bare slug.
const PATH_EXTS: &[&str] = &[
    ".md", ".yaml", ".yml", ".py", ".ts", ".tsx", ".js", ".rs", ".json", ".toml", ".sh",
    ".feature", ".css", ".html", ".sql", ".groovy", ".mjs",
];

/// Slug stems too generic to identify a document — fall through to the title.
const GENERIC_STEMS: &[&str] = &["design", "plan", "readme", "index", "claude", "spec"];

/// Bare content filenames repeated under many directories — disambiguate with the parent directory.
const CONTENT_AMBIGUOUS_STEMS: &[&str] = &[
    "epic",
    "overview",
    "summary",
    "notes",
    "vision",
    "manifesto",
    "story",
];

// ---------------------------------------------------------------------------
// The envelope
// ---------------------------------------------------------------------------

/// One parsed `cites:` entry. A legacy path-string cite (no `|`) keeps its path in `reference`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cite {
    pub reference: String,
    pub legacy_path: bool,
    pub desc: String,
    pub fingerprint: String,
    pub status: String,
    pub path: String,
}

/// Strip a trailing YAML inline comment (` # note`), mirroring `cite_graph._INLINE_COMMENT`
/// (`\s+#(\s.*)?$`, leftmost match). The minimal frontmatter reader keeps `- path   # note` as one
/// string, so the envelope parser has to cut it here.
fn strip_inline_comment(raw: &str) -> &str {
    let bytes: Vec<char> = raw.chars().collect();
    for (i, c) in bytes.iter().enumerate() {
        if *c != '#' || i == 0 {
            continue;
        }
        if !bytes[i - 1].is_whitespace() {
            continue;
        }
        // `(\s.*)?$` — either the comment marker ends the string or whitespace follows it.
        if i + 1 < bytes.len() && !bytes[i + 1].is_whitespace() {
            continue;
        }
        // The match starts at the beginning of the whitespace run before `#`.
        let mut start = i;
        while start > 0 && bytes[start - 1].is_whitespace() {
            start -= 1;
        }
        let byte_start = raw
            .char_indices()
            .nth(start)
            .map(|(b, _)| b)
            .unwrap_or(raw.len());
        return &raw[..byte_start];
    }
    raw
}

/// Unwrap a quoted scalar the way the oracle does: matching outer quotes, and `\"` unescaped
/// inside a double-quoted item.
fn unquote(item: &str) -> String {
    let chars: Vec<char> = item.chars().collect();
    if chars.len() >= 2
        && chars[0] == chars[chars.len() - 1]
        && (chars[0] == '"' || chars[0] == '\'')
    {
        let inner: String = chars[1..chars.len() - 1].iter().collect();
        return if chars[0] == '"' {
            inner.replace("\\\"", "\"")
        } else {
            inner
        };
    }
    item.to_string()
}

/// True for a fingerprint token: the `sha256:hex16` short form, or a full CIDv1 (`baf…`).
fn is_fingerprint_token(segment: &str) -> bool {
    segment.starts_with("sha256:") || segment.starts_with("baf")
}

/// Parse one cite string into its segments — the exact port of `cite_graph.parse_cite`.
pub fn parse_cite(entry: &str) -> Cite {
    let trimmed = entry.trim();
    let raw = strip_inline_comment(trimmed).trim();
    let raw = unquote(raw);
    if !raw.contains('|') {
        let is_path = raw.contains('/') || PATH_EXTS.iter().any(|e| raw.ends_with(e));
        return Cite {
            reference: raw,
            legacy_path: is_path,
            desc: String::new(),
            fingerprint: String::new(),
            status: String::new(),
            path: String::new(),
        };
    }
    let parts: Vec<&str> = raw.split('|').map(str::trim).collect();
    let reference = parts[0].to_string();
    let (mut desc, mut fingerprint, mut status, mut path) =
        (String::new(), String::new(), String::new(), String::new());
    for part in &parts[1..] {
        let lower = part.to_lowercase();
        if is_fingerprint_token(part) {
            fingerprint = (*part).to_string();
        } else if lower.starts_with("status:") {
            status = part
                .split_once(':')
                .map(|(_, v)| v.trim())
                .unwrap_or("")
                .to_string();
        } else if lower.starts_with("path:") {
            path = part
                .split_once(':')
                .map(|(_, v)| v.trim())
                .unwrap_or("")
                .to_string();
        } else if desc.is_empty() {
            desc = (*part).to_string();
        }
    }
    Cite {
        reference,
        legacy_path: false,
        desc,
        fingerprint,
        status,
        path,
    }
}

/// Render a cite back to its single-line envelope — the exact port of `cite_graph.serialize_cite`.
///
/// The oracle's filter is `p != "" or p == parts[0]`, compared BY VALUE: an empty description is
/// dropped, and every other populated segment is kept in declaration order.
pub fn serialize_cite(cite: &Cite) -> String {
    if cite.legacy_path {
        return cite.reference.clone();
    }
    let mut parts = vec![cite.reference.clone(), cite.desc.clone()];
    if !cite.fingerprint.is_empty() {
        parts.push(cite.fingerprint.clone());
    }
    if !cite.status.is_empty() {
        parts.push(format!("status: {}", cite.status));
    }
    if !cite.path.is_empty() {
        parts.push(format!("path: {}", cite.path));
    }
    let first = parts[0].clone();
    parts
        .into_iter()
        .filter(|p| !p.is_empty() || *p == first)
        .collect::<Vec<_>>()
        .join(SEP)
}

/// Render one cite string as a YAML list-item scalar. An envelope carrying `: ` (which every
/// `status:`/`path:` one does) is invalid as a plain scalar — `path:` opens a mapping mid-scalar
/// and hard-blocks the native frontmatter evaluator — so those are minted double-quoted.
pub fn yaml_cite_item(s: &str) -> String {
    if s.contains(": ") {
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

// ---------------------------------------------------------------------------
// Frontmatter: split, read, write, reassemble
// ---------------------------------------------------------------------------

/// Split a document into `(frontmatter lines, body)`, or `None` when no `---` block opens it.
/// Mirrors `cite_graph.split_frontmatter`: `splitlines()` semantics, `strip() == "---"` fences.
pub fn split_frontmatter(text: &str) -> Option<(Vec<String>, String)> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return None;
    }
    let end = lines.iter().enumerate().skip(1).find_map(|(i, l)| {
        if l.trim() == "---" {
            Some(i)
        } else {
            None
        }
    })?;
    let fm = lines[1..end].iter().map(|s| (*s).to_string()).collect();
    Some((fm, lines[end + 1..].join("\n")))
}

/// Reassemble a document from its frontmatter lines and body, guaranteeing exactly one trailing
/// newline — the oracle's `_reassemble`.
pub fn reassemble(fm_lines: &[String], body: &str) -> String {
    let mut out = String::from("---\n");
    out.push_str(&fm_lines.join("\n"));
    out.push_str("\n---\n");
    out.push_str(body);
    if !body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// True for a frontmatter list item line (`^\s*-\s+`).
fn is_list_item(line: &str) -> bool {
    let t = line.trim_start();
    match t.strip_prefix('-') {
        Some(rest) => rest.starts_with(|c: char| c.is_whitespace()),
        None => false,
    }
}

/// The `cites:` entries of a frontmatter block, read with the ORACLE's list semantics (a blank line
/// or any non-`- ` line, including a `#` comment, closes the list). See the module header for why
/// this cannot be [`super::parse_frontmatter`].
///
/// ONE deliberate divergence, in the reader's favour: `cites: []` declares NO cites. The oracle
/// stores that as the scalar string `"[]"` and then iterates it character by character, minting two
/// phantom edges whose refs are `[` and `]` — 18 of the 22 `dead` verdicts its corpus pass reported
/// on 2026-09-10, across the 9 documents that write the empty inline sequence. The bug is inert on
/// write (its `cites:` block matcher never fires on `cites: []`) but it inflates every count, and
/// reproducing it would eventually stamp `status: dead` onto a bracket. Refused.
fn read_cite_items(fm_lines: &[String]) -> Vec<String> {
    let mut items = Vec::new();
    let mut inside = false;
    for line in fm_lines {
        let stripped = line.trim_end();
        if stripped.is_empty() {
            inside = false;
            continue;
        }
        if inside && is_list_item(stripped) {
            let item = stripped.trim_start();
            let item = item[1..].trim();
            items.push(unquote(item));
            continue;
        }
        match split_key(stripped) {
            None => inside = false,
            Some((key, value)) => inside = key == "cites" && value.is_empty(),
        }
    }
    items
}

/// The oracle's `_KV_RE` (`^([a-zA-Z0-9_-]+)\s*:\s*(.*)$`) — anchored, so an indented line is not a key.
fn split_key(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(':')?;
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some((key.to_string(), value.trim().to_string()))
}

/// Replace the `cites:` list block with `items`. `quote` selects the minting style: `cite-gen`
/// quotes an envelope that needs it, `cites-migrate` does not — a live divergence between the two
/// oracle scripts that the corpus already carries, so both are reproduced rather than unified.
///
/// Only the contiguous `- ` run directly under `cites:` is dropped; everything else (including a
/// mid-list `#` comment and any lines below it) is carried through verbatim.
fn set_cites_block(fm_lines: &[String], items: &[String], quote: bool) -> (Vec<String>, bool) {
    let mut out = Vec::new();
    let mut i = 0;
    let mut replaced = false;
    while i < fm_lines.len() {
        if fm_lines[i].trim() == "cites:" {
            out.push("cites:".to_string());
            i += 1;
            while i < fm_lines.len() && is_list_item(&fm_lines[i]) {
                i += 1;
            }
            for item in items {
                let rendered = if quote {
                    yaml_cite_item(item)
                } else {
                    item.clone()
                };
                out.push(format!("  - {rendered}"));
            }
            replaced = true;
            continue;
        }
        out.push(fm_lines[i].clone());
        i += 1;
    }
    (out, replaced)
}

/// Insert an `id:` line after `title:` (or at the top of the block when there is no title).
fn insert_id(fm_lines: &[String], slug: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut done = false;
    for line in fm_lines {
        out.push(line.clone());
        if !done && line.trim_start().starts_with("title:") {
            out.push(format!("id: {slug}"));
            done = true;
        }
    }
    if !done {
        out.insert(0, format!("id: {slug}"));
    }
    out
}

/// Read a document's scalar frontmatter field through the crate reader (scalar semantics agree).
fn scalar(text: &str, key: &str) -> String {
    parse_frontmatter(text).get(key).unwrap_or("").to_string()
}

fn read_text(path: &Path) -> FlowResult<String> {
    std::fs::read_to_string(path).map_err(|source| FlowError::Read {
        path: path.to_path_buf(),
        source,
    })
}

/// Read a file with the oracle's `errors="replace"` tolerance — invalid UTF-8 becomes U+FFFD
/// rather than an error, so one undecodable byte cannot silence a corpus pass.
fn read_lossy(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

// ---------------------------------------------------------------------------
// Slugs
// ---------------------------------------------------------------------------

/// Lowercase, collapse every non-`[a-z0-9]` run to one `-`, trim the edges.
pub fn slugify(text: &str) -> String {
    let lowered = text.to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut in_bad = false;
    for c in lowered.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            in_bad = false;
        } else if !in_bad {
            out.push('-');
            in_bad = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Strip a `YYYY-MM-DD-` filename prefix.
fn strip_date_prefix(stem: &str) -> &str {
    let b = stem.as_bytes();
    if b.len() > 11
        && b[0..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
        && b[10] == b'-'
    {
        &stem[11..]
    } else {
        stem
    }
}

/// The title's first segment, split on `\s+[—–-]\s+` (em dash, en dash, hyphen).
fn title_head(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '—' || chars[i] == '–' || chars[i] == '-' {
            let before_ws = i > 0 && chars[i - 1].is_whitespace();
            let after_ws = i + 1 < chars.len() && chars[i + 1].is_whitespace();
            if before_ws && after_ws {
                let mut start = i;
                while start > 0 && chars[start - 1].is_whitespace() {
                    start -= 1;
                }
                return chars[..start].iter().collect();
            }
        }
        i += 1;
    }
    title.to_string()
}

/// A readable, stable slug for a document — the exact port of `cite_graph.derive_slug`.
pub fn derive_slug(path: &Path, title: &str) -> String {
    let raw_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let stem = slugify(strip_date_prefix(raw_stem));
    if CONTENT_AMBIGUOUS_STEMS.contains(&stem.as_str()) {
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .map(slugify)
            .unwrap_or_default();
        if !parent.is_empty() {
            return format!("{parent}-{stem}");
        }
    }
    if !stem.is_empty() && !GENERIC_STEMS.contains(&stem.as_str()) {
        return stem;
    }
    if !title.is_empty() {
        let t = slugify(&title_head(title));
        if !t.is_empty() {
            return t;
        }
    }
    if stem.is_empty() {
        slugify(raw_stem)
    } else {
        stem
    }
}

/// Collision guard: `candidate`, else `candidate-2`, `candidate-3`, …
pub fn allocate_slug(candidate: &str, taken: &BTreeSet<String>) -> String {
    if !taken.contains(candidate) {
        return candidate.to_string();
    }
    let mut i = 2;
    while taken.contains(&format!("{candidate}-{i}")) {
        i += 1;
    }
    format!("{candidate}-{i}")
}

// ---------------------------------------------------------------------------
// The corpus and the slug index
// ---------------------------------------------------------------------------

/// The doc-ROOT half of the graph: `genesis/docs` + `.claude/memory`.
fn doc_roots(root: &Path) -> Vec<PathBuf> {
    vec![root.join("genesis/docs"), root.join(".claude/memory")]
}

/// Every `*.md` under `dir`, sorted — the deterministic stand-in for `rglob`.
fn markdown_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// The repo's gospel `CLAUDE.md` paths, pruning vendored/build/dot directories.
pub fn gospel_claude_md_paths(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if path.is_dir() {
                if !name.starts_with('.') && !GOSPEL_EXCLUDE_DIRS.contains(&name) {
                    stack.push(path);
                }
            } else if name == "CLAUDE.md" {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// True iff `path` is a gospel `CLAUDE.md` inside the walk scope above.
pub fn is_gospel_claude_md(path: &Path, root: &Path) -> bool {
    if path.file_name().and_then(|n| n.to_str()) != Some("CLAUDE.md") {
        return false;
    }
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    let parts: Vec<_> = rel.components().collect();
    parts[..parts.len().saturating_sub(1)].iter().all(|c| {
        let s = c.as_os_str().to_string_lossy();
        !s.starts_with('.') && !GOSPEL_EXCLUDE_DIRS.contains(&s.as_ref())
    })
}

/// `slug -> absolute path` over every `id:`-declaring document under `roots`.
///
/// Later entries win (the oracle's `index[sid] = str(md)`), and the walk is SORTED so the outcome
/// is deterministic where the oracle's `rglob` order was not.
fn build_slug_index(roots: &[PathBuf]) -> BTreeMap<String, PathBuf> {
    let mut index = BTreeMap::new();
    for root in roots {
        for md in markdown_under(root) {
            if let Some(text) = read_lossy(&md) {
                let id = scalar(&text, "id");
                if !id.is_empty() {
                    index.insert(id, md);
                }
            }
        }
    }
    index
}

/// Add `id:`-declaring gospels. Existing entries win — doc-root declarations take precedence.
fn extend_index_with_gospels(index: &mut BTreeMap<String, PathBuf>, root: &Path) {
    for md in gospel_claude_md_paths(root) {
        let Some(text) = read_lossy(&md) else {
            continue;
        };
        let id = scalar(&text, "id");
        if !id.is_empty() {
            index.entry(id).or_insert(md);
        }
    }
}

/// The graph's full slug index: doc roots plus gospels.
fn slug_index(root: &Path) -> BTreeMap<String, PathBuf> {
    let mut index = build_slug_index(&doc_roots(root));
    extend_index_with_gospels(&mut index, root);
    index
}

/// Duplicate `id:` declarations WITHIN one precedence class. A doc-root document and a gospel
/// sharing a slug is not a collision — the doc root wins by declared precedence — but two doc-root
/// documents (or two gospels neither of which is shadowed) make every verdict on that slug
/// untrustworthy.
fn duplicate_slugs(root: &Path) -> BTreeMap<String, Vec<PathBuf>> {
    let mut seen: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for dir in doc_roots(root) {
        for md in markdown_under(&dir) {
            if let Some(text) = read_lossy(&md) {
                let id = scalar(&text, "id");
                if !id.is_empty() {
                    seen.entry(id).or_default().push(md);
                }
            }
        }
    }
    let doc_root_ids: BTreeSet<String> = seen.keys().cloned().collect();
    let mut gospels: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for md in gospel_claude_md_paths(root) {
        let Some(text) = read_lossy(&md) else {
            continue;
        };
        let id = scalar(&text, "id");
        if !id.is_empty() && !doc_root_ids.contains(&id) {
            gospels.entry(id).or_default().push(md);
        }
    }
    for (id, paths) in gospels {
        seen.entry(id).or_default().extend(paths);
    }
    seen.into_iter().filter(|(_, v)| v.len() > 1).collect()
}

/// The citing corpus: doc roots (minus index surfaces and state/kit trees) plus every gospel that
/// declares `id:` or `cites:`.
fn corpus(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in doc_roots(root) {
        for md in markdown_under(&dir) {
            let name = md.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let s = md.to_string_lossy().replace('\\', "/");
            if SKIP_NAMES.contains(&name) || s.contains("/_state/") || s.contains("/memory-kit/") {
                continue;
            }
            out.push(md);
        }
    }
    for g in gospel_claude_md_paths(root) {
        let Some(text) = read_lossy(&g) else { continue };
        if !scalar(&text, "id").is_empty() || text.contains("\ncites:") {
            out.push(g);
        }
    }
    out.sort();
    out.dedup();
    out
}

// ---------------------------------------------------------------------------
// Resolution helpers
// ---------------------------------------------------------------------------

/// Resolve a cite ref (slug or repo-relative path) to the target document, or `None`.
fn resolve_doc(root: &Path, reference: &str, index: &BTreeMap<String, PathBuf>) -> Option<PathBuf> {
    if let Some(p) = index.get(reference) {
        return Some(p.clone());
    }
    let candidate = if Path::new(reference).is_absolute() {
        PathBuf::from(reference)
    } else {
        root.join(reference)
    };
    if candidate.extension().and_then(|e| e.to_str()) == Some("md") && candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

/// A cite ref that names a file IN this repo (so its absence is a DEAD-CITE), rather than a URL,
/// a bare slug, or free prose. Conservative on purpose: a false DEAD-CITE fails a gate.
fn looks_like_repo_path(reference: &str) -> bool {
    if reference.contains("://")
        || reference.starts_with("http")
        || reference.starts_with("mailto:")
        || reference.starts_with('#')
    {
        return false;
    }
    if !reference.contains('/') || reference.trim() != reference || reference.contains(' ') {
        return false;
    }
    let p = Path::new(reference);
    p.extension().is_some() && !p.is_absolute()
}

/// Doc-root membership: under `genesis/docs` / `.claude/memory`, or a gospel `CLAUDE.md`.
fn is_doc_root(root: &Path, path: &Path) -> bool {
    let s = path.to_string_lossy().to_string();
    doc_roots(root)
        .iter()
        .any(|r| s.starts_with(&r.to_string_lossy().to_string()))
        || is_gospel_claude_md(path, root)
}

/// Repo-relative posix locator, falling back to the raw path outside the repo.
fn rel_locator(root: &Path, path: &Path) -> String {
    let canon_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let canon = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    match canon.strip_prefix(&canon_root) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    }
}

/// The declared short-form fingerprint of a document's canonical body.
fn fingerprint_of(path: &Path) -> Option<String> {
    let text = read_lossy(path)?;
    Some(BlobCid::compute_raw(canonical_body(&text).as_bytes()).short_fingerprint())
}

/// The first segment of a target's title (or its filename stem) — the migration-default description.
fn default_desc(path: &Path, text: &str) -> String {
    let title = scalar(text, "title");
    let raw = if title.is_empty() {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    } else {
        title
    };
    title_head(&raw).trim().to_string()
}

/// Stamp/refresh each envelope's tool-managed `path:` locator from the live resolution. A dead slug
/// KEEPS its last known path as a forensic breadcrumb; legacy path-cites are untouched (their ref
/// IS the path). Returns the number of cites changed.
fn materialize_paths(
    cites: &mut [Cite],
    index: &BTreeMap<String, PathBuf>,
    root: &Path,
    only: Option<&BTreeSet<String>>,
) -> usize {
    let mut changed = 0;
    for cite in cites.iter_mut() {
        if cite.legacy_path {
            continue;
        }
        if let Some(only) = only {
            if !only.contains(&cite.reference) {
                continue;
            }
        }
        let Some(target) = index.get(&cite.reference) else {
            continue;
        };
        let new = rel_locator(root, target);
        if cite.path != new {
            cite.path = new;
            changed += 1;
        }
    }
    changed
}

// ---------------------------------------------------------------------------
// Verdicts — one computation, two renderings
// ---------------------------------------------------------------------------

/// The five stamped verdicts. `ok` clears the field; the other four write the hint below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StampVerdict {
    Ok,
    Held,
    Stale,
    Remote,
    Dead,
}

impl StampVerdict {
    pub fn word(self) -> &'static str {
        match self {
            StampVerdict::Ok => "ok",
            StampVerdict::Held => "held",
            StampVerdict::Stale => "stale",
            StampVerdict::Remote => "remote",
            StampVerdict::Dead => "dead",
        }
    }

    /// The exact `status:` string written for this verdict. `ok` removes the field.
    pub fn hint(self) -> &'static str {
        match self {
            StampVerdict::Ok => "",
            StampVerdict::Held => "held — target sequestered (see cluster-state.yaml)",
            StampVerdict::Stale => "stale — target content moved on; re-verify",
            StampVerdict::Remote => "remote — resolvable on the substrate, absent locally",
            StampVerdict::Dead => "dead — target no longer resolves",
        }
    }
}

/// The health of one envelope.
///
/// Resolution, held-sequestration and unreadability are decided here; the DIGEST COMPARISON is
/// delegated to [`derive_verdict`] — the same function [`super::concerns`] uses — so the inline
/// `status:` and the concern view can never disagree about whether a seal still holds. A full-CID
/// token is normalized to the raw codec before comparison, because the oracle compares the sha2-256
/// digest and not the codec tag.
///
/// Returns `(verdict, target_unreadable)`.
fn stamp_verdict(
    root: &Path,
    cite: &Cite,
    index: &BTreeMap<String, PathBuf>,
) -> (StampVerdict, bool) {
    if cite.legacy_path {
        return (StampVerdict::Ok, false);
    }
    let Some(target) = index.get(&cite.reference) else {
        // Absent locally. Conservative: `dead` UNLESS a full-CID token gives positive
        // substrate-resolvable evidence.
        return if cite.fingerprint.starts_with("baf") {
            (StampVerdict::Remote, false)
        } else {
            (StampVerdict::Dead, false)
        };
    };
    if target
        .to_string_lossy()
        .replace('\\', "/")
        .contains("/held/")
    {
        return (StampVerdict::Held, false);
    }
    if cite.fingerprint.is_empty() {
        return (StampVerdict::Ok, false);
    }
    let Some(text) = read_lossy(target) else {
        // An I/O failure is not fingerprint-drift evidence.
        return (StampVerdict::Ok, true);
    };
    let current = BlobCid::compute_raw(canonical_body(&text).as_bytes());
    let seal = if cite.fingerprint.starts_with("baf") {
        match BlobCid::parse(&cite.fingerprint) {
            // Re-tag with the raw codec so equality is DIGEST equality, matching the oracle's
            // decode-and-compare rule rather than a string comparison of two renderings. A
            // dag-cbor-tagged token over the same body must not read as drift.
            Ok(parsed) => SealForm::Full(Cid::new_v1(RAW_CODEC, *parsed.as_cid().hash())),
            Err(_) => return (StampVerdict::Stale, false),
        }
    } else {
        SealForm::Short(cite.fingerprint.clone())
    };
    let edge = IndexedEdge {
        from: String::new(),
        to: rel_locator(root, target),
        desc: None,
        governor: Governor::CiteSeal,
        seal,
        held: None,
        plane: EdgePlane::Doc,
        target_exists: true,
    };
    match derive_verdict(&edge, Some(&current)) {
        Verdict::Ok => (StampVerdict::Ok, false),
        Verdict::Stale => (StampVerdict::Stale, false),
        // A cite-seal edge with a readable upstream cannot reach these arms; degrade honestly.
        _ => (StampVerdict::Stale, false),
    }
}

// ---------------------------------------------------------------------------
// Verb: assign-id
// ---------------------------------------------------------------------------

/// Ensure the document carries a collision-guarded `id:`. Returns whether one was written.
fn assign_id(root: &Path, doc: &Path) -> FlowResult<bool> {
    let text = read_text(doc)?;
    let Some((fm_lines, body)) = split_frontmatter(&text) else {
        return Ok(false);
    };
    if !scalar(&text, "id").is_empty() {
        return Ok(false);
    }
    let taken: BTreeSet<String> = slug_index(root).keys().cloned().collect();
    let slug = allocate_slug(&derive_slug(doc, &scalar(&text, "title")), &taken);
    std::fs::write(doc, reassemble(&insert_id(&fm_lines, &slug), &body))?;
    Ok(true)
}

// ---------------------------------------------------------------------------
// Verb: seal (assign-id → convert → verify)
// ---------------------------------------------------------------------------

/// Rewrite legacy DOC path-cites to envelopes and refresh every `path:` locator.
/// Returns `(converted, left_legacy, paths_stamped)`.
fn rewrite_into(root: &Path, doc: &Path) -> FlowResult<(usize, usize, usize)> {
    let text = read_text(doc)?;
    let Some((fm_lines, body)) = split_frontmatter(&text) else {
        return Ok((0, 0, 0));
    };
    let items = read_cite_items(&fm_lines);
    if items.is_empty() {
        return Ok((0, 0, 0));
    }
    let index = slug_index(root);
    let (mut converted, mut left) = (0, 0);
    let mut new_cites = Vec::new();
    for item in &items {
        let cite = parse_cite(item);
        if !cite.legacy_path {
            new_cites.push(cite);
            continue;
        }
        let target = resolve_doc(root, &cite.reference, &index);
        if let Some(target) = target.filter(|t| is_doc_root(root, t)) {
            if let Some(target_text) = read_lossy(&target) {
                let id = scalar(&target_text, "id");
                if !id.is_empty() {
                    new_cites.push(Cite {
                        reference: id,
                        legacy_path: false,
                        desc: default_desc(&target, &target_text),
                        fingerprint: fingerprint_of(&target).unwrap_or_default(),
                        status: String::new(),
                        path: rel_locator(root, &target),
                    });
                    converted += 1;
                    continue;
                }
            }
        }
        new_cites.push(cite);
        left += 1;
    }
    let stamped = materialize_paths(&mut new_cites, &index, root, None);
    let rendered: Vec<String> = new_cites.iter().map(serialize_cite).collect();
    let (out, ok) = set_cites_block(&fm_lines, &rendered, true);
    if ok && (converted > 0 || stamped > 0) {
        std::fs::write(doc, reassemble(&out, &body))?;
    }
    Ok((converted, left, stamped))
}

/// The dissolution gate: legacy cites that should be envelopes, dead targets, unresolvable slugs.
fn verify_doc(root: &Path, doc: &Path) -> FlowResult<Vec<String>> {
    let text = read_text(doc)?;
    let Some((fm_lines, _)) = split_frontmatter(&text) else {
        return Ok(Vec::new());
    };
    let index = slug_index(root);
    let mut problems = Vec::new();
    for item in read_cite_items(&fm_lines) {
        let cite = parse_cite(&item);
        if cite.legacy_path {
            match resolve_doc(root, &cite.reference, &index) {
                Some(target) => {
                    let has_id = read_lossy(&target)
                        .map(|t| !scalar(&t, "id").is_empty())
                        .unwrap_or(false);
                    if is_doc_root(root, &target) && has_id {
                        problems.push(format!(
                            "legacy doc cite (migrate to envelope): {}",
                            cite.reference
                        ));
                    }
                }
                None => {
                    if looks_like_repo_path(&cite.reference) && !root.join(&cite.reference).exists()
                    {
                        problems.push(format!("DEAD-CITE (target missing): {}", cite.reference));
                    }
                }
            }
        } else if !index.contains_key(&cite.reference) {
            problems.push(format!("unresolvable slug: {}", cite.reference));
        }
    }
    Ok(problems)
}

/// Envelope cites still carrying the title-default description — the progressive-discovery debt a
/// seal reports.
fn weak_desc_count(root: &Path, doc: &Path) -> FlowResult<usize> {
    let text = read_text(doc)?;
    let Some((fm_lines, _)) = split_frontmatter(&text) else {
        return Ok(0);
    };
    let index = slug_index(root);
    let mut weak = 0;
    for item in read_cite_items(&fm_lines) {
        let cite = parse_cite(&item);
        if cite.legacy_path {
            continue;
        }
        let Some(target) = resolve_doc(root, &cite.reference, &index) else {
            continue;
        };
        let Some(target_text) = read_lossy(&target) else {
            continue;
        };
        let title = default_desc(&target, &target_text);
        let d = cite.desc.trim();
        if d.is_empty() || d.to_lowercase() == title.to_lowercase() || d == cite.reference {
            weak += 1;
        }
    }
    Ok(weak)
}

#[derive(Debug, Serialize)]
pub struct SealOutcome {
    pub doc: String,
    pub id_assigned: bool,
    pub converted: usize,
    pub left_legacy: usize,
    pub paths_stamped: usize,
    pub problems: Vec<String>,
    pub weak_descriptions: usize,
}

fn seal_doc(root: &Path, doc: &Path) -> FlowResult<SealOutcome> {
    let id_assigned = assign_id(root, doc)?;
    let (converted, left_legacy, paths_stamped) = rewrite_into(root, doc)?;
    let problems = verify_doc(root, doc)?;
    let weak_descriptions = weak_desc_count(root, doc)?;
    Ok(SealOutcome {
        doc: rel_locator(root, doc),
        id_assigned,
        converted,
        left_legacy,
        paths_stamped,
        problems,
        weak_descriptions,
    })
}

impl SealOutcome {
    fn render(&self) {
        println!("epr flow cites seal {}:", self.doc);
        println!(
            "   id: {}  ·  cites: {} sealed to envelope, {} legacy-kept, {} path: locator(s) stamped",
            if self.id_assigned { "assigned" } else { "present" },
            self.converted,
            self.left_legacy,
            self.paths_stamped
        );
        if self.problems.is_empty() {
            println!("   ✅ gate: all cites content-addressed + resolvable");
        } else {
            println!(
                "   ❌ gate: {} unresolved — {}",
                self.problems.len(),
                self.problems
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }
        if self.weak_descriptions > 0 {
            println!(
                "   ✍ {} cite(s) on the title-default desc — author relationship hints \
                 (epr flow cites describe) for progressive discovery",
                self.weak_descriptions
            );
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct SealAllOutcome {
    pub sealed: usize,
    pub converted: usize,
    pub weak_descriptions: usize,
    pub gate_failures: usize,
}

/// A legacy path-cite pointing at a doc-root `.md` that HAS an `id:` — a sealable candidate.
fn legacy_doc_cite_with_id(root: &Path, cite: &Cite) -> bool {
    if !cite.legacy_path || !cite.reference.ends_with(".md") {
        return false;
    }
    let target = if Path::new(&cite.reference).is_absolute() {
        PathBuf::from(&cite.reference)
    } else {
        root.join(&cite.reference)
    };
    target.is_file()
        && is_doc_root(root, &target)
        && read_lossy(&target)
            .map(|t| !scalar(&t, "id").is_empty())
            .unwrap_or(false)
}

/// End-of-sprint sweep: seal every graph member carrying un-sealed cite debt.
fn seal_all(root: &Path) -> FlowResult<SealAllOutcome> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for dir in doc_roots(root) {
        for md in markdown_under(&dir) {
            let name = md.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "CLAUDE.md" | "README.md" | "MEMORY.md") {
                continue;
            }
            candidates.push(md);
        }
    }
    for g in gospel_claude_md_paths(root) {
        let Some(text) = read_lossy(&g) else { continue };
        if let Some((fm_lines, _)) = split_frontmatter(&text) {
            if !read_cite_items(&fm_lines).is_empty() {
                candidates.push(g);
            }
        }
    }
    let mut out = SealAllOutcome::default();
    for doc in candidates {
        let Some(text) = read_lossy(&doc) else {
            continue;
        };
        let Some((fm_lines, _)) = split_frontmatter(&text) else {
            continue;
        };
        let cites: Vec<Cite> = read_cite_items(&fm_lines)
            .iter()
            .map(|c| parse_cite(c))
            .collect();
        if cites.is_empty() {
            continue;
        }
        let needs = scalar(&text, "id").is_empty()
            || cites.iter().any(|c| legacy_doc_cite_with_id(root, c));
        if !needs {
            continue;
        }
        assign_id(root, &doc)?;
        let (converted, _, _) = rewrite_into(root, &doc)?;
        out.converted += converted;
        if !verify_doc(root, &doc)?.is_empty() {
            out.gate_failures += 1;
        }
        out.weak_descriptions += weak_desc_count(root, &doc)?;
        out.sealed += 1;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Verb: describe
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DescribeOutcome {
    pub doc: String,
    pub set: usize,
}

/// Set authored one-sentence descriptions on a document's envelope cites, preserving ref,
/// fingerprint, status and path. Idempotent; only touches cites whose ref is in the map.
fn describe(
    root: &Path,
    doc: &Path,
    descs: &BTreeMap<String, String>,
) -> FlowResult<DescribeOutcome> {
    let text = read_text(doc)?;
    let Some((fm_lines, body)) = split_frontmatter(&text) else {
        return Err(FlowError::InvalidArguments(format!(
            "cites describe: no frontmatter: {}",
            doc.display()
        )));
    };
    let mut cites: Vec<Cite> = read_cite_items(&fm_lines)
        .iter()
        .map(|c| parse_cite(c))
        .collect();
    let mut set = 0;
    for cite in cites.iter_mut() {
        if cite.legacy_path {
            continue;
        }
        let Some(new) = descs.get(&cite.reference) else {
            continue;
        };
        // `|` is the envelope delimiter — never inside a description.
        let new = new.trim().replace('|', "/");
        if new.is_empty() || cite.desc == new {
            continue;
        }
        cite.desc = new;
        set += 1;
    }
    if set > 0 {
        let rendered: Vec<String> = cites.iter().map(serialize_cite).collect();
        let (out, ok) = set_cites_block(&fm_lines, &rendered, true);
        if ok {
            std::fs::write(doc, reassemble(&out, &body))?;
        }
    }
    Ok(DescribeOutcome {
        doc: rel_locator(root, doc),
        set,
    })
}

// ---------------------------------------------------------------------------
// Verb: stamp
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize)]
pub struct StampOutcome {
    pub docs: usize,
    pub stamped: usize,
    pub by_verdict: BTreeMap<String, usize>,
    pub cleared: usize,
    pub paths_refreshed: usize,
    pub touched: usize,
    pub unreadable: usize,
    pub refusals: Vec<String>,
    pub wrote: bool,
}

/// Fan each cited target's verdict back onto every citing edge as inline `status:` / `path:`
/// segments, so link health travels WITH the doc in git rather than living in a report.
///
/// Default is a dry run; `write` is the explicit act. Idempotent: a second pass over a stamped tree
/// reports zero of everything and leaves every byte identical.
fn stamp(root: &Path, targets: &[PathBuf], named: bool, write: bool) -> FlowResult<StampOutcome> {
    // Refusal 5 — a colliding slug index makes every verdict untrustworthy, so the WHOLE run is
    // refused by name rather than resolving the ambiguity silently.
    let dups = duplicate_slugs(root);
    if !dups.is_empty() {
        let named_dups: Vec<String> = dups
            .iter()
            .map(|(id, paths)| {
                format!(
                    "{id} → {}",
                    paths
                        .iter()
                        .map(|p| rel_locator(root, p))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect();
        return Err(FlowError::InvalidArguments(format!(
            "cites stamp: refused — {} duplicate id: declaration(s) make the slug index ambiguous: {}",
            dups.len(),
            named_dups.join("; ")
        )));
    }
    let mut index = build_slug_index(&[
        root.join("genesis/docs"),
        root.join(".claude/memory"),
        root.join("genesis/docs/superpowers/held"),
    ]);
    extend_index_with_gospels(&mut index, root);

    let mut out = StampOutcome {
        docs: targets.len(),
        wrote: write,
        ..Default::default()
    };
    for doc in targets {
        let Some(text) = read_lossy(doc) else {
            out.refusals
                .push(format!("unreadable: {}", rel_locator(root, doc)));
            continue;
        };
        let Some((fm_lines, body)) = split_frontmatter(&text) else {
            // Refusal 1 — never synthesize a frontmatter block. A NAMED document is refused; in a
            // corpus sweep a frontmatter-less file is simply not a citing edge and is counted, not
            // failed, because most repository markdown carries no frontmatter at all.
            if named {
                out.refusals
                    .push(format!("no frontmatter: {}", rel_locator(root, doc)));
            }
            continue;
        };
        let items = read_cite_items(&fm_lines);
        if items.is_empty() {
            continue; // Refusal 2 — no `cites:` key is the common case, not an error.
        }
        let mut cites: Vec<Cite> = items.iter().map(|c| parse_cite(c)).collect();
        let mut changed = false;
        for cite in cites.iter_mut() {
            if cite.legacy_path {
                continue; // a legacy cite's ref IS its path; it carries no status
            }
            let (verdict, unreadable) = stamp_verdict(root, cite, &index);
            if unreadable {
                out.unreadable += 1;
            }
            let want = verdict.hint();
            if cite.status != want {
                if want.is_empty() {
                    out.cleared += 1;
                } else {
                    out.stamped += 1;
                    *out.by_verdict
                        .entry(verdict.word().to_string())
                        .or_insert(0) += 1;
                }
                cite.status = want.to_string();
                changed = true;
            }
        }
        let refreshed = materialize_paths(&mut cites, &index, root, None);
        if refreshed > 0 {
            out.paths_refreshed += refreshed;
            changed = true;
        }
        if !changed {
            continue;
        }
        out.touched += 1;
        if write {
            let rendered: Vec<String> = cites.iter().map(serialize_cite).collect();
            let (lines, ok) = set_cites_block(&fm_lines, &rendered, true);
            if !ok {
                // Refusal 3 — the whole reassembled file or none of it; never a partial rewrite.
                out.refusals.push(format!(
                    "cites: block not rewritable: {}",
                    rel_locator(root, doc)
                ));
                continue;
            }
            std::fs::write(doc, reassemble(&lines, &body))?;
        }
    }
    Ok(out)
}

impl StampOutcome {
    fn render(&self) {
        let verb = if self.wrote { "APPLIED" } else { "DRY-RUN" };
        println!("epr flow cites stamp [{verb}]: {} docs", self.docs);
        let by = if self.by_verdict.is_empty() {
            String::new()
        } else {
            format!(
                "{{{}}}",
                self.by_verdict
                    .iter()
                    .map(|(k, v)| format!("'{k}': {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        println!(
            "  status stamped: {} {}  |  cleared (recovered/healthy): {}  |  \
             path: stamped/refreshed: {}  |  docs touched: {}",
            self.stamped, by, self.cleared, self.paths_refreshed, self.touched
        );
        if self.unreadable > 0 {
            println!("  unreadable: {}", self.unreadable);
        }
        for refusal in &self.refusals {
            println!("  refused: {refusal}");
        }
        if !self.wrote && self.touched > 0 {
            println!("  (re-run with --write to write the self-describing status:/path: fields)");
        }
    }
}

// ---------------------------------------------------------------------------
// Verb: migrate
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize)]
pub struct MigrateOutcome {
    pub docs: usize,
    pub assigned: usize,
    pub already_had_id: usize,
    pub converted: usize,
    pub touched: usize,
    pub applied: bool,
}

/// The corpus-wide two-pass sweep: assign every document a collision-guarded `id:`, then convert
/// legacy DOC cites to envelopes. Deterministic and idempotent — a second run reports zero.
///
/// The minted envelopes are NOT YAML-quoted here. That is not an oversight: `cites-migrate.py`'s
/// own `_set_cites` writes `f"  - {it}"` while `cite-gen --into` quotes, and the corpus carries
/// both forms. Reproducing the divergence keeps a migrate pass byte-comparable with the oracle;
/// a later `seal` on the same document is what quotes it.
fn migrate(root: &Path, apply: bool) -> FlowResult<MigrateOutcome> {
    let docs = corpus(root);
    let mut out = MigrateOutcome {
        docs: docs.len(),
        applied: apply,
        ..Default::default()
    };

    // PASS 1 — assign `id:`, tracking collisions incrementally so newly-derived slugs cannot clash.
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut id_by_path: BTreeMap<PathBuf, String> = BTreeMap::new();
    for doc in &docs {
        if let Some(text) = read_lossy(doc) {
            let id = scalar(&text, "id");
            if !id.is_empty() {
                taken.insert(id.clone());
                id_by_path.insert(doc.clone(), id);
            }
        }
    }
    out.already_had_id = id_by_path.len();
    for doc in &docs {
        if id_by_path.contains_key(doc) {
            continue;
        }
        let Some(text) = read_lossy(doc) else {
            continue;
        };
        let Some((fm_lines, body)) = split_frontmatter(&text) else {
            continue; // no frontmatter — nothing to declare an id in
        };
        let slug = allocate_slug(&derive_slug(doc, &scalar(&text, "title")), &taken);
        taken.insert(slug.clone());
        id_by_path.insert(doc.clone(), slug.clone());
        if apply {
            std::fs::write(doc, reassemble(&insert_id(&fm_lines, &slug), &body))?;
        }
        out.assigned += 1;
    }

    // PASS 2 — legacy DOC cites → envelopes, resolving each target path to the id pass 1 minted.
    for doc in &docs {
        let Some(text) = read_lossy(doc) else {
            continue;
        };
        let Some((fm_lines, body)) = split_frontmatter(&text) else {
            continue;
        };
        let items = read_cite_items(&fm_lines);
        if items.is_empty() {
            continue;
        }
        let mut rendered = Vec::new();
        let mut did = 0;
        for item in &items {
            let cite = parse_cite(item);
            if !cite.legacy_path {
                rendered.push(serialize_cite(&cite));
                continue;
            }
            let reference = cite.reference.trim_matches('/').to_string();
            let target = if Path::new(&cite.reference).is_absolute() {
                PathBuf::from(&cite.reference)
            } else {
                root.join(&reference)
            };
            let is_doc = target.extension().and_then(|e| e.to_str()) == Some("md")
                && target.is_file()
                && is_doc_root(root, &target);
            match (is_doc, id_by_path.get(&target)) {
                (true, Some(id)) => {
                    let target_text = read_lossy(&target).unwrap_or_default();
                    let desc: String = default_desc(&target, &target_text)
                        .chars()
                        .take(90)
                        .collect();
                    rendered.push(serialize_cite(&Cite {
                        reference: id.clone(),
                        legacy_path: false,
                        desc,
                        fingerprint: fingerprint_of(&target).unwrap_or_default(),
                        status: String::new(),
                        path: rel_locator(root, &target),
                    }));
                    did += 1;
                }
                _ => rendered.push(serialize_cite(&cite)),
            }
        }
        if did > 0 {
            out.converted += did;
            out.touched += 1;
            if apply {
                let (lines, ok) = set_cites_block(&fm_lines, &rendered, false);
                if ok {
                    std::fs::write(doc, reassemble(&lines, &body))?;
                }
            }
        }
    }
    Ok(out)
}

impl MigrateOutcome {
    fn render(&self) {
        let verb = if self.applied { "APPLIED" } else { "DRY-RUN" };
        println!("epr flow cites migrate [{verb}]: {} docs", self.docs);
        println!(
            "  pass 1: {} id: slugs assigned ({} already had one)",
            self.assigned, self.already_had_id
        );
        println!(
            "  pass 2: {} cites converted to envelopes across {} docs",
            self.converted, self.touched
        );
        if !self.applied {
            println!("  (re-run with --apply to write)");
        }
    }
}

// ---------------------------------------------------------------------------
// Verb: refresh
// ---------------------------------------------------------------------------

/// DELIBERATE post-re-verification blessing: re-fingerprint every envelope whose slug resolves,
/// refresh its locator, and recompute its status hint. The stale `status:` is the QUEUE; this is
/// the dequeue — run it ONLY after re-verifying that the citing doc's claims still hold against the
/// moved-on target. `only` limits the blessing to named slugs, because verification is EDGE-granular.
fn refresh(root: &Path, doc: &Path, only: Option<&BTreeSet<String>>) -> FlowResult<usize> {
    let text = read_text(doc)?;
    let Some((fm_lines, body)) = split_frontmatter(&text) else {
        return Ok(0);
    };
    let items = read_cite_items(&fm_lines);
    if items.is_empty() {
        return Ok(0);
    }
    let index = slug_index(root);
    let mut cites: Vec<Cite> = items.iter().map(|c| parse_cite(c)).collect();
    let mut changed = 0;
    for cite in cites.iter_mut() {
        if cite.legacy_path {
            continue;
        }
        if let Some(only) = only {
            if !only.contains(&cite.reference) {
                continue;
            }
        }
        let Some(target) = index.get(&cite.reference).cloned() else {
            continue;
        };
        let Some(fp) = fingerprint_of(&target) else {
            continue;
        };
        if fp != cite.fingerprint {
            cite.fingerprint = fp;
            changed += 1;
        }
        let (verdict, _) = stamp_verdict(root, cite, &index);
        let want = verdict.hint();
        if cite.status != want {
            cite.status = want.to_string();
            changed += 1;
        }
    }
    changed += materialize_paths(&mut cites, &index, root, only);
    if changed > 0 {
        let rendered: Vec<String> = cites.iter().map(serialize_cite).collect();
        let (out, ok) = set_cites_block(&fm_lines, &rendered, true);
        if ok {
            std::fs::write(doc, reassemble(&out, &body))?;
        }
    }
    Ok(changed)
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

pub fn usage() -> &'static str {
    "usage: epr flow cites <verb> [<doc>…] [--json] [--root DIR]\n  \
     seal <doc>            assign id: if absent, convert legacy doc path-cites to envelopes, \
     refresh path: locators, then run the dissolution gate (exit 1 on an unresolved cite)\n  \
     seal --all            end-of-sprint sweep: seal every graph member carrying un-sealed debt\n  \
     assign-id <doc>       ensure a collision-guarded id: slug (idempotent)\n  \
     describe <doc> --slug <s> --desc <text> [--slug … --desc …]\n  \
     describe <doc> '<json {ref: desc}>'\n  \
     verify <doc>          the dissolution gate alone; exit 1 if any cite is legacy-migratable, \
     dead or unresolvable\n  \
     refresh <doc> [<slug>…]  DELIBERATE stale-dequeue after re-verifying the claim\n  \
     stamp [<doc>…|--all] [--write]   fan each target's verdict onto every citing edge as inline \
     status:/path:; dry run unless --write\n  \
     migrate [--dry-run|--apply]      corpus sweep: assign id: + convert legacy doc cites"
}

fn resolve_doc_arg(root: &Path, value: &str) -> FlowResult<PathBuf> {
    let p = Path::new(value);
    let doc = if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    };
    if !doc.is_file() {
        return Err(FlowError::InvalidArguments(format!("no such doc: {value}")));
    }
    Ok(doc)
}

/// Entry point for `epr flow cites …`. `root` and `json` are already parsed by the caller.
pub fn run(root: &Path, json: bool, rest: &[String]) -> FlowResult<ExitCode> {
    let Some(verb) = rest.first().map(String::as_str) else {
        return Err(FlowError::InvalidArguments(usage().to_string()));
    };
    if verb == "--help" || verb == "-h" {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    }
    let args = &rest[1..];
    let flag = |name: &str| args.iter().any(|a| a == name);
    let positionals: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();

    match verb {
        "seal" if flag("--all") => {
            let outcome = seal_all(root)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                println!(
                    "epr flow cites seal --all: {} doc(s) sealed, {} cite(s) → envelope",
                    outcome.sealed, outcome.converted
                );
                if outcome.gate_failures > 0 {
                    println!(
                        "   ❌ {} doc(s) still have unresolved cites (DEAD-CITE — run verify per doc)",
                        outcome.gate_failures
                    );
                }
                if outcome.weak_descriptions > 0 {
                    println!(
                        "   ✍ {} cite(s) on the title-default desc across the sweep — author \
                         relationship hints (epr flow cites describe)",
                        outcome.weak_descriptions
                    );
                }
                if outcome.sealed == 0 {
                    println!("   ✅ corpus already born-linked (no un-sealed debt)");
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        "seal" => {
            let target = positionals
                .first()
                .ok_or_else(|| FlowError::InvalidArguments("seal needs a <doc>".into()))?;
            let outcome = seal_doc(root, &resolve_doc_arg(root, target)?)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                outcome.render();
            }
            Ok(if outcome.problems.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        "assign-id" => {
            let target = positionals
                .first()
                .ok_or_else(|| FlowError::InvalidArguments("assign-id needs a <doc>".into()))?;
            let doc = resolve_doc_arg(root, target)?;
            let wrote = assign_id(root, &doc)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "doc": rel_locator(root, &doc), "assigned": wrote })
                );
            } else {
                println!(
                    "epr flow cites assign-id {target}: {}",
                    if wrote {
                        "assigned"
                    } else {
                        "already had id: (no-op)"
                    }
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        "describe" => {
            let target = positionals
                .first()
                .ok_or_else(|| FlowError::InvalidArguments("describe needs a <doc>".into()))?;
            let doc = resolve_doc_arg(root, target)?;
            let descs = parse_describe_args(args, &positionals)?;
            if descs.is_empty() {
                return Err(FlowError::InvalidArguments(
                    "describe needs --slug <s> --desc <text> (repeatable) or a positional JSON map"
                        .into(),
                ));
            }
            let outcome = describe(root, &doc, &descs)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                println!(
                    "epr flow cites describe {}: {} description(s) set",
                    outcome.doc, outcome.set
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        "verify" => {
            let target = positionals
                .first()
                .ok_or_else(|| FlowError::InvalidArguments("verify needs a <doc>".into()))?;
            let doc = resolve_doc_arg(root, target)?;
            let problems = verify_doc(root, &doc)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "doc": rel_locator(root, &doc), "problems": problems })
                );
            } else if problems.is_empty() {
                println!(
                    "epr flow cites verify {target}: ✅ all cites content-addressed + resolvable"
                );
            } else {
                println!(
                    "epr flow cites verify {target}: ❌ {} problem(s):",
                    problems.len()
                );
                for p in &problems {
                    println!("   - {p}");
                }
            }
            Ok(if problems.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        "refresh" => {
            let target = positionals
                .first()
                .ok_or_else(|| FlowError::InvalidArguments("refresh needs a <doc>".into()))?;
            let doc = resolve_doc_arg(root, target)?;
            let only: BTreeSet<String> = positionals[1..].iter().map(|s| (*s).clone()).collect();
            let scope = if only.is_empty() { None } else { Some(&only) };
            let changed = refresh(root, &doc, scope)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "doc": rel_locator(root, &doc), "changed": changed })
                );
            } else if changed > 0 {
                println!(
                    "epr flow cites refresh {target}: {changed} field(s) re-blessed \
                     (fingerprint/status/path)"
                );
            } else {
                println!("epr flow cites refresh {target}: already current (no-op)");
            }
            Ok(ExitCode::SUCCESS)
        }
        "stamp" => {
            let all = flag("--all") || positionals.is_empty();
            let write = flag("--write") || flag("--apply");
            let targets: Vec<PathBuf> = if all {
                corpus(root)
            } else {
                positionals
                    .iter()
                    .map(|p| resolve_doc_arg(root, p))
                    .collect::<FlowResult<_>>()?
            };
            let outcome = stamp(root, &targets, !all, write)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                outcome.render();
            }
            Ok(if outcome.refusals.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        "migrate" => {
            let apply = flag("--apply");
            let outcome = migrate(root, apply)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                outcome.render();
            }
            Ok(ExitCode::SUCCESS)
        }
        other => Err(FlowError::InvalidArguments(format!(
            "unknown cites verb `{other}`\n{}",
            usage()
        ))),
    }
}

/// `--slug <s> --desc <text>` pairs, or a positional JSON `{ref: desc}` map.
fn parse_describe_args(
    args: &[String],
    positionals: &[&String],
) -> FlowResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let mut pending: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--slug" => {
                pending = Some(
                    args.get(i + 1)
                        .ok_or_else(|| FlowError::InvalidArguments("--slug needs a value".into()))?
                        .clone(),
                );
                i += 2;
            }
            "--desc" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| FlowError::InvalidArguments("--desc needs a value".into()))?;
                let slug = pending.take().ok_or_else(|| {
                    FlowError::InvalidArguments("--desc must follow a --slug".into())
                })?;
                out.insert(slug, value.clone());
                i += 2;
            }
            _ => i += 1,
        }
    }
    if pending.is_some() {
        return Err(FlowError::InvalidArguments(
            "--slug given without a following --desc".into(),
        ));
    }
    if out.is_empty() {
        // Drop-in compatibility with the oracle's `cite-describe.py <doc> '<json>'` shape.
        if let Some(raw) = positionals.get(1) {
            let parsed: BTreeMap<String, String> = serde_json::from_str(raw)
                .map_err(|e| FlowError::InvalidArguments(format!("bad json: {e}")))?;
            return Ok(parsed);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_comment_is_stripped_at_the_whitespace_run() {
        assert_eq!(strip_inline_comment("a/b.py   # note here"), "a/b.py");
        assert_eq!(strip_inline_comment("a/b.py #"), "a/b.py");
        // A `#` with no leading whitespace, or glued to the next word, is content — not a comment.
        assert_eq!(strip_inline_comment("slug#anchor"), "slug#anchor");
        assert_eq!(strip_inline_comment("slug #anchor"), "slug #anchor");
    }

    #[test]
    fn envelope_round_trips_through_parse_and_serialize() {
        let raw = "my-slug | the desc | sha256:1234567890abcdef | status: stale — moved on | path: docs/x.md";
        let cite = parse_cite(raw);
        assert_eq!(cite.reference, "my-slug");
        assert_eq!(cite.desc, "the desc");
        assert_eq!(cite.fingerprint, "sha256:1234567890abcdef");
        assert_eq!(cite.status, "stale — moved on");
        assert_eq!(cite.path, "docs/x.md");
        assert_eq!(serialize_cite(&cite), raw);
    }

    #[test]
    fn escaped_quotes_survive_the_reader_and_the_minter() {
        // 9 live envelopes carry `\"`; the crate reader would leave the backslash and the minter
        // would double it on every pass.
        let item = "\"a-slug | the \\\"quoted\\\" hint | sha256:0000000000000000\"";
        let cite = parse_cite(item);
        assert_eq!(cite.desc, "the \"quoted\" hint");
        let minted = yaml_cite_item(&serialize_cite(&cite));
        assert_eq!(parse_cite(&minted).desc, cite.desc);
    }

    #[test]
    fn a_comment_closes_the_cites_list_for_the_reader_and_the_writer_alike() {
        let fm: Vec<String> = vec![
            "title: T".into(),
            "cites:".into(),
            "  - a | x | sha256:0000000000000000".into(),
            "  # legacy path cites below".into(),
            "  - some/path.md".into(),
        ];
        // Reader sees ONE entry (the oracle's semantics) …
        assert_eq!(read_cite_items(&fm).len(), 1);
        // … and the writer drops exactly the one it saw, carrying the comment and the rest through.
        let (out, ok) = set_cites_block(&fm, &["a | x | sha256:0000000000000000".into()], true);
        assert!(ok);
        assert_eq!(out.len(), 5);
        assert_eq!(out[3], "  # legacy path cites below");
        assert_eq!(out[4], "  - some/path.md");
    }

    #[test]
    fn legacy_path_detection_matches_the_oracle() {
        assert!(parse_cite("genesis/docs/x.md").legacy_path);
        assert!(parse_cite("codegen-ts.mjs").legacy_path);
        assert!(!parse_cite("a-bare-slug").legacy_path);
    }

    #[test]
    fn slug_derivation_strips_dates_and_falls_back_to_the_title() {
        assert_eq!(
            derive_slug(
                Path::new("g/2026-06-02-scope-tree-reconciler-design.md"),
                ""
            ),
            "scope-tree-reconciler-design"
        );
        // A generic stem falls through to the title's first segment.
        assert_eq!(
            derive_slug(Path::new("a/b/CLAUDE.md"), "Qahal Pillar — gospel"),
            "qahal-pillar"
        );
        // A content-ambiguous stem takes its parent directory as a prefix.
        assert_eq!(
            derive_slug(Path::new("governance/epic.md"), ""),
            "governance-epic"
        );
    }

    #[test]
    fn allocate_slug_guards_collisions() {
        let taken: BTreeSet<String> = ["a".into(), "a-2".into()].into_iter().collect();
        assert_eq!(allocate_slug("a", &taken), "a-3");
        assert_eq!(allocate_slug("b", &taken), "b");
    }

    #[test]
    fn reassemble_guarantees_exactly_one_trailing_newline() {
        let fm = vec!["title: T".to_string()];
        assert_eq!(reassemble(&fm, "body"), "---\ntitle: T\n---\nbody\n");
        assert_eq!(reassemble(&fm, "body\n"), "---\ntitle: T\n---\nbody\n");
    }

    #[test]
    fn empty_description_is_dropped_but_every_other_segment_is_kept() {
        let cite = Cite {
            reference: "s".into(),
            legacy_path: false,
            desc: String::new(),
            fingerprint: "sha256:0000000000000000".into(),
            status: String::new(),
            path: "a/b.md".into(),
        };
        assert_eq!(
            serialize_cite(&cite),
            "s | sha256:0000000000000000 | path: a/b.md"
        );
    }

    #[test]
    fn only_a_colon_space_envelope_is_quoted() {
        assert_eq!(yaml_cite_item("a | b | sha256:00"), "a | b | sha256:00");
        assert_eq!(
            yaml_cite_item("a | b | path: x.md"),
            "\"a | b | path: x.md\""
        );
    }
}
