//! The acceptance projection must retain legacy fulfilled work and later contrary evidence.
use elohim_epr_cli::flow::{body_cid, context, reconciliation};
use elohim_epr_rea::{
    AgentRef, Commitment, CommitmentState, FlowEvent, FlowRecord, FlowStore, Intent, Magnitude,
    ReaVerb, ResourceSpec, SidecarFlowStore,
};
use tempfile::TempDir;

fn fixture() -> (TempDir, SidecarFlowStore, cid::Cid, cid::Cid) {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("scope.md"), "original assertion").unwrap();
    let scope = body_cid("original assertion");
    let mut store = SidecarFlowStore::open(dir.path()).unwrap();
    let spec = ResourceSpec {
        classified_as: vec!["gap:open".into(), "fixture#1".into()],
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
    let commitment = store
        .append(FlowRecord::Commitment(Commitment {
            action: ReaVerb::Produce,
            provider: AgentRef("implementer".into()),
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
        serde_json::to_string(&serde_json::json!({scope.to_string(): "scope.md"})).unwrap(),
    )
    .unwrap();
    (dir, store, scope, commitment)
}

fn event(
    scope: cid::Cid,
    resource: cid::Cid,
    action: ReaVerb,
    fulfills: Vec<cid::Cid>,
    unit: &str,
    tags: Vec<String>,
) -> FlowRecord {
    FlowRecord::Event(FlowEvent {
        action,
        provider: AgentRef("implementer".into()),
        receiver: AgentRef("repo".into()),
        resource,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: unit.into(),
        },
        process: None,
        in_scope_of: scope,
        fulfills,
        satisfies: vec![],
        occurred_at: "2026-09-09T00:00:00Z".into(),
        classified_as: tags,
    })
}

#[test]
fn production_and_review_remain_visible_without_inventing_acceptance() {
    let (dir, mut store, scope, commitment) = fixture();
    let produce = store
        .append(event(
            scope,
            body_cid("report"),
            ReaVerb::Produce,
            vec![commitment],
            "task-report",
            vec![],
        ))
        .unwrap();
    let review = store
        .append(event(
            scope,
            commitment,
            ReaVerb::Cite,
            vec![],
            "run-note",
            vec![
                "run:verdict".into(),
                "fixture#1".into(),
                "verdict:approved".into(),
            ],
        ))
        .unwrap();
    let result = context::context(dir.path(), "scope.md").unwrap();
    assert!(
        result.commitments.is_empty(),
        "legacy stock discharge remains unchanged"
    );
    let row = &result.reconciliation.assertions[0];
    assert_eq!(row.acceptance, "acceptance-unestablished");
    assert_eq!(row.commitments[0].fulfillments, vec![produce.to_string()]);
    assert_eq!(
        row.commitments[0].technical_reviews,
        vec![review.to_string()]
    );
    let text = result.reconciliation.render_text(3);
    assert!(text.contains(&row.acceptance));
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(
        json["reconciliation"]["assertions"][0]["acceptance"],
        row.acceptance
    );
}

#[test]
fn unclassified_red_run_after_green_does_not_disappear_behind_discharge() {
    let (dir, mut store, scope, commitment) = fixture();
    let feature = body_cid("scenario");
    store
        .append(event(
            scope,
            feature,
            ReaVerb::Produce,
            vec![commitment],
            "green-run",
            vec![],
        ))
        .unwrap();
    let red = store
        .append(event(
            scope,
            feature,
            ReaVerb::Dismiss,
            vec![],
            "red-run",
            vec![],
        ))
        .unwrap();
    let result = reconciliation::reconcile(
        dir.path(),
        &scope,
        Some("scope.md"),
        &store.records().unwrap(),
    );
    assert_eq!(result.assertions[0].acceptance, "revalidation-required");
    assert!(result.assertions[0].commitments[0]
        .issues
        .iter()
        .any(|s| s.contains(&red.to_string())));
}

#[test]
fn changed_source_exposes_historical_candidates_and_missing_source_needs_revalidation() {
    let (dir, store, scope, _) = fixture();
    std::fs::write(dir.path().join("scope.md"), "revised assertion").unwrap();
    let revised = context::context(dir.path(), "scope.md").unwrap();
    assert!(revised.reconciliation.assertions.is_empty());
    assert_eq!(revised.reconciliation.historical_candidates.len(), 2);
    assert!(revised
        .reconciliation
        .historical_candidates
        .iter()
        .all(|c| c.linkage.contains("unverified")));
    std::fs::remove_file(dir.path().join("scope.md")).unwrap();
    let missing = context::context(dir.path(), "scope.md").unwrap();
    assert_eq!(
        missing.reconciliation.assertions[0].acceptance,
        "revalidation-required"
    );
    let by_cid = reconciliation::reconcile(dir.path(), &scope, None, &store.records().unwrap());
    assert_eq!(by_cid.assertions[0].acceptance, "revalidation-required");
}

#[test]
fn orphan_commitments_and_all_intents_are_ordered_and_truncation_is_explicit() {
    let (dir, mut store, scope, _) = fixture();
    for n in 2..7 {
        store
            .append(FlowRecord::Intent(Intent {
                action: ReaVerb::Produce,
                resource_spec: ResourceSpec {
                    classified_as: vec!["gap:open".into(), format!("fixture#{n}")],
                    quantity: None,
                },
                in_scope_of: scope,
                raised_by: AgentRef("author".into()),
            }))
            .unwrap();
    }
    let mut orphan = match store.records().unwrap()[1].1.clone() {
        FlowRecord::Commitment(c) => c,
        _ => unreachable!(),
    };
    orphan.satisfies.clear();
    store.append(FlowRecord::Commitment(orphan)).unwrap();
    let records = store.records().unwrap();
    let result = reconciliation::reconcile(dir.path(), &scope, Some("scope.md"), &records);
    assert_eq!(result.assertions.len(), 7);
    assert_eq!(
        result
            .assertions
            .iter()
            .filter(|a| a.intent.is_none())
            .count(),
        1
    );
    assert!(result.render_text(3).contains("and 4 more assertions"));
    let mut reversed = records.clone();
    reversed.reverse();
    let reordered = reconciliation::reconcile(dir.path(), &scope, Some("scope.md"), &reversed);
    assert_eq!(
        serde_json::to_value(&result.assertions).unwrap(),
        serde_json::to_value(&reordered.assertions).unwrap()
    );
    assert_ne!(
        result.record_set_fingerprint, reordered.record_set_fingerprint,
        "snapshot fingerprint must preserve append order"
    );
}

#[test]
fn revised_scenario_bytes_do_not_inherit_historical_green() {
    let (dir, mut store, scope, _) = fixture();
    std::fs::write(dir.path().join("case.feature"), "Scenario: revised").unwrap();
    let c = Commitment {
        action: ReaVerb::Produce,
        provider: AgentRef("ci".into()),
        receiver: AgentRef("repo".into()),
        resource_spec: ResourceSpec {
            classified_as: vec!["a2o:scenario-green".into(), "case.feature".into()],
            quantity: None,
        },
        in_scope_of: scope,
        valid_from: None,
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: vec![],
        bound: None,
    };
    let id = store.append(FlowRecord::Commitment(c)).unwrap();
    store
        .append(event(
            scope,
            body_cid("Scenario: old"),
            ReaVerb::Produce,
            vec![id],
            "green-run",
            vec![],
        ))
        .unwrap();
    let result = reconciliation::reconcile(
        dir.path(),
        &scope,
        Some("scope.md"),
        &store.records().unwrap(),
    );
    let row = result
        .assertions
        .iter()
        .find(|a| a.intent.is_none())
        .unwrap();
    assert_eq!(row.acceptance, "revalidation-required");
}

#[test]
fn unvalidated_acceptance_cannot_bless_work_and_competing_decisions_stay_visible() {
    let (dir, mut store, scope, commitment) = fixture();
    store
        .append(event(
            scope,
            commitment,
            ReaVerb::Cite,
            vec![],
            "run-note",
            vec![
                "run:verdict".into(),
                "fixture#1".into(),
                "purpose:acceptance".into(),
                "verdict:approved".into(),
            ],
        ))
        .unwrap();
    let invalid = reconciliation::reconcile(
        dir.path(),
        &scope,
        Some("scope.md"),
        &store.records().unwrap(),
    );
    assert_eq!(invalid.assertions[0].acceptance, "revalidation-required");
    assert!(!invalid.assertions[0].commitments[0].issues.is_empty());
    store
        .append(event(
            scope,
            commitment,
            ReaVerb::Cite,
            vec![],
            "run-note",
            vec![
                "run:verdict".into(),
                "fixture#1".into(),
                "purpose:acceptance".into(),
                "verdict:changes-requested".into(),
            ],
        ))
        .unwrap();
    let contested = reconciliation::reconcile(
        dir.path(),
        &scope,
        Some("scope.md"),
        &store.records().unwrap(),
    );
    assert_eq!(contested.assertions[0].acceptance, "contested");
    assert_eq!(
        contested.assertions[0].commitments[0]
            .acceptance_records
            .len(),
        2
    );
}

#[test]
fn bare_scope_without_locator_exposes_unknown_source() {
    let (dir, store, scope, _) = fixture();
    std::fs::remove_file(dir.path().join(".eprfs/status/labels.json")).unwrap();
    let view = reconciliation::reconcile(dir.path(), &scope, None, &store.records().unwrap());
    assert_eq!(view.assertions[0].acceptance, "revalidation-required");
    assert!(view.assertions[0]
        .issues
        .iter()
        .any(|issue| issue.contains("current source unestablished")));
    assert!(view.render_text(3).contains("current source unestablished"));
    let located = reconciliation::reconcile(
        dir.path(),
        &scope,
        Some("scope.md"),
        &store.records().unwrap(),
    );
    assert_eq!(located.assertions[0].acceptance, "acceptance-unestablished");
}

#[test]
fn imported_unknown_acceptance_shapes_are_not_technical_reviews() {
    let (dir, mut store, scope, commitment) = fixture();
    let mut unknown_ids = Vec::new();
    for metadata in [
        "purpose:future-acceptance",
        "appointment:missing-purpose",
        "report-cid:missing-purpose",
    ] {
        unknown_ids.push(
            store
                .append(event(
                    scope,
                    commitment,
                    ReaVerb::Cite,
                    vec![],
                    "run-note",
                    vec![
                        "run:verdict".into(),
                        "fixture#1".into(),
                        "verdict:approved".into(),
                        metadata.into(),
                    ],
                ))
                .unwrap()
                .to_string(),
        );
    }
    let view = reconciliation::reconcile(
        dir.path(),
        &scope,
        Some("scope.md"),
        &store.records().unwrap(),
    );
    let evidence = &view.assertions[0].commitments[0];
    assert_eq!(view.assertions[0].acceptance, "revalidation-required");
    assert!(evidence.technical_reviews.is_empty());
    assert!(evidence.acceptance_records.is_empty());
    unknown_ids.sort();
    assert_eq!(evidence.unrecognized_records, unknown_ids);
    for id in unknown_ids {
        assert!(view.render_text(3).contains(&id));
    }
}
