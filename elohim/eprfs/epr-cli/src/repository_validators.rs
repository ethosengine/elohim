//! Elohim repository policy implementations.
//!
//! This is the application composition root for the current repository. The
//! EPRFS crates know only the `ValidatorProvider` contract. A future provider
//! can resolve the same references to content-addressed WASM without changing
//! `.epr-meta` parsing or policy evaluation.

use std::{collections::HashSet, fs, path::Path};

use eprfs_meta::{ValidatorOutcome, ValidatorProvider, ValidatorRequest};
use serde::Deserialize;
use serde_json::Value;

const ARCHETYPE_BUDGETS: &str = "genesis/data/devices/archetype-resource-budgets.json";
const CAPACITY_LEDGER: &str = "genesis/data/rakia/compute-capacity.json";
const DEPLOYMENTS: &str = "genesis/orchestrator/data/deployments.json";

#[derive(Debug, Default)]
pub struct ElohimRepositoryValidators;

impl ValidatorProvider for ElohimRepositoryValidators {
    fn evaluate(&self, request: &ValidatorRequest<'_>) -> ValidatorOutcome {
        let detail = match request.reference {
            "epr:validator-p2p-design-gate" => p2p_design_gate(request),
            "epr:validator-sovereignty-ontology-guard" => sovereignty_guard(request),
            "epr:validator-archetype-resource-alignment" => archetype_resource_alignment(request),
            "epr:validator-test-bench-aggregate-capacity" => test_bench_aggregate_capacity(request),
            "epr:validator-eprfs-meta-domain-neutrality" => eprfs_meta_domain_neutrality(request),
            "epr:validator-escalation-ladder" => escalation_ladder(request),
            _ => return ValidatorOutcome::Unavailable,
        };
        detail.map_or(ValidatorOutcome::Pass, |reason| ValidatorOutcome::Flag {
            reason,
        })
    }
}

fn eprfs_meta_domain_neutrality(request: &ValidatorRequest<'_>) -> Option<String> {
    let marker = ["epr:", "validator-"].concat();
    let count = |content: &str| content.matches(&marker).count();
    let post = request.write.content.as_deref().unwrap_or("");
    let prior = request.write.prior_content.as_deref().unwrap_or("");
    (count(post) > count(prior)).then(|| {
        "net-new concrete validator identity in eprfs-meta; implement it behind ValidatorProvider"
            .into()
    })
}

/// Agency charter — the escalation ladder. Agents may self-grant `measure` /
/// `inject` / `dispatch` freely (observation cannot harm), but authoring or
/// raising a rule to `ask` / `deny` requires a policy pin carrying deliberation
/// provenance — ratified by the deliberating community (today operator+agents;
/// tomorrow a qahal), never agent self-declaration. Ratification completes at
/// the branch rung: the dev-merge acceptance is the CanonizationRef of repo
/// governance.
///
/// Fires on writes to `.epr-meta` governance files: any rule that introduces or
/// raises to `ask`/`deny` (relative to prior content) without binding a
/// `policy:` whose registry row carries `established_by: deliberated-*` (or the
/// legacy `operator-*` form) is flagged. The canonical flag token is
/// `escalation-requires-ratification`, which the host surfaces as a refer.
fn escalation_ladder(request: &ValidatorRequest<'_>) -> Option<String> {
    if !is_epr_meta_file(&request.write.path) {
        return None;
    }
    let post_rules = parse_epr_meta_rules(request.write.content.as_deref());
    let prior_escalated: HashSet<String> =
        parse_epr_meta_rules(request.write.prior_content.as_deref())
            .into_iter()
            .filter(|rule| is_escalation_class(rule.class.as_deref()))
            .map(|rule| rule.id)
            .collect();

    let mut offenders = Vec::new();
    for rule in &post_rules {
        if !is_escalation_class(rule.class.as_deref()) {
            continue;
        }
        // Already at ask/deny in the prior content — not a new escalation.
        if prior_escalated.contains(&rule.id) {
            continue;
        }
        let ratified = rule
            .policy
            .as_deref()
            .and_then(|policy| eprfs_meta::policy_established_by(request.repo_root, policy))
            .is_some_and(|established_by| {
                // Deliberation provenance (the "us" convention, 2026-07-23): `deliberated-*`
                // records the deliberating community (today operator+agents; tomorrow a qahal);
                // ratification completes at dev-merge acceptance. `operator-*` is the legacy
                // pre-convention form on rows predating it.
                established_by.starts_with("operator-")
                    || established_by.starts_with("deliberated-")
            });
        if !ratified {
            offenders.push(format!(
                "{}({})",
                rule.id,
                rule.class.as_deref().unwrap_or("inject")
            ));
        }
    }
    (!offenders.is_empty())
        .then(|| format!("escalation-requires-ratification: {}", offenders.join(", ")))
}

#[derive(Debug, Deserialize)]
struct LadderRule {
    id: String,
    #[serde(default)]
    class: Option<String>,
    #[serde(default)]
    policy: Option<String>,
}

fn parse_epr_meta_rules(content: Option<&str>) -> Vec<LadderRule> {
    let Some(content) = content else {
        return Vec::new();
    };
    #[derive(Debug, Deserialize)]
    struct Doc {
        #[serde(default)]
        rules: Vec<LadderRule>,
    }
    serde_yaml::from_str::<Doc>(frontmatter_or_whole(content))
        .map(|doc| doc.rules)
        .unwrap_or_default()
}

fn frontmatter_or_whole(text: &str) -> &str {
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            return &rest[..end];
        }
    }
    text
}

fn is_escalation_class(class: Option<&str>) -> bool {
    matches!(class, Some("ask") | Some("deny"))
}

fn is_epr_meta_file(path: &str) -> bool {
    let base = basename(path);
    base == ".epr-meta" || (base == "manifest.md" && path.contains(".epr-meta/"))
}

fn p2p_design_gate(request: &ValidatorRequest<'_>) -> Option<String> {
    let content = request.write.content.as_deref().unwrap_or("");
    ["GET /api/v1", "PRIMARY KEY", "uuid"]
        .iter()
        .find(|needle| content.contains(**needle))
        .map(|needle| format!("found conventional-first design marker `{needle}`"))
}

fn sovereignty_guard(request: &ValidatorRequest<'_>) -> Option<String> {
    const PHRASES: &[&str] = &[
        "self-sovereign",
        "self sovereign",
        "self-sovereignty",
        "self sovereignty",
        "true data sovereignty",
        "full data sovereignty",
        "sovereign identity",
        "digital sovereignty",
        "fully sovereign",
    ];
    let post = request
        .write
        .content
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    if post.contains("sovereignty-frame:") {
        return None;
    }
    let count = |text: &str| {
        PHRASES
            .iter()
            .map(|phrase| text.matches(phrase).count())
            .sum::<usize>()
    };
    let post_count = count(&post);
    if post_count == 0 {
        return None;
    }
    let prior = request
        .write
        .prior_content
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    if !request.write.is_new && post_count <= count(&prior) {
        return None;
    }
    Some("net-new apex-sovereignty framing needs an explicit bounded frame".into())
}

fn archetype_resource_alignment(request: &ValidatorRequest<'_>) -> Option<String> {
    if basename(&request.write.path) != "deployments.json" {
        return None;
    }
    let deployments = parse_content(request.write.content.as_deref())?;
    let budgets = read_json(request.repo_root, ARCHETYPE_BUDGETS)?;
    let budgets = budgets.get("budgets")?.as_object()?;
    let mut drift = Vec::new();
    let fields = [
        ("edgenodeCpuRequest", "cpuRequest"),
        ("edgenodeCpuLimit", "cpuLimit"),
        ("edgenodeMemoryRequest", "memoryRequest"),
        ("edgenodeMemoryLimit", "memoryLimit"),
    ];
    for human in deployments.get("humans")?.as_array()? {
        if human.get("pattern").and_then(Value::as_str) != Some("consolidated") {
            continue;
        }
        let archetype = human
            .get("deviceArchetype")
            .and_then(Value::as_str)
            .unwrap_or("?");
        let Some(canonical) = budgets.get(archetype).and_then(Value::as_object) else {
            continue;
        };
        let override_ = human.get("resourceOverride").and_then(Value::as_object);
        let justified = override_
            .and_then(|value| value.get("justification"))
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty());
        for (deployment_key, budget_key) in fields {
            let Some(actual) = human.get(deployment_key) else {
                continue;
            };
            let expected = if justified {
                override_.and_then(|value| value.get(budget_key))
            } else {
                None
            }
            .or_else(|| canonical.get(budget_key));
            if expected.is_some_and(|expected| expected != actual) {
                drift.push(format!(
                    "{}[{archetype}].{budget_key}",
                    human.get("name").and_then(Value::as_str).unwrap_or("?")
                ));
            }
        }
    }
    (!drift.is_empty()).then(|| format!("intra-archetype drift at {}", drift.join(", ")))
}

fn test_bench_aggregate_capacity(request: &ValidatorRequest<'_>) -> Option<String> {
    let (deployments, ledger) = match basename(&request.write.path) {
        "deployments.json" => (
            parse_content(request.write.content.as_deref())?,
            read_json(request.repo_root, CAPACITY_LEDGER)?,
        ),
        "compute-capacity.json" => (
            read_json(request.repo_root, DEPLOYMENTS)?,
            parse_content(request.write.content.as_deref())?,
        ),
        _ => return None,
    };
    let cluster = ledger.get("cluster")?;
    let allocatable = bundle(cluster.get("totalAllocatable"));
    let committed = bundle(cluster.get("totalCommitted"));
    let observed_requests = observed_humans(&ledger, "requests");
    let observed_limits = observed_humans(&ledger, "limits");
    let planned_requests = planned_humans(&deployments, "requests");
    let planned_limits = planned_humans(&deployments, "limits");
    let total_limits = observed_total_limits(&ledger);
    let projected_requests = committed.replace(observed_requests, planned_requests);
    let projected_limits = total_limits.replace(observed_limits, planned_limits);

    let mut violations = Vec::new();
    check_ceiling(
        "requests.cpu_m",
        projected_requests.cpu_m,
        allocatable.cpu_m,
        &mut violations,
    );
    check_ceiling(
        "requests.memory_Mi",
        projected_requests.memory_mi,
        allocatable.memory_mi,
        &mut violations,
    );
    check_ceiling(
        "limits.cpu_m",
        projected_limits.cpu_m,
        allocatable.cpu_m,
        &mut violations,
    );
    check_ceiling(
        "limits.memory_Mi",
        projected_limits.memory_mi,
        allocatable.memory_mi,
        &mut violations,
    );
    (!violations.is_empty()).then(|| violations.join(", "))
}

fn check_ceiling(label: &str, actual: i64, ceiling: i64, violations: &mut Vec<String>) {
    if ceiling > 0 && actual > ceiling {
        violations.push(format!("{label}={actual} > allocatable={ceiling}"));
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Resources {
    cpu_m: i64,
    memory_mi: i64,
}

impl Resources {
    fn add(&mut self, other: Self) {
        self.cpu_m += other.cpu_m;
        self.memory_mi += other.memory_mi;
    }

    fn replace(self, observed: Self, planned: Self) -> Self {
        Self {
            cpu_m: (self.cpu_m - observed.cpu_m).max(0) + planned.cpu_m,
            memory_mi: (self.memory_mi - observed.memory_mi).max(0) + planned.memory_mi,
        }
    }
}

fn planned_humans(deployments: &Value, kind: &str) -> Resources {
    let (cpu_key, memory_key) = if kind == "requests" {
        ("edgenodeCpuRequest", "edgenodeMemoryRequest")
    } else {
        ("edgenodeCpuLimit", "edgenodeMemoryLimit")
    };
    let mut total = Resources::default();
    for human in deployments
        .get("humans")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if human.get("pattern").and_then(Value::as_str) != Some("consolidated")
            || human.get("suspended").and_then(Value::as_bool) == Some(true)
        {
            continue;
        }
        total.add(Resources {
            cpu_m: cpu_m(human.get(cpu_key)).unwrap_or(0),
            memory_mi: memory_mi(human.get(memory_key)).unwrap_or(0),
        });
    }
    total
}

fn observed_humans(ledger: &Value, kind: &str) -> Resources {
    let mut total = Resources::default();
    for human in ledger
        .get("elohimHumans")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|humans| humans.values())
    {
        if human.get("suspended").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        for deployment in human
            .get("deployments")
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(|deployments| deployments.values())
        {
            let resource = deployment
                .get(kind)
                .or_else(|| deployment.get(format!("{kind}_sum")));
            total.add(bundle(resource));
        }
    }
    total
}

fn observed_total_limits(ledger: &Value) -> Resources {
    let mut total = Resources::default();
    for node_type in ledger
        .pointer("/cluster/nodeTypes")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|types| types.values())
    {
        for node in node_type
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if node.get("ready").and_then(Value::as_bool) == Some(true) {
                total.add(bundle(node.get("limits")));
            }
        }
    }
    total
}

fn bundle(value: Option<&Value>) -> Resources {
    Resources {
        cpu_m: cpu_m(value.and_then(|value| value.get("cpu_m"))).unwrap_or(0),
        memory_mi: memory_mi(value.and_then(|value| value.get("memory_Mi"))).unwrap_or(0),
    }
}

fn cpu_m(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    if let Some(value) = value.as_i64() {
        return Some(value);
    }
    let text = value.as_str()?;
    text.strip_suffix('m')
        .map(str::parse)
        .transpose()
        .ok()
        .flatten()
        .or_else(|| {
            text.parse::<f64>()
                .ok()
                .map(|cores| (cores * 1000.0) as i64)
        })
}

fn memory_mi(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    if let Some(value) = value.as_i64() {
        return Some(value);
    }
    let text = value.as_str()?;
    [
        ("Ki", 1.0 / 1024.0),
        ("Mi", 1.0),
        ("Gi", 1024.0),
        ("Ti", 1024.0 * 1024.0),
    ]
    .into_iter()
    .find_map(|(suffix, factor)| {
        text.strip_suffix(suffix)
            .and_then(|number| number.parse::<f64>().ok())
            .map(|number| (number * factor) as i64)
    })
    .or_else(|| text.parse().ok())
}

fn parse_content(content: Option<&str>) -> Option<Value> {
    serde_json::from_str(content?).ok()
}

fn read_json(repo_root: &Path, relative: &str) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(repo_root.join(relative)).ok()?).ok()
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    use eprfs_core::{GovernanceRule, GovernanceRuleClass, GovernanceRulePredicate};
    use eprfs_meta::GovernanceWrite;
    use tempfile::TempDir;

    fn dummy_rule() -> GovernanceRule {
        GovernanceRule {
            id: "ladder".into(),
            class: GovernanceRuleClass::Ask,
            when: Value::Null,
            predicate: GovernanceRulePredicate::Validator,
            parameters: Value::String("epr:validator-escalation-ladder".into()),
            policy_ref: None,
            why: None,
        }
    }

    fn write_charter_registry(dir: &TempDir) {
        fs::create_dir_all(dir.path().join(".claude/epr-meta")).unwrap();
        fs::write(
            dir.path().join(".claude/epr-meta/policies.yaml"),
            "epr-meta-policies-version: 1\npolicies:\n  - id: governance-escalation-ladder\n    version: 1\n    class: ask\n    validator: epr:validator-escalation-ladder\n    established_by: operator-ratification-pending-2026-07-23\n",
        )
        .unwrap();
    }

    #[test]
    fn escalation_ladder_flags_unratified_ask_or_deny() {
        let dir = TempDir::new().unwrap();
        write_charter_registry(&dir);
        let rule = dummy_rule();
        let mut write = GovernanceWrite::new(".claude/epr-meta/.epr-meta");
        write.content = Some(
            "---\nepr-meta-version: 1\nid: local\nrules:\n  - id: bound-ask\n    class: ask\n    policy: governance-escalation-ladder@1\n  - id: bare-deny\n    class: deny\n  - id: obs\n    class: measure\n---\n"
                .into(),
        );
        let request = ValidatorRequest {
            repo_root: dir.path(),
            reference: "epr:validator-escalation-ladder",
            rule: &rule,
            write: &write,
        };

        let flag = escalation_ladder(&request).expect("bare deny must be flagged");
        assert!(flag.starts_with("escalation-requires-ratification"));
        assert!(flag.contains("bare-deny"), "{flag}");
        // The operator-ratified ask is not an offender.
        assert!(!flag.contains("bound-ask"), "{flag}");
    }

    #[test]
    fn escalation_ladder_permits_prior_escalation_and_ratified_pins() {
        let dir = TempDir::new().unwrap();
        write_charter_registry(&dir);
        let rule = dummy_rule();
        let mut write = GovernanceWrite::new(".claude/epr-meta/.epr-meta");
        // bare-deny was already deny in prior content — not a NEW escalation.
        write.prior_content = Some(
            "---\nepr-meta-version: 1\nrules:\n  - id: bare-deny\n    class: deny\n---\n".into(),
        );
        write.content = Some(
            "---\nepr-meta-version: 1\nrules:\n  - id: bound-ask\n    class: ask\n    policy: governance-escalation-ladder@1\n  - id: bare-deny\n    class: deny\n---\n"
                .into(),
        );
        let request = ValidatorRequest {
            repo_root: dir.path(),
            reference: "epr:validator-escalation-ladder",
            rule: &rule,
            write: &write,
        };

        assert!(escalation_ladder(&request).is_none());
    }

    #[test]
    fn escalation_ladder_ignores_non_meta_writes() {
        let dir = TempDir::new().unwrap();
        write_charter_registry(&dir);
        let rule = dummy_rule();
        let mut write = GovernanceWrite::new("docs/note.md");
        write.content = Some("---\nrules:\n  - id: x\n    class: deny\n---\n".into());
        let request = ValidatorRequest {
            repo_root: dir.path(),
            reference: "epr:validator-escalation-ladder",
            rule: &rule,
            write: &write,
        };

        assert!(escalation_ladder(&request).is_none());
    }

    #[test]
    fn aggregate_replaces_observed_humans_before_checking_ceiling() {
        let deployments = serde_json::json!({"humans": [{
            "pattern": "consolidated", "edgenodeCpuRequest": "500m",
            "edgenodeCpuLimit": "1800m", "edgenodeMemoryRequest": "1Gi",
            "edgenodeMemoryLimit": "2Gi"
        }]});
        let ledger = serde_json::json!({
            "cluster": {
                "totalAllocatable": {"cpu_m": 2000, "memory_Mi": 4096},
                "totalCommitted": {"cpu_m": 700, "memory_Mi": 1024},
                "nodeTypes": {"edge": {"nodes": [{"ready": true,
                    "limits": {"cpu_m": 1500, "memory_Mi": 2048}}]}}
            },
            "elohimHumans": {"old": {"suspended": false, "deployments": {"alpha": {
                "requests": {"cpu_m": 200, "memory_Mi": 256},
                "limits": {"cpu_m": 400, "memory_Mi": 512}
            }}}}
        });
        let committed = bundle(ledger.pointer("/cluster/totalCommitted"));
        let projected = committed.replace(
            observed_humans(&ledger, "requests"),
            planned_humans(&deployments, "requests"),
        );
        assert_eq!(projected.cpu_m, 1000);
        assert_eq!(projected.memory_mi, 1792);
    }
}
