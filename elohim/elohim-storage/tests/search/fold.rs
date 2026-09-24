//! The content lexical fold (ruling R-S3): where it lives, what a unit holds, that it is
//! incremental by fingerprint, that it attests itself, that a notify wakes it, and that a corrupt
//! store is replaced rather than repaired.

use std::sync::Arc;
use std::time::{Duration, Instant};

use diesel::connection::SimpleConnection;
use diesel::RunQueryDsl;
use elohim_epr_index::attest::{LATEST_FILE, LOG_FILE};
use elohim_epr_index::terms::match_expression;
use elohim_epr_rea::{FoldAttestation, FoldState};
use elohim_storage::db::content_diesel::{self, CreateContentInput};
use elohim_storage::db::{init_pool_from_dir, AppContext, DbPool};
use elohim_storage::search::fold::{store_path, HEAD_SECTION, INDEX_DIR, STORE_FILE, TAGS_SECTION};
use elohim_storage::search::measure::{MEASURE_JSON, RECIPE_JSON};
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

/// Move every row's `updated_at` a minute into the past — out of the settle window, which
/// deliberately re-reads anything stamped within [`SETTLE_SECONDS`] of now. A fixture that needs
/// the watermark to advance across several runs ages its rows first; without this a small
/// per-run cap can never get past its first batch, because every row is still settling.
fn age_rows(peer: &Peer) {
    sql(
        peer,
        "UPDATE content SET updated_at = datetime('now', '-1 minute')",
    );
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

// ── The fold loop's bounds (ruling R-S10, the first-tranche review) ──

/// A `Declared` whose fold-lag limit — the per-run cap on rows read AND on units swept for
/// demotion — is `limit`, so a bound that the shipped 200 would need 200 rows to show is
/// measurable with a handful.
fn declared_with_cap(limit: u64) -> Declared {
    let mut measure: serde_json::Value = serde_json::from_str(MEASURE_JSON).unwrap();
    measure["foldLag"]["limit"] = serde_json::json!(limit);
    Declared::from_json(&measure.to_string(), RECIPE_JSON).expect("a capped declaration")
}

fn open_with(peer: &Peer, declared: Declared) -> SearchIndex {
    SearchIndex::open(peer.dir.path(), declared, "test-peer").unwrap()
}

#[test]
fn demote_set_is_capped_per_run_and_drained() {
    let peer = peer();
    let index = open_with(&peer, declared_with_cap(3));
    for n in 0..7 {
        create(
            &peer,
            input(
                &format!("orchard-{n}"),
                &format!("Orchard {n}"),
                serde_json::json!({}),
            ),
        );
    }
    age_rows(&peer);
    // Fold them in (3 rows a run, the same cap).
    for _ in 0..4 {
        fold(&peer, &index);
    }
    assert_eq!(matches(&index, "orchard").len(), 7, "all seven are folded");

    sql(&peer, "DELETE FROM content");
    let mut demoted = 0;
    let mut runs = 0;
    for _ in 0..10 {
        let report = fold(&peer, &index);
        assert!(
            report.demoted <= 3,
            "a run demotes at most the declared cap: {report:?}"
        );
        assert!(
            report.swept <= 3,
            "a run sweeps at most the declared cap: {report:?}"
        );
        demoted += report.demoted;
        runs += 1;
        if matches(&index, "orchard").is_empty() {
            break;
        }
    }
    assert_eq!(demoted, 7, "every unit drains across runs");
    assert!(
        runs >= 3,
        "seven units at three a run cannot drain in fewer: {runs}"
    );
    assert!(
        matches(&index, "orchard").is_empty(),
        "nothing stays live once its row is gone"
    );
}

#[test]
fn settle_cutoff_is_read_from_the_database_clock() {
    let peer = peer();
    let mut conn = peer.pool.get().unwrap();
    let cutoff = elohim_storage::search::fold::settle_cutoff(&mut conn).expect("the db clock");

    #[derive(diesel::QueryableByName)]
    struct At {
        #[diesel(sql_type = diesel::sql_types::Text)]
        at_key: String,
    }
    let now: String = diesel::sql_query("SELECT datetime('now') AS at_key")
        .get_result::<At>(&mut conn)
        .unwrap()
        .at_key;
    let floor: String = diesel::sql_query("SELECT datetime('now', '-30 seconds') AS at_key")
        .get_result::<At>(&mut conn)
        .unwrap()
        .at_key;
    // The cutoff is this database's own time, a settle window back — not a second clock's.
    assert!(cutoff < now, "{cutoff} is behind the db's now {now}");
    assert!(
        cutoff > floor,
        "{cutoff} is inside the settle window of {now}"
    );
    assert_eq!(cutoff.len(), now.len(), "the same datetime() spelling");
}

#[test]
fn forward_clock_step_does_not_park_the_watermark() {
    let peer = peer();
    let index = open_with(&peer, declared_with_cap(10));
    create(
        &peer,
        input("settled", "Settled row", serde_json::json!({})),
    );
    // A row stamped ahead of the database's clock — a writer whose clock stepped forward.
    create(
        &peer,
        input("ahead", "Row from the future", serde_json::json!({})),
    );
    sql(
        &peer,
        "UPDATE content SET updated_at = datetime('now', '+1 hour') WHERE id = 'ahead'",
    );

    let first = fold(&peer, &index);
    assert_eq!(first.folded, 2, "both rows fold: {first:?}");

    // The cutoff is the DATABASE's now, so the future row sits beyond it and the watermark
    // clamps to the settle window — by design, that row is re-read (and found unchanged) until
    // the clock catches up. What must NOT happen is the run reporting a backlog it has already
    // folded, or attesting degraded forever because one writer's clock ran ahead.
    let mut last = first;
    for _ in 0..3 {
        last = fold(&peer, &index);
    }
    assert_eq!(last.folded, 0, "nothing is re-chunked: {last:?}");
    assert_eq!(
        last.behind, 0,
        "no phantom backlog behind the watermark: {last:?}"
    );
    assert!(
        last.unchanged <= 2,
        "at most the settling batch is re-read, never the corpus: {last:?}"
    );
    assert!(
        matches!(
            index.snapshot().attestation.map(|a| a.state),
            Some(FoldState::Complete)
        ),
        "a fold that has read everything it can date attests complete"
    );
    assert_eq!(matches(&index, "future").len(), 1, "it is searchable");
}

#[test]
fn a_no_op_sweep_reads_only_a_bounded_page() {
    let peer = peer();
    let index = open_with(&peer, declared_with_cap(4));
    for n in 0..20 {
        create(
            &peer,
            input(
                &format!("meadow-{n:02}"),
                &format!("Meadow {n}"),
                serde_json::json!({}),
            ),
        );
    }
    age_rows(&peer);
    for _ in 0..6 {
        fold(&peer, &index);
    }
    assert_eq!(matches(&index, "meadow").len(), 20);

    // Steady state: nothing changed, so a run reads one page of units (the cap) and nothing else.
    // The old sweep materialised every live unit AND every content id on every run.
    for _ in 0..5 {
        let report = fold(&peer, &index);
        assert_eq!(
            report.demoted, 0,
            "a no-op sweep demotes nothing: {report:?}"
        );
        assert!(
            report.swept <= 4,
            "a no-op sweep reads one bounded page, not the corpus: {report:?}"
        );
    }
    assert_eq!(matches(&index, "meadow").len(), 20, "nothing was lost");
}

#[test]
fn unparsable_updated_at_is_counted_and_keeps_the_fold_honest() {
    let peer = peer();
    let index = open_with(&peer, declared_with_cap(10));
    create(
        &peer,
        input("readable", "Readable row", serde_json::json!({})),
    );
    create(
        &peer,
        input("undated", "Undated row", serde_json::json!({})),
    );
    sql(
        &peer,
        "UPDATE content SET updated_at = 'whenever, really' WHERE id = 'undated'",
    );

    let report = fold(&peer, &index);
    assert_eq!(
        report.unparsed_at, 1,
        "the undated row is counted: {report:?}"
    );
    let state = index.snapshot().attestation.expect("a run attests").state;
    assert!(
        matches!(state, FoldState::Degraded { .. }),
        "a fold that could not date a row it read does not claim complete: {state:?}"
    );
}

#[test]
fn rebuild_starts_the_retry_count_fresh() {
    let peer = peer();
    let index = open(&peer);
    let dir = index.store_dir().to_path_buf();
    create(&peer, input("a", "Repair cafe", serde_json::json!({})));
    fold(&peer, &index);
    // Stand in for a history of failed runs: an attestation whose retry count is high.
    let attestation: FoldAttestation =
        serde_json::from_slice(&std::fs::read(dir.join(LATEST_FILE)).unwrap()).unwrap();
    let degraded = FoldAttestation {
        state: FoldState::Degraded { retried: 9 },
        ..attestation
    };
    std::fs::write(
        dir.join(LATEST_FILE),
        serde_json::to_vec(&degraded).unwrap(),
    )
    .unwrap();
    drop(index);

    // A store that cannot serve is replaced — and the attestation history goes with it, because
    // it describes the store that is gone (review m1).
    std::fs::write(
        store_path(peer.dir.path(), &Declared::load().unwrap().measure_cid),
        b"not a database",
    )
    .unwrap();
    let index = open(&peer);
    assert!(
        index.opened().starts_with("rebuilt ("),
        "{}",
        index.opened()
    );
    assert!(
        !dir.join(LATEST_FILE).exists(),
        "the latest attestation went with the store"
    );
    assert!(!dir.join(LOG_FILE).exists(), "so did the log");
    assert!(
        index.snapshot().attestation.is_none(),
        "a rebuilt store carries no attestation from the one it replaced"
    );

    // Its first degraded run starts counting from zero, not from the replaced store's nine.
    for _ in 0..12 {
        create(
            &peer,
            input(
                &format!("row-{}", uuid::Uuid::new_v4().simple()),
                "Repair cafe",
                serde_json::json!({}),
            ),
        );
    }
    let index = open_with(&peer, declared_with_cap(2));
    fold(&peer, &index);
    if let Some(FoldState::Degraded { retried }) = index.snapshot().attestation.map(|a| a.state) {
        assert!(retried <= 1, "the retry count starts fresh, got {retried}");
    }
}
