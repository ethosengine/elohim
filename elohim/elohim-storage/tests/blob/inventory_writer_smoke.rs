//! Smoke test for the inventory projection writer's apply paths.
//! Exercises `rebase_after_recon` (the one cursor re-base) and `apply_delta`
//! against an in-memory pool, mirroring what the live receive arm in p2p/mod.rs does.

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use elohim_epr::address::compute_bytes_cid;
use elohim_storage::db::peer_blob_inventory::{
    apply_delta, lookup_hosts, rebase_after_recon, DeltaApplyOutcome, InventoryWitness,
};

fn cid(label: &str) -> String {
    compute_bytes_cid(label.as_bytes()).to_string()
}
use elohim_storage::db::{run_migrations, DbPool};

fn test_pool(prefix: &str) -> DbPool {
    let url = format!(
        "file:iws_test_{}_{}?mode=memory&cache=shared",
        prefix,
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(&url))
        .expect("pool");
    run_migrations(&pool).expect("migrations");
    pool
}

#[test]
fn writer_applies_recon_rebase_then_in_order_deltas() {
    let pool = test_pool("snap_delta");
    let mut conn = pool.get().unwrap();

    rebase_after_recon(
        &mut conn,
        "12D3KooWtest1",
        &[cid("a"), cid("b")],
        &[],
        1,
        "2026-05-02T00:00:00Z",
    )
    .expect("recon rebase");

    let outcome = apply_delta(
        &mut conn,
        "12D3KooWtest1",
        &[cid("c")],
        &[],
        2,
        "2026-05-02T00:00:30Z",
        InventoryWitness::SignedAuthor,
    )
    .expect("delta");
    assert_eq!(outcome, DeltaApplyOutcome::Applied);

    let hosts = lookup_hosts(&mut conn, &cid("c"), "2026-05-01T00:00:00Z").unwrap();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].peer_id, "12D3KooWtest1");
}

#[test]
fn writer_detects_gap_on_out_of_order_delta() {
    let pool = test_pool("gap_detect");
    let mut conn = pool.get().unwrap();

    rebase_after_recon(
        &mut conn,
        "12D3KooWtest2",
        &[cid("a")],
        &[],
        1,
        "2026-05-02T00:00:00Z",
    )
    .expect("recon rebase");

    let outcome = apply_delta(
        &mut conn,
        "12D3KooWtest2",
        &[cid("b")],
        &[],
        5,
        "2026-05-02T00:01:00Z",
        InventoryWitness::SignedAuthor,
    )
    .expect("delta");
    match outcome {
        DeltaApplyOutcome::Gap { expected, received } => {
            assert_eq!(expected, 2);
            assert_eq!(received, 5);
        }
        other => panic!("expected Gap, got {other:?}"),
    }
}
