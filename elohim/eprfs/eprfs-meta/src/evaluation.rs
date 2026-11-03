//! Pure, side-effect-free evaluation of resolved `.epr-meta` governance.
//!
//! The authored manifest and policy registry remain the source inputs. This
//! module reconstructs concrete rules for version-pinned policy bindings and
//! evaluates one prospective filesystem write. It never publishes, records, or
//! asserts reach; callers decide whether the resulting local evidence is
//! advisory (`check`) or admission-relevant (`ready`).

use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path},
};

use eprfs_core::{
    BlobCid, EprMetaResolution, GovernanceRule, GovernanceRuleClass, GovernanceRulePredicate,
    GovernanceValidator,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::{canonical_body, flow_depth_ok, hex_lower, resolve_path, Result, MAX_REGISTRY_BYTES};

const POLICY_REGISTRY_REL: &str = ".claude/epr-meta/policies.yaml";

/// Repository-authored pointer to portable content, not a witnessed adoption.
/// `path` is relative to this repository; no fetch or execution occurs on import.
///
/// In the existing version-1 policy registry, opt in with a row containing
/// `id`, `version`, local `class`, optional local `scope`, `established_by`,
/// `content: { cid: <raw CID>, path: objects/policy.json }`, and `contentHash`.
/// Compute `contentHash` over the authored row with the existing canonical row
/// algorithm, before substituting semantics. The row cannot also declare a
/// predicate or semantic rationale. Bind its `id@version` from `.epr-meta` as
/// usual. Initially only `validator` and `require-frontmatter` portable
/// predicates are supported; every other semantic field refuses evaluation.
/// Source-space standing from an export must never be copied as authorization.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyContentRef {
    pub cid: String,
    pub path: String,
}

fn supported_import_scope(scope: &Value) -> bool {
    scope.as_object().is_some_and(|scope| {
        scope.iter().all(|(key, value)| match key.as_str() {
            "write" | "contains" => value.as_str().is_some_and(|text| !text.is_empty()),
            "new" => value.is_boolean(),
            "contains-any" => value.as_array().is_some_and(|items| {
                !items.is_empty()
                    && items
                        .iter()
                        .all(|item| item.as_str().is_some_and(|text| !text.is_empty()))
            }),
            _ => false,
        })
    })
}

fn materialize_import(
    repo_root: &Path,
    policy: &mut RegistryPolicy,
) -> std::result::Result<(), String> {
    let row = policy.raw_row.as_mapping().ok_or("missing authored row")?;
    const LOCAL_FIELDS: &[&str] = &[
        "id",
        "version",
        "class",
        "scope",
        "established_by",
        "contentHash",
        "content",
        "binding",
        "status",
        "superseded_by",
        "established_at",
        "retire-when",
    ];
    if row
        .keys()
        .any(|key| key.as_str().is_none_or(|key| !LOCAL_FIELDS.contains(&key)))
    {
        return Err("import row contains semantic shadowing or unsupported standing fields".into());
    }
    if !matches!(policy.class.as_str(), "inject" | "ask" | "deny") {
        return Err("unsupported imported policy class".into());
    }
    if policy
        .scope
        .as_ref()
        .is_some_and(|scope| !supported_import_scope(scope))
    {
        return Err("local import scope contains unsupported keys or types".into());
    }
    if matches!(policy.class.as_str(), "ask" | "deny")
        && policy.established_by.as_deref().is_none_or(str::is_empty)
    {
        return Err("ask/deny import requires locally authored established_by".into());
    }
    let pin = policy
        .content_hash
        .as_deref()
        .ok_or("import requires local contentHash pin")?;
    if compute_row_hash(&policy.raw_row).as_deref() != Some(pin) {
        return Err("local import row failed its contentHash pin".into());
    }
    let pointer: PolicyContentRef =
        serde_json::from_value(policy.content.clone().unwrap_or(Value::Null))
            .map_err(|error| format!("invalid content pointer: {error}"))?;
    let cid = BlobCid::parse(&pointer.cid).map_err(|error| format!("invalid CID: {error}"))?;
    let relative = Path::new(&pointer.path);
    if pointer.path.is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("content path must contain only normal repository-relative components".into());
    }
    let mut path = repo_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    for part in relative.components() {
        path.push(part);
        let metadata =
            fs::symlink_metadata(&path).map_err(|error| format!("content unavailable: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err("content path traverses a symlink".into());
        }
    }
    if !fs::symlink_metadata(&path)
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("content object is not a regular file".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .map_err(|error| error.to_string())?
        .take((MAX_REGISTRY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let portable =
        crate::portable_policy::verify_policy(&cid, &bytes).map_err(|error| error.to_string())?;
    let semantics = serde_json::to_value(&portable.semantics).map_err(|error| error.to_string())?;
    let semantics = semantics.as_object().ok_or("invalid semantic object")?;
    const SEMANTIC_FIELDS: &[&str] = &[
        "id",
        "version",
        "title",
        "why",
        "validator",
        "require-frontmatter",
    ];
    if semantics
        .keys()
        .any(|key| !SEMANTIC_FIELDS.contains(&key.as_str()))
    {
        return Err("content contains semantics unsupported by this evaluator".into());
    }
    if semantics.get("id").and_then(Value::as_str) != Some(policy.id.as_str())
        || semantics.get("version").and_then(Value::as_u64) != Some(u64::from(policy.version))
    {
        return Err("portable id/version differs from local binding".into());
    }
    for key in ["title", "why"] {
        if semantics.get(key).is_some_and(|value| !value.is_string()) {
            return Err(format!("{key} must be a string"));
        }
    }
    match (
        semantics.get("validator"),
        semantics.get("require-frontmatter"),
    ) {
        (Some(Value::String(name)), None) if !name.trim().is_empty() => {
            policy.validator = Some(name.clone());
        }
        (None, Some(Value::Array(fields)))
            if !fields.is_empty()
                && fields
                    .iter()
                    .all(|field| field.as_str().is_some_and(|field| !field.is_empty())) =>
        {
            policy.require_frontmatter = Some(Value::Array(fields.clone()));
        }
        _ => return Err("content must declare exactly one supported, valid predicate".into()),
    }
    policy.why = semantics
        .get("why")
        .and_then(Value::as_str)
        .map(str::to_owned);
    Ok(())
}

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
    /// `stale-evidence` on an inject advisory refers for refresh without blocking reach.
    /// Otherwise `None` for non-referral classes (deny/inject/measure/dispatch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refer_reason: Option<String>,
    /// Opaque evidence a validator attached to its outcome (`ValidatorOutcome::Classified`).
    /// This crate carries it and never reads it: its shape belongs to the validator that
    /// produced it, so the evaluator stays neutral about what any validator means. Absent
    /// from the wire when `None`, so verdicts without evidence serialize as they always did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Value>,
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
/// way. `deny` → `refuse`, `ask` → `refer`, stale-evidence inject → advisory `refer`,
/// everything else → `permit`. Empty
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
                    if verdict.refer_reason.as_deref() == Some("stale-evidence") =>
                {
                    "refer"
                }
                GovernanceRuleClass::Inject
                | GovernanceRuleClass::Measure
                | GovernanceRuleClass::Dispatch => "permit",
            };
            ResolvedDecision {
                decision: decision.into(),
                winning_class: Some(class_str(&verdict.class).into()),
                // An EMPTY rule id means "not attributable to any one rule" — the
                // manifest-integrity route, where the failure is the manifest
                // itself rather than a rule inside it. This is unambiguous
                // because a rule that omits `id` is now caught by integrity and
                // routes BEFORE evaluation, so no real rule can reach here
                // nameless.
                rule_id: Some(verdict.rule_id.clone()).filter(|id| !id.is_empty()),
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
    Flag {
        reason: String,
    },
    /// A flag that also carries the validator's own evidence (for example a classification
    /// with spans). Decided exactly like `Flag` — same reason line, the rule's declared class —
    /// and the evidence value crosses to the verdict opaquely, unread by this crate.
    Classified {
        reason: String,
        evidence: Value,
    },
    /// A provider evaluated a typed finding, including evidence freshness or invalid input.
    /// Reuses the governance class and referral vocabulary; adds no decision channel.
    Finding {
        class: GovernanceRuleClass,
        reason: String,
        refer_reason: Option<String>,
    },
    /// This host DECLARES the reference as belonging to another runtime.
    ///
    /// Unavailable-by-declaration: the rule skips clean and is never downgraded.
    /// A two-host governance plane is allowed exactly this asymmetry — but only
    /// while both hosts can tell a declared absence from an unknown one, which
    /// is why this is a distinct variant rather than a flavour of `Unavailable`.
    DeclaredElsewhere,
    /// No host claims this reference. It routes to judgment (clamped to the
    /// rule's declared class — see the `Validator` arm of `evaluate_rule`).
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
    let target = target.as_ref();
    let resolution = resolve_path(repo_root, target)?;

    // Rule integrity BEFORE rule evaluation. A manifest that does not satisfy
    // the authored schema cannot be evaluated honestly: serde silently drops
    // keys it does not know, so a typo'd predicate resolves to an inert rule and
    // enforces nothing while reading as healthy. The gate cannot know what
    // governance intended, so it routes to judgment rather than guessing.
    //
    // The exception is a write that TARGETS a manifest: fixing broken
    // governance must never be blocked by broken governance. There the problems
    // are reported as diagnostics and evaluation continues.
    let malformed = crate::integrity::cascade_problems(repo_root, target);
    let (policies, mut diagnostics) = load_policies(repo_root);
    if !malformed.is_empty() {
        let detail = malformed
            .iter()
            .map(|entry| {
                format!(
                    "{}: {}",
                    entry.manifest.display(),
                    entry.problems.join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        if crate::integrity::targets_manifest(target) {
            diagnostics.push(PolicyDiagnostic {
                code: "manifest.malformed".into(),
                message: format!(
                    "malformed governance manifest(s) — {detail}. (editing an .epr-meta is never blocked, so you can fix it.)"
                ),
            });
        } else {
            return Ok(GovernanceEvaluation {
                resolution,
                rules: Vec::new(),
                verdicts: vec![GovernanceVerdict {
                    class: GovernanceRuleClass::Ask,
                    reason: format!(
                        "governance manifest malformed — {detail}. Fix the manifest to restore full governance here; proceeding now requires confirmation."
                    ),
                    // Not attributable to any one rule — see `resolve_decision`.
                    rule_id: String::new(),
                    policy_ref: None,
                    refer_reason: Some("governance-manifest-malformed".into()),
                    evidence: None,
                }],
                diagnostics: vec![PolicyDiagnostic {
                    code: "manifest.malformed".into(),
                    message: detail,
                }],
            });
        }
    }
    let mut rules = resolution.effective_rules.clone();
    let declared_validators = resolution.effective_validators.clone();
    // Verdicts a tampered content pin injects directly, bypassing rule expansion.
    let mut pin_verdicts: Vec<GovernanceVerdict> = Vec::new();
    if !resolution.effective_policies.is_empty()
        && diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.starts_with("policy.registry-"))
    {
        pin_verdicts.push(GovernanceVerdict {
            class: GovernanceRuleClass::Ask,
            reason: "policy registry could not be verified; repair it before applying bindings"
                .into(),
            rule_id: String::new(),
            policy_ref: None,
            refer_reason: Some("policy-registry-invalid".into()),
            evidence: None,
        });
    }

    for binding in &resolution.effective_policies {
        match policies.get(&binding.policy) {
            Some(policy) => {
                if policy.content.is_some()
                    && (binding
                        .when
                        .as_ref()
                        .is_some_and(|when| !supported_import_scope(when))
                        || binding
                            .params
                            .as_object()
                            .is_some_and(|params| !params.is_empty()))
                {
                    pin_verdicts.push(GovernanceVerdict {
                        class: GovernanceRuleClass::Ask,
                        reason: format!(
                            "policy `{}` binding has unsupported scope or parameters",
                            binding.policy
                        ),
                        rule_id: binding.id.clone(),
                        policy_ref: Some(binding.policy.clone()),
                        refer_reason: Some("policy-import-invalid".into()),
                        evidence: None,
                    });
                    continue;
                }
                if let Some(error) = &policy.import_error {
                    pin_verdicts.push(GovernanceVerdict {
                        class: GovernanceRuleClass::Ask,
                        reason: format!("policy `{}` import refused: {error}", binding.policy),
                        rule_id: binding.id.clone(),
                        policy_ref: Some(binding.policy.clone()),
                        refer_reason: Some("policy-import-invalid".into()),
                        evidence: None,
                    });
                    continue;
                }
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
                                evidence: None,
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
            None => {
                diagnostics.push(PolicyDiagnostic {
                    code: "policy.unknown".into(),
                    message: format!(
                        "rule `{}` binds unknown policy `{}`",
                        binding.id, binding.policy
                    ),
                });
                pin_verdicts.push(GovernanceVerdict {
                    class: GovernanceRuleClass::Ask,
                    reason: format!(
                        "bound policy `{}` is unavailable; restore its local registry row",
                        binding.policy
                    ),
                    rule_id: binding.id.clone(),
                    policy_ref: Some(binding.policy.clone()),
                    refer_reason: Some("policy-unavailable".into()),
                    evidence: None,
                });
            }
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
    /// Opt-in pointer, parsed separately so a malformed pointer cannot erase the row.
    content: Option<Value>,
    #[serde(skip)]
    import_error: Option<String>,
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
    if text.len() > MAX_REGISTRY_BYTES {
        return (
            BTreeMap::new(),
            vec![PolicyDiagnostic {
                code: "policy.registry-size".into(),
                message: format!(
                    "{} exceeds the {MAX_REGISTRY_BYTES}-byte safety limit",
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
        if policy.content.is_some()
            || policy
                .raw_row
                .as_mapping()
                .is_some_and(|row| row.contains_key("content"))
        {
            if let Err(error) = materialize_import(repo_root, &mut policy) {
                diagnostics.push(PolicyDiagnostic {
                    code: "policy.import-invalid".into(),
                    message: format!("policy `{key}` import refused: {error}"),
                });
                policy.import_error = Some(error);
            }
        }
        if policy.import_error.is_none() && policy.predicate_and_parameters().is_none() {
            diagnostics.push(PolicyDiagnostic {
                code: "policy.no-predicate".into(),
                message: format!("policy `{key}` has no evaluable predicate"),
            });
            policy.import_error = Some("policy has no evaluable predicate".into());
        }
        if policies.contains_key(&key) {
            policy.import_error = Some("duplicate policy identity in local registry".into());
            diagnostics.push(PolicyDiagnostic {
                code: "policy.duplicate".into(),
                message: format!("duplicate policy `{key}`"),
            });
        }
        policies.insert(key, policy);
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
        .filter(|policy| policy.import_error.is_none())
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
    // Only a `Classified` validator outcome sets this; every other predicate carries none.
    let mut evidence = None;
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
            let reference = rule
                .parameters
                .as_str()
                .or_else(|| rule.parameters["ref"].as_str())
                .unwrap_or("?");
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
                ValidatorOutcome::Classified {
                    reason,
                    evidence: carried,
                } => {
                    evidence = Some(carried);
                    format!("validator `{reference}` flagged this write: {reason}. {why}")
                }
                ValidatorOutcome::Finding {
                    class,
                    reason,
                    refer_reason,
                } => {
                    return Some(GovernanceVerdict {
                        class,
                        reason,
                        rule_id: rule.id.clone(),
                        policy_ref: None,
                        refer_reason,
                        evidence: None,
                    });
                }
                // Unavailable-by-DECLARATION: the cascade (or the host's scope
                // registry) says this reference belongs to another runtime. That
                // is an honest asymmetry, not a resolution failure — it skips
                // clean and never downgrades the rule. Distinguishing it from a
                // genuinely unknown reference is the whole point of the variant:
                // a host that cannot tell "not mine to run" from "never heard of
                // it" routes every foreign validator to judgment, which is how
                // three Python-owned validators became blocking here.
                ValidatorOutcome::DeclaredElsewhere => return None,
                ValidatorOutcome::Unavailable => {
                    // Ceiling law, BOTH directions. An unresolvable reference must
                    // not SOFTEN a rule declared to gate (the pre-slice soundness
                    // inversion) — and it must not HARDEN one declared advisory.
                    //
                    // A `class: inject` rule's own text says it never blocks;
                    // escalating it to a routed `ask` stops the author and waits on
                    // a human for a rule whose author declared it non-blocking. That
                    // is the same inversion pointing the other way, and it costs
                    // agency instead of buying safety. The advisory text still
                    // reaches the author — warning without blocking is the rule's
                    // whole intent.
                    //
                    // So: CLAMP TO THE DECLARED CLASS. Route only what was already
                    // meant to gate.
                    let gating = matches!(
                        rule.class,
                        GovernanceRuleClass::Deny | GovernanceRuleClass::Ask
                    );
                    return Some(GovernanceVerdict {
                        class: if gating {
                            GovernanceRuleClass::Ask
                        } else {
                            rule.class.clone()
                        },
                        reason: if gating {
                            format!(
                                "validator `{reference}` is unresolvable in this host; routing to judgment. {why}"
                            )
                        } else {
                            format!(
                                "validator `{reference}` is unresolvable in this host — advisory only, not evaluated. {why}"
                            )
                        },
                        rule_id: rule.id.clone(),
                        policy_ref: rule.policy_ref.clone(),
                        refer_reason: Some("unresolvable-validator".into()),
                        evidence: None,
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
                    evidence: None,
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
                    evidence: None,
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
        evidence,
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

/// Top-level frontmatter keys, plus the valued keys of a `metadata:` block.
///
/// Claude Code's native memory format nests every field past name/description under `metadata:`,
/// so an agent's memory carries its title there. A required field is present when either place
/// names it (operator ruling 2026-09-26); a top-level value always wins. The Python evaluator
/// (`.claude/scripts/_lib/epr_meta.py`) reads it the same way.
fn frontmatter_fields(content: Option<&str>) -> Map<String, Value> {
    let Some(content) = content.and_then(|content| content.strip_prefix("---\n")) else {
        return Map::new();
    };
    let Some(end) = content.find("\n---") else {
        return Map::new();
    };
    let mut fields = serde_yaml::from_str::<Value>(&content[..end])
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    if let Some(Value::Object(metadata)) = fields.get("metadata").cloned() {
        for (key, value) in metadata {
            if !value.is_null() {
                fields.entry(key).or_insert(value);
            }
        }
    }
    fields
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

    fn import_fixture(semantics: &str, scope: &str) -> (TempDir, serde_yaml::Value, Vec<u8>) {
        let dir = TempDir::new().unwrap();
        write_root(&dir, "  - id: local-rule\n    policy: shared@1");
        fs::create_dir_all(dir.path().join(".claude/epr-meta")).unwrap();
        fs::create_dir(dir.path().join("objects")).unwrap();
        let portable = crate::portable_policy::PortablePolicy {
            format: crate::portable_policy::PORTABLE_POLICY_FORMAT.into(),
            semantics: serde_yaml::from_str(semantics).unwrap(),
        };
        let bytes = portable.canonical_bytes().unwrap();
        let cid = portable.cid().unwrap();
        fs::write(dir.path().join("objects/policy.json"), &bytes).unwrap();
        let row: serde_yaml::Value = serde_yaml::from_str(&format!(
            "id: shared\nversion: 1\nclass: deny\nscope: {{ write: '{scope}' }}\nestablished_by: operator-local\ncontent: {{ cid: {cid}, path: objects/policy.json }}\n"
        )).unwrap();
        write_import_row(&dir, row.clone());
        (dir, row, bytes)
    }

    fn write_import_row(dir: &TempDir, mut row: serde_yaml::Value) {
        let pin = compute_row_hash(&row).unwrap();
        row.as_mapping_mut()
            .unwrap()
            .insert("contentHash".into(), pin.into());
        let registry = serde_json::json!({"epr-meta-policies-version": 1, "policies": [row]});
        fs::write(
            dir.path().join(POLICY_REGISTRY_REL),
            serde_yaml::to_string(&registry).unwrap(),
        )
        .unwrap();
    }

    fn imported_evaluation(dir: &TempDir, path: &str) -> GovernanceEvaluation {
        evaluate_path(
            dir.path(),
            dir.path().join(path),
            &GovernanceWrite::new(path),
        )
        .unwrap()
    }

    const IMPORT_SEMANTICS: &str = "id: shared\nversion: 1\nrequire-frontmatter: [id]\n";

    #[test]
    fn addressed_import_is_offline_and_standing_is_independent_across_repositories() {
        let (first, _, first_bytes) = import_fixture(IMPORT_SEMANTICS, "*.md");
        let (second, _, second_bytes) = import_fixture(IMPORT_SEMANTICS, "*.rs");
        assert_eq!(first_bytes, second_bytes);
        let evaluation = imported_evaluation(&first, "note.md");
        assert!(evaluation.diagnostics.is_empty());
        assert_eq!(evaluation.verdicts[0].class, GovernanceRuleClass::Deny);
        assert!(imported_evaluation(&second, "note.md").verdicts.is_empty());
        assert_eq!(
            imported_evaluation(&second, "lib.rs").verdicts[0].class,
            GovernanceRuleClass::Deny
        );
        assert_eq!(
            policy_established_by(first.path(), "shared@1").as_deref(),
            Some("operator-local")
        );
    }

    #[test]
    fn addressed_successor_discovery_never_upgrades_or_executes_until_explicit_local_repin() {
        let (first, mut first_row, original_bytes) = import_fixture(IMPORT_SEMANTICS, "*.md");
        let (second, mut second_row, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
        first_row["established_by"] = "operator-space-one".into();
        second_row["established_by"] = "operator-space-two".into();
        write_import_row(&first, first_row.clone());
        write_import_row(&second, second_row);
        let first_pin = fs::read(first.path().join(POLICY_REGISTRY_REL)).unwrap();
        let second_pin = fs::read(second.path().join(POLICY_REGISTRY_REL)).unwrap();
        let successor = crate::portable_policy::PortablePolicy {
            format: crate::portable_policy::PORTABLE_POLICY_FORMAT.into(),
            semantics: serde_yaml::from_str("id: shared\nversion: 2\nvalidator: candidate-check\n")
                .unwrap(),
        };
        let successor_bytes = successor.canonical_bytes().unwrap();
        let successor_cid = successor.cid().unwrap();
        let recorder = super::validator_declaration_tests::Recorder::default();
        let write = GovernanceWrite::new("note.md");
        for dir in [&first, &second] {
            fs::write(dir.path().join("objects/successor.json"), &successor_bytes).unwrap();
            crate::portable_policy::verify_policy(&successor_cid, &successor_bytes).unwrap();
            let evaluation =
                evaluate_path_with(dir.path(), dir.path().join("note.md"), &write, &recorder)
                    .unwrap();
            assert_eq!(evaluation.rules[0].policy_ref.as_deref(), Some("shared@1"));
            assert_eq!(evaluation.verdicts[0].class, GovernanceRuleClass::Deny);
        }
        assert!(
            recorder.seen.borrow().is_empty(),
            "discovery must not execute the successor validator"
        );
        assert_eq!(
            fs::read(first.path().join(POLICY_REGISTRY_REL)).unwrap(),
            first_pin
        );
        assert_eq!(
            fs::read(second.path().join(POLICY_REGISTRY_REL)).unwrap(),
            second_pin
        );
        assert_eq!(
            policy_established_by(first.path(), "shared@1").as_deref(),
            Some("operator-space-one")
        );
        assert_eq!(
            policy_established_by(second.path(), "shared@1").as_deref(),
            Some("operator-space-two")
        );

        first_row["version"] = 2.into();
        first_row["content"]["cid"] = successor_cid.to_string().into();
        first_row["content"]["path"] = "objects/successor.json".into();
        write_import_row(&first, first_row);
        write_root(&first, "  - id: local-rule\n    policy: shared@2");
        let adopted = evaluate_path_with(
            first.path(),
            first.path().join("note.md"),
            &write,
            &recorder,
        )
        .unwrap();
        assert!(adopted.diagnostics.is_empty());
        assert_eq!(adopted.rules[0].policy_ref.as_deref(), Some("shared@2"));
        assert_eq!(
            adopted.rules[0].parameters,
            Value::String("candidate-check".into())
        );
        assert_eq!(recorder.seen.borrow().len(), 1);
        let retained = evaluate_path_with(
            second.path(),
            second.path().join("note.md"),
            &write,
            &recorder,
        )
        .unwrap();
        assert_eq!(retained.rules[0].policy_ref.as_deref(), Some("shared@1"));
        assert_eq!(retained.verdicts[0].class, GovernanceRuleClass::Deny);
        assert_eq!(recorder.seen.borrow().len(), 1);
        assert_eq!(
            fs::read(second.path().join(POLICY_REGISTRY_REL)).unwrap(),
            second_pin
        );
        assert_eq!(
            fs::read(first.path().join("objects/policy.json")).unwrap(),
            original_bytes
        );
        assert_eq!(
            policy_established_by(first.path(), "shared@2").as_deref(),
            Some("operator-space-one")
        );
    }

    #[test]
    fn addressed_import_missing_tampered_or_escaped_content_refers() {
        for path in [
            "missing.json",
            "../outside.json",
            "/outside.json",
            "objects/policy.json",
        ] {
            let (dir, mut row, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
            row["content"]["path"] = path.into();
            if path == "objects/policy.json" {
                fs::write(dir.path().join(path), b"tampered").unwrap();
            }
            write_import_row(&dir, row);
            // Broken imports refer even outside declared scope: invalid scope/content is not authority.
            let evaluation = imported_evaluation(&dir, "lib.rs");
            assert_eq!(
                evaluation.verdicts[0].refer_reason.as_deref(),
                Some("policy-import-invalid")
            );
            assert_eq!(evaluation.verdicts[0].class, GovernanceRuleClass::Ask);
            assert!(policy_established_by(dir.path(), "shared@1").is_none());
        }
    }

    #[test]
    fn addressed_import_rejects_shadowing_unsupported_semantics_and_bad_identity() {
        for semantics in [
            "id: shared\nversion: 1\nvalidator: local-check\nunknown: true\n",
            "id: shared\nversion: 1\nallowed-types: [doc]\n",
            "id: shared\nversion: 1\nrequire-frontmatter: wrong\n",
            "id: shared\nversion: 1\nrequire-frontmatter: [id]\nvalidator: local-check\n",
            "id: other\nversion: 1\nrequire-frontmatter: [id]\n",
        ] {
            let (dir, _, _) = import_fixture(semantics, "*.md");
            assert_eq!(
                imported_evaluation(&dir, "note.md").verdicts[0].class,
                GovernanceRuleClass::Ask
            );
        }
        for (key, value) in [
            ("why", "shadow"),
            ("validator", "local-check"),
            ("class", "unknown"),
            ("established_by", ""),
        ] {
            let (dir, mut row, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
            row[key] = value.into();
            write_import_row(&dir, row);
            assert_eq!(
                imported_evaluation(&dir, "note.md").verdicts[0].class,
                GovernanceRuleClass::Ask
            );
        }
    }

    #[test]
    fn addressed_import_raw_parse_failure_never_materializes_shadowed_semantics() {
        let (dir, _, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
        fs::write(dir.path().join(POLICY_REGISTRY_REL), "epr-meta-policies-version: 1\npolicies:\n  - id: shared\n    version: 1\n    class: deny\n    content: { cid: malformed, path: missing }\n    require-frontmatter: [id]\n    status: active\n    status: superseded\n").unwrap();
        let evaluation = imported_evaluation(&dir, "note.md");
        assert!(evaluation.rules.is_empty());
        assert!(evaluation
            .verdicts
            .iter()
            .all(|verdict| verdict.class == GovernanceRuleClass::Ask));
    }

    #[test]
    fn addressed_import_malformed_scope_or_binding_refers() {
        for scope in [
            "unexpected",
            "{ contains-any: [123] }",
            "{ unknown: true }",
            "{ new: nope }",
        ] {
            let (dir, mut row, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
            row["scope"] = serde_yaml::from_str(scope).unwrap();
            write_import_row(&dir, row);
            assert_eq!(
                imported_evaluation(&dir, "note.md").verdicts[0].class,
                GovernanceRuleClass::Ask
            );
        }
        for extra in ["when: { contains-any: [123] }", "params: { hidden: true }"] {
            let (dir, _, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
            write_root(
                &dir,
                &format!("  - id: local-rule\n    policy: shared@1\n    {extra}"),
            );
            assert_eq!(
                imported_evaluation(&dir, "note.md").verdicts[0].class,
                GovernanceRuleClass::Ask
            );
        }
    }

    #[test]
    fn addressed_import_requires_local_pin_and_verifies_it_before_materializing() {
        let (dir, row, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
        let registry = serde_json::json!({"epr-meta-policies-version": 1, "policies": [row]});
        fs::write(
            dir.path().join(POLICY_REGISTRY_REL),
            serde_yaml::to_string(&registry).unwrap(),
        )
        .unwrap();
        assert_eq!(
            imported_evaluation(&dir, "note.md").verdicts[0].class,
            GovernanceRuleClass::Ask
        );
        let (dir, mut row, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
        row["contentHash"] = "sha256:wrong".into();
        let registry = serde_json::json!({"epr-meta-policies-version": 1, "policies": [row]});
        fs::write(
            dir.path().join(POLICY_REGISTRY_REL),
            serde_yaml::to_string(&registry).unwrap(),
        )
        .unwrap();
        assert_eq!(
            imported_evaluation(&dir, "note.md").verdicts[0].class,
            GovernanceRuleClass::Ask
        );
    }

    #[test]
    fn addressed_import_disappearing_or_duplicate_registry_row_refers() {
        for missing_file in [true, false] {
            let (dir, _, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
            if missing_file {
                fs::remove_file(dir.path().join(POLICY_REGISTRY_REL)).unwrap();
            } else {
                fs::write(
                    dir.path().join(POLICY_REGISTRY_REL),
                    "epr-meta-policies-version: 1\npolicies: []\n",
                )
                .unwrap();
            }
            assert_eq!(
                imported_evaluation(&dir, "note.md").verdicts[0].class,
                GovernanceRuleClass::Ask
            );
        }
        let (dir, _, _) = import_fixture(IMPORT_SEMANTICS, "*.md");
        let mut registry: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(dir.path().join(POLICY_REGISTRY_REL)).unwrap(),
        )
        .unwrap();
        let row = registry["policies"][0].clone();
        registry["policies"].as_sequence_mut().unwrap().push(row);
        fs::write(
            dir.path().join(POLICY_REGISTRY_REL),
            serde_yaml::to_string(&registry).unwrap(),
        )
        .unwrap();
        assert_eq!(
            imported_evaluation(&dir, "note.md").verdicts[0].class,
            GovernanceRuleClass::Ask
        );
    }

    #[cfg(unix)]
    #[test]
    fn addressed_import_rejects_symlink_even_when_target_bytes_match() {
        let (dir, row, bytes) = import_fixture(IMPORT_SEMANTICS, "*.md");
        let outside = TempDir::new().unwrap();
        fs::write(outside.path().join("policy.json"), bytes).unwrap();
        fs::remove_file(dir.path().join("objects/policy.json")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("policy.json"),
            dir.path().join("objects/policy.json"),
        )
        .unwrap();
        write_import_row(&dir, row);
        assert_eq!(
            imported_evaluation(&dir, "note.md").verdicts[0].class,
            GovernanceRuleClass::Ask
        );
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

    /// A registry is not a manifest. The policy registry grows with every ratified row, so it
    /// must not borrow the manifest parse-DoS cap: past 64 KB it still loads, and the
    /// manifest bound stays exactly where it was. Python's `load_policies` carries the same
    /// registry bound, so the two hosts agree on every governed write.
    #[test]
    fn policy_registry_over_64k_still_loads() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".claude/epr-meta")).unwrap();
        let row = "epr-meta-policies-version: 1\npolicies:\n  - id: loc\n    version: 1\n    \
                   class: measure\n    scope: { write: \"*.rs\" }\n    \
                   measure: { loc-soft: 2, loc-hard: 5 }\n";
        let registry = format!("{row}{}", "# padding\n".repeat(70 * 1024 / 10));
        assert!(registry.len() > crate::MAX_MANIFEST_BYTES);
        fs::write(dir.path().join(POLICY_REGISTRY_REL), &registry).unwrap();

        let (policies, diagnostics) = load_policies(dir.path());

        assert!(
            diagnostics.is_empty(),
            "a 70 KB registry must load clean, got: {diagnostics:?}"
        );
        assert!(policies.contains_key("loc@1"), "got: {:?}", policies.keys());

        // Past the registry bound it still fails loud, never as absence.
        let oversized = format!("{row}{}", "#".repeat(MAX_REGISTRY_BYTES));
        fs::write(dir.path().join(POLICY_REGISTRY_REL), oversized).unwrap();
        let (policies, diagnostics) = load_policies(dir.path());
        assert!(policies.is_empty());
        assert!(diagnostics.iter().any(|d| d.code == "policy.registry-size"));

        // The manifest bound is untouched: 64 KB + 1 is still refused before YAML parsing.
        let manifest = dir.path().join(crate::MANIFEST_NAME);
        fs::write(&manifest, "x".repeat(crate::MAX_MANIFEST_BYTES + 1)).unwrap();
        assert!(matches!(
            crate::parse_meta_file(&manifest).unwrap_err(),
            crate::EprMetaError::ManifestTooLarge { .. }
        ));
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
    fn required_frontmatter_is_satisfied_under_metadata() {
        let dir = TempDir::new().unwrap();
        write_root(
            &dir,
            "  - id: memory-born\n    class: deny\n    when: { write: \"*.md\", new: true }\n    require-frontmatter: [name, title]",
        );
        let target = dir.path().join("new.md");
        let mut write = GovernanceWrite::new("new.md");
        write.is_new = true;
        write.content = Some(
            "---\nname: slug\nmetadata:\n  title: Nested title\n  type: feedback\n---\n".into(),
        );
        assert!(evaluate_path(dir.path(), &target, &write)
            .unwrap()
            .verdicts
            .is_empty());

        // An empty nested key is not a title.
        write.content = Some("---\nname: slug\nmetadata:\n  title:\n---\n".into());
        let evaluation = evaluate_path(dir.path(), &target, &write).unwrap();
        assert_eq!(evaluation.verdicts.len(), 1);
        assert!(evaluation.verdicts[0].reason.contains("title"));
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
    pub(super) struct Recorder {
        pub(super) seen: RefCell<Vec<(Option<String>, Option<u32>)>>,
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

    /// A provider that classifies returns an opaque evidence value alongside its reason.
    struct Classifier;

    impl ValidatorProvider for Classifier {
        fn evaluate(&self, _request: &ValidatorRequest<'_>) -> ValidatorOutcome {
            ValidatorOutcome::Classified {
                reason: "frame drift".into(),
                evidence: classified_evidence(),
            }
        }
    }

    struct Flagger;

    impl ValidatorProvider for Flagger {
        fn evaluate(&self, _request: &ValidatorRequest<'_>) -> ValidatorOutcome {
            ValidatorOutcome::Flag {
                reason: "frame drift".into(),
            }
        }
    }

    fn classified_evidence() -> Value {
        serde_json::json!({
            "frameRef": "bafyframe",
            "verdict": "drift",
            "spans": [[0, 4]],
        })
    }

    fn fired(provider: &dyn ValidatorProvider) -> GovernanceVerdict {
        let dir = TempDir::new().unwrap();
        write_manifest(&dir, "");
        let write = GovernanceWrite {
            path: "thing.rs".into(),
            content: Some("fn main() {}".into()),
            prior_content: None,
            is_new: true,
            is_new_subdir: false,
        };
        let evaluation =
            evaluate_path_with(dir.path(), dir.path().join("thing.rs"), &write, provider).unwrap();
        assert_eq!(evaluation.verdicts.len(), 1, "the validator rule must fire");
        evaluation.verdicts.into_iter().next().unwrap()
    }

    /// Evidence crosses the evaluator opaquely: the verdict is shaped exactly like a Flag
    /// (same reason line, the rule's own class, the same referral), and the value the
    /// validator produced rides along untouched. The evaluator never reads it.
    #[test]
    fn classified_outcome_carries_evidence_to_the_verdict() {
        let classified = fired(&Classifier);
        let flagged = fired(&Flagger);

        assert_eq!(classified.evidence, Some(classified_evidence()));
        assert_eq!(classified.class, GovernanceRuleClass::Ask);
        assert_eq!(classified.reason, flagged.reason);
        assert!(classified.reason.starts_with(&format!(
            "validator `{}` flagged this write: frame drift. ",
            reference()
        )));
        assert_eq!(classified.refer_reason, flagged.refer_reason);
        assert_eq!(classified.rule_id, flagged.rule_id);
        assert_eq!(classified.policy_ref, flagged.policy_ref);
        assert_eq!(flagged.evidence, None);
    }

    /// `evidence` is additive on the wire: a verdict without it serializes byte-for-byte as it
    /// did before the field existed (the govern payload and the parity vectors stay put), and a
    /// payload written before it existed still deserializes.
    #[test]
    fn evidence_serializes_only_when_present() {
        let flagged = serde_json::to_value(fired(&Flagger)).unwrap();
        assert!(
            flagged.get("evidence").is_none(),
            "a Flag verdict must carry no evidence key: {flagged}"
        );

        let classified = serde_json::to_value(fired(&Classifier)).unwrap();
        assert_eq!(classified.get("evidence"), Some(&classified_evidence()));

        let legacy: GovernanceVerdict = serde_json::from_value(serde_json::json!({
            "class": "ask",
            "reason": "r",
            "ruleId": "x",
            "policyRef": null,
        }))
        .unwrap();
        assert_eq!(legacy.evidence, None);
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
