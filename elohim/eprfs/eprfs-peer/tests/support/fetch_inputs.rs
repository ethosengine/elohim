use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn daemon_fetches_only_input_document_and_filters_unadmitted_holders() {
    let t = three();
    let lines = "link:fixture\tcontent\n";
    let key = key_for(lines);
    let a_side = Side::new(&t.rows);
    let store = a_side.store();
    put_method_record(store.as_ref(), &descriptor())
        .await
        .unwrap();
    eprfs_peer::put_input_document(store.as_ref(), key.input.cid(), Bytes::from(lines))
        .await
        .unwrap();
    let bytes = Bytes::from_static(b"executable not requested");
    let put = put_local_blob(
        store.as_ref(),
        &mut a_side.checkout.store(),
        record(&key, &bytes),
        bytes,
        &context(&t.a.did),
    )
    .await
    .unwrap();
    let a_port = free_port();
    let a_daemon = a_side
        .start(
            &t,
            &t.a,
            PeerKey::load_or_generate(t._dirs[0].path()).unwrap(),
            a_port,
            vec![],
        )
        .await;
    let addr =
        NodeAddr::new(a_daemon.node_id()).with_direct_addresses([std::net::SocketAddr::from((
            std::net::Ipv4Addr::LOCALHOST,
            a_port,
        ))]);
    let c_side = Side::new(&t.rows);
    let c_daemon = c_side
        .start(
            &t,
            &t.c,
            PeerKey::load_or_generate(t._dirs[2].path()).unwrap(),
            free_port(),
            vec![addr],
        )
        .await;
    let mut request = request_for(
        &key,
        &put.manifest.record.output,
        vec![eprfs_peer::RequestedHolder {
            device: t.a.did.clone(),
            manifest: put.manifest_cid.clone(),
        }],
        std::path::PathBuf::new(),
    );
    request.inputs_only = true;
    request.scope.clear();
    within("verified input document", async {
        loop {
            request.nonce = eprfs_peer::FetchRequest::fresh_nonce();
            match queue_and_wait(&c_side.queue(), &request).await {
                eprfs_peer::FetchDone::InputDocument { cid, .. } => {
                    assert_eq!(cid, *key.input.cid());
                    break;
                }
                eprfs_peer::FetchDone::Gap {
                    gap: FetchGap::Refused { .. },
                    ..
                } => {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
                other => panic!("unexpected answer {other:?}"),
            }
        }
    })
    .await;
    let fetched = c_side
        .store()
        .fetch_blob(key.input.cid(), eprfs_core::FetchPolicy::LocalOnly)
        .await
        .unwrap();
    assert_eq!(fetched.bytes, lines.as_bytes());
    assert!(c_side
        .store()
        .fetch_blob(
            &put.manifest.chunks[0].cid,
            eprfs_core::FetchPolicy::LocalOnly
        )
        .await
        .is_err());
    assert!(std::fs::read_dir(c_side.install_root())
        .unwrap()
        .next()
        .is_none());
    assert!(!c_side
        .checkout
        .events()
        .iter()
        .any(|(_, event)| event.action == ReaVerb::Use));
    request.nonce = eprfs_peer::FetchRequest::fresh_nonce();
    request.holders[0].device = "did:key:stranger".into();
    assert!(matches!(
        queue_and_wait(&c_side.queue(), &request).await,
        eprfs_peer::FetchDone::Gap { .. }
    ));
    c_daemon.shutdown().await;
    a_daemon.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn input_document_fetch_verifies_requested_cid_before_use() {
    let lines = "link:crt1.o\tfirst\nmacro:__SSE2__\t1\n";
    let key = key_for(lines);
    let held = held_artifact(&key, Bytes::from_static(b"artifact")).await;
    eprfs_peer::put_input_document(held.store.as_ref(), key.input.cid(), Bytes::from(lines))
        .await
        .unwrap();
    assert!(eprfs_peer::put_input_document(
        held.store.as_ref(),
        key.input.cid(),
        Bytes::from_static(b"wrong")
    )
    .await
    .is_err());
    let (_dir, peer) = peer_key();
    let endpoint = bind(&peer, None, vec![]).await.unwrap();
    for corrupt in [false, true] {
        let running = scripted(
            &held,
            Faults {
                corrupt: corrupt.then(|| key.input.cid().clone()),
                ..Faults::default()
            },
        )
        .await;
        let holder = Holder {
            device: "admitted-fixture".into(),
            addr: running.addr.clone(),
            manifest: held.manifest_cid.clone(),
        };
        let result = eprfs_peer::fetch_input_document(
            &endpoint,
            &key,
            &held.manifest.record.output,
            &[holder],
            &FetchBudget::default(),
        )
        .await;
        if corrupt {
            assert!(result.is_err(), "mismatched input bytes must refuse");
        } else {
            assert_eq!(result.unwrap(), lines.as_bytes());
        }
        assert_eq!(
            running.tier.asked.lock().unwrap().as_slice(),
            &[
                TierRequest::Record {
                    artifact: held.manifest.record.output.clone(),
                    manifest: held.manifest_cid.clone(),
                    key: Some(eprfs_peer::key_cid(&key).unwrap()),
                    cid: None,
                },
                TierRequest::Record {
                    artifact: held.manifest.record.output.clone(),
                    manifest: held.manifest_cid.clone(),
                    key: Some(eprfs_peer::key_cid(&key).unwrap()),
                    cid: Some(key.input.cid().clone()),
                },
            ],
            "no artifact chunks or unrelated objects are fetched"
        );
        running._router.shutdown().await.unwrap();
    }
    endpoint.close().await;
}
