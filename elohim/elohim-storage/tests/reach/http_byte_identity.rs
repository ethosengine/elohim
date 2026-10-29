//! HTTP status and refusal-byte golden across reach tiers, viewers, and route families.

use std::io::Write as _;
use std::sync::Arc;

use diesel::RunQueryDsl;
use elohim_storage::blob_store::BlobStore;
use elohim_storage::db::content_diesel::{create_content, CreateContentInput};
use elohim_storage::db::diesel_schema::humans;
use elohim_storage::db::models::NewHuman;
use elohim_storage::db::relationships_diesel::{create_relationship, CreateRelationshipInput};
use elohim_storage::db::{AppContext, HUMANS_HAPP_ID};
use elohim_storage::http::HttpServer;
use elohim_storage::services::Services;
use elohim_storage::test_util::test_pool;
use elohim_storage::views::{ContentView, RelationshipView};
use serde::Serialize;

const TIERS: &[&str] = &[
    "commons",
    "public",
    "community",
    "familiar",
    "trusted",
    "intimate",
    "self",
    "private",
    "unknown",
];

const STEWARD: &str = "human-steward";
const STEWARD_AGENT: &str = "uhCAk-steward";
const NON_MEMBER: &str = "human-non-member";
const NON_MEMBER_AGENT: &str = "uhCAk-non-member";
const UNRESOLVED: &str = "uhCAk-unresolved";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Viewer {
    None,
    Unresolved,
    NonMember,
    Steward,
}

impl Viewer {
    fn header(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Unresolved => Some(UNRESOLVED),
            Self::NonMember => Some(NON_MEMBER_AGENT),
            Self::Steward => Some(STEWARD_AGENT),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Answer {
    status: u16,
    body: Vec<u8>,
}

struct TierFixture {
    tier: &'static str,
    id: String,
    slug: String,
    sha: String,
    cid: String,
    blob_body: Vec<u8>,
    app_body: Vec<u8>,
    row_body: Vec<u8>,
    list_body: Vec<u8>,
    edge_body: Vec<u8>,
}

struct Fixture {
    base: String,
    tiers: Vec<TierFixture>,
    _blob_dir: tempfile::TempDir,
}

fn collection_bytes<T: Serialize>(items: Vec<T>) -> Vec<u8> {
    let count = items.len();
    serde_json::to_vec(&serde_json::json!({
        "items": items,
        "count": count,
        "limit": 100,
        "offset": 0,
    }))
    .unwrap()
}

fn bundle(marker: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("index.html", options).unwrap();
        write!(zip, "<!doctype html><title>{marker}</title>").unwrap();
        zip.finish().unwrap();
    }
    bytes
}

fn insert_human(conn: &mut diesel::SqliteConnection, id: &str, agent_pub_key: &str) {
    diesel::insert_into(humans::table)
        .values(&NewHuman {
            id: id.to_string(),
            agent_pub_key: Some(agent_pub_key.to_string()),
            display_name: id.to_string(),
            bio: None,
            affinities: "[]".to_string(),
            profile_reach: "commons".to_string(),
            location: None,
            profile_photo_url: None,
            h_app_id: HUMANS_HAPP_ID.to_string(),
            household_id: None,
        })
        .execute(conn)
        .unwrap();
}

async fn serve() -> Fixture {
    let blob_dir = tempfile::tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(blob_dir.path().to_path_buf()).await.unwrap());
    let pool = test_pool();
    let mut fixtures = Vec::new();
    for tier in TIERS {
        let marker = tier.replace('-', "_");
        let id = format!("reach-golden-{marker}");
        let slug = format!("reach-golden-app-{marker}");
        let blob_body = bundle(tier);
        let stored = blob_store.store(&blob_body).await.unwrap();
        let digest = BlobStore::parse_content_address(&stored.hash).unwrap();
        let cid = BlobStore::hash_to_cid(&digest).unwrap().to_string();
        fixtures.push(TierFixture {
            tier,
            id,
            slug,
            sha: stored.hash,
            cid,
            blob_body,
            app_body: format!("<!doctype html><title>{tier}</title>").into_bytes(),
            row_body: Vec::new(),
            list_body: Vec::new(),
            edge_body: Vec::new(),
        });
    }
    {
        let mut conn = pool.get().unwrap();
        insert_human(&mut conn, STEWARD, STEWARD_AGENT);
        insert_human(&mut conn, NON_MEMBER, NON_MEMBER_AGENT);
        for fixture in &mut fixtures {
            let created = create_content(
                &mut conn,
                &AppContext::default_lamad(),
                CreateContentInput {
                    id: fixture.id.clone(),
                    title: fixture.id.clone(),
                    description: None,
                    content_type: "app".to_string(),
                    content_format: "html5-app".to_string(),
                    blob_hash: Some(fixture.sha.clone()),
                    blob_cid: Some(fixture.cid.clone()),
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: fixture.tier.to_string(),
                    created_by: Some(STEWARD.to_string()),
                    tags: Vec::new(),
                    content_body: Some(
                        serde_json::json!({"slug": fixture.slug.clone()}).to_string(),
                    ),
                    dht_anchor_hash: Some(format!("uhCkk-{}", fixture.tier)),
                },
            )
            .unwrap();
            let view = ContentView::from(created);
            fixture.row_body = serde_json::to_vec(&view).unwrap();
            fixture.list_body = collection_bytes(vec![view]);

            let edge = create_relationship(
                &mut conn,
                &AppContext::default_lamad(),
                CreateRelationshipInput {
                    id: Some(format!("reach-golden-edge-{}", fixture.tier)),
                    source_id: fixture.id.clone(),
                    target_id: fixture.id.clone(),
                    relationship_type: "RELATES_TO".to_string(),
                    confidence: 1.0,
                    inference_source: "explicit".to_string(),
                    is_bidirectional: false,
                    provenance_chain_json: None,
                    governance_layer: None,
                    reach: Some(fixture.tier.to_string()),
                    metadata_json: None,
                },
            )
            .unwrap();
            fixture.edge_body = collection_bytes(vec![RelationshipView::from(edge)]);
        }
    }
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
        .with_db_pool(pool.clone())
        .with_services(Arc::new(Services::new(pool.clone())));
    server.load_slug_index().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(Arc::new(server).serve(listener));
    Fixture {
        base,
        tiers: fixtures,
        _blob_dir: blob_dir,
    }
}

async fn get(base: &str, path: &str, viewer: Viewer) -> Answer {
    let mut request = reqwest::Client::new().get(format!("{base}{path}"));
    if let Some(agent) = viewer.header() {
        request = request.header("X-Agent-Cid", agent);
    }
    let response = request.send().await.unwrap();
    Answer {
        status: response.status().as_u16(),
        body: response.bytes().await.unwrap().to_vec(),
    }
}

fn json(error: &str, reach: &str, reason: Option<&str>) -> Vec<u8> {
    match reason {
        Some(reason) => {
            format!(r#"{{"error":"{error}","requiredReach":"{reach}","reason":"{reason}"}}"#)
                .into_bytes()
        }
        None => format!(r#"{{"error":"{error}","requiredReach":"{reach}"}}"#).into_bytes(),
    }
}

fn row_refusal(tier: &str, viewer: Viewer) -> Option<Vec<u8>> {
    if matches!(tier, "commons" | "public") {
        return None;
    }
    if viewer == Viewer::None {
        return Some(json("Authentication required", tier, None));
    }
    if tier == "community" {
        return None;
    }
    if viewer == Viewer::Unresolved {
        return Some(json("Reach authorization required", tier, None));
    }
    let reason = match tier {
        "familiar" => "No shared collective with content steward",
        "trusted" => "No trusted relationship with content steward",
        "intimate" => "No mutual intimate relationship with content steward",
        "self" | "private" if viewer == Viewer::Steward => return None,
        "self" | "private" => "Content is private — only the creator can access it",
        "unknown" => "Unknown reach level: unknown",
        _ => return None,
    };
    Some(json("Reach denied", tier, Some(reason)))
}

fn blob_refusal(tier: &str, viewer: Viewer) -> Option<Vec<u8>> {
    if matches!(tier, "commons" | "public") {
        return None;
    }
    if viewer == Viewer::None {
        return Some(json("Authentication required", tier, None));
    }
    if tier == "community" {
        return None;
    }
    Some(json("Reach authorization required", tier, None))
}

fn assert_answer(answer: &Answer, refusal: Option<Vec<u8>>, success: &[u8], label: &str) {
    match refusal {
        Some(body) => {
            assert_eq!(answer.status, 403, "{label}: {:?}", answer.body);
            assert_eq!(answer.body, body, "{label}");
        }
        None => {
            assert_eq!(answer.status, 200, "{label}: {:?}", answer.body);
            assert_eq!(answer.body, success, "{label}: successful response bytes");
        }
    }
}

#[tokio::test]
async fn http_status_and_refusal_bytes_match_the_pre_extraction_golden() {
    let fixture = serve().await;
    let empty_collection = collection_bytes(Vec::<serde_json::Value>::new());
    for tier in &fixture.tiers {
        for viewer in [
            Viewer::None,
            Viewer::Unresolved,
            Viewer::NonMember,
            Viewer::Steward,
        ] {
            let label = format!("{} as {viewer:?}", tier.tier);
            let row = get(&fixture.base, &format!("/db/content/{}", tier.id), viewer).await;
            assert_answer(
                &row,
                row_refusal(tier.tier, viewer),
                &tier.row_body,
                &format!("row {label}"),
            );

            let edges = get(
                &fixture.base,
                &format!("/db/relationships?contentId={}", tier.id),
                viewer,
            )
            .await;
            assert_answer(
                &edges,
                row_refusal(tier.tier, viewer),
                &tier.edge_body,
                &format!("edges {label}"),
            );

            let blob_sha = get(&fixture.base, &format!("/blob/{}", tier.sha), viewer).await;
            let blob_cid = get(&fixture.base, &format!("/blob/{}", tier.cid), viewer).await;
            assert_answer(
                &blob_sha,
                blob_refusal(tier.tier, viewer),
                &tier.blob_body,
                &format!("blob sha {label}"),
            );
            assert_eq!(blob_cid, blob_sha, "blob address rendering {label}");

            let app_slug = get(
                &fixture.base,
                &format!("/apps/{}/index.html", tier.slug),
                viewer,
            )
            .await;
            assert_answer(
                &app_slug,
                row_refusal(tier.tier, viewer),
                &tier.app_body,
                &format!("app slug {label}"),
            );
            let app_cid = get(
                &fixture.base,
                &format!("/apps/{}/index.html", tier.cid),
                viewer,
            )
            .await;
            assert_answer(
                &app_cid,
                blob_refusal(tier.tier, viewer),
                &tier.app_body,
                &format!("app cid {label}"),
            );

            let list = get(
                &fixture.base,
                &format!("/db/content?reach={}", tier.tier),
                viewer,
            )
            .await;
            let visible = row_refusal(tier.tier, viewer).is_none()
                && !(tier.tier == "community" && viewer == Viewer::Unresolved);
            assert_eq!(list.status, 200, "list {label}");
            let expected = if visible {
                tier.list_body.as_slice()
            } else {
                empty_collection.as_slice()
            };
            assert_eq!(list.body, expected, "list bytes {label}");
        }
    }
}
