//! Tests for the accountable-correction application path.
//!
//! Every one of these runs against a real in-memory SQLite (the migrations are
//! the schema under test) and a FAKE `FeedbackDhtReader` — no conductor. The
//! trait is the seam that makes that possible.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;

use super::*;
use crate::db::feedback_application as app_db;
use crate::db::feedback_subscriptions as sub_db;
use crate::db::standing_generations as gen_db;
use crate::db::{run_migrations, DbPool};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const DNA: &str = "uhC0kTESTDNA";

fn test_pool() -> DbPool {
    let url = format!(
        "file:feedback_projector_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(&url))
        .expect("pool");
    run_migrations(&pool).expect("migrations");
    pool
}

fn key(seed: u8) -> Vec<u8> {
    vec![seed; 32]
}

/// A 39-byte `get_raw_39()`-shaped key wrapping the same 32 bytes.
fn key39(seed: u8) -> Vec<u8> {
    let mut v = vec![0x84u8, 0x20, 0x24];
    v.extend_from_slice(&key(seed));
    v.extend_from_slice(&[1, 2, 3, 4]);
    v
}

fn correction_entry(target: &str, evidence: &str, impact: &str) -> DhtFeedbackSignal {
    DhtFeedbackSignal {
        target_cid: target.to_string(),
        signal_kind: "correction".to_string(),
        vouch_kind: None,
        evidence_cid: Some(evidence.to_string()),
        standing_impact: impact.to_string(),
        signer_pubkey: vec![],
    }
}

fn vouch_entry(correction_action: &str) -> DhtFeedbackSignal {
    DhtFeedbackSignal {
        target_cid: correction_action.to_string(),
        signal_kind: "vouch".to_string(),
        vouch_kind: Some("accept-correction".to_string()),
        evidence_cid: None,
        standing_impact: "debit-soft".to_string(),
        signer_pubkey: vec![],
    }
}

fn act(action_hash: &str, author: u8, ts: i64, entry: DhtFeedbackSignal) -> VerifiedAct {
    VerifiedAct {
        action_hash: action_hash.to_string(),
        origin_dna_hash: DNA.to_string(),
        author: key(author),
        timestamp_micros: ts,
        entry,
    }
}

fn record(action_hash: &str, author: u8, ts: i64, entry: DhtFeedbackSignal) -> FetchedRecord {
    FetchedRecord {
        action_hash: action_hash.to_string(),
        author_raw: key39(author),
        timestamp_micros: ts,
        entry_hash_bound: true,
        entry: Some(entry),
        entry_bytes_len: 256,
    }
}

/// A `uhCAk…`-shaped display of a 32-byte key, decodable by
/// [`decode_agent_display`].
fn agent_display(seed: u8) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    format!("u{}", URL_SAFE_NO_PAD.encode(key(seed)))
}

#[derive(Default)]
struct FakeState {
    refs: HashMap<String, Vec<DiscoveredRef>>,
    records: HashMap<String, FetchedRecord>,
    lineages: HashMap<String, ContentLineage>,
    /// Members enumerated, in visit order — the rotation-fairness witness.
    visited: Vec<String>,
}

struct FakeReader {
    state: Mutex<FakeState>,
}

impl FakeReader {
    fn new() -> Self {
        Self {
            state: Mutex::new(FakeState::default()),
        }
    }
    fn with_refs(self, base: &str, refs: Vec<DiscoveredRef>) -> Self {
        self.state
            .lock()
            .unwrap()
            .refs
            .insert(base.to_string(), refs);
        self
    }
    fn with_record(self, r: FetchedRecord) -> Self {
        self.state
            .lock()
            .unwrap()
            .records
            .insert(r.action_hash.clone(), r);
        self
    }
    fn with_lineage(self, action: &str, l: ContentLineage) -> Self {
        self.state
            .lock()
            .unwrap()
            .lineages
            .insert(action.to_string(), l);
        self
    }
    fn visited(&self) -> Vec<String> {
        self.state.lock().unwrap().visited.clone()
    }
}

#[async_trait::async_trait]
impl FeedbackDhtReader for FakeReader {
    fn origin_dna_hash(&self) -> String {
        DNA.to_string()
    }
    async fn refs_for_target(
        &self,
        base_action_hash: &str,
    ) -> Result<Vec<DiscoveredRef>, StorageError> {
        let mut s = self.state.lock().unwrap();
        s.visited.push(base_action_hash.to_string());
        Ok(s.refs.get(base_action_hash).cloned().unwrap_or_default())
    }
    async fn signal_record(
        &self,
        action_hash: &str,
    ) -> Result<Option<FetchedRecord>, StorageError> {
        Ok(self.state.lock().unwrap().records.get(action_hash).cloned())
    }
    async fn content_lineage(
        &self,
        action_hash: &str,
    ) -> Result<Option<ContentLineage>, StorageError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .lineages
            .get(action_hash)
            .cloned())
    }
}

fn lineage(referenced: &str, root: &str, root_author: u8) -> ContentLineage {
    ContentLineage {
        referenced_action_hash: referenced.to_string(),
        root_action_hash: root.to_string(),
        root_author: agent_display(root_author),
        content_id: "doc-1".to_string(),
        candidates: vec![],
        head_action_hash: Some(root.to_string()),
        contested: false,
        contested_predecessors: vec![],
    }
}

fn referenced(action: &str) -> DiscoveredRef {
    DiscoveredRef {
        action_hash: action.to_string(),
        fetch_outcome: "referenced".to_string(),
    }
}

// ---------------------------------------------------------------------------
// §3 — fair rotation
// ---------------------------------------------------------------------------

/// The ninth member of a nine-member set IS visited.
///
/// This is the exact starvation the release-adoption watcher's
/// `take(MAX_CHANNELS_PER_SWEEP)` produces: with a budget of 8 and 9 members,
/// the ninth is never reached because the take always starts at the head.
#[tokio::test]
async fn ninth_subscription_is_visited_within_two_ticks() {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        for i in 1..=9 {
            sub_db::add_member(
                &mut conn,
                sub_db::KIND_CONTENT_TARGET,
                &format!("target-{i:02}"),
                DNA,
                sub_db::SOURCE_STEWARD,
                "2026-09-06T00:00:00Z",
            )
            .unwrap();
        }
    }
    let reader = Arc::new(FakeReader::new());
    let projector =
        FeedbackProjector::new(pool, reader.clone(), key(0xE0), PinnedPolicy::default());

    let t1 = projector.tick().await.expect("tick 1");
    assert_eq!(t1.members_visited, MAX_MEMBERS_PER_SWEEP as usize);
    let t2 = projector.tick().await.expect("tick 2");
    assert!(t2.members_visited > 0);

    let visited = reader.visited();
    assert!(
        visited.iter().any(|m| m == "target-09"),
        "the ninth member must be visited within ceil(9/8)=2 ticks; visited: {visited:?}"
    );
}

// ---------------------------------------------------------------------------
// §7 — group dedup, replay, late arrival
// ---------------------------------------------------------------------------

fn accepted_group_fixture() -> (DbPool, Arc<FakeReader>, FeedbackProjector) {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        sub_db::add_member(
            &mut conn,
            sub_db::KIND_CORRECTION_ACTION,
            "corr-1",
            DNA,
            sub_db::SOURCE_STEWARD,
            "2026-09-06T00:00:00Z",
        )
        .unwrap();
    }
    let reader = Arc::new(
        FakeReader::new()
            // The correction action is the base acceptance vouches link from.
            .with_refs("corr-1", vec![referenced("vouch-1")])
            .with_record(record(
                "corr-1",
                2,
                100,
                correction_entry("content-1", "ev-1", "debit-soft"),
            ))
            .with_record(record("vouch-1", 1, 200, vouch_entry("corr-1")))
            .with_lineage("content-1", lineage("content-1", "content-1", 1)),
    );
    let projector = FeedbackProjector::new(
        pool.clone(),
        reader.clone(),
        key(0xE0),
        PinnedPolicy::default(),
    );
    (pool, reader, projector)
}

/// An accepted correction contributes ONCE, against the TARGET's root author.
#[tokio::test]
async fn accepted_correction_contributes_once_against_the_root_author() {
    let (pool, _reader, projector) = accepted_group_fixture();
    let report = projector.tick().await.expect("tick");
    assert_eq!(report.groups_applied, 1, "report: {report:?}");

    let mut conn = pool.get().unwrap();
    let gen = current_generation(&mut conn)
        .unwrap()
        .expect("generation opened");
    let row = app_db::fetch_application(&mut conn, gen.generation_id, "ev-1")
        .unwrap()
        .expect("group applied under its EVIDENCE action hash");
    assert_eq!(row.status, app_db::STATUS_APPLIED);
    assert_eq!(row.accepted, 1);
    assert_eq!(row.contribution, 2, "debit-soft accepted correction");
    assert_eq!(
        row.subject_pubkey.as_deref(),
        Some(key(1).as_slice()),
        "subject is the TARGET's exact root author, never the signal's signer"
    );
    // MAXIMUM included timestamp, not the last enumerated.
    assert_eq!(row.max_included_at_micros, Some(200));

    let agg = gen_db::fetch_aggregate(&mut conn, gen.generation_id, &key(0xE0), &key(1))
        .unwrap()
        .expect("aggregate");
    assert_eq!(agg.debit_weight_sum, 2);
}

/// Replay after commit is a NO-OP: a second tick over the same acts does not
/// move the aggregate again.
#[tokio::test]
async fn replay_after_commit_is_a_no_op() {
    let (pool, _reader, projector) = accepted_group_fixture();
    projector.tick().await.expect("tick 1");
    projector.tick().await.expect("tick 2");
    projector.tick().await.expect("tick 3");

    let mut conn = pool.get().unwrap();
    let gen = current_generation(&mut conn).unwrap().unwrap();
    let agg = gen_db::fetch_aggregate(&mut conn, gen.generation_id, &key(0xE0), &key(1))
        .unwrap()
        .expect("aggregate");
    assert_eq!(
        agg.debit_weight_sum, 2,
        "three ticks over one accepted group must contribute once"
    );
}

/// Two members of ONE group contribute ONCE. This is the rev-2 review's
/// convergence hole: keyed on the action, a second same-operation act would
/// have been a second contribution.
#[tokio::test]
async fn two_members_of_one_group_contribute_once() {
    let (pool, _r, projector) = accepted_group_fixture();
    projector.tick().await.expect("tick");

    let mut conn = pool.get().unwrap();
    let gen = current_generation(&mut conn).unwrap().unwrap();
    let members = app_db::members_of_group(&mut conn, gen.generation_id, "ev-1").unwrap();
    assert_eq!(members.len(), 2, "the vouch and the correction it accepts");
    let agg = gen_db::fetch_aggregate(&mut conn, gen.generation_id, &key(0xE0), &key(1))
        .unwrap()
        .unwrap();
    assert_eq!(agg.debit_weight_sum, 2, "two members, one contribution");
}

/// A LATE-ARRIVING act discovered after the group already applied is recorded
/// as a member and contributes nothing more.
#[tokio::test]
async fn late_arrival_after_apply_records_without_recontributing() {
    let (pool, _r, projector) = accepted_group_fixture();
    projector
        .tick()
        .await
        .expect("first tick applies the group");

    let mut conn = pool.get().unwrap();
    let gen = current_generation(&mut conn).unwrap().unwrap();
    // A second, OLDER correction under the same operation shows up now.
    let outcome = GroupOutcome {
        group_key: "ev-1".to_string(),
        status: app_db::STATUS_APPLIED.to_string(),
        contribution: 99,
        subject: Some(key(1)),
        accepted: true,
        max_included_at_micros: Some(50),
        error: None,
        members: vec![MemberRow {
            generation_id: gen.generation_id,
            origin_dna_hash: DNA.to_string(),
            action_hash: "corr-1-late".to_string(),
            group_key: "ev-1".to_string(),
            member_status: app_db::MEMBER_STATUS_MEMBER.to_string(),
            member_role: app_db::MEMBER_ROLE_CORRECTION.to_string(),
            author_pubkey: Some(key(2)),
            action_timestamp_micros: Some(50),
            last_error: None,
            discovered_at: "2026-09-06T00:00:00Z".to_string(),
        }],
    };
    let changed = commit_group(&mut conn, gen.generation_id, &key(0xE0), &outcome).unwrap();
    assert!(!changed, "an already-applied group does not re-apply");

    let members = app_db::members_of_group(&mut conn, gen.generation_id, "ev-1").unwrap();
    assert_eq!(members.len(), 3, "the late member IS recorded");
    let agg = gen_db::fetch_aggregate(&mut conn, gen.generation_id, &key(0xE0), &key(1))
        .unwrap()
        .unwrap();
    assert_eq!(
        agg.debit_weight_sum, 2,
        "a late member never re-contributes, whatever weight it claims"
    );
}

/// CRASH WINDOW 1: the application row is written and the aggregate write
/// fails. Both must roll back — a committed row with no aggregate would make
/// the contribution unrecoverable (replay sees "already applied").
#[tokio::test]
async fn crash_between_row_and_aggregate_rolls_both_back() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let gen_id = gen_db::create_generation(
        &mut conn,
        &key(0xE0),
        "sha256:test",
        "{}",
        gen_db::GEN_BUILDING,
        "2026-09-06T00:00:00Z",
    )
    .unwrap();

    // Seed an aggregate at i32::MAX so the checked add inside the transaction
    // overflows — the aggregate write fails AFTER the application row was
    // written inside the same transaction.
    gen_db::upsert_aggregate(
        &mut conn,
        &GenerationAggregateRow {
            generation_id: gen_id,
            evaluator_pubkey: key(0xE0),
            subject_pubkey: key(1),
            debit_weight_sum: i32::MAX,
            last_signal_at_micros: Some(1),
        },
    )
    .unwrap();

    let outcome = GroupOutcome {
        group_key: "ev-overflow".to_string(),
        status: app_db::STATUS_APPLIED.to_string(),
        contribution: 1,
        subject: Some(key(1)),
        accepted: true,
        max_included_at_micros: Some(10),
        error: None,
        members: vec![],
    };
    let result = commit_group(&mut conn, gen_id, &key(0xE0), &outcome);
    assert!(result.is_err(), "checked overflow must abort the apply");

    assert!(
        app_db::fetch_application(&mut conn, gen_id, "ev-overflow")
            .unwrap()
            .is_none(),
        "the application row must roll back with the aggregate — a committed row \
         with no contribution would make replay a permanent no-op"
    );
    let agg = gen_db::fetch_aggregate(&mut conn, gen_id, &key(0xE0), &key(1))
        .unwrap()
        .unwrap();
    assert_eq!(agg.debit_weight_sum, i32::MAX, "aggregate untouched");
}

// ---------------------------------------------------------------------------
// §5.2 — acceptance verification
// ---------------------------------------------------------------------------

#[test]
fn acceptance_by_the_root_author_is_accepted() {
    let corr = act(
        "corr-1",
        2,
        100,
        correction_entry("content-1", "ev-1", "debit-soft"),
    );
    let vouch = act("vouch-1", 1, 200, vouch_entry("corr-1"));
    let l = lineage("content-1", "content-1", 1);
    let verdict = verify_acceptance(&vouch, &corr, Some(&l), Some(&key(1)));
    assert_eq!(verdict, AcceptanceVerdict::Accepted { subject: key(1) });
}

/// Integrity admits any vouch by anyone. Authority is a STORAGE-verified
/// classification, and a non-root-author acceptance is REFUSED, not pending.
#[test]
fn acceptance_by_a_non_root_author_is_refused() {
    let corr = act(
        "corr-1",
        2,
        100,
        correction_entry("content-1", "ev-1", "debit-soft"),
    );
    let vouch = act("vouch-1", 7, 200, vouch_entry("corr-1"));
    let l = lineage("content-1", "content-1", 1);
    match verify_acceptance(&vouch, &corr, Some(&l), Some(&key(1))) {
        AcceptanceVerdict::Refused(reason) => {
            assert!(reason.contains("root author"), "got: {reason}")
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// An unfetchable dependency is PENDING — retried, never rejected, and never
/// shown as accepted.
#[test]
fn unfetchable_dependency_is_pending_not_refused() {
    let corr = act(
        "corr-1",
        2,
        100,
        correction_entry("content-1", "ev-1", "debit-soft"),
    );
    let vouch = act("vouch-1", 1, 200, vouch_entry("corr-1"));
    match verify_acceptance(&vouch, &corr, None, None) {
        AcceptanceVerdict::Pending(reason) => {
            assert!(reason.contains("lineage"), "got: {reason}")
        }
        other => panic!("expected Pending, got {other:?}"),
    }
}

/// Same-cell self-acceptance stays visibly PENDING — never reported as
/// accepted (§5.2, explicitly unsupported in slice 1).
#[test]
fn same_cell_self_acceptance_is_pending_never_accepted() {
    let corr = act(
        "corr-1",
        1,
        100,
        correction_entry("content-1", "ev-1", "debit-soft"),
    );
    let vouch = act("vouch-1", 1, 200, vouch_entry("corr-1"));
    let l = lineage("content-1", "content-1", 1);
    match verify_acceptance(&vouch, &corr, Some(&l), Some(&key(1))) {
        AcceptanceVerdict::Pending(reason) => {
            assert!(reason.contains("self-acceptance"), "got: {reason}")
        }
        other => panic!("expected Pending, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// §1 — record verification
// ---------------------------------------------------------------------------

#[test]
fn foreign_origin_dna_is_rejected() {
    match verify_record("a-1", DNA, "uhC0kOTHERDNA", None) {
        ActVerification::Rejected(r) => assert!(r.contains("foreign origin DNA")),
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn absent_entry_bytes_are_pending_not_evidence() {
    let mut r = record("a-1", 1, 10, correction_entry("c", "e", "debit-soft"));
    r.entry = None;
    match verify_record("a-1", DNA, DNA, Some(r)) {
        ActVerification::Pending(reason) => {
            assert!(reason.contains("signed header alone"), "got: {reason}")
        }
        other => panic!("expected Pending, got {other:?}"),
    }
}

#[test]
fn substituted_action_hash_is_rejected() {
    let r = record("a-2", 1, 10, correction_entry("c", "e", "debit-soft"));
    match verify_record("a-1", DNA, DNA, Some(r)) {
        ActVerification::Rejected(reason) => assert!(reason.contains("names action")),
        other => panic!("expected Rejected, got {other:?}"),
    }
}

/// 39-byte `get_raw_39()` and bare 32-byte representations of ONE key must
/// compare equal. Comparing them raw silently never matches, which turns an
/// author gate into a permanent refusal.
#[test]
fn agent_key_normalisation_bridges_39_and_32_byte_forms() {
    assert_eq!(normalize_agent_key(&key39(5)), key(5));
    assert_eq!(normalize_agent_key(&key(5)), key(5));
}

// ---------------------------------------------------------------------------
// §7 — policy, Unknown vs Neutral, rebuild
// ---------------------------------------------------------------------------

/// An allegation debits nobody: a correction with no acceptance contributes 0
/// and leaves the subject's aggregate ABSENT (readers see Unknown, not
/// Neutral).
#[tokio::test]
async fn correction_alone_leaves_the_subject_absent() {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        sub_db::add_member(
            &mut conn,
            sub_db::KIND_CONTENT_TARGET,
            "content-1",
            DNA,
            sub_db::SOURCE_STEWARD,
            "2026-09-06T00:00:00Z",
        )
        .unwrap();
    }
    let reader = Arc::new(
        FakeReader::new()
            .with_refs("content-1", vec![referenced("corr-1")])
            .with_record(record(
                "corr-1",
                2,
                100,
                correction_entry("content-1", "ev-1", "debit-firm"),
            ))
            .with_lineage("content-1", lineage("content-1", "content-1", 1)),
    );
    let projector =
        FeedbackProjector::new(pool.clone(), reader, key(0xE0), PinnedPolicy::default());
    projector.tick().await.expect("tick");

    let mut conn = pool.get().unwrap();
    let gen = current_generation(&mut conn).unwrap().unwrap();
    let row = app_db::fetch_application(&mut conn, gen.generation_id, "ev-1")
        .unwrap()
        .expect("group opened");
    assert_eq!(row.contribution, 0, "an allegation debits nobody");
    assert_eq!(row.accepted, 0);
    assert!(
        gen_db::fetch_aggregate(&mut conn, gen.generation_id, &key(0xE0), &key(1))
            .unwrap()
            .is_none(),
        "no accepted correction => NO aggregate row => readers see Unknown, not Neutral"
    );
}

#[test]
fn policy_contributes_only_when_accepted() {
    let p = PinnedPolicy::default();
    assert_eq!(p.contribution("correction", false, "debit-firm"), 0);
    assert_eq!(p.contribution("correction", true, "debit-soft"), 2);
    assert_eq!(p.contribution("correction", true, "debit-firm"), 8);
    assert_eq!(p.contribution("correction", true, "advisory"), 0);
}

/// A rebuilt generation reproduces the SAME canonical rows as the published
/// one: identical policy digest, identical application rows, identical
/// aggregate — SQLite layout and operational timestamps excluded.
#[tokio::test]
async fn rebuild_reproduces_the_published_generation() {
    let (pool, _r, projector) = accepted_group_fixture();
    projector.tick().await.expect("build");

    let (gen_id, before_apps, before_aggs) = {
        let mut conn = pool.get().unwrap();
        let gen = current_generation(&mut conn).unwrap().unwrap();
        let apps = app_db::members_of_group(&mut conn, gen.generation_id, "ev-1").unwrap();
        let aggs = gen_db::list_aggregates(&mut conn, gen.generation_id).unwrap();
        publish_generation(&mut conn, gen.generation_id, &key(0xE0), "sha256:test").unwrap();
        (gen.generation_id, apps, aggs)
    };
    assert!(!before_aggs.is_empty());

    // `rebuilding` is a VISIBLE state, not a silent gap.
    {
        let mut conn = pool.get().unwrap();
        begin_rebuild(&mut conn, gen_id).unwrap();
        let g = gen_db::fetch_generation(&mut conn, gen_id)
            .unwrap()
            .unwrap();
        assert_eq!(g.status, gen_db::GEN_REBUILDING);
        assert!(gen_db::list_aggregates(&mut conn, gen_id)
            .unwrap()
            .is_empty());
    }

    projector.tick().await.expect("replay");

    let mut conn = pool.get().unwrap();
    let after_apps = app_db::members_of_group(&mut conn, gen_id, "ev-1").unwrap();
    let after_aggs = gen_db::list_aggregates(&mut conn, gen_id).unwrap();
    assert_eq!(
        before_aggs
            .iter()
            .map(|a| (a.subject_pubkey.clone(), a.debit_weight_sum))
            .collect::<Vec<_>>(),
        after_aggs
            .iter()
            .map(|a| (a.subject_pubkey.clone(), a.debit_weight_sum))
            .collect::<Vec<_>>(),
        "rebuild must reproduce the published generation's canonical aggregate"
    );
    assert_eq!(
        before_apps
            .iter()
            .map(|m| (m.action_hash.clone(), m.group_key.clone()))
            .collect::<Vec<_>>(),
        after_apps
            .iter()
            .map(|m| (m.action_hash.clone(), m.group_key.clone()))
            .collect::<Vec<_>>(),
        "and the same member rows"
    );
}

#[test]
fn backoff_grows_and_is_capped() {
    assert_eq!(backoff_secs(0), RETRY_BASE_SECS);
    assert_eq!(backoff_secs(1), RETRY_BASE_SECS * 2);
    assert_eq!(backoff_secs(30), RETRY_MAX_SECS);
}

#[test]
fn agent_display_round_trips() {
    assert_eq!(decode_agent_display(&agent_display(9)), Some(key(9)));
    // PANIC GUARD: an empty suffix must not reach the hash decoder.
    assert_eq!(decode_agent_display("u"), None);
    assert_eq!(decode_agent_display(""), None);
}

#[tokio::test]
async fn spawned_loop_visits_subscription_on_first_tick() {
    let pool = test_pool();
    sub_db::add_member(
        &mut pool.get().unwrap(),
        sub_db::KIND_CONTENT_TARGET,
        "target",
        DNA,
        sub_db::SOURCE_STEWARD,
        "2026-09-06T00:00:00Z",
    )
    .unwrap();
    let reader = Arc::new(FakeReader::new());
    let task = spawn(FeedbackProjector::new(
        pool,
        reader.clone(),
        key(0xE0),
        PinnedPolicy::default(),
    ));
    let result = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if !reader.state.lock().unwrap().visited.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    task.abort();
    result.expect("registered loop must tick without waiting for its first interval");
    assert_eq!(reader.state.lock().unwrap().visited, vec!["target"]);
}

fn current_generation(
    conn: &mut SqliteConnection,
) -> Result<Option<gen_db::GenerationRow>, diesel::result::Error> {
    Ok(gen_db::published_generation(conn, &key(0xE0))?
        .or(gen_db::in_flight_generation(conn, &key(0xE0))?))
}

#[tokio::test]
async fn acceptance_after_zero_contribution_applies_once() {
    let (pool, reader, projector) = accepted_group_fixture();
    let acceptance_refs = reader.state.lock().unwrap().refs.remove("corr-1");
    let generation = resolve_generation(
        &mut pool.get().unwrap(),
        &key(0xE0),
        &PinnedPolicy::default(),
    )
    .unwrap();
    projector
        .project_correction(
            generation,
            &act(
                "corr-1",
                2,
                100,
                correction_entry("content-1", "ev-1", "debit-soft"),
            ),
        )
        .unwrap();
    projector.tick().await.unwrap();
    if let Some(refs) = acceptance_refs {
        reader
            .state
            .lock()
            .unwrap()
            .refs
            .insert("corr-1".into(), refs);
    }
    projector.tick().await.unwrap();
    projector.tick().await.unwrap();
    let mut conn = pool.get().unwrap();
    let gen = current_generation(&mut conn).unwrap().unwrap();
    let aggregate = gen_db::fetch_aggregate(&mut conn, gen.generation_id, &key(0xE0), &key(1))
        .unwrap()
        .unwrap();
    assert_eq!(aggregate.debit_weight_sum, 2);
}

#[tokio::test]
async fn fresh_rebuild_replays_retained_acts_after_links_disappear() {
    let (pool, reader, projector) = accepted_group_fixture();
    projector.tick().await.unwrap();
    let new_id = {
        let mut conn = pool.get().unwrap();
        rebuild::start(&mut conn, &key(0xE0)).unwrap()
    };
    reader.state.lock().unwrap().refs.clear();
    projector.tick().await.unwrap();
    let mut conn = pool.get().unwrap();
    let generation = gen_db::published_generation(&mut conn, &key(0xE0))
        .unwrap()
        .unwrap();
    assert_eq!(generation.generation_id, new_id);
    assert_eq!(
        gen_db::fetch_aggregate(&mut conn, new_id, &key(0xE0), &key(1))
            .unwrap()
            .unwrap()
            .debit_weight_sum,
        2
    );
    assert_eq!(
        app_db::members_of_group(&mut conn, new_id, "ev-1")
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn pending_acceptance_retries_without_holding_a_database_connection() {
    let (pool, reader, projector) = accepted_group_fixture();
    let root = reader
        .state
        .lock()
        .unwrap()
        .lineages
        .remove("content-1")
        .unwrap();
    projector.tick().await.unwrap();
    assert!(
        gen_db::published_generation(&mut pool.get().unwrap(), &key(0xE0))
            .unwrap()
            .is_none()
    );
    reader
        .state
        .lock()
        .unwrap()
        .lineages
        .insert("content-1".into(), root);
    projector.tick().await.unwrap();
    let mut conn = pool.get().unwrap();
    let generation = gen_db::published_generation(&mut conn, &key(0xE0))
        .unwrap()
        .unwrap();
    assert_eq!(
        gen_db::fetch_aggregate(&mut conn, generation.generation_id, &key(0xE0), &key(1))
            .unwrap()
            .unwrap()
            .debit_weight_sum,
        2
    );
}

// ---------------------------------------------------------------------------
// Sweep heat — cold-retire, re-arm, ordering (backlog atom
// `feedback-discovery-sweep-is-o-n-in-history.md`, cures (a) + (b))
// ---------------------------------------------------------------------------

/// Seed quiet content-target members and a projector on an ISOLATED scheduler.
/// Quiet = the fake reader answers with no references at all, which is exactly
/// the settled history the atom found the rotation spending its whole budget
/// on.
fn quiet_member_fixture(keys: &[&str]) -> (DbPool, Arc<FakeReader>, FeedbackProjector) {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        for k in keys {
            sub_db::add_member(
                &mut conn,
                sub_db::KIND_CONTENT_TARGET,
                k,
                DNA,
                sub_db::SOURCE_STEWARD,
                "2026-09-06T00:00:00Z",
            )
            .unwrap();
        }
    }
    let reader = Arc::new(FakeReader::new());
    let projector = FeedbackProjector::with_scheduler(
        pool.clone(),
        reader.clone(),
        key(0xE0),
        PinnedPolicy::default(),
        Arc::new(SweepScheduler::new()),
    );
    (pool, reader, projector)
}

fn act_ref(action_hash: &str, routing_key: &str) -> crate::p2p::feedback_signal::FeedbackActRef {
    crate::p2p::feedback_signal::FeedbackActRef {
        origin_dna_hash: DNA.to_string(),
        action_hash: action_hash.to_string(),
        routing_key: routing_key.to_string(),
    }
}

/// (1) A member with nothing open retires after EXACTLY K clean sweeps, and the
/// (K+1)th sweep does not pay a `get_links` for it.
///
/// This is the whole cost cure. The atom measured `ceil(N/8)` sweeps of
/// acceptance→application lag over a set that had grown to 127 members of
/// settled history, so what matters is not that a quiet member is swept less
/// often but that it leaves the denominator entirely.
#[tokio::test]
async fn quiet_member_goes_cold_after_exactly_three_clean_sweeps() {
    let (_pool, reader, projector) = quiet_member_fixture(&["target-quiet"]);
    let scheduler = projector.scheduler();

    for expected in [MemberHeat::Warm(1), MemberHeat::Warm(2), MemberHeat::Cold] {
        let report = projector.tick().await.expect("tick");
        assert_eq!(report.members_visited, 1, "report: {report:?}");
        assert_eq!(
            scheduler.heat(sub_db::KIND_CONTENT_TARGET, "target-quiet"),
            expected
        );
    }
    assert_eq!(
        reader.visited().len(),
        COLD_AFTER_CLEAN_SWEEPS as usize,
        "exactly K enumerations before retirement, no more"
    );

    let fourth = projector.tick().await.expect("fourth tick");
    assert_eq!(
        fourth.members_visited, 0,
        "a Cold member is absent from the sweep's member set"
    );
    assert_eq!(fourth.members_cold, 1);
    assert_eq!(
        reader.visited().len(),
        COLD_AFTER_CLEAN_SWEEPS as usize,
        "the fourth sweep must not spend a get_links on the retired member"
    );
}

/// (2) A notification through the real ingress path re-arms a Cold member, and
/// it is swept FIRST on the next rotation.
///
/// The peer relationship is what makes retirement safe: a target only stays
/// cold while nobody says otherwise.
#[tokio::test]
async fn notification_re_arms_a_cold_member_to_the_front() {
    let (pool, reader, projector) = quiet_member_fixture(&["target-a", "target-b", "target-cold"]);
    let scheduler = projector.scheduler();

    for _ in 0..COLD_AFTER_CLEAN_SWEEPS {
        projector.tick().await.expect("cooling tick");
    }
    assert!(scheduler
        .heat(sub_db::KIND_CONTENT_TARGET, "target-cold")
        .is_cold());
    let swept_before = reader.visited().len();

    {
        let mut conn = pool.get().unwrap();
        let admitted = admit_notified_signal_with(
            &mut conn,
            Some(&act_ref("corr-notified", "target-cold")),
            Some(DNA),
            Some(&scheduler),
        )
        .expect("notification admitted");
        assert!(admitted);
    }
    assert_eq!(
        scheduler.heat(sub_db::KIND_CONTENT_TARGET, "target-cold"),
        MemberHeat::Hot,
        "a notification naming the member re-arms it"
    );

    projector.tick().await.expect("post-notification tick");
    let mut swept_after = reader.visited();
    let swept_after: Vec<String> = swept_after.split_off(swept_before);
    assert_eq!(
        swept_after.first().map(String::as_str),
        Some("target-cold"),
        "the re-armed member leads the next rotation; swept: {swept_after:?}"
    );
}

/// (3) A new act reference re-arms — the second path the atom names.
///
/// A correction action that was discovered, never accepted and then went quiet
/// is retired; discovering the correction again puts it straight back at the
/// front rather than behind every member the key order precedes it.
#[tokio::test]
async fn a_new_act_reference_re_arms_a_cold_member() {
    let (pool, reader, projector) = quiet_member_fixture(&["content-target-1"]);
    let scheduler = projector.scheduler();
    {
        let mut conn = pool.get().unwrap();
        sub_db::add_member(
            &mut conn,
            sub_db::KIND_CORRECTION_ACTION,
            "corr-1",
            DNA,
            sub_db::SOURCE_DISCOVERED,
            "2026-09-06T00:00:00Z",
        )
        .unwrap();
    }
    for _ in 0..COLD_AFTER_CLEAN_SWEEPS {
        projector.tick().await.expect("cooling tick");
    }
    assert!(scheduler
        .heat(sub_db::KIND_CORRECTION_ACTION, "corr-1")
        .is_cold());

    // The content target now carries the correction. Its own re-arm is the
    // notification path (already covered); what this test is about is what that
    // sweep then does to the RETIRED correction-action member.
    scheduler.rearm(sub_db::KIND_CONTENT_TARGET, "content-target-1");
    {
        let mut s = reader.state.lock().unwrap();
        s.refs
            .insert("content-target-1".into(), vec![referenced("corr-1")]);
        s.records.insert(
            "corr-1".into(),
            record(
                "corr-1",
                2,
                100,
                correction_entry("content-target-1", "ev-1", "debit-soft"),
            ),
        );
    }
    let report = projector.tick().await.expect("discovery tick");
    assert_eq!(report.groups_applied, 1, "report: {report:?}");
    assert_eq!(
        scheduler.heat(sub_db::KIND_CORRECTION_ACTION, "corr-1"),
        MemberHeat::Hot,
        "a new act reference for a retired member re-arms it"
    );
}

/// (4) The race: a notification for a member ALREADY SWEPT this round is not
/// lost, in both of its shapes.
///
/// Shape one — the notification lands after the visit finished: the re-arm
/// simply wins, and the member is Hot for the next round.
///
/// Shape two — the notification lands WHILE the member is being enumerated: the
/// cooling that follows must not overwrite a re-arm it could not have seen.
/// That is what the visit token is for; without it the sweep would silently
/// swallow the one signal that makes retirement safe.
#[tokio::test]
async fn a_notification_racing_a_sweep_is_not_lost() {
    let (pool, _reader, projector) = quiet_member_fixture(&["target-race"]);
    let scheduler = projector.scheduler();
    let member = (sub_db::KIND_CONTENT_TARGET, "target-race");

    // Two clean sweeps: one more retires it.
    projector.tick().await.expect("tick 1");
    projector.tick().await.expect("tick 2");
    assert_eq!(scheduler.heat(member.0, member.1), MemberHeat::Warm(2));

    // Shape one: swept (going Cold), then notified.
    projector.tick().await.expect("tick 3");
    assert!(scheduler.heat(member.0, member.1).is_cold());
    {
        let mut conn = pool.get().unwrap();
        admit_notified_signal_with(
            &mut conn,
            Some(&act_ref("corr-race", "target-race")),
            Some(DNA),
            Some(&scheduler),
        )
        .unwrap();
    }
    assert_eq!(
        scheduler.heat(member.0, member.1),
        MemberHeat::Hot,
        "a notification after the sweep re-arms rather than being swallowed"
    );

    // Shape two: the token is taken, the notification lands, THEN the sweep
    // reports its clean visit. The clean visit must be discarded.
    let token = scheduler.begin_visit(member.0, member.1);
    scheduler.rearm(member.0, member.1);
    scheduler.record_sweep(member.0, member.1, token, false);
    assert_eq!(
        scheduler.heat(member.0, member.1),
        MemberHeat::Hot,
        "a sweep may not cool a member that was re-armed under it"
    );
}

/// (5) Two Hot members sort by last-new-act descending — the atom's cure (b),
/// "hot targets rotate faster".
#[test]
fn hot_members_sort_by_last_new_act_descending() {
    let scheduler = SweepScheduler::new();
    let rows: Vec<sub_db::SubscriptionRow> = ["target-a", "target-b", "target-c"]
        .iter()
        .map(|k| sub_db::SubscriptionRow {
            member_kind: sub_db::KIND_CONTENT_TARGET.to_string(),
            member_key: (*k).to_string(),
            origin_dna_hash: DNA.to_string(),
            source: sub_db::SOURCE_STEWARD.to_string(),
            added_at: "2026-09-06T00:00:00Z".to_string(),
            last_visited_at: Some("2026-09-06T00:00:00Z".to_string()),
            visit_count: 1,
        })
        .collect();

    // Re-armed in an order that disagrees with the key order, so a passing
    // assertion cannot be an accident of the alphabet.
    scheduler.rearm(sub_db::KIND_CONTENT_TARGET, "target-a");
    scheduler.rearm(sub_db::KIND_CONTENT_TARGET, "target-c");
    scheduler.rearm(sub_db::KIND_CONTENT_TARGET, "target-b");

    let ordered: Vec<String> = scheduler
        .schedule(rows.clone(), 8)
        .into_iter()
        .map(|r| r.member_key)
        .collect();
    assert_eq!(ordered, vec!["target-b", "target-c", "target-a"]);

    // And Hot outranks Warm regardless of arm stamp.
    let token = scheduler.begin_visit(sub_db::KIND_CONTENT_TARGET, "target-b");
    scheduler.record_sweep(sub_db::KIND_CONTENT_TARGET, "target-b", token, false);
    assert_eq!(
        scheduler.heat(sub_db::KIND_CONTENT_TARGET, "target-b"),
        MemberHeat::Warm(1)
    );
    let ordered: Vec<String> = scheduler
        .schedule(rows, 8)
        .into_iter()
        .map(|r| r.member_key)
        .collect();
    assert_eq!(ordered, vec!["target-c", "target-a", "target-b"]);
}

/// (6) `publish_generation` still fires on a clean sweep once members have gone
/// Cold.
///
/// The invariant a Cold member rests on: retirement REQUIRES K completed visits
/// that each found nothing open, so a Cold member has a non-null
/// `last_visited_at` and no pending application member — it can never be the
/// reason a generation stays unpublished. Cold counts as clean by construction.
#[tokio::test]
async fn publication_still_fires_when_members_have_retired() {
    let (pool, _reader, projector) = quiet_member_fixture(&["target-p", "target-q"]);
    let scheduler = projector.scheduler();

    for _ in 0..COLD_AFTER_CLEAN_SWEEPS {
        projector.tick().await.expect("cooling tick");
    }
    assert_eq!(scheduler.cold_count(), 2);

    let mut conn = pool.get().unwrap();
    assert!(
        gen_db::published_generation(&mut conn, &key(0xE0))
            .unwrap()
            .is_some(),
        "a sweep that found nothing unvisited and nothing pending publishes, and Cold members \
         do not hold it back"
    );
    let unvisited: i64 = {
        use crate::db::diesel_schema::feedback_subscriptions::dsl as t;
        t::feedback_subscriptions
            .filter(t::last_visited_at.is_null())
            .count()
            .get_result(&mut conn)
            .unwrap()
    };
    assert_eq!(unvisited, 0, "every Cold member has been visited K times");
}
