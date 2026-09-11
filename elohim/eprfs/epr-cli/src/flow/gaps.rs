//! Checkbox-station extraction — the native port of `decompose.py`'s `checkbox-tasks` method.
//!
//! A spec or plan's `- [ ]` / `- [x]` stations ARE its gap list. The memory kit extracted them with
//! `decompose.py` and parked the result in `.claude/memory-kit/gap-items/<slug>.json`, which the
//! projector then read back. That indirection is the thing station two removes: the doc is the
//! source, so `epr flow project` reads the doc.
//!
//! **CHECKED ≠ VERIFIED.** A ticked box is a CLAIM awaiting the verification gate, never evidence of
//! done — which is why `[x]` mints `CLAIMED` rather than a fulfilment. The iroh gates proved the
//! distinction the expensive way.
//!
//! Extraction is heading-agnostic on purpose: every checkbox in the document is a station, wherever
//! it sits. The `## Delivery stations` heading this repository's plans use is a convention, not a
//! parser input — a plan that names its stations under a different heading still decomposes, and a
//! plan that grows a second station list does not silently lose half of it. The requirement-bullet
//! FALLBACK is the arm that reads headings, and only when a doc has no checkboxes at all.
//!
//! Regex-free by construction. Every matcher below is a hand-written scan against the exact Python
//! pattern quoted above it, because adding a regex dependency to buy back six patterns would put a
//! new crate in the resolution path of a tool the SessionStart hooks shell out to.

use std::path::Path;

use super::env_scope;

/// The gap-count above which a doc is a runaway — `decompose.py`'s `THRESHOLD`. A doc past it should
/// be SPLIT, never silently truncated: the count is reported, every item is kept.
pub const THRESHOLD: usize = 40;

/// Which extraction arm produced the items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `- [ ]` / `- [x]` stations anywhere in the doc.
    CheckboxTasks,
    /// Bullets under a requirement-style heading — the fallback when there are no checkboxes.
    RequirementBullets,
    /// No machine-extractable structure. Honest absence: this doc needs agent decomposition.
    None,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::CheckboxTasks => "checkbox-tasks",
            Method::RequirementBullets => "requirement-bullets",
            Method::None => "none",
        }
    }
}

/// One extracted station.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    /// `<parent-dir>__<stem>#<n>` — the same id the kit minted, so a native id and a gap-items id
    /// name the same station.
    pub id: String,
    pub text: String,
    /// 1-based source line, the `cites:` locator's `#L<n>`.
    pub line: usize,
    /// `OPEN` (unticked) or `CLAIMED` (ticked — a claim awaiting verification).
    pub state: GapState,
    /// Per-gap `@requires:<cap>` overrides. Empty means "inherit the doc default".
    pub requires_env: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapState {
    Open,
    Claimed,
}

impl GapState {
    pub fn as_str(self) -> &'static str {
        match self {
            GapState::Open => "OPEN",
            GapState::Claimed => "CLAIMED",
        }
    }
}

/// One document's decomposition.
#[derive(Debug, Clone)]
pub struct Decomposition {
    pub slug: String,
    pub method: Method,
    pub items: Vec<Gap>,
    /// The doc-level `requires_env` — the inheritance default for every gap.
    pub doc_requires_env: Vec<String>,
}

impl Decomposition {
    pub fn open(&self) -> usize {
        self.items
            .iter()
            .filter(|g| g.state == GapState::Open)
            .count()
    }

    pub fn claimed(&self) -> usize {
        self.items.len() - self.open()
    }

    /// Whether the doc is past the runaway threshold — reported, never acted on.
    pub fn runaway(&self) -> bool {
        self.items.len() > THRESHOLD
    }
}

/// `<parent-dir-name>__<file-stem>` — surface-prefixed so two docs with the same stem in different
/// directories (`plans/CLAUDE.md` vs `specs/CLAUDE.md`) do not collide on one id namespace.
pub fn slug_for(path: &Path) -> String {
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let stem = path
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    format!("{parent}__{stem}")
}

/// Decompose one document's text. `slug` is normally [`slug_for`] of its path.
pub fn decompose(slug: &str, text: &str, doc_requires_env: Vec<String>) -> Decomposition {
    let mut items = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    // 1) checkbox tasks — the primary arm, heading-agnostic.
    for (index, line) in text.lines().enumerate() {
        let Some((ticked, body)) = checkbox(line) else {
            continue;
        };
        push_item(
            &mut items,
            &mut seen,
            body,
            line,
            index + 1,
            if ticked {
                GapState::Claimed
            } else {
                GapState::Open
            },
            MetaOrder::BeforeTagStrip,
        );
    }
    let mut method = Method::CheckboxTasks;

    // 2) fallback: requirement bullets under a requirement-style heading.
    if items.is_empty() {
        method = Method::RequirementBullets;
        let mut in_req = false;
        for (index, line) in text.lines().enumerate() {
            if let Some(heading) = heading(line) {
                in_req = requirement_heading(heading);
                continue;
            }
            if !in_req {
                continue;
            }
            let Some(body) = bullet(line) else { continue };
            push_item(
                &mut items,
                &mut seen,
                body,
                line,
                index + 1,
                GapState::Open,
                MetaOrder::AfterTagStrip,
            );
        }
    }
    if items.is_empty() {
        method = Method::None;
    }

    for (n, item) in items.iter_mut().enumerate() {
        item.id = format!("{slug}#{}", n + 1);
    }

    Decomposition {
        slug: slug.to_string(),
        method,
        items,
        doc_requires_env,
    }
}

/// Where the meta filter sits relative to the `@requires:` tag strip.
///
/// The two extraction arms differ, and the difference is the kit's, not a tidy-up opportunity. The
/// checkbox arm tests META on the text WITH its tag still attached; the bullet arm strips the tag
/// first and tests what remains. It matters exactly when a tag's own characters would trip a meta
/// phrase, and transcribing one ordering onto both arms would silently admit or drop items the kit
/// does the opposite with. Named as an enum rather than a bool so a reader learns which is which
/// from the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetaOrder {
    /// `decompose.py` checkbox arm: `clean` → META → strip tags.
    BeforeTagStrip,
    /// `decompose.py` requirement-bullet arm: `clean` → strip tags → META.
    AfterTagStrip,
}

/// Clean → (META and tag strip, in the arm's order) → dedupe → push.
#[allow(clippy::too_many_arguments)]
fn push_item(
    items: &mut Vec<Gap>,
    seen: &mut Vec<String>,
    body: &str,
    raw_line: &str,
    line: usize,
    state: GapState,
    meta_order: MetaOrder,
) {
    let text = clean(body);
    if text.is_empty() {
        return;
    }
    // The tags are read from the WHOLE raw line, matching `_es.requires_tags(line)`.
    let tags = env_scope::requires_tags(raw_line);
    let strip = |text: &str| {
        if tags.is_empty() {
            text.to_string()
        } else {
            env_scope::strip_requires_tags(text)
        }
    };
    let text = match meta_order {
        MetaOrder::BeforeTagStrip => {
            if is_meta(&text) {
                return;
            }
            strip(&text)
        }
        MetaOrder::AfterTagStrip => {
            let stripped = strip(&text);
            if stripped.is_empty() || is_meta(&stripped) {
                return;
            }
            stripped
        }
    };
    if text.is_empty() {
        return;
    }
    // Dedupe by the first 80 chars of the lowercased text — a doc that restates a station in its
    // summary yields one gap, not two.
    let key: String = text.to_lowercase().chars().take(80).collect();
    if seen.contains(&key) {
        return;
    }
    seen.push(key);
    items.push(Gap {
        id: String::new(), // assigned after the whole list is known
        text,
        line,
        state,
        requires_env: tags,
    });
}

/// `^\s*[-*]\s*\[([ xX])\]\s+(.*\S)\s*$` → `(ticked, body)`.
fn checkbox(line: &str) -> Option<(bool, &str)> {
    let rest = line.trim_start();
    let rest = rest.strip_prefix('-').or_else(|| rest.strip_prefix('*'))?;
    let rest = rest.trim_start_matches([' ', '\t']);
    let rest = rest.strip_prefix('[')?;
    let mark = rest.chars().next()?;
    if mark != ' ' && mark != 'x' && mark != 'X' {
        return None;
    }
    let rest = rest[mark.len_utf8()..].strip_prefix(']')?;
    // `\s+` — at least one space must follow the bracket.
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let body = rest.trim();
    if body.is_empty() {
        return None;
    }
    Some((mark != ' ', body))
}

/// `^#{1,4}\s+(.*\S)\s*$` — an ATX heading, one to four hashes.
fn heading(line: &str) -> Option<&str> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if !(1..=4).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let body = rest.trim();
    if body.is_empty() {
        None
    } else {
        Some(body)
    }
}

/// `^\s*[-*]\s+(.*\S)\s*$` — a plain bullet.
fn bullet(line: &str) -> Option<&str> {
    let rest = line.trim_start();
    let rest = rest.strip_prefix('-').or_else(|| rest.strip_prefix('*'))?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let body = rest.trim();
    if body.is_empty() {
        None
    } else {
        Some(body)
    }
}

/// `re.sub(r"\s+", " ", re.sub(r"`|\*\*|__", "", s)).strip().rstrip(".")`.
fn clean(value: &str) -> String {
    let mut stripped = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            i += 1;
            continue;
        }
        if i + 1 < bytes.len() && (bytes[i] == b'*' || bytes[i] == b'_') && bytes[i + 1] == bytes[i]
        {
            i += 2;
            continue;
        }
        // Copy one whole UTF-8 char.
        let ch_len = utf8_len(bytes[i]);
        stripped.push_str(&value[i..i + ch_len]);
        i += ch_len;
    }
    let collapsed = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.trim().trim_end_matches('.').to_string()
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

/// `\b(atomic commit|tdd|self-review|no tbd|no placeholder|cargo fmt|clippy|run the|spec coverage|
/// scenario shapes?|definition of done|rustflags)\b`, case-insensitive.
///
/// `scenario shapes?` is spelled as two literals here — checking `scenario shape` alone with a right
/// word boundary would REFUSE the plural, so the alternation is the honest transcription.
const META_PHRASES: [&str; 13] = [
    "atomic commit",
    "tdd",
    "self-review",
    "no tbd",
    "no placeholder",
    "cargo fmt",
    "clippy",
    "run the",
    "spec coverage",
    "scenario shapes",
    "scenario shape",
    "definition of done",
    "rustflags",
];

fn is_meta(text: &str) -> bool {
    let lower = text.to_lowercase();
    META_PHRASES
        .iter()
        .any(|phrase| contains_word(&lower, phrase))
}

/// `\b(requirement|acceptance|task|gate|component|deliverable|criteria|must)\b`, case-insensitive.
///
/// The word boundaries are the kit's, and they are narrower than they look: `\brequirement\b` does
/// NOT match "Requirements", nor `\btask\b` "Tasks", because the trailing `s` is a word character.
/// That is transcribed rather than corrected — this arm exists only as a fallback for docs with no
/// checkboxes, and "widening" it here would make the native gap set diverge from every id already
/// minted under the kit's reading.
const REQ_WORDS: [&str; 8] = [
    "requirement",
    "acceptance",
    "task",
    "gate",
    "component",
    "deliverable",
    "criteria",
    "must",
];

fn requirement_heading(text: &str) -> bool {
    let lower = text.to_lowercase();
    REQ_WORDS.iter().any(|word| contains_word(&lower, word))
}

/// `\b<needle>\b` over an already-lowercased haystack. A word character is `[A-Za-z0-9_]`, matching
/// Python's `\w` for the ASCII phrases above.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let hb = haystack.as_bytes();
    let nb = needle.as_bytes();
    if nb.is_empty() || nb.len() > hb.len() {
        return false;
    }
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    // A boundary is only required on a side whose OWN edge character is a word character —
    // `\bself-review\b` opens on `s` and closes on `w`, but `\b-x\b` would not demand one on the left.
    let need_left = word(nb[0]);
    let need_right = word(nb[nb.len() - 1]);
    for start in 0..=(hb.len() - nb.len()) {
        if &hb[start..start + nb.len()] != nb {
            continue;
        }
        let left_ok = !need_left || start == 0 || !word(hb[start - 1]);
        let end = start + nb.len();
        let right_ok = !need_right || end == hb.len() || !word(hb[end]);
        if left_ok && right_ok {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkbox_stations_mint_ids_and_states() {
        let doc = "## Delivery stations\n\n- [ ] first station\n- [x] second station\n";
        let d = decompose("plans__x", doc, vec![]);
        assert_eq!(d.method, Method::CheckboxTasks);
        assert_eq!(d.items.len(), 2);
        assert_eq!(d.items[0].id, "plans__x#1");
        assert_eq!(d.items[0].state, GapState::Open);
        assert_eq!(d.items[0].line, 3);
        assert_eq!(d.items[1].id, "plans__x#2");
        assert_eq!(d.items[1].state, GapState::Claimed);
    }

    #[test]
    fn a_checked_box_is_claimed_never_fulfilled() {
        let d = decompose("plans__x", "- [X] done, allegedly\n", vec![]);
        assert_eq!(d.items[0].state, GapState::Claimed);
        assert_eq!(d.claimed(), 1);
        assert_eq!(d.open(), 0);
    }

    #[test]
    fn extraction_is_heading_agnostic() {
        // No `## Delivery stations` heading at all — the stations still extract.
        let d = decompose("plans__x", "prose\n\n* [ ] a station\n", vec![]);
        assert_eq!(d.items.len(), 1);
        assert_eq!(d.method, Method::CheckboxTasks);
    }

    #[test]
    fn meta_chatter_is_filtered_before_the_tag_is_stripped() {
        let doc = "- [ ] cargo fmt and clippy\n- [ ] real work @requires:shem\n";
        let d = decompose("plans__x", doc, vec![]);
        assert_eq!(d.items.len(), 1);
        assert_eq!(d.items[0].text, "real work");
        assert_eq!(d.items[0].requires_env, vec!["shem"]);
    }

    #[test]
    fn duplicate_text_yields_one_gap() {
        let d = decompose("plans__x", "- [ ] same thing\n- [ ] same thing\n", vec![]);
        assert_eq!(d.items.len(), 1);
    }

    #[test]
    fn markup_is_cleaned_the_way_the_kit_cleans_it() {
        assert_eq!(
            clean("`code`  **bold**  __x__ trailing."),
            "code bold x trailing"
        );
        assert_eq!(clean("multi\tspaced   words"), "multi spaced words");
    }

    #[test]
    fn the_fallback_arm_reads_requirement_headings_with_the_kits_boundaries() {
        // "Acceptance criteria" matches; "Requirements" does NOT (the trailing `s` kills `\b`).
        let doc = "## Requirements\n- not extracted\n\n## Acceptance criteria\n- extracted\n";
        let d = decompose("specs__y", doc, vec![]);
        assert_eq!(d.method, Method::RequirementBullets);
        assert_eq!(d.items.len(), 1);
        assert_eq!(d.items[0].text, "extracted");
        assert_eq!(d.items[0].state, GapState::Open);
    }

    #[test]
    fn the_two_arms_order_the_meta_filter_differently_because_the_kit_does() {
        // The checkbox arm tests META on the text WITH its tag; the bullet arm strips first. Only
        // an item whose META phrase lives inside the tag can tell them apart, and both readings are
        // pinned so a future tidy-up cannot collapse them onto one.
        let checkbox = decompose(
            "plans__x",
            "- [ ] real work @requires:clippy-lane\n",
            vec![],
        );
        assert!(
            checkbox.items.is_empty(),
            "the checkbox arm sees `clippy` inside the tag and filters the item"
        );
        let bullet = decompose(
            "specs__y",
            "## Acceptance criteria\n- real work @requires:clippy-lane\n",
            vec![],
        );
        assert_eq!(
            bullet.items.len(),
            1,
            "the bullet arm strips the tag first, so the same item survives"
        );
        assert_eq!(bullet.items[0].text, "real work");
        assert_eq!(bullet.items[0].requires_env, vec!["clippy-lane"]);
    }

    #[test]
    fn a_doc_with_no_structure_decomposes_to_nothing_rather_than_a_guess() {
        let d = decompose("specs__y", "# Title\n\nJust prose.\n", vec![]);
        assert_eq!(d.method, Method::None);
        assert!(d.items.is_empty());
    }

    #[test]
    fn the_slug_is_surface_prefixed() {
        assert_eq!(
            slug_for(Path::new("genesis/docs/superpowers/plans/2026-09-10-x.md")),
            "plans__2026-09-10-x"
        );
        assert_eq!(
            slug_for(Path::new("genesis/docs/superpowers/specs/CLAUDE.md")),
            "specs__CLAUDE"
        );
    }

    #[test]
    fn word_boundaries_match_pythons() {
        assert!(contains_word("run the gate", "run the"));
        assert!(!contains_word("overrun there", "run the"));
        assert!(contains_word("acceptance criteria", "acceptance"));
        assert!(!contains_word("requirements", "requirement"));
        assert!(!contains_word("tasks", "task"));
    }

    #[test]
    fn the_runaway_threshold_is_reported_not_enforced() {
        let doc: String = (0..THRESHOLD + 1)
            .map(|n| format!("- [ ] station number {n}\n"))
            .collect();
        let d = decompose("plans__big", &doc, vec![]);
        assert_eq!(d.items.len(), THRESHOLD + 1);
        assert!(d.runaway());
    }
}
