//! K12 packet 2: the gate decides each artifact object on the selected record's own reach claim
//! (`env:reach=<level>` on this device's own custody Produce events), through
//! `eprfs_peer::serve_decision`. Two devices in process, as the rest of this suite.
//!
//! No writer records a reach slot yet (packet 3); these tests write the slot themselves with
//! `put_local_blob_with`, exactly where the writer will put it.

use std::time::{SystemTime, UNIX_EPOCH};

use eprfs_peer::{
    artifact_gate_counts, Basis, DeclaredReach, ReachClaim, ServeScope, StewardScope,
    GATE_REACH_SLOT, GATE_ROW_SLOT, LADDER_UNBUILT,
};

use super::*;

/// A method no `serves` row of the shipped policy names: a sysroot floor's.
const FLOOR: &str = "sysroot-pack";

const SELF: &str = "env:reach=self";
const PRIVATE: &str = "env:reach=private";

/// The `berth.artifacts` rows of `genesis/agentic/pool-policy.json` as shipped: `mesh-build`
/// served to the steward's devices and disclosed, nothing else.
fn shipped_policy() -> serde_json::Value {
    let mut policy = serve_policy();
    policy["berth"]["artifacts"]["discloseMethod"] = serde_json::json!([GRANTED]);
    policy
}

fn set_policy(holder: &Holder, policy: &serde_json::Value) {
    *holder.serves.write().unwrap() = Some(ArtifactServeList::from_policy(policy).unwrap());
}

/// Put `bytes` in custody as the output of `descriptor`, its method record held, its custody
/// Produce carrying `slots` after the writer's own.
async fn put_slotted_with<F: FlowStore + ?Sized>(
    store: &dyn EprfsStorage,
    flow: &mut F,
    descriptor: MethodDescriptor,
    bytes: Bytes,
    slots: &[&str],
) -> TierBlobPut {
    put_method(store, &descriptor).await;
    let record = DerivationRecord {
        key: DerivationKey {
            method: descriptor.cid().expect("method cid"),
            input: KeyInput::Raw(BlobCid::compute_raw(descriptor.name.as_bytes())),
        },
        output: BlobCid::compute_bytes(&bytes),
        computed_by: "did:key:zDeviceA".into(),
        basis: DerivationBasis::Computed,
        deterministic: true,
        witness: Witness::Signature {
            sig: "fixture-signature".into(),
        },
    };
    eprfs_peer::put_local_blob_with(
        store,
        flow,
        record,
        bytes,
        &context(),
        slots.iter().map(|slot| slot.to_string()).collect(),
    )
    .await
    .expect("put")
}

impl Holder {
    async fn put_slotted(&mut self, method: &str, bytes: Bytes, slots: &[&str]) -> TierBlobPut {
        put_slotted_with(
            self.payload.as_ref(),
            &mut self.flow,
            descriptor(method),
            bytes,
            slots,
        )
        .await
    }

    fn scope(&self) -> DeviceScope {
        DeviceScope::new(
            Arc::new(ChannelStore::default()),
            Some(self.mount.clone()),
            self.serves.clone(),
            self.admission.clone(),
        )
    }
}

fn manifest_of(put: &TierBlobPut) -> TierRequest {
    TierRequest::Manifest {
        artifact: output(put),
    }
}

fn blob_of(put: &TierBlobPut, cid: &BlobCid) -> TierRequest {
    TierRequest::Blob {
        artifact: output(put),
        cid: cid.clone(),
    }
}

fn record_of(put: &TierBlobPut, cid: Option<&BlobCid>) -> TierRequest {
    TierRequest::Record {
        artifact: output(put),
        manifest: put.manifest_cid.clone(),
        key: None,
        cid: cid.cloned(),
    }
}

fn method_cid(put: &TierBlobPut) -> BlobCid {
    put.manifest.record.key.method.clone()
}

fn input_cid(put: &TierBlobPut) -> BlobCid {
    put.manifest.record.key.input.cid().clone()
}

/// Draw `put` whole as a fetcher does over output-only requests: manifest, then every chunk.
async fn draw(b: &Endpoint, addr: &NodeAddr, put: &TierBlobPut) -> Bytes {
    served_bytes(call(b, addr, manifest_of(put)).await, &put.manifest_cid);
    let mut drawn = Vec::new();
    for chunk in chunks(put) {
        drawn.extend_from_slice(&served_bytes(
            call(b, addr, blob_of(put, &chunk)).await,
            &chunk,
        ));
    }
    Bytes::from(drawn)
}

/// [`draw`] over one open connection: one serve session, closed by its last chunk.
async fn draw_on(connection: &Connection, put: &TierBlobPut) -> Bytes {
    served_bytes(ask(connection, manifest_of(put)).await, &put.manifest_cid);
    let mut drawn = Vec::new();
    for chunk in chunks(put) {
        drawn.extend_from_slice(&served_bytes(
            ask(connection, blob_of(put, &chunk)).await,
            &chunk,
        ));
    }
    Bytes::from(drawn)
}

fn standing(put: &TierBlobPut, label: &str) -> TierAnswer {
    typed(&output(put), ReachReason::OutsideStanding, Some(label))
}

/// What a record-bound request naming `put`'s manifest is answered when that manifest is not
/// held here.
fn no_record(put: &TierBlobPut) -> TierAnswer {
    typed(
        &output(put),
        ReachReason::NoRecord,
        Some(&format!("manifest:{}", put.manifest_cid)),
    )
}

/// An answer's frame with its echoed CID set aside.
fn frame_without_echo(answer: &TierAnswer) -> Vec<u8> {
    let TierAnswer::Unanswered { path, why, .. } = answer.clone() else {
        panic!("a refusal, got {answer:?}");
    };
    serde_ipld_dagcbor::to_vec(&TierAnswer::Unanswered {
        cid: BlobCid::compute(b"echo"),
        path,
        why,
    })
    .expect("encode")
}

fn refusal_reasons(draws: &[ServedDraw]) -> Vec<String> {
    draws
        .iter()
        .filter_map(|draw| match &draw.outcome {
            DrawOutcome::Refused { reason } => Some(reason.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_admitted_sibling_draws_a_self_floor_with_no_row_for_its_method() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    let bytes = artifact_bytes(101, 600 * 1024);
    let floor = a.put_slotted(FLOOR, bytes.clone(), &[SELF]).await;
    assert!(a.reindex().await.is_empty());
    assert_eq!(
        a.mount.claim(&floor.manifest_cid),
        Some(ReachClaim::Declared(DeclaredReach::OwnDevices))
    );
    let shipped = ArtifactServeList::from_policy(&shipped_policy()).unwrap();
    assert!(!shipped.row_grants_steward(FLOOR) && !shipped.narrows(FLOOR));

    let b = client(&h.key_b).await;
    assert_eq!(draw(&b, &a.addr, &floor).await, bytes);
    served_bytes(
        call(&b, &a.addr, record_of(&floor, None)).await,
        &floor.manifest_cid,
    );
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_admitted_sibling_draws_a_self_mesh_build_with_the_shipped_policy() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    let bytes = artifact_bytes(102, 600 * 1024);
    let build = a.put_slotted(GRANTED, bytes.clone(), &[SELF]).await;
    assert!(a.reindex().await.is_empty());
    let b = client(&h.key_b).await;
    assert_eq!(draw(&b, &a.addr, &build).await, bytes);
    for chunk in chunks(&build) {
        served_bytes(
            call(&b, &a.addr, record_of(&build, Some(&chunk))).await,
            &chunk,
        );
    }
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_record_is_indistinguishable_from_an_unknown_root_to_a_sibling() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    // Declared private, and a slot the gate cannot read (read as private).
    let private = a
        .put_slotted(GRANTED, artifact_bytes(103, 400 * 1024), &[PRIVATE])
        .await;
    let unreadable = a
        .put_slotted(
            GRANTED,
            artifact_bytes(104, 400 * 1024),
            &["env:reach=PRIVATE"],
        )
        .await;
    assert!(a.reindex().await.is_empty());
    assert_eq!(
        a.mount.claim(&unreadable.manifest_cid),
        Some(ReachClaim::Declared(DeclaredReach::Unreadable))
    );
    // A record and an output this device never held, asked the same ways.
    let never = {
        let mut elsewhere = MemoryFlowStore::new();
        let store = eprfs_storage::MemoryStorage::default();
        put_slotted_with(
            &store,
            &mut elsewhere,
            descriptor(GRANTED),
            artifact_bytes(105, 400 * 1024),
            &[SELF],
        )
        .await
    };
    let b = client(&h.key_b).await;
    let unknown_legacy = frame_without_echo(&call(&b, &a.addr, manifest_of(&never)).await);
    assert_eq!(
        call(&b, &a.addr, record_of(&never, None)).await,
        no_record(&never)
    );
    for put in [&private, &unreadable] {
        let artifact = output(put);
        for request in [
            manifest_of(put),
            blob_of(put, &chunks(put)[0]),
            blob_of(put, &method_cid(put)),
        ] {
            let answer = call(&b, &a.addr, request).await;
            assert_eq!(answer, uniform(&artifact), "as an unknown root is");
            assert_eq!(frame_without_echo(&answer), unknown_legacy);
        }
        for cid in [None, Some(&chunks(put)[0])] {
            assert_eq!(
                call(&b, &a.addr, record_of(put, cid)).await,
                no_record(put),
                "as a manifest not held is"
            );
        }
    }
    // The serving device keeps the true reason, locally.
    let reasons = refusal_reasons(&a.drain());
    assert!(
        reasons.iter().any(|r| r == "serve-reach:private"),
        "{reasons:?}"
    );
    assert!(
        reasons.iter().any(|r| r == "serve-reach:unreadable"),
        "{reasons:?}"
    );
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_slotless_record_is_served_only_by_its_method_row() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    let bytes = artifact_bytes(106, 400 * 1024);
    let row = a.put(GRANTED, bytes.clone(), true).await;
    let no_row = a
        .put(NOT_SHARED, artifact_bytes(107, 400 * 1024), true)
        .await;
    assert!(a.reindex().await.is_empty());
    assert_eq!(a.mount.claim(&row.manifest_cid), Some(ReachClaim::Slotless));
    let b = client(&h.key_b).await;
    assert_eq!(draw(&b, &a.addr, &row).await, bytes);
    assert_eq!(
        call(&b, &a.addr, manifest_of(&no_row)).await,
        standing(&no_row, "serve-blob:not-shared")
    );
    // Reach never stands in for a row: no row, or a row without the steward, refuses it.
    for rows in [
        serde_json::json!({}),
        serde_json::json!({ GRANTED: [] }),
        serde_json::json!({ GRANTED: [format!("carrier:{}", h.b.did)] }),
    ] {
        let mut policy = shipped_policy();
        policy["berth"]["artifacts"]["serves"] = rows;
        set_policy(&a, &policy);
        for request in [manifest_of(&row), record_of(&row, None)] {
            assert_eq!(
                call(&b, &a.addr, request).await,
                standing(&row, "serve-blob:mesh-build")
            );
        }
    }
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_slotless_sysroot_floor_is_refused_with_the_shipped_policy() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    let floor = a.put(FLOOR, artifact_bytes(108, 400 * 1024), true).await;
    assert!(a.reindex().await.is_empty());
    assert_eq!(
        a.mount.claim(&floor.manifest_cid),
        Some(ReachClaim::Slotless)
    );
    let b = client(&h.key_b).await;
    for request in [
        manifest_of(&floor),
        blob_of(&floor, &chunks(&floor)[0]),
        record_of(&floor, None),
        record_of(&floor, Some(&method_cid(&floor))),
    ] {
        assert_eq!(
            call(&b, &a.addr, request).await,
            standing(&floor, "serve-blob:sysroot-pack")
        );
    }
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_row_that_excludes_steward_denies_a_self_record() {
    let h = household();
    let mut a = holder(&h).await;
    let build = a
        .put_slotted(GRANTED, artifact_bytes(109, 400 * 1024), &[SELF])
        .await;
    assert!(a.reindex().await.is_empty());
    let b = client(&h.key_b).await;
    for rows in [
        serde_json::json!([]),
        serde_json::json!([format!("carrier:{}", h.b.did)]),
    ] {
        let mut policy = shipped_policy();
        policy["berth"]["artifacts"]["serves"][GRANTED] = rows;
        set_policy(&a, &policy);
        for request in [
            manifest_of(&build),
            blob_of(&build, &chunks(&build)[0]),
            blob_of(&build, &method_cid(&build)),
            record_of(&build, None),
        ] {
            assert_eq!(
                call(&b, &a.addr, request).await,
                standing(&build, "serve-blob:mesh-build")
            );
        }
    }
    // The steward's row restored, the same record is served by reach.
    set_policy(&a, &shipped_policy());
    served_bytes(
        call(&b, &a.addr, manifest_of(&build)).await,
        &build.manifest_cid,
    );
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_record_declared_above_self_is_refused() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    let mut above = Vec::new();
    for level in [
        "intimate",
        "trusted",
        "familiar",
        "community",
        "public",
        "commons",
    ] {
        let slot = format!("env:reach={level}");
        let bytes = Bytes::from(format!("an artifact declared {level}"));
        above.push(a.put_slotted(GRANTED, bytes, &[slot.as_str()]).await);
    }
    // A floor, which no row serves, declared public: refused the same way.
    above.push(
        a.put_slotted(
            FLOOR,
            Bytes::from_static(b"a public floor"),
            &["env:reach=public"],
        )
        .await,
    );
    assert!(a.reindex().await.is_empty());
    let b = client(&h.key_b).await;
    for put in &above {
        assert!(matches!(
            a.mount.claim(&put.manifest_cid),
            Some(ReachClaim::Declared(DeclaredReach::Wider(_)))
        ));
        for request in [
            manifest_of(put),
            blob_of(put, &method_cid(put)),
            record_of(put, None),
        ] {
            assert_eq!(
                call(&b, &a.addr, request).await,
                standing(put, LADDER_UNBUILT)
            );
        }
    }
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_self_record_discloses_its_method_and_input_at_its_reach() {
    let h = household();
    let mut a = holder(&h).await;
    // No discloseMethod row at all: reach alone discloses a self record's method and input.
    set_policy(&a, &serve_policy());
    let build = a
        .put_slotted(GRANTED, Bytes::from_static(b"a self build"), &[SELF])
        .await;
    let floor = a
        .put_slotted(FLOOR, Bytes::from_static(b"a self floor"), &[SELF])
        .await;
    for put in [&build, &floor] {
        a.payload
            .put_blob_verified(&input_cid(put), Bytes::from(descriptor_name(put)))
            .await
            .unwrap();
    }
    assert!(a.reindex().await.is_empty());
    let b = client(&h.key_b).await;
    for put in [&build, &floor] {
        for object in [method_cid(put), input_cid(put)] {
            served_bytes(call(&b, &a.addr, blob_of(put, &object)).await, &object);
            served_bytes(
                call(&b, &a.addr, record_of(put, Some(&object))).await,
                &object,
            );
        }
    }
    let scope = a.scope();
    assert_eq!(ServeScope::methods_of(&scope, &method_cid(&floor)), [FLOOR]);
    b.close().await;
    a.router.shutdown().await.unwrap();
}

/// The method name a put's input document was keyed from.
fn descriptor_name(put: &TierBlobPut) -> Vec<u8> {
    [GRANTED, FLOOR]
        .into_iter()
        .find(|name| BlobCid::compute_raw(name.as_bytes()) == input_cid(put))
        .expect("a fixture method")
        .as_bytes()
        .to_vec()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_slotless_record_still_needs_disclose_method() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &serve_policy());
    let build = a
        .put(GRANTED, Bytes::from_static(b"a slotless build"), true)
        .await;
    a.payload
        .put_blob_verified(&input_cid(&build), Bytes::from(GRANTED))
        .await
        .unwrap();
    assert!(a.reindex().await.is_empty());
    let b = client(&h.key_b).await;
    for object in [method_cid(&build), input_cid(&build)] {
        for request in [blob_of(&build, &object), record_of(&build, Some(&object))] {
            assert_eq!(
                call(&b, &a.addr, request).await,
                standing(&build, "disclose-method:mesh-build")
            );
        }
    }
    set_policy(&a, &shipped_policy());
    for object in [method_cid(&build), input_cid(&build)] {
        served_bytes(call(&b, &a.addr, blob_of(&build, &object)).await, &object);
    }
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folded_sibling_produce_with_a_reach_slot_grants_nothing_here() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    // The bytes, chunks and method record are all held here (as after this device fetched
    // them), but the only Produce naming them is B's, folded via B, slot and all.
    let mut sibling_flow = MemoryFlowStore::new();
    let mut puts = Vec::new();
    for (method, seed) in [(FLOOR, 110), (GRANTED, 111)] {
        puts.push(
            put_slotted_with(
                a.payload.as_ref(),
                &mut sibling_flow,
                descriptor(method),
                artifact_bytes(seed, 400 * 1024),
                &[SELF],
            )
            .await,
        );
    }
    let b_did = h.b.did.clone();
    for (_, event) in sibling_flow.events().unwrap() {
        if event.action != ReaVerb::Produce {
            continue;
        }
        let mut event = event;
        event.classified_as.extend([
            "measure:artifact-produce@1".to_string(),
            "reason:produce".to_string(),
        ]);
        event.classified_as.push(format!("via:{b_did}"));
        a.flow
            .append(elohim_epr_rea::FlowRecord::Event(event))
            .unwrap();
    }
    assert!(a.reindex().await.is_empty(), "not even tried");
    let b = client(&h.key_b).await;
    for put in &puts {
        assert!(a.mount.held(&output(put)).is_none());
        assert_eq!(a.mount.claim(&put.manifest_cid), None);
        assert_eq!(
            call(&b, &a.addr, manifest_of(put)).await,
            typed(&output(put), ReachReason::OutsideStanding, None)
        );
        assert_eq!(
            call(&b, &a.addr, record_of(put, None)).await,
            no_record(put)
        );
    }
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reach_slot_on_another_custodians_produce_is_not_a_claim() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    let mut elsewhere = MemoryFlowStore::new();
    let theirs = put_slotted_with(
        a.payload.as_ref(),
        &mut elsewhere,
        descriptor(FLOOR),
        artifact_bytes(112, 400 * 1024),
        &[SELF],
    )
    .await;
    let b_did = h.b.did.clone();
    append_edited(&mut a, &elsewhere, |event| {
        event.receiver = AgentRef(b_did);
    });
    assert!(a.reindex().await.is_empty());
    assert!(a.mount.held(&output(&theirs)).is_none(), "B's custody");
    assert_eq!(a.mount.claim(&theirs.manifest_cid), None);
    let b = client(&h.key_b).await;
    assert_eq!(
        call(&b, &a.addr, manifest_of(&theirs)).await,
        typed(&output(&theirs), ReachReason::OutsideStanding, None)
    );
    assert_eq!(
        call(&b, &a.addr, record_of(&theirs, None)).await,
        no_record(&theirs)
    );
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_malformed_policy_reload_revokes_reach_serving_too() {
    let p = pair();
    let side = Side::new(&p.rows);
    let floor = put_slotted_with(
        side.payload().as_ref(),
        &mut side.flow(),
        descriptor(FLOOR),
        artifact_bytes(113, 400 * 1024),
        &[SELF],
    )
    .await;
    let policy_dir = tempfile::tempdir().expect("policy dir");
    let policy = policy_dir.path().join("pool-policy.json");
    let valid = serde_json::to_vec_pretty(&shipped_policy()).expect("policy");
    std::fs::write(&policy, &valid).expect("write policy");
    let port = free_port();
    let daemon = start_holder_with(&p, &side, port, Some(policy.clone())).await;
    let to_a = NodeAddr::new(daemon.node_id())
        .with_direct_addresses([std::net::SocketAddr::V4(local(port))]);
    let b = client(&load_key(p.key_dir_b.path())).await;
    served_bytes(
        until_served(&b, &to_a, manifest_of(&floor)).await,
        &floor.manifest_cid,
    );
    let revoked = standing(&floor, "serve-blob:sysroot-pack");
    let mut unknown_audience = shipped_policy();
    unknown_audience["berth"]["artifacts"]["serves"][GRANTED] = serde_json::json!(["everyone"]);
    for bad in [
        b"{ not a policy".to_vec(),
        serde_json::to_vec(&unknown_audience).expect("policy"),
    ] {
        std::fs::write(&policy, &bad).expect("write bad policy");
        tokio::time::timeout(WITHIN, async {
            loop {
                let answer = call(&b, &to_a, manifest_of(&floor)).await;
                if answer == revoked {
                    break;
                }
                assert!(matches!(answer, TierAnswer::Blob { .. }), "{answer:?}");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("reach serving revoked by a policy that does not read");
        assert_eq!(call(&b, &to_a, record_of(&floor, None)).await, revoked);
        std::fs::write(&policy, &valid).expect("restore policy");
        served_bytes(
            until_served(&b, &to_a, manifest_of(&floor)).await,
            &floor.manifest_cid,
        );
    }
    b.close().await;
    daemon.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_without_a_policy_serves_no_artifact_by_reach() {
    let p = pair();
    let side = Side::new(&p.rows);
    let payload = side.payload();
    let floor = put_slotted_with(
        payload.as_ref(),
        &mut side.flow(),
        descriptor(FLOOR),
        artifact_bytes(114, 400 * 1024),
        &[SELF],
    )
    .await;
    let build = put_slotted_with(
        payload.as_ref(),
        &mut side.flow(),
        descriptor(GRANTED),
        artifact_bytes(115, 400 * 1024),
        &[SELF],
    )
    .await;
    // The crossing and steward rules, and no serve policy.
    let rules = Rules::new(
        CrossingAllowList::from_policy(&support::policy()).unwrap(),
        StewardScope::from_policy(&support::policy()).unwrap(),
    );
    assert!(rules.serve_policy().is_none());
    assert!(rules.serves().is_empty());
    let port = free_port();
    let roster = SidecarRoster::open(side.checkout.dir.path(), HANDLE)
        .unwrap()
        .log_path()
        .to_path_buf();
    let daemon = PeerDaemon::start(PeerConfig {
        key: load_key(p.key_dir_a.path()),
        device: p.a.did.clone(),
        chain_root: p.a.did.clone(),
        flow_root: side.checkout.dir.path().to_path_buf(),
        roster,
        state_dir: side.state.path().to_path_buf(),
        rules,
        policy: None,
        relay: None,
        listen: Some(local(port)),
        peers: Vec::new(),
        signer: Arc::new(p.a.signer()),
        verifier: verifier(&[&p.a, &p.b]),
        log: Arc::new(|_line: &str| {}),
        payload_store: Some(payload),
        install: Default::default(),
    })
    .await
    .expect("A starts");
    let to_a = NodeAddr::new(daemon.node_id())
        .with_direct_addresses([std::net::SocketAddr::V4(local(port))]);
    let b = client(&load_key(p.key_dir_b.path())).await;
    // Unindexed, a record is refused with no label; indexed, it is refused naming its method.
    // It is never served.
    for (put, label) in [
        (&floor, "serve-blob:sysroot-pack"),
        (&build, "serve-blob:mesh-build"),
    ] {
        tokio::time::timeout(WITHIN, async {
            loop {
                let answer = call(&b, &to_a, manifest_of(put)).await;
                if answer == standing(put, label) {
                    break;
                }
                assert_eq!(
                    answer,
                    typed(&output(put), ReachReason::OutsideStanding, None)
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("indexed and refused");
        assert_eq!(
            call(&b, &to_a, record_of(put, None)).await,
            standing(put, label)
        );
    }
    b.close().await;
    daemon.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_later_narrower_slot_takes_effect_at_the_next_rebuild() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    let bytes = artifact_bytes(116, 400 * 1024);
    let first = a.put_slotted(GRANTED, bytes.clone(), &[SELF]).await;
    assert!(a.reindex().await.is_empty());
    let b = client(&h.key_b).await;
    served_bytes(
        call(&b, &a.addr, manifest_of(&first)).await,
        &first.manifest_cid,
    );
    let indexed = a.mount.reconstructions();
    // The same derivation re-put, its custody event now declaring private: the same manifest.
    let again = a.put_slotted(GRANTED, bytes, &[PRIVATE]).await;
    assert_eq!(again.manifest_cid, first.manifest_cid);
    // Until the next rebuild, the claim is the one folded last.
    served_bytes(
        call(&b, &a.addr, manifest_of(&first)).await,
        &first.manifest_cid,
    );
    assert!(a.reindex().await.is_empty());
    assert_eq!(
        a.mount.reconstructions(),
        indexed,
        "the indexed record is not rebuilt; only its claim is refolded"
    );
    assert_eq!(
        a.mount.claim(&first.manifest_cid),
        Some(ReachClaim::Declared(DeclaredReach::Private))
    );
    assert_eq!(
        call(&b, &a.addr, manifest_of(&first)).await,
        uniform(&output(&first))
    );
    assert_eq!(
        call(&b, &a.addr, record_of(&first, None)).await,
        no_record(&first)
    );
    // A slotless re-put by an older binary cannot send it back to the method-row path.
    a.put(GRANTED, artifact_bytes(116, 400 * 1024), true).await;
    assert!(a.reindex().await.is_empty());
    assert_eq!(
        call(&b, &a.addr, manifest_of(&first)).await,
        uniform(&output(&first))
    );
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_object_shared_with_a_private_record_is_served_only_under_the_permitted_one() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    // Two records of one method share its method record and input document.
    let private = a
        .put_slotted(GRANTED, artifact_bytes(117, 400 * 1024), &[PRIVATE])
        .await;
    let open = a
        .put_slotted(GRANTED, artifact_bytes(118, 400 * 1024), &[SELF])
        .await;
    assert!(a.reindex().await.is_empty());
    let method = method_cid(&open);
    assert_eq!(method, method_cid(&private));
    let private_chunk = chunks(&private)[0].clone();
    assert!(!chunks(&open).contains(&private_chunk));

    let b = client(&h.key_b).await;
    served_bytes(call(&b, &a.addr, blob_of(&open, &method)).await, &method);
    assert_eq!(
        call(&b, &a.addr, blob_of(&private, &method)).await,
        uniform(&output(&private))
    );
    // The any-match counts only the record that serves the object to this device.
    let scope = a.scope();
    assert_eq!(ServeScope::methods_of(&scope, &method), [GRANTED]);
    assert!(ServeScope::methods_of(&scope, &private_chunk).is_empty());
    assert!(!ServeScope::admits_object(&scope, &h.b.did, &private_chunk));
    assert!(ServeScope::admits_object(
        &scope,
        &h.b.did,
        &chunks(&open)[0]
    ));
    // Membership still binds an object to the record it was asked under.
    assert_eq!(
        call(&b, &a.addr, blob_of(&open, &private_chunk)).await,
        typed(&output(&open), ReachReason::NotUnderRoot, None)
    );
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stranger_gets_the_uniform_refusal_for_every_claim() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    let mut puts = vec![
        a.put(GRANTED, artifact_bytes(120, 200 * 1024), true).await,
        a.put(NOT_SHARED, artifact_bytes(121, 200 * 1024), true)
            .await,
    ];
    for (method, seed, slot) in [
        (FLOOR, 122, SELF),
        (GRANTED, 123, SELF),
        (GRANTED, 124, PRIVATE),
        (GRANTED, 125, "env:reach="),
        (GRANTED, 126, "env:reach=public"),
    ] {
        puts.push(
            a.put_slotted(method, artifact_bytes(seed, 200 * 1024), &[slot])
                .await,
        );
    }
    assert!(a.reindex().await.is_empty());
    let (_dir, stranger_key) = peer_key();
    let stranger = client(&stranger_key).await;
    let mut frames = Vec::new();
    let ask_all = async |frames: &mut Vec<Vec<u8>>| {
        for put in &puts {
            for request in [
                manifest_of(put),
                blob_of(put, &chunks(put)[0]),
                blob_of(put, &method_cid(put)),
                record_of(put, None),
            ] {
                let answer = call(&stranger, &a.addr, request).await;
                assert_eq!(answer, uniform(&output(put)));
                frames.push(frame_without_echo(&answer));
            }
        }
    };
    ask_all(&mut frames).await;
    // With no policy loaded too: an unavailable policy tells a stranger nothing either.
    *a.serves.write().unwrap() = None;
    ask_all(&mut frames).await;
    assert!(frames.windows(2).all(|pair| pair[0] == pair[1]));
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(a.drain().is_empty(), "strangers are never recorded");
    stranger.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_serve_use_event_names_its_basis() {
    let h = household();
    let mut a = holder(&h).await;
    set_policy(&a, &shipped_policy());
    let by_reach = a
        .put_slotted(FLOOR, artifact_bytes(130, 400 * 1024), &[SELF])
        .await;
    let by_row = a.put(GRANTED, artifact_bytes(131, 400 * 1024), true).await;
    let refused = a
        .put(NOT_SHARED, artifact_bytes(132, 400 * 1024), true)
        .await;
    assert!(a.reindex().await.is_empty());
    let b = client(&h.key_b).await;
    let connection = b.connect(a.addr.clone(), TIER_ALPN).await.expect("connect");
    draw_on(&connection, &by_reach).await;
    draw_on(&connection, &by_row).await;
    ask(&connection, manifest_of(&refused)).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let draws = a.drain();
    let completed = |put: &TierBlobPut| {
        draws
            .iter()
            .find(|draw| {
                draw.outcome == DrawOutcome::Completed
                    && draw.manifest.as_ref() == Some(&put.manifest_cid)
            })
            .cloned()
            .expect("a completed draw")
    };
    let at = "2026-11-08T00:00:00Z".to_string();
    let reach = completed(&by_reach);
    let row = completed(&by_row);
    assert_eq!(reach.basis, Some(Basis::Reach));
    assert_eq!(row.basis, Some(Basis::MethodRow));
    let reach_event = reach.use_event(&h.a.did, at.clone());
    let row_event = row.use_event(&h.a.did, at.clone());
    assert!(reach_event
        .classified_as
        .iter()
        .any(|s| s == GATE_REACH_SLOT));
    assert!(!reach_event.classified_as.iter().any(|s| s == GATE_ROW_SLOT));
    assert!(row_event.classified_as.iter().any(|s| s == GATE_ROW_SLOT));
    let refusal = draws
        .iter()
        .find(|draw| matches!(draw.outcome, DrawOutcome::Refused { .. }))
        .expect("a recorded refusal");
    assert_eq!(refusal.basis, None);
    let refusal_event = refusal.use_event(&h.a.did, at.clone());
    assert!(!refusal_event
        .classified_as
        .iter()
        .any(|s| s.starts_with("gate:")));
    for event in [&reach_event, &row_event, &refusal_event] {
        assert!(!event
            .classified_as
            .iter()
            .any(|s| s.starts_with("measure:")));
    }

    // `epr peer status` reads the three exit numbers from these events (K12 §4).
    let mut events: Vec<FlowEvent> = a
        .flow
        .events()
        .unwrap()
        .into_iter()
        .map(|(_, e)| e)
        .collect();
    events.extend([reach_event, row_event, refusal_event]);
    let mut old = completed(&by_row).use_event(&h.a.did, "2026-09-01T00:00:00Z".into());
    old.classified_as.retain(|s| !s.starts_with("gate:"));
    events.push(old);
    // 2026-11-08T18:55:42Z: the fixture custody events (2026-11-07) are inside the window.
    let now = UNIX_EPOCH + Duration::from_secs(1_794_077_742 + 86_400);
    let counts = artifact_gate_counts(events.iter(), &h.a.did, now);
    assert_eq!(counts.since, "2026-10-25T18:55:42Z");
    assert_eq!((counts.reach_draws, counts.row_draws), (1, 1));
    // The slotless mesh build and the slotless ungranted one; not the self floor.
    assert_eq!(counts.slotless_designated, 2);
    let later = SystemTime::UNIX_EPOCH + Duration::from_secs(1_794_077_742 + 30 * 86_400);
    let later = artifact_gate_counts(events.iter(), &h.a.did, later);
    assert_eq!(
        (
            later.reach_draws,
            later.row_draws,
            later.slotless_designated
        ),
        (0, 0, 0)
    );
    b.close().await;
    a.router.shutdown().await.unwrap();
}
