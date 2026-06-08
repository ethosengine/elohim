//! Acquisition pull-queue e2e — byte-arrival completion proof (spec §11).
//!
//! ## Design
//!
//! `P2PNode`'s acquisition methods (`run_acquisition_reconcile`,
//! `drain_acquisition_queue`) are private and the shard-protocol harness
//! spins a full libp2p swarm — both make a network-level two-node test
//! impractical without large harness extensions. This test follows the
//! same transport-neutral approach as `back_prop_record_predecessor_announce_e2e.rs`:
//! exercise the exact same logic the event loop invokes, without spinning
//! a swarm.
//!
//! ## Production path replicated here
//!
//! 1. `run_acquisition_reconcile` loads active item pins, diffs against
//!    `content_diesel::content_ids_present`, enqueues gaps into
//!    `AcquisitionState::reconcile`.
//! 2. A `ShardRequest::GetContent` is dispatched; on success the response
//!    handler calls `bulk_create_content` then `acquisition.mark_completed`.
//! 3. `rollup()` returns `{total=1, fetched=1, pending=0, caught_up=true}`.
//!
//! This test drives steps 1–3 directly using the public Rust APIs:
//!   - `acquisition_pins::upsert_pin`          — insert an active item pin
//!   - `content_diesel::content_ids_present`   — check local presence
//!   - `AcquisitionState::reconcile`           — diff wants vs local
//!   - `content_diesel::bulk_create_content`   — simulate byte-arrival
//!   - `AcquisitionState::mark_completed`      — wire the completion signal
//!   - `AcquisitionState::rollup`              — assert final counts
//!
//! ## Gap from a full two-node test
//!
//! A network-level test would additionally exercise: shard protocol framing,
//! round-robin peer dispatch, in-flight dedup, and the swarm event loop. Those
//! layers are covered by the shard-protocol tests (`iroh_shard_parity`,
//! `iroh_shard_real_backend`, `epr_atom_federation_integration`). The
//! acquisition *state machine* (reconcile / mark_completed / rollup) is
//! fully proven here. A household-nodes cucumber scenario
//! (`@requires:household-nodes @wip` in
//! `genesis/a2o/features/delivery/acquisition-pins.feature`) will complete
//! the two-node story when that stack is available.

use std::collections::HashSet;

use elohim_storage::db::acquisition_pins;
use elohim_storage::db::content_diesel::{
    bulk_create_content, content_ids_present, CreateContentInput,
};
use elohim_storage::db::context::AppContext;
use elohim_storage::db::models::NewAcquisitionPin;
use elohim_storage::p2p::acquisition::AcquisitionState;
use elohim_storage::test_util::test_pool;

// ─────────────────────────────────────────────────────────────────────────────
// §1  Happy path — item pin + byte-arrival → caught_up=true
//
// Simulates: node B declares a pin for "pull-e2e-1", the acquisition state
// reconciles (item is absent from local projection), byte-arrival lands
// (bulk_create_content + mark_completed), rollup shows caught_up=true.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn item_pin_completes_on_byte_arrival() {
    let pool = test_pool();
    let app_ctx = AppContext::default_lamad();
    let content_id = "pull-e2e-1";

    // ── Step 1: insert an active item pin ────────────────────────────────────
    let mut conn = pool.get().expect("pool connection");
    let pin = acquisition_pins::upsert_pin(
        &mut conn,
        NewAcquisitionPin {
            agent_pub_key: "local-device".into(),
            head_ref: content_id.into(),
            kind: "item".into(),
            closure_rule_json: None,
            priority: 1,
        },
    )
    .expect("upsert_pin must succeed");
    assert_eq!(pin.status, "active");
    assert_eq!(pin.head_ref, content_id);

    // ── Step 2: check local presence (empty — content not yet arrived) ────────
    let want_ids = vec![content_id.to_string()];
    let local_has = content_ids_present(&mut conn, &app_ctx, &want_ids)
        .expect("content_ids_present query must succeed");
    assert!(
        !local_has.contains(content_id),
        "content must NOT be present before byte-arrival"
    );
    drop(conn); // release before async block

    // ── Step 3: reconcile the acquisition state (mirrors run_acquisition_reconcile) ─
    let acq = AcquisitionState::new();
    let pin_wants: Vec<(i32, Vec<String>)> = vec![(pin.id, vec![content_id.to_string()])];
    let to_dispatch = acq.reconcile(pin_wants, &local_has).await;
    assert_eq!(
        to_dispatch,
        vec![content_id.to_string()],
        "reconcile must enqueue the missing item for dispatch"
    );

    // Verify rollup shows pending before byte-arrival
    let pre = acq.rollup().await;
    assert_eq!(pre.total, 1, "total must be 1");
    assert_eq!(pre.fetched, 0, "nothing fetched yet");
    assert_eq!(pre.pending, 1, "one item pending");
    assert!(!pre.caught_up, "must not be caught_up before byte-arrival");

    // ── Step 4: byte-arrival (mirrors bulk_create_content + mark_completed) ──
    let mut conn2 = pool.get().expect("pool connection 2");
    let result = bulk_create_content(
        &mut conn2,
        &app_ctx,
        vec![CreateContentInput {
            id: content_id.to_string(),
            title: "Strawberry Guide".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "public".to_string(),
            created_by: None,
            tags: vec![],
            content_body: Some("# Strawberry Guide\nTest content.".to_string()),
        }],
    )
    .expect("bulk_create_content must succeed");
    assert_eq!(result.inserted, 1, "content row must be inserted");
    assert_eq!(result.errors.len(), 0, "no insertion errors");

    // Signal byte-arrival to acquisition state (mirrors the event loop's
    // post-bulk_create_content call in p2p/mod.rs).
    acq.mark_completed(content_id).await;

    // ── Step 5: assert rollup — byte-arrival completion ──────────────────────
    let post = acq.rollup().await;
    assert_eq!(post.total, 1, "total must still be 1 after completion");
    assert_eq!(post.fetched, 1, "fetched must be 1 after byte-arrival");
    assert_eq!(post.pending, 0, "pending must be 0 after completion");
    assert!(
        post.caught_up,
        "caught_up must be true after all items are fetched (spec R-A)"
    );

    // ── Step 6: verify content row landed in the DB ───────────────────────────
    let present = content_ids_present(&mut conn2, &app_ctx, &[content_id.to_string()])
        .expect("presence query must succeed");
    assert!(
        present.contains(content_id),
        "content must be in the local projection after byte-arrival"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  Negative guard — a failed fetch never increments fetched count or caught_up
//
// Proves: a pin for an id that never byte-arrives does NOT increment fetched,
// AND does NOT report caught_up (spec R-A: byte-arrival complete means
// fetched == total, not merely pending == 0).
//
// With the old `pending == 0 && total > 0` formula, mark_failed would
// transiently set caught_up=true (items leave pending before re-queue).
// The corrected `fetched == total` formula closes that window entirely.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn failed_fetch_never_increments_fetched_count() {
    let pool = test_pool();
    let app_ctx = AppContext::default_lamad();
    let content_id = "pull-e2e-missing-1";

    let mut conn = pool.get().expect("pool connection");
    let pin = acquisition_pins::upsert_pin(
        &mut conn,
        NewAcquisitionPin {
            agent_pub_key: "local-device".into(),
            head_ref: content_id.into(),
            kind: "item".into(),
            closure_rule_json: None,
            priority: 1,
        },
    )
    .expect("upsert_pin must succeed");

    // Local presence: nothing in DB
    let local_has: HashSet<String> =
        content_ids_present(&mut conn, &app_ctx, &[content_id.to_string()])
            .expect("presence query");
    drop(conn);

    let acq = AcquisitionState::new();
    let to_dispatch = acq
        .reconcile(vec![(pin.id, vec![content_id.to_string()])], &local_has)
        .await;
    assert_eq!(to_dispatch.len(), 1, "must enqueue the missing item");

    // Before any attempt: pending=1, fetched=0, caught_up=false
    let r = acq.rollup().await;
    assert_eq!(r.total, 1);
    assert_eq!(r.fetched, 0);
    assert_eq!(r.pending, 1);
    assert!(
        !r.caught_up,
        "must NOT be caught_up before any byte-arrival"
    );

    // Simulate a failed attempt (back-off semantics: item leaves pending,
    // will re-enter on next reconcile cycle — not immediately re-queued).
    acq.mark_failed(content_id).await;

    // Core invariant (spec R-A): fetched must NOT increment on a failed fetch,
    // and caught_up must NOT be true even though pending is transiently 0 —
    // the fix (fetched==total gate) prevents false-completion in this window.
    let r2 = acq.rollup().await;
    assert_eq!(
        r2.fetched, 0,
        "fetched must NOT increment on a failed fetch — only byte-arrival earns completion (spec R-A)"
    );
    assert_eq!(r2.total, 1, "total must remain 1 after failure");
    assert_eq!(
        r2.failed, 1,
        "failed count must be 1 after one failed attempt"
    );
    assert!(
        !r2.caught_up,
        "failed fetch must not false-complete (R-A) — pending==0 but fetched < total"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Resolved-empty desired set — never caught_up (spec §4.3/§10)
//
// Mirrors AcquisitionState's own unit test but exercises the DB layer:
// a pin whose head_ref resolves to zero content ids must surface as
// resolved-empty, NOT caught_up=true.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn zero_item_resolved_set_is_not_caught_up() {
    let pool = test_pool();
    let app_ctx = AppContext::default_lamad();

    let mut conn = pool.get().expect("pool connection");
    let pin = acquisition_pins::upsert_pin(
        &mut conn,
        NewAcquisitionPin {
            agent_pub_key: "local-device".into(),
            // A head_ref that resolves to no items in this test (no content seeded)
            head_ref: "epr:empty-set-pin".into(),
            kind: "item".into(),
            closure_rule_json: None,
            priority: 1,
        },
    )
    .expect("upsert_pin");

    // Simulate Slice-1 resolution that returned zero items (e.g. a cluster pin
    // whose closure resolved empty). In the p2p loop this would be:
    //   pin_wants = vec![(pin.id, vec![])]
    let local_has = content_ids_present(&mut conn, &app_ctx, &[]).expect("presence query");
    drop(conn);

    let acq = AcquisitionState::new();
    // Reconcile with an empty desired set for this pin
    let to_dispatch = acq.reconcile(vec![(pin.id, vec![])], &local_has).await;
    assert!(
        to_dispatch.is_empty(),
        "zero desired set must dispatch nothing"
    );

    let r = acq.rollup().await;
    assert_eq!(r.total, 0, "total must be 0 for empty desired set");
    assert!(
        !r.caught_up,
        "zero-item desired set must NOT be caught_up (spec §4.3/§10 — never silently false-complete)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  Cluster pin is not stored — the HTTP layer rejects kind=="cluster" at 501
//     (pure DB assertion: upsert with kind "cluster" still succeeds at the DB
//     layer — the guard lives in the HTTP handler, not in the DB function;
//     this test pins that separation-of-concerns invariant).
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cluster_pin_is_db_insertable_but_http_guards_it() {
    // At the DB layer a cluster pin can be inserted (no enum constraint in SQLite).
    // The HTTP 501 guard lives in handle_create_pin in http.rs. This is the
    // correct separation: the DB is schema-generic; the API enforces the contract.
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");

    let pin = acquisition_pins::upsert_pin(
        &mut conn,
        NewAcquisitionPin {
            agent_pub_key: "local-device".into(),
            head_ref: "epr:cluster-candidate".into(),
            kind: "cluster".into(), // NOT "item"
            closure_rule_json: None,
            priority: 1,
        },
    )
    .expect("DB layer accepts cluster kind");

    assert_eq!(
        pin.kind, "cluster",
        "DB stores what it receives — the guard is in the HTTP layer"
    );

    // Verify list_active_pins returns the cluster pin (DB is kind-agnostic).
    let active = acquisition_pins::list_active_pins(&mut conn).expect("list_active_pins");
    assert!(
        active.iter().any(|p| p.kind == "cluster"),
        "DB returns cluster pins in list_active — the HTTP GET /api/v1/pins filter is in http.rs"
    );
}
