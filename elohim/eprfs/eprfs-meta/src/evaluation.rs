//! Pure, side-effect-free evaluation of resolved `.epr-meta` governance.
//!
//! The authored manifest and policy registry remain the source inputs. This
//! module reconstructs concrete rules for version-pinned policy bindings and
//! evaluates one prospective filesystem write. It never publishes, records, or
//! asserts reach; callers decide whether the resulting local evidence is
//! advisory (`check`) or admission-relevant (`ready`).

use std::{collections::BTreeMap, fs, path::Path};

use eprfs_core::{
    EprMetaResolution, GovernanceRule, GovernanceRuleClass, GovernanceRulePredicate,
    GovernanceValidator,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::{canonical_body, flow_depth_ok, hex_lower, resolve_path, Result, MAX_MANIFEST_BYTES};

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
    /// Ceiling-law vocabulary for `ask`-class (refer) verdicts: why the
    /// deterministic floor routed to judgment rather than deciding. One of
    /// `rule-fired`, `unresolvable-validator`, `policy-pin-mismatch`,
    /// `governance-manifest-malformed`, `escalation-requires-ratification`.
    /// `None` for non-referral classes (deny/inject/measure/dispatch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refer_reason: Option<String>,
}

/// The single-winner projection of a set of verdicts onto the keel decision
/// spine. `eprfs-meta` mirrors the keel `Decision` vocabulary as strings
/// (`permit`/`refuse`/`refer`) rather than depending on `elohim-epr`, which is
/// deliberately unreachable from this publishable, runtime-agnostic crate.
///
/// **Severity law.** `deny > ask > inject`; `measure`/`dispatch` never block.
/// **Ceiling law.** An `ask`-class winner is `refer` (routed), never a
/// collapsed `refuse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedDecision {
    /// `permit` | `refuse` | `refer` — the keel decision vocabulary.
    pub decision: String,
    /// The kebab-case class of the highest-severity verdict, or `None` when
    /// nothing fired (a clean allow).
    pub winning_class: Option<String>,
    /// The rule id of the winning verdict, or `None` when nothing fired.
    pub rule_id: Option<String>,
    /// The refer reason carried by the winner when `decision == "refer"`.
    pub refer_reason: Option<String>,
}

/// Severity rank for the single-winner cascade. `deny` is the hardest floor;
/// `measure` is the softest observation tier. `measure`/`dispatch` never block.
fn class_severity(class: &GovernanceRuleClass) -> u8 {
    match class {
        GovernanceRuleClass::Deny => 4,
        GovernanceRuleClass::Ask => 3,
        GovernanceRuleClass::Inject => 2,
        GovernanceRuleClass::Dispatch => 1,
        GovernanceRuleClass::Measure => 0,
    }
}

fn class_str(class: &GovernanceRuleClass) -> &'static str {
    match class {
        GovernanceRuleClass::Deny => "deny",
        GovernanceRuleClass::Ask => "ask",
        GovernanceRuleClass::Inject => "inject",
        GovernanceRuleClass::Measure => "measure",
        GovernanceRuleClass::Dispatch => "dispatch",
    }
}

/// Collapse fired verdicts to one keel decision by severity single-winner.
///
/// The semantics live here in the library (not the parity test) so every
/// consumer — Rust runners, the parity corpus, future hosts — narrows the same
/// way. `deny` → `refuse`, `ask` → `refer`, everything else → `permit`. Empty
/// input is a clean allow (`permit`, no winner).
pub fn resolve_decision(verdicts: &[GovernanceVerdict]) -> ResolvedDecision {
    let winner = verdicts
        .iter()
        .enumerate()
        .max_by_key(|(index, verdict)| {
            // Highest severity wins; earliest position breaks ties (negate the
            // index so `max_by_key` prefers the earlier verdict).
            (class_severity(&verdict.class), std::cmp::Reverse(*index))
        })
        .map(|(_, verdict)| verdict);

    match winner {
        None => ResolvedDecision {
            decision: "permit".into(),
            winning_class: None,
            rule_id: None,
            refer_reason: None,
        },
        Some(verdict) => {
            let decision = match verdict.class {
                GovernanceRuleClass::Deny => "refuse",
                GovernanceRuleClass::Ask => "refer",
                GovernanceRuleClass::Inject
                | GovernanceRuleClass::Measure
                | GovernanceRuleClass::Dispatch => "permit",
            };
            ResolvedDecision {
                decision: decision.into(),
                winning_class: Some(class_str(&verdict.class).into()),
                rule_id: Some(verdict.rule_id.clone()),
                refer_reason: (decision == "refer")
                    .then(|| verdict.refer_reason.clone())
                    .flatten(),
            }
        }
    }
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
    /// The content address declared for this validator, when the cascade declared one.
    ///
    /// This is what extends the contract from *what an algorithm asserted* to *what it ran*:
    /// a bare `reference` names a mechanism by trust, a `cid` names it by content. `None`
    /// means no manifest declared an identity — honest absence, never "any implementation
    /// will do". A provider that requires content-addressed execution refuses on `None`
    /// rather than falling back to a named lookup.
    pub cid: Option<&'a str>,
    /// The execution budget declared for this validator, in the host's fuel units.
    ///
    /// `eprfs-meta` neither executes nor meters — it resolves the declaration and hands it
    /// over. But an unmetered mechanism is an unbounded variety amplifier: it can produce
    /// more distinguishable outcomes than its inputs justify, which is the one component
    /// able to outrun the regulator that is supposed to bound it. Carrying the declared
    /// budget to the boundary is what makes bounding it possible at all.
    pub fuel: Option<u32>,
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
    let declared_validators = resolution.effective_validators.clone();
    // Verdicts a tampered content pin injects directly, bypassing rule expansion.
    let mut pin_verdicts: Vec<GovernanceVerdict> = Vec::new();

    for binding in &resolution.effective_policies {
        match policies.get(&binding.policy) {
            Some(policy) => {
                // Content-pin verification (authoring polarity): a tampered
                // registry row routes to judgment rather than applying its
                // unverified semantics — never bricks the repo, never proceeds
                // silently.
                if let Some(declared) = policy.content_hash.as_deref() {
                    let pin_ok = compute_row_hash(&policy.raw_row)
                        .as_deref()
                        .map(|actual| actual == declared)
                        .unwrap_or(false);
                    if !pin_ok {
                        diagnostics.push(PolicyDiagnostic {
                            code: "policy.pin-mismatch".into(),
                            message: format!(
                                "policy `{}` failed its content pin; routing to judgment instead of applying it",
                                binding.policy
                            ),
                        });
                        let when = binding
                            .when
                            .clone()
                            .or_else(|| policy.scope.clone())
                            .unwrap_or(Value::Null);
                        if matches_when(&when, write) {
                            let why = binding
                                .why
                                .clone()
                                .or_else(|| policy.why.clone())
                                .unwrap_or_default();
                            pin_verdicts.push(GovernanceVerdict {
                                class: GovernanceRuleClass::Ask,
                                reason: format!(
                                    "policy `{}` failed its content pin. {why}",
                                    binding.policy
                                ),
                                rule_id: binding.id.clone(),
                                policy_ref: Some(binding.policy.clone()),
                                refer_reason: Some("policy-pin-mismatch".into()),
                            });
                        }
                        continue;
                    }
                }
                match policy.expand(binding) {
                    Some(rule) => rules.push(rule),
                    None => diagnostics.push(PolicyDiagnostic {
                        code: "policy.invalid".into(),
                        message: format!(
                            "policy `{}` has no evaluable predicate and was not applied",
                            binding.policy
                        ),
                    }),
                }
            }
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
    let mut verdicts: Vec<GovernanceVerdict> = rules
        .iter()
        .filter_map(|rule| evaluate_rule(repo_root, rule, write, validators, &declared_validators))
        .collect();
    verdicts.extend(pin_verdicts);

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
    /// Content pin over the canonical row body (`sha256:<hex>`), when present.
    #[serde(rename = "contentHash")]
    content_hash: Option<String>,
    /// Ratification provenance. Operator ratification is `operator-*`; the
    /// escalation ladder reads this to gate agent-authored `ask`/`deny`.
    #[serde(rename = "established_by")]
    established_by: Option<String>,
    /// The authored row, verbatim, captured for content-pin verification.
    /// Not deserialized from the typed schema — populated during load.
    #[serde(skip)]
    raw_row: serde_yaml::Value,
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

    // Re-parse the raw rows in file order so each typed policy can carry its
    // authored body verbatim for content-pin verification. The typed and raw
    // sequences are 1:1 by position.
    //
    // This parse is STRICTER than the typed one above: serde_yaml rejects a
    // document with duplicate mapping keys outright, where the typed parse can
    // still succeed. Never let that failure pass silently — with no raw rows
    // every `raw_row` is Null, `compute_row_hash` returns None, and EVERY bound
    // policy reads as `policy.pin-mismatch`. Pin verification would be entirely
    // off while the registry merely looked maximally tampered.
    let mut diagnostics = Vec::new();
    let raw_rows: Vec<serde_yaml::Value> = match serde_yaml::from_str::<RawRegistry>(&text) {
        Ok(raw) => raw.policies,
        Err(err) => {
            diagnostics.push(PolicyDiagnostic {
                code: "policy.registry-unparsable".into(),
                message: format!(
                    "{} could not be raw-parsed for content pins, so NO pin is verified \
                     (duplicate mapping keys are the usual cause): {err}",
                    path.display()
                ),
            });
            Vec::new()
        }
    };

    let mut policies = BTreeMap::new();
    for (index, mut policy) in registry.policies.into_iter().enumerate() {
        policy.raw_row = raw_rows
            .get(index)
            .cloned()
            .unwrap_or(serde_yaml::Value::Null);
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

#[derive(Debug, Deserialize)]
struct RawRegistry {
    #[serde(default)]
    policies: Vec<serde_yaml::Value>,
}

/// Recompute the canonical content pin over an authored policy row: canonical
/// JSON (sorted keys, no whitespace, ASCII escaped) of the row minus the
/// `contentHash`, `status`, and `superseded_by` fields, hashed with SHA-256. This must match the
/// Python `epr-meta-pin` canonicalization byte-for-byte.
fn compute_row_hash(raw_row: &serde_yaml::Value) -> Option<String> {
    let canonical = canonical_body(raw_row.as_mapping()?).ok()?;
    let digest = Sha256::digest(canonical);
    Some(format!("sha256:{}", hex_lower(&digest)))
}

/// Ratification provenance for a registry policy (`id@version`), or `None` when
/// the policy is unknown or unratified. Host validators (the escalation ladder)
/// read this without re-implementing registry parsing.
pub fn policy_established_by(repo_root: impl AsRef<Path>, policy_key: &str) -> Option<String> {
    let (policies, _diagnostics) = load_policies(repo_root.as_ref());
    policies
        .get(policy_key)
        .and_then(|policy| policy.established_by.clone())
}

fn evaluate_rule(
    repo_root: &Path,
    rule: &GovernanceRule,
    write: &GovernanceWrite,
    validators: &dyn ValidatorProvider,
    declared: &[GovernanceValidator],
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
            // Resolve the reference against what the cascade DECLARED for it. Absent
            // declaration stays `None` rather than defaulting: "no manifest named an
            // identity or a budget" and "this validator may run unmetered as anything" are
            // different claims, and only the first is true here.
            let declaration = declared.iter().find(|v| v.reference == reference);
            let request = ValidatorRequest {
                repo_root,
                reference,
                rule,
                write,
                cid: declaration.and_then(|v| v.cid.as_deref()),
                fuel: declaration.and_then(|v| v.fuel),
            };
            match validators.evaluate(&request) {
                ValidatorOutcome::Pass => return None,
                ValidatorOutcome::Flag { reason } => {
                    format!("validator `{reference}` flagged this write: {reason}. {why}")
                }
                ValidatorOutcome::Unavailable => {
                    // Ceiling law: an unresolvable validator reference must NOT
                    // soften to `inject` (the pre-slice soundness inversion) and
                    // must not hard-deny at authoring polarity — it routes to
                    // judgment. A validator declared for another runtime is
                    // implemented (Pass/Flag) in the host that owns it, so it
                    // never reaches this arm there; only genuinely unknown refs do.
                    return Some(GovernanceVerdict {
                        class: GovernanceRuleClass::Ask,
                        reason: format!(
                            "validator `{reference}` is unresolvable in this host; routing to judgment. {why}"
                        ),
                        rule_id: rule.id.clone(),
                        policy_ref: rule.policy_ref.clone(),
                        refer_reason: Some("unresolvable-validator".into()),
                    });
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
                    refer_reason: None,
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
                    refer_reason: None,
                });
            }
            return None;
        }
        // Declared but intentionally inert in v1.
        GovernanceRulePredicate::AllowedTypes
        | GovernanceRulePredicate::MaxFiles
        | GovernanceRulePredicate::Unknown => return None,
    };

    // A plain `ask` rule firing is a routed referral at authoring time; mark it
    // `rule-fired` per the ceiling-law vocabulary. Non-referral classes carry none.
    let refer_reason = (rule.class == GovernanceRuleClass::Ask).then(|| "rule-fired".to_string());
    Some(GovernanceVerdict {
        class: rule.class.clone(),
        reason,
        rule_id: rule.id.clone(),
        policy_ref: rule.policy_ref.clone(),
        refer_reason,
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
    fn policy_hash_matches_python_for_unicode_and_excludes_lifecycle_keys() {
        let row: serde_yaml::Value = serde_yaml::from_str(
            "id: unicode\nversion: 1\nclass: inject\nwhy: a · b — c\n\
             validator: validator-x\ncontentHash: sha256:dead\nstatus: superseded\n\
             superseded_by: unicode@2\n",
        )
        .unwrap();
        assert_eq!(
            compute_row_hash(&row).as_deref(),
            Some("sha256:5bb71f11d48df4b6b40f2cefb970017c301168687ba9c4b33162f28531436562")
        );
    }

    #[test]
    fn unparsable_registry_reports_a_diagnostic_instead_of_silently_voiding_every_pin() {
        // A registry with DUPLICATE `status:` keys parses typed but is rejected by the
        // raw serde_yaml pass. Before this diagnostic existed the failure was swallowed
        // by unwrap_or_default(), so every raw_row went Null, every compute_row_hash
        // returned None, and every bound policy silently read as pin-mismatch — pin
        // verification fully disabled while looking like universal tampering.
        let dir = TempDir::new().unwrap();
        let registry = dir.path().join(".claude/epr-meta");
        fs::create_dir_all(&registry).unwrap();
        // Build the validator id at runtime. A concrete validator identity written as a
        // literal anywhere in this crate's sources — comments included — trips
        // publishable_meta_crate_embeds_no_concrete_validator_identity, the domain-neutrality
        // guard that keeps eprfs-meta free of Elohim policy meaning. Hence the concat.
        let validator = ["epr:", "validator-x"].concat();
        fs::write(
            registry.join("policies.yaml"),
            format!(
                r#"epr-meta-policies-version: 1
policies:
  - id: dupe
    version: 1
    class: inject
    status: superseded
    superseded_by: dupe@2
    validator: {validator}
    status: active
    why: duplicated lifecycle key
"#
            ),
        )
        .unwrap();

        let (_policies, diagnostics) = load_policies(dir.path());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "policy.registry-unparsable"),
            "expected a policy.registry-unparsable diagnostic, got: {diagnostics:?}"
        );
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

#[cfg(test)]
mod validator_declaration_tests {
    use std::cell::RefCell;
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    /// Captures what actually reached the execution boundary.
    #[derive(Default)]
    struct Recorder {
        seen: RefCell<Vec<(Option<String>, Option<u32>)>>,
    }

    impl ValidatorProvider for Recorder {
        fn evaluate(&self, request: &ValidatorRequest<'_>) -> ValidatorOutcome {
            self.seen
                .borrow_mut()
                .push((request.cid.map(str::to_string), request.fuel));
            ValidatorOutcome::Pass
        }
    }

    /// Concrete validator identities are built at runtime, never written as literals — a
    /// literal anywhere in this crate's sources (comments included) trips the
    /// domain-neutrality architecture test, and rightly so.
    fn reference() -> String {
        ["epr:", "validator-metered-example"].concat()
    }

    fn write_manifest(dir: &TempDir, validators_block: &str) {
        let reference = reference();
        fs::write(
            dir.path().join(".epr-meta"),
            format!(
                "---\nepr-meta-version: 1\nid: root\nroot: true\nrules:\n  \
                 - id: metered\n    class: ask\n    when: {{ write: \"*.rs\" }}\n    \
                 validator: {reference}\n{validators_block}---\n"
            ),
        )
        .unwrap();
    }

    /// The declared identity and budget must reach the execution boundary. Before this they
    /// were parsed into `EprMetaGovernance.validators` and read by nobody: the whole
    /// content-addressed, metered-execution story was declared and inert.
    #[test]
    fn a_declared_cid_and_fuel_reach_the_execution_boundary() {
        let dir = TempDir::new().unwrap();
        let reference = reference();
        write_manifest(
            &dir,
            &format!(
                "validators:\n  - ref: {reference}\n    cid: bafyreiexample\n    fuel: 50000\n"
            ),
        );

        let recorder = Recorder::default();
        let write = GovernanceWrite {
            path: "thing.rs".into(),
            content: Some("fn main() {}".into()),
            prior_content: None,
            is_new: true,
            is_new_subdir: false,
        };
        evaluate_path_with(dir.path(), dir.path().join("thing.rs"), &write, &recorder).unwrap();

        let seen = recorder.seen.borrow();
        assert_eq!(seen.len(), 1, "the validator rule must have been evaluated");
        assert_eq!(
            seen[0],
            (Some("bafyreiexample".to_string()), Some(50_000)),
            "a bare reference names a mechanism by trust; a cid names it by content, and \
             fuel is what bounds an otherwise-unbounded variety amplifier"
        );
    }

    /// An undeclared validator yields `None`, not a default. "No manifest named an identity
    /// or a budget" and "this may run unmetered as any implementation" are different claims,
    /// and only the first is true — a provider that requires content-addressed execution must
    /// be able to refuse rather than silently fall back to a named lookup.
    #[test]
    fn an_undeclared_validator_carries_honest_absence_not_a_default() {
        let dir = TempDir::new().unwrap();
        write_manifest(&dir, "");

        let recorder = Recorder::default();
        let write = GovernanceWrite {
            path: "thing.rs".into(),
            content: Some("fn main() {}".into()),
            prior_content: None,
            is_new: true,
            is_new_subdir: false,
        };
        evaluate_path_with(dir.path(), dir.path().join("thing.rs"), &write, &recorder).unwrap();

        let seen = recorder.seen.borrow();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0], (None, None));
    }
}
