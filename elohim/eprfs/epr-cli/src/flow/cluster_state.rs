//! `genesis/manifests/cluster-state.yaml` — the ONE native parser.
//!
//! A transcription of `.claude/scripts/_lib/cluster_state.py`, which is itself the consolidation of
//! three drifted hand-rolled regex walks (budget reader, mover, focus baseline). The Python module's
//! header records why it is line-based rather than a YAML parse: the file carries multi-line
//! unquoted `note:` scalars that strict YAML rejects. That constraint is a property of the FILE, not
//! of Python, so the native reader inherits it verbatim — a `serde_yaml::from_str` here would refuse
//! the live manifest.
//!
//! Three semantic decisions are ported with their reasons, because each one has cost real work:
//!
//! 1. **A column-0 comment is NOT a block terminator.** Only a real top-level key (`^[A-Za-z]`)
//!    ends the `resources:` block. The narrower `^[A-Za-z#]` reading silently truncates the resource
//!    list at the first section comment — and the mover then `git mv`s every doc below it into
//!    `held/`. A comment is prose, not structure.
//! 2. **A resource must DECLARE `available:` to be an availability CLAIM.** [`ClusterState::available_map`]
//!    (the budget reader's `known` source) omits role-only placeholder blocks: the format has no way
//!    to distinguish "capability whose availability we forgot to state" from "planned capability that
//!    was never a runtime claim", so we refuse to bench work on an assertion nobody made.
//!    [`ClusterState::all_names`] deliberately keeps EVERY name — that is the mover's vocabulary, and
//!    an unknown cap conservatively blocks held→live escape, so demoting a role-only resource to
//!    "unknown" would block more, not less.
//! 3. **A repeated declaration MERGES, never RESETS — per field.** `available:` is TRUE-WINS (any
//!    `available: true` anywhere makes the resource available), `role:` is FIRST-wins, and
//!    `provides_node_types:` UNIONs. True-wins is the permissive direction on purpose: the expensive
//!    error on a scope gate is wrongly benching work, and a parser must never be the thing that takes
//!    an available capability away.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One declared resource block.
#[derive(Debug, Clone, Default)]
pub struct Resource {
    pub name: String,
    /// The raw scalar: `true` / `false` / `degraded`, or empty when no `available:` key was present.
    pub available: String,
    /// Whether an `available:` line was actually present (decision 2).
    pub declared_available: bool,
    pub role: String,
    pub provides_node_types: Vec<String>,
}

impl Resource {
    /// `available == true`. `degraded`, `false` and a missing key are all NOT available — the same
    /// comparison every prior copy made against the unquoted scalars this file actually uses.
    pub fn is_available(&self) -> bool {
        self.available == "true"
    }
}

/// The parsed manifest.
#[derive(Debug, Clone, Default)]
pub struct ClusterState {
    pub resources: BTreeMap<String, Resource>,
    pub updated: String,
    /// Resource names declared more than once. Recorded for any consumer that wants to warn;
    /// nothing gates on it — a parse crash here would break every session.
    pub duplicate_keys: Vec<String>,
}

impl ClusterState {
    /// ALL resource names — the env-availability VOCABULARY. A `requires_env` cap not in this set is
    /// not a hardware-availability concern (it is an a2o fixture tag or a typo).
    pub fn all_names(&self) -> BTreeSet<String> {
        self.resources.keys().cloned().collect()
    }

    /// Names that CLAIM an availability (`available:` present) — the cluster-TRACKED gating set.
    pub fn declared_names(&self) -> BTreeSet<String> {
        self.resources
            .values()
            .filter(|r| r.declared_available)
            .map(|r| r.name.clone())
            .collect()
    }

    /// Names where `available == true`.
    pub fn available_names(&self) -> BTreeSet<String> {
        self.resources
            .values()
            .filter(|r| r.is_available())
            .map(|r| r.name.clone())
            .collect()
    }

    /// `name -> raw available scalar`, for resources that declare one. Kept RAW rather than boolean
    /// so `degraded` and `false` render distinctly instead of collapsing to one unavailable.
    pub fn available_map(&self) -> BTreeMap<String, String> {
        self.resources
            .values()
            .filter(|r| r.declared_available)
            .map(|r| (r.name.clone(), r.available.clone()))
            .collect()
    }

    /// `name -> role`, only for resources that name one. Role is the human-readable WHY; it gates
    /// nothing.
    pub fn roles(&self) -> BTreeMap<String, String> {
        self.resources
            .values()
            .filter(|r| !r.role.is_empty())
            .map(|r| (r.name.clone(), r.role.clone()))
            .collect()
    }

    /// `nodeType -> {resource}` — the deployments arm. Each resource declares which human
    /// `nodeTypes` it can place.
    pub fn provides_map(&self) -> BTreeMap<String, BTreeSet<String>> {
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for r in self.resources.values() {
            for nt in &r.provides_node_types {
                out.entry(nt.clone()).or_default().insert(r.name.clone());
            }
        }
        out
    }
}

fn strip_quotes(value: &str) -> &str {
    value.trim().trim_matches('"').trim_matches('\'')
}

/// Parse `cluster-state.yaml`. A missing file is an EMPTY state, never an error — fail-open, exactly
/// as every prior copy behaved: an absent manifest means "nothing declared available", and a crash
/// here would take out SessionStart.
pub fn load(path: &Path) -> ClusterState {
    let Ok(text) = std::fs::read_to_string(path) else {
        return ClusterState {
            updated: "?".to_string(),
            ..ClusterState::default()
        };
    };
    parse(&text)
}

/// The line walk, split out so a test can drive it without a file.
pub fn parse(text: &str) -> ClusterState {
    let mut resources: BTreeMap<String, Resource> = BTreeMap::new();
    let mut duplicates: Vec<String> = Vec::new();
    let mut updated = "?".to_string();
    let mut in_resources = false;
    let mut cur: Option<String> = None;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("updated:") {
            // `^updated:\s*(\S+)` — the first token only, so a trailing comment never becomes
            // part of the stamp the focus header prints.
            if let Some(token) = rest.split_whitespace().next() {
                updated = token.to_string();
            }
        }
        if line.trim_end() == "resources:" && !line.starts_with(' ') {
            in_resources = true;
            cur = None;
            continue;
        }
        if !in_resources {
            continue;
        }
        // Decision 1: only a real top-level KEY ends the block. A column-0 `#` comment does not.
        if line.starts_with(|c: char| c.is_ascii_alphabetic()) {
            in_resources = false;
            cur = None;
            continue;
        }
        if let Some(name) = resource_key(line) {
            if resources.contains_key(&name) {
                duplicates.push(name.clone()); // decision 3: MERGE into the record, never reset
            } else {
                resources.insert(
                    name.clone(),
                    Resource {
                        name: name.clone(),
                        ..Resource::default()
                    },
                );
            }
            cur = Some(name);
            continue;
        }
        let Some(name) = cur.clone() else { continue };
        let Some(rec) = resources.get_mut(&name) else {
            continue;
        };
        if let Some(raw) = field(line, "available") {
            // Decision 3: TRUE-WINS. Any `available: true` for a resource wins, whichever block or
            // line it sits on; otherwise the FIRST non-true scalar stands, so `degraded` is not
            // clobbered by a later `false`.
            let value = strip_quotes(raw.split_whitespace().next().unwrap_or("")).to_string();
            if !rec.declared_available || (value == "true" && rec.available != "true") {
                rec.available = value;
            }
            rec.declared_available = true;
            continue;
        }
        if let Some(raw) = field(line, "role") {
            if rec.role.is_empty() {
                // FIRST explicit wins. Inline `#` comments are stripped: role is prose.
                rec.role = raw.split('#').next().unwrap_or("").trim().to_string();
            }
            continue;
        }
        if let Some(raw) = field(line, "provides_node_types") {
            let inner = raw.trim();
            let Some(inner) = inner.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
                continue;
            };
            for nt in inner.split(',') {
                let nt = strip_quotes(nt);
                if !nt.is_empty() && !rec.provides_node_types.iter().any(|x| x == nt) {
                    rec.provides_node_types.push(nt.to_string()); // UNION, order-preserving
                }
            }
            continue;
        }
    }

    ClusterState {
        resources,
        updated,
        duplicate_keys: duplicates,
    }
}

/// `^  ([A-Za-z0-9_-]+):\s*$` — a key at exactly two spaces of indent, with nothing but optional
/// TRAILING WHITESPACE after the colon.
///
/// The `\s*$` is load-bearing and was missing once. A resource key line carrying a single trailing
/// space stopped being a resource: the block's name vanished from the vocabulary AND its
/// `available: true` merged into the PRECEDING resource's record under the true-wins rule, handing
/// an unavailable capability an availability it never declared. On the live manifest that turned
/// `3 to hold` into `7 to hold` with a different move set — which `--apply` would then `git mv` and
/// write `deployments.json` for. The colon must still sit IMMEDIATELY after the name (`  shem :` is
/// not a key), which is why the line is trimmed before the suffix is taken rather than after.
fn resource_key(line: &str) -> Option<String> {
    let rest = line.strip_prefix("  ")?;
    if rest.starts_with(' ') {
        return None;
    }
    let name = rest.trim_end().strip_suffix(':')?;
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some(name.to_string())
}

/// `^    <key>:\s*(\S...)` — a field at exactly four spaces of indent, carrying a NON-EMPTY value.
///
/// Emptiness is a refusal, not an empty string. The kit's patterns all require at least one
/// non-space character (`(\S+)` for `available:`, `(.+?)` for `role:`), so a bare `    available:`
/// declares nothing — and treating it as a declaration would set `declared_available` on a resource
/// that never claimed one, which is precisely the "block with just `role:` is a placeholder"
/// distinction decision 2 exists to keep.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix("    ")?;
    if rest.starts_with(' ') {
        return None;
    }
    let value = rest.strip_prefix(key)?.strip_prefix(':')?.trim_end();
    (!value.trim().is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "updated: 2026-06-04\nresources:\n  shem:\n    role: remote compute\n    available: false\n    provides_node_types: [remote, cloud]\n# ---- a section comment at column zero ----\n  household-nodes:\n    available: true\n    provides_node_types: [household]\n  planned-thing:\n    role: not a runtime claim\nnotes:\n  decoy:\n    available: true\n";

    #[test]
    fn a_column_zero_comment_does_not_truncate_the_resource_block() {
        let state = parse(SAMPLE);
        // decision 1: household-nodes sits BELOW the comment and must still be read.
        assert!(state.all_names().contains("household-nodes"));
        assert!(state.available_names().contains("household-nodes"));
    }

    #[test]
    fn a_top_level_key_does_end_the_block() {
        let state = parse(SAMPLE);
        // `notes:` is a real top-level key; its nested `decoy:` is not a resource.
        assert!(!state.all_names().contains("decoy"));
    }

    #[test]
    fn a_role_only_block_is_vocabulary_but_not_an_availability_claim() {
        let state = parse(SAMPLE);
        // decision 2: the mover knows the name; the budget reader does not gate on it.
        assert!(state.all_names().contains("planned-thing"));
        assert!(!state.declared_names().contains("planned-thing"));
        assert!(!state.available_map().contains_key("planned-thing"));
    }

    #[test]
    fn availability_is_true_wins_across_repeated_blocks() {
        // decision 3: `true` first, then a non-true scalar — the `true` STANDS.
        let state =
            parse("resources:\n  shem:\n    available: true\n  shem:\n    available: false\n");
        assert!(state.available_names().contains("shem"));
        assert_eq!(state.duplicate_keys, vec!["shem".to_string()]);
        // …and the reverse ordering, which is the one that actually happens.
        let state =
            parse("resources:\n  shem:\n    available: false\n  shem:\n    available: true\n");
        assert!(state.available_names().contains("shem"));
    }

    #[test]
    fn a_non_true_scalar_is_kept_raw_rather_than_collapsed() {
        let state = parse("resources:\n  x:\n    available: degraded\n");
        assert_eq!(
            state.available_map().get("x").map(String::as_str),
            Some("degraded")
        );
        assert!(!state.available_names().contains("x"));
    }

    #[test]
    fn provides_node_types_union_and_dedupe() {
        let state = parse(
            "resources:\n  a:\n    provides_node_types: [x, y]\n  a:\n    provides_node_types: [y, z]\n",
        );
        let map = state.provides_map();
        assert!(map.get("x").unwrap().contains("a"));
        assert!(map.get("z").unwrap().contains("a"));
        assert_eq!(
            state.resources["a"].provides_node_types,
            vec!["x", "y", "z"]
        );
    }

    #[test]
    fn a_key_line_tolerates_trailing_whitespace() {
        // The `\s*$` arm. Without it `  shem: ` is not a resource, and its `available: true` merges
        // into the block ABOVE it under true-wins — handing an unavailable capability an
        // availability nobody declared.
        let state = parse(
            "resources:\n  local-conductor:\n    available: false\n  shem: \n    available: true\n",
        );
        assert!(state.all_names().contains("shem"));
        assert!(state.available_names().contains("shem"));
        assert!(
            !state.available_names().contains("local-conductor"),
            "the trailing-space key must not leak its availability into the block above it"
        );
    }

    #[test]
    fn a_space_before_the_colon_is_not_a_key() {
        // The colon sits IMMEDIATELY after the name in the kit's pattern.
        let state = parse("resources:\n  shem :\n    available: true\n");
        assert!(!state.all_names().contains("shem"));
    }

    #[test]
    fn a_bare_field_declares_nothing() {
        let state = parse("resources:\n  x:\n    available:\n    role:\n");
        assert!(state.all_names().contains("x"));
        assert!(
            !state.declared_names().contains("x"),
            "a bare `available:` is not an availability claim"
        );
        assert!(!state.roles().contains_key("x"));
        // …and a later real value in the same block is still read.
        let state = parse("resources:\n  x:\n    available:\n    role: the real one\n");
        assert_eq!(
            state.roles().get("x").map(String::as_str),
            Some("the real one")
        );
    }

    #[test]
    fn a_missing_file_is_an_empty_state_not_an_error() {
        let state = load(Path::new("/nonexistent/cluster-state.yaml"));
        assert!(state.all_names().is_empty());
        assert_eq!(state.updated, "?");
    }
}
