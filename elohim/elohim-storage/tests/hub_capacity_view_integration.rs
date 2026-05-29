use elohim_storage::services::hub_capacity_service::compute_hub_capacity;
use elohim_storage::test_util::test_pool;
use elohim_views::hub_capacity::HubKind;

// NOTE (cross-pillar-cleanup 2026-05-29): these tests were written against the
// original `compute_hub_capacity` stub that "always returned one member". The
// Wave 2 T3 refactor (839f707cc) routes membership through `resolve_hub_members`,
// which queries `stewarded_nodes` by household slug. An un-projected peer (no
// stewarded_nodes rows) therefore resolves to ZERO devices and `capacity: None`
// — the service doc-comments this as correct, not an error. The assertions below
// now pin that documented empty-hub contract.
//
// TODO(fast-follow): add a seeded-member case (insert a `stewarded_nodes` device
// row with household_id == the hub slug) to cover the non-empty aggregation path
// — `member_device_count == 1` and `capacity.is_some()`. No `stewarded_nodes`
// seed helper exists in tests/ yet; that fixture is the prerequisite.

/// An un-projected peer still classifies as a Computed hub, but resolves zero
/// member devices (no `stewarded_nodes` rows) and therefore carries no capacity.
#[test]
fn unprojected_peer_is_computed_hub_with_zero_devices() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_hub_capacity(&mut conn, "peer:test").unwrap();
    assert_eq!(view.hub_kind, HubKind::Computed);
    assert_eq!(view.member_device_count, 0);
}

/// Empty-hub contract: no resolved member devices → `capacity` is `None`
/// (zero aggregate is represented as absence, not a zeroed struct).
#[test]
fn empty_hub_returns_none_capacity() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_hub_capacity(&mut conn, "peer:empty").unwrap();
    assert!(view.capacity.is_none());
}
