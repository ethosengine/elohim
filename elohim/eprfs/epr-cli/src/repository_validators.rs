//! Elohim repository policy implementations.
//!
//! This is the application composition root for the current repository. The
//! EPRFS crates know only the `ValidatorProvider` contract. A future provider
//! can resolve the same references to content-addressed WASM without changing
//! `.epr-meta` parsing or policy evaluation.

use std::{collections::HashMap, collections::HashSet, fs, path::Path};

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
            "epr:validator-brand-vocabulary-boundary" => brand_vocabulary_boundary(request),
            "epr:validator-sovereignty-ontology-guard" => sovereignty_guard(request),
            "epr:validator-ownership-ontology-guard" => ownership_guard(request),
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

/// Keep opaque project/domain vocabulary at the brand and documentation layer.
///
/// This is deliberately an advisory, net-new-only source lint. Existing literals remain
/// maintainable, but there is no compatibility suppression for new internal role, zome, package,
/// persistence, or wire identifiers while the protocol remains in development.
fn brand_vocabulary_boundary(request: &ValidatorRequest<'_>) -> Option<String> {
    let path = request.write.path.as_str();
    let normalized_path = path.replace('\\', "/").to_ascii_lowercase();
    // Repo-relative and absolute write paths must compare alike, so every segment test runs
    // against a rooted form (mirrors the Python twin's `f"/{normalized_path.lstrip('/')}"`).
    let rooted_path = format!("/{}", normalized_path.trim_start_matches('/'));
    if [
        "/.claude/scripts/_lib/epr_meta.py",
        "/.claude/scripts/_lib/__tests__/brand_vocabulary_guard_test.py",
        "/elohim/eprfs/epr-cli/src/repository_validators.rs",
    ]
    .iter()
    .any(|internal| rooted_path.ends_with(internal))
    {
        return None;
    }
    if rooted_path.contains("/.claude/memory-kit/") {
        return None;
    }
    let base = basename(path).to_ascii_lowercase();
    let suffix = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{}", value.to_ascii_lowercase()))
        .unwrap_or_default();
    if !is_brand_source(&suffix, &base) {
        return None;
    }

    let post = request.write.content.as_deref()?;
    let prior = request.write.prior_content.as_deref().unwrap_or("");
    let added = added_lines(prior, post);
    if added.is_empty() {
        return None;
    }

    let lines: Vec<&str> = post.lines().collect();
    let package_body_is_prose =
        suffix == ".json" && rooted_path.contains("/.epr-meta/elohim/packages/");
    let prose = brand_prose_lines(&lines, &suffix, package_body_is_prose);
    let mut hits = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !added.contains(*line)
            || prose.contains(&index)
            || brand_comment_only(line, &suffix, &base)
        {
            continue;
        }
        let lowered = line.to_lowercase();
        for (term, replacement) in brand_entries() {
            if contains_brand_term(&lowered, term) {
                hits.push(format!(
                    "line {} [{term}] `{}`; prefer {replacement}",
                    index + 1,
                    line.trim().chars().take(120).collect::<String>()
                ));
            }
        }
    }

    (!hits.is_empty()).then(|| {
        format!(
            "net-new brand vocabulary in compilable artifact: {}. Name the capability in symbols, \
             routes, schemas, tables, and wire values; keep brand vocabulary in product/architecture \
             prose. Internal role names, zome names, package ids, discriminators, and wire literals \
             are not compatibility boundaries during development—rename them too. This advisory \
             never blocks. {}",
            path,
            hits.join(" | ")
        )
    })
}

fn brand_entries() -> &'static [(&'static str, &'static str)] {
    &[
        ("imagodei", "identity, presence, or stewardship-of-self"),
        (
            "lamad",
            "learning, teaching, content, path, assessment, or mastery",
        ),
        (
            "avodah",
            "work, service, contribution, project, story, or flow",
        ),
        (
            "qahal",
            "community, collective, assembly, consent, or social governance",
        ),
        (
            "shefa",
            "economy, value flow, mutual credit, resource, or stewardship",
        ),
        (
            "mishpat",
            "governance, judgment, decision, policy, commitment, authority, or recovery quorum",
        ),
    ]
}

fn is_brand_source(suffix: &str, base: &str) -> bool {
    const SUFFIXES: &[&str] = &[
        ".bash", ".c", ".cc", ".cjs", ".cpp", ".cs", ".css", ".go", ".gql", ".graphql", ".groovy",
        ".h", ".hpp", ".html", ".java", ".js", ".json", ".jsonc", ".jsx", ".kt", ".kts", ".mjs",
        ".proto", ".py", ".rs", ".scss", ".sh", ".sql", ".svelte", ".swift", ".toml", ".ts",
        ".tsx", ".vue", ".xml", ".yaml", ".yml",
    ];
    const BASENAMES: &[&str] = &["dockerfile", "jenkinsfile", "justfile", "makefile"];
    SUFFIXES.contains(&suffix) || BASENAMES.contains(&base)
}

fn added_lines<'a>(prior: &str, post: &'a str) -> HashSet<&'a str> {
    let mut remaining = HashMap::<&str, usize>::new();
    for line in prior.lines() {
        *remaining.entry(line).or_default() += 1;
    }
    let mut added = HashSet::new();
    for line in post.lines() {
        match remaining.get_mut(line) {
            Some(count) if *count > 0 => *count -= 1,
            _ => {
                added.insert(line);
            }
        }
    }
    added
}

fn brand_comment_only(line: &str, suffix: &str, base: &str) -> bool {
    let stripped = line.trim_start();
    if ["//", "/*", "*", "*/", "<!--", "-->"]
        .iter()
        .any(|prefix| stripped.starts_with(prefix))
    {
        return true;
    }
    if suffix == ".sql" && stripped.starts_with("--") {
        return true;
    }
    matches!(suffix, ".py" | ".sh" | ".bash" | ".yaml" | ".yml" | ".toml")
        .then(|| stripped.starts_with('#'))
        .unwrap_or(false)
        || matches!(base, "dockerfile" | "makefile") && stripped.starts_with('#')
}

fn brand_prose_lines(lines: &[&str], suffix: &str, package_body_is_prose: bool) -> HashSet<usize> {
    let mut prose = HashSet::new();
    let mut block_indent = None;
    for (index, line) in lines.iter().enumerate() {
        let stripped = line.trim();
        let indent = line.len() - line.trim_start().len();
        if let Some(parent_indent) = block_indent {
            if !stripped.is_empty() && indent <= parent_indent {
                block_indent = None;
            } else {
                prose.insert(index);
                continue;
            }
        }
        if let Some(value) = prose_field_value(line, package_body_is_prose) {
            prose.insert(index);
            if matches!(suffix, ".yaml" | ".yml" | ".toml")
                && matches!(value.trim_start().chars().next(), Some('>') | Some('|'))
            {
                block_indent = Some(indent);
            }
        }
    }
    prose
}

fn prose_field_value(line: &str, package_body_is_prose: bool) -> Option<&str> {
    let trimmed = line.trim_start();
    let unquoted = trimmed.strip_prefix(['"', '\'']).unwrap_or(trimmed);
    for field in ["description", "$comment", "why", "purpose", "body"] {
        if field == "body" && !package_body_is_prose {
            continue;
        }
        let Some(rest) = unquoted
            .get(..field.len())
            .filter(|head| head.eq_ignore_ascii_case(field))
        else {
            continue;
        };
        let after = &unquoted[rest.len()..];
        let after = after
            .strip_prefix(['"', '\''])
            .unwrap_or(after)
            .trim_start();
        if let Some(value) = after.strip_prefix(':').or_else(|| after.strip_prefix('=')) {
            return Some(value);
        }
    }
    None
}

/// Word-boundary match of an all-lowercase `term` against an ALREADY-LOWERCASED line. The
/// caller owns the lowercasing so a line is folded once, not once per brand term.
fn contains_brand_term(lowered_line: &str, term: &str) -> bool {
    lowered_line
        .match_indices(term)
        .any(|(index, _)| index == 0 || !lowered_line.as_bytes()[index - 1].is_ascii_alphanumeric())
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

/// Ownership ontology guard — the sibling drift to sovereignty, same shape. OWNERSHIP is the
/// enclosure-flavoured apex the protocol subordinates to STEWARDSHIP and CUSTODY. The phrase list
/// is deliberately narrow (property-flavoured only): English overloads "ownership" for
/// RESPONSIBILITY too ("take full ownership of this bug"), so `full ownership` and `sole ownership`
/// are not members. Either frame marker quiets it — the two frames travel together.
fn ownership_guard(request: &ValidatorRequest<'_>) -> Option<String> {
    const PHRASES: &[&str] = &[
        "data ownership",
        "own your data",
        "owns their data",
        "owns your data",
        "true ownership",
        "outright ownership",
        "ownership rights",
        "ownership of the commons",
        "owns the commons",
    ];
    let post = request
        .write
        .content
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    if post.contains("stewardship-frame:") || post.contains("sovereignty-frame:") {
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
    Some("net-new apex-ownership framing needs an explicit custody/stewardship frame".into())
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

    fn brand_flag(path: &str, prior: Option<&str>, content: &str) -> Option<String> {
        let dir = TempDir::new().unwrap();
        let rule = dummy_rule();
        let mut write = GovernanceWrite::new(path);
        write.is_new = prior.is_none();
        write.prior_content = prior.map(str::to_string);
        write.content = Some(content.to_string());
        let request = ValidatorRequest {
            repo_root: dir.path(),
            reference: "epr:validator-brand-vocabulary-boundary",
            rule: &rule,
            write: &write,
            cid: None,
            fuel: None,
        };
        brand_vocabulary_boundary(&request)
    }

    #[test]
    fn brand_lint_flags_each_net_new_opaque_symbol() {
        for (term, replacement) in brand_entries() {
            let source = format!("export type {term}Signal = {{ id: string }};");
            let reason = brand_flag("src/signals.ts", None, &source).expect(term);
            assert!(reason.contains(term), "{reason}");
            assert!(reason.contains(replacement), "{reason}");
            assert!(reason.contains("never blocks"), "{reason}");
        }
    }

    #[test]
    fn brand_lint_ignores_docs_comments_and_schema_prose() {
        let term = brand_entries()[5].0;
        assert!(brand_flag("README.md", None, &format!("# About {term}")).is_none());
        assert!(brand_flag(
            "src/client.ts",
            None,
            &format!("// {term} is the project name")
        )
        .is_none());
        assert!(brand_flag(
            "schema.json",
            None,
            &format!("{{\n  \"description\": \"The {term} project\"\n}}"),
        )
        .is_none());

        let package = format!("{{\n  \"body\": \"# About {term}\\n\"\n}}");
        assert!(brand_flag(
            ".epr-meta/elohim/packages/skills/reference.json",
            None,
            &package,
        )
        .is_none());

        let report = format!("{{\"last_changed\": \"src/{term}.rs\"}}");
        assert!(brand_flag(".claude/memory-kit/generated-report.json", None, &report,).is_none());
    }

    #[test]
    fn brand_lint_has_no_internal_compatibility_suppression() {
        let term = brand_entries()[5].0;
        let source = format!(
            "// brand-boundary: {term} — stable Holochain role identifier\n\
             const GOVERNANCE_ROLE: &str = \"{term}\";"
        );
        assert!(brand_flag("src/roles.rs", None, &source).is_some());
    }

    #[test]
    fn brand_lint_does_not_trap_maintenance_of_an_existing_literal() {
        let term = brand_entries()[5].0;
        let prior = format!("const OLD_ROLE: &str = \"{term}\";\n");
        let post = format!("{prior}const TIMEOUT_SECONDS: u64 = 30;\n");
        assert!(brand_flag("src/roles.rs", Some(&prior), &post).is_none());
    }

    #[test]
    fn live_root_policy_resolves_as_a_pinned_nonblocking_advisory() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let term = brand_entries()[5].0;
        let mut write = GovernanceWrite::new("brand-vocabulary-probe.ts");
        write.is_new = true;
        write.content = Some(format!("export type {term}Signal = {{ id: string }};"));

        let evaluation = eprfs_meta::evaluate_path_with(
            &repo_root,
            repo_root.join("brand-vocabulary-probe.ts"),
            &write,
            &ElohimRepositoryValidators,
        )
        .unwrap();

        assert!(
            evaluation
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "policy.pin-mismatch"),
            "{:?}",
            evaluation.diagnostics
        );
        let verdict = evaluation
            .verdicts
            .iter()
            .find(|verdict| verdict.rule_id == "brand-vocabulary-boundary")
            .expect("live root brand policy should fire");
        assert_eq!(verdict.class, GovernanceRuleClass::Inject);
        assert_eq!(
            eprfs_meta::resolve_decision(&evaluation.verdicts).decision,
            "permit"
        );
    }

    fn ownership_flag(prior: Option<&str>, content: &str) -> Option<String> {
        let dir = TempDir::new().unwrap();
        let rule = dummy_rule();
        let mut write = GovernanceWrite::new("genesis/docs/content/elohim-protocol/note.md");
        write.is_new = prior.is_none();
        write.prior_content = prior.map(str::to_string);
        write.content = Some(content.to_string());
        let request = ValidatorRequest {
            repo_root: dir.path(),
            reference: "epr:validator-ownership-ontology-guard",
            rule: &rule,
            write: &write,
            cid: None,
            fuel: None,
        };
        ownership_guard(&request)
    }

    #[test]
    fn ownership_guard_flags_net_new_apex_framing() {
        assert!(ownership_flag(None, "Members get true data ownership.").is_some());
        assert!(ownership_flag(None, "The platform grants ownership rights.").is_some());
        let prior = "Platforms promise true data ownership.\n";
        let post = format!("{prior}Contributors receive outright ownership.\n");
        assert!(ownership_flag(Some(prior), &post).is_some());
    }

    #[test]
    fn ownership_guard_does_not_trap_maintenance_of_existing_framing() {
        let prior = "Platforms promise true data ownership and ownership rights.\n";
        let post = format!("{prior}Custody is stewarded, not held.\n");
        assert!(ownership_flag(Some(prior), &post).is_none());
        // Cleaning the framing out is never trapped either.
        assert!(ownership_flag(Some(prior), "Platforms promise stewardship.\n").is_none());
    }

    #[test]
    fn ownership_guard_passes_responsibility_idiom_and_declared_frames() {
        assert!(ownership_flag(None, "Please take full ownership of this bug.").is_none());
        assert!(ownership_flag(None, "See LIFECYCLE.md for the full ownership matrix.").is_none());
        assert!(ownership_flag(None, "The owner of the file is the steward.").is_none());
        for marker in [
            "stewardship-frame: adversary",
            "sovereignty-frame: adversary",
        ] {
            assert!(
                ownership_flag(
                    None,
                    &format!("{marker}\nPlatforms promise true data ownership.")
                )
                .is_none(),
                "{marker}"
            );
        }
    }

    #[test]
    fn ownership_reference_resolves_in_the_native_provider() {
        let dir = TempDir::new().unwrap();
        let rule = dummy_rule();
        let mut write = GovernanceWrite::new("genesis/docs/content/elohim-protocol/note.md");
        write.is_new = true;
        write.content = Some("Custody is stewarded, never enclosed.".into());
        let request = ValidatorRequest {
            repo_root: dir.path(),
            reference: "epr:validator-ownership-ontology-guard",
            rule: &rule,
            write: &write,
            cid: None,
            fuel: None,
        };
        // Not `Unavailable` — that arm would route every such write to an unconditional `ask`.
        assert!(matches!(
            ElohimRepositoryValidators.evaluate(&request),
            ValidatorOutcome::Pass
        ));
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
            cid: None,
            fuel: None,
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
            cid: None,
            fuel: None,
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
            cid: None,
            fuel: None,
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
