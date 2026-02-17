//! ElohimStorageBehaviour - Combined network behaviour for P2P shard and sync transfer

use libp2p::{
    autonat,
    dcutr,
    identify,
    identity::Keypair,
    kad::{self, Behaviour as Kademlia},
    mdns,
    relay,
    request_response::{self, Behaviour as RequestResponse, ProtocolSupport},
    swarm::{behaviour::toggle::Toggle, NetworkBehaviour},
    PeerId,
};

use super::kad_store::SledRecordStore;

use std::time::Duration;

use super::shard_protocol::{ShardCodec, ShardProtocol};
use super::sync_protocol::{SyncCodec, SyncProtocol};
use super::P2PConfig;

/// Relay operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayMode {
    /// Desktop stewards behind NAT — connect through relay servers
    Client,
    /// K8s edgenode pods with stable IPs — serve as relays for others
    Server,
    /// Doorway hosts — both relay client and server
    Both,
}

impl Default for RelayMode {
    fn default() -> Self {
        Self::Client
    }
}

impl std::str::FromStr for RelayMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "client" => Ok(Self::Client),
            "server" => Ok(Self::Server),
            "both" => Ok(Self::Both),
            _ => Err(format!("Invalid relay mode '{}': expected client, server, or both", s)),
        }
    }
}

impl std::fmt::Display for RelayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Client => write!(f, "client"),
            Self::Server => write!(f, "server"),
            Self::Both => write!(f, "both"),
        }
    }
}

/// Combined network behaviour for elohim-storage
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "ElohimStorageBehaviourEvent")]
pub struct ElohimStorageBehaviour {
    /// Kademlia DHT for peer/content discovery (sled-backed for persistence)
    pub kademlia: Kademlia<SledRecordStore>,
    /// Request-response for shard transfer
    pub shard_protocol: RequestResponse<ShardCodec>,
    /// Request-response for CRDT sync
    pub sync_protocol: RequestResponse<SyncCodec>,
    /// Local network discovery (mDNS)
    pub mdns: mdns::tokio::Behaviour,
    /// Relay client for NAT traversal (connect through relay servers)
    pub relay_client: relay::client::Behaviour,
    /// Relay server (accept relay reservations from NAT-ed peers)
    pub relay_server: Toggle<relay::Behaviour>,
    /// Direct Connection Upgrade through Relay (hole punching)
    pub dcutr: dcutr::Behaviour,
    /// Protocol identification (advertise supported protocols to peers)
    pub identify: identify::Behaviour,
    /// Automatic NAT detection (probe peers to determine NAT status)
    pub autonat: autonat::Behaviour,
}

/// Events emitted by ElohimStorageBehaviour
#[derive(Debug)]
pub enum ElohimStorageBehaviourEvent {
    /// Kademlia event
    Kademlia(kad::Event),
    /// Shard protocol event
    ShardProtocol(request_response::Event<super::ShardRequest, super::ShardResponse>),
    /// Sync protocol event
    SyncProtocol(request_response::Event<super::SyncRequest, super::SyncResponse>),
    /// mDNS event
    Mdns(mdns::Event),
    /// Relay client event (reservations, connection through relay)
    RelayClient(relay::client::Event),
    /// Relay server event (incoming reservations from NAT-ed peers)
    RelayServer(relay::Event),
    /// DCUtR event (direct connection upgrade after relay)
    Dcutr(dcutr::Event),
    /// Identify event (peer protocol information)
    Identify(identify::Event),
    /// AutoNAT event (NAT status changes)
    AutoNat(autonat::Event),
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

impl From<request_response::Event<super::SyncRequest, super::SyncResponse>>
    for ElohimStorageBehaviourEvent
{
    fn from(event: request_response::Event<super::SyncRequest, super::SyncResponse>) -> Self {
        Self::SyncProtocol(event)
    }
}

impl From<relay::client::Event> for ElohimStorageBehaviourEvent {
    fn from(event: relay::client::Event) -> Self {
        Self::RelayClient(event)
    }
}

impl From<relay::Event> for ElohimStorageBehaviourEvent {
    fn from(event: relay::Event) -> Self {
        Self::RelayServer(event)
    }
}

impl From<dcutr::Event> for ElohimStorageBehaviourEvent {
    fn from(event: dcutr::Event) -> Self {
        Self::Dcutr(event)
    }
}

impl From<identify::Event> for ElohimStorageBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        Self::Identify(event)
    }
}

impl From<autonat::Event> for ElohimStorageBehaviourEvent {
    fn from(event: autonat::Event) -> Self {
        Self::AutoNat(event)
    }
}

impl ElohimStorageBehaviour {
    /// Create a new behaviour with NAT traversal support.
    ///
    /// The `relay_client` is injected by SwarmBuilder's `.with_relay_client()` chain.
    /// The relay server is enabled based on `config.relay_mode`.
    pub fn new(
        keypair: Keypair,
        config: P2PConfig,
        relay_client: relay::client::Behaviour,
        sled_db: sled::Db,
    ) -> Self {
        let peer_id = PeerId::from(keypair.public());

        // Kademlia DHT with sled persistence (shared DB handle with DocStore)
        let store = SledRecordStore::from_db(sled_db)
            .expect("Failed to open sled Kademlia store");
        let kademlia = Kademlia::new(peer_id, store);

        // Shard request-response protocol
        let shard_protocol = RequestResponse::new(
            [(ShardProtocol, ProtocolSupport::Full)],
            request_response::Config::default()
                .with_request_timeout(config.request_timeout),
        );

        // Sync request-response protocol
        let sync_protocol = RequestResponse::new(
            [(SyncProtocol, ProtocolSupport::Full)],
            request_response::Config::default()
                .with_request_timeout(config.request_timeout),
        );

        // mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            peer_id,
        )
        .expect("mDNS behaviour should be created");

        // Relay server (enabled for Server/Both modes)
        let relay_server = match config.relay_mode {
            RelayMode::Server | RelayMode::Both => Toggle::from(Some(relay::Behaviour::new(
                peer_id,
                relay::Config::default(),
            ))),
            RelayMode::Client => Toggle::from(None),
        };

        // DCUtR for direct connection upgrade after relay
        let dcutr = dcutr::Behaviour::new(peer_id);

        // Identify protocol — advertise who we are and what we support
        let identify = identify::Behaviour::new(
            identify::Config::new(
                "/elohim/id/1.0.0".to_string(),
                keypair.public(),
            )
            .with_agent_version(format!(
                "elohim-storage/{}",
                env!("CARGO_PKG_VERSION")
            )),
        );

        // AutoNAT — probe peers to detect NAT status
        let autonat = autonat::Behaviour::new(
            peer_id,
            autonat::Config {
                boot_delay: Duration::from_secs(15),
                ..Default::default()
            },
        );

        Self {
            kademlia,
            shard_protocol,
            sync_protocol,
            mdns,
            relay_client,
            relay_server,
            dcutr,
            identify,
            autonat,
        }
    }
}
