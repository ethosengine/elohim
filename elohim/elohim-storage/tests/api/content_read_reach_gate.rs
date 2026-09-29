//! Every GET that reveals a content atom — its body, its head, its head record,
//! its schedule, its edges — answers a caller who may not read the atom the
//! way the body route does for an intimate row, and no edge route names an
//! atom its caller may not read. Backlog
//! `security-content-head-route-bypasses-reach-gate` (observed on the household
//! mesh 2026-09-27: `/db/content/love-map-matthew-jessica` 403, `/head` 200).
//!
//! These tests drive the REAL server over HTTP (`HttpServer::serve` on a bound
//! loopback port) and assert the SERVED bodies, not a gate verdict:
//!
//! - the atom sweep is DERIVED from the route manifest (`http::build_manifest`),
//!   so a new `GET /db/content/{id}/…` or `/db/relationships…` read registered
//!   there is swept automatically;
//! - the edge tests walk a fixture whose commons atoms point AT restricted ones
//!   (and past them), and check the list (with and without `contentId`), a
//!   single edge by id and graph walks of depth 2-3 for an anonymous caller, an
//!   identified stranger, and the steward who may read the restricted atom;
//! - the edges of the commons atom `COMMONS` are the VERIFIED projection of its
//!   signed head (`authored_edges::replace_authored_edges`), so its edge to a
//!   commons atom held elsewhere still serves; edges POSTed through the real
//!   write route (`/db/relationships/bulk`) — to an unknown id, from an unknown
//!   source, or stating a reach wider than their source — never serve openly.

use std::sync::Arc;

use diesel::RunQueryDsl;
use doorway_client::HttpMethod;
use elohim_storage::db::authored_edges::{replace_authored_edges, AuthoredEdge};
use elohim_storage::db::content_diesel::{create_content, CreateContentInput};
use elohim_storage::db::diesel_schema::humans;
use elohim_storage::db::models::NewHuman;
use elohim_storage::db::relationships_diesel::{create_relationship, CreateRelationshipInput};
use elohim_storage::db::{AppContext, HUMANS_HAPP_ID};
use elohim_storage::http::{build_manifest, HttpServer};
use elohim_storage::services::Services;
use elohim_storage::test_util::test_pool;
use serde_json::Value;

const INTIMATE: &str = "love-map-matthew-jessica";
const COMMONS: &str = "fct-bible-psalm-13";
const COMMONS_B: &str = "fct-bible-psalm-23";
/// Commons, but reachable in the graph ONLY through the intimate atom.
const BEYOND: &str = "fct-bible-psalm-139";
/// Reach `self`, created by the steward below.
const JOURNAL: &str = "journal-matthew";
/// No local row: an intimate atom held elsewhere, named only by an edge.
const REMOTE_INTIMATE: &str = "love-letter-held-elsewhere";
/// No local row: a commons atom held elsewhere, named by COMMONS' VERIFIED
/// authored edge.
const REMOTE_COMMONS: &str = "fct-bible-psalm-1";
/// No local row: an id a caller named in a POSTed edge from a commons atom —
/// it could be anything, an intimate atom held elsewhere included.
const FABRICATED: &str = "love-note-named-by-a-post";
/// No local row: the SOURCE of a POSTed edge.
const GHOST_SOURCE: &str = "ghost-source-named-by-a-post";
const STEWARD: &str = "human-matthew";
const STRANGER: &str = "uhCAk-a-stranger";

/// Ids no anonymous or stranger response may ever contain.
const HIDDEN: &[&str] = &[
    INTIMATE,
    JOURNAL,
    REMOTE_INTIMATE,
    BEYOND,
    FABRICATED,
    GHOST_SOURCE,
];

/// Edges POSTed through `/db/relationships/bulk` by `serve`: (id, source,
/// target, stated reach).
const POSTED: &[(&str, &str, &str, &str)] = &[
    ("rel-posted-fabricated", COMMONS, FABRICATED, "commons"),
    ("rel-posted-ghost", GHOST_SOURCE, COMMONS, "commons"),
    ("rel-posted-widen", INTIMATE, COMMONS_B, "commons"),
];

struct Fixture {
    base: String,
    pool: elohim_storage::db::DbPool,
    /// Edge ids by (source, target).
    edges: Vec<(String, String, String)>,
}

impl Fixture {
    fn edge(&self, source: &str, target: &str) -> &str {
        &self
            .edges
            .iter()
            .find(|(s, t, _)| s == source && t == target)
            .expect("fixture edge")
            .2
    }
}

fn seed(pool: &elohim_storage::db::DbPool) -> Vec<(String, String, String)> {
    let mut conn = pool.get().unwrap();
    let ctx = AppContext::default_lamad();
    diesel::insert_into(humans::table)
        .values(&NewHuman {
            id: STEWARD.into(),
            agent_pub_key: Some("uhCAk-matthew".into()),
            display_name: "Matthew".into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: HUMANS_HAPP_ID.into(),
            household_id: None,
        })
        .execute(&mut conn)
        .unwrap();
    for (id, reach, created_by) in [
        (INTIMATE, "intimate", None),
        (COMMONS, "commons", None),
        (COMMONS_B, "commons", None),
        (BEYOND, "commons", None),
        (JOURNAL, "self", Some(STEWARD)),
    ] {
        create_content(
            &mut conn,
            &ctx,
            CreateContentInput {
                id: id.into(),
                title: id.into(),
                description: None,
                content_type: "path".into(),
                content_format: "markdown".into(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: reach.into(),
                created_by: created_by.map(str::to_owned),
                tags: Vec::new(),
                content_body: Some("body".into()),
                dht_anchor_hash: Some(format!("uhCkk-{id}")),
            },
        )
        .unwrap();
    }
    // COMMONS' edges are what its verified signed head states, projected at
    // its reach (commons) and marked as the verified projection. The one to
    // INTIMATE carries `commons` too — the filter must not trust the edge's own
    // tier when both endpoints are held locally.
    let authored: Vec<AuthoredEdge> = [INTIMATE, COMMONS_B, REMOTE_COMMONS]
        .iter()
        .map(|target| AuthoredEdge {
            relationship_type: "RELATES_TO".into(),
            target_id: (*target).into(),
            role: None,
        })
        .collect();
    replace_authored_edges(
        &mut conn,
        &ctx,
        COMMONS,
        &authored,
        "commons",
        Some(&format!("uhCkk-{COMMONS}")),
    )
    .unwrap();
    let mut out: Vec<(String, String, String)> = [INTIMATE, COMMONS_B, REMOTE_COMMONS]
        .iter()
        .map(|target| {
            let id = elohim_storage::db::relationships_diesel::list_relationships(
                &mut conn,
                &ctx,
                &elohim_storage::db::relationships_diesel::RelationshipQuery {
                    content_id: Some(COMMONS.into()),
                    direction: Some("outgoing".into()),
                    limit: 100,
                    ..Default::default()
                },
            )
            .unwrap()
            .into_iter()
            .find(|r| r.target_id == *target)
            .expect("authored edge")
            .id;
            (COMMONS.to_string(), (*target).to_string(), id)
        })
        .collect();
    // (source, target, edge reach), written directly (not the verified
    // projection; not a route either — fixture rows).
    let edges = [
        (COMMONS_B, JOURNAL, "commons"),
        (COMMONS_B, REMOTE_INTIMATE, "intimate"),
        (INTIMATE, COMMONS, "intimate"),
        (INTIMATE, BEYOND, "intimate"),
        (JOURNAL, COMMONS, "self"),
    ];
    edges
        .iter()
        .enumerate()
        .map(|(n, (source, target, reach))| {
            let id = create_relationship(
                &mut conn,
                &ctx,
                CreateRelationshipInput {
                    // Opaque ids: a 404 echoes the requested id, and that echo
                    // must not be what names a hidden atom.
                    id: Some(format!("rel-fixture-{n}")),
                    source_id: (*source).into(),
                    target_id: (*target).into(),
                    relationship_type: "RELATES_TO".into(),
                    confidence: 1.0,
                    inference_source: "explicit".into(),
                    is_bidirectional: false,
                    provenance_chain_json: None,
                    governance_layer: None,
                    reach: Some((*reach).into()),
                    metadata_json: None,
                },
            )
            .unwrap()
            .id;
            ((*source).to_string(), (*target).to_string(), id)
        })
        .for_each(|e| out.push(e));
    out
}

async fn serve() -> Fixture {
    let pool = test_pool();
    let edges = seed(&pool);
    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(
            tempfile::tempdir().unwrap().path().to_path_buf(),
        )
        .await
        .unwrap(),
    );
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
        .with_db_pool(pool.clone())
        .with_services(Arc::new(Services::new(pool.clone())));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(Arc::new(server).serve(listener));

    // Edges any caller could POST: through the real write route.
    let body: Vec<Value> = POSTED
        .iter()
        .map(|(id, source, target, reach)| {
            serde_json::json!({
                "id": id,
                "sourceId": source,
                "targetId": target,
                "relationshipType": "RELATES_TO",
                "reach": reach,
            })
        })
        .collect();
    let resp = reqwest::Client::new()
        .post(format!("{base}/db/relationships/bulk"))
        .header("X-Schema-Version", "1")
        .json(&body)
        .send()
        .await
        .expect("bulk POST");
    assert_eq!(resp.status().as_u16(), 200, "{:?}", resp.text().await);
    let mut edges = edges;
    for (id, source, target, _) in POSTED {
        edges.push((source.to_string(), target.to_string(), id.to_string()));
    }
    Fixture { base, pool, edges }
}

/// GET `/db/{path}` as `caller` (the explicit `X-Agent-Cid`), returning the
/// status and the body text.
async fn get(fx: &Fixture, path: &str, caller: Option<&str>) -> (u16, String) {
    let mut req = reqwest::Client::new().get(format!("{}/db/{path}", fx.base));
    if let Some(cid) = caller {
        req = req.header("X-Agent-Cid", cid);
    }
    let resp = req.send().await.expect("request");
    let status = resp.status().as_u16();
    (status, resp.text().await.unwrap_or_default())
}

fn assert_hides(body: &str, what: &str) {
    for id in HIDDEN {
        assert!(
            !body.contains(id),
            "{what} must not name {id}; served: {body}"
        );
    }
}

/// The (source, target) pairs of a list response.
fn pairs(body: &str) -> Vec<(String, String)> {
    let v: Value = serde_json::from_str(body).expect("json list");
    let items = v["items"].as_array().expect("items");
    assert_eq!(v["count"].as_u64(), Some(items.len() as u64));
    items
        .iter()
        .map(|e| {
            (
                e["sourceId"].as_str().unwrap().to_string(),
                e["targetId"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn graph_ids(body: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(body).expect("json graph");
    v["related"]
        .as_array()
        .expect("related")
        .iter()
        .map(|n| n["contentId"].as_str().unwrap().to_string())
        .collect()
}

/// Every content-scoped GET in the route manifest, as a `/db/…` path for the
/// atom `id` whose edge is `edge_id`.
fn content_reads(id: &str, edge_id: &str) -> Vec<String> {
    let reads: Vec<String> = build_manifest()
        .routes
        .into_iter()
        .filter(|r| r.method == HttpMethod::Get)
        .filter_map(|r| {
            let path = r.path.strip_prefix("/db/")?.to_string();
            if path == "relationships" {
                return Some(format!("{path}?contentId={id}"));
            }
            if path == "relationships/bulk" {
                return None;
            }
            let atom = path.starts_with("content/{") || path.starts_with("relationships/");
            atom.then(|| {
                if path == "relationships/{id}" {
                    path.replace("{id}", edge_id)
                } else {
                    path.replace("{content_id}", id).replace("{id}", id)
                }
            })
        })
        .collect();
    assert!(
        reads.len() >= 7,
        "the manifest should name body, schedule, head, head-record, relationships of, \
         graph and a single edge — got {reads:?}"
    );
    reads
}

#[tokio::test]
async fn every_read_of_an_intimate_atom_refuses_a_caller_who_cannot_read_it() {
    let fx = serve().await;
    let edge = fx.edge(INTIMATE, COMMONS).to_string();
    for path in content_reads(INTIMATE, &edge) {
        for caller in [None, Some(STRANGER)] {
            let (status, body) = get(&fx, &path, caller).await;
            assert_eq!(
                status, 403,
                "GET /db/{path} as {caller:?} must be refused for an intimate atom; got {body}"
            );
            assert!(
                body.contains("requiredReach"),
                "refusal keeps requiredReach: {body}"
            );
        }
    }
}

#[tokio::test]
async fn anonymous_edge_list_never_names_a_restricted_atom() {
    let fx = serve().await;
    for caller in [None, Some(STRANGER)] {
        // No contentId: the corpus-wide list.
        let (status, body) = get(&fx, "relationships", caller).await;
        assert_eq!(status, 200, "{body}");
        assert_hides(&body, "the corpus-wide edge list");
        let got = pairs(&body);
        assert!(got.contains(&(COMMONS.into(), COMMONS_B.into())), "{body}");
        assert!(
            got.contains(&(COMMONS.into(), REMOTE_COMMONS.into())),
            "an edge to a commons atom held elsewhere still serves: {body}"
        );
        assert_eq!(got.len(), 2, "exactly the two open edges: {body}");

        // contentId = a commons atom whose edge points at the love map.
        let (status, body) = get(&fx, &format!("relationships?contentId={COMMONS}"), caller).await;
        assert_eq!(status, 200, "{body}");
        assert_hides(&body, "a commons atom's edge list");
        assert_eq!(pairs(&body).len(), 2, "{body}");
    }
}

#[tokio::test]
async fn anonymous_single_edge_to_a_restricted_atom_is_absent() {
    let fx = serve().await;
    for caller in [None, Some(STRANGER)] {
        for (source, target) in [
            (COMMONS, INTIMATE),
            (COMMONS_B, JOURNAL),
            (COMMONS_B, REMOTE_INTIMATE),
            (COMMONS, FABRICATED),
            (GHOST_SOURCE, COMMONS),
        ] {
            let (status, body) = get(
                &fx,
                &format!("relationships/{}", fx.edge(source, target)),
                caller,
            )
            .await;
            assert_eq!(status, 404, "{source}->{target} as {caller:?}: {body}");
            assert_hides(&body, "a single hidden edge");
        }
        let (status, _) = get(
            &fx,
            &format!("relationships/{}", fx.edge(COMMONS, COMMONS_B)),
            caller,
        )
        .await;
        assert_eq!(status, 200, "an open edge still serves");
    }
}

#[tokio::test]
async fn anonymous_graph_walk_prunes_and_never_walks_through_a_restricted_atom() {
    let fx = serve().await;
    for caller in [None, Some(STRANGER)] {
        for depth in [2, 3] {
            let (status, body) = get(
                &fx,
                &format!("relationships/graph/{COMMONS}?depth={depth}"),
                caller,
            )
            .await;
            assert_eq!(status, 200, "{body}");
            assert_hides(&body, "a graph walk");
            let ids = graph_ids(&body);
            assert!(ids.contains(&COMMONS_B.to_string()), "{body}");
            assert!(ids.contains(&REMOTE_COMMONS.to_string()), "{body}");
        }
    }
}

#[tokio::test]
async fn a_steward_with_rights_still_sees_the_edges() {
    let fx = serve().await;
    let caller = Some(STEWARD);

    let (_, body) = get(&fx, "relationships", caller).await;
    let got = pairs(&body);
    assert!(got.contains(&(COMMONS_B.into(), JOURNAL.into())), "{body}");
    assert!(got.contains(&(JOURNAL.into(), COMMONS.into())), "{body}");
    // Rights to the journal are not rights to the love map.
    assert!(!body.contains(INTIMATE), "{body}");

    let (_, body) = get(&fx, &format!("relationships?contentId={JOURNAL}"), caller).await;
    assert_eq!(pairs(&body).len(), 2, "{body}");

    let (status, body) = get(
        &fx,
        &format!("relationships/{}", fx.edge(COMMONS_B, JOURNAL)),
        caller,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains(JOURNAL));

    let (_, body) = get(
        &fx,
        &format!("relationships/graph/{COMMONS}?depth=2"),
        caller,
    )
    .await;
    assert!(graph_ids(&body).contains(&JOURNAL.to_string()), "{body}");
    assert!(!body.contains(INTIMATE), "{body}");
}

#[tokio::test]
async fn an_unknown_atom_is_the_route_s_to_answer() {
    let fx = serve().await;
    let (status, body) = get(&fx, "relationships/graph/no-such-atom?depth=2", None).await;
    assert_eq!(status, 200, "{body}");
    assert!(graph_ids(&body).is_empty());
    let (status, _) = get(&fx, "relationships/no-such-edge", None).await;
    assert_eq!(status, 404);
    let (status, _) = get(&fx, "content/no-such-atom", None).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn the_corpus_wide_list_clamps_its_page() {
    let fx = serve().await;
    let (status, body) = get(&fx, "relationships?limit=100000&offset=-5", None).await;
    assert_eq!(status, 200, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["limit"].as_i64(), Some(500));
    assert_eq!(v["offset"].as_i64(), Some(0));
}

/// A POSTed edge is stored no more open than its SOURCE row allows, and an id
/// only a POST names is served to no one — while COMMONS' verified edge to a
/// commons atom held elsewhere still serves anonymously.
#[tokio::test]
async fn a_posted_edge_is_never_more_open_than_its_source_nor_vouches_for_an_unknown_id() {
    let fx = serve().await;
    let stored_reach = |id: &str| {
        let mut conn = fx.pool.get().unwrap();
        elohim_storage::db::relationships_diesel::get_relationship(
            &mut conn,
            &AppContext::default_lamad(),
            id,
        )
        .unwrap()
        .expect("posted edge stored")
        .reach
    };
    assert_eq!(
        stored_reach("rel-posted-widen"),
        "intimate",
        "a stated `commons` cannot widen an intimate source's edge"
    );
    assert_eq!(
        stored_reach("rel-posted-ghost"),
        "self",
        "an edge from a source this peer does not hold lands at the most restrictive tier"
    );
    assert_eq!(stored_reach("rel-posted-fabricated"), "commons");

    for caller in [None, Some(STRANGER), Some(STEWARD)] {
        let (status, body) = get(&fx, "relationships/rel-posted-fabricated", caller).await;
        assert_eq!(
            status, 404,
            "a POSTed commons edge to an unknown id never serves ({caller:?}): {body}"
        );
        let (status, body) = get(&fx, "relationships", caller).await;
        assert_eq!(status, 200);
        assert!(!body.contains(FABRICATED), "{caller:?}: {body}");
        assert!(!body.contains(GHOST_SOURCE), "{caller:?}: {body}");
        let (_, body) = get(
            &fx,
            &format!("relationships/graph/{COMMONS}?depth=2"),
            caller,
        )
        .await;
        assert!(
            !graph_ids(&body).contains(&FABRICATED.to_string()),
            "{body}"
        );
    }
    let (status, body) = get(
        &fx,
        &format!("relationships/{}", fx.edge(COMMONS, REMOTE_COMMONS)),
        None,
    )
    .await;
    assert_eq!(
        status, 200,
        "the verified edge to a commons atom held elsewhere still serves: {body}"
    );

    // An unrecognized tier is refused, not stored.
    let resp = reqwest::Client::new()
        .post(format!("{}/db/relationships/bulk", fx.base))
        .header("X-Schema-Version", "1")
        .json(&serde_json::json!([{
            "sourceId": COMMONS,
            "targetId": COMMONS_B,
            "relationshipType": "RELATES_TO",
            "reach": "everyone",
        }]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 400);
}

/// F19 — the peer adopts the steward's signed grade. A peer holds an atom at
/// `private` (an old seeder default) with an authored edge; its own conductor
/// hands it the steward's EARNED head, whose verified entry says `commons`.
/// After the adoption stamp an anonymous caller is SERVED the body and the
/// edge — the row and the edge its verified source head states opened together.
#[tokio::test]
async fn adopting_an_earned_commons_head_opens_a_private_row_to_anonymous_readers() {
    use elohim_storage::db::content_diesel::{
        stamp_declared_head_mode, CanonicalOrdering, ContentProjectionPatch, StampMode,
        StampOutcome,
    };
    const ADOPTED: &str = "fct-teacher-workbook-atom";
    let fx = serve().await;
    {
        let mut conn = fx.pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        create_content(
            &mut conn,
            &ctx,
            CreateContentInput {
                id: ADOPTED.into(),
                title: ADOPTED.into(),
                description: None,
                content_type: "concept".into(),
                content_format: "markdown".into(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "private".into(),
                created_by: None,
                tags: Vec::new(),
                content_body: Some("the steward's words".into()),
                dht_anchor_hash: Some(format!("uhCkk-{ADOPTED}")),
            },
        )
        .unwrap();
        replace_authored_edges(
            &mut conn,
            &ctx,
            ADOPTED,
            &[AuthoredEdge {
                relationship_type: "RELATES_TO".into(),
                target_id: COMMONS_B.into(),
                role: None,
            }],
            "private",
            Some(&format!("uhCkk-{ADOPTED}")),
        )
        .unwrap();
    }
    let (status, body) = get(&fx, &format!("content/{ADOPTED}"), None).await;
    assert_ne!(
        status, 200,
        "a private atom is not served anonymously: {body}"
    );
    let (_, edges) = get(&fx, &format!("relationships?contentId={ADOPTED}"), None).await;
    assert!(!edges.contains(COMMONS_B), "nor is its edge: {edges}");

    {
        let mut conn = fx.pool.get().unwrap();
        let outcome = stamp_declared_head_mode(
            &mut conn,
            &AppContext::default_lamad(),
            ADOPTED,
            "uhCkk-steward-earned-head",
            Some(10),
            Some(ContentProjectionPatch {
                reach: Some("commons".into()),
                title: Some(ADOPTED.into()),
                description: Some(String::new()),
                content_type: Some("concept".into()),
                content_format: Some("markdown".into()),
                metadata_json: Some("{}".into()),
                content_body: Some(Some("the steward's words".into())),
                tags: Some(Vec::new()),
                relationships: Some(vec![AuthoredEdge {
                    relationship_type: "RELATES_TO".into(),
                    target_id: COMMONS_B.into(),
                    role: None,
                }]),
                ..Default::default()
            }),
            StampMode::HealCanonical,
            Some(CanonicalOrdering::new(10, true)),
        )
        .unwrap();
        assert_eq!(outcome, StampOutcome::Stamped);
    }

    let (status, body) = get(&fx, &format!("content/{ADOPTED}"), None).await;
    assert_eq!(
        status, 200,
        "the adopted commons atom serves anonymously: {body}"
    );
    assert!(body.contains("the steward's words"), "served body: {body}");
    let (status, edges) = get(&fx, &format!("relationships?contentId={ADOPTED}"), None).await;
    assert_eq!(status, 200, "{edges}");
    assert!(
        pairs(&edges).contains(&(ADOPTED.to_string(), COMMONS_B.to_string())),
        "the authored edge opened with its source: {edges}"
    );
}
