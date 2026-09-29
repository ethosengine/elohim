//! Standalone projection attribution is explicit and preserves idempotence.
use elohim_epr_cli::flow::{project, repository_agent, repository_scope};
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use std::path::Path;

fn fixture(agent: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".epr-meta")).unwrap();
    std::fs::write(
        root.join(".epr-meta/repository.yaml"),
        format!("version: 1\nagent: {agent}\n"),
    )
    .unwrap();
    std::fs::write(root.join("recipes.yaml"), "version: 1\nrecipes:\n  - id: fixture\n    version: 1\n    description: Standalone fixture\n    stages:\n      - name: scenario\n        artifactKind: a2o:feature\n        paths: ['*.feature']\n    edges: []\n").unwrap();
    std::fs::write(
        root.join("work.feature"),
        "Feature: Work\n  Scenario: Delivery\n    Then the habit holds\n",
    )
    .unwrap();
    dir
}
fn records(root: &Path) -> Vec<(cid::Cid, FlowRecord)> {
    SidecarFlowStore::open(root).unwrap().records().unwrap()
}
#[test]
fn standalone_projection_uses_its_own_repository_and_deduplicates() {
    let a = fixture("repo:fixture/a");
    let b = fixture("repo:fixture/b");
    for dir in [&a, &b] {
        let root = dir.path();
        project::project(root, &root.join("recipes.yaml")).unwrap();
        let before = records(root);
        assert!(!before.is_empty());
        for (_, record) in &before {
            if let FlowRecord::Commitment(commitment) = record {
                assert_eq!(commitment.receiver, repository_agent(root).unwrap());
                assert_eq!(commitment.in_scope_of, repository_scope(root).unwrap());
            }
        }
        project::project(root, &root.join("recipes.yaml")).unwrap();
        assert_eq!(records(root).len(), before.len());
    }
    let commitment = |root: &Path| {
        records(root)
            .into_iter()
            .find_map(|(cid, record)| matches!(record, FlowRecord::Commitment(_)).then_some(cid))
            .unwrap()
    };
    assert_ne!(commitment(a.path()), commitment(b.path()));
}
#[test]
fn undeclared_projection_refuses_before_appending_any_records() {
    let dir = fixture("repo:fixture/missing");
    std::fs::remove_file(dir.path().join(".epr-meta/repository.yaml")).unwrap();
    let error = project::project(dir.path(), &dir.path().join("recipes.yaml")).unwrap_err();
    assert!(error.to_string().contains("repository.yaml"));
    assert!(!dir.path().join(".eprfs").exists());
}
