//! `GET /db/content/search` (ruling R-S4, route half): the recipe-governed lexical search over
//! the content projection, reach-gated per candidate after ranking and before any snippet leaves.
//!
//! The route is driven through its production path (`HttpServer::test_content_search`, the same
//! handler body the dispatcher calls) and once through a bound server, to prove the dispatcher
//! reaches it before the `content/{id}` catch-all.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use elohim_storage::db::content_diesel::{create_content, CreateContentInput};
use elohim_storage::db::humans::{create_human, CreateHumanInput};
use elohim_storage::db::{init_pool_from_dir, AppContext, DbPool};
use elohim_storage::http::{reach_admits, HttpServer, PreparedReachGate};
use elohim_storage::search::query::{ContentSearchQuery, DEFAULT_LIMIT, MAX_LIMIT};
use elohim_storage::search::{Declared, SearchIndex};
use serde_json::Value;
use tempfile::TempDir;

/// The recipe CID every answer prints (`declaration.rs` pins the same value).
const RECIPE_CID: &str = "bafyreiedggzebsgekiw26kdkcdot6rxqpwsvbozpozmdwmig2mhhba4x3q";

struct Peer {
    _dir: TempDir,
    blobs: TempDir,
    pool: DbPool,
    index: Arc<SearchIndex>,
}

fn peer() -> Peer {
    let dir = tempfile::tempdir().unwrap();
    let pool = init_pool_from_dir(dir.path()).expect("migrated content db");
    let index = Arc::new(
        SearchIndex::open(dir.path(), Declared::load().unwrap(), "test-peer").expect("fold store"),
    );
    let blobs = tempfile::tempdir().unwrap();
    Peer {
        _dir: dir,
        blobs,
        pool,
        index,
    }
}

/// One content row above the serving floor (an ingest anchor makes it `notarized`).
fn seed(
    peer: &Peer,
    id: &str,
    title: &str,
    reach: &str,
    content_type: &str,
    tags: &[&str],
    body: &str,
) {
    let mut conn = peer.pool.get().unwrap();
    create_content(
        &mut conn,
        &AppContext::default_lamad(),
        CreateContentInput {
            id: id.into(),
            title: title.into(),
            description: None,
            content_type: content_type.into(),
            content_format: "markdown".into(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.into(),
            created_by: None,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            content_body: Some(body.into()),
            dht_anchor_hash: Some(format!("uhCkk-{id}-anchor")),
        },
    )
    .expect("seed a content row");
}

fn seed_human(peer: &Peer, id: &str) {
    let mut conn = peer.pool.get().unwrap();
    create_human(
        &mut conn,
        CreateHumanInput {
            id: id.into(),
            agent_pub_key: Some(format!("uhCAk-{id}")),
            display_name: id.into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: "lamad".into(),
            household_id: None,
        },
    )
    .expect("seed a human");
}

fn fold(peer: &Peer) {
    let mut conn = peer.pool.get().unwrap();
    // A backlog drains one declared batch per run.
    for _ in 0..4 {
        let report = peer.index.fold_once(&mut conn).expect("a fold run");
        if report.behind == 0 {
            break;
        }
    }
}

async fn server(peer: &Peer) -> HttpServer {
    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(peer.blobs.path().to_path_buf())
            .await
            .unwrap(),
    );
    HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
        .with_db_pool(peer.pool.clone())
        .with_search_index(peer.index.clone())
}

async fn search(server: &HttpServer, query: &str, agent_cid: Option<&str>) -> Value {
    let resp = server.test_content_search(query, agent_cid).await;
    let body: Value = serde_json::from_slice(&resp.body).unwrap_or_else(|e| {
        panic!(
            "the answer is JSON ({e}): {}",
            String::from_utf8_lossy(&resp.body)
        )
    });
    assert_eq!(resp.status, 200, "GET /db/content/search?{query} → {body}");
    body
}

fn ids(view: &Value) -> Vec<String> {
    view["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .map(|c| c["contentId"].as_str().unwrap().to_string())
        .collect()
}

fn facet_values(view: &Value, facet: &str) -> Vec<String> {
    view["facets"][facet]
        .as_array()
        .unwrap_or_else(|| panic!("facets.{facet}: {view}"))
        .iter()
        .map(|f| f["value"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn content_search_dispatches_before_content_by_id() {
    let peer = peer();
    seed(
        &peer,
        "watershed-basics",
        "Watershed basics",
        "commons",
        "concept",
        &[],
        "Rain.",
    );
    fold(&peer);
    let addr = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(peer.blobs.path().to_path_buf())
            .await
            .unwrap(),
    );
    let server = Arc::new(
        HttpServer::new(blob_store, addr)
            .with_db_pool(peer.pool.clone())
            .with_search_index(peer.index.clone()),
    );
    tokio::spawn(server.run());

    let url = format!("http://{addr}/db/content/search?q=watershed");
    let deadline = Instant::now() + Duration::from_secs(10);
    let resp = loop {
        match reqwest::get(&url).await {
            Ok(resp) => break resp,
            Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(50)).await
            }
            Err(e) => panic!("the bound server never answered: {e}"),
        }
    };
    let status = resp.status().as_u16();
    let body: Value = resp.json().await.expect("a JSON answer");
    assert_eq!(status, 200, "content/search is a route, not an id: {body}");
    assert_eq!(body["recipe"]["cid"], RECIPE_CID, "{body}");
    assert_eq!(ids(&body), ["watershed-basics"]);
}

#[tokio::test]
async fn seeded_title_ranks_first_with_recipe_cid() {
    let peer = peer();
    seed(
        &peer,
        "commons-stewards",
        "The commons and its stewards",
        "commons",
        "concept",
        &["stewardship"],
        "A watershed is one example among many.",
    );
    seed(
        &peer,
        "watershed-keeping",
        "Watershed keeping",
        "commons",
        "concept",
        &["water"],
        "# Watershed\nSwales slow the rain across the watershed.\n",
    );
    seed(
        &peer,
        "loam",
        "Loam and compost",
        "commons",
        "concept",
        &[],
        "Soil.",
    );
    fold(&peer);
    let server = server(&peer).await;
    let view = search(&server, "q=watershed", None).await;

    assert_eq!(view["query"], "watershed");
    assert_eq!(view["rankingKnown"], true, "{view}");
    assert_eq!(view["recipe"]["name"], "rrf-v2");
    assert_eq!(view["recipe"]["cid"], RECIPE_CID);
    let measure = peer.index.declared().measure_cid.clone();
    assert_eq!(view["recipe"]["producers"][0]["id"], "lexical");
    assert_eq!(view["recipe"]["producers"][0]["method"], measure.as_str());
    assert_eq!(view["fold"]["state"], "present", "{view}");
    assert_eq!(view["fold"]["value"]["measure"], measure.as_str());
    assert_eq!(view["foldLag"]["state"], "present", "{view}");
    assert_eq!(view["foldLag"]["value"]["unit"], "units");
    assert_eq!(view["foldLag"]["value"]["limit"], 200);
    assert_eq!(view["lens"]["level"], "standard");
    assert_eq!(view["lens"]["provenance"], "defaulted");

    let hits = ids(&view);
    assert_eq!(
        hits[0], "watershed-keeping",
        "the seeded title ranks first: {view}"
    );
    assert!(hits.contains(&"commons-stewards".to_string()), "{view}");
    assert!(!hits.contains(&"loam".to_string()), "{view}");
    assert_eq!(view["totalCount"], 2);
    let first = &view["candidates"][0];
    assert_eq!(first["producer"], "lexical");
    assert_eq!(first["method"], measure.as_str());
    assert_eq!(first["trust"], "notarized");
    assert_eq!(first["tags"], serde_json::json!(["water"]));
    let snippet = first["bestSection"]["snippet"].as_str().expect("a snippet");
    assert!(snippet.chars().count() <= 240, "{snippet}");
    assert!(snippet.to_lowercase().contains("watershed"), "{snippet}");
    // Candidates arrive in the recipe's order: scores never increase.
    let scores: Vec<f64> = view["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["score"].as_f64().unwrap())
        .collect();
    assert!(scores.windows(2).all(|w| w[0] >= w[1]), "{scores:?}");
}

#[tokio::test]
async fn reach_gated_row_absent_for_non_holder() {
    let peer = peer();
    seed(
        &peer,
        "watershed-open",
        "Watershed in the open",
        "commons",
        "concept",
        &["water"],
        "Open.",
    );
    seed(
        &peer,
        "watershed-kept",
        "Watershed kept close",
        "intimate",
        "private-note",
        &["secret"],
        "A private watershed note.",
    );
    seed_human(&peer, "susan");
    fold(&peer);
    let server = server(&peer).await;

    for (who, agent) in [("anonymous", None), ("susan", Some("susan"))] {
        let view = search(&server, "q=watershed", agent).await;
        assert_eq!(ids(&view), ["watershed-open"], "{who}: {view}");
        assert_eq!(view["totalCount"], 1, "{who}");
        assert!(
            !facet_values(&view, "tags").contains(&"secret".to_string()),
            "{who}: {view}"
        );
        assert!(
            !facet_values(&view, "contentType").contains(&"private-note".to_string()),
            "{who}: {view}"
        );
        assert!(
            !facet_values(&view, "reach").contains(&"intimate".to_string()),
            "{who}: {view}"
        );
        let body = view.to_string();
        assert!(
            !body.contains("watershed-kept"),
            "{who}: the withheld id never leaves: {body}"
        );
        assert!(
            !body.contains("private watershed note"),
            "{who}: no snippet leaves: {body}"
        );
        let refusals: Vec<&str> = view["omissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .filter(|line| line.contains("reach"))
            .collect();
        assert_eq!(refusals.len(), 1, "{who}: one refusal line: {view}");
        // The refusal names NO count (ruling R-S11, S3): a number is a cardinality channel into
        // a private ring — vary the question, read the count, learn what is kept there.
        assert_eq!(
            refusals[0], "some matches were withheld by reach",
            "{who}: the refusal is honest and count-free"
        );
        assert!(
            !refusals[0].chars().any(|c| c.is_ascii_digit()),
            "{who}: no digit leaves: {}",
            refusals[0]
        );
    }
}

#[tokio::test]
async fn limit_is_clamped_to_100() {
    let parse = |q: &str| ContentSearchQuery::parse(q).unwrap();
    assert_eq!(parse("q=a&limit=1000").limit(), MAX_LIMIT);
    assert_eq!(parse("q=a&limit=0").limit(), 1);
    assert_eq!(parse("q=a").limit(), DEFAULT_LIMIT);
    assert_eq!(MAX_LIMIT, 100);
    assert_eq!(DEFAULT_LIMIT, 20);

    let peer = peer();
    for n in 0..120 {
        seed(
            &peer,
            &format!("orchard-{n:03}"),
            &format!("Orchard note {n}"),
            "commons",
            "concept",
            &[],
            "Fruit trees.",
        );
    }
    fold(&peer);
    let server = server(&peer).await;
    let view = search(&server, "q=orchard&lens=whole&limit=1000", None).await;
    assert_eq!(
        view["candidates"].as_array().unwrap().len(),
        100,
        "{}",
        view["selection"]
    );
    let view = search(&server, "q=orchard&limit=1000", None).await;
    assert_eq!(
        view["candidates"].as_array().unwrap().len(),
        20,
        "the default lens cuts to its 20 choices before the page: {}",
        view["selection"]
    );
}

#[tokio::test]
async fn unknown_lens_is_never_refused() {
    let peer = peer();
    seed(
        &peer,
        "orchard",
        "Orchard",
        "commons",
        "concept",
        &[],
        "Fruit.",
    );
    fold(&peer);
    let server = server(&peer).await;
    let view = search(&server, "q=orchard&lens=galaxy", None).await;
    assert_eq!(view["lens"]["level"], "standard", "{view}");
    assert_eq!(view["lens"]["choiceCount"], 20);
    assert_eq!(view["lens"]["provenance"], "defaulted");
    assert_eq!(view["lens"]["cid"], peer.index.declared().lens_cid.as_str());
    assert_eq!(ids(&view), ["orchard"]);

    let view = search(&server, "q=orchard&lens=minimal", None).await;
    assert_eq!(view["lens"]["level"], "minimal");
    assert_eq!(view["lens"]["choiceCount"], 5);
    assert_eq!(view["lens"]["provenance"], "requested");
}

#[tokio::test]
async fn recipe_pin_mismatch_yields_unresolved_line() {
    let peer = peer();
    seed(
        &peer,
        "orchard",
        "Orchard",
        "commons",
        "concept",
        &[],
        "Fruit.",
    );
    fold(&peer);
    let server = server(&peer).await;

    let view = search(&server, "q=orchard&recipe=bafyreipinned", None).await;
    let expected = format!("recipe pinned bafyreipinned is not the served {RECIPE_CID}");
    assert!(
        view["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|line| line == expected.as_str()),
        "{view}"
    );
    assert_eq!(ids(&view), ["orchard"], "a mismatch still answers");

    let pinned = search(&server, &format!("q=orchard&recipe={RECIPE_CID}"), None).await;
    assert!(
        !pinned.to_string().contains("recipe pinned"),
        "the served pin is resolved: {pinned}"
    );
}

#[tokio::test]
async fn unfolded_store_answers_fold_absent() {
    let peer = peer();
    seed(
        &peer,
        "orchard",
        "Orchard",
        "commons",
        "concept",
        &[],
        "Fruit.",
    );
    // No fold run: the store exists but holds nothing attested.
    let server = server(&peer).await;
    let view = search(&server, "q=orchard", None).await;
    assert_eq!(
        view["fold"],
        serde_json::json!({"state": "absent", "reason": "observed_absent"}),
        "{view}"
    );
    assert_eq!(view["rankingKnown"], false);
    assert_eq!(view["candidates"], serde_json::json!([]));
    assert_eq!(view["totalCount"], 0);
    assert_eq!(
        view["recipe"]["cid"], RECIPE_CID,
        "the recipe is printed even so"
    );
}

// ── Schema conformance of served answers ──

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sdk/schemas/v1")
}

fn ref_map() -> HashMap<String, Value> {
    let mut refs = HashMap::new();
    for subdir in ["enums", "views", "objects"] {
        for entry in std::fs::read_dir(schema_dir().join(subdir))
            .unwrap()
            .flatten()
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let schema: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            refs.insert(name.clone(), schema.clone());
            refs.insert(format!("./{name}"), schema.clone());
            refs.insert(format!("../{subdir}/{name}"), schema.clone());
            if let Some(Value::String(id)) = schema.get("$id") {
                refs.insert(id.clone(), schema);
            }
        }
    }
    refs
}

/// Inline every cross-file `$ref` (internal `#/…` refs stay for the validator).
fn inline(schema: &Value, refs: &HashMap<String, Value>) -> Value {
    match schema {
        Value::Object(map) => {
            if let Some(Value::String(target)) = map.get("$ref") {
                if !target.starts_with('#') {
                    let (file, fragment) = target.split_once('#').unwrap_or((target, ""));
                    let mut found = refs
                        .get(file)
                        .unwrap_or_else(|| panic!("unresolved $ref {target}"))
                        .clone();
                    if let Value::Object(obj) = &mut found {
                        obj.remove("$id");
                        obj.remove("$schema");
                    }
                    if !fragment.is_empty() {
                        found = found.pointer(fragment).cloned().unwrap();
                    }
                    return inline(&found, refs);
                }
            }
            Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), inline(v, refs)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(|v| inline(v, refs)).collect()),
        other => other.clone(),
    }
}

fn assert_conforms(instance: &Value) {
    let raw: Value = serde_json::from_str(
        &std::fs::read_to_string(schema_dir().join("views/content-search-view.schema.json"))
            .unwrap(),
    )
    .unwrap();
    let schema = inline(&raw, &ref_map());
    let validator = jsonschema::validator_for(&schema).expect("the schema compiles");
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{e} (at {})", e.instance_path))
        .collect();
    assert!(errors.is_empty(), "{errors:#?}\n{instance}");
}

#[tokio::test]
async fn content_search_view_validates_against_schema() {
    let peer = peer();
    seed(
        &peer,
        "orchard",
        "Orchard",
        "commons",
        "concept",
        &["fruit"],
        "# Pruning\nOrchard work.",
    );
    seed(
        &peer,
        "orchard-kept",
        "Orchard kept",
        "intimate",
        "concept",
        &[],
        "Kept.",
    );
    let server = server(&peer).await;
    // Absent: before any fold.
    assert_conforms(&search(&server, "q=orchard", None).await);
    fold(&peer);
    // Present, with a reach refusal, an unknown lens and a pin mismatch.
    let view = search(
        &server,
        "q=orchard&lens=galaxy&recipe=bafyreipinned&tags=fruit",
        None,
    )
    .await;
    assert!(!view["candidates"].as_array().unwrap().is_empty(), "{view}");
    assert_conforms(&view);
    // A query with no searchable term.
    assert_conforms(&search(&server, "q=a", None).await);
}

#[tokio::test]
async fn list_route_still_gates_reach_through_reach_admits() {
    let peer = peer();
    seed(
        &peer,
        "open-row",
        "Open row",
        "commons",
        "concept",
        &[],
        "Open.",
    );
    seed(
        &peer,
        "kin-row",
        "Community row",
        "community",
        "concept",
        &[],
        "Kin.",
    );
    seed(
        &peer,
        "kept-row",
        "Kept row",
        "intimate",
        "concept",
        &[],
        "Kept.",
    );
    seed_human(&peer, "susan");
    let server = server(&peer).await;

    let listed = |body: bytes::Bytes| -> Vec<String> {
        let body: Value = serde_json::from_slice(&body).unwrap();
        let mut ids: Vec<String> = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_str().unwrap().to_string())
            .collect();
        ids.sort();
        ids
    };
    let anonymous = server.test_list_db_content("", None).await;
    assert_eq!(listed(anonymous.body), ["open-row"]);
    let susan = server.test_list_db_content("", Some("susan")).await;
    assert_eq!(listed(susan.body), ["kin-row", "open-row"]);

    // The one gate both routes use.
    let mut conn = peer.pool.get().unwrap();
    let ctx = AppContext::default_lamad();
    let human = elohim_storage::db::humans::get_human_by_id(&mut conn, "susan")
        .unwrap()
        .unwrap();
    // One gate, prepared once, consulted per row.
    let gate = PreparedReachGate::new(&None);
    assert!(reach_admits(
        &mut conn, &ctx, None, &gate, "commons", "open-row"
    ));
    assert!(!reach_admits(
        &mut conn,
        &ctx,
        None,
        &gate,
        "community",
        "kin-row"
    ));
    assert!(reach_admits(
        &mut conn,
        &ctx,
        Some(&human),
        &gate,
        "community",
        "kin-row"
    ));
    assert!(!reach_admits(
        &mut conn,
        &ctx,
        Some(&human),
        &gate,
        "intimate",
        "kept-row"
    ));
    assert!(!reach_admits(
        &mut conn,
        &ctx,
        None,
        &gate,
        "no-such-ring",
        "open-row"
    ));
}

#[tokio::test]
async fn candidate_set_is_capped_before_the_join() {
    // A public question must not buy a corpus-sized walk: the fused set is cut to
    // `(offset + limit) x candidate_headroom` (floored at the lens's choices) BEFORE the join and
    // the reach gate, and the cut is named in `omissions` (ruling R-S11, delta review W1).
    let peer = peer();
    seed(
        &peer,
        "orchard-keeping",
        "Orchard orchard orchard",
        "commons",
        "concept",
        &[],
        "Orchard orchard orchard orchard keeping.",
    );
    for n in 0..90 {
        seed(
            &peer,
            &format!("orchard-{n:03}"),
            &format!("Orchard note {n}"),
            "commons",
            "concept",
            &[],
            "Fruit trees.",
        );
    }
    fold(&peer);

    // The uncapped reading: the widest lens and the largest page cannot be cut.
    let server = server(&peer).await;
    let whole = search(&server, "q=orchard&lens=whole&limit=100", None).await;
    assert!(
        !whole.to_string().contains("past the cap were not examined"),
        "the widest lens at the largest page is never cut: {whole}"
    );
    let top = ids(&whole)[0].clone();

    // The default reading, driven through `answer` with a counting gate: 91 units rank, the cap
    // is 20 x 4 = 80, so the join and the gate run 80 times, not 91.
    let mut conn = peer.pool.get().unwrap();
    let reader = elohim_storage::search::reader::reader_for(None, None);
    let query = ContentSearchQuery::parse("q=orchard").unwrap();
    let mut gated = 0usize;
    let view = {
        let mut gate = |_: &mut diesel::SqliteConnection, _: &str, _: &str| {
            gated += 1;
            true
        };
        elohim_storage::search::query::answer(&peer.index, &mut conn, &reader, &query, &mut gate)
    };
    assert_eq!(
        gated,
        80,
        "the join and the gate run at most the cap: {}",
        serde_json::to_string(&view).unwrap()
    );
    let view = serde_json::to_value(&view).unwrap();
    let cut: Vec<&str> = view["omissions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .filter(|line| line.contains("past the cap were not examined"))
        .collect();
    assert_eq!(cut.len(), 1, "the cut is named once: {view}");
    assert!(
        cut[0].starts_with("11 "),
        "it names what it did not examine: {}",
        cut[0]
    );
    assert_eq!(
        ids(&view)[0],
        top,
        "the cap bounds the walk, never the ranking's head: {view}"
    );
}

#[tokio::test]
async fn reach_gate_is_prepared_once_per_request() {
    // The reach gate carries an `EprService` and its peer-trust cache. Built per row it was a
    // fresh authorizer for every candidate; it is prepared ONCE per request (ruling R-S11, S1).
    let peer = peer();
    seed(
        &peer,
        "orchard-open",
        "Orchard in the open",
        "commons",
        "concept",
        &[],
        "Open orchard.",
    );
    for n in 0..8 {
        seed(
            &peer,
            &format!("orchard-kin-{n}"),
            &format!("Orchard kin {n}"),
            "community",
            "concept",
            &[],
            "Orchard among kin.",
        );
    }
    seed_human(&peer, "susan");
    fold(&peer);
    let server = server(&peer).await;

    let before = server.reach_gates_prepared();
    let view = search(&server, "q=orchard", Some("susan")).await;
    assert_eq!(
        ids(&view).len(),
        9,
        "every row is admitted for susan: {view}"
    );
    assert_eq!(
        server.reach_gates_prepared() - before,
        1,
        "nine candidates, one prepared gate"
    );

    let before = server.reach_gates_prepared();
    let listed = server.test_list_db_content("", Some("susan")).await;
    assert_eq!(listed.status, 200);
    assert_eq!(
        server.reach_gates_prepared() - before,
        1,
        "the listing prepares one gate for the whole page"
    );
}
