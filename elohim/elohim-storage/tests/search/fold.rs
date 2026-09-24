//! The content lexical fold (ruling R-S3): where it lives, what a unit holds, that it is
//! incremental by fingerprint, that it attests itself, that a notify wakes it, and that a corrupt
//! store is replaced rather than repaired.

use std::sync::Arc;
use std::time::{Duration, Instant};

use diesel::connection::SimpleConnection;
use elohim_epr_index::attest::{LATEST_FILE, LOG_FILE};
use elohim_epr_index::terms::match_expression;
use elohim_epr_rea::{FoldAttestation, FoldState};
use elohim_storage::db::content_diesel::{self, CreateContentInput};
use elohim_storage::db::{init_pool_from_dir, AppContext, DbPool};
use elohim_storage::search::fold::{store_path, HEAD_SECTION, INDEX_DIR, STORE_FILE, TAGS_SECTION};
use elohim_storage::search::{Declared, SearchIndex};
use elohim_storage::services::events::EventBus;
use elohim_storage::services::ContentService;
use tempfile::TempDir;

struct Peer {
    dir: TempDir,
    pool: DbPool,
}

fn peer() -> Peer {
    let dir = tempfile::tempdir().unwrap();
    let pool = init_pool_from_dir(dir.path()).expect("migrated content db");
    Peer { dir, pool }
}

fn open(peer: &Peer) -> SearchIndex {
    SearchIndex::open(peer.dir.path(), Declared::load().unwrap(), "test-peer").unwrap()
}

fn input(id: &str, title: &str, extra: serde_json::Value) -> CreateContentInput {
    let mut object = serde_json::json!({"id": id, "title": title, "contentType": "concept"});
    if let (Some(base), Some(more)) = (object.as_object_mut(), extra.as_object()) {
        base.extend(more.clone());
    }
    // CreateContentInput reads snake_case (it is the db layer's input, not a wire view).
    let snake: serde_json::Map<String, serde_json::Value> = object
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| {
            let key = match k.as_str() {
                "contentType" => "content_type".to_string(),
                "contentFormat" => "content_format".to_string(),
                "contentBody" => "content_body".to_string(),
                "blobCid" => "blob_cid".to_string(),
                other => other.to_string(),
            };
            (key, v.clone())
        })
        .collect();
    serde_json::from_value(serde_json::Value::Object(snake)).unwrap()
}

fn create(peer: &Peer, content: CreateContentInput) {
    let mut conn = peer.pool.get().unwrap();
    content_diesel::create_content(&mut conn, &AppContext::default_lamad(), content).unwrap();
}

fn fold(peer: &Peer, index: &SearchIndex) -> elohim_storage::search::FoldReport {
    let mut conn = peer.pool.get().unwrap();
    index.fold_once(&mut conn).expect("a fold run")
}

/// `(unit_id, section)` of every live chunk the term matches, best bm25 first.
fn matches(index: &SearchIndex, term: &str) -> Vec<(String, String)> {
    let expression = match_expression(&[term.to_string()]).expect("a match expression");
    index.with_store(|store| {
        let mut hits = Vec::new();
        store
            .lexical(&expression, |id, unit, rank| {
                hits.push((rank, id, unit.to_string()))
            })
            .unwrap();
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        hits.into_iter()
            .map(|(_, id, unit)| (unit, store.chunk(id).unwrap().0))
            .collect()
    })
}

fn sql(peer: &Peer, statement: &str) {
    peer.pool.get().unwrap().batch_execute(statement).unwrap();
}

#[test]
fn store_path_is_under_storage_dir_index_measure_cid() {
    let peer = peer();
    let index = open(&peer);
    let cid = &index.declared().measure_cid;
    let expected = peer.dir.path().join(INDEX_DIR).join(cid).join(STORE_FILE);
    assert_eq!(index.store_path(), expected);
    assert_eq!(store_path(peer.dir.path(), cid), expected);
    assert!(expected.is_file(), "the store exists after open");
    assert_eq!(index.opened(), "created");
    drop(index);
    assert_eq!(open(&peer).opened(), "reused", "a sound store is reused");
}

#[test]
fn fold_indexes_title_description_body_and_tags() {
    let peer = peer();
    create(
        &peer,
        input(
            "commons-stewards",
            "The commons and its stewards",
            serde_json::json!({
                "description": "How gardeners share a watershed",
                "contentFormat": "markdown",
                "contentBody": "Intro line.\n\n# Soil\nLoam and compost.\n\n# Water\nSwales slow the rain.\n",
                "tags": ["permaculture", "stewardship"],
            }),
        ),
    );
    create(
        &peer,
        input("unrelated", "Bread baking", serde_json::json!({})),
    );
    let index = open(&peer);
    let report = fold(&peer, &index);
    assert_eq!(report.folded, 2, "{report:?}");
    assert_eq!(report.behind, 0);

    let unit = "commons-stewards".to_string();
    assert_eq!(
        matches(&index, "commons"),
        [(unit.clone(), HEAD_SECTION.into())]
    );
    assert_eq!(
        matches(&index, "gardeners"),
        [(unit.clone(), HEAD_SECTION.into())]
    );
    assert_eq!(
        matches(&index, "compost"),
        [(unit.clone(), "# Soil".into())]
    );
    assert_eq!(
        matches(&index, "swales"),
        [(unit.clone(), "# Water".into())]
    );
    assert_eq!(
        matches(&index, "permaculture"),
        [(unit, TAGS_SECTION.into())]
    );
    assert_eq!(
        matches(&index, "bread"),
        [("unrelated".into(), HEAD_SECTION.into())]
    );
}

#[test]
fn fold_is_incremental_by_fingerprint() {
    let peer = peer();
    create(
        &peer,
        input(
            "a",
            "Seed saving",
            serde_json::json!({"contentBody": "Keep the strongest plants.", "tags": ["seeds"]}),
        ),
    );
    let index = open(&peer);
    assert_eq!(fold(&peer, &index).folded, 1);
    let first = index.with_store(|s| s.tally().unwrap());
    assert_eq!(first.demoted, 0);

    // updated_at moves, nothing else: the row is seen and skipped.
    sql(
        &peer,
        "UPDATE content SET updated_at = datetime('now', '+1 second') WHERE id = 'a'",
    );
    let touched = fold(&peer, &index);
    assert_eq!((touched.folded, touched.unchanged), (0, 1), "{touched:?}");
    assert_eq!(index.with_store(|s| s.tally().unwrap()), first);

    // The body moves: the unit is re-chunked, its old chunks demoted, never deleted.
    sql(
        &peer,
        "UPDATE content SET content_body = 'Keep the hardiest plants.', \
         updated_at = datetime('now', '+2 seconds') WHERE id = 'a'",
    );
    let rebodied = fold(&peer, &index);
    assert_eq!(rebodied.folded, 1, "{rebodied:?}");
    let after = index.with_store(|s| s.tally().unwrap());
    assert_eq!(after.demoted, first.chunks, "every old chunk demoted once");
    assert!(matches(&index, "strongest").is_empty());
    assert_eq!(matches(&index, "hardiest").len(), 1);

    // A retitle with the same body re-folds too: the stat covers the head and the tags.
    sql(
        &peer,
        "UPDATE content SET title = 'Seed libraries', \
         updated_at = datetime('now', '+3 seconds') WHERE id = 'a'",
    );
    assert_eq!(fold(&peer, &index).folded, 1);
    assert_eq!(matches(&index, "libraries").len(), 1);

    // A row that is gone has its unit demoted by the next run.
    sql(
        &peer,
        "DELETE FROM content_tags WHERE content_id = 'a'; DELETE FROM content WHERE id = 'a'",
    );
    let gone = fold(&peer, &index);
    assert_eq!(gone.demoted, 1, "{gone:?}");
    assert!(index.with_store(|s| s.live_units().unwrap()).is_empty());
    assert_eq!(index.with_store(|s| s.tally().unwrap()).chunks, 0);
}

#[test]
fn fold_writes_attestation_json_and_log() {
    let peer = peer();
    create(&peer, input("a", "Mutual aid", serde_json::json!({})));
    let index = open(&peer);
    let report = fold(&peer, &index);
    let dir = index.store_dir().to_path_buf();
    let latest: FoldAttestation =
        serde_json::from_slice(&std::fs::read(dir.join(LATEST_FILE)).unwrap()).unwrap();
    assert_eq!(latest.measure.to_string(), index.declared().measure_cid);
    assert_eq!(latest.state, FoldState::Complete);
    assert_eq!(latest.shard.atoms, 1);
    assert_eq!(latest.heads_at.len(), 1, "one head for the Content surface");
    assert_eq!(latest.attested_by.0, "test-peer");
    assert_eq!(
        report.attestation_cid.as_deref(),
        Some(latest.cid().unwrap().to_string().as_str())
    );
    let log = || std::fs::read_to_string(dir.join(LOG_FILE)).unwrap();
    assert_eq!(log().lines().count(), 1);
    assert_eq!(index.snapshot().attestation.as_ref(), Some(&latest));

    // A run with nothing to do attests nothing: a timer does not grow the log by itself.
    assert_eq!(fold(&peer, &index).attestation_cid, None);
    assert_eq!(log().lines().count(), 1);

    // A run that changes the fold appends, and the log is never behind the snapshot.
    create(&peer, input("b", "Tool library", serde_json::json!({})));
    assert!(fold(&peer, &index).attestation_cid.is_some());
    let lines: Vec<String> = log().lines().map(str::to_string).collect();
    assert_eq!(lines.len(), 2);
    let last: FoldAttestation = serde_json::from_str(&lines[1]).unwrap();
    let latest: FoldAttestation =
        serde_json::from_slice(&std::fs::read(dir.join(LATEST_FILE)).unwrap()).unwrap();
    assert_eq!(last, latest);
    assert_eq!(latest.shard.atoms, 2);
}

#[test]
fn corrupt_store_is_recreated() {
    let peer = peer();
    create(&peer, input("a", "Repair cafe", serde_json::json!({})));
    let path = store_path(peer.dir.path(), &Declared::load().unwrap().measure_cid);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        b"this is not a sqlite database, it is a corrupt fold store",
    )
    .unwrap();

    let index = open(&peer);
    assert!(
        index.opened().starts_with("rebuilt ("),
        "a corrupt store is replaced: {}",
        index.opened()
    );
    assert_eq!(fold(&peer, &index).folded, 1);
    assert_eq!(matches(&index, "repair").len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notify_wakes_the_loop() {
    let peer = peer();
    let index = Arc::new(open(&peer));
    let content = ContentService::new(
        peer.pool.clone(),
        AppContext::default_lamad(),
        Arc::new(EventBus::new()),
    );
    content.attach_search_index(index.clone());

    // A sweep period no test waits out: only the boot run and a notify can fold.
    tokio::spawn(
        index
            .clone()
            .run_loop(peer.pool.clone(), Duration::from_secs(3600)),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while index.snapshot().runs == 0 {
        assert!(Instant::now() < deadline, "the boot run never happened");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // The content service's write wakes the loop; the new row is folded well inside the sweep.
    content
        .create(input(
            "woken",
            "Neighbourhood orchard",
            serde_json::json!({}),
        ))
        .expect("create through the content service");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let held = index.with_store(|s| s.live_units().unwrap());
        if held.contains_key("woken") {
            break;
        }
        assert!(Instant::now() < deadline, "a notify did not wake the loop");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        matches(&index, "orchard"),
        [("woken".into(), HEAD_SECTION.into())]
    );
}
