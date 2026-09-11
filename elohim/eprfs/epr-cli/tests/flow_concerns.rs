//! Native concern projection is scoped, paged and read-only; drift carries no repair judgment.
use elohim_epr_cli::flow::concerns::concerns;
use elohim_epr_rea::{AgentRef, DepEdge, FlowRecord, FlowStore, Governor, SidecarFlowStore};
use eprfs_core::BlobCid;
use std::path::Path;

fn edge(root: &Path, from: &str, to: &str, governor: Governor, stamp: i64) {
    let seal =
        matches!(governor, Governor::CiteSeal).then(|| *BlobCid::compute_raw(b"old").as_cid());
    let e = DepEdge::new(
        from.into(),
        to.into(),
        Some("asserted relationship".into()),
        governor,
        seal,
        AgentRef("agent:test".into()),
        stamp,
        None,
    )
    .unwrap();
    SidecarFlowStore::open(root)
        .unwrap()
        .append(FlowRecord::Edge(e))
        .unwrap();
}

#[test]
fn scope_paging_preserves_slots_and_unknown_judgment_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/up.md"), "new").unwrap();
    edge(root, "a.md", "docs/up.md", Governor::CiteSeal, 0);
    edge(root, "b.md", "docs/up.md", Governor::CiteSeal, 0);
    edge(root, "c.md", "absent.md", Governor::CiteSeal, 0);
    edge(
        root,
        "d.md",
        "docs/up.md",
        Governor::Compiler("rust".into()),
        0,
    );
    let before = SidecarFlowStore::open(root).unwrap().records().unwrap();
    let first = concerns(root, ".", 0, 1).unwrap();
    assert_eq!(first.counts.total_edges, 4);
    assert_eq!(first.counts.stale, 2);
    assert_eq!(first.counts.dangling, 1);
    assert_eq!(first.counts.governed, 1);
    assert_eq!(first.counts.groups, 2);
    assert_eq!(first.page.next_offset, Some(1));
    assert_eq!(first.page.omitted_edges, 2);
    assert!(!first.groups[0].edges[0].source_readable);
    assert!(first
        .omissions
        .iter()
        .any(|s| s.contains("registry missing")));
    let second = concerns(root, ".", 1, 1).unwrap();
    assert_eq!(second.groups[0].edges[0].slot.from, "a.md");
    assert!(second.groups[0].edges[0].reason.contains("unreviewed"));
    assert!(second.groups[0].edges[0].current_evidence.is_some());
    assert!(second.render_text().contains("stale"));
    let scoped = concerns(root, "docs", 0, 10).unwrap();
    assert_eq!(scoped.counts.total_edges, 3);
    assert_eq!(scoped.counts.selected_edges, 2);
    assert_eq!(scoped.groups.len(), 1);
    assert_eq!(concerns(root, ".", 99, 10).unwrap().page.returned_edges, 0);
    assert!(concerns(root, ".", 0, 0).is_err());
    assert!(concerns(root, ".", 0, 101).is_err());
    assert!(concerns(root, "../", 0, 1).is_err());
    assert!(concerns(
        root,
        &BlobCid::compute_raw(b"old").as_cid().to_string(),
        0,
        1
    )
    .is_err());
    assert_eq!(
        before,
        SidecarFlowStore::open(root).unwrap().records().unwrap()
    );
    // A later native slot declaration replaces its previous governor, never doubles its count.
    edge(
        root,
        "a.md",
        "docs/up.md",
        Governor::Test("proof".into()),
        1,
    );
    let after = concerns(root, ".", 0, 10).unwrap();
    assert_eq!(after.counts.total_edges, 4);
    assert_eq!(after.counts.stale, 1);
    assert_eq!(after.counts.governed, 2);
}

#[test]
fn missing_source_and_invalid_registry_remain_visible() {
    let dir = tempfile::tempdir().unwrap();
    edge(dir.path(), "gone.md", "absent.md", Governor::CiteSeal, 0);
    let result = concerns(dir.path(), "gone.md", 0, 20).unwrap();
    assert_eq!(result.scope_kind, "file");
    assert_eq!(result.counts.dangling, 1);
    assert!(!result.groups[0].edges[0].consumer_readable);
    assert_eq!(result.groups[0].edges[0].assertion_context, "gone.md");
}

#[test]
fn merges_doc_and_sidecar_without_conflating_shared_source_edges() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".claude/epr-meta")).unwrap();
    std::fs::create_dir(root.join("docs")).unwrap();
    std::fs::write(root.join(".claude/epr-meta/recipes.yaml"), "version: 1\nrecipes:\n  - id: fixture\n    version: 1\n    stages:\n      - name: spec\n        artifactKind: document\n        paths: ['docs/*.md']\n").unwrap();
    std::fs::write(root.join("docs/up.md"), "new").unwrap();
    let pin = BlobCid::compute_raw(b"old").short_fingerprint();
    std::fs::write(root.join("docs/down.md"), format!("---\ncites:\n  - \"up | first assertion | {pin} | path: docs/up.md\"\n  - \"up | second assertion | {pin} | path: docs/up.md\"\n---\nconsumer\n")).unwrap();
    std::fs::write(root.join("docs/unreadable.md"), [255, 254]).unwrap();
    edge(root, "docs/down.md", "docs/up.md", Governor::CiteSeal, 0);
    let result = concerns(root, ".", 0, 10).unwrap();
    assert_eq!(result.counts.total_edges, 3);
    assert_eq!(result.counts.selected_edges, 3);
    assert_eq!(result.groups.len(), 1);
    let descriptions: Vec<_> = result.groups[0]
        .edges
        .iter()
        .map(|e| e.slot.description.as_deref().unwrap())
        .collect();
    assert!(descriptions.contains(&"first assertion"));
    assert!(descriptions.contains(&"second assertion"));
    assert!(descriptions.contains(&"asserted relationship"));
    assert!(result
        .omissions
        .iter()
        .any(|s| s.contains("1 registry doc sources unreadable")));
    // Same endpoints across planes remain separate when a native slot becomes governed.
    edge(
        root,
        "docs/down.md",
        "docs/up.md",
        Governor::Test("proof".into()),
        1,
    );
    let drift = concerns(root, "docs/down.md", 0, 20).unwrap();
    assert_eq!(drift.counts.selected_edges, 2);
    let all =
        elohim_epr_cli::flow::concerns::concerns_with(root, "docs/down.md", 0, 20, true).unwrap();
    assert_eq!(all.counts.selected_edges, 3);
    assert_eq!(
        all.groups[0]
            .edges
            .iter()
            .filter(|e| e.verdict == "governed")
            .count(),
        1
    );
    // A malformed registry must not silently make a partial sidecar view look complete.
    std::fs::write(root.join(".claude/epr-meta/recipes.yaml"), "not: [valid").unwrap();
    let partial = concerns(root, ".", 0, 10).unwrap();
    assert_eq!(partial.counts.total_edges, 1);
    assert!(partial
        .omissions
        .iter()
        .any(|s| s.contains("doc plane unavailable")));
}

#[test]
fn empty_repository_read_does_not_initialize_a_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let result = concerns(dir.path(), ".", 0, 20).unwrap();
    assert_eq!(result.counts.total_edges, 0);
    assert!(result
        .omissions
        .iter()
        .any(|s| s.contains("sidecar absent")));
    assert!(!dir.path().join(".eprfs").exists());
}

#[test]
fn all_states_resumes_healthy_held_governed_and_changed_consumer_slots() {
    use elohim_epr_cli::flow::concerns::concerns_with;
    use elohim_epr_rea::EdgeStatus;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("up.md"), "old").unwrap();
    std::fs::write(root.join("down.md"), "consumer before").unwrap();
    edge(root, "down.md", "up.md", Governor::CiteSeal, 0);
    edge(root, "gov.md", "up.md", Governor::Test("proof".into()), 0);
    let held = DepEdge::new(
        "held.md".into(),
        "up.md".into(),
        None,
        Governor::CiteSeal,
        Some(*BlobCid::compute_raw(b"old").as_cid()),
        AgentRef("agent:test".into()),
        0,
        Some(EdgeStatus::Held {
            reason: "deliberate deviation".into(),
            valid_from: 0,
            superseded_by: None,
        }),
    )
    .unwrap();
    SidecarFlowStore::open(root)
        .unwrap()
        .append(FlowRecord::Edge(held))
        .unwrap();
    assert_eq!(concerns(root, ".", 0, 20).unwrap().counts.selected_edges, 0);
    let all = concerns_with(root, ".", 0, 20, true).unwrap();
    assert_eq!(all.counts.selected_edges, 3);
    assert_eq!(all.counts.ok, 1);
    assert_eq!(all.counts.held, 1);
    assert_eq!(all.counts.governed, 1);
    assert!(all.selection_rule.contains("All incident"));
    assert!(all.groups[0]
        .edges
        .iter()
        .all(|e| e.current_evidence.is_some()));
    assert!(all.groups[0]
        .edges
        .iter()
        .any(|e| e.reason.contains("deliberate deviation")));
    let before = concerns_with(root, "down.md", 0, 1, true).unwrap();
    let before_edge = &before.groups[0].edges[0];
    assert_eq!(before_edge.verdict, "ok");
    assert!(before_edge.consumer_evidence.is_some());
    std::fs::write(root.join("down.md"), "consumer after").unwrap();
    let after = concerns_with(root, "down.md", 0, 1, true).unwrap();
    assert_eq!(
        after.groups[0].edges[0].current_evidence,
        before_edge.current_evidence
    );
    assert_ne!(
        after.groups[0].edges[0].consumer_evidence,
        before_edge.consumer_evidence
    );
    std::fs::remove_file(root.join("down.md")).unwrap();
    let missing = concerns_with(root, "down.md", 0, 1, true).unwrap();
    assert!(!missing.groups[0].edges[0].consumer_readable);
    assert!(missing.groups[0].edges[0].consumer_evidence.is_none());
}
