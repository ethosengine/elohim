use super::*;
use crate::db::{rea_commitment_lifecycle as lifecycle, rea_commitments, AppContext};
use crate::services::rea_commitment_projection as projection;
use diesel::{Connection, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use ed25519_dalek::{Signer, SigningKey};

fn commitment(state: &str) -> Commitment {
    let provider = AgentPubKey::from_raw_32(
        SigningKey::from_bytes(&[1; 32])
            .verifying_key()
            .as_bytes()
            .to_vec(),
    )
    .to_string();
    serde_json::from_value(serde_json::json!({
        "id":"own-provide", "action":"provide", "provider":provider, "receiver":"",
        "resource_classified_as_json":"[\"content:commons\"]", "in_scope_of_json":"[]",
        "finished":state == "cancelled", "state":state, "metadata_json":"{}",
        "created_at":"2026-09-09T00:00:00Z", "updated_at":state
    }))
    .unwrap()
}

fn project_epr_commitment(reach: &str) -> Commitment {
    let mut value = commitment("active");
    value.action = "project-epr".into();
    value.in_scope_of_json = r#"["doorway:alpha-elohim-host|epr:own-provide"]"#.into();
    value.metadata_json = serde_json::json!({
        "urlPath": "/garden",
        "mode": "cached",
        "reach": reach,
        "gateHints": if reach == "commons" {
            serde_json::json!([])
        } else {
            serde_json::json!([{"eprRef":"collective:dowell", "relation":"membershipPrerequisite"}])
        }
    })
    .to_string();
    value
}

fn record(c: Commitment, seq: u32, parent: Option<&Record>, seed: u8) -> Record {
    let key = SigningKey::from_bytes(&[seed; 32]);
    let entry = Entry::App(
        AppEntryBytes::try_from(SerializedBytes::from(UnsafeBytes::from(
            rmp_serde::to_vec_named(&c).unwrap(),
        )))
        .unwrap(),
    );
    let entry_hash = EntryHash::with_data_sync(&entry);
    let entry_type = EntryType::App(AppEntryDef::new(
        EntryDefIndex(COMMITMENT_ENTRY),
        ZomeIndex(0),
        EntryVisibility::Public,
    ));
    let data = match parent {
        Some(p) => ActionData::Update(UpdateData {
            original_action_address: p.action_address().clone(),
            original_entry_address: p.action().entry_hash().unwrap().clone(),
            entry_type,
            entry_hash,
        }),
        None => ActionData::Create(CreateData {
            entry_type,
            entry_hash,
        }),
    };
    let action = Action {
        header: ActionHeader {
            author: AgentPubKey::from_raw_32(key.verifying_key().as_bytes().to_vec()),
            timestamp: Timestamp::from_micros(i64::from(seq) * 1000),
            action_seq: seq,
            prev_action: Some(
                parent
                    .map(|p| p.action_address().clone())
                    .unwrap_or_else(|| ActionHash::from_raw_32(vec![9; 32])),
            ),
        },
        data,
    };
    let bytes = SerializedBytes::try_from(action.clone()).unwrap();
    let signature = Signature(key.sign(bytes.bytes()).to_bytes());
    Record::new(
        SignedActionHashed::new_unchecked(action, signature),
        RecordEntry::Present(entry),
    )
}

fn records(items: &[&Record]) -> HashMap<String, CarriedRecordWire> {
    items
        .iter()
        .map(|r| {
            let hash = r.action_address().to_string();
            (
                hash.clone(),
                CarriedRecordWire {
                    action_hash: hash,
                    record: rmp_serde::to_vec_named(*r).unwrap(),
                },
            )
        })
        .collect()
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

fn database() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    conn
}

async fn apply(
    conn: &mut SqliteConnection,
    map: &HashMap<String, CarriedRecordWire>,
    r: &Record,
) -> Result<lifecycle::ApplyOutcome, StorageError> {
    let ctx = AppContext::default_lamad();
    let snapshot = lifecycle::snapshot(conn, &ctx, "own-provide")?;
    let prepared = projection::prepare_with_reader(
        "own-provide",
        &r.action_address().to_string(),
        snapshot,
        |hash| std::future::ready(Ok(map.get(&hash).cloned())),
    )
    .await?;
    match prepared {
        Some(p) => projection::apply_prepared(conn, &ctx, p),
        None => Ok(lifecycle::ApplyOutcome::Unchanged),
    }
}

#[tokio::test]
async fn active_then_cancelled_refuses_delayed_active_and_created() {
    let created = record(commitment("created"), 1, None, 1);
    let active = record(commitment("active"), 2, Some(&created), 1);
    let cancelled = record(commitment("cancelled"), 3, Some(&created), 1);
    let map = records(&[&created, &active, &cancelled]);
    let mut conn = database();
    assert_eq!(
        apply(&mut conn, &map, &active).await.unwrap(),
        lifecycle::ApplyOutcome::Advanced
    );
    assert_eq!(
        apply(&mut conn, &map, &cancelled).await.unwrap(),
        lifecycle::ApplyOutcome::Advanced
    );
    for stale in [&active, &created, &cancelled] {
        assert_eq!(
            apply(&mut conn, &map, stale).await.unwrap(),
            lifecycle::ApplyOutcome::Unchanged
        );
    }
    let row =
        rea_commitments::get_commitment(&mut conn, &AppContext::default_lamad(), "own-provide")
            .unwrap()
            .unwrap();
    assert_eq!(row.state, "cancelled");
    assert_eq!(row.finished, 1);
    assert_eq!(
        row.dht_anchor_hash.as_deref(),
        Some(cancelled.action_address().to_string().as_str())
    );
}

#[tokio::test]
async fn delayed_cancellation_does_not_reverse_newer_active_observation() {
    let created = record(commitment("created"), 1, None, 1);
    let cancelled = record(commitment("cancelled"), 2, Some(&created), 1);
    let newer = record(commitment("active"), 3, Some(&created), 1);
    let map = records(&[&created, &cancelled, &newer]);
    let mut conn = database();
    apply(&mut conn, &map, &newer).await.unwrap();
    assert_eq!(
        apply(&mut conn, &map, &cancelled).await.unwrap(),
        lifecycle::ApplyOutcome::Unchanged
    );
    let row =
        rea_commitments::get_commitment(&mut conn, &AppContext::default_lamad(), "own-provide")
            .unwrap()
            .unwrap();
    assert_eq!(row.state, "active");
    assert_eq!(row.finished, 0);
}

#[tokio::test]
async fn signed_project_epr_terms_advance_and_stale_delivery_cannot_roll_them_back() {
    let created = record(project_epr_commitment("commons"), 1, None, 1);
    let narrowed = record(project_epr_commitment("local"), 2, Some(&created), 1);
    let map = records(&[&created, &narrowed]);
    let mut conn = database();
    assert_eq!(
        apply(&mut conn, &map, &created).await.unwrap(),
        lifecycle::ApplyOutcome::Advanced
    );
    assert_eq!(
        apply(&mut conn, &map, &narrowed).await.unwrap(),
        lifecycle::ApplyOutcome::Advanced
    );
    assert_eq!(
        apply(&mut conn, &map, &created).await.unwrap(),
        lifecycle::ApplyOutcome::Unchanged
    );
    let row =
        rea_commitments::get_commitment(&mut conn, &AppContext::default_lamad(), "own-provide")
            .unwrap()
            .unwrap();
    let metadata: serde_json::Value =
        serde_json::from_str(row.metadata_json.as_deref().unwrap()).unwrap();
    assert_eq!(metadata["reach"], "local");
}

#[tokio::test]
async fn signed_project_epr_update_cannot_replace_immutable_terms() {
    let created = record(project_epr_commitment("commons"), 1, None, 1);
    let mut changed = project_epr_commitment("local");
    let mut metadata: serde_json::Value = serde_json::from_str(&changed.metadata_json).unwrap();
    metadata["urlPath"] = serde_json::json!("/other");
    changed.metadata_json = metadata.to_string();
    let changed = record(changed, 2, Some(&created), 1);
    let map = records(&[&created, &changed]);
    let mut conn = database();
    apply(&mut conn, &map, &created).await.unwrap();
    assert!(apply(&mut conn, &map, &changed).await.is_err());
    let row =
        rea_commitments::get_commitment(&mut conn, &AppContext::default_lamad(), "own-provide")
            .unwrap()
            .unwrap();
    let metadata: serde_json::Value =
        serde_json::from_str(row.metadata_json.as_deref().unwrap()).unwrap();
    assert_eq!(metadata["urlPath"], "/garden");
    assert_eq!(metadata["reach"], "commons");
}

#[tokio::test]
async fn missing_stored_authority_cannot_replace_projection() {
    let created = record(commitment("created"), 1, None, 1);
    let active = record(commitment("active"), 2, Some(&created), 1);
    let cancelled = record(commitment("cancelled"), 3, Some(&created), 1);
    let mut map = records(&[&created, &active, &cancelled]);
    let mut conn = database();
    apply(&mut conn, &map, &active).await.unwrap();
    map.remove(&active.action_address().to_string());
    assert!(apply(&mut conn, &map, &cancelled).await.is_err());
    assert_eq!(
        rea_commitments::get_commitment(&mut conn, &AppContext::default_lamad(), "own-provide")
            .unwrap()
            .unwrap()
            .state,
        "active"
    );
}

#[tokio::test]
async fn cas_refuses_same_anchor_local_state_or_metadata_race() {
    let created = record(commitment("created"), 1, None, 1);
    let active = record(commitment("active"), 2, Some(&created), 1);
    let map = records(&[&created, &active]);
    let ctx = AppContext::default_lamad();
    for metadata_only in [false, true] {
        let mut conn = database();
        apply(&mut conn, &map, &created).await.unwrap();
        let expected = lifecycle::snapshot(&mut conn, &ctx, "own-provide").unwrap();
        let prepared = projection::prepare_with_reader(
            "own-provide",
            &active.action_address().to_string(),
            expected,
            |hash| std::future::ready(Ok(map.get(&hash).cloned())),
        )
        .await
        .unwrap()
        .unwrap();
        use crate::db::diesel_schema::rea_commitments::dsl as c;
        use diesel::prelude::*;
        if metadata_only {
            diesel::update(c::rea_commitments)
                .set(c::metadata_json.eq("{\"local\":true}"))
                .execute(&mut conn)
                .unwrap();
        } else {
            diesel::update(c::rea_commitments)
                .set(c::state.eq("graduated-locally"))
                .execute(&mut conn)
                .unwrap();
        }
        assert_eq!(
            projection::apply_prepared(&mut conn, &ctx, prepared).unwrap(),
            lifecycle::ApplyOutcome::Deferred
        );
        let row = rea_commitments::get_commitment(&mut conn, &ctx, "own-provide")
            .unwrap()
            .unwrap();
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some(created.action_address().to_string().as_str())
        );
        if !metadata_only {
            assert_eq!(row.state, "graduated-locally");
        }
    }
}

#[tokio::test]
async fn same_anchor_replay_preserves_local_graduation() {
    let created = record(commitment("created"), 1, None, 1);
    let map = records(&[&created]);
    let mut conn = database();
    apply(&mut conn, &map, &created).await.unwrap();
    use crate::db::diesel_schema::rea_commitments::dsl as c;
    use diesel::prelude::*;
    diesel::update(c::rea_commitments)
        .set(c::state.eq("active"))
        .execute(&mut conn)
        .unwrap();
    assert_eq!(
        apply(&mut conn, &map, &created).await.unwrap(),
        lifecycle::ApplyOutcome::Unchanged
    );
    assert_eq!(
        rea_commitments::get_commitment(&mut conn, &AppContext::default_lamad(), "own-provide")
            .unwrap()
            .unwrap()
            .state,
        "active"
    );
}

#[tokio::test]
async fn foreign_author_root_and_changed_undertaking_are_refused() {
    let created = record(commitment("created"), 1, None, 1);
    let active = record(commitment("active"), 2, Some(&created), 1);
    let foreign = record(commitment("cancelled"), 3, Some(&created), 2);
    let another_root = record(commitment("cancelled"), 4, None, 1);
    let mut changed = commitment("cancelled");
    changed.provider = "another-party".into();
    let changed = record(changed, 3, Some(&created), 1);
    let map = records(&[&created, &active, &foreign, &another_root, &changed]);
    for candidate in [&foreign, &another_root, &changed] {
        assert!(observe(
            "own-provide",
            &candidate.action_address().to_string(),
            Some(&active.action_address().to_string()),
            |hash| std::future::ready(Ok(map.get(&hash).cloned()))
        )
        .await
        .is_err());
    }
}

#[tokio::test]
async fn rejects_tampered_record_before_projection() {
    let created = record(commitment("created"), 1, None, 1);
    let mut map = records(&[&created]);
    let mut wire = map.remove(&created.action_address().to_string()).unwrap();
    let mut bad = created.clone();
    bad.signed_action.signature = Signature([0; 64]);
    wire.record = rmp_serde::to_vec_named(&bad).unwrap();
    assert!(
        observe("own-provide", &wire.action_hash.clone(), None, |_| {
            std::future::ready(Ok(Some(wire.clone())))
        })
        .await
        .is_err()
    );
}

#[tokio::test]
async fn shared_read_budget_stops_before_sixty_fifth_record() {
    let mut chain = vec![record(commitment("created"), 1, None, 1)];
    for seq in 2..=65 {
        chain.push(record(commitment("active"), seq, chain.last(), 1));
    }
    let map = records(&chain.iter().collect::<Vec<_>>());
    let mut calls = 0;
    let result = observe(
        "own-provide",
        &chain.last().unwrap().action_address().to_string(),
        None,
        |hash| {
            calls += 1;
            std::future::ready(Ok(map.get(&hash).cloned()))
        },
    )
    .await;
    assert!(result.is_err());
    assert_eq!(calls, 64);
}

#[test]
fn commitment_entry_pin_matches_current_integrity_declaration() {
    let source =
        include_str!("../../../holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs");
    let entries = source
        .split("pub enum EntryTypes {")
        .nth(1)
        .unwrap()
        .split("\n}")
        .next()
        .unwrap();
    let names: Vec<_> = entries
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix("    ")?;
            let (name, _) = line.split_once('(')?;
            (!name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
                .then_some(name)
        })
        .collect();
    assert_eq!(names[usize::from(COMMITMENT_ENTRY)], "Commitment");
}

#[tokio::test]
async fn missing_incoming_record_never_inserts_a_projection() {
    let created = record(commitment("created"), 1, None, 1);
    let mut conn = database();
    assert!(apply(&mut conn, &HashMap::new(), &created).await.is_err());
    assert!(rea_commitments::get_commitment(
        &mut conn,
        &AppContext::default_lamad(),
        "own-provide"
    )
    .unwrap()
    .is_none());
}

#[tokio::test]
async fn failed_lifecycle_write_rolls_back_anchor_and_fields_together() {
    use diesel::connection::SimpleConnection;
    let created = record(commitment("created"), 1, None, 1);
    let active = record(commitment("active"), 2, Some(&created), 1);
    let map = records(&[&created, &active]);
    let mut conn = database();
    apply(&mut conn, &map, &created).await.unwrap();
    conn.batch_execute("CREATE TRIGGER refuse_lifecycle BEFORE UPDATE OF state ON rea_commitments BEGIN SELECT RAISE(ABORT, 'injected lifecycle write failure'); END;").unwrap();
    assert!(apply(&mut conn, &map, &active).await.is_err());
    let row =
        rea_commitments::get_commitment(&mut conn, &AppContext::default_lamad(), "own-provide")
            .unwrap()
            .unwrap();
    assert_eq!(row.state, "created");
    assert_eq!(
        row.dht_anchor_hash.as_deref(),
        Some(created.action_address().to_string().as_str())
    );
}

#[tokio::test]
async fn explicit_refresh_recovers_missing_and_universally_stale_projection() {
    let created = record(commitment("created"), 1, None, 1);
    let active = record(commitment("active"), 2, Some(&created), 1);
    let map = records(&[&created, &active]);
    for initially_missing in [true, false] {
        let mut conn = database();
        if !initially_missing {
            apply(&mut conn, &map, &created).await.unwrap();
        }
        let ctx = AppContext::default_lamad();
        let expected = lifecycle::snapshot(&mut conn, &ctx, "own-provide").unwrap();
        let observed = shefa_types::ReaCommitmentOutput {
            action_hash: active.action_address().clone(),
            entry_hash: active.action().entry_hash().unwrap().clone(),
            commitment: commitment("active"),
        };
        let prepared = projection::prepare_refresh(
            "own-provide",
            expected,
            std::future::ready(Ok(Some(observed))),
            |hash| std::future::ready(Ok(map.get(&hash).cloned())),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            projection::apply_prepared(&mut conn, &ctx, prepared).unwrap(),
            lifecycle::ApplyOutcome::Advanced
        );
        let row = rea_commitments::get_commitment(&mut conn, &ctx, "own-provide")
            .unwrap()
            .unwrap();
        assert_eq!(row.state, "active");
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some(active.action_address().to_string().as_str())
        );
    }
}

#[tokio::test]
async fn explicit_refresh_missing_or_refused_authority_never_falls_back() {
    let created = record(commitment("created"), 1, None, 1);
    let map = records(&[&created]);
    let mut conn = database();
    apply(&mut conn, &map, &created).await.unwrap();
    let ctx = AppContext::default_lamad();
    for lookup in [Ok(None), Err(StorageError::Conductor("refused".into()))] {
        let expected = lifecycle::snapshot(&mut conn, &ctx, "own-provide").unwrap();
        let result = projection::prepare_refresh(
            "own-provide",
            expected,
            std::future::ready(lookup),
            |_| std::future::ready(Err(StorageError::Internal("must not read".into()))),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            rea_commitments::get_commitment(&mut conn, &ctx, "own-provide")
                .unwrap()
                .unwrap()
                .state,
            "created"
        );
    }
}

#[tokio::test]
async fn explicit_refresh_wrong_id_refuses_before_record_read() {
    let created = record(commitment("created"), 1, None, 1);
    let mut wrong = commitment("active");
    wrong.id = "another-undertaking".into();
    let observed = shefa_types::ReaCommitmentOutput {
        action_hash: created.action_address().clone(),
        entry_hash: created.action().entry_hash().unwrap().clone(),
        commitment: wrong,
    };
    let result = projection::prepare_refresh(
        "own-provide",
        None,
        std::future::ready(Ok(Some(observed))),
        |_| {
            panic!("wrong undertaking must not invoke exact-record reader");
            #[allow(unreachable_code)]
            std::future::ready(Ok(None))
        },
    )
    .await;
    assert!(matches!(result, Err(StorageError::InvalidInput(_))));
}
