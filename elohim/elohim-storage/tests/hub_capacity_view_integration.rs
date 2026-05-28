use elohim_storage::services::hub_capacity_service::compute_hub_capacity;
use elohim_storage::test_util::test_pool;
use elohim_views::hub_capacity::HubKind;

#[test]
fn single_device_returns_computed_kind() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_hub_capacity(&mut conn, "peer:test").unwrap();
    assert_eq!(view.hub_kind, HubKind::Computed);
    assert_eq!(view.member_device_count, 1);
}

#[test]
fn single_member_returns_some_capacity() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_hub_capacity(&mut conn, "peer:empty").unwrap();
    // single-member (the stub always returns one member) → Some aggregate (zeros).
    assert!(view.capacity.is_some());
}
