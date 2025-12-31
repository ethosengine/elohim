//! ElohimStorageBehaviour - Combined network behaviour for P2P shard transfer

use libp2p::{
    identity::Keypair,
    kad::{self, store::MemoryStore, Behaviour as Kademlia},
    mdns,
    request_response::{self, Behaviour as RequestResponse, ProtocolSupport},
    swarm::NetworkBehaviour,
    PeerId,
};
use std::time::Duration;

use super::shard_protocol::{ShardCodec, ShardProtocol};
use super::P2PConfig;

/// Combined network behaviour for elohim-storage
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "ElohimStorageBehaviourEvent")]
pub struct ElohimStorageBehaviour {
    /// Kademlia DHT for peer/content discovery
    pub kademlia: Kademlia<MemoryStore>,
    /// Request-response for shard transfer
    pub shard_protocol: RequestResponse<ShardCodec>,
    /// Local network discovery (mDNS)
    pub mdns: mdns::tokio::Behaviour,
}

/// Events emitted by ElohimStorageBehaviour
#[derive(Debug)]
pub enum ElohimStorageBehaviourEvent {
    /// Kademlia event
    Kademlia(kad::Event),
    /// Shard protocol event
    ShardProtocol(request_response::Event<super::ShardRequest, super::ShardResponse>),
    /// mDNS event
    Mdns(mdns::Event),
}

impl From<kad::Event> for ElohimStorageBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        Self::Kademlia(event)
    }
}

impl From<request_response::Event<super::ShardRequest, super::ShardResponse>>
    for ElohimStorageBehaviourEvent
{
    fn from(event: request_response::Event<super::ShardRequest, super::ShardResponse>) -> Self {
        Self::ShardProtocol(event)
    }
}

impl From<mdns::Event> for ElohimStorageBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        Self::Mdns(event)
    }
}

impl ElohimStorageBehaviour {
    /// Create a new behaviour
    pub fn new(keypair: Keypair, config: P2PConfig) -> Self {
        let peer_id = PeerId::from(keypair.public());

        // Kademlia DHT
        let store = MemoryStore::new(peer_id);
        let kademlia = Kademlia::new(peer_id, store);

        // Shard request-response protocol
        let shard_protocol = RequestResponse::new(
            [(ShardProtocol, ProtocolSupport::Full)],
            request_response::Config::default()
                .with_request_timeout(config.request_timeout),
        );

        // mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            peer_id,
        )
        .expect("mDNS behaviour should be created");

        Self {
            kademlia,
            shard_protocol,
            mdns,
        }
    }
}
