//! Phase 4 T3 — projection_events append-only log tests.

use chrono::Utc;
use elohim_storage::db::projection_events::{
    append_projection_event, list_recent_for_blob, distinct_projectors_for_blob,
    NewProjectionEvent,
};
use elohim_storage::test_util::test_pool;

#[test]
fn append_and_read_back_projection_event() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");

    let now = Utc::now().to_rfc3339();
    append_projection_event(
        &mut conn,
        &NewProjectionEvent {
            blob_hash: "sha256-test1".into(),
            projector_agent_cid: "agent-doorway-1".into(),
            emitted_at: now.clone(),
            source_action_hash: "u-source-1".into(),
        },
    )
    .expect("append");

    let recent = list_recent_for_blob(&mut conn, "sha256-test1", 10).expect("list");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].projector_agent_cid, "agent-doorway-1");
}

#[test]
fn list_recent_returns_newest_first_and_respects_limit() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");

    for (i, agent) in ["agent-1", "agent-2", "agent-3"].iter().enumerate() {
        append_projection_event(
            &mut conn,
            &NewProjectionEvent {
                blob_hash: "sha256-test2".into(),
                projector_agent_cid: agent.to_string(),
                emitted_at: format!("2026-05-{:02}T00:00:00Z", i + 1),
                source_action_hash: format!("u-source-{}", i),
            },
        )
        .expect("append");
    }

    let recent = list_recent_for_blob(&mut conn, "sha256-test2", 2).expect("list");
    assert_eq!(recent.len(), 2);
    // Newest first: agent-3 emitted on 2026-05-03, agent-2 on 2026-05-02.
    assert_eq!(recent[0].projector_agent_cid, "agent-3");
    assert_eq!(recent[1].projector_agent_cid, "agent-2");
}

#[test]
fn dedup_on_source_action_hash() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");

    // Insert the same source_action_hash twice — second insert must be silently ignored.
    for _ in 0..2 {
        append_projection_event(
            &mut conn,
            &NewProjectionEvent {
                blob_hash: "sha256-test3".into(),
                projector_agent_cid: "agent-doorway-1".into(),
                emitted_at: "2026-05-11T12:00:00Z".into(),
                source_action_hash: "u-source-dedup".into(),
            },
        )
        .expect("append");
    }

    let recent = list_recent_for_blob(&mut conn, "sha256-test3", 10).expect("list");
    assert_eq!(recent.len(), 1, "duplicate source_action_hash must be deduplicated");
}

#[test]
fn distinct_projectors_returns_unique_agents() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");

    // Two acks from the same doorway (different action hashes).
    for i in 0..2u32 {
        append_projection_event(
            &mut conn,
            &NewProjectionEvent {
                blob_hash: "sha256-test4".into(),
                projector_agent_cid: "agent-doorway-1".into(),
                emitted_at: "2026-05-11T12:00:00Z".into(),
                source_action_hash: format!("u-source-d1-{i}"),
            },
        )
        .expect("append");
    }
    // One ack from a different doorway.
    append_projection_event(
        &mut conn,
        &NewProjectionEvent {
            blob_hash: "sha256-test4".into(),
            projector_agent_cid: "agent-doorway-2".into(),
            emitted_at: "2026-05-11T12:01:00Z".into(),
            source_action_hash: "u-source-d2-0".into(),
        },
    )
    .expect("append");

    let mut projectors = distinct_projectors_for_blob(&mut conn, "sha256-test4").expect("list");
    projectors.sort();
    assert_eq!(projectors, vec!["agent-doorway-1", "agent-doorway-2"]);
}
