//! Projection shading — an operational (Path C) admin verb that HIDES one
//! kind of this peer's projections from the read route the doorway's EPR
//! router refresh consults, without deleting anything.
//!
//! ## Why this exists
//!
//! `genesis/a2o/features/federation/doorway-pool-degrade.feature` needs the
//! degraded-primary incident shape: a doorway's PRIMARY storage answering
//! ZERO `project-epr` rows while a pool peer still holds them, so the
//! doorway's pool fallback (`FallbackOutcome::PeerServed`, the `EPR router
//! DEGRADED` WARN) is exercised rather than merely observed. The lane could
//! previously only wait for that shape to occur on its own: DELETING the rows
//! races the 30 s projection reconcile that heals exactly that gap, and
//! deleting truth to construct a test premise is the wrong move anyway.
//!
//! Shading is the honest construction. Nothing is deleted; writes, sync and
//! reconcile are untouched, so the reconcile has nothing to race and a restore
//! loses nothing.
//!
//! ## Truth class — operational (Path C), deliberately
//!
//! The shaded set is IN-PROCESS state on ONE peer:
//!
//! - never persisted (reset on restart — a restarted peer serves its rows
//!   again, which is the safe direction),
//! - never gossiped and never projected (shading one peer says nothing about
//!   any other peer; that per-peer independence is the whole point — the a2o
//!   lane shades the primary and leaves the pool peer serving),
//! - never a write path. It masks ONE read, and the rows it masks stay in
//!   SQLite exactly as the reconciler left them.
//!
//! Nothing here reaches the DHT, the REA ledger, or any peer's projection.
//!
//! ## Posture
//!
//! Node-local, like every other `/admin` route on elohim-storage (see the
//! `/admin/lineage/*`, `/admin/runtime-config` and `/admin/coordinators/sync`
//! arms in `http.rs`): the storage admin surface is not exposed through the
//! doorway and carries no separate key of its own today. The a2o glue sends
//! `X-API-Key` and this route ignores it, exactly as its siblings do — if the
//! storage admin surface ever grows a key check, it belongs on all of those
//! arms at once, not on this one alone.

use std::collections::BTreeSet;
use std::sync::{OnceLock, RwLock};

use bytes::Bytes;
use http_body_util::Full;
use hyper::Response;
use serde::Deserialize;
use tracing::warn;

use super::response;

/// The only shadeable kind today: the `project-epr` REA action whose rows the
/// doorway EPR router reads from
/// `GET /db/rea_commitments?action=project-epr&doorwayId={id}`.
pub const PROJECT_EPR: &str = "project-epr";

/// Every kind this verb accepts. A 400 names this list verbatim.
pub const ACCEPTED_KINDS: &[&str] = &[PROJECT_EPR];

/// Canonicalise a requested kind to its `'static` form, or `None` if unknown.
/// Storing `&'static str` keeps the set allocation-free and makes an unknown
/// kind unrepresentable once past this function.
fn canonical(kind: &str) -> Option<&'static str> {
    ACCEPTED_KINDS.iter().copied().find(|k| *k == kind.trim())
}

fn shaded_set() -> &'static RwLock<BTreeSet<&'static str>> {
    static SHADED: OnceLock<RwLock<BTreeSet<&'static str>>> = OnceLock::new();
    SHADED.get_or_init(|| RwLock::new(BTreeSet::new()))
}

/// Is this kind currently shaded on this peer? The read-path guard.
///
/// Takes the raw kind string (not a canonicalised one) so callers on the read
/// path need no knowledge of the vocabulary: an unknown kind is never shaded.
pub fn is_shaded(kind: &str) -> bool {
    let Some(kind) = canonical(kind) else {
        return false;
    };
    shaded_set()
        .read()
        .map(|set| set.contains(kind))
        .unwrap_or(false)
}

/// The kinds currently shaded, sorted — the wire value of `shaded` in both
/// the GET and POST responses. `[]` when none.
pub fn shaded_kinds() -> Vec<String> {
    shaded_set()
        .read()
        .map(|set| set.iter().map(|k| (*k).to_string()).collect())
        .unwrap_or_default()
}

/// Arm or disarm shading for one kind. `Err` carries the accepted-kinds
/// message for an unknown kind; `Ok` carries the post-change shaded list.
///
/// Logs at WARN on every state CHANGE (arm and disarm both) — a peer that is
/// hiding rows it holds must say so in its own log, and the disarm line is
/// what proves the teardown ran. A no-op repeat is silent.
pub fn set_shaded(kind: &str, shaded: bool) -> Result<Vec<String>, String> {
    let Some(kind) = canonical(kind) else {
        return Err(format!(
            "unknown projection kind {:?}; accepted kinds: {}",
            kind,
            ACCEPTED_KINDS.join(", ")
        ));
    };

    let changed = {
        let mut set = shaded_set()
            .write()
            .map_err(|e| format!("projection shade state poisoned: {e}"))?;
        if shaded {
            set.insert(kind)
        } else {
            set.remove(kind)
        }
    };

    if changed {
        if shaded {
            warn!(
                kind,
                "projections SHADED: this peer will answer 0 rows of this kind on \
                 GET /db/rea_commitments until disarmed or restarted; nothing is \
                 deleted and writes/sync/reconcile are untouched"
            );
        } else {
            warn!(
                kind,
                "projections UN-SHADED: this peer serves its rows of this kind again"
            );
        }
        set_gauge(kind, shaded);
    }

    Ok(shaded_kinds())
}

/// `elohim_projections_shaded{kind}` — 0/1, materialised on first change so a
/// peer that never shaded reports no series rather than a fabricated zero.
fn set_gauge(kind: &'static str, shaded: bool) {
    crate::metrics::ELOHIM_PROJECTIONS_SHADED
        .with_label_values(&[kind])
        .set(i64::from(shaded));
}

/// Body of `POST /admin/projections/shade`.
#[derive(Debug, Deserialize)]
pub struct ShadeRequest {
    pub kind: String,
    pub shaded: bool,
}

#[derive(Debug, serde::Serialize)]
struct ShadeStateResponse {
    shaded: Vec<String>,
}

/// `GET /admin/projections/shade` → 200 `{"shaded":[...]}`.
pub fn handle_get() -> Response<Full<Bytes>> {
    response::ok(&ShadeStateResponse {
        shaded: shaded_kinds(),
    })
}

/// `POST /admin/projections/shade` with body `{"kind":"…","shaded":bool}`
/// → 200 `{"shaded":[…]}`, or 400 naming the accepted kinds.
///
/// Pure over the body bytes so the HTTP arm stays a body-read plus one call,
/// and so the 400 path is unit-testable without constructing a request.
pub fn handle_post(body_bytes: &[u8]) -> Response<Full<Bytes>> {
    let request: ShadeRequest = match serde_json::from_slice(body_bytes) {
        Ok(request) => request,
        Err(e) => {
            return response::bad_request(&format!(
                "expected {{\"kind\":\"<kind>\",\"shaded\":true|false}} \
                 (accepted kinds: {}): {e}",
                ACCEPTED_KINDS.join(", ")
            ))
        }
    };

    match set_shaded(&request.kind, request.shaded) {
        Ok(shaded) => response::ok(&ShadeStateResponse { shaded }),
        Err(message) => response::bad_request(&message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shaded set is process-global, so these tests serialise against each
    /// other rather than racing under the default parallel test harness — the
    /// same discipline that keeps hot-path env vars out of tests.
    ///
    /// A `tokio::sync::Mutex` (not `std::sync::Mutex`) because the async tests
    /// below hold this guard across `.await` points for the whole test body —
    /// that's the serialisation guarantee working as intended, not a bug — and
    /// only an async-aware lock lets a guard span an await without tripping
    /// `clippy::await_holding_lock`'s deadlock-shape lint (a std guard held
    /// across an await can park the executor thread while still holding the
    /// lock; tokio's guard cooperates with the runtime instead).
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// Guard for plain `#[test]` (synchronous) functions.
    fn guard() -> tokio::sync::MutexGuard<'static, ()> {
        let g = LOCK.blocking_lock();
        // Every test starts from the un-shaded state regardless of what the
        // previous one left behind.
        let _ = set_shaded(PROJECT_EPR, false);
        g
    }

    /// Guard for `#[tokio::test]` (async) functions — awaits the lock instead
    /// of blocking the runtime thread for it.
    async fn guard_async() -> tokio::sync::MutexGuard<'static, ()> {
        let g = LOCK.lock().await;
        // Every test starts from the un-shaded state regardless of what the
        // previous one left behind.
        let _ = set_shaded(PROJECT_EPR, false);
        g
    }

    async fn json_of(response: Response<Full<Bytes>>) -> (u16, serde_json::Value) {
        let status = response.status().as_u16();
        let bytes = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("collect body")
            .to_bytes();
        let value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&bytes).into()));
        (status, value)
    }

    #[test]
    fn unshaded_is_the_boot_state() {
        let _g = guard();
        assert!(!is_shaded(PROJECT_EPR));
        assert_eq!(shaded_kinds(), Vec::<String>::new());
    }

    #[test]
    fn arm_then_disarm_round_trips() {
        let _g = guard();

        let armed = set_shaded(PROJECT_EPR, true).expect("arm must succeed");
        assert_eq!(armed, vec![PROJECT_EPR.to_string()]);
        assert!(is_shaded(PROJECT_EPR));

        let disarmed = set_shaded(PROJECT_EPR, false).expect("disarm must succeed");
        assert!(disarmed.is_empty(), "disarm must leave the set empty");
        assert!(!is_shaded(PROJECT_EPR));
    }

    #[test]
    fn arming_is_idempotent() {
        let _g = guard();
        set_shaded(PROJECT_EPR, true).unwrap();
        let again = set_shaded(PROJECT_EPR, true).unwrap();
        assert_eq!(again, vec![PROJECT_EPR.to_string()]);
    }

    #[test]
    fn unknown_kind_is_rejected_and_names_the_accepted_kinds() {
        let _g = guard();
        let err = set_shaded("project-blob", true).expect_err("unknown kind must be rejected");
        assert!(err.contains("project-blob"), "message must name the input");
        assert!(
            err.contains(PROJECT_EPR),
            "message must name the accepted kinds: {err}"
        );
        assert!(
            shaded_kinds().is_empty(),
            "a rejected kind must not change state"
        );
    }

    #[test]
    fn an_unknown_kind_is_never_shaded_on_the_read_path() {
        let _g = guard();
        set_shaded(PROJECT_EPR, true).unwrap();
        assert!(is_shaded(PROJECT_EPR));
        assert!(
            !is_shaded("custody-blob"),
            "shading project-epr must leave every other kind untouched"
        );
    }

    #[tokio::test]
    async fn get_reflects_the_current_state() {
        let _g = guard_async().await;

        let (status, value) = json_of(handle_get()).await;
        assert_eq!(status, 200);
        assert_eq!(value, serde_json::json!({ "shaded": [] }));

        set_shaded(PROJECT_EPR, true).unwrap();
        let (status, value) = json_of(handle_get()).await;
        assert_eq!(status, 200);
        assert_eq!(value, serde_json::json!({ "shaded": ["project-epr"] }));
    }

    #[tokio::test]
    async fn post_arms_disarms_and_400s_on_an_unknown_kind() {
        let _g = guard_async().await;

        let (status, value) =
            json_of(handle_post(br#"{"kind":"project-epr","shaded":true}"#)).await;
        assert_eq!(status, 200);
        assert_eq!(value, serde_json::json!({ "shaded": ["project-epr"] }));
        assert!(is_shaded(PROJECT_EPR));

        let (status, value) = json_of(handle_post(br#"{"kind":"nope","shaded":true}"#)).await;
        assert_eq!(status, 400, "unknown kind must be a 400: {value}");
        assert!(
            value.to_string().contains(PROJECT_EPR),
            "400 body must name the accepted kinds: {value}"
        );
        assert!(
            is_shaded(PROJECT_EPR),
            "a rejected request must not disturb existing state"
        );

        let (status, value) =
            json_of(handle_post(br#"{"kind":"project-epr","shaded":false}"#)).await;
        assert_eq!(status, 200);
        assert_eq!(value, serde_json::json!({ "shaded": [] }));
        assert!(!is_shaded(PROJECT_EPR));
    }

    #[tokio::test]
    async fn malformed_body_is_a_400() {
        let _g = guard_async().await;
        let (status, _) = json_of(handle_post(b"not json")).await;
        assert_eq!(status, 400);
    }

    /// The read path itself: shading must make
    /// `GET /db/rea_commitments?action=project-epr&doorwayId=…` answer 200
    /// with ZERO rows while the rows stay in SQLite, and un-shading must bring
    /// exactly those rows back.
    #[tokio::test]
    async fn shading_empties_the_epr_read_route_without_deleting_rows() {
        use crate::db::context::AppContext;
        use crate::db::rea_commitments::{
            create_commitment, find_active_projections, CreateReaCommitmentInput,
        };

        let _g = guard_async().await;

        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        {
            let mut conn = pool.get().unwrap();
            let metadata = serde_json::json!({
                "urlPath": "/lamad",
                "mode": "cached",
                "reach": "commons",
                "baseHref": "/lamad/",
                "entryFile": "index.html",
                "redirectsFrom": [],
                "gateHints": [],
                "deadEnd": false
            });
            create_commitment(
                &mut conn,
                &ctx,
                CreateReaCommitmentInput {
                    id: Some("lamad-shade-test".into()),
                    action: PROJECT_EPR.into(),
                    provider: "test-steward".into(),
                    receiver: "test-operator".into(),
                    in_scope_of: Some("doorway:alpha-elohim-host|epr:lamad-spa".into()),
                    note: Some("lamad-spa".into()),
                    metadata_json: Some(metadata.to_string()),
                    ..Default::default()
                },
            )
            .expect("seed project-epr commitment");
        }

        let blob_store = std::sync::Arc::new(
            crate::blob_store::BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let server = crate::http::HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
            .with_db_pool(pool.clone());

        let before = server
            .test_get_db_rea_commitments(PROJECT_EPR, "alpha-elohim-host")
            .await;
        assert_eq!(before.status, 200);
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&before.body).unwrap();
        assert_eq!(rows.len(), 1, "precondition: the peer serves its row");

        set_shaded(PROJECT_EPR, true).unwrap();
        let shaded = server
            .test_get_db_rea_commitments(PROJECT_EPR, "alpha-elohim-host")
            .await;
        assert_eq!(shaded.status, 200, "a shaded read is 200, never an error");
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&shaded.body).unwrap();
        assert!(
            rows.is_empty(),
            "shaded read must answer a bare empty array, got {rows:?}"
        );

        // Nothing was deleted — the DB still holds the row the read hid.
        {
            let mut conn = pool.get().unwrap();
            let still_there = find_active_projections(&mut conn, &ctx, "alpha-elohim-host")
                .expect("db read must still succeed while shaded");
            assert_eq!(
                still_there.len(),
                1,
                "shading must NOT delete rows — the reconcile has nothing to race"
            );
        }

        set_shaded(PROJECT_EPR, false).unwrap();
        let after = server
            .test_get_db_rea_commitments(PROJECT_EPR, "alpha-elohim-host")
            .await;
        assert_eq!(after.status, 200);
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&after.body).unwrap();
        assert_eq!(rows.len(), 1, "un-shading must bring the rows back");
        assert_eq!(
            after.body, before.body,
            "un-shaded response must be byte-identical to the pre-shade one"
        );
    }

    /// Parameter validation still runs while shaded: a bad action is a 400,
    /// not a shaded empty 200.
    #[tokio::test]
    async fn shading_does_not_swallow_parameter_validation() {
        let _g = guard_async().await;
        set_shaded(PROJECT_EPR, true).unwrap();

        let blob_store = std::sync::Arc::new(
            crate::blob_store::BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let server = crate::http::HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
            .with_db_pool(crate::test_util::test_pool());

        let bad_action = server
            .test_get_db_rea_commitments("wrong-action", "alpha-elohim-host")
            .await;
        assert_ne!(bad_action.status, 200, "a wrong action is still rejected");

        let no_doorway = server.test_get_db_rea_commitments(PROJECT_EPR, "").await;
        assert_ne!(
            no_doorway.status, 200,
            "a missing doorwayId is still rejected"
        );
    }
}
