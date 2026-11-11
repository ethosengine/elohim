//! Integration tests for the upgraded `distribute_shards` — contract-aware
//! peer selection + placement_gaps recording.
//!
//! These tests require the `p2p` feature because they exercise `P2PHandle`.

use elohim_storage::db::placement_gaps;
use elohim_storage::test_util::{spawn_p2p_with_peers, test_pool};

/// Regression: selection SUCCESS must not fabricate a green placement.
///
/// A small blob → single "none"-encoded shard → `desired_count == 1`. One
/// ACCEPTING peer makes `PeerSelection` return `Ok` (`gap_kind_opt == None`).
/// But that peer carries no `peer_transport_manifest` / `peer_identity_bindings`
/// row, so `distribute_shards` cannot resolve its `agent_cid` to a dialable
/// libp2p id and SKIPS the push (`distributed == 0`). The old code cleared
/// `placement_gaps` on selection-success alone — an empty gap set beside zero
/// real placements (fabricated green). The fix records a gap whenever the pushes
/// behind a full selection did not land.
#[tokio::test]
async fn distribute_records_gap_when_selection_ok_but_zero_pushed() {
    let pool = test_pool();
    let harness = spawn_p2p_with_peers(
        pool.clone(),
        &[("agent-alpha-1", "home-alpha", "accepting")], // selection succeeds…
    )
    .await;

    // …but the peer has no transport binding, so the push is skipped.
    let blob = vec![7u8; 4096]; // <= single_shard_max → 1 shard, desired_count == 1
    let distributed = harness
        .p2p
        .distribute_shards("content-zero-push", &blob, &pool, "lamad")
        .await
        .unwrap();
    assert_eq!(
        distributed, 0,
        "unresolvable peer → zero shards actually pushed despite selection success"
    );

    let mut conn = pool.get().unwrap();
    let gaps = placement_gaps::list_gaps(
        &mut conn,
        "lamad",
        placement_gaps::GapQuery {
            content_id: Some("content-zero-push".into()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        !gaps.is_empty(),
        "zero pushed shards must leave/record a placement gap, not a fabricated-green empty set"
    );
    // gap_kind must stay within the placement-gap-view schema enum.
    assert!(
        gaps.iter()
            .all(|g| g.achieved_steward_count == 0 && g.gap_kind == "peers-unavailable"),
        "the recorded gap must reflect the zero-push reality: {:?}",
        gaps.iter()
            .map(|g| (&g.gap_kind, g.achieved_steward_count))
            .collect::<Vec<_>>()
    );
}

/// Seeds three peers across two households and calls `distribute_shards`.
///
/// **This test is `#[ignore]`'d.**
///
/// The push-side identity bridge is now fixed (slice 2): `distribute_shards`
/// resolves each selected `agent_cid` → libp2p `PeerId` via
/// `services::transport_resolve` BEFORE dialing, and skips peers with no
/// transport binding. But two things still keep this test from running under the
/// stub harness:
///   1. The stub `P2PHandle` returns `Err("stub: no P2P swarm in test")` from
///      `push_shard`, so `shard_locations` is never populated even for a
///      resolvable peer — the household-diversity assertion cannot be verified.
///   2. The seeded peers carry non-`uhCAk` agent keys and NO `peer_transport_manifest`
///      / `peer_identity_bindings` rows, so the new resolver returns `None` and
///      they are skipped (distributed == 0) by design.
///
/// A live re-enable therefore needs a `spawn_live_p2p_cluster` fixture that (a)
/// runs a real libp2p swarm and (b) seeds transport bindings for the peers so the
/// resolver can dial them.
///
/// TODO(task-17): implement live P2P integration harness + transport-binding seed,
/// then re-enable.
#[tokio::test]
#[ignore = "Requires live libp2p swarm + seeded transport bindings — stub handle cannot push, and unbound peers are (correctly) skipped by the slice-2 resolver. See Task 17."]
async fn distribute_picks_diverse_households() {
    let pool = test_pool();
    let harness = spawn_p2p_with_peers(
        pool.clone(),
        &[
            ("agent-alpha-1", "home-alpha", "accepting"),
            ("agent-alpha-2", "home-alpha", "accepting"),
            ("agent-beta-1", "home-beta", "accepting"),
        ],
    )
    .await;

    let blob = vec![42u8; 4096];
    let distributed = harness
        .p2p
        .distribute_shards("content-x", &blob, &pool, "lamad")
        .await
        .unwrap();
    assert!(
        distributed > 0,
        "expected at least one shard distributed, got {distributed}"
    );

    // Verify shard_locations has ≥ 2 distinct households represented.
    let mut conn = pool.get().unwrap();
    let locations = elohim_storage::db::shard_locations::get_locations_for_content(
        &mut conn,
        "lamad",
        "content-x",
    )
    .unwrap();
    let households: std::collections::HashSet<String> = locations
        .iter()
        .filter_map(|l| {
            elohim_storage::db::humans::get_human_by_agent_key(&mut conn, &l.peer_id)
                .ok()
                .flatten()
                .and_then(|h| h.household_id)
        })
        .collect();
    assert!(
        households.len() >= 2,
        "expected ≥ 2 households in shard_locations, got {households:?}"
    );
}

/// Seeds a peer whose REA commitment exists but is *leaving* (not accepting),
/// so `PeerSelection` returns `Short(peers-unavailable)` regardless of blob
/// size.  `distribute_shards` must then write at least one `placement_gaps`
/// row with `gap_kind = "peers-unavailable"`.
///
/// Using a "leaving" peer rather than an "accepting" peer with a tiny blob
/// ensures the gap path is exercised independently of blob-size / shard-count
/// arithmetic.  This test does NOT require an actual libp2p swarm.
#[tokio::test]
async fn distribute_records_gap_when_households_are_short() {
    let pool = test_pool();
    let harness = spawn_p2p_with_peers(
        pool.clone(),
        &[("agent-alpha-1", "home-alpha", "leaving")], // committed but not accepting
    )
    .await;

    let blob = vec![42u8; 4096];
    // distribute_shards returns Ok even on short placement (gaps recorded separately)
    let _ = harness
        .p2p
        .distribute_shards("content-short", &blob, &pool, "lamad")
        .await
        .unwrap();

    let mut conn = pool.get().unwrap();
    let gaps = placement_gaps::list_gaps(
        &mut conn,
        "lamad",
        placement_gaps::GapQuery {
            content_id: Some("content-short".into()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        !gaps.is_empty(),
        "expected placement_gaps rows for content-short but got none"
    );
    assert!(
        gaps.iter().any(|g| {
            g.gap_kind == "under-committed"
                || g.gap_kind == "contracts-short"
                || g.gap_kind == "peers-unavailable"
        }),
        "expected a recognized gap_kind, got: {:?}",
        gaps.iter().map(|g| &g.gap_kind).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Self-stewardship: the selector can only ever pick THIS node today
// ---------------------------------------------------------------------------
//
// `peer_statuses` is structurally self-only (author-local rows; cross-agent
// aggregation deferred in the infrastructure zome), so `peer_selection` selects
// the local node. The old push path treated that as a remote peer, failed to
// resolve a transport binding to itself, and skipped every shard — the direct
// cause of `elohim_shard_push_peer_unresolved_total` climbing on the pod that
// selected itself, and of `shard_locations` staying empty.

/// Self-selected + bytes present locally → exactly ONE `shard_locations` row,
/// marked `self-held`, and NOT a single P2P command sent (no self-dial).
#[tokio::test]
async fn self_selection_records_self_held_location_and_never_pushes() {
    use std::sync::Arc;

    let pool = test_pool();
    // The one selectable peer IS this node (agent_cid must be `uhCAk…`-shaped:
    // `shard_locations.peer_id` is an agent_cid join key).
    let _seed = spawn_p2p_with_peers(
        pool.clone(),
        &[("uhCAkSelfAgent", "home-self", "accepting")],
    )
    .await;

    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(
        elohim_storage::BlobStore::new(tmp.path())
            .await
            .expect("blob store"),
    );

    // 4 KiB is one chunk, unstriped → ONE custody unit whose byte CID IS the
    // blob's. Stocking the blob locally is therefore exactly "this node holds
    // the unit's bytes" — the claim the self-held row makes.
    let blob = vec![7u8; 4096];
    let stored = store.store(&blob).await.expect("stock blob");

    let (handle, mut cmd_rx) = elohim_storage::p2p::P2PHandle::for_testing_capturing_commands(
        "uhCAkSelfAgent".to_string(),
    );
    let handle = handle.with_blob_store_for_testing(Arc::clone(&store));

    let distributed = handle
        .distribute_shards("content-self-held", &blob, &pool, "lamad")
        .await
        .unwrap();
    assert_eq!(distributed, 1, "the self-held shard counts as placed");

    let mut conn = pool.get().unwrap();
    let rows = elohim_storage::db::shard_locations::get_locations_for_shard(
        &mut conn,
        &stored.cid.to_string(),
    )
    .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "exactly one shard_locations row, got {rows:?}"
    );
    assert_eq!(
        rows[0].peer_id, "uhCAkSelfAgent",
        "the row is agent-keyed (agent_cid), never a transport id"
    );
    assert_eq!(
        rows[0].status, "self-held",
        "a self-held location must be distinguishable from a peer-verified one"
    );

    // The self-held write now ANNOUNCES custody on the gossip plane (the
    // convergence hot path) — exactly one custody announcement, handed to the
    // swarm task to sign and fan out on `CUSTODY_ANNOUNCE_TOPIC` over every
    // transport (R1: the producer no longer emits a raw libp2p `GossipPublish`),
    // and still never a push/dial (the original no-self-dial contract).
    match cmd_rx.try_recv() {
        Ok(elohim_storage::p2p::P2PCommand::AnnounceCustody(ann)) => {
            assert_eq!(
                ann.shard_cid.to_string(),
                stored.cid.to_string(),
                "the only command a self-held shard emits is its own custody announcement"
            );
            assert_eq!(
                ann.holder_agent_cid, "uhCAkSelfAgent",
                "the announcement names the holder by agent_cid, never a transport id"
            );
            assert_eq!(
                ann.status, "self-held",
                "the announcement carries the source-honest status it recorded locally"
            );
        }
        Ok(_) => panic!(
            "a self-selected shard emitted a non-gossip P2P command — \
             it must never push or dial"
        ),
        Err(e) => panic!("expected the custody GossipPublish announcement, got recv error: {e}"),
    }
    assert!(
        cmd_rx.try_recv().is_err(),
        "no further P2P commands after the custody announcement (no self-dial, no push)"
    );
}

/// Self-selected + bytes ABSENT locally → NO row. An unverifiable location is
/// never fabricated; the placement gap stays honestly visible.
#[tokio::test]
async fn self_selection_records_nothing_when_bytes_are_absent() {
    use std::sync::Arc;

    let pool = test_pool();
    let _seed = spawn_p2p_with_peers(
        pool.clone(),
        &[("uhCAkSelfAgent", "home-self", "accepting")],
    )
    .await;

    let tmp = tempfile::tempdir().unwrap();
    // Empty pantry — the blob below is never stocked.
    let store = Arc::new(
        elohim_storage::BlobStore::new(tmp.path())
            .await
            .expect("blob store"),
    );

    let blob = vec![9u8; 4096];
    let expected_shard_cid = elohim_epr::address::compute_bytes_cid(&blob).to_string();

    let (handle, mut cmd_rx) = elohim_storage::p2p::P2PHandle::for_testing_capturing_commands(
        "uhCAkSelfAgent".to_string(),
    );
    let handle = handle.with_blob_store_for_testing(Arc::clone(&store));

    let distributed = handle
        .distribute_shards("content-self-dark", &blob, &pool, "lamad")
        .await
        .unwrap();
    assert_eq!(distributed, 0, "unverifiable bytes place nothing");

    let mut conn = pool.get().unwrap();
    let rows = elohim_storage::db::shard_locations::get_locations_for_shard(
        &mut conn,
        &expected_shard_cid,
    )
    .unwrap();
    assert!(
        rows.is_empty(),
        "a location we cannot verify must never be recorded, got {rows:?}"
    );

    assert!(
        cmd_rx.try_recv().is_err(),
        "still no self-dial when the bytes are absent"
    );
}

/// Uploader NOT selected + verified full copy in its own pantry → the uploader
/// still records ONE `self-held` row and announces it, while the selected peer
/// keeps its slot (storage-owner ruling on chaos-peer-churn:112). Before this,
/// the uploader's copy was un-recorded whenever another household member's
/// agent key sorted first — a holder answering 200 that the fold never saw.
#[tokio::test]
async fn uploader_not_selected_still_records_self_held_and_announces() {
    use std::sync::Arc;

    let pool = test_pool();
    // One household, two members. `PeerSelection` sorts by agent_cid and takes
    // the first candidate per household, so `uhCAkAAA…` is selected and the
    // uploader (`uhCAkSelfAgent`) is not.
    let _seed = spawn_p2p_with_peers(
        pool.clone(),
        &[
            ("uhCAkAAAOtherAgent", "home-self", "accepting"),
            ("uhCAkSelfAgent", "home-self", "accepting"),
        ],
    )
    .await;

    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(
        elohim_storage::BlobStore::new(tmp.path())
            .await
            .expect("blob store"),
    );
    // One chunk, unstriped: the single custody unit's CID is the blob's.
    let blob = vec![5u8; 4096];
    let stored = store.store(&blob).await.expect("stock blob");

    let (handle, mut cmd_rx) = elohim_storage::p2p::P2PHandle::for_testing_capturing_commands(
        "uhCAkSelfAgent".to_string(),
    );
    let handle = handle.with_blob_store_for_testing(Arc::clone(&store));

    let distributed = handle
        .distribute_shards("content-uploader-unselected", &blob, &pool, "lamad")
        .await
        .unwrap();
    assert_eq!(
        distributed, 0,
        "the uploader's own copy is not a placement: the selected peer has no \
         transport binding here, so nothing was placed"
    );

    let mut conn = pool.get().unwrap();
    let rows = elohim_storage::db::shard_locations::get_locations_for_shard(
        &mut conn,
        &stored.cid.to_string(),
    )
    .unwrap();
    assert_eq!(rows.len(), 1, "exactly the uploader's row, got {rows:?}");
    assert_eq!(rows[0].peer_id, "uhCAkSelfAgent");
    assert_eq!(
        rows[0].status, "self-held",
        "the uploader's copy stays distinguishable from a peer-verified one"
    );

    match cmd_rx.try_recv() {
        Ok(elohim_storage::p2p::P2PCommand::AnnounceCustody(ann)) => {
            assert_eq!(ann.shard_cid.to_string(), stored.cid.to_string());
            assert_eq!(ann.holder_agent_cid, "uhCAkSelfAgent");
            assert_eq!(ann.status, "self-held");
        }
        Ok(_) => {
            panic!("the uploader emitted a non-announce P2P command (it must never dial itself)")
        }
        Err(e) => panic!("expected the uploader's custody announcement, got recv error: {e}"),
    }
    assert!(
        cmd_rx.try_recv().is_err(),
        "one announcement, no push, no self-dial"
    );
}

/// Uploader NOT selected and its pantry does NOT hold the blob → nothing is
/// recorded or announced: a copy that cannot be read back verified is no copy.
#[tokio::test]
async fn uploader_not_selected_without_a_verified_copy_records_nothing() {
    use std::sync::Arc;

    let pool = test_pool();
    let _seed = spawn_p2p_with_peers(
        pool.clone(),
        &[
            ("uhCAkAAAOtherAgent", "home-self", "accepting"),
            ("uhCAkSelfAgent", "home-self", "accepting"),
        ],
    )
    .await;

    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(
        elohim_storage::BlobStore::new(tmp.path())
            .await
            .expect("blob store"),
    );
    let blob = vec![6u8; 4096];
    let shard_cid = elohim_epr::address::compute_bytes_cid(&blob).to_string();

    let (handle, mut cmd_rx) = elohim_storage::p2p::P2PHandle::for_testing_capturing_commands(
        "uhCAkSelfAgent".to_string(),
    );
    let handle = handle.with_blob_store_for_testing(Arc::clone(&store));

    handle
        .distribute_shards("content-uploader-dark", &blob, &pool, "lamad")
        .await
        .unwrap();

    let mut conn = pool.get().unwrap();
    let rows = elohim_storage::db::shard_locations::get_locations_for_shard(&mut conn, &shard_cid)
        .unwrap();
    assert!(rows.is_empty(), "no verified copy, no row; got {rows:?}");
    assert!(
        cmd_rx.try_recv().is_err(),
        "no announcement without a verified copy"
    );
}

// ── R7: the selected self-placement path records only after a verified read ──
//
// A self-held row, its custody announcement and the pledge rest on bytes this node
// read back VERIFIED — never on the payload names `BlobStore::list_cids` reports.

/// The file named `name` anywhere under `root` (the store's fan-out buckets).
fn find_file(root: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if entry.file_name().to_string_lossy() == name {
            return Some(path);
        }
    }
    None
}

/// Every custody announcement the handle emitted; any other command fails the test
/// (a self-selected unit must never push or dial).
fn drain_announcements(
    cmd_rx: &mut tokio::sync::mpsc::Receiver<elohim_storage::p2p::P2PCommand>,
) -> Vec<String> {
    let mut announced = Vec::new();
    while let Ok(cmd) = cmd_rx.try_recv() {
        match cmd {
            elohim_storage::p2p::P2PCommand::AnnounceCustody(ann) => {
                announced.push(ann.shard_cid.to_string())
            }
            _ => panic!("a self-selected unit emitted a non-announce P2P command"),
        }
    }
    announced
}

/// Deterministic, non-repeating bytes, so the chunker cuts distinct chunks.
fn varied_bytes(len: usize) -> Vec<u8> {
    let mut state: u64 = 0x5241_5348_4f4d;
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        out.extend_from_slice(&state.to_le_bytes());
    }
    out.truncate(len);
    out
}

/// Self-selected, and the payload's file is present under its name at its full
/// length — but its bytes are wrong. The name is in `list_cids`; the bytes do not
/// read back verified. No row, no announcement.
#[tokio::test]
async fn selected_path_refuses_corrupt_payload() {
    use std::sync::Arc;

    let pool = test_pool();
    let _seed = spawn_p2p_with_peers(
        pool.clone(),
        &[("uhCAkSelfAgent", "home-self", "accepting")],
    )
    .await;
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(
        elohim_storage::BlobStore::new(tmp.path())
            .await
            .expect("blob store"),
    );
    // One chunk, unstriped: the single custody unit is the payload itself.
    let blob = vec![3u8; 4096];
    let stored = store.store(&blob).await.expect("stock blob");
    let path = find_file(&tmp.path().join("blobs"), &stored.cid.to_string())
        .expect("the stored payload's file");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[100] ^= 0xff; // same length: only a hash can tell
    std::fs::write(&path, &bytes).unwrap();
    assert!(
        store.list_cids().unwrap().contains(&stored.cid),
        "precondition: the corrupt payload is still listed by name"
    );

    let (handle, mut cmd_rx) = elohim_storage::p2p::P2PHandle::for_testing_capturing_commands(
        "uhCAkSelfAgent".to_string(),
    );
    let handle = handle.with_blob_store_for_testing(Arc::clone(&store));
    let distributed = handle
        .distribute_shards("content-self-corrupt", &blob, &pool, "lamad")
        .await
        .unwrap();
    assert_eq!(distributed, 0, "corrupt bytes place nothing");

    let mut conn = pool.get().unwrap();
    let rows = elohim_storage::db::shard_locations::get_locations_for_shard(
        &mut conn,
        &stored.cid.to_string(),
    )
    .unwrap();
    assert!(
        rows.is_empty(),
        "no custody row over corrupt bytes, got {rows:?}"
    );
    assert!(
        drain_announcements(&mut cmd_rx).is_empty(),
        "no custody announcement over corrupt bytes"
    );
}

/// Self-selected for every chunk of a chunked payload whose marker is present and
/// listed, but one chunk file is gone. The missing unit gets no row and no
/// announcement; each chunk that reads back verified is its own unit and does.
#[tokio::test]
async fn selected_path_refuses_missing_chunk_payload() {
    use std::sync::Arc;

    let pool = test_pool();
    let _seed = spawn_p2p_with_peers(
        pool.clone(),
        &[("uhCAkSelfAgent", "home-self", "accepting")],
    )
    .await;
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(
        elohim_storage::BlobStore::new(tmp.path())
            .await
            .expect("blob store"),
    );
    // Several chunks, under the striping threshold: the units are the chunks.
    let blob = varied_bytes(6 * 1024 * 1024);
    let stored = store.store(&blob).await.expect("stock blob");
    assert!(stored.chunks.len() > 1, "fixture must be chunked");
    let missing = stored.chunks[1].cid;
    let chunk_file =
        find_file(&tmp.path().join("chunks"), &missing.to_string()).expect("the chunk's file");
    std::fs::remove_file(&chunk_file).unwrap();
    assert!(
        store.list_cids().unwrap().contains(&stored.cid),
        "precondition: the payload is still listed by name"
    );

    let (handle, mut cmd_rx) = elohim_storage::p2p::P2PHandle::for_testing_capturing_commands(
        "uhCAkSelfAgent".to_string(),
    );
    let handle = handle.with_blob_store_for_testing(Arc::clone(&store));
    handle
        .distribute_shards("content-self-missing-chunk", &blob, &pool, "lamad")
        .await
        .unwrap();

    let mut conn = pool.get().unwrap();
    let rows_for = |conn: &mut _, cid: &str| {
        elohim_storage::db::shard_locations::get_locations_for_shard(conn, cid).unwrap()
    };
    assert!(
        rows_for(&mut conn, &missing.to_string()).is_empty(),
        "no row for a unit whose bytes are missing"
    );
    assert!(
        rows_for(&mut conn, &stored.cid.to_string()).is_empty(),
        "no payload-level row"
    );
    let announced = drain_announcements(&mut cmd_rx);
    assert!(
        !announced.contains(&missing.to_string()),
        "no announcement for the missing unit"
    );
    for chunk in stored.chunks.iter().filter(|c| c.cid != missing) {
        let rows = rows_for(&mut conn, &chunk.cid.to_string());
        assert_eq!(rows.len(), 1, "a verified chunk is its own self-held unit");
        assert_eq!(rows[0].status, "self-held");
        assert!(announced.contains(&chunk.cid.to_string()));
    }
}

/// A second distribution of the same payload finds the self-held rows exactly as the
/// first left them: it counts the units as held, changes nothing and announces nothing.
/// (Liveness after a restart is inventory freshness and recon, not a re-announcement.)
#[tokio::test]
async fn redistribution_emits_no_unchanged_announcement() {
    use std::sync::Arc;

    let pool = test_pool();
    let _seed = spawn_p2p_with_peers(
        pool.clone(),
        &[
            ("uhCAkAAAOtherAgent", "home-self", "accepting"),
            ("uhCAkSelfAgent", "home-self", "accepting"),
        ],
    )
    .await;
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(
        elohim_storage::BlobStore::new(tmp.path())
            .await
            .expect("blob store"),
    );
    let blob = varied_bytes(6 * 1024 * 1024);
    let stored = store.store(&blob).await.expect("stock blob");
    assert!(stored.chunks.len() > 1, "fixture must be chunked");

    let (handle, mut cmd_rx) = elohim_storage::p2p::P2PHandle::for_testing_capturing_commands(
        "uhCAkSelfAgent".to_string(),
    );
    let handle = handle.with_blob_store_for_testing(Arc::clone(&store));

    handle
        .distribute_shards("content-redistributed", &blob, &pool, "lamad")
        .await
        .unwrap();
    let first = drain_announcements(&mut cmd_rx);
    for chunk in &stored.chunks {
        assert!(
            first.contains(&chunk.cid.to_string()),
            "first distribution announces chunk {}",
            chunk.cid
        );
    }

    handle
        .distribute_shards("content-redistributed", &blob, &pool, "lamad")
        .await
        .unwrap();
    let second = drain_announcements(&mut cmd_rx);
    assert!(
        second.is_empty(),
        "identical custody is not re-announced, got {second:?}"
    );

    // One row per chunk, still self-held.
    let mut conn = pool.get().unwrap();
    for chunk in &stored.chunks {
        let rows = elohim_storage::db::shard_locations::get_locations_for_shard(
            &mut conn,
            &chunk.cid.to_string(),
        )
        .unwrap();
        assert_eq!(rows.len(), 1, "one row per unit, got {rows:?}");
        assert_eq!(rows[0].status, "self-held");
    }
}
