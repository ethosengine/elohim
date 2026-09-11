//! The gap-granular substrate-scope resolver — a transcription of `.claude/scripts/_lib/env_scope.py`.
//!
//! The scope model is **gap-granular**, isomorphic with a2o's per-scenario `@requires:<cap>` tags: a
//! plan's gaps each resolve a `requires_env`, defaulting to the document-level frontmatter value and
//! overridable per gap. A gap is BLOCKED-BY-ENV iff a cluster-TRACKED required capability is
//! unavailable — independent of whether its parent doc sits in `held/`. That independence is what
//! honours "iroh ≠ shem": a mixed plan keeps its household-testable gaps pickable while only its
//! cross-node-assertion gaps wait for the unavailable canvas.
//!
//! The authoring convention the hold decision leans on:
//!   - a doc-level `requires_env` is the default for EVERY gap ⇒ a UNIFORMLY-blocked doc ⇒ held whole;
//!   - a MIXED plan declares NO doc-level `requires_env` and tags only its divergent gaps
//!     `@requires:<cap>` ⇒ it stays on the plate, and only the tagged gaps are BLOCKED-BY-ENV.
//!
//! ## `@act:<i|ii|iii|host>` — the a2o feature-file act baseline
//!
//! `.feature` files carry no frontmatter. a2o's runtime arm resolves an `@act:` tag against that
//! ACT's OWN lane-contract file, because that lane file is what the mesh stage actually points the
//! runtime at (`ELOHIM_CLUSTER_STATE_PATH_OVERRIDE`) — not the live/default `cluster-state.yaml` a
//! non-mesh dev loop reads. [`act_baseline_caps`] mirrors that: it ADDS capabilities a `.feature`'s
//! `@requires:` tag can be satisfied by, on top of live cluster-state, and never removes one.
//!
//! Deliberately narrower than the runtime's full gating: the planning layer only needs "would this
//! feature run in ITS OWN act's lane", so a false HELD (a feature that only fails to run in the
//! wrong lane) never benches work the mesh stage can actually exercise.

use std::collections::BTreeSet;
use std::path::Path;

use super::cluster_state;

/// Which cluster-state file declares each act's BASELINE. `host` → `None`: no substrate at all, so
/// there is no baseline file to read.
pub fn act_baseline_file(act: &str) -> Option<&'static str> {
    match act {
        "i" => Some("cluster-state.act1-household.yaml"),
        "ii" => Some("cluster-state.act2-neighbourhood.yaml"),
        "iii" => Some("cluster-state.yaml"),
        _ => None,
    }
}

/// Normalize a frontmatter `requires_env` scalar → the capability list.
///
/// Handles the three shapes the minimal frontmatter parsers produce: an already-split block list, an
/// inline list `[a, b]` / `[]` (which the scalar arm hands back as a string), or a bare scalar.
/// Strips inline comments and quotes.
pub fn parse_requires_env_scalar(value: &str) -> Vec<String> {
    let s = strip_inline_comment(value.trim());
    let s = s.trim();
    if s.is_empty() {
        return Vec::new();
    }
    let items: Vec<&str> =
        if let Some(inner) = s.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            let inner = inner.trim();
            if inner.is_empty() {
                Vec::new()
            } else {
                inner.split(',').collect()
            }
        } else {
            vec![s]
        };
    items
        .into_iter()
        .map(clean_item)
        .filter(|x| !x.is_empty())
        .collect()
}

/// The same normalization for an already-split block list.
pub fn parse_requires_env_list(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|x| clean_item(x))
        .filter(|x| !x.is_empty())
        .collect()
}

fn clean_item(raw: &str) -> String {
    strip_inline_comment(raw)
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .trim()
        .to_string()
}

/// Python's `re.sub(r"\s+#.*$", "", s)` — a `#` is only a comment when whitespace precedes it, so a
/// capability name containing `#` (there are none, but the rule is the rule) survives.
fn strip_inline_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'#' && i > 0 && bytes[i - 1].is_ascii_whitespace() {
            return value[..i].trim_end();
        }
    }
    value
}

/// Parse `@requires:<cap>` markers out of a gap's source text — the per-gap override source,
/// isomorphic with a2o's per-scenario tags.
pub fn requires_tags(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find("@requires:") {
        let tail = &rest[pos + "@requires:".len()..];
        let end = tail
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .unwrap_or(tail.len());
        let cap = &tail[..end];
        // `[a-z0-9][a-z0-9-]*` — a tag must OPEN on an alphanumeric, so `@requires:-x` is not a tag.
        if !cap.is_empty()
            && cap.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            out.push(cap.to_string());
        }
        rest = &tail[end.max(1).min(tail.len())..];
    }
    out
}

/// Strip every `@requires:<cap>` marker out of a gap's text, the way `decompose.py` does before it
/// stores the item — the tag is scope metadata, not part of the assertion.
pub fn strip_requires_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find("@requires:") {
        let (head, tail) = rest.split_at(pos);
        // `\s*@requires:[a-z0-9-]+` — the leading whitespace goes with the tag.
        out.push_str(head.trim_end_matches([' ', '\t']));
        let tail = &tail["@requires:".len()..];
        let end = tail
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    out.trim().to_string()
}

/// The gap-level `requires_env` when it is explicitly set, else the document default.
pub fn resolved_requires_env(gap_env: &[String], doc_env: &[String]) -> Vec<String> {
    if !gap_env.is_empty() {
        return gap_env.to_vec();
    }
    doc_env.to_vec()
}

/// A gap is BLOCKED-BY-ENV iff a cluster-TRACKED required capability is unavailable.
///
/// Capabilities outside `known` (a stray a2o fixture tag, say) do NOT gate; an empty or
/// fully-satisfied requirement is not blocked.
pub fn gap_blocked(
    resolved: &[String],
    available: &BTreeSet<String>,
    known: &BTreeSet<String>,
) -> bool {
    let relevant: Vec<&String> = resolved.iter().filter(|c| known.contains(*c)).collect();
    !relevant.is_empty() && !relevant.iter().all(|c| available.contains(*c))
}

/// The `@act:<i|ii|iii|host>` tag a `.feature` declares, or `None`.
///
/// Scans only the tag line(s) ABOVE the first `Feature:` keyword, skipping gherkin `#` comments, so
/// prose mentioning the tag is not read as one. First tag wins if a doc mistakenly declares two —
/// the same choice the a2o runtime makes.
pub fn feature_act(text: &str) -> Option<String> {
    for line in text.lines() {
        let stripped = line.trim_start();
        if stripped.starts_with("Feature:") {
            break;
        }
        if stripped.starts_with('#') {
            continue;
        }
        if let Some(act) = act_tag(stripped) {
            return Some(act);
        }
    }
    None
}

/// `@act:(i|ii|iii|host)\b` on one line.
fn act_tag(line: &str) -> Option<String> {
    let mut rest = line;
    while let Some(pos) = rest.find("@act:") {
        let tail = &rest[pos + "@act:".len()..];
        for candidate in ["host", "iii", "ii", "i"] {
            if let Some(after) = tail.strip_prefix(candidate) {
                // `\b` — the token must end at a non-word character.
                if after
                    .chars()
                    .next()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
                {
                    return Some(candidate.to_string());
                }
            }
        }
        rest = tail;
    }
    None
}

/// Capabilities `<act>` provides BY DEFINITION — the `available: true` resources in the act's OWN
/// lane-contract file.
///
/// `host` → empty (no substrate at all). `iii` reads the live `cluster-state.yaml`, its own baseline
/// being the live fleet, plus `shem`, its stage by definition. An unreadable or missing act file →
/// empty: fail-open, because a missing file must never invent a gate.
///
/// ADDITIVE ONLY. Callers UNION this into an already-known `available` set for `.feature` docs and
/// never replace it, so a capability the live manifest withholds from the shared lane on purpose is
/// still rescued for a scenario tagged with the act whose OWN lane contract grants it.
pub fn act_baseline_caps(act: &str, manifests_dir: &Path) -> BTreeSet<String> {
    act_baseline_caps_in(act, std::slice::from_ref(&manifests_dir.to_path_buf()))
}

/// The same reading over SEVERAL candidate manifest directories, first hit wins.
///
/// A `--cluster-state` override names ONE file, and the act baselines are its siblings — so an
/// override pointed at an isolated copy of `cluster-state.yaml` finds no lane contracts beside it,
/// every `@act:` rescue silently evaporates, and features that only ever run in their own act's lane
/// are proposed for `held/`. Measured: three moves became nine on an isolated copy.
///
/// Fail-open in the same direction as before, and additive: a directory that genuinely holds the
/// whole lane set is still used as the lane set (an operator pointing at an alternative manifests
/// tree gets that tree), and the repository's own `genesis/manifests` answers only for the lane
/// files that directory does not have. A capability is still never taken away by a missing file.
pub fn act_baseline_caps_in(act: &str, manifests_dirs: &[std::path::PathBuf]) -> BTreeSet<String> {
    let Some(file) = act_baseline_file(act) else {
        return BTreeSet::new();
    };
    let mut caps = BTreeSet::new();
    for dir in manifests_dirs {
        let candidate = dir.join(file);
        if candidate.is_file() {
            caps = cluster_state::load(&candidate).available_names();
            break;
        }
    }
    if act == "iii" {
        caps.insert("shem".to_string());
    }
    caps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn requires_env_reads_all_three_authored_shapes() {
        assert_eq!(parse_requires_env_scalar("[a, b]"), vec!["a", "b"]);
        assert_eq!(parse_requires_env_scalar("[]"), Vec::<String>::new());
        assert_eq!(parse_requires_env_scalar("shem"), vec!["shem"]);
        assert_eq!(parse_requires_env_scalar("shem  # why"), vec!["shem"]);
        assert_eq!(
            parse_requires_env_list(&["'a'".into(), "".into(), "b # c".into()]),
            vec!["a", "b"]
        );
    }

    #[test]
    fn requires_tags_are_parsed_and_stripped_the_way_decompose_does() {
        let line = "- [ ] land the thing @requires:shem and then some";
        assert_eq!(requires_tags(line), vec!["shem"]);
        assert_eq!(
            strip_requires_tags("land the thing @requires:shem and then some"),
            "land the thing and then some"
        );
        assert_eq!(strip_requires_tags("no tags here"), "no tags here");
        assert_eq!(requires_tags("@requires:a @requires:b-c"), vec!["a", "b-c"]);
    }

    #[test]
    fn a_gap_inherits_the_doc_default_only_when_it_declares_nothing() {
        let doc = vec!["household-nodes".to_string()];
        assert_eq!(resolved_requires_env(&[], &doc), doc);
        assert_eq!(
            resolved_requires_env(&["shem".to_string()], &doc),
            vec!["shem"]
        );
    }

    #[test]
    fn only_cluster_tracked_caps_gate() {
        let available = set(&["household-nodes"]);
        let known = set(&["household-nodes", "shem"]);
        // untracked cap (an a2o fixture tag) — not a hardware concern, does not block
        assert!(!gap_blocked(&["doorway".to_string()], &available, &known));
        assert!(!gap_blocked(&[], &available, &known));
        assert!(!gap_blocked(
            &["household-nodes".to_string()],
            &available,
            &known
        ));
        assert!(gap_blocked(&["shem".to_string()], &available, &known));
        // one blocked cap among satisfied ones still blocks
        assert!(gap_blocked(
            &["household-nodes".to_string(), "shem".to_string()],
            &available,
            &known
        ));
    }

    #[test]
    fn the_act_tag_is_read_only_above_the_feature_keyword() {
        assert_eq!(
            feature_act("@act:i @requires:owned-substrate\nFeature: x\n  @act:iii\n").as_deref(),
            Some("i")
        );
        assert_eq!(feature_act("# @act:ii is a comment\nFeature: x\n"), None);
        assert_eq!(
            feature_act("@act:host\nFeature: x\n").as_deref(),
            Some("host")
        );
        assert_eq!(
            feature_act("@act:iii\nFeature: x\n").as_deref(),
            Some("iii")
        );
        assert_eq!(feature_act("Feature: x\n@act:i\n"), None);
        // `\b`: `@act:ii` must not be read out of `@act:iii`
        assert_eq!(feature_act("@act:iii\n").as_deref(), Some("iii"));
    }

    #[test]
    fn a_lane_file_missing_beside_an_override_is_found_at_the_repository_default() {
        use std::path::PathBuf;
        let tmp = tempfile::TempDir::new().unwrap();
        let isolated = tmp.path().join("isolated");
        let repo = tmp.path().join("genesis/manifests");
        std::fs::create_dir_all(&isolated).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("cluster-state.act1-household.yaml"),
            "resources:\n  owned-substrate:\n    available: true\n",
        )
        .unwrap();
        let dirs: Vec<PathBuf> = vec![isolated.clone(), repo.clone()];
        // The isolated directory holds no lane contract; the repository default answers.
        assert!(act_baseline_caps_in("i", &dirs).contains("owned-substrate"));
        // …and a directory that DOES hold one is still the one used.
        std::fs::write(
            isolated.join("cluster-state.act1-household.yaml"),
            "resources:\n  something-else:\n    available: true\n",
        )
        .unwrap();
        let caps = act_baseline_caps_in("i", &dirs);
        assert!(caps.contains("something-else"));
        assert!(!caps.contains("owned-substrate"));
        // No candidate at all is still fail-open, never an invented gate.
        assert!(act_baseline_caps_in("i", &[tmp.path().join("nowhere")]).is_empty());
    }

    #[test]
    fn host_has_no_baseline_file_and_therefore_no_caps() {
        assert_eq!(act_baseline_file("host"), None);
        assert!(act_baseline_caps("host", Path::new("/nonexistent")).is_empty());
        // a missing act file is fail-open, never an invented gate
        assert!(act_baseline_caps("i", Path::new("/nonexistent")).is_empty());
    }
}
