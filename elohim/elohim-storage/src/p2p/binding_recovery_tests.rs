use super::*;
use crate::p2p::binding_mint::{agent_half_preimage, assemble_core, seal_proof};
use crate::reconcile::controller::ReconcileController;
use crate::reconcile::pubkey_timeline::PubkeyTimelineCache;
use crate::reconcile::signal_stream::InMemoryDnaSignalStream;
use diesel::r2d2::{ConnectionManager, Pool};
use ed25519_dalek::{Signer, SigningKey};

struct Original {
    agent: String,
    peer: String,
    record: Record,
    now: DateTime<Utc>,
}

impl Original {
    fn row(&self) -> OriginalBinding {
        OriginalBinding {
            action_hash: self.anchor(),
            record: Some(rmp_serde::to_vec_named(&Some(&self.record)).unwrap()),
        }
    }

    fn anchor(&self) -> String {
        self.record.action_address().to_string()
    }
}

fn original(expired: bool, foreign_author: bool, superseded: bool) -> Original {
    let now = Utc::now();
    let key = SigningKey::from_bytes(&[21; 32]);
    let agent = AgentPubKey::from_raw_32(key.verifying_key().as_bytes().to_vec()).to_string();
    let transport = libp2p::identity::Keypair::generate_ed25519();
    let peer = transport.public().to_peer_id().to_base58();
    let from = if expired {
        now - chrono::Duration::days(2)
    } else {
        now
    };
    let core = assemble_core(&agent, &peer, from, 1);
    let sealed = seal_proof(
        &core,
        &transport,
        &key.sign(&agent_half_preimage(&core)).to_bytes(),
        key.verifying_key().as_bytes(),
    )
    .unwrap();
    let payload = serde_json::json!({
        "peer_id": peer, "agent_cid": agent,
        "valid_from": core.valid_from.parse::<DateTime<Utc>>().unwrap().timestamp_micros(),
        "valid_until": core.valid_until.as_ref().unwrap().parse::<DateTime<Utc>>().unwrap().timestamp_micros(),
        "device_archetype": "node", "signature": sealed.envelope.into_bytes(),
        "superseded_by": superseded.then(|| ActionHash::from_raw_32(vec![7; 32]).to_string())
    });
    let entry = Entry::App(
        AppEntryBytes::try_from(SerializedBytes::from(UnsafeBytes::from(
            rmp_serde::to_vec_named(&payload).unwrap(),
        )))
        .unwrap(),
    );
    let author_key = if foreign_author {
        SigningKey::from_bytes(&[22; 32])
    } else {
        key
    };
    let action = Action {
        header: ActionHeader {
            author: AgentPubKey::from_raw_32(author_key.verifying_key().as_bytes().to_vec()),
            timestamp: Timestamp::from_micros(from.timestamp_micros()),
            action_seq: 1,
            prev_action: Some(ActionHash::from_raw_32(vec![8; 32])),
        },
        data: ActionData::Create(CreateData {
            entry_type: EntryType::App(AppEntryDef::new(
                EntryDefIndex(20),
                ZomeIndex(0),
                EntryVisibility::Public,
            )),
            entry_hash: EntryHash::with_data_sync(&entry),
        }),
    };
    let bytes = SerializedBytes::try_from(action.clone()).unwrap();
    let signature = Signature(author_key.sign(bytes.bytes()).to_bytes());
    Original {
        agent,
        peer,
        now,
        record: Record::new(
            SignedActionHashed::new_unchecked(action, signature),
            RecordEntry::Present(entry),
        ),
    }
}

fn pool() -> DbPool {
    let url = format!(
        "file:binding_recovery_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<diesel::SqliteConnection>::new(url))
        .unwrap();
    crate::db::run_migrations(&pool).unwrap();
    pool
}

fn recover(
    original: &Original,
    pool: &DbPool,
    projection: &OwnBindingProjection,
) -> Result<(), StorageError> {
    recover_records(
        vec![original.row()],
        pool,
        &original.peer,
        &original.agent,
        projection,
        original.now,
    )
}

async fn project(pool: &DbPool, signal: DnaSignal) -> Vec<crate::p2p::P2PCommand> {
    let stream = InMemoryDnaSignalStream::with_signals(vec![signal]);
    let (swarm_tx, mut swarm_rx) = mpsc::channel(4);
    let mut controller = ReconcileController::new_with_storage(
        stream,
        Arc::new(pool.clone()),
        Arc::new(tokio::sync::Mutex::new(PubkeyTimelineCache::with_capacity(
            4,
        ))),
    )
    .with_swarm_tx(swarm_tx);
    controller.run_one_pass().await.unwrap();
    let mut published = Vec::new();
    while let Ok(command) = swarm_rx.try_recv() {
        published.push(command);
    }
    published
}

fn supersede(pool: &DbPool, original: &Original) {
    use crate::db::diesel_schema::peer_identity_bindings::dsl;
    diesel::update(dsl::peer_identity_bindings.filter(dsl::peer_id.eq(&original.peer)))
        .set(dsl::superseded_by.eq(Some("preserved-successor")))
        .execute(&mut pool.get().unwrap())
        .unwrap();
}

#[tokio::test]
async fn signed_original_is_acknowledged_only_after_actual_controller_projection() {
    let original = original(false, false, false);
    let pool = pool();
    let (tx, mut rx) = mpsc::channel(1);
    let projection = OwnBindingProjection::new(&tx);
    assert!(
        !exact_acknowledged(&pool, &original.peer, &original.agent, &original.anchor()).unwrap()
    );
    assert!(
        recover(&original, &pool, &projection).is_err(),
        "queued is not acknowledged"
    );
    let commands = project(&pool, rx.try_recv().unwrap()).await;
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        crate::p2p::P2PCommand::PublishIdentityBinding(payload) => {
            assert_eq!(payload.binding_action_hash, original.anchor());
            assert_eq!(payload.agent_cid, original.agent);
            assert_eq!(payload.peer_id, original.peer);
        }
        _ => panic!("original recovery must publish the original binding"),
    }
    let row = existing_row(&pool, &original.peer, &original.anchor())
        .unwrap()
        .unwrap();
    assert!(row.is_cross_signed());
    assert_eq!(row.dht_anchor_hash, original.anchor());
    assert!(
        exact_acknowledged(&pool, &original.peer, &original.agent, &original.anchor()).unwrap()
    );
    assert!(
        !exact_acknowledged(&pool, &original.peer, "foreign-agent", &original.anchor()).unwrap()
    );
    assert!(
        !exact_acknowledged(&pool, &original.peer, &original.agent, "different-anchor").unwrap()
    );
    assert!(recover(&original, &pool, &projection).is_ok());
    assert!(
        rx.try_recv().is_err(),
        "acknowledged original must not be republished"
    );
}

#[tokio::test]
async fn supersession_before_enqueue_or_after_enqueue_is_never_cleared() {
    for queued_first in [false, true] {
        let original = original(false, false, false);
        let pool = pool();
        let (tx, mut rx) = mpsc::channel(1);
        let projection = OwnBindingProjection::new(&tx);
        assert!(recover(&original, &pool, &projection).is_err());
        let queued = rx.try_recv().unwrap();
        // Persist the same original via the actual controller, then race its
        // later authoritative supersession against the previously queued replay.
        assert!(recover(&original, &pool, &projection).is_err());
        assert_eq!(project(&pool, rx.try_recv().unwrap()).await.len(), 1);
        supersede(&pool, &original);
        if queued_first {
            assert!(
                project(&pool, queued).await.is_empty(),
                "superseded original must not be re-gossiped"
            );
        }
        assert!(recover(&original, &pool, &projection).is_err());
        assert!(rx.try_recv().is_err());
        assert_eq!(
            existing_row(&pool, &original.peer, &original.anchor())
                .unwrap()
                .unwrap()
                .superseded_by
                .as_deref(),
            Some("preserved-successor")
        );
        assert!(
            !exact_acknowledged(&pool, &original.peer, &original.agent, &original.anchor())
                .unwrap()
        );
    }
}

#[test]
fn closed_or_full_signal_queue_defers_instead_of_claiming_absence() {
    for mode in ["weak-closed", "receiver-closed", "full"] {
        let original = original(false, false, false);
        let pool = pool();
        let (tx, mut rx) = mpsc::channel(1);
        let projection = OwnBindingProjection::new(&tx);
        if mode == "full" {
            assert!(recover(&original, &pool, &projection).is_err());
        }
        if mode == "receiver-closed" {
            rx.close();
        }
        if mode == "weak-closed" {
            drop(tx);
        }
        assert!(recover(&original, &pool, &projection).is_err());
        assert!(
            !exact_acknowledged(&pool, &original.peer, &original.agent, &original.anchor())
                .unwrap()
        );
    }
}

#[test]
fn malformed_foreign_tampered_and_superseded_originals_never_enqueue() {
    for kind in [
        "malformed",
        "foreign",
        "tampered",
        "superseded",
        "wrong-peer",
    ] {
        let original = original(false, kind == "foreign", kind == "superseded");
        let pool = pool();
        let (tx, mut rx) = mpsc::channel(1);
        let projection = OwnBindingProjection::new(&tx);
        let mut row = original.row();
        if kind == "malformed" {
            row.record = Some(vec![0xc1]);
        }
        if kind == "tampered" {
            let mut record = original.record.clone();
            record.signed_action =
                SignedActionHashed::new_unchecked(record.action().clone(), Signature([0; 64]));
            row.record = Some(rmp_serde::to_vec_named(&Some(record)).unwrap());
        }
        let peer = if kind == "wrong-peer" {
            "different-peer"
        } else {
            &original.peer
        };
        assert!(
            recover_records(
                vec![row],
                &pool,
                peer,
                &original.agent,
                &projection,
                original.now
            )
            .is_err(),
            "{kind}"
        );
        assert!(rx.try_recv().is_err(), "{kind}");
    }
}

#[test]
fn expired_authentic_original_does_not_recover_as_current() {
    let original = original(true, false, false);
    let pool = pool();
    let (tx, mut rx) = mpsc::channel(1);
    assert!(recover(&original, &pool, &OwnBindingProjection::new(&tx)).is_ok());
    assert!(rx.try_recv().is_err());
    assert!(
        !exact_acknowledged(&pool, &original.peer, &original.agent, &original.anchor()).unwrap()
    );
}

#[test]
fn recovery_capability_cannot_be_serialized_or_claimed_by_external_signal() {
    let original = original(false, false, false);
    let pool = pool();
    let (tx, mut rx) = mpsc::channel(1);
    assert!(recover(&original, &pool, &OwnBindingProjection::new(&tx)).is_err());
    let recovery = rx.try_recv().unwrap();
    assert!(serde_json::to_value(&recovery).is_err());
    assert!(rmp_serde::to_vec_named(&recovery).is_err());
    let DnaSignal::AgentPeerBindingRecovery(payload) = recovery else {
        panic!("authenticated recovery must use its internal capability");
    };
    let ordinary = DnaSignal::AgentPeerBinding(payload);
    let mut wire = serde_json::to_value(&ordinary).unwrap();
    assert!(serde_json::from_value::<DnaSignal>(wire.clone()).is_ok());
    wire["signalType"] = "agentPeerBindingRecovery".into();
    assert!(serde_json::from_value::<DnaSignal>(wire.clone()).is_err());
    assert!(rmp_serde::from_slice::<DnaSignal>(&rmp_serde::to_vec_named(&wire).unwrap()).is_err());
}

#[test]
fn reconnect_refreshes_existing_weak_capability_without_retaining_old_stream() {
    let original = original(false, false, false);
    let pool = pool();
    let (old_tx, mut old_rx) = mpsc::channel(1);
    let projection = OwnBindingProjection::new(&old_tx);
    let retained_by_mint = projection.clone();
    drop(old_tx);
    assert!(recover(&original, &pool, &retained_by_mint).is_err());
    assert!(matches!(
        old_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Disconnected)
    ));

    let (new_tx, mut new_rx) = mpsc::channel(1);
    let connected = OwnBindingProjection::new(&new_tx);
    projection.use_connected_stream(&connected).unwrap();
    assert!(recover(&original, &pool, &retained_by_mint).is_err());
    let DnaSignal::AgentPeerBindingRecovery(signal) = new_rx.try_recv().unwrap() else {
        panic!("original must enter the newly connected recovery stream");
    };
    assert_eq!(signal.binding_action_hash, original.anchor());
    assert!(matches!(
        old_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Disconnected)
    ));
    drop(new_tx);
    assert!(matches!(
        new_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Disconnected)
    ));
}
