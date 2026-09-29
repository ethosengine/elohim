//! Ephemeral work suggestions. Reconciliation remains the acceptance authority.
use std::path::Path;

use cid::Cid;
use elohim_epr_rea::FlowRecord;
use serde::Serialize;

use super::{HabitView, Seals};
use crate::flow::{
    cluster_state, env_scope, gaps, parse_frontmatter, placement, reconciliation::Reconciliation,
    registers::HabitEntry,
};

#[derive(Debug, Serialize)]
pub struct WorkQueue {
    pub method: &'static str,
    pub scope: &'static str,
    pub ranked: Vec<WorkItem>,
    pub blocked: Vec<WorkItem>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkItem {
    pub source_path: Option<String>,
    pub source_cid: String,
    pub intent: Option<String>,
    pub gap_id: Option<String>,
    pub habit: Option<String>,
    pub priority: Option<usize>,
    pub authored_line: Option<usize>,
    pub assertion: Option<String>,
    pub owners: Vec<String>,
    pub action: String,
    pub remaining_action: String,
    pub acceptance: String,
    pub reasons: Vec<String>,
    pub blockers: Vec<String>,
}

pub fn derive(
    root: &Path,
    path: Option<&str>,
    reconciliation: &Reconciliation,
    records: &[(Cid, FlowRecord)],
    habits: &[HabitView],
    register: &[HabitEntry],
    seals: Option<&Seals>,
) -> WorkQueue {
    let mut queue = WorkQueue {
        method: "covenant-readiness-authored-v1",
        scope: "target assertions only; not a repository-wide queue",
        ranked: vec![],
        blocked: vec![],
        unknowns: vec![],
    };
    let source = path.and_then(|p| confined_source(root, p));
    let source_unavailable = path.is_some() && source.is_none();
    if source_unavailable {
        queue.unknowns.push(
            "source path unavailable or outside repository; inspect source before continuing"
                .into(),
        );
    }
    let text = source.and_then(|p| std::fs::read_to_string(p).ok());
    let fm = parse_frontmatter(text.as_deref().unwrap_or(""));
    let doc_env = placement::requires_env_of(&fm);
    let decomposition = path
        .zip(text.as_deref())
        .map(|(p, t)| gaps::decompose(&gaps::slug_for(Path::new(p)), t, doc_env.clone()));
    let cluster = cluster_state::load(&root.join("genesis/manifests/cluster-state.yaml"));
    let available = cluster.available_names();
    let known = cluster.all_names();
    let served = fm
        .scalars
        .get("serves")
        .cloned()
        .or_else(|| habits.first().map(|h| h.id.clone()));
    if register.is_empty() {
        queue
            .unknowns
            .push("no habit declarations available; delivery readiness cannot be inferred".into());
    }
    if reconciliation.assertions.is_empty() {
        queue.unknowns.push("no current projected assertions; inspect source and historical candidates before projecting work".into());
    }
    if !reconciliation.historical_candidates.is_empty() {
        queue.unknowns.push("historical source scopes require explicit reconciliation; their ownership and acceptance are not transferred".into());
    }
    for assertion in &reconciliation.assertions {
        if assertion.acceptance == "accepted" {
            continue;
        }
        let commitments: Vec<_> = assertion
            .commitments
            .iter()
            .filter_map(|e| {
                records.iter().find_map(|(cid, record)| match record {
                    FlowRecord::Commitment(c) if cid.to_string() == e.commitment => Some(c),
                    _ => None,
                })
            })
            .collect();
        let mut owners: Vec<_> = commitments.iter().map(|c| c.provider.0.clone()).collect();
        owners.sort();
        owners.dedup();
        let habit = commitments
            .iter()
            .flat_map(|c| &c.resource_spec.classified_as)
            .find_map(|s| s.strip_prefix("habit:").map(str::to_owned))
            .or_else(|| served.clone());
        let priority = habit
            .as_ref()
            .and_then(|id| register.iter().find(|h| &h.id == id))
            .and_then(|h| h.priority);
        let gap = decomposition.as_ref().and_then(|d| {
            d.items
                .iter()
                .find(|g| Some(&g.id) == assertion.gap_id.as_ref())
        });
        let required = env_scope::resolved_requires_env(
            gap.map(|g| g.requires_env.as_slice()).unwrap_or(&[]),
            &doc_env,
        );
        let mut blockers: Vec<String> = required
            .iter()
            .filter(|cap| !known.contains(*cap))
            .map(|cap| format!("environment prerequisite unknown: {cap}"))
            .collect();
        if source_unavailable {
            blockers.push(
                "source path outside repository or unavailable; revalidation required".into(),
            );
        }
        if env_scope::gap_blocked(&required, &available, &known) {
            blockers.extend(
                required
                    .iter()
                    .filter(|cap| !available.contains(*cap))
                    .map(|cap| format!("environment unavailable or undeclared: {cap}")),
            );
        }
        if let Some(seals) = seals {
            blockers.extend(
                seals
                    .edges
                    .iter()
                    .filter(|e| matches!(e.verdict.as_str(), "stale" | "dangling" | "held"))
                    .map(|e| format!("prerequisite seal {}: {}", e.verdict, e.to)),
            );
        }
        let mut action = if habit
            .as_ref()
            .and_then(|id| register.iter().find(|h| &h.id == id))
            .is_some_and(|h| h.status == "unwired")
            && action_for(assertion, owners.is_empty()) == "claim"
        {
            "write-failing-check"
        } else {
            action_for(assertion, owners.is_empty())
        };
        if action == "independent-acceptance" {
            action = review_action(assertion, records);
        }
        let mut reasons = vec![
            priority
                .map(|p| format!("covenant priority {}", p + 1))
                .unwrap_or_else(|| "habit unranked by covenant".into()),
            "source order then stable identifier; ordering grants no acceptance".into(),
        ];
        if !owners.is_empty() {
            reasons.push("existing owners retain their claims; resume with owner or contribute/review without replacing them".into());
        }
        reasons.extend(assertion.issues.iter().cloned());
        reasons.extend(
            assertion
                .commitments
                .iter()
                .flat_map(|c| c.issues.iter().cloned()),
        );
        let item = WorkItem {
            source_path: path.map(str::to_owned),
            source_cid: reconciliation.scope_cid.clone(),
            intent: assertion.intent.clone(),
            gap_id: assertion.gap_id.clone(),
            habit,
            priority,
            authored_line: gap.map(|g| g.line),
            assertion: gap.map(|g| g.text.clone()),
            owners,
            action: action.into(),
            remaining_action: assertion.remaining_action.clone(),
            acceptance: assertion.acceptance.clone(),
            reasons,
            blockers,
        };
        if item.blockers.is_empty() {
            queue.ranked.push(item);
        } else {
            queue.blocked.push(item);
        }
    }
    for rows in [&mut queue.ranked, &mut queue.blocked] {
        rows.sort_by(|a, b| {
            (
                a.priority.unwrap_or(usize::MAX),
                a.authored_line.unwrap_or(usize::MAX),
                &a.gap_id,
                &a.intent,
            )
                .cmp(&(
                    b.priority.unwrap_or(usize::MAX),
                    b.authored_line.unwrap_or(usize::MAX),
                    &b.gap_id,
                    &b.intent,
                ))
        });
    }
    queue
}

/// Follow only declared habit refs and already-accounted commitments, retaining the
/// caller's record snapshot. This is a bounded view, never a new global work registry.
pub fn for_habit(
    root: &Path,
    scope: &super::HabitScope,
    records: &[(Cid, FlowRecord)],
    habits: &[HabitView],
    register: &[HabitEntry],
) -> WorkQueue {
    use std::collections::BTreeMap;
    let mut sources: BTreeMap<String, (Option<String>, usize)> = BTreeMap::new();
    let labels: crate::flow::Labels =
        std::fs::read_to_string(root.join(".eprfs/status/labels.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
    let mut unknowns = vec![];
    for (order, reference) in scope.refs.iter().take(32).enumerate() {
        match confined_source(root, reference)
            .and_then(|p| crate::flow::body_cid_of_file(&p).map(|cid| (cid, p)))
        {
            Some((cid, p)) => {
                sources.insert(
                    cid.to_string(),
                    (Some(crate::flow::rel_to_root(root, &p)), order),
                );
            }
            None => unknowns.push(format!(
                "habit reference unreadable or outside repository: {reference}"
            )),
        }
    }
    if scope.refs.len() > 32 {
        unknowns.push(format!(
            "{} declared habit references omitted before reading; open a feature directly",
            scope.refs.len() - 32
        ));
    }
    let habit_tag = format!("habit:{}", scope.id);
    for (_, record) in records {
        if let FlowRecord::Commitment(c) = record {
            if c.resource_spec.classified_as.contains(&habit_tag) {
                sources.entry(c.in_scope_of.to_string()).or_insert_with(|| {
                    (labels.get(&c.in_scope_of.to_string()).cloned(), usize::MAX)
                });
            }
        }
    }
    let count = sources.len();
    let mut queue = WorkQueue {
        method: "covenant-readiness-authored-v1",
        scope: "habit-declared references and accounted commitments; at most 32 source scopes",
        ranked: vec![],
        blocked: vec![],
        unknowns,
    };
    let mut sources: Vec<_> = sources.into_iter().collect();
    sources.sort_by(|a, b| (a.1 .1, &a.0).cmp(&(b.1 .1, &b.0)));
    for (cid, (path, _)) in sources.into_iter().take(32) {
        let path = path.and_then(|p| match confined_source(root, &p) {
            Some(confined) => Some(crate::flow::rel_to_root(root, &confined)),
            None => {
                queue.unknowns.push(format!(
                    "source label unavailable or outside repository: {p}"
                ));
                None
            }
        });
        let Ok(cid) = cid.parse::<Cid>() else {
            continue;
        };
        let reconciliation =
            crate::flow::reconciliation::reconcile(root, &cid, path.as_deref(), records);
        let walked = path
            .as_deref()
            .and_then(|p| crate::flow::walk::walk_with_records(root, p, records).ok());
        let seals = walked.map(|w| Seals {
            edges: w.edges.outgoing,
            stale_downstream: w.frontier.stale_edges.len(),
        });
        let derived = derive(
            root,
            path.as_deref(),
            &reconciliation,
            records,
            habits,
            register,
            seals.as_ref(),
        );
        queue.ranked.extend(derived.ranked);
        queue.blocked.extend(derived.blocked);
        queue.unknowns.extend(derived.unknowns);
    }
    if count == 0 {
        queue
            .unknowns
            .push("habit has no readable feature references or accounted commitments".into());
    }
    if count > 32 {
        queue.unknowns.push(format!(
            "{} additional habit source scopes omitted; open a feature directly",
            count - 32
        ));
    }
    for rows in [&mut queue.ranked, &mut queue.blocked] {
        rows.sort_by(|a, b| {
            (
                a.priority.unwrap_or(usize::MAX),
                a.authored_line.unwrap_or(usize::MAX),
                &a.gap_id,
                &a.intent,
            )
                .cmp(&(
                    b.priority.unwrap_or(usize::MAX),
                    b.authored_line.unwrap_or(usize::MAX),
                    &b.gap_id,
                    &b.intent,
                ))
        });
    }
    queue.unknowns.sort();
    queue.unknowns.dedup();
    queue
}

fn confined_source(root: &Path, source: &str) -> Option<std::path::PathBuf> {
    let root = std::fs::canonicalize(root).ok()?;
    crate::flow::confine_under(&root, &root.join(source)).ok()
}

// Suggestions are conservative: a technical review predating fresh production cannot
// recommend acceptance. Actual acceptance continues through the existing validator.
fn review_action(
    assertion: &crate::flow::reconciliation::Assertion,
    records: &[(Cid, FlowRecord)],
) -> &'static str {
    for commitment in &assertion.commitments {
        let produced = records
            .iter()
            .enumerate()
            .filter(|(_, (cid, _))| commitment.fulfillments.contains(&cid.to_string()))
            .map(|(i, _)| i)
            .max();
        let Some(produced) = produced else {
            return "resume-with-owner";
        };
        let reviews: Vec<_> = records
            .iter()
            .enumerate()
            .filter_map(|(i, (cid, record))| match record {
                FlowRecord::Event(event)
                    if i > produced && commitment.technical_reviews.contains(&cid.to_string()) =>
                {
                    Some(event)
                }
                _ => None,
            })
            .collect();
        if reviews.iter().any(|r| {
            r.classified_as
                .iter()
                .any(|s| s == "verdict:changes-requested")
        }) {
            return "revise-with-owner";
        }
        if !reviews.iter().any(|r| {
            r.action == elohim_epr_rea::ReaVerb::Cite
                && r.fulfills.is_empty()
                && r.satisfies.is_empty()
                && r.classified_as
                    .iter()
                    .filter(|s| s.starts_with("verdict:"))
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    == ["verdict:approved"]
        }) {
            return "independent-review";
        }
    }
    "independent-acceptance"
}

fn action_for(assertion: &crate::flow::reconciliation::Assertion, ownerless: bool) -> &'static str {
    match assertion.acceptance.as_str() {
        "contested" => "resolve-conflict",
        "revalidation-required" => "revalidate",
        "changes-requested" => "revise-with-owner",
        _ if assertion.commitments.is_empty() && ownerless => "claim",
        _ if assertion
            .commitments
            .iter()
            .any(|c| c.fulfillments.is_empty()) =>
        {
            "resume-with-owner"
        }
        _ if assertion
            .commitments
            .iter()
            .any(|c| c.technical_reviews.is_empty()) =>
        {
            "independent-review"
        }
        _ => "independent-acceptance",
    }
}

impl WorkQueue {
    pub fn render_text(&self, limit: usize) -> String {
        let mut out = format!(
            "\n  ACTIONABLE ({}; {} blocked) — {}\n",
            self.ranked.len(),
            self.blocked.len(),
            self.scope
        );
        for (label, rows) in [("next", &self.ranked), ("blocked", &self.blocked)] {
            for row in rows.iter().take(limit) {
                out.push_str(&format!(
                    "    {label}: {} — {} [{}]\n",
                    row.gap_id
                        .as_deref()
                        .or(row.intent.as_deref())
                        .unwrap_or("unnamed"),
                    row.action,
                    row.acceptance
                ));
                if let Some(assertion) = &row.assertion {
                    let preview: String = assertion.chars().take(180).collect();
                    out.push_str(&format!(
                        "      {preview}{}\n",
                        if assertion.chars().count() > 180 {
                            "…"
                        } else {
                            ""
                        }
                    ));
                }
                if let Some(path) = &row.source_path {
                    out.push_str(&format!("      source: {path}\n"));
                }
                out.push_str(&format!("      {}\n", row.remaining_action));
                if !row.owners.is_empty() {
                    out.push_str(&format!("      owners: {}\n", row.owners.join(", ")));
                }
                for reason in row.reasons.iter().chain(&row.blockers) {
                    out.push_str(&format!("      {reason}\n"));
                }
            }
            if rows.len() > limit {
                out.push_str(&format!(
                    "    … {} more {label} rows; use --section actionable.{label_section}\n",
                    rows.len() - limit,
                    label_section = if label == "next" { "ranked" } else { "blocked" }
                ));
            }
        }
        for unknown in &self.unknowns {
            out.push_str(&format!("    unknown: {unknown}\n"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::reconciliation::{Assertion, CommitmentEvidence};
    fn assertion(gap: &str) -> Assertion {
        Assertion {
            intent: Some(gap.into()),
            gap_id: Some(gap.into()),
            authored_state: Some("gap:open".into()),
            issues: vec![],
            commitments: vec![],
            acceptance: "acceptance-unestablished".into(),
            remaining_action: "inspect evidence".into(),
        }
    }
    fn reconciliation(assertions: Vec<Assertion>) -> Reconciliation {
        Reconciliation {
            version: "test",
            evaluator: serde_json::json!({}),
            scope_cid: "scope".into(),
            record_set_fingerprint: "records".into(),
            assertions,
            historical_candidates: vec![],
        }
    }
    #[test]
    fn serves_ranks_source_order_and_unknown_environment_stays_out_of_queue() {
        let dir = tempfile::tempdir().unwrap();
        let source = "---\nserves: readiness\n---\n- [ ] First\n- [ ] Blocked @requires:remote\n- [ ] Third\n";
        std::fs::write(dir.path().join("plan.md"), source).unwrap();
        let decomposition = gaps::decompose(&gaps::slug_for(Path::new("plan.md")), source, vec![]);
        let mut register =
            vec![
                serde_yaml::from_str::<HabitEntry>("id: readiness\nstatus: red\npriority: 2\n")
                    .unwrap(),
            ];
        register[0].priority = Some(2);
        let assertions = decomposition
            .items
            .iter()
            .rev()
            .map(|g| assertion(&g.id))
            .collect();
        let queue = derive(
            dir.path(),
            Some("plan.md"),
            &reconciliation(assertions),
            &[],
            &[],
            &register,
            None,
        );
        assert_eq!(queue.ranked.len(), 2);
        assert_eq!(queue.blocked.len(), 1);
        assert_eq!(
            queue.ranked[0].gap_id.as_deref(),
            Some(decomposition.items[0].id.as_str())
        );
        assert_eq!(queue.ranked[0].priority, Some(2));
        assert_eq!(queue.ranked[0].action, "claim");
        assert!(queue.render_text(3).contains("First"));
        register[0].status = "unwired".into();
        let mut stale = assertion(&decomposition.items[0].id);
        stale.acceptance = "revalidation-required".into();
        let stale_queue = derive(
            dir.path(),
            Some("plan.md"),
            &reconciliation(vec![stale]),
            &[],
            &[],
            &register,
            None,
        );
        assert_eq!(
            stale_queue.ranked[0].action, "revalidate",
            "unwired cannot bypass source revalidation"
        );

        assert!(queue.blocked[0].blockers[0].contains("unknown"));
    }
    #[test]
    fn existing_work_needs_review_and_contrary_evidence_requires_revalidation() {
        let mut a = assertion("gap#1");
        a.commitments.push(CommitmentEvidence {
            commitment: "c".into(),
            fulfillments: vec![],
            technical_reviews: vec![],
            acceptance_records: vec![],
            unrecognized_records: vec![],
            acceptance: "acceptance-unestablished".into(),
            issues: vec![],
        });
        assert_eq!(action_for(&a, false), "resume-with-owner");
        a.commitments[0].fulfillments.push("produced".into());
        assert_eq!(action_for(&a, false), "independent-review");
        a.commitments[0].technical_reviews.push("review".into());
        assert_eq!(action_for(&a, false), "independent-acceptance");
        a.acceptance = "revalidation-required".into();
        assert_eq!(action_for(&a, false), "revalidate");
        a.acceptance = "contested".into();
        assert_eq!(action_for(&a, false), "resolve-conflict");
    }
    #[test]
    fn missing_projection_is_unknown_and_stale_prerequisites_block_claiming() {
        let dir = tempfile::tempdir().unwrap();
        let queue = derive(
            dir.path(),
            None,
            &reconciliation(vec![]),
            &[],
            &[],
            &[],
            None,
        );
        assert!(!queue.unknowns.is_empty());
        let seals = Seals {
            edges: vec![crate::flow::walk::EdgeView {
                from: "source".into(),
                to: "dependency".into(),
                verdict: "stale".into(),
                governor: "cite".into(),
                desc: None,
            }],
            stale_downstream: 0,
        };
        let queue = derive(
            dir.path(),
            None,
            &reconciliation(vec![assertion("gap#1")]),
            &[],
            &[],
            &[],
            Some(&seals),
        );
        assert!(queue.ranked.is_empty());
        assert!(queue.blocked[0].blockers[0].contains("dependency"));
        assert!(queue.render_text(3).contains("blocked: gap#1"));
    }
    #[test]
    fn context_preserves_owner_and_revised_source_never_reclaims_old_work() {
        use elohim_epr_rea::{
            AgentRef, Commitment, CommitmentState, FlowStore, Intent, ReaVerb, ResourceSpec,
            SidecarFlowStore,
        };
        let dir = tempfile::tempdir().unwrap();
        let text = "- [ ] Deliver
";
        std::fs::write(dir.path().join("plan.md"), text).unwrap();
        let scope = crate::flow::body_cid(text);
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        let spec = ResourceSpec {
            classified_as: vec!["gap:open".into(), "plan#1".into()],
            quantity: None,
        };
        let intent = store
            .append(FlowRecord::Intent(Intent {
                action: ReaVerb::Produce,
                resource_spec: spec.clone(),
                in_scope_of: scope,
                raised_by: AgentRef("author".into()),
            }))
            .unwrap();
        store
            .append(FlowRecord::Commitment(Commitment {
                action: ReaVerb::Produce,
                provider: AgentRef("existing-owner".into()),
                receiver: AgentRef("repo".into()),
                resource_spec: spec,
                in_scope_of: scope,
                valid_from: None,
                valid_until: None,
                state: CommitmentState::Active,
                satisfies: vec![intent],
                bound: None,
            }))
            .unwrap();
        std::fs::write(
            dir.path().join(".eprfs/status/labels.json"),
            serde_json::to_string(&serde_json::json!({scope.to_string(): "plan.md"})).unwrap(),
        )
        .unwrap();
        let habit_scope = super::super::HabitScope {
            id: "readiness".into(),
            status: "red".into(),
            active: false,
            checks: vec![],
            refs: vec!["plan.md".into()],
            open_commitments: vec![],
        };
        let habit_queue = for_habit(
            dir.path(),
            &habit_scope,
            &store.records().unwrap(),
            &[],
            &[],
        );
        assert_eq!(habit_queue.ranked.len(), 1);
        assert_eq!(
            habit_queue.ranked[0].source_path.as_deref(),
            Some("plan.md")
        );
        assert_eq!(habit_queue.ranked[0].owners, vec!["existing-owner"]);
        let outside = tempfile::tempdir().unwrap();
        let external = outside.path().join("external.md");
        std::fs::write(
            &external,
            "---\nserves: stolen\n---\nprivate external source",
        )
        .unwrap();
        let queue = derive(
            dir.path(),
            external.to_str(),
            &reconciliation(vec![assertion("gap#1")]),
            &[],
            &[],
            &[],
            None,
        );
        assert!(
            queue.blocked[0].habit.is_none(),
            "external frontmatter must never be read"
        );
        assert!(queue
            .unknowns
            .iter()
            .any(|s| s.contains("outside repository")));
        let before = store.records().unwrap().len();
        let current = super::super::context(dir.path(), "plan.md").unwrap();
        assert_eq!(current.actionable.ranked[0].owners, vec!["existing-owner"]);
        assert_eq!(current.actionable.ranked[0].action, "resume-with-owner");
        std::fs::write(
            dir.path().join("plan.md"),
            "- [ ] Changed assertion
",
        )
        .unwrap();
        let revised = super::super::context(dir.path(), "plan.md").unwrap();
        assert!(revised.actionable.ranked.is_empty());
        assert!(revised
            .actionable
            .unknowns
            .iter()
            .any(|s| s.contains("historical")));
        assert_eq!(
            store.records().unwrap().len(),
            before,
            "context reads never claim or rewrite work"
        );
    }
}
