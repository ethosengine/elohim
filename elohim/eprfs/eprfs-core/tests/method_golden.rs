use eprfs_core::MethodDescriptor;

#[test]
fn derive_doorway_method_cid_golden() {
    // Captured by `artifact describe` at K1 3326de062, Rust 1.96.1. All paths are fixed.
    let descriptor: MethodDescriptor =
        serde_json::from_str(include_str!("fixtures/doorway-method-k1.json")).unwrap();
    assert_eq!(
        descriptor.cid().unwrap().to_string(),
        "bafyreifwrlwmgys6pnovr5e7xadi2v227hcmxmh2hsbmzwa6d6thhk7txi"
    );
    assert_eq!(descriptor.family_cid().unwrap(), descriptor.cid().unwrap());
}
