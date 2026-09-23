//! CHUNK — the semantic fold's declared chunker (governed-discovery station 4, task 4.3).
//!
//! The rule is not this file's to choose. `recall-semantic-index@1` carries it as `_chunk_rule`,
//! and its `chunkRule` CID is `atom_cid` of that object; the fold re-hashes the object before it
//! runs (see `index.rs`) and this file only executes what the object says:
//!
//! - **markdown** (`*.md`) splits at ATX headings up to `markdown.max_level` (a `#` inside a fenced
//!   code block is code, not a heading); a file with no heading is one section;
//! - **python** (`*.py`) splits at top-level `def`/`class` lines (`python.split`);
//! - **other** text is cut into `other.window_bytes` line windows;
//! - every chunk is at most `max_chunk_bytes` (a longer section is cut into line windows of that
//!   size), and a file keeps at most `max_chunks_per_file` chunks — the rest are dropped and
//!   COUNTED, never silently lost.
//!
//! `passage.rs` is the lexical route's shape (any `#`, a density window) and is deliberately not
//! reused: the semantic method is the declared rule, and its CID is what every candidate prints.
use super::*;

/// The declared chunk rule, read strictly: a shape this chunker does not implement is refused
/// rather than approximated, because the rule's CID would then name a method nobody ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkRule {
    pub markdown_max_level: usize,
    pub python_split: Vec<String>,
    pub window_bytes: usize,
    pub max_chunk_bytes: usize,
    pub max_chunks_per_file: usize,
}

/// One chunk: the section it came from (a heading, a `def`/`class` line, or `lines a-b` for a
/// window of an unsectioned file) and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub section: String,
    pub text: String,
}

/// A file's chunks, and how many the per-file cap dropped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Chunked {
    pub chunks: Vec<Chunk>,
    pub dropped: usize,
}

fn rule_refused(what: &str) -> FlowError {
    refused(format!("chunk rule: {what}"))
}

fn positive(value: &Value, pointer: &str) -> FlowResult<usize> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
        .map(|n| n as usize)
        .ok_or_else(|| rule_refused(&format!("{pointer} must be a positive integer")))
}

impl ChunkRule {
    /// Read the measure's `_chunk_rule` object.
    pub fn from_declared(rule: &Value) -> FlowResult<Self> {
        if rule.pointer("/markdown/split").and_then(Value::as_str) != Some("heading") {
            return Err(rule_refused("markdown.split must be \"heading\""));
        }
        if rule.pointer("/other/split").and_then(Value::as_str) != Some("line-window") {
            return Err(rule_refused("other.split must be \"line-window\""));
        }
        let python_split: Vec<String> = rule
            .pointer("/python/split")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if python_split.is_empty() || python_split.iter().any(|k| k != "def" && k != "class") {
            return Err(rule_refused("python.split must name `def` and/or `class`"));
        }
        let parsed = Self {
            markdown_max_level: positive(rule, "/markdown/max_level")?,
            python_split,
            window_bytes: positive(rule, "/other/window_bytes")?,
            max_chunk_bytes: positive(rule, "/max_chunk_bytes")?,
            max_chunks_per_file: positive(rule, "/max_chunks_per_file")?,
        };
        if parsed.markdown_max_level > 6 {
            return Err(rule_refused("markdown.max_level is at most 6"));
        }
        Ok(parsed)
    }

    /// Chunk one file's text by the rule its extension selects.
    pub fn chunk(&self, path: &str, text: &str) -> Chunked {
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        let window = self.window_bytes.min(self.max_chunk_bytes);
        let sections = match extension {
            "md" => self.markdown_sections(text),
            "py" => self.python_sections(text),
            _ => vec![(String::new(), text.to_string())],
        };
        let mut chunks = Vec::new();
        for (section, body) in sections {
            if body.trim().is_empty() {
                continue;
            }
            let label_windows = section.is_empty();
            let limit = if extension == "md" || extension == "py" {
                self.max_chunk_bytes
            } else {
                window
            };
            for (first, last, piece) in windows(&body, limit) {
                if piece.trim().is_empty() {
                    continue;
                }
                let label = if label_windows {
                    format!("lines {first}-{last}")
                } else {
                    section.clone()
                };
                chunks.push(Chunk {
                    section: label,
                    text: piece,
                });
            }
        }
        let dropped = chunks.len().saturating_sub(self.max_chunks_per_file);
        chunks.truncate(self.max_chunks_per_file);
        Chunked { chunks, dropped }
    }

    /// `(heading, body)` pairs; the text before the first heading is an unlabelled section.
    fn markdown_sections(&self, text: &str) -> Vec<(String, String)> {
        let mut sections = vec![(String::new(), String::new())];
        let mut fence: Option<&str> = None;
        for line in text.split_inclusive('\n') {
            let trimmed = line.trim_start();
            if let Some(open) = fence {
                if trimmed.starts_with(open) {
                    fence = None;
                }
            } else if trimmed.starts_with("```") {
                fence = Some("```");
            } else if trimmed.starts_with("~~~") {
                fence = Some("~~~");
            } else if let Some(title) = heading(line, self.markdown_max_level) {
                sections.push((title, String::new()));
            }
            sections.last_mut().expect("one section").1.push_str(line);
        }
        sections
    }

    /// `(def/class line, body)` pairs at top level; the module prelude is unlabelled.
    fn python_sections(&self, text: &str) -> Vec<(String, String)> {
        let mut sections = vec![(String::new(), String::new())];
        for line in text.split_inclusive('\n') {
            let opens = self.python_split.iter().any(|keyword| {
                line.strip_prefix(keyword.as_str())
                    .is_some_and(|rest| rest.starts_with(' '))
                    || (keyword == "def" && line.starts_with("async def "))
            });
            if opens {
                sections.push((line.trim_end().to_string(), String::new()));
            }
            sections.last_mut().expect("one section").1.push_str(line);
        }
        sections
    }
}

/// An ATX heading of level `1..=max_level`: `#`s then a space (or nothing), no indentation.
fn heading(line: &str, max_level: usize) -> Option<String> {
    let level = line.bytes().take_while(|b| *b == b'#').count();
    if level == 0 || level > max_level {
        return None;
    }
    let rest = line[level..].trim_end();
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some(line.trim_end().to_string())
}

/// Cut `text` into pieces of at most `limit` bytes that end on a line boundary, with each piece's
/// first and last line number (1-based, within `text`). A single line longer than `limit` is cut
/// at the last character boundary that fits — the only place a window may end mid-line.
fn windows(text: &str, limit: usize) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    let mut current = String::new();
    let (mut first, mut line_no) = (1usize, 0usize);
    for line in text.split_inclusive('\n') {
        line_no += 1;
        if !current.is_empty() && current.len() + line.len() > limit {
            out.push((first, line_no - 1, std::mem::take(&mut current)));
        }
        if current.is_empty() {
            first = line_no;
        }
        // `current` is empty whenever `rest` alone is over the limit (the flush above saw to it).
        let mut rest = line;
        while rest.len() > limit {
            let mut cut = limit;
            while !rest.is_char_boundary(cut) {
                cut -= 1;
            }
            if cut == 0 {
                // A limit narrower than one character still makes progress, one character a piece.
                cut = rest.chars().next().map_or(rest.len(), char::len_utf8);
            }
            out.push((line_no, line_no, rest[..cut].to_string()));
            rest = &rest[cut..];
        }
        current.push_str(rest);
    }
    if !current.is_empty() {
        out.push((first, line_no.max(first), current));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared() -> ChunkRule {
        ChunkRule::from_declared(&json!({
            "markdown": { "split": "heading", "max_level": 4 },
            "python": { "split": ["def", "class"] },
            "other": { "split": "line-window", "window_bytes": 2000 },
            "max_chunk_bytes": 2000,
            "max_chunks_per_file": 40
        }))
        .expect("the declared rule reads")
    }

    #[test]
    fn markdown_splits_at_headings_up_to_level_four_and_never_inside_a_fence() {
        let text = "intro\n# One\na\n## Two\nb\n#### Four\nc\n##### Five\nd\n```sh\n# comment\n```\n#nospace\n";
        let chunked = declared().chunk("doc.md", text);
        let sections: Vec<&str> = chunked.chunks.iter().map(|c| c.section.as_str()).collect();
        assert_eq!(sections, ["lines 1-1", "# One", "## Two", "#### Four"]);
        let four = &chunked.chunks[3].text;
        assert!(
            four.contains("##### Five") && four.contains("# comment") && four.contains("#nospace")
        );
        assert_eq!(chunked.dropped, 0);
    }

    #[test]
    fn a_markdown_file_with_no_heading_is_one_chunk_then_windowed() {
        let rule = declared();
        assert_eq!(rule.chunk("a.md", "just prose\nmore\n").chunks.len(), 1);
        let long: String = (0..300).map(|i| format!("line number {i:04}\n")).collect();
        let chunked = rule.chunk("a.md", &long);
        assert!(chunked.chunks.len() > 1);
        for chunk in &chunked.chunks {
            assert!(chunk.text.len() <= 2000);
            assert!(
                chunk.text.ends_with('\n'),
                "a window ends on a line boundary"
            );
        }
        assert_eq!(
            chunked
                .chunks
                .iter()
                .map(|c| c.text.as_str())
                .collect::<String>(),
            long,
            "windows cover the section exactly"
        );
    }

    #[test]
    fn python_splits_at_top_level_def_and_class_only() {
        let text = "import os\n\ndef a():\n    def inner():\n        pass\n\nclass B:\n    def m(self):\n        pass\nasync def c():\n    pass\ndefault = 1\n";
        let sections: Vec<String> = declared()
            .chunk("m.py", text)
            .chunks
            .into_iter()
            .map(|c| c.section)
            .collect();
        // `default = 1` begins with `def` but is not a `def` line; a nested `def` is not top level.
        assert_eq!(
            sections,
            ["lines 1-2", "def a():", "class B:", "async def c():"]
        );
    }

    #[test]
    fn other_text_is_cut_into_line_windows_within_the_cap() {
        let text: String = (0..500).map(|i| format!("\"key{i}\": {i},\n")).collect();
        let chunked = declared().chunk("data.json", &text);
        assert!(chunked.chunks.len() > 1);
        assert!(chunked.chunks[0].section.starts_with("lines 1-"));
        for chunk in &chunked.chunks {
            assert!(chunk.text.len() <= 2000 && chunk.text.ends_with('\n'));
        }
    }

    #[test]
    fn a_line_longer_than_the_cap_is_cut_at_a_character_boundary() {
        let line = "é".repeat(1500); // 3000 bytes, two-byte characters
        let chunked = declared().chunk("x.json", &line);
        assert_eq!(chunked.chunks.len(), 2);
        assert!(chunked.chunks.iter().all(|c| c.text.len() <= 2000));
        assert_eq!(chunked.chunks.concat_text(), line);
    }

    #[test]
    fn at_most_forty_chunks_per_file_and_the_rest_are_counted() {
        let text: String = (0..50).map(|i| format!("# H{i}\nbody {i}\n")).collect();
        let chunked = declared().chunk("many.md", &text);
        assert_eq!(chunked.chunks.len(), 40);
        assert_eq!(chunked.dropped, 10);
    }

    #[test]
    fn an_undeclared_rule_shape_is_refused() {
        let mut rule = json!({
            "markdown": { "split": "heading", "max_level": 4 },
            "python": { "split": ["def", "class"] },
            "other": { "split": "line-window", "window_bytes": 2000 },
            "max_chunk_bytes": 2000,
            "max_chunks_per_file": 40
        });
        rule["other"]["split"] = json!("paragraph");
        assert!(ChunkRule::from_declared(&rule).is_err());
        rule["other"]["split"] = json!("line-window");
        rule["max_chunks_per_file"] = json!(0);
        assert!(ChunkRule::from_declared(&rule).is_err());
    }

    trait ConcatText {
        fn concat_text(&self) -> String;
    }
    impl ConcatText for Vec<Chunk> {
        fn concat_text(&self) -> String {
            self.iter().map(|c| c.text.as_str()).collect()
        }
    }
}
