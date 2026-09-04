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

/// Same-origin, relative `<script src>` and `<link rel="stylesheet" href>`
/// references in document order. Skips anything with a scheme or a leading
/// `//` (CDN), and the doorway-injected `/chrome/` island, which is never part
/// of a bundle. A leading `/` is treated as bundle-root-relative.
pub fn shell_asset_refs(index_html: &str) -> Vec<String> {
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
        let trimmed = raw.trim_start_matches('/');
        if trimmed.starts_with("chrome/") || trimmed.is_empty() {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out
}

/// Judge the extracted entries of one bundle. `entries` is the `(name, bytes)`
/// list the extraction walk already produces; directories are not included.
pub fn judge_deliverability(entries: &[(String, Vec<u8>)]) -> DeliverabilityVerdict {
    let Some((index_name, index_bytes)) = entries
        .iter()
        .find(|(n, _)| n == "index.html" || n.ends_with("/index.html"))
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
        let in_index_dir = format!("{index_dir}{asset}");
        if held.contains(in_index_dir.as_str()) || held.contains(asset.as_str()) {
            continue;
        }
        return DeliverabilityVerdict::Broken(BrokenReason::MissingAsset(asset));
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
        let refs = shell_asset_refs(INDEX);
        assert_eq!(
            refs,
            vec![
                "styles-7XLYMW2X.css".to_string(),
                "polyfills-X2TQPNDQ.js".to_string(),
                "main-7QFGHX5X.js".to_string(),
            ],
            "icons, CDN scripts and the doorway-injected /chrome/ script are not bundle assets"
        );
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
