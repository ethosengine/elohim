//! Component proof of the production receiving handler, not a network-wire test.
//! Deliberately deliver a private record that an honest sender would withhold.

use crate::blob_store::BlobStore;
use crate::db::{content_diesel, AppContext, DbPool};
use crate::p2p::acquisition::AcquisitionState;
use crate::p2p::replication::ReplicationState;
use crate::p2p::shard_protocol::ContentRecord;
use crate::p2p::{store_acquired_record, AcquisitionIngestCtx, StoredRecord};
use crate::private_reach::WithholdReason;
use crate::services::custody_standing::{FakeCustodyStanding, Requester};
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;

const RECEIVER: &str = "uhCAkReceiverFixtureAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const WARD: &str = "uhCAkWardFixtureAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
// Content identity from the measured witness fixture; no alternate CID encoder.
const CID: &str = "bafyreigwunm4vxkdsmu6frmcc6jv2yzwy73dwsxlsj5bqdp4gqhkktaaya";
const DIGEST: &str = "d6a359cadd439329e2c58217935d6336c7f63b4aeb927a180dfc340ea54c00c0";

fn test_pool() -> DbPool {
    let url = format!(
        "file:private_ingest_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<diesel::SqliteConnection>::new(url))
        .expect("isolated pool");
    crate::db::run_migrations(&pool).expect("migrated production schema");
    pool
}

fn delivered_record() -> ContentRecord {
    ContentRecord {
        id: CID.to_string(),
        title: "Private witness delivered without permission".to_string(),
        description: None,
        content_type: "issue-report".to_string(),
        content_format: "json".to_string(),
        blob_hash: Some(format!("sha256-{DIGEST}")),
        blob_cid: None,
        content_size_bytes: None,
        metadata_json: Some(r#"{"kind":"death-witness"}"#.to_string()),
        reach: "private".to_string(),
        // The payload must not manufacture standing by claiming this receiver.
        created_by: Some(RECEIVER.to_string()),
        tags: vec![],
        content_body: None,
    }
}

async fn assert_handler_refuses_then_accepts_custody(sender: Requester) {
    let directory = tempfile::tempdir().expect("isolated byte store");
    let blob_store = Arc::new(BlobStore::new(directory.path()).await.expect("blob store"));
    let pool = test_pool();
    let standing = Arc::new(FakeCustodyStanding::new());
    standing
        .bind(&Requester::local(), RECEIVER)
        .bind(&sender, WARD);
    let ctx = AcquisitionIngestCtx {
        db_pool: Some(pool.clone()),
        replication_state: ReplicationState::new(),
        acquisition: AcquisitionState::new(),
        blob_store: blob_store.clone(),
        self_cid: "fixture-transport-not-agent".to_string(),
        write_gate: Arc::new(tokio::sync::Mutex::new(())),
        custody_standing: Some(standing.clone()),
    };
    assert_eq!(
        ctx.replication_state
            .discover(vec![CID.to_string()])
            .await
            .len(),
        1
    );
    let before = crate::metrics::private_preauth_skipped_count(WithholdReason::NoStanding);
    let outcome = store_acquired_record(&ctx, sender.clone(), delivered_record()).await;
    assert!(matches!(outcome, StoredRecord::Failed));
    assert!(
        crate::metrics::private_preauth_skipped_count(WithholdReason::NoStanding) > before,
        "the deliberately delivered private record must reach the preauthorization gate"
    );
    assert!(content_diesel::get_content(
        &mut pool.get().expect("read connection"),
        &AppContext::default_lamad(),
        CID,
        content_diesel::MinTrust::Invisible,
    )
    .expect("read rejected record")
    .is_none());
    assert!(!blob_store.exists(&format!("sha256-{DIGEST}")).await);
    let status = ctx.replication_state.status().await;
    assert_eq!(status.pending, 0);
    assert_eq!(status.completed, 0);

    // Same handler, DB, sender and record; only actual fixture standing changes.
    standing.blob_custody(RECEIVER, DIGEST);
    let outcome = store_acquired_record(&ctx, sender, delivered_record()).await;
    assert!(matches!(outcome, StoredRecord::Stored { .. }));
    let row = content_diesel::get_content(
        &mut pool.get().expect("read connection"),
        &AppContext::default_lamad(),
        CID,
        content_diesel::MinTrust::Invisible,
    )
    .expect("read accepted record")
    .expect("positive custody control persists exact CID");
    assert_eq!(row.id, CID);
    assert_eq!(row.reach, "private");
    assert_eq!(ctx.replication_state.status().await.completed, 1);
}

#[tokio::test]
async fn libp2p_private_receiver_handler_requires_custody_before_persistence() {
    assert_handler_refuses_then_accepts_custody(Requester::libp2p("fixture-libp2p-sender")).await;
}

#[tokio::test]
async fn iroh_private_receiver_handler_requires_custody_before_persistence() {
    assert_handler_refuses_then_accepts_custody(Requester::iroh("fixture-iroh-sender")).await;
}
