//! The declared chunk rule, executed by the crate exactly as the recall executor executed it
//! before the lift: the same fixture strings the executor's own chunk tests use, and the same
//! `(section, text)` vectors out.
use elohim_epr_index::chunk::{ChunkRule, Chunked};
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

fn pairs(chunked: &Chunked) -> Vec<(String, String)> {
    chunked
        .chunks
        .iter()
        .map(|c| (c.section.clone(), c.text.clone()))
        .collect()
}

fn owned(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(s, t)| (s.to_string(), t.to_string()))
        .collect()
}

/// Greedy line packing, written independently of the crate: whole lines while they fit in
/// `limit` bytes, each window labelled by its first and last line.
fn reference_windows(text: &str, limit: usize) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let (mut current, mut first) = (String::new(), 1usize);
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let number = index + 1;
        if !current.is_empty() && current.len() + line.len() > limit {
            out.push((
                format!("lines {first}-{}", number - 1),
                std::mem::take(&mut current),
            ));
        }
        if current.is_empty() {
            first = number;
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        let last = text.split_inclusive('\n').count();
        out.push((format!("lines {first}-{last}"), current));
    }
    out
}

#[test]
fn markdown_atx_splits_to_level_4() {
    let text = "intro\n# One\na\n## Two\nb\n#### Four\nc\n##### Five\nd\n```sh\n# comment\n```\n#nospace\n";
    let chunked = declared().chunk("doc.md", text);
    assert_eq!(
        pairs(&chunked),
        owned(&[
            ("lines 1-1", "intro\n"),
            ("# One", "# One\na\n"),
            ("## Two", "## Two\nb\n"),
            (
                "#### Four",
                "#### Four\nc\n##### Five\nd\n```sh\n# comment\n```\n#nospace\n"
            ),
        ])
    );
    assert_eq!(chunked.dropped, 0);
}

#[test]
fn python_def_class_sections() {
    let text = "import os\n\ndef a():\n    def inner():\n        pass\n\nclass B:\n    def m(self):\n        pass\nasync def c():\n    pass\ndefault = 1\n";
    let chunked = declared().chunk("m.py", text);
    assert_eq!(
        pairs(&chunked),
        owned(&[
            ("lines 1-2", "import os\n\n"),
            ("def a():", "def a():\n    def inner():\n        pass\n\n"),
            ("class B:", "class B:\n    def m(self):\n        pass\n"),
            ("async def c():", "async def c():\n    pass\ndefault = 1\n"),
        ])
    );
}

#[test]
fn other_2000_byte_windows() {
    let text: String = (0..500).map(|i| format!("\"key{i}\": {i},\n")).collect();
    let chunked = declared().chunk("data.json", &text);
    assert!(chunked.chunks.len() > 1);
    assert_eq!(pairs(&chunked), reference_windows(&text, 2000));

    // A line longer than the cap is cut at a character boundary — the one mid-line cut.
    let line = "é".repeat(1500);
    let long = declared().chunk("x.json", &line);
    assert_eq!(
        pairs(&long),
        owned(&[
            ("lines 1-1", &"é".repeat(1000)),
            ("lines 1-1", &"é".repeat(500)),
        ])
    );
}

#[test]
fn cap_40_chunks_per_unit() {
    let text: String = (0..50).map(|i| format!("# H{i}\nbody {i}\n")).collect();
    let chunked = declared().chunk("many.md", &text);
    let expected: Vec<(String, String)> = (0..40)
        .map(|i| (format!("# H{i}"), format!("# H{i}\nbody {i}\n")))
        .collect();
    assert_eq!(pairs(&chunked), expected);
    assert_eq!(chunked.dropped, 10);
}

#[test]
fn an_undeclared_rule_shape_is_refused_as_invalid_arguments() {
    let mut rule = json!({
        "markdown": { "split": "heading", "max_level": 4 },
        "python": { "split": ["def", "class"] },
        "other": { "split": "line-window", "window_bytes": 2000 },
        "max_chunk_bytes": 2000,
        "max_chunks_per_file": 40
    });
    rule["other"]["split"] = json!("paragraph");
    let refused = ChunkRule::from_declared(&rule).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "invalid arguments: chunk rule: other.split must be \"line-window\""
    );
    rule["other"]["split"] = json!("line-window");
    rule["max_chunks_per_file"] = json!(0);
    assert!(ChunkRule::from_declared(&rule).is_err());
}
