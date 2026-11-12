use super::*;

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
