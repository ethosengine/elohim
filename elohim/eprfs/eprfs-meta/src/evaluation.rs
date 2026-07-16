//! Pure, side-effect-free evaluation of resolved `.epr-meta` governance.
//!
//! The authored manifest and policy registry remain the source inputs. This
//! module reconstructs concrete rules for version-pinned policy bindings and
//! evaluates one prospective filesystem write. It never publishes, records, or
//! asserts reach; callers decide whether the resulting local evidence is
//! advisory (`check`) or admission-relevant (`ready`).

use std::{collections::BTreeMap, fs, path::Path};

use eprfs_core::{EprMetaResolution, GovernanceRule, GovernanceRuleClass, GovernanceRulePredicate};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{flow_depth_ok, resolve_path, Result, MAX_MANIFEST_BYTES};

const POLICY_REGISTRY_REL: &str = ".claude/epr-meta/policies.yaml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceWrite {
    pub path: String,
    pub content: Option<String>,
    pub prior_content: Option<String>,
    pub is_new: bool,
    pub is_new_subdir: bool,
}

impl GovernanceWrite {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: None,
            prior_content: None,
            is_new: false,
            is_new_subdir: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceVerdict {
    pub class: GovernanceRuleClass,
    pub reason: String,
    pub rule_id: String,
    pub policy_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceEvaluation {
    pub resolution: EprMetaResolution,
    pub rules: Vec<GovernanceRule>,
    pub verdicts: Vec<GovernanceVerdict>,
    pub diagnostics: Vec<PolicyDiagnostic>,
}

/// Host-supplied execution boundary for validator EPRs.
///
/// `eprfs-meta` owns resolution and policy ontology only. It deliberately has
/// no knowledge of repository domains or concrete validator implementations.
pub trait ValidatorProvider {
    fn evaluate(&self, request: &ValidatorRequest<'_>) -> ValidatorOutcome;
}

pub struct ValidatorRequest<'a> {
    pub repo_root: &'a Path,
    pub reference: &'a str,
    pub rule: &'a GovernanceRule,
    pub write: &'a GovernanceWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatorOutcome {
    Pass,
    Flag { reason: String },
    Unavailable,
}

#[derive(Debug, Default)]
pub struct NoValidators;

impl ValidatorProvider for NoValidators {
    fn evaluate(&self, _request: &ValidatorRequest<'_>) -> ValidatorOutcome {
        ValidatorOutcome::Unavailable
    }
}

/// Resolve the target's manifest cascade, expand version-pinned policy
/// bindings, and evaluate the prospective write without side effects.
pub fn evaluate_path(
    repo_root: impl AsRef<Path>,
    target: impl AsRef<Path>,
    write: &GovernanceWrite,
) -> Result<GovernanceEvaluation> {
    evaluate_path_with(repo_root, target, write, &NoValidators)
}

/// Resolve and evaluate with a host-provided validator implementation.
pub fn evaluate_path_with(
    repo_root: impl AsRef<Path>,
    target: impl AsRef<Path>,
    write: &GovernanceWrite,
    validators: &dyn ValidatorProvider,
) -> Result<GovernanceEvaluation> {
    let repo_root = repo_root.as_ref();
    let resolution = resolve_path(repo_root, target)?;
    let (policies, mut diagnostics) = load_policies(repo_root);
    let mut rules = resolution.effective_rules.clone();

    for binding in &resolution.effective_policies {
        match policies.get(&binding.policy) {
            Some(policy) => match policy.expand(binding) {
                Some(rule) => rules.push(rule),
                None => diagnostics.push(PolicyDiagnostic {
                    code: "policy.invalid".into(),
                    message: format!(
                        "policy `{}` has no evaluable predicate and was not applied",
                        binding.policy
                    ),
                }),
            },
            None => diagnostics.push(PolicyDiagnostic {
                code: "policy.unknown".into(),
                message: format!(
                    "rule `{}` binds unknown policy `{}`; it was not applied",
                    binding.id, binding.policy
                ),
            }),
        }
    }

    rules.sort_by(|left, right| left.id.cmp(&right.id));
    let verdicts = rules
        .iter()
        .filter_map(|rule| evaluate_rule(repo_root, rule, write, validators))
        .collect();

    Ok(GovernanceEvaluation {
        resolution,
        rules,
        verdicts,
        diagnostics,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PolicyRegistry {
    #[serde(rename = "epr-meta-policies-version")]
    version: u32,
    #[serde(default)]
    policies: Vec<RegistryPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct RegistryPolicy {
    id: String,
    version: u32,
    class: String,
    scope: Option<Value>,
    why: Option<String>,
    #[serde(rename = "require-frontmatter")]
    require_frontmatter: Option<Value>,
    #[serde(rename = "allowed-types")]
    allowed_types: Option<Value>,
    #[serde(rename = "route-to")]
    route_to: Option<Value>,
    #[serde(rename = "no-new-subdirs")]
    no_new_subdirs: Option<bool>,
    #[serde(rename = "require-sibling")]
    require_sibling: Option<String>,
    #[serde(rename = "dedupe-of")]
    dedupe_of: Option<String>,
    #[serde(rename = "max-files")]
    max_files: Option<Value>,
    measure: Option<Value>,
    validator: Option<String>,
}

impl RegistryPolicy {
    fn key(&self) -> String {
        format!("{}@{}", self.id, self.version)
    }

    fn expand(&self, binding: &eprfs_core::GovernancePolicyBinding) -> Option<GovernanceRule> {
        let (predicate, mut parameters) = self.predicate_and_parameters()?;
        if predicate == GovernanceRulePredicate::Measure {
            merge_object(&mut parameters, &binding.params);
        }

        Some(GovernanceRule {
            id: binding.id.clone(),
            class: rule_class(&self.class),
            when: binding
                .when
                .clone()
                .or_else(|| self.scope.clone())
                .unwrap_or(Value::Null),
            predicate,
            parameters,
            policy_ref: Some(binding.policy.clone()),
            why: binding.why.clone().or_else(|| self.why.clone()),
        })
    }

    fn predicate_and_parameters(&self) -> Option<(GovernanceRulePredicate, Value)> {
        if let Some(value) = self.require_frontmatter.clone() {
            Some((GovernanceRulePredicate::RequireFrontmatter, value))
        } else if let Some(value) = self.allowed_types.clone() {
            Some((GovernanceRulePredicate::AllowedTypes, value))
        } else if let Some(value) = self.route_to.clone() {
            Some((GovernanceRulePredicate::RouteTo, value))
        } else if let Some(value) = self.no_new_subdirs {
            Some((GovernanceRulePredicate::NoNewSubdirs, Value::Bool(value)))
        } else if let Some(value) = self.require_sibling.clone() {
            Some((
                GovernanceRulePredicate::RequireSibling,
                Value::String(value),
            ))
        } else if let Some(value) = self.dedupe_of.clone() {
            Some((GovernanceRulePredicate::DedupeOf, Value::String(value)))
        } else if let Some(value) = self.max_files.clone() {
            Some((GovernanceRulePredicate::MaxFiles, value))
        } else if let Some(value) = self.measure.clone() {
            Some((GovernanceRulePredicate::Measure, value))
        } else {
            self.validator
                .clone()
                .map(|value| (GovernanceRulePredicate::Validator, Value::String(value)))
        }
    }
}

fn load_policies(repo_root: &Path) -> (BTreeMap<String, RegistryPolicy>, Vec<PolicyDiagnostic>) {
    let path = repo_root.join(POLICY_REGISTRY_REL);
    if !path.is_file() {
        return (BTreeMap::new(), Vec::new());
    }

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            return (
                BTreeMap::new(),
                vec![PolicyDiagnostic {
                    code: "policy.registry-read".into(),
                    message: format!("cannot read {}: {error}", path.display()),
                }],
            )
        }
    };
    if text.len() > MAX_MANIFEST_BYTES {
        return (
            BTreeMap::new(),
            vec![PolicyDiagnostic {
                code: "policy.registry-size".into(),
                message: format!(
                    "{} exceeds the {MAX_MANIFEST_BYTES}-byte safety limit",
                    path.display()
                ),
            }],
        );
    }
    if !flow_depth_ok(&text) {
        return (
            BTreeMap::new(),
            vec![PolicyDiagnostic {
                code: "policy.registry-depth".into(),
                message: format!("{} exceeds the YAML nesting safety limit", path.display()),
            }],
        );
    }
    let registry: PolicyRegistry = match serde_yaml::from_str(&text) {
        Ok(registry) => registry,
        Err(error) => {
            return (
                BTreeMap::new(),
                vec![PolicyDiagnostic {
                    code: "policy.registry-parse".into(),
                    message: format!("cannot parse {}: {error}", path.display()),
                }],
            )
        }
    };
    if registry.version != 1 {
        return (
            BTreeMap::new(),
            vec![PolicyDiagnostic {
                code: "policy.registry-version".into(),
                message: format!(
                    "{} has unsupported policy registry version {}",
                    path.display(),
                    registry.version
                ),
            }],
        );
    }

    let mut policies = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for policy in registry.policies {
        let key = policy.key();
        if policy.predicate_and_parameters().is_none() {
            diagnostics.push(PolicyDiagnostic {
                code: "policy.no-predicate".into(),
                message: format!("policy `{key}` has no evaluable predicate"),
            });
            continue;
        }
        if policies.insert(key.clone(), policy).is_some() {
            diagnostics.push(PolicyDiagnostic {
                code: "policy.duplicate".into(),
                message: format!("duplicate policy `{key}`"),
            });
        }
    }
    (policies, diagnostics)
}

fn evaluate_rule(
    repo_root: &Path,
    rule: &GovernanceRule,
    write: &GovernanceWrite,
    validators: &dyn ValidatorProvider,
) -> Option<GovernanceVerdict> {
    if !matches_when(&rule.when, write) {
        return None;
    }

    let why = rule.why.as_deref().unwrap_or("");
    let reason = match rule.predicate {
        GovernanceRulePredicate::RequireFrontmatter => {
            let present = frontmatter_fields(write.content.as_deref());
            let missing: Vec<_> = rule
                .parameters
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|field| !present.contains_key(*field))
                .collect();
            if missing.is_empty() {
                return None;
            }
            format!("missing required frontmatter {missing:?}. {why}")
        }
        GovernanceRulePredicate::RouteTo => {
            let destination = rule
                .parameters
                .get("dest")
                .and_then(Value::as_str)
                .unwrap_or("?");
            format!("{} routes to {destination}. {why}", basename(&write.path))
        }
        GovernanceRulePredicate::NoNewSubdirs => {
            if !write.is_new_subdir || rule.parameters == Value::Bool(false) {
                return None;
            }
            format!("new subdirectories are not allowed here. {why}")
        }
        GovernanceRulePredicate::RequireSibling => {
            let sibling = rule.parameters.as_str().unwrap_or(".epr-meta");
            if !write.is_new_subdir || basename(&write.path) == sibling {
                return None;
            }
            format!("a new subtree must carry its own `{sibling}`. {why}")
        }
        GovernanceRulePredicate::DedupeOf => {
            let path = rule.parameters.as_str().unwrap_or("?");
            format!("this concern already lives at {path}. {why}")
        }
        GovernanceRulePredicate::Validator => {
            let reference = rule.parameters.as_str().unwrap_or("?");
            let request = ValidatorRequest {
                repo_root,
                reference,
                rule,
                write,
            };
            match validators.evaluate(&request) {
                ValidatorOutcome::Pass => return None,
                ValidatorOutcome::Flag { reason } => {
                    format!("validator `{reference}` flagged this write: {reason}. {why}")
                }
                ValidatorOutcome::Unavailable => {
                    return Some(GovernanceVerdict {
                        class: GovernanceRuleClass::Inject,
                        reason: format!(
                            "validator `{reference}` has no provider in this host (advisory). {why}"
                        ),
                        rule_id: rule.id.clone(),
                        policy_ref: rule.policy_ref.clone(),
                    })
                }
            }
        }
        GovernanceRulePredicate::Measure => {
            let content = write.content.as_deref()?;
            let lines = line_count(content);
            let hard = rule.parameters.get("loc-hard").and_then(Value::as_u64);
            let soft = rule.parameters.get("loc-soft").and_then(Value::as_u64);
            if hard.is_some_and(|ceiling| lines >= ceiling) {
                return Some(GovernanceVerdict {
                    class: GovernanceRuleClass::Measure,
                    reason: format!(
                        "`{}` is {lines} lines — at/over the {}-line HARD LoC ceiling. {why}",
                        basename(&write.path),
                        hard.unwrap_or_default()
                    ),
                    rule_id: rule.id.clone(),
                    policy_ref: rule.policy_ref.clone(),
                });
            }
            if soft.is_some_and(|ceiling| lines >= ceiling) {
                return Some(GovernanceVerdict {
                    class: GovernanceRuleClass::Inject,
                    reason: format!(
                        "`{}` is {lines} lines — over the {}-line soft LoC ceiling. {why}",
                        basename(&write.path),
                        soft.unwrap_or_default()
                    ),
                    rule_id: rule.id.clone(),
                    policy_ref: rule.policy_ref.clone(),
                });
            }
            return None;
        }
        // Declared but intentionally inert in v1.
        GovernanceRulePredicate::AllowedTypes
        | GovernanceRulePredicate::MaxFiles
        | GovernanceRulePredicate::Unknown => return None,
    };

    Some(GovernanceVerdict {
        class: rule.class.clone(),
        reason,
        rule_id: rule.id.clone(),
        policy_ref: rule.policy_ref.clone(),
    })
}

fn matches_when(when: &Value, write: &GovernanceWrite) -> bool {
    let Some(when) = when.as_object() else {
        return true;
    };
    if let Some(pattern) = when.get("write").and_then(Value::as_str) {
        if !wildcard_match(pattern, basename(&write.path)) {
            return false;
        }
    }
    if when.get("new").and_then(Value::as_bool) == Some(true) && !write.is_new {
        return false;
    }

    let content = write.content.as_deref().unwrap_or("");
    if let Some(needle) = when.get("contains").and_then(Value::as_str) {
        if !content.contains(needle) {
            return false;
        }
    }
    if let Some(needles) = when.get("contains-any").and_then(Value::as_array) {
        if !needles
            .iter()
            .filter_map(Value::as_str)
            .any(|needle| content.contains(needle))
        {
            return false;
        }
    }
    true
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.to_lowercase().into_bytes();
    let text = text.to_lowercase().into_bytes();
    let (mut p, mut t, mut star, mut checkpoint) = (0, 0, None, 0);

    while t < text.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            p += 1;
            checkpoint = t;
        } else if let Some(star_at) = star {
            p = star_at + 1;
            checkpoint += 1;
            t = checkpoint;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

fn frontmatter_fields(content: Option<&str>) -> Map<String, Value> {
    let Some(content) = content.and_then(|content| content.strip_prefix("---\n")) else {
        return Map::new();
    };
    let Some(end) = content.find("\n---") else {
        return Map::new();
    };
    serde_yaml::from_str::<Value>(&content[..end])
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn line_count(content: &str) -> u64 {
    content.lines().count() as u64
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn merge_object(target: &mut Value, overlay: &Value) {
    let Some(overlay) = overlay.as_object() else {
        return;
    };
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    if let Some(target) = target.as_object_mut() {
        target.extend(overlay.clone());
    }
}

fn rule_class(class: &str) -> GovernanceRuleClass {
    match class {
        "deny" => GovernanceRuleClass::Deny,
        "ask" => GovernanceRuleClass::Ask,
        "measure" => GovernanceRuleClass::Measure,
        "dispatch" => GovernanceRuleClass::Dispatch,
        _ => GovernanceRuleClass::Inject,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn write_root(dir: &TempDir, rules: &str) {
        fs::write(
            dir.path().join(".epr-meta"),
            format!("---\nepr-meta-version: 1\nid: root\nroot: true\nrules:\n{rules}\n---\n"),
        )
        .unwrap();
    }

    #[test]
    fn evaluates_required_frontmatter_from_preserved_parameters() {
        let dir = TempDir::new().unwrap();
        write_root(
            &dir,
            "  - id: born-linked\n    class: deny\n    when: { write: \"*.md\", new: true }\n    require-frontmatter: [id, cites]",
        );
        let target = dir.path().join("new.md");
        let mut write = GovernanceWrite::new("new.md");
        write.is_new = true;
        write.content = Some("---\nid: present\n---\nbody\n".into());

        let evaluation = evaluate_path(dir.path(), &target, &write).unwrap();

        assert_eq!(evaluation.verdicts.len(), 1);
        assert_eq!(evaluation.verdicts[0].class, GovernanceRuleClass::Deny);
        assert!(evaluation.verdicts[0].reason.contains("cites"));
        assert_eq!(
            evaluation.rules[0].parameters,
            serde_json::json!(["id", "cites"])
        );
    }

    #[test]
    fn expands_policy_and_merges_measure_parameters() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".claude/epr-meta")).unwrap();
        fs::write(
            dir.path().join(POLICY_REGISTRY_REL),
            "epr-meta-policies-version: 1\npolicies:\n  - id: loc\n    version: 1\n    class: measure\n    scope: { write: \"*.rs\" }\n    measure: { loc-soft: 2, loc-hard: 5 }\n",
        )
        .unwrap();
        write_root(
            &dir,
            "  - id: local-loc\n    policy: loc@1\n    params: { loc-soft: 1 }",
        );
        let target = dir.path().join("lib.rs");
        let mut write = GovernanceWrite::new("lib.rs");
        write.content = Some("one\ntwo\n".into());

        let evaluation = evaluate_path(dir.path(), &target, &write).unwrap();

        assert_eq!(evaluation.verdicts.len(), 1);
        assert_eq!(evaluation.verdicts[0].class, GovernanceRuleClass::Inject);
        assert_eq!(evaluation.verdicts[0].policy_ref.as_deref(), Some("loc@1"));
    }

    #[test]
    fn check_is_side_effect_free_and_case_insensitive() {
        let dir = TempDir::new().unwrap();
        write_root(
            &dir,
            "  - id: route\n    class: ask\n    when: { write: \"*-plan.md\", new: true }\n    route-to: { dest: plans/ }",
        );
        let target = dir.path().join("NEW-PLAN.MD");
        let mut write = GovernanceWrite::new("NEW-PLAN.MD");
        write.is_new = true;

        let evaluation = evaluate_path(dir.path(), &target, &write).unwrap();

        assert_eq!(evaluation.verdicts.len(), 1);
        assert!(evaluation.verdicts[0].reason.contains("plans/"));
        assert!(!target.exists());
    }
}
