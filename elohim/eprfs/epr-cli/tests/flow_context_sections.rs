use elohim_epr_cli::flow::context_sections::{context_section, project_section};
use serde_json::json;

#[test]
fn huge_context_opens_progressively_without_hiding_counts_or_evidence() {
    let assertions: Vec<_> = (0..2000).map(|i| json!({"intent":format!("cid-{i}"),"acceptance":"contested","evidence":"w".repeat(4000)})).collect();
    let value = json!({"identity":{"cid":"scope-cid"},"reconciliation":{"assertions":assertions,"record_set_fingerprint":"records-pin"}});
    assert!(serde_json::to_vec(&value).unwrap().len() > 64 * 1024);
    let top = project_section(&value, "justice.md", ".", 0, 20).unwrap();
    assert_eq!(top.total_items, 2);
    assert_eq!(top.projection_identity["scope_cid"], "scope-cid");
    assert_eq!(
        top.projection_identity["record_set_fingerprint"],
        "records-pin"
    );
    assert!(serde_json::to_vec(&top).unwrap().len() < 4096);
    let mut changed = value.clone();
    changed["governance"] = json!({"rule_count": 7});
    let changed_top = project_section(&changed, "justice.md", ".", 0, 20).unwrap();
    assert_ne!(
        top.projection_identity["context_cid"],
        changed_top.projection_identity["context_cid"]
    );
    assert_eq!(
        top.projection_identity["record_set_fingerprint"],
        changed_top.projection_identity["record_set_fingerprint"]
    );

    let page = project_section(&value, "justice.md", "reconciliation.assertions", 5, 2).unwrap();
    assert_eq!(page.total_items, 2000);
    assert_eq!(page.page.next_offset, Some(7));
    assert_eq!(page.page.omitted_items, 1998);
    assert_eq!(page.items[0].section, "reconciliation.assertions.5");
    assert!(page.items[0].preview);
    assert_eq!(page.items[0].value["intent"], "cid-5");
    assert!(page.items[0].value["evidence"]
        .as_str()
        .unwrap()
        .ends_with('…'));

    let node = project_section(&value, "justice.md", &page.items[0].section, 0, 20).unwrap();
    let evidence = node.items.iter().find(|v| v.key == "evidence").unwrap();
    assert_eq!(evidence.total_items, 4000);
    assert_eq!(evidence.value.as_str().unwrap().len(), 256);
    assert_eq!(evidence.omitted_items, 3744);
    assert!(evidence.preview);
    let chunk = project_section(&value, "justice.md", &evidence.section, 256, 100).unwrap();
    assert_eq!(chunk.unit, "characters");
    assert_eq!(chunk.items[0].key, "value");
    assert_eq!(chunk.items[0].value.as_str().unwrap().len(), 100);
    assert_eq!(chunk.page.next_offset, Some(356));
    let absent = project_section(&value, "justice.md", "reconciliation.not_here", 0, 20).unwrap();
    assert!(!absent.found);
    assert_eq!(absent.kind, "missing");
    assert!(absent
        .omissions
        .iter()
        .any(|x| x.contains("does not exist")));
    assert!(project_section(&value, "justice.md", ".", 0, 0).is_err());
}

#[test]
fn byte_budget_has_continuation_and_unicode_offsets_are_characters() {
    let value = json!((0..100).map(|_| "\u{0001}".repeat(300)).collect::<Vec<_>>());
    let page = project_section(&value, "huge.md", ".", 0, 100).unwrap();
    assert!(page.page.returned_items < 100);
    assert!(page.page.next_offset.is_some());
    assert!(serde_json::to_vec_pretty(&page).unwrap().len() < 64 * 1024);
    let unicode = json!({"text":"é🌿漢字"});
    let chunk = project_section(&unicode, "unicode.md", "text", 1, 2).unwrap();
    assert_eq!(chunk.items[0].value, "🌿漢");
    assert_eq!(chunk.page.next_offset, Some(3));
    assert_eq!(chunk.total_items, 4);
}

#[test]
fn native_context_sections_reuse_identity_and_reconciliation() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("source.md"), "A source claim").unwrap();
    let native = context_section(dir.path(), "source.md", "identity", 5, 0, 20).unwrap();
    assert!(native.found);
    assert_eq!(native.kind, "object");
    assert_eq!(
        native.items.iter().find(|i| i.key == "path").unwrap().value,
        "source.md"
    );
    let missing = context_section(
        dir.path(),
        "source.md",
        "reconciliation.assertions.9999",
        5,
        0,
        1,
    )
    .unwrap();
    assert!(!missing.found);
    assert!(missing.render_text().contains("does not exist"));
}
