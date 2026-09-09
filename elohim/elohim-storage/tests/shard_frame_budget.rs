//! Native libp2p regression for the bounded RS shard transfer station.
//!
//! The provider and fetcher are real TCP + Noise + Yamux swarms. Distribution
//! crosses the production `ShardCodec`; the targeted Q4-style read crosses the
//! production `BlobCodec`. No doorway or HTTP path participates.

#![cfg(feature = "p2p")]

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use elohim_storage::blob_store::BlobStore;
use elohim_storage::p2p::{
    BlobCodec, BlobFetchRequest, BlobFetchResponse, BlobProtocol, ShardCodec, ShardProtocol,
    ShardRequest, ShardResponse, SHARD_TRANSFER_MAX_FRAME_SIZE,
};
use elohim_storage::services::custody_standing::Requester;
use elohim_storage::shard_service::ShardService;
use elohim_storage::sharding::{ShardConfig, ShardEncoder};
use futures::StreamExt;
use libp2p::{
    identity, noise,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use tempfile::tempdir;

const MIB: usize = 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(NetworkBehaviour)]
struct TestBehaviour {
    shard: request_response::Behaviour<ShardCodec>,
    blob: request_response::Behaviour<BlobCodec>,
}

fn build_swarm() -> Swarm<TestBehaviour> {
    let key = identity::Keypair::generate_ed25519();
    SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .expect("tcp transport")
        .with_behaviour(|_| TestBehaviour {
            shard: request_response::Behaviour::with_codec(
                ShardCodec,
                [(ShardProtocol, ProtocolSupport::Full)],
                request_response::Config::default().with_request_timeout(IO_TIMEOUT),
            ),
            blob: request_response::Behaviour::with_codec(
                BlobCodec::default(),
                [(BlobProtocol, ProtocolSupport::Full)],
                request_response::Config::default().with_request_timeout(IO_TIMEOUT),
            ),
        })
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(120)))
        .build()
}

async fn listen(swarm: &mut Swarm<TestBehaviour>) -> Multiaddr {
    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .expect("listen");
    loop {
        if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
            return address;
        }
    }
}

async fn await_connection(swarm: &mut Swarm<TestBehaviour>, provider: PeerId) {
    tokio::time::timeout(IO_TIMEOUT, async {
        loop {
            if let SwarmEvent::ConnectionEstablished { peer_id, .. } =
                swarm.select_next_some().await
            {
                if peer_id == provider {
                    return;
                }
            }
        }
    })
    .await
    .expect("libp2p connection timed out");
}

async fn await_shard_response(
    swarm: &mut Swarm<TestBehaviour>,
    request_id: request_response::OutboundRequestId,
) -> ShardResponse {
    tokio::time::timeout(IO_TIMEOUT, async {
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::Behaviour(TestBehaviourEvent::Shard(
                    request_response::Event::Message {
                        message:
                            request_response::Message::Response {
                                request_id: id,
                                response,
                            },
                        ..
                    },
                )) if id == request_id => return response,
                SwarmEvent::Behaviour(TestBehaviourEvent::Shard(
                    request_response::Event::OutboundFailure {
                        request_id: id,
                        error,
                        ..
                    },
                )) if id == request_id => panic!("shard request failed: {error:?}"),
                _ => {}
            }
        }
    })
    .await
    .expect("shard response timed out")
}

async fn await_blob_response(
    swarm: &mut Swarm<TestBehaviour>,
    request_id: request_response::OutboundRequestId,
) -> BlobFetchResponse {
    tokio::time::timeout(IO_TIMEOUT, async {
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::Behaviour(TestBehaviourEvent::Blob(
                    request_response::Event::Message {
                        message:
                            request_response::Message::Response {
                                request_id: id,
                                response,
                            },
                        ..
                    },
                )) if id == request_id => return response,
                SwarmEvent::Behaviour(TestBehaviourEvent::Blob(
                    request_response::Event::OutboundFailure {
                        request_id: id,
                        error,
                        ..
                    },
                )) if id == request_id => panic!("blob request failed: {error:?}"),
                _ => {}
            }
        }
    })
    .await
    .expect("blob response timed out")
}

fn encoded_rs_shard() -> (String, Vec<u8>) {
    // 0xff pins the largest per-byte representation in the existing
    // MessagePack integer-array encoding: the 17 MiB raw shard becomes a frame
    // above the old 16 MiB default while remaining below the shared 64 MiB cap.
    let artifact = vec![0xff; 68 * MIB];
    let encoder = ShardEncoder::new(ShardConfig::default());
    let manifest = encoder
        .create_manifest(&artifact, "application/octet-stream", "commons")
        .expect("RS manifest");
    assert_eq!(manifest.encoding, "rs-4-7");
    assert_eq!(manifest.data_shards, 4);
    assert_eq!(manifest.total_shards, 7);
    assert_eq!(manifest.shard_size, (17 * MIB) as u64);

    let shard = encoder
        .create_shards(&artifact, &manifest.encoding)
        .expect("RS shards")
        .into_iter()
        .next()
        .expect("first data shard");
    let frame = rmp_serde::to_vec(&ShardRequest::Push {
        hash: manifest.shard_hashes[0].clone(),
        data: shard.clone(),
    })
    .expect("historical shard wire");
    assert!(frame.len() > 16 * MIB, "fixture must cross the old cap");
    assert!(frame.len() <= SHARD_TRANSFER_MAX_FRAME_SIZE);
    (manifest.shard_hashes[0].clone(), shard)
}

#[tokio::test]
async fn rs_68_mib_shard_pushes_and_fetches_over_real_libp2p_codecs() -> Result<()> {
    let (hash, shard) = encoded_rs_shard();
    let provider_dir = tempdir()?;
    let store = Arc::new(BlobStore::new(provider_dir.path().to_path_buf()).await?);
    let service = Arc::new(ShardService::new(store.clone(), None));

    let mut provider = build_swarm();
    let provider_id = *provider.local_peer_id();
    let provider_addr = listen(&mut provider).await;
    let provider_task = tokio::spawn(async move {
        loop {
            match provider.select_next_some().await {
                SwarmEvent::Behaviour(TestBehaviourEvent::Shard(
                    request_response::Event::Message {
                        peer,
                        message:
                            request_response::Message::Request {
                                request, channel, ..
                            },
                        ..
                    },
                )) => {
                    let requester = Requester::libp2p(peer.to_string());
                    let response = service.handle(&requester, request).await;
                    let _ = provider
                        .behaviour_mut()
                        .shard
                        .send_response(channel, response);
                }
                SwarmEvent::Behaviour(TestBehaviourEvent::Blob(
                    request_response::Event::Message {
                        peer,
                        message:
                            request_response::Message::Request {
                                request, channel, ..
                            },
                        ..
                    },
                )) => {
                    let requester = Requester::libp2p(peer.to_string());
                    let response = match service
                        .handle(&requester, ShardRequest::Get { hash: request.hash })
                        .await
                    {
                        ShardResponse::Data(bytes) => BlobFetchResponse::found(bytes),
                        ShardResponse::Error(error) => BlobFetchResponse::error(error),
                        _ => BlobFetchResponse::not_found(),
                    };
                    let _ = provider
                        .behaviour_mut()
                        .blob
                        .send_response(channel, response);
                }
                _ => {}
            }
        }
    });

    let mut fetcher = build_swarm();
    fetcher.dial(provider_addr)?;
    await_connection(&mut fetcher, provider_id).await;

    let push_id = fetcher.behaviour_mut().shard.send_request(
        &provider_id,
        ShardRequest::Push {
            hash: hash.clone(),
            data: shard.clone(),
        },
    );
    assert!(matches!(
        await_shard_response(&mut fetcher, push_id).await,
        ShardResponse::PushAck
    ));
    assert!(store.exists(&hash).await, "PushAck requires stored bytes");

    let get_id = fetcher
        .behaviour_mut()
        .blob
        .send_request(&provider_id, BlobFetchRequest { hash: hash.clone() });
    match await_blob_response(&mut fetcher, get_id).await {
        BlobFetchResponse::Found(bytes) => assert_eq!(bytes, shard),
        other => panic!("expected Found, got {other:?}"),
    }

    provider_task.abort();
    Ok(())
}
