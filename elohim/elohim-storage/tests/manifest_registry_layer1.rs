//! Phase 4 T12 — manifest registry layer-1 disk loader tests.

use elohim_storage::services::manifest_registry::load_pillar_manifest_layer1;

#[test]
fn write_through_layer1_loaded_from_pillar_manifests() {
    let manifest_dir = std::env::temp_dir().join(format!(
        "phase4_manifest_test_{}",
        uuid::Uuid::new_v4().as_simple()
    ));
    std::fs::create_dir_all(manifest_dir.join("lamad")).unwrap();
    std::fs::write(
        manifest_dir.join("lamad/manifest.json"),
        r#"{"writeThrough": {"enabled": true}}"#,
    )
    .unwrap();

    let layer1 = load_pillar_manifest_layer1(&manifest_dir).expect("load");

    assert!(
        layer1.contains_key("lamad"),
        "lamad pillar must be present in layer1"
    );
    assert!(
        layer1["lamad"].enabled,
        "lamad writeThrough.enabled must be true"
    );
}

#[test]
fn missing_manifest_dir_returns_empty_map() {
    let non_existent =
        std::path::PathBuf::from("/tmp/phase4_nonexistent_dir_for_test_do_not_create");
    // Should not panic; should return empty map.
    let layer1 = load_pillar_manifest_layer1(&non_existent).expect("load");
    assert!(layer1.is_empty(), "non-existent dir should produce empty map");
}

#[test]
fn pillar_without_write_through_is_skipped() {
    let manifest_dir = std::env::temp_dir().join(format!(
        "phase4_manifest_test_nwt_{}",
        uuid::Uuid::new_v4().as_simple()
    ));
    std::fs::create_dir_all(manifest_dir.join("shefa")).unwrap();
    std::fs::write(
        manifest_dir.join("shefa/manifest.json"),
        r#"{"name": "shefa", "version": "1.0"}"#,
    )
    .unwrap();

    let layer1 = load_pillar_manifest_layer1(&manifest_dir).expect("load");
    assert!(
        layer1.is_empty(),
        "pillar without writeThrough block should not appear in layer1"
    );
}
