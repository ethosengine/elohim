//! CHUNK — the semantic fold's declared chunker (governed-discovery station 4, task 4.3), executed
//! by `elohim_epr_index::chunk` since the post-station-4 sprint lifted it (ruling R-S1); this file
//! is its adapter, and the tests below pin that the executor's rule still cuts exactly as before.
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
pub(super) use elohim_epr_index::chunk::ChunkRule;

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr_index::chunk::Chunk;
    use serde_json::json;

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
