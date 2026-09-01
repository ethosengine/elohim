//! The doorbell on the iroh plane: a locally-authored change reaches a peer
//! WITHOUT any sync round being driven.
//!
//! These are the iroh twins of `sync_libp2p_convergence`'s announce proofs. The
//! discipline they enforce is the same one that made the libp2p cure real:
//! **no round is ever driven here**. `run_iroh_sync_round` is never called and
//! no driver is spawned, so a passing assertion can only mean the announce
//! itself carried the change — either as pushed bytes, or as the receive arm's
//! pull back at the announcer.
//!
//! Two shapes, matching the two ways a doorbell can be answered:
//!
//! 1. [`a_fresh_change_converges_on_the_announce_alone`] — the peer holds
//!    nothing and the announced change has no dependencies, so the pushed bytes
//!    apply directly. Pinned by the `announce_pull` counter staying at ZERO: a
//!    convergence that quietly pulled would pass a naive "did it converge?"
//!    assertion while the eager payload was inert.
//! 2. [`an_announce_whose_deps_the_peer_lacks_falls_back_to_a_pull`] — Automerge
//!    QUEUES a change whose dependencies the receiver lacks and `apply_changes`
//!    still returns `Ok` with the doc untouched. The receive arm must notice
//!    (via `get_change_by_hash`) and pull, which carries the deps. Pinned by the
//!    peer holding the doc's EARLIEST change, not just the announced head — a
//!    head without its history matches on corpus digest and would never re-heal.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use elohim_storage::db::models::Content;
use elohim_storage::p2p_iroh::{
    announce_local_change, parity_harness::TwoNodeFixture, AlpnRegistration, IrohAnnounceInputs,
    IrohPeerBook, IrohPeerEntry, IrohSyncProtocol, SyncBackend, SyncManagerBackend, SYNC_ALPN,
};
use elohim_storage::sync::projector::{project_content_doc, PROJECTION_NAMESPACE as SYNC_NS};
use elohim_storage::sync::{DocStore, StreamTracker, SyncManager};
use tempfile::tempdir;

const CONTENT_ID: &str = "iroh-doorbell-1";
const DOC_ID: &str = "node:iroh-doorbell-1";

async fn sync_manager(dir: &std::path::Path) -> Arc<SyncManager> {
    let doc_store = Arc::new(
        DocStore::at_path(dir.join("sync.sled"))
            .await
            .expect("doc store"),
    );
    Arc::new(SyncManager::new(doc_store, Arc::new(StreamTracker::new())))
}

fn sample_content(title: &str, body: &str) -> Content {
    Content {
        id: CONTENT_ID.to_string(),
        h_app_id: "lamad".to_string(),
        title: title.to_string(),
        description: None,
        content_type: "concept".to_string(),
        content_format: "markdown".to_string(),
        blob_hash: None,
        blob_cid: None,
        content_size_bytes: None,
        metadata_json: Some("{}".to_string()),
        // Broadcast-tier: the sync plane's fail-closed reach gate excludes
        // non-broadcast reach values by design.
        reach: "commons".to_string(),
        validation_status: "valid".to_string(),
        created_by: None,
        created_at: "2026-08-23T00:00:00Z".to_string(),
        updated_at: "2026-08-23T00:00:00Z".to_string(),
        content_body: Some(body.to_string()),
        dht_anchor_hash: None,
        p2p_published_at: None,
        server_blob_hash: None,
        crdt_converged_at: None,
        declared_head_action_hash: None,
        declared_head_at: None,
        canonical_declared_at: None,
        canonical_earned: None,
        dht_anchor_state: None,
        dht_anchor_checked_at: None,
    }
}

fn announce_pull_ok() -> u64 {
    elohim_storage::metrics::IROH_SYNC_REQUESTS
        .with_label_values(&["announce_pull", "ok"])
        .get()
}

/// Both nodes serve the sync ALPN over their OWN `SyncManager`: the author so
/// the peer can pull back at it, the peer so the author can ring it. Returns
/// (fixture, author_sync, peer_sync, peer_backend).
async fn two_sync_nodes(
    author_dir: &std::path::Path,
    author_sync_dir: &std::path::Path,
    peer_dir: &std::path::Path,
    peer_sync_dir: &std::path::Path,
) -> Result<(
    TwoNodeFixture,
    Arc<SyncManager>,
    Arc<SyncManager>,
    Arc<SyncManagerBackend>,
)> {
    let author_sync = sync_manager(author_sync_dir).await;
    let peer_sync = sync_manager(peer_sync_dir).await;

    let author_backend: Arc<dyn SyncBackend> =
        Arc::new(SyncManagerBackend::new(author_sync.clone()));
    let peer_backend = Arc::new(SyncManagerBackend::new(peer_sync.clone()));
    let peer_dyn: Arc<dyn SyncBackend> = peer_backend.clone();

    let author_protocols: Vec<AlpnRegistration> = vec![(
        SYNC_ALPN.to_vec(),
        Box::new(IrohSyncProtocol::new(author_backend)),
    )];
    let peer_protocols: Vec<AlpnRegistration> = vec![(
        SYNC_ALPN.to_vec(),
        Box::new(IrohSyncProtocol::new(peer_dyn)),
    )];

    // provider = author, fetcher = peer.
    let fixture =
        TwoNodeFixture::new_asymmetric(author_dir, author_protocols, peer_dir, peer_protocols)
            .await?;
    Ok((fixture, author_sync, peer_sync, peer_backend))
}

/// A first change has no dependencies, so the pushed bytes apply directly and
/// the peer converges on the announce alone — no round, and no pull.
#[tokio::test]
async fn a_fresh_change_converges_on_the_announce_alone() -> Result<()> {
    let (author_dir, author_sync_dir) = (tempdir()?, tempdir()?);
    let (peer_dir, peer_sync_dir) = (tempdir()?, tempdir()?);
    let (fixture, author_sync, peer_sync, peer_backend) = two_sync_nodes(
        author_dir.path(),
        author_sync_dir.path(),
        peer_dir.path(),
        peer_sync_dir.path(),
    )
    .await?;

    // The peer can dial back at the author (it will not need to here — that is
    // the point of the counter assertion below).
    let peer_book = IrohPeerBook::new();
    peer_book.upsert(IrohPeerEntry {
        addr: fixture.provider_addr.clone(),
        agent_cid: None,
        libp2p_peer_id: None,
        user_agent: None,
        announced_at_ms: 1,
    });
    assert!(peer_backend.set_pull_back(fixture.fetcher.endpoint().clone(), peer_book, None));

    // The author knows the peer.
    let author_book = IrohPeerBook::new();
    author_book.upsert(IrohPeerEntry {
        addr: fixture.fetcher.node_addr().await?,
        agent_cid: None,
        libp2p_peer_id: None,
        user_agent: None,
        announced_at_ms: 1,
    });

    // One locally-authored change.
    assert!(project_content_doc(&author_sync, &sample_content("Version 1", "hello")).await?);
    let head = author_sync.get_heads(SYNC_NS, DOC_ID).await?[0].clone();
    assert!(
        peer_sync.get_heads(SYNC_NS, DOC_ID).await?.is_empty(),
        "the peer must start with no history for this doc"
    );

    let pulls_before = announce_pull_ok();
    let accepted = announce_local_change(
        &IrohAnnounceInputs {
            endpoint: fixture.provider.endpoint().clone(),
            book: author_book,
            sync_manager: author_sync.clone(),
        },
        DOC_ID,
        &head,
    )
    .await;

    assert_eq!(accepted, 1, "the peer must have ACCEPTED the pushed change");
    assert_eq!(
        peer_sync.get_doc_field(SYNC_NS, DOC_ID, "title").await?,
        "Version 1",
        "the peer must hold the announced change — no round was ever driven"
    );
    assert_eq!(
        peer_sync.get_heads(SYNC_NS, DOC_ID).await?,
        author_sync.get_heads(SYNC_NS, DOC_ID).await?,
        "heads must converge"
    );
    assert_eq!(
        announce_pull_ok(),
        pulls_before,
        "a dependency-free change must land on the PUSH — a pull here means the eager payload was inert"
    );

    fixture.shutdown().await?;
    Ok(())
}

/// Automerge queues a change whose dependencies the receiver lacks and
/// `apply_changes` still returns `Ok`. The receive arm must notice and pull.
#[tokio::test]
async fn an_announce_whose_deps_the_peer_lacks_falls_back_to_a_pull() -> Result<()> {
    let (author_dir, author_sync_dir) = (tempdir()?, tempdir()?);
    let (peer_dir, peer_sync_dir) = (tempdir()?, tempdir()?);
    let (fixture, author_sync, peer_sync, peer_backend) = two_sync_nodes(
        author_dir.path(),
        author_sync_dir.path(),
        peer_dir.path(),
        peer_sync_dir.path(),
    )
    .await?;

    let peer_book = IrohPeerBook::new();
    peer_book.upsert(IrohPeerEntry {
        addr: fixture.provider_addr.clone(),
        agent_cid: None,
        libp2p_peer_id: None,
        user_agent: None,
        announced_at_ms: 1,
    });
    assert!(peer_backend.set_pull_back(fixture.fetcher.endpoint().clone(), peer_book, None));

    let author_book = IrohPeerBook::new();
    author_book.upsert(IrohPeerEntry {
        addr: fixture.fetcher.node_addr().await?,
        agent_cid: None,
        libp2p_peer_id: None,
        user_agent: None,
        announced_at_ms: 1,
    });

    // Six authored versions while the peer holds NOTHING: the announced (sixth)
    // change depends on five it has never seen.
    let mut early_head = String::new();
    for v in 1..=6 {
        assert!(
            project_content_doc(
                &author_sync,
                &sample_content(&format!("Version {v}"), &format!("body {v}"))
            )
            .await?
        );
        if v == 1 {
            early_head = author_sync.get_heads(SYNC_NS, DOC_ID).await?[0].clone();
        }
    }
    let head = author_sync.get_heads(SYNC_NS, DOC_ID).await?[0].clone();
    assert!(peer_sync.get_heads(SYNC_NS, DOC_ID).await?.is_empty());

    let pulls_before = announce_pull_ok();
    let accepted = announce_local_change(
        &IrohAnnounceInputs {
            endpoint: fixture.provider.endpoint().clone(),
            book: author_book,
            sync_manager: author_sync.clone(),
        },
        DOC_ID,
        &head,
    )
    .await;
    assert_eq!(
        accepted, 0,
        "the peer must NOT report a landing for a change whose deps it lacks"
    );

    // The pull is detached (the ack returns immediately, as on libp2p), so poll.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        if peer_sync
            .get_doc_field(SYNC_NS, DOC_ID, "title")
            .await
            .ok()
            .as_deref()
            == Some("Version 6")
        {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the peer never converged — a QUEUED (unapplied) change was mistaken for a landing"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert!(
        announce_pull_ok() > pulls_before,
        "convergence here is only possible via the fallback pull"
    );
    // The pull carried the DEPENDENCIES, not just the head.
    assert!(
        peer_sync
            .get_change_by_hash(SYNC_NS, DOC_ID, &early_head)
            .await?
            .is_some(),
        "the peer must hold the doc's earliest change, not just the announced head"
    );
    assert_eq!(
        peer_sync.get_heads(SYNC_NS, DOC_ID).await?,
        author_sync.get_heads(SYNC_NS, DOC_ID).await?,
        "heads must converge"
    );

    fixture.shutdown().await?;
    Ok(())
}
