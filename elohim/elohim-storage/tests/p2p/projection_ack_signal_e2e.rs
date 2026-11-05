//! Phase 4 T4 + T9 — integration tests for the projection_ack_handler
//! and distribution_view integration.
//!
//! Verifies that:
//! 1. An ack-projection event writes to projection_events.
//! 2. Non-ack-projection events are silently ignored.
//! 3. distribution_view returns real projector_count after acks.

use diesel::prelude::*;
use elohim_storage::db::models::NewPeerBlobInventoryRow;
use elohim_storage::db::projection_events::list_recent_for_blob;
use elohim_storage::p2p::projection_ack_handler::handle_projection_ack;
use elohim_storage::services::distribution_view::{
    compose_distribution_summary, DistributionContext,
};
use elohim_storage::test_util::test_pool;

#[tokio::test]
async fn ack_projection_signal_writes_projection_event() {
    let pool = test_pool();

    handle_projection_ack(
        &pool,
        "ack-projection",
        "agent-doorway-1",
        "bafkr4icy2zhwqmhxaqdyulgo6ky6ec5oghd65faxe6pvfnc5o6sp3m6bnq",
        "u-source-1",
        "2026-05-11T12:00:00Z",
    )
    .await
    .expect("handle");

    let mut conn = pool.get().expect("conn");
    let recent = list_recent_for_blob(
        &mut conn,
        "bafkr4icy2zhwqmhxaqdyulgo6ky6ec5oghd65faxe6pvfnc5o6sp3m6bnq",
        10,
    )
    .expect("list");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].projector_agent_cid, "agent-doorway-1");
}

#[tokio::test]
async fn non_ack_projection_action_is_ignored() {
    let pool = test_pool();

    handle_projection_ack(
        &pool,
        "custody-blob", // NOT ack-projection
        "agent-bob",
        "bafkr4ib7pfqwh27zi4mn4hgxlatflqas7gk4a3y6nfyouk64cw6nrcrsji",
        "u-source-2",
        "2026-05-11T12:00:00Z",
    )
    .await
    .expect("handle");

    let mut conn = pool.get().expect("conn");
    let recent = list_recent_for_blob(
        &mut conn,
        "bafkr4ib7pfqwh27zi4mn4hgxlatflqas7gk4a3y6nfyouk64cw6nrcrsji",
        10,
    )
    .expect("list");
    assert!(
        recent.is_empty(),
        "non-ack-projection events must not write to projection_events"
    );
}

// ============================================================================
// T9: distribution_view returns real projector_count after acks
// ============================================================================

#[tokio::test]
async fn distribution_summary_returns_real_projector_count_after_ack() {
    let pool = test_pool();

    // Pre-condition: blob has 1 replica entry.
    {
        let mut conn = pool.get().expect("conn");
        diesel::insert_or_ignore_into(
            elohim_storage::db::diesel_schema::peer_blob_inventory::table,
        )
        .values(&NewPeerBlobInventoryRow {
            peer_id: "12D3KooWreplica".to_string(),
            blob_cid: "bafkr4iejpufl5a67s2djzdx346ufycwm5gv4awn4q7vl2avrr37n2vcgwe".to_string(),
            last_seen_at: "2026-05-11T12:00:00Z".to_string(),
            source: "gossip-delta".to_string(),
            sequence: 1,
        })
        .execute(&mut conn)
        .expect("seed inventory");
    }

    // Act: 2 different doorway projectors ack the projection.
    handle_projection_ack(
        &pool,
        "ack-projection",
        "agent-doorway-1",
        "bafkr4iejpufl5a67s2djzdx346ufycwm5gv4awn4q7vl2avrr37n2vcgwe",
        "u-source-A1",
        "2026-05-11T12:00:00Z",
    )
    .await
    .unwrap();
    handle_projection_ack(
        &pool,
        "ack-projection",
        "agent-doorway-2",
        "bafkr4iejpufl5a67s2djzdx346ufycwm5gv4awn4q7vl2avrr37n2vcgwe",
        "u-source-A2",
        "2026-05-11T12:01:00Z",
    )
    .await
    .unwrap();

    // Assert: distribution view returns projector_count=2 (not stubbed 0).
    let summary = compose_distribution_summary(
        &pool,
        "bafkr4iejpufl5a67s2djzdx346ufycwm5gv4awn4q7vl2avrr37n2vcgwe",
        DistributionContext::Visitor,
    )
    .await
    .expect("compose");

    assert_eq!(
        summary.projector_count, 2,
        "must reflect distinct projectors from projection_events"
    );
}
