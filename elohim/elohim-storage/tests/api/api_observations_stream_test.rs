//! `GET /api/v1/observations/stream` — a person's own lifestream (plan task
//! A5, ruling R-A4 of the post-station-4 sprint).
//!
//! The handler reads the explicit `X-Agent-Cid` header and the query string,
//! then delegates to `api::observations::stream_observations`, which these
//! tests drive directly (the codebase has no hyper test harness; see
//! `api_observations_write_test.rs`). Each error is mapped through
//! `services::response::error_response`, so the asserted status IS the status
//! the route answers with.
//!
//! What the route promises:
//! - reach is the requester's own rows: `observer_cid == X-Agent-Cid`, no
//!   header → 401, never the `local_sessions` fallback;
//! - the view is rendered through the observation-lifestream recipe: ranked
//!   newest first then longest dwell, windowed (default 7d), filtered by a
//!   declared lens (unknown → 400), and it prints the recipe's CID;
//! - what it cannot show or vouch for rides in `omissions[]`: rows outside
//!   the window, payloads that did not parse, the absent signature.

use diesel::prelude::*;
use elohim_storage::api::observations::stream_observations;
use elohim_storage::db::diesel_schema::observations;
use elohim_storage::db::models::NewObservationRow;
use elohim_storage::error::StorageError;
use elohim_storage::observation::recipe::recipe_cid;
use elohim_storage::services::response::error_response;
use elohim_storage::test_util::test_pool;
use elohim_storage::views::ObservationStreamView;
use hyper::StatusCode;
use serde_json::{json, Value};
use std::path::PathBuf;

const NOW: i64 = 1_790_000_000;
const DAY: i64 = 86_400;
const JESSICA: &str = "human-jessica";
const JAMES: &str = "human-james";

fn status_of(err: StorageError) -> StatusCode {
    error_response(err).status()
}

fn payload(dwell_ms: u64, depth: u8) -> String {
    json!({ "ref_cid": "bafy", "dwell_ms": dwell_ms, "scroll_depth_pct": depth }).to_string()
}

/// Seed one row directly (the write path refuses the malformed payloads and
/// back-dated rows some of these tests need).
fn seed(
    conn: &mut SqliteConnection,
    observer: &str,
    offset: i64,
    observed_at: i64,
    kind: &str,
    subject: &str,
    payload_json: &str,
) {
    let log_cid = format!("blake3:{observer}");
    diesel::insert_into(observations::table)
        .values(NewObservationRow {
            observer_cid: observer,
            log_cid: &log_cid,
            log_offset: offset,
            observed_at,
            seq: offset + 1,
            observation_kind: kind,
            subject_cid: Some(subject),
            subject_kind: Some("content"),
            payload_json,
            observer_household_cid: None,
            observer_collective_cid: None,
            observer_region: None,
            observer_archetype: None,
            observer_compute_class: None,
            signature_b64: "",
        })
        .execute(conn)
        .expect("seed observation");
}

fn viewed(
    conn: &mut SqliteConnection,
    observer: &str,
    offset: i64,
    observed_at: i64,
    subject: &str,
    dwell_ms: u64,
    depth: u8,
) {
    seed(
        conn,
        observer,
        offset,
        observed_at,
        "lamad:content-viewed",
        subject,
        &payload(dwell_ms, depth),
    );
}

fn stream(conn: &mut SqliteConnection, who: &str, query: &str) -> ObservationStreamView {
    stream_observations(conn, Some(who), query, NOW).expect("stream renders")
}

fn subjects(view: &ObservationStreamView) -> Vec<&str> {
    view.entries
        .iter()
        .map(|e| e.subject_cid.as_deref().unwrap_or(""))
        .collect()
}

fn omission_named<'a>(view: &'a ObservationStreamView, name: &str) -> Option<&'a String> {
    let prefix = format!("{name}:");
    view.omissions.iter().find(|line| line.starts_with(&prefix))
}

/// Validate an instance against a view schema (the schema's refs are all
/// local `#/$defs/…`, so no cross-file inlining is needed).
fn validate_against_schema(schema_path_str: &str, instance: &Value) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../sdk/schemas/v1")
        .join(schema_path_str);
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read schema {}: {e}", path.display())),
    )
    .expect("schema parses");
    let validator = jsonschema::validator_for(&schema)
        .unwrap_or_else(|e| panic!("compile schema {schema_path_str}: {e}"));
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("  - {} (at {})", e, e.instance_path))
        .collect();
    assert!(
        errors.is_empty(),
        "Schema validation failed for {schema_path_str}:\n{}\n\nInstance:\n{}",
        errors.join("\n"),
        serde_json::to_string_pretty(instance).unwrap()
    );
}

#[test]
fn stream_without_agent_header_is_401() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    viewed(&mut conn, JESSICA, 0, NOW - 60, "bafy-j1", 3000, 50);

    let err = stream_observations(&mut conn, None, "", NOW).unwrap_err();
    assert_eq!(status_of(err), StatusCode::UNAUTHORIZED);

    // An empty header asserts nothing either.
    let err = stream_observations(&mut conn, Some(""), "", NOW).unwrap_err();
    assert_eq!(status_of(err), StatusCode::UNAUTHORIZED);
}

#[test]
fn stream_returns_only_requesters_rows() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    viewed(&mut conn, JESSICA, 0, NOW - 300, "bafy-jessica-1", 3000, 50);
    viewed(&mut conn, JESSICA, 1, NOW - 200, "bafy-jessica-2", 4000, 60);
    viewed(&mut conn, JAMES, 0, NOW - 250, "bafy-james-1", 9000, 90);
    viewed(&mut conn, JAMES, 1, NOW - 150, "bafy-james-2", 9000, 90);
    viewed(&mut conn, JAMES, 2, NOW - 100, "bafy-james-3", 9000, 90);

    let view = stream(&mut conn, JESSICA, "");
    assert_eq!(view.total_count, 2);
    assert_eq!(subjects(&view), vec!["bafy-jessica-2", "bafy-jessica-1"]);
    let wire = serde_json::to_string(&view).unwrap();
    assert!(!wire.contains("bafy-james"), "james's rows leaked: {wire}");

    // There is no knob naming another observer: a query that tries is not
    // honoured, and the stream is still the header's own.
    let view = stream(&mut conn, JESSICA, "observerCid=human-james");
    assert_eq!(subjects(&view), vec!["bafy-jessica-2", "bafy-jessica-1"]);

    let view = stream(&mut conn, JAMES, "");
    assert_eq!(view.total_count, 3);
    assert!(subjects(&view).iter().all(|s| s.starts_with("bafy-james")));
}

#[test]
fn stream_is_time_ranked_then_dwell() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    viewed(&mut conn, JESSICA, 0, NOW - 600, "bafy-old-short", 100, 10);
    viewed(&mut conn, JESSICA, 1, NOW - 600, "bafy-old-long", 5000, 10);
    viewed(&mut conn, JESSICA, 2, NOW - 60, "bafy-new-tiny", 10, 10);
    viewed(&mut conn, JESSICA, 3, NOW - 3600, "bafy-oldest", 90_000, 10);

    let view = stream(&mut conn, JESSICA, "");
    assert_eq!(
        subjects(&view),
        vec![
            "bafy-new-tiny",
            "bafy-old-long",
            "bafy-old-short",
            "bafy-oldest"
        ]
    );
    assert_eq!(view.entries[1].dwell_ms, 5000);
}

#[test]
fn window_default_7d_excludes_older_rows_and_counts_them_in_omissions() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    viewed(&mut conn, JESSICA, 0, NOW - DAY, "bafy-1d", 1000, 10);
    viewed(&mut conn, JESSICA, 1, NOW - 6 * DAY, "bafy-6d", 1000, 10);
    viewed(&mut conn, JESSICA, 2, NOW - 8 * DAY, "bafy-8d", 1000, 10);
    viewed(&mut conn, JESSICA, 3, NOW - 30 * DAY, "bafy-30d", 1000, 10);

    let view = stream(&mut conn, JESSICA, "");
    assert_eq!(view.window, "7d", "the recipe's default window");
    assert_eq!(view.as_of, NOW, "asOf defaults to now");
    assert_eq!(subjects(&view), vec!["bafy-1d", "bafy-6d"]);
    assert_eq!(
        view.total_count, 4,
        "the lens selected four before the window"
    );
    let line = omission_named(&view, "window").expect("a window omission line");
    assert!(line.contains('2') && line.contains("7d"), "{line}");

    // An explicit window and asOf move the frame.
    let view = stream(&mut conn, JESSICA, "window=48h");
    assert_eq!(view.window, "48h");
    assert_eq!(subjects(&view), vec!["bafy-1d"]);
    let view = stream(
        &mut conn,
        JESSICA,
        &format!("asOf={}&window=2d", NOW - 5 * DAY),
    );
    assert_eq!(view.as_of, NOW - 5 * DAY);
    assert_eq!(subjects(&view), vec!["bafy-6d"]);

    // Everything inside the window: no window line.
    let view = stream(&mut conn, JESSICA, "window=60d");
    assert_eq!(view.entries.len(), 4);
    assert!(
        omission_named(&view, "window").is_none(),
        "{:?}",
        view.omissions
    );

    // The window grammar is Nd | Nh only.
    for bad in ["7w", "0d", "d", "-3d", "3.5d", "99999999999999999999d"] {
        let err = stream_observations(&mut conn, Some(JESSICA), &format!("window={bad}"), NOW)
            .unwrap_err();
        assert_eq!(status_of(err), StatusCode::BAD_REQUEST, "window={bad}");
    }
}

#[test]
fn lens_long_dwell_filters_by_recipe_table() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    viewed(&mut conn, JESSICA, 0, NOW - 400, "bafy-59999", 59_999, 10);
    viewed(&mut conn, JESSICA, 1, NOW - 300, "bafy-60000", 60_000, 10);
    viewed(&mut conn, JESSICA, 2, NOW - 200, "bafy-120000", 120_000, 10);
    seed(
        &mut conn,
        JESSICA,
        3,
        NOW - 100,
        "lamad:path-followed",
        "bafy-other-kind",
        &payload(500_000, 10),
    );

    let view = stream(&mut conn, JESSICA, "lens=long-dwell");
    assert_eq!(view.lens, "long-dwell");
    assert_eq!(subjects(&view), vec!["bafy-120000", "bafy-60000"]);
    assert_eq!(view.total_count, 2);

    let view = stream(&mut conn, JESSICA, "lens=content");
    assert_eq!(
        view.total_count, 3,
        "content lens keeps only content-viewed"
    );
    assert!(!subjects(&view).contains(&"bafy-other-kind"));

    let view = stream(&mut conn, JESSICA, "");
    assert_eq!(view.lens, "all", "the default lens keeps every row");
    assert_eq!(view.total_count, 4);

    // The kind parameter narrows further.
    let view = stream(&mut conn, JESSICA, "kind=lamad:path-followed");
    assert_eq!(subjects(&view), vec!["bafy-other-kind"]);
}

#[test]
fn unknown_lens_is_400() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let err = stream_observations(&mut conn, Some(JESSICA), "lens=everyone-else", NOW).unwrap_err();
    let message = err.to_string();
    assert_eq!(status_of(err), StatusCode::BAD_REQUEST);
    assert!(message.contains("everyone-else"), "{message}");
}

#[test]
fn response_carries_recipe_cid_equal_to_recipe_cid() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    // Even an empty stream prints its provenance.
    let view = stream(&mut conn, JESSICA, "");
    assert_eq!(view.recipe.cid, recipe_cid());
    assert_eq!(view.recipe.name, "observation-lifestream");
    assert!(view.entries.is_empty());
    assert_eq!(view.total_count, 0);
}

#[test]
fn signature_absent_omission_present() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    viewed(&mut conn, JESSICA, 0, NOW - 60, "bafy-j1", 3000, 50);
    let view = stream(&mut conn, JESSICA, "");
    let line = omission_named(&view, "signature").expect("a signature omission line");
    assert!(line.starts_with("signature: absent"), "{line}");
    assert_eq!(
        view.omissions
            .iter()
            .filter(|l| l.starts_with("signature:"))
            .count(),
        1,
        "one line, not one per row"
    );
}

#[test]
fn unparseable_payload_renders_zeros_and_is_named() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    seed(
        &mut conn,
        JESSICA,
        0,
        NOW - 60,
        "lamad:content-viewed",
        "bafy-broken",
        "not json",
    );
    seed(
        &mut conn,
        JESSICA,
        1,
        NOW - 50,
        "lamad:content-viewed",
        "bafy-wrong-type",
        r#"{"ref_cid":"bafy","dwell_ms":"long","scroll_depth_pct":5}"#,
    );
    viewed(&mut conn, JESSICA, 2, NOW - 40, "bafy-fine", 7000, 150);

    let view = stream(&mut conn, JESSICA, "");
    assert_eq!(view.entries.len(), 3);
    for e in view
        .entries
        .iter()
        .filter(|e| e.subject_cid.as_deref() != Some("bafy-fine"))
    {
        assert_eq!((e.dwell_ms, e.scroll_depth_pct), (0, 0), "{e:?}");
    }
    let fine = &view.entries[0];
    assert_eq!(fine.subject_cid.as_deref(), Some("bafy-fine"));
    assert_eq!(fine.dwell_ms, 7000);
    assert_eq!(fine.scroll_depth_pct, 100, "depth is clamped to 100");
    let line = omission_named(&view, "payload").expect("a payload omission line");
    assert!(line.contains('2'), "{line}");
}

#[test]
fn response_validates_against_observation_stream_schema() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    diesel::sql_query("INSERT INTO content (id, title) VALUES (?, ?)")
        .bind::<diesel::sql_types::Text, _>("bafy-titled")
        .bind::<diesel::sql_types::Text, _>("The commons and its stewards")
        .execute(&mut conn)
        .expect("seed content");
    viewed(&mut conn, JESSICA, 0, NOW - 60, "bafy-titled", 64_000, 100);
    viewed(&mut conn, JESSICA, 1, NOW - 30, "bafy-untitled", 1_000, 20);
    viewed(&mut conn, JESSICA, 2, NOW - 9 * DAY, "bafy-old", 1_000, 20);
    seed(
        &mut conn,
        JESSICA,
        3,
        NOW - 10,
        "lamad:content-viewed",
        "bafy-broken",
        "{",
    );

    let view = stream(&mut conn, JESSICA, "");
    let titled = view
        .entries
        .iter()
        .find(|e| e.subject_cid.as_deref() == Some("bafy-titled"))
        .unwrap();
    assert_eq!(
        titled.title.as_deref(),
        Some("The commons and its stewards")
    );
    let json = serde_json::to_value(&view).unwrap();
    // An unknown title is omitted, never null.
    let untitled = json["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["subjectCid"] == "bafy-untitled")
        .unwrap();
    assert!(untitled.get("title").is_none(), "{untitled}");
    for name in ["signature", "window", "payload"] {
        assert!(
            omission_named(&view, name).is_some(),
            "{name}: {:?}",
            view.omissions
        );
    }
    validate_against_schema("views/observation-stream-view.schema.json", &json);

    let empty = serde_json::to_value(stream(&mut conn, JAMES, "")).unwrap();
    validate_against_schema("views/observation-stream-view.schema.json", &empty);
}
