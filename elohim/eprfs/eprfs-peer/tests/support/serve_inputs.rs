use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn exact_older_record_serves_manifest_inputs_and_bytes_and_refuses_unknown_manifest() {
    let h = household();
    let mut a = holder(&h).await;
    let bytes = Bytes::from_static(b"same output at two commits");
    let seed = a.put(GRANTED, bytes.clone(), true).await;
    let mut records = Vec::new();
    for (commit, at) in [("A", "2026-11-08T10:00:00Z"), ("B", "2026-11-09T10:00:00Z")] {
        let mut record = seed.manifest.record.clone();
        record.witness = Witness::Signature {
            sig: format!("fixture-{commit}"),
        };
        let mut context = context();
        context.occurred_at = at.into();
        records.push(
            eprfs_peer::put_local_blob_with(
                a.payload.as_ref(),
                &mut a.flow,
                record,
                bytes.clone(),
                &context,
                vec![format!("env:commit={commit}")],
            )
            .await
            .unwrap(),
        );
    }
    let old = &records[0];
    let new = &records[1];
    assert_ne!(old.manifest_cid, new.manifest_cid);
    assert_eq!(old.manifest.record.key, new.manifest.record.key);
    let key = &old.manifest.record.key;
    a.payload
        .put_blob_verified(key.input.cid(), Bytes::from(GRANTED))
        .await
        .unwrap();
    assert!(a.reindex().await.is_empty());
    assert_eq!(
        a.mount.held(&output(old)).unwrap().manifest,
        new.manifest_cid
    );
    let mut policy = serve_policy();
    policy["berth"]["artifacts"]["discloseMethod"] = serde_json::json!([GRANTED]);
    *a.serves.write().unwrap() = ArtifactServeList::from_policy(&policy).unwrap();
    let b = client(&h.key_b).await;
    let requested = TierRequest::Record {
        artifact: output(old),
        manifest: old.manifest_cid.clone(),
        key: Some(eprfs_peer::key_cid(key).unwrap()),
        cid: None,
    };
    served_bytes(
        call(&b, &a.addr, requested.clone()).await,
        &old.manifest_cid,
    );
    let holders = [eprfs_peer::Holder {
        device: h.a.did.clone(),
        addr: a.addr.clone(),
        manifest: old.manifest_cid.clone(),
    }];
    let budget = eprfs_peer::FetchBudget::default();
    let inputs = eprfs_peer::fetch_input_document(&b, key, &output(old), &holders, &budget)
        .await
        .unwrap();
    assert_eq!(inputs, GRANTED.as_bytes());
    let fetched_store = eprfs_storage::MemoryStorage::default();
    let fetched =
        eprfs_peer::fetch_artifact(&b, &fetched_store, key, &output(old), &holders, &budget)
            .await
            .unwrap();
    assert_eq!(fetched.manifest_cid, old.manifest_cid);
    // Output-authorized artifact fetch uses the same exact record and method disclosure path.
    let output_store = eprfs_storage::MemoryStorage::default();
    let fetched = eprfs_peer::fetch_authorized_until(
        &b,
        &output_store,
        &eprfs_peer::FetchAuthority::Output { cid: output(old) },
        &output(old),
        &holders,
        &budget,
        Instant::now() + budget.total,
    )
    .await
    .unwrap();
    assert_eq!(fetched.manifest_cid, old.manifest_cid);

    let unknown = BlobCid::compute(b"not in custody");
    let mut missing = holders[0].clone();
    missing.manifest = unknown.clone();
    for inputs_only in [true, false] {
        let gap = if inputs_only {
            eprfs_peer::fetch_input_document(&b, key, &output(old), &[missing.clone()], &budget)
                .await
                .unwrap_err()
        } else {
            eprfs_peer::fetch_artifact(
                &b,
                &fetched_store,
                key,
                &output(old),
                &[missing.clone()],
                &budget,
            )
            .await
            .unwrap_err()
        };
        assert!(
            matches!(gap, eprfs_peer::FetchGap::Refused { ref refusal, .. }
            if refusal.contains("NoRecord") && refusal.contains(&unknown.to_string())),
            "{gap:?}"
        );
    }
    // Neither another key nor another output can borrow this record's grant.
    for (artifact, asked_key) in [
        (output(old), unknown.clone()),
        (unknown.clone(), eprfs_peer::key_cid(key).unwrap()),
    ] {
        assert_eq!(
            call(
                &b,
                &a.addr,
                TierRequest::Record {
                    artifact: artifact.clone(),
                    manifest: old.manifest_cid.clone(),
                    key: Some(asked_key),
                    cid: None,
                }
            )
            .await,
            typed(
                &artifact,
                ReachReason::NoRecord,
                Some(&format!("manifest:{}", old.manifest_cid))
            )
        );
    }
    // Disclosure revocation still blocks this older record's input document.
    *a.serves.write().unwrap() = ArtifactServeList::from_policy(&serve_policy()).unwrap();
    assert!(
        eprfs_peer::fetch_input_document(&b, key, &output(old), &holders, &budget)
            .await
            .is_err()
    );
    // Judge the selected record's method, even when the latest output record is ungranted.
    let ungranted = a.put(NOT_SHARED, bytes, true).await;
    assert!(a.reindex().await.is_empty());
    served_bytes(
        call(&b, &a.addr, requested.clone()).await,
        &old.manifest_cid,
    );
    assert_eq!(
        call(
            &b,
            &a.addr,
            TierRequest::Record {
                artifact: output(&ungranted),
                manifest: ungranted.manifest_cid.clone(),
                key: None,
                cid: None,
            }
        )
        .await,
        typed(
            &output(old),
            ReachReason::OutsideStanding,
            Some("serve-blob:not-shared")
        )
    );
    let (_dir, stranger_key) = peer_key();
    let stranger = client(&stranger_key).await;
    assert_eq!(
        call(&stranger, &a.addr, requested).await,
        uniform(&output(old))
    );
    stranger.close().await;
    b.close().await;
    a.router.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn input_document_shares_method_disclosure_and_steward_only_admission() {
    let h = household();
    let mut a = holder(&h).await;
    let put = a.put(GRANTED, Bytes::from_static(b"payload"), true).await;
    let input = put.manifest.record.key.input.cid().clone();
    a.payload
        .put_blob_verified(&input, Bytes::from(GRANTED))
        .await
        .unwrap();
    assert!(a.reindex().await.is_empty());
    let scope = DeviceScope::new(
        Arc::new(ChannelStore::default()),
        Some(a.mount.clone()),
        a.serves.clone(),
        a.admission.clone(),
    );
    let request = TierRequest::Blob {
        artifact: output(&put),
        cid: input.clone(),
    };
    let b = client(&h.key_b).await;
    let denied = typed(
        &output(&put),
        ReachReason::OutsideStanding,
        Some("disclose-method:mesh-build"),
    );
    assert_eq!(call(&b, &a.addr, request.clone()).await, denied);
    assert!(eprfs_peer::ServeScope::methods_of(&scope, &input).is_empty());
    let mut policy = serve_policy();
    policy["berth"]["artifacts"]["discloseMethod"] = serde_json::json!([GRANTED]);
    *a.serves.write().unwrap() = ArtifactServeList::from_policy(&policy).unwrap();
    assert_eq!(
        served_bytes(call(&b, &a.addr, request.clone()).await, &input),
        GRANTED.as_bytes()
    );
    assert_eq!(
        eprfs_peer::ServeScope::methods_of(&scope, &input),
        [GRANTED]
    );

    let (_dir, stranger_key) = peer_key();
    let stranger = client(&stranger_key).await;
    assert_eq!(
        call(&stranger, &a.addr, request.clone()).await,
        uniform(&output(&put))
    );
    policy["berth"]["artifacts"]["serves"][GRANTED] =
        serde_json::json!([format!("carrier:{}", h.b.did)]);
    *a.serves.write().unwrap() = ArtifactServeList::from_policy(&policy).unwrap();
    assert_eq!(
        call(&b, &a.addr, request.clone()).await,
        typed(
            &output(&put),
            ReachReason::OutsideStanding,
            Some("serve-blob:mesh-build")
        )
    );
    *a.serves.write().unwrap() = ArtifactServeList::from_policy(&serve_policy()).unwrap();
    assert_eq!(call(&b, &a.addr, request).await, denied);
    assert!(eprfs_peer::ServeScope::methods_of(&scope, &input).is_empty());
    stranger.close().await;
    b.close().await;
    a.router.shutdown().await.unwrap();
}
