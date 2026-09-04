//! Deliverability verdict — a pure derivation of a bundle's bytes.
//!
//! Whether an EPR app head can boot is a property of the ZIP, not of any
//! doorway: `index.html` either names assets the bundle holds or it does not.
//! The peer that holds the bytes judges them once (inside the extraction walk
//! it already performs) and every other peer re-derives the same answer from
//! the same CID. Spec: 2026-09-05 EPR-app delivery verdict §2.
//!
//! The 2026-09-04 incident this exists for: a shell naming `main-EAKNZDUP.js`
//! was served while the bundle held `main-7QFGHX5X.js` — a blank page for every
//! visitor, and nothing on the serving side had judged the head at all.

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokenReason {
    /// `index.html` names a same-origin asset the bundle does not hold.
    MissingAsset(String),
    /// The blob is not a readable ZIP archive.
    InvalidZip,
    /// No `index.html` (top-level or nested) in the bundle.
    NoIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotJudgedWhy {
    /// The bytes are not held locally yet (syncing / absent) — honest absence.
    NotHeld,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliverabilityVerdict {
    Boots,
    Broken(BrokenReason),
    NotJudged(NotJudgedWhy),
}

impl DeliverabilityVerdict {
    pub fn header_value(&self) -> &'static str {
        match self {
            Self::Boots => "boots",
            Self::Broken(_) => "broken",
            Self::NotJudged(_) => "not-judged",
        }
    }

    pub fn reason_value(&self) -> Option<String> {
        match self {
            Self::Boots => None,
            Self::Broken(BrokenReason::MissingAsset(name)) => Some(format!("missing-asset:{name}")),
            Self::Broken(BrokenReason::InvalidZip) => Some("invalid-zip".to_string()),
            Self::Broken(BrokenReason::NoIndex) => Some("no-index".to_string()),
            Self::NotJudged(NotJudgedWhy::NotHeld) => Some("not-held".to_string()),
        }
    }
}

/// A same-origin asset reference extracted from a shell document, carrying the
/// resolution intent the markup expressed: `root_relative` (a leading `/`)
/// means "resolve against the bundle root", not "resolve against the
/// document's own directory". The two are NOT interchangeable — a fix for the
/// 2026-09-04 incident's review round: a doc-relative ref that happens to also
/// exist at the bundle root must NOT be treated as satisfied by that root
/// copy (false-positive Boots), and a root-relative ref must NOT be resolved
/// against a nested index's own directory (false-positive Broken).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRef {
    /// The reference with any leading `/` stripped.
    pub path: String,
    /// `true` if the original markup had a leading `/` (bundle-root-relative).
    /// `false` means document-relative (resolve against the index's own dir).
    pub root_relative: bool,
}

/// Same-origin `<script src>` and `<link rel="stylesheet" href>` references in
/// document order, each tagged with its resolution intent (see [`AssetRef`]).
/// Skips anything with a scheme or a leading `//` (CDN) or a `data:` URI, and
/// the doorway-injected `/chrome/` island, which is never part of a bundle.
pub fn shell_asset_refs(index_html: &str) -> Vec<AssetRef> {
    let mut out = Vec::new();
    for tag in html_tags(index_html) {
        let lower = tag.to_ascii_lowercase();
        let attr = if lower.starts_with("<script") {
            "src"
        } else if lower.starts_with("<link")
            && attr_value(&lower, "rel").as_deref() == Some("stylesheet")
        {
            "href"
        } else {
            continue;
        };
        let Some(raw) = attr_value(&tag, attr) else {
            continue;
        };
        if raw.contains("://") || raw.starts_with("//") || raw.starts_with("data:") {
            continue;
        }
        let root_relative = raw.starts_with('/');
        let trimmed = raw.trim_start_matches('/');
        if trimmed.starts_with("chrome/") || trimmed.is_empty() {
            continue;
        }
        out.push(AssetRef {
            path: trimmed.to_string(),
            root_relative,
        });
    }
    out
}

/// Judge the extracted entries of one bundle. `entries` is the `(name, bytes)`
/// list the extraction walk already produces; directories are not included.
///
/// Resolution is browser-semantics, not a two-way lookup: a `root_relative`
/// asset ref (leading `/` in the markup) is checked ONLY against the bundle
/// root; a document-relative ref is checked ONLY against the index's own
/// directory. A ref held at the "other" location does not satisfy it — that
/// would let a nested bundle read as bootable when the browser, resolving the
/// same markup against the same base URL, would 404.
///
/// When more than one `index.html` exists (top-level and/or nested), the
/// shallowest one (fewest `/` in its path) is the shell; ties at equal depth
/// are broken by `entries` order (the earlier one wins).
pub fn judge_deliverability(entries: &[(String, Vec<u8>)]) -> DeliverabilityVerdict {
    let Some((index_name, index_bytes)) = entries
        .iter()
        .filter(|(n, _)| n == "index.html" || n.ends_with("/index.html"))
        .min_by_key(|(n, _)| n.matches('/').count())
    else {
        return DeliverabilityVerdict::Broken(BrokenReason::NoIndex);
    };
    let index_dir = index_name
        .rfind('/')
        .map(|i| &index_name[..=i])
        .unwrap_or("");
    let held: HashSet<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    let index_html = String::from_utf8_lossy(index_bytes);
    for asset in shell_asset_refs(&index_html) {
        let resolved = if asset.root_relative {
            asset.path.clone()
        } else {
            format!("{index_dir}{}", asset.path)
        };
        if held.contains(resolved.as_str()) {
            continue;
        }
        return DeliverabilityVerdict::Broken(BrokenReason::MissingAsset(asset.path));
    }
    DeliverabilityVerdict::Boots
}

/// Every `<...>` tag as a slice, without a parser dependency: the shell is a
/// build artifact, not user input, so a tag scanner is enough.
fn html_tags(html: &str) -> impl Iterator<Item = &str> {
    html.match_indices('<').filter_map(move |(start, _)| {
        let rest = &html[start..];
        let end = rest.find('>')?;
        Some(&rest[..=end])
    })
}

fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{attr}=");
    let mut search = 0;
    while let Some(pos) = lower[search..].find(&needle) {
        let at = search + pos;
        let before = if at == 0 {
            ' '
        } else {
            lower.as_bytes()[at - 1] as char
        };
        if before.is_whitespace() {
            let value_start = at + needle.len();
            let rest = &tag[value_start..];
            let quote = rest.chars().next()?;
            return if quote == '"' || quote == '\'' {
                rest[1..].find(quote).map(|e| rest[1..1 + e].to_string())
            } else {
                Some(
                    rest.split(|c: char| c.is_whitespace() || c == '>')
                        .next()?
                        .to_string(),
                )
            };
        }
        search = at + needle.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, body: &str) -> (String, Vec<u8>) {
        (name.to_string(), body.as_bytes().to_vec())
    }

    fn doc_relative(path: &str) -> AssetRef {
        AssetRef {
            path: path.to_string(),
            root_relative: false,
        }
    }

    fn root_relative(path: &str) -> AssetRef {
        AssetRef {
            path: path.to_string(),
            root_relative: true,
        }
    }

    const INDEX: &str = r#"<!doctype html><html><head>
      <link rel="stylesheet" href="styles-7XLYMW2X.css">
      <link rel="icon" href="favicon.ico">
      <script src="https://cdn.example/x.js"></script>
      <script src="/chrome/omni-element.abc.js"></script>
    </head><body>
      <script src="polyfills-X2TQPNDQ.js" type="module"></script>
      <script src="main-7QFGHX5X.js" type="module"></script>
    </body></html>"#;

    #[test]
    fn shell_asset_refs_keeps_only_same_origin_relative_scripts_and_stylesheets() {
        // /chrome/ stays skipped even though it's root-relative; a genuine
        // root-relative asset (/vendor/lib.js, not under /chrome/) is kept
        // and tagged root_relative: true.
        let html = INDEX.replacen(
            "</head>",
            r#"<script src="/vendor/lib.js"></script></head>"#,
            1,
        );
        let refs = shell_asset_refs(&html);
        assert_eq!(
            refs,
            vec![
                doc_relative("styles-7XLYMW2X.css"),
                root_relative("vendor/lib.js"),
                doc_relative("polyfills-X2TQPNDQ.js"),
                doc_relative("main-7QFGHX5X.js"),
            ],
            "icons, CDN scripts and the doorway-injected /chrome/ script are not bundle assets; \
             a genuine root-relative ref is kept and flagged"
        );
    }

    #[test]
    fn data_uri_and_protocol_relative_cdn_refs_are_skipped() {
        let html = r#"<script src="data:text/javascript;base64,ZmFrZQ=="></script>
            <script src="//cdn.example/lib.js"></script>
            <script src="ok.js"></script>"#;
        let refs = shell_asset_refs(html);
        assert_eq!(refs, vec![doc_relative("ok.js")]);
    }

    #[test]
    fn link_href_before_rel_is_still_collected() {
        let html = r#"<link href="x.css" rel="stylesheet">"#;
        let refs = shell_asset_refs(html);
        assert_eq!(refs, vec![doc_relative("x.css")]);
    }

    #[test]
    fn a_bundle_whose_index_names_only_held_assets_boots() {
        let entries = vec![
            entry("index.html", INDEX),
            entry("styles-7XLYMW2X.css", "body{}"),
            entry("polyfills-X2TQPNDQ.js", "//p"),
            entry("main-7QFGHX5X.js", "//m"),
        ];
        assert!(matches!(
            judge_deliverability(&entries),
            DeliverabilityVerdict::Boots
        ));
    }

    #[test]
    fn a_bundle_whose_index_names_a_missing_entry_script_is_broken_and_names_it() {
        // The 2026-09-04 shape: the shell names an entry script the bundle does not hold.
        let entries = vec![
            entry("index.html", INDEX),
            entry("styles-7XLYMW2X.css", "body{}"),
            entry("polyfills-X2TQPNDQ.js", "//p"),
        ];
        match judge_deliverability(&entries) {
            DeliverabilityVerdict::Broken(BrokenReason::MissingAsset(name)) => {
                assert_eq!(name, "main-7QFGHX5X.js");
            }
            other => panic!("expected Broken(MissingAsset), got {other:?}"),
        }
    }

    #[test]
    fn a_bundle_with_no_index_is_broken_no_index() {
        let entries = vec![entry("main-7QFGHX5X.js", "//m")];
        assert!(matches!(
            judge_deliverability(&entries),
            DeliverabilityVerdict::Broken(BrokenReason::NoIndex)
        ));
    }

    #[test]
    fn a_nested_index_resolves_assets_relative_to_its_own_directory() {
        // Angular dists are sometimes zipped with a top-level folder.
        let entries = vec![
            entry("browser/index.html", r#"<script src="main-A.js"></script>"#),
            entry("browser/main-A.js", "//m"),
        ];
        assert!(matches!(
            judge_deliverability(&entries),
            DeliverabilityVerdict::Boots
        ));
    }

    #[test]
    fn a_doc_relative_ref_held_only_at_bundle_root_is_broken_for_a_nested_index() {
        // A document-relative ref must resolve against the index's OWN
        // directory only; a same-named file sitting at the bundle root does
        // not satisfy it (that would be reading a browser's base-URL
        // resolution wrong).
        let entries = vec![
            entry("browser/index.html", r#"<script src="main-A.js"></script>"#),
            entry("main-A.js", "//m"), // held only at the root, not at browser/
        ];
        match judge_deliverability(&entries) {
            DeliverabilityVerdict::Broken(BrokenReason::MissingAsset(name)) => {
                assert_eq!(name, "main-A.js");
            }
            other => panic!("expected Broken(MissingAsset), got {other:?}"),
        }
    }

    #[test]
    fn a_root_relative_ref_resolves_at_the_bundle_root_for_a_nested_index() {
        // A root-relative ref (leading `/`) must resolve against the bundle
        // root even when the index itself is nested.
        let entries = vec![
            entry(
                "browser/index.html",
                r#"<script src="/main-A.js"></script>"#,
            ),
            entry("main-A.js", "//m"),
        ];
        assert!(matches!(
            judge_deliverability(&entries),
            DeliverabilityVerdict::Boots
        ));
    }

    #[test]
    fn when_multiple_index_html_exist_the_shallowest_wins() {
        // browser/index.html (depth 1) appears first in entries order, but
        // the shallower top-level index.html (depth 0) must be the shell.
        // Proven by making the deep one's asset absent: if the deep index
        // were (wrongly) chosen, this would report Broken instead of Boots.
        let entries = vec![
            entry(
                "browser/index.html",
                r#"<script src="missing-if-picked.js"></script>"#,
            ),
            entry("index.html", r#"<script src="shallow.js"></script>"#),
            entry("shallow.js", "//s"),
        ];
        assert!(matches!(
            judge_deliverability(&entries),
            DeliverabilityVerdict::Boots
        ));
    }

    #[test]
    fn ties_at_equal_depth_are_broken_by_entries_order() {
        // a/index.html and b/index.html are both depth 1; a/ comes first in
        // entries order and must win. Proven by making b/'s asset absent: if
        // b/index.html were (wrongly) chosen, this would report Broken.
        let entries = vec![
            entry("a/index.html", r#"<script src="a.js"></script>"#),
            entry("a/a.js", "//a"),
            entry("b/index.html", r#"<script src="missing.js"></script>"#),
        ];
        assert!(matches!(
            judge_deliverability(&entries),
            DeliverabilityVerdict::Boots
        ));
    }

    #[test]
    fn header_and_reason_values_are_the_wire_vocabulary() {
        assert_eq!(DeliverabilityVerdict::Boots.header_value(), "boots");
        assert_eq!(DeliverabilityVerdict::Boots.reason_value(), None);
        let b = DeliverabilityVerdict::Broken(BrokenReason::MissingAsset("main-X.js".into()));
        assert_eq!(b.header_value(), "broken");
        assert_eq!(b.reason_value().as_deref(), Some("missing-asset:main-X.js"));
        assert_eq!(
            DeliverabilityVerdict::Broken(BrokenReason::InvalidZip)
                .reason_value()
                .as_deref(),
            Some("invalid-zip")
        );
        assert_eq!(
            DeliverabilityVerdict::NotJudged(NotJudgedWhy::NotHeld).header_value(),
            "not-judged"
        );
        assert_eq!(
            DeliverabilityVerdict::NotJudged(NotJudgedWhy::NotHeld)
                .reason_value()
                .as_deref(),
            Some("not-held")
        );
    }
}
