//! Seam A's chain contract (interfaces spec §A.4) run against eprfs-core's `TierChain` through
//! eprfs-serve's harness, and the leaf contract against this crate's stores.

use std::sync::Arc;

use bytes::Bytes;
use eprfs_core::{BlobCid, EprfsError, EprfsStorage, FetchPolicy, TierChain};
use eprfs_serve::testing::{assert_leaf_contract, assert_tier_contract, TierContractSetup};
use eprfs_storage::{DirectoryStorage, MemoryStorage, OffloadedDirectory};

fn tier_chain(setup: TierContractSetup) -> Arc<dyn EprfsStorage> {
    Arc::new(TierChain::new(
        setup.own,
        setup.records,
        setup.order,
        setup.connector.as_ref(),
        setup.ask,
        setup.gate,
        setup.who,
    ))
}

#[tokio::test]
async fn tier_chain_passes_the_tier_contract() {
    assert_tier_contract(&tier_chain).await;
}

#[tokio::test]
async fn memory_and_directory_storage_pass_the_leaf_contract() {
    let held_bytes = Bytes::from_static(b"held by the leaf");
    let held = BlobCid::compute_raw(&held_bytes);
    let missing = BlobCid::compute_raw(b"never stored");

    let memory = MemoryStorage::default();
    memory.insert_blob(held.clone(), held_bytes.clone()).await;
    assert_leaf_contract(&memory, &held, &missing).await;

    let dir = tempfile::tempdir().unwrap();
    let directory = DirectoryStorage::create(dir.path().join("objects")).unwrap();
    directory
        .put_blob_verified(&held, held_bytes)
        .await
        .unwrap();
    assert_leaf_contract(&directory, &held, &missing).await;
}

/// Interfaces spec §4, Blocking I/O: the directory store behind tokio's blocking pool answers as
/// the store itself does, on a multi-thread runtime, and a corrupt object is the typed
/// `EprfsError::Corrupt` through either.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offloaded_directory_passes_the_leaf_contract_on_a_multi_thread_runtime() {
    let held_bytes = Bytes::from_static(b"held behind the blocking pool");
    let held = BlobCid::compute_raw(&held_bytes);
    let missing = BlobCid::compute_raw(b"never stored");
    let dir = tempfile::tempdir().unwrap();
    let objects = dir.path().join("objects");
    let offloaded = OffloadedDirectory::new(DirectoryStorage::create(&objects).unwrap());
    offloaded
        .put_blob_verified(&held, held_bytes)
        .await
        .unwrap();
    assert_leaf_contract(&offloaded, &held, &missing).await;

    // Overwriting keeps the object's owner-only mode, so only the bytes are wrong.
    std::fs::write(objects.join(held.to_string()), b"corrupt").unwrap();
    for store in [&offloaded as &dyn EprfsStorage, offloaded.inner()] {
        match store.fetch_blob(&held, FetchPolicy::LocalOnly).await {
            Err(EprfsError::Corrupt(cid)) => assert_eq!(cid, held),
            other => panic!("expected Corrupt, got {other:?}"),
        }
    }
}
