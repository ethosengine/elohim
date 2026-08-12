//! Manifest integrity — the rule-INTEGRITY half of governance resolution.
//!
//! `resolve_path` parses a manifest into typed structures with serde, which
//! silently ignores keys it does not know. That is the right behaviour for
//! forward compatibility and the wrong behaviour for a governance gate: a
//! typo'd predicate key (`require-frontmater:`) parses clean, resolves to an
//! inert rule, and produces no verdict — so a rule the author believed was
//! gating enforces nothing, silently, forever.
//!
//! This module re-reads each cascade manifest's raw frontmatter as a generic
//! YAML value and checks it against the authored schema contract. A manifest
//! that fails is not "invalid input" to be rejected — it is a CEILING EVENT:
//! the gate cannot know what governance intended, so it routes to judgment
//! rather than guessing. The one exception is a write that TARGETS a manifest,
//! because fixing broken governance must never be blocked by broken governance.
//!
//! Parity note. This mirrors `_lib.epr_meta.validate_meta` check-for-check, and
//! deliberately mirrors ALL of it rather than only the checks the golden vectors
//! exercise. A partial integrity layer would agree with Python on the vectors
//! and diverge everywhere else — a fork that passes its own tests, which is the
//! precise failure mode this corpus exists to catch.

use std::path::{Path, PathBuf};

use serde_yaml::Value;

use crate::{collect_cascade, frontmatter, MAX_MANIFEST_BYTES};

/// The five enforcement classes a rule may declare.
const ENFORCEMENT_CLASSES: [&str; 5] = ["deny", "ask", "inject", "measure", "dispatch"];

/// A rule of an enforcing class must carry at least one of these, else it fires
/// on nothing. `parameters` is deliberately absent: it is dispatch-only config,
/// legitimately paired with a real predicate, so counting it would trip the
/// more-than-one check on valid rules.
const ACTIONABLE_KEYS: [&str; 7] = [
    "require-frontmatter",
    "allowed-types",
    "route-to",
    "no-new-subdirs",
    "require-sibling",
    "dedupe-of",
    "validator",
];

/// Every key a non-binding rule may carry. Anything else is a typo.
const KNOWN_RULE_KEYS: [&str; 17] = [
    "id",
    "class",
    "why",
    "when",
    "max-files",
    "measure",
    "policy",
    "params",
    "parameters",
    "retire-when",
    "require-frontmatter",
    "allowed-types",
    "route-to",
    "no-new-subdirs",
    "require-sibling",
    "dedupe-of",
    "validator",
];

/// A policy BINDING carries only placement and local variance; the registry
/// policy owns class, predicates, measure defaults, and why.
const BINDING_KEYS: [&str; 5] = ["id", "policy", "params", "when", "why"];

/// A measure rule's `kind` lives inside its `measure:` block. A `rate` cannot
/// forget its denominator.
const MEASURE_KIND_VOCAB: [&str; 3] = ["level", "rate", "ratio"];

/// One manifest and everything wrong with it. Empty `problems` never appears —
/// healthy manifests are omitted entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestProblems {
    pub manifest: PathBuf,
    pub problems: Vec<String>,
}

/// `<id>@<version>` — the version pin is a DECLARED dependency, never recency.
/// Hand-rolled rather than pulling in a regex dependency for one pattern:
/// `^[a-z0-9][a-z0-9-]*@\d+$`.
fn is_policy_ref(text: &str) -> bool {
    let Some((id, version)) = text.split_once('@') else {
        return false;
    };
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return false;
    }
    !version.is_empty() && version.chars().all(|c| c.is_ascii_digit())
}

/// The two `retire-when` shapes that carry no information: an empty condition,
/// and a bare `never` with no reason. Whether the condition is TRUE is not
/// checked here — that is the census's job, and ultimately a human's.
fn retire_when_problems(value: Option<&Value>, label: &str) -> Vec<String> {
    let Some(text) = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
    else {
        return vec![format!(
            "{label}: `retire-when` must be a non-empty condition string — the state of the world under which this intervenor is REMOVED"
        )];
    };
    let bare_never = text.to_ascii_lowercase().starts_with("never")
        && text
            .chars()
            .nth(5)
            .is_none_or(|c| !c.is_alphanumeric() && c != '_')
        && text.split_whitespace().count() < 2;
    if bare_never {
        return vec![format!(
            "{label}: a bare `retire-when: never` is indistinguishable from not having asked — an honest never carries its reason (`never: <why this is a floor>`)"
        )];
    }
    Vec::new()
}

fn mapping_keys(value: &Value) -> Vec<String> {
    value
        .as_mapping()
        .map(|map| {
            map.keys()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get(Value::String(key.to_string()))
}

/// Check one parsed manifest body against the authored schema contract.
/// An empty result means healthy.
pub fn validate_meta(cfg: &Value) -> Vec<String> {
    let mut errs: Vec<String> = Vec::new();
    if cfg.as_mapping().is_none() {
        return vec!["`.epr-meta` is not a mapping".into()];
    }

    match get(cfg, "epr-meta-version").and_then(Value::as_u64) {
        Some(1) => {}
        _ => errs.push("missing/invalid `epr-meta-version` (must be 1)".into()),
    }

    if let Some(covers) = get(cfg, "covers") {
        let text = covers.as_str().unwrap_or("<non-string>");
        if !matches!(text, "subtree" | "dir-only") {
            errs.push(format!(
                "`covers` value `{text}` not in ('subtree', 'dir-only') — a typo here would silently leave the subtree unclaimed"
            ));
        }
    }

    let rules = get(cfg, "rules")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    let mut seen_ids: Vec<String> = Vec::new();

    for (index, rule) in rules.iter().enumerate() {
        if rule.as_mapping().is_none() {
            errs.push(format!("rules[{index}] is not a mapping"));
            continue;
        }
        let rid = get(rule, "id")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();
        if get(rule, "id").is_none() {
            errs.push(format!("rules[{index}] missing `id`"));
        } else if seen_ids.contains(&rid) {
            errs.push(format!(
                "rules[{index}] (`{rid}`) duplicate rule id — ids must be unique within a manifest (a later rule silently overrides an earlier one on merge)"
            ));
        } else {
            seen_ids.push(rid.clone());
        }

        let keys = mapping_keys(rule);

        // Policy BINDING: the registry policy owns semantics; the binding owns
        // placement. Checked and then skipped — class/predicates come from the
        // registry, so the rest of the rule contract does not apply.
        if let Some(policy) = get(rule, "policy") {
            match policy.as_str() {
                Some(text) if is_policy_ref(text) => {}
                other => errs.push(format!(
                    "rules[{index}] (`{rid}`) `policy` must be `<id>@<version>` — the version pin is a declared dependency, never recency — got `{}`",
                    other.unwrap_or("<non-string>")
                )),
            }
            let mut extra: Vec<&String> = keys
                .iter()
                .filter(|key| !BINDING_KEYS.contains(&key.as_str()))
                .collect();
            extra.sort();
            if !extra.is_empty() {
                errs.push(format!(
                    "rules[{index}] (`{rid}`) policy-binding must not redeclare {extra:?} — class/predicates/measure come from the registry policy; local variance goes in `params` / `when`"
                ));
            }
            if let Some(params) = get(rule, "params") {
                if params.as_mapping().is_none() {
                    errs.push(format!(
                        "rules[{index}] (`{rid}`) `params` must be a mapping"
                    ));
                }
            }
            continue;
        }

        let class = get(rule, "class").and_then(Value::as_str);
        match class {
            Some(text) if ENFORCEMENT_CLASSES.contains(&text) => {}
            other => errs.push(format!(
                "rules[{index}] (`{rid}`) class `{}` not in {ENFORCEMENT_CLASSES:?}",
                other.unwrap_or("None")
            )),
        }

        let mut unknown: Vec<&String> = keys
            .iter()
            .filter(|key| !KNOWN_RULE_KEYS.contains(&key.as_str()))
            .collect();
        unknown.sort();
        if !unknown.is_empty() {
            errs.push(format!(
                "rules[{index}] (`{rid}`) unknown key(s) {unknown:?} (typo?)"
            ));
        }

        let actionable: Vec<&str> = ACTIONABLE_KEYS
            .iter()
            .filter(|key| keys.iter().any(|present| present == *key))
            .copied()
            .collect();
        if matches!(class, Some("deny" | "ask" | "inject")) && actionable.is_empty() {
            errs.push(format!(
                "rules[{index}] (`{rid}`) class `{}` has no actionable predicate (require-frontmatter/route-to/no-new-subdirs/…) — it fires on nothing",
                class.unwrap_or("?")
            ));
        }
        if actionable.len() > 1 {
            errs.push(format!(
                "rules[{index}] (`{rid}`) carries multiple actionable predicates {actionable:?} — only one predicate per rule is evaluated (first-match wins); split into separate rules"
            ));
        }

        if class == Some("measure") {
            let measure = get(rule, "measure");
            let kind = measure.and_then(|m| get(m, "kind"));
            match kind.and_then(Value::as_str) {
                None => errs.push(format!(
                    "rules[{index}] (`{rid}`): a measure rule must declare kind: level|rate|ratio (L6) — an undeclared kind is how a rate gets compared to a cumulative level"
                )),
                Some(text) if !MEASURE_KIND_VOCAB.contains(&text) => errs.push(format!(
                    "rules[{index}] (`{rid}`): unknown kind '{text}'; expected one of {MEASURE_KIND_VOCAB:?}"
                )),
                Some("rate")
                    if measure
                        .and_then(|m| get(m, "per"))
                        .and_then(Value::as_str)
                        .is_none() =>
                {
                    errs.push(format!(
                        "rules[{index}] (`{rid}`): kind: rate requires per: (second|minute|hour|day|week|month|year)"
                    ));
                }
                Some(_) => {}
            }
        }

        if keys.iter().any(|key| key == "retire-when") {
            errs.extend(retire_when_problems(
                get(rule, "retire-when"),
                &format!("rules[{index}] (`{rid}`)"),
            ));
        }
    }

    for (index, validator) in get(cfg, "validators")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        if validator.as_mapping().is_none() || get(validator, "ref").is_none() {
            errs.push(format!("validators[{index}] missing `ref`"));
        }
    }

    errs
}

/// Read one manifest's frontmatter as a generic YAML value and validate it.
///
/// Read/parse failures are reported as problems rather than swallowed: a
/// manifest the gate cannot read is exactly as unusable as one that is
/// malformed, and both must route rather than silently permit.
fn problems_for(manifest: &Path) -> Vec<String> {
    let metadata = match std::fs::metadata(manifest) {
        Ok(metadata) => metadata,
        Err(error) => return vec![format!("unreadable: {error}")],
    };
    if metadata.len() > MAX_MANIFEST_BYTES as u64 {
        return vec![format!(
            "exceeds the {}KB manifest size cap",
            MAX_MANIFEST_BYTES / 1024
        )];
    }
    let text = match std::fs::read_to_string(manifest) {
        Ok(text) => text,
        Err(error) => return vec![format!("unreadable: {error}")],
    };
    let Some(body) = frontmatter(&text) else {
        return vec!["empty manifest (no frontmatter)".into()];
    };
    match serde_yaml::from_str::<Value>(body) {
        Ok(cfg) => validate_meta(&cfg),
        Err(error) => vec![format!("unparseable YAML: {error}")],
    }
}

/// Every malformed manifest in `target`'s cascade, nearest-last (cascade order).
pub fn cascade_problems(repo_root: &Path, target: &Path) -> Vec<ManifestProblems> {
    collect_cascade(repo_root, target)
        .into_iter()
        .filter_map(|manifest| {
            let problems = problems_for(&manifest);
            (!problems.is_empty()).then_some(ManifestProblems { manifest, problems })
        })
        .collect()
}

/// True when the write targets a governance manifest itself.
///
/// Editing an `.epr-meta` is never blocked by a malformed `.epr-meta` — that
/// would make broken governance unfixable, which is the one failure mode a
/// fail-closed gate must never have.
pub fn targets_manifest(path: &Path) -> bool {
    match path.file_name().and_then(|name| name.to_str()) {
        Some(crate::MANIFEST_NAME) => true,
        Some(crate::MANIFEST_FILE_NAME) => path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == crate::MANIFEST_NAME),
        _ => false,
    }
}
