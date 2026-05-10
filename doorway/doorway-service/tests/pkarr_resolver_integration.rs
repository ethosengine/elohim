//! Integration test for the pkarr resolver endpoint.
//!
//! Uses the service module's handlers directly (no full HTTP server needed)
//! to verify the publish → retrieve → verify roundtrip and LRU eviction.

use bytes::Bytes;
use doorway::services::pkarr_resolver::{
    handle_get_pkarr, handle_put_pkarr, PkarrResolverConfig, PkarrResolverService,
};
use pkarr::{
    dns::{rdata::TXT, Name},
    Keypair, SignedPacketBuilder,
};
use std::sync::Arc;

fn make_packet(kp: &Keypair) -> pkarr::SignedPacket {
    SignedPacketBuilder::default()
        .txt(
            Name::new("_iroh").expect("name"),
            TXT::new()
                .with_string("relay=https://my-doorway.example/iroh-relay")
                .expect("txt"),
            300,
        )
        .build(kp)
        .expect("packet builds")
}

#[tokio::test]
async fn full_publish_resolve_roundtrip() {
    let cfg = PkarrResolverConfig {
        enabled: true,
        cache_capacity: Some(100),
        persist_dir: None,
    };
    let svc = Arc::new(PkarrResolverService::new(cfg));

    let kp = Keypair::random();
    let public_key = kp.public_key();
    let pk_str = public_key.to_string();
    let packet = make_packet(&kp);

    // Publish
    let put_body = Bytes::copy_from_slice(&packet.to_relay_payload());
    let put_resp = handle_put_pkarr(svc.clone(), &pk_str, put_body, None).await;
    assert_eq!(put_resp.status(), 200, "PUT should return 200");

    // Resolve
    let get_resp = handle_get_pkarr(svc.clone(), &pk_str, None).await;
    assert_eq!(get_resp.status(), 200, "GET should return 200 after PUT");

    let bytes = http_body_util::BodyExt::collect(get_resp.into_body())
        .await
        .unwrap()
        .to_bytes();

    // Verify signature round-trips correctly
    let recovered = pkarr::SignedPacket::from_relay_payload(&public_key, &bytes)
        .expect("relay payload deserializes + verifies");
    assert_eq!(
        recovered.public_key().to_string(),
        pk_str,
        "recovered public key must match"
    );
}

#[tokio::test]
async fn cache_evicts_under_capacity_pressure() {
    let cfg = PkarrResolverConfig {
        enabled: true,
        cache_capacity: Some(2),
        persist_dir: None,
    };
    let svc = Arc::new(PkarrResolverService::new(cfg));

    // Publish 3 packets — LRU capacity is 2, so oldest should evict
    for _ in 0..3 {
        let kp = Keypair::random();
        let packet = SignedPacketBuilder::default()
            .txt(
                Name::new("_iroh").unwrap(),
                TXT::new().with_string("v=0.1").unwrap(),
                300,
            )
            .build(&kp)
            .unwrap();
        let body = Bytes::copy_from_slice(&packet.to_relay_payload());
        let resp = handle_put_pkarr(svc.clone(), &kp.public_key().to_string(), body, None).await;
        assert_eq!(resp.status(), 200);
    }

    let stats = svc.stats().await;
    assert_eq!(
        stats.cached_packets, 2,
        "LRU should have evicted the oldest"
    );
}
