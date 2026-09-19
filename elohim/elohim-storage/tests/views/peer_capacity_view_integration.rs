//! Integration: peer_capacity_service against test_pool() harness.

use elohim_storage::services::peer_capacity_service::compute_peer_capacity;
use elohim_storage::test_util::test_pool;

#[test]
fn empty_state_returns_zeroed_view() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_peer_capacity(&mut conn, "peer:empty").unwrap();
    assert_eq!(view.peer_cid, "peer:empty");
    // With zero raw bytes the donut is trivially compliant (free=100%).
    assert!(view.ratio_compliance.compliant_with_donut);
}

#[test]
fn realistic_state_returns_zero_rollups_for_now() {
    // Sprint-3 stubs return zeros; follow-up seeds rea_commitments + asserts pledges.
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_peer_capacity(&mut conn, "peer:realistic").unwrap();
    assert_eq!(view.pledges.total_pledged_bytes, 0);
}
