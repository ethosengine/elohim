//! ElohimStorageBehaviour - Combined network behaviour for P2P shard and sync transfer

use libp2p::{
    autonat, dcutr, gossipsub, identify,
    identity::Keypair,
    kad::{self, Behaviour as Kademlia},
    mdns, ping, relay,
    request_response::{self, Behaviour as RequestResponse, ProtocolSupport},
    swarm::{behaviour::toggle::Toggle, NetworkBehaviour},
    PeerId,
};

use super::kad_store::SledRecordStore;

use std::time::Duration;

use super::blob_protocol::{BlobCodec, BlobProtocol};
use super::epr_atom_protocol::{EprAtomCodec, EprAtomProtocol};
use super::epr_protocol::{EprCodec, EprProtocol};
use super::identity_binding_gossip::IDENTITY_BINDING_TOPIC;
use super::identity_handshake::{IdentityHandshakeCodec, IdentityHandshakeProtocol};
use super::recovery_invitation::RECOVERY_INVITATION_TOPIC;
use super::shamir_transport::{
    ShamirShareCodec, ShamirShareProtocol, ShamirShareRequest, ShamirShareResponse,
};
use super::shard_protocol::{ShardCodec, ShardProtocol};
use super::sync_protocol::{SyncCodec, SyncProtocol};
use super::trust_protocol::{TrustCodec, TrustProtocol};
use super::view_federation::{ViewFederationCodec, ViewFederationProtocol};
use super::P2PConfig;

/// Relay operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelayMode {
    /// Desktop stewards behind NAT — connect through relay servers
    #[default]
    Client,
    /// K8s edgenode pods with stable IPs — serve as relays for others
    Server,
    /// Doorway hosts — both relay client and server
    Both,
}

impl std::str::FromStr for RelayMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "client" => Ok(Self::Client),
            "server" => Ok(Self::Server),
            "both" => Ok(Self::Both),
            _ => Err(format!(
                "Invalid relay mode '{}': expected client, server, or both",
                s
            )),
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
    /// Request-response for explicit-peer blob fetch (`/elohim/blob/1.0.0`).
    ///
    /// T21: Stage-2 wiring of the T17/T20 race-fetch helper. Differs from
    /// `shard_protocol` in that the caller routes to a specific `peer_id`
    /// rather than any-connected-peer.
    pub blob_protocol: RequestResponse<BlobCodec>,
    /// Request-response for CRDT sync
    pub sync_protocol: RequestResponse<SyncCodec>,
    /// Request-response for EPR Head resolution (legacy /elohim/epr/1.0.0)
    pub epr_protocol: RequestResponse<EprCodec>,
    /// Request-response for signed EPR atom federation (/elohim/epr-atom/1.0.0)
    pub epr_atom_protocol: RequestResponse<EprAtomCodec>,
    /// Request-response for trust negotiation
    pub trust_protocol: RequestResponse<TrustCodec>,
    /// Request-response for Shamir share delivery (`/elohim/shamir-share/1.0.0`).
    ///
    /// Recovery M4 T19: optional cryptographic proof layer for key-material
    /// recovery. The DHT carries authorization (recovery-approval attestations);
    /// this protocol carries the share material point-to-point. Low-security
    /// recovery completes via social-threshold quorum alone and never invokes
    /// this transport. The swarm event-loop match arm + custodian dial logic
    /// land in T20; until then the behaviour is registered (so peers advertise
    /// the protocol via identify) but produces no handled events.
    pub shamir_share: RequestResponse<ShamirShareCodec>,
    /// Request-response for identity handshake (/elohim/identity/handshake/1.0.0)
    ///
    /// Category C — session-local projection of the AgentPeerBinding DHT entry.
    /// Fires on ConnectionEstablished; receiver verifies and writes into
    /// `peer_identity_bindings` with source='handshake'.
    pub identity_handshake: RequestResponse<IdentityHandshakeCodec>,
    /// Request-response for federated view-slice fetch (/elohim/view-federation/1.0.0).
    ///
    /// F-T18: Phase 3 wiring — peers ask each other for signed cluster /
    /// peer-topology slices. The dispatcher (caller side) and responder
    /// handler land in F-T19/F-T20; until then the behaviour is registered
    /// (so peers advertise the protocol) but produces no handled events.
    pub view_federation: RequestResponse<ViewFederationCodec>,
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
    /// Ping protocol for RTT measurement
    pub ping: ping::Behaviour,
    /// Gossipsub for pub/sub messaging (M3: recovery invitation broadcast)
    pub gossipsub: gossipsub::Behaviour,
}

/// Events emitted by ElohimStorageBehaviour
#[derive(Debug)]
pub enum ElohimStorageBehaviourEvent {
    /// Kademlia event
    Kademlia(kad::Event),
    /// Shard protocol event
    ShardProtocol(request_response::Event<super::ShardRequest, super::ShardResponse>),
    /// Blob protocol event (`/elohim/blob/1.0.0`)
    BlobProtocol(request_response::Event<super::BlobFetchRequest, super::BlobFetchResponse>),
    /// Sync protocol event
    SyncProtocol(request_response::Event<super::SyncRequest, super::SyncResponse>),
    /// EPR protocol event (legacy /elohim/epr/1.0.0)
    EprProtocol(request_response::Event<super::EprRequest, super::EprResponse>),
    /// EPR atom federation event (/elohim/epr-atom/1.0.0)
    EprAtomProtocol(request_response::Event<super::EprAtomRequest, super::EprAtomResponse>),
    /// Trust protocol event
    TrustProtocol(
        request_response::Event<
            super::trust_protocol::TrustHandshake,
            super::trust_protocol::TrustResponse,
        >,
    ),
    /// Shamir share protocol event (`/elohim/shamir-share/1.0.0`).
    ///
    /// Recovery M4 T19: variant constructed via the `From` impl below. The
    /// match arm in `p2p/mod.rs`'s swarm event loop is wired in T20 (custodian
    /// dial logic + responder authorization checks against DHT-projected
    /// recovery-approval attestations).
    ShamirShare(request_response::Event<ShamirShareRequest, ShamirShareResponse>),
    /// Identity handshake event (/elohim/identity/handshake/1.0.0)
    IdentityHandshake(
        request_response::Event<
            super::identity_handshake::IdentityHandshakeRequest,
            super::identity_handshake::IdentityHandshakeResponse,
        >,
    ),
    /// View federation event (/elohim/view-federation/1.0.0).
    ///
    /// F-T18: variant constructed via the `From` impl below. The match arm
    /// in `p2p/mod.rs`'s swarm event loop is wired in F-T19/F-T20.
    ViewFederation(
        request_response::Event<
            crate::views::ViewFederationRequest,
            crate::views::ViewFederationResponse,
        >,
    ),
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
    /// Ping event (RTT measurement)
    Ping(ping::Event),
    /// Gossipsub event (pub/sub messages)
    Gossipsub(gossipsub::Event),
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

impl From<request_response::Event<super::BlobFetchRequest, super::BlobFetchResponse>>
    for ElohimStorageBehaviourEvent
{
    fn from(
        event: request_response::Event<super::BlobFetchRequest, super::BlobFetchResponse>,
    ) -> Self {
        Self::BlobProtocol(event)
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

impl From<request_response::Event<super::EprRequest, super::EprResponse>>
    for ElohimStorageBehaviourEvent
{
    fn from(event: request_response::Event<super::EprRequest, super::EprResponse>) -> Self {
        Self::EprProtocol(event)
    }
}

impl From<request_response::Event<super::EprAtomRequest, super::EprAtomResponse>>
    for ElohimStorageBehaviourEvent
{
    fn from(event: request_response::Event<super::EprAtomRequest, super::EprAtomResponse>) -> Self {
        Self::EprAtomProtocol(event)
    }
}

impl
    From<
        request_response::Event<
            super::trust_protocol::TrustHandshake,
            super::trust_protocol::TrustResponse,
        >,
    > for ElohimStorageBehaviourEvent
{
    fn from(
        event: request_response::Event<
            super::trust_protocol::TrustHandshake,
            super::trust_protocol::TrustResponse,
        >,
    ) -> Self {
        Self::TrustProtocol(event)
    }
}

impl From<request_response::Event<ShamirShareRequest, ShamirShareResponse>>
    for ElohimStorageBehaviourEvent
{
    fn from(event: request_response::Event<ShamirShareRequest, ShamirShareResponse>) -> Self {
        Self::ShamirShare(event)
    }
}

impl
    From<
        request_response::Event<
            super::identity_handshake::IdentityHandshakeRequest,
            super::identity_handshake::IdentityHandshakeResponse,
        >,
    > for ElohimStorageBehaviourEvent
{
    fn from(
        event: request_response::Event<
            super::identity_handshake::IdentityHandshakeRequest,
            super::identity_handshake::IdentityHandshakeResponse,
        >,
    ) -> Self {
        Self::IdentityHandshake(event)
    }
}

impl
    From<
        request_response::Event<
            crate::views::ViewFederationRequest,
            crate::views::ViewFederationResponse,
        >,
    > for ElohimStorageBehaviourEvent
{
    fn from(
        event: request_response::Event<
            crate::views::ViewFederationRequest,
            crate::views::ViewFederationResponse,
        >,
    ) -> Self {
        Self::ViewFederation(event)
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

impl From<ping::Event> for ElohimStorageBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        Self::Ping(event)
    }
}

impl From<gossipsub::Event> for ElohimStorageBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        Self::Gossipsub(event)
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
        let store = SledRecordStore::from_db(sled_db).expect("Failed to open sled Kademlia store");
        let kademlia = Kademlia::new(peer_id, store);

        // Shard request-response protocol
        let shard_protocol = RequestResponse::new(
            [(ShardProtocol, ProtocolSupport::Full)],
            request_response::Config::default().with_request_timeout(config.request_timeout),
        );

        // T21: Blob fetch request-response protocol — explicit-peer fetch for
        // the T17/T20 race-fetch helper. `with_codec` lets us plumb the
        // configurable max response size (default 16 MiB, hard-capped at 64 MiB).
        let blob_protocol = RequestResponse::with_codec(
            BlobCodec::with_max_response_size(config.max_blob_response_size),
            [(BlobProtocol, ProtocolSupport::Full)],
            request_response::Config::default().with_request_timeout(config.request_timeout),
        );

        // Sync request-response protocol
        let sync_protocol = RequestResponse::new(
            [(SyncProtocol, ProtocolSupport::Full)],
            request_response::Config::default().with_request_timeout(config.request_timeout),
        );

        // EPR request-response protocol for cross-peer content resolution (legacy)
        let epr_protocol = RequestResponse::new(
            [(EprProtocol, ProtocolSupport::Full)],
            request_response::Config::default().with_request_timeout(config.request_timeout),
        );

        // EPR atom federation protocol (/elohim/epr-atom/1.0.0) — signed atoms
        let epr_atom_protocol = RequestResponse::new(
            [(EprAtomProtocol, ProtocolSupport::Full)],
            request_response::Config::default().with_request_timeout(config.request_timeout),
        );

        // Trust request-response protocol for per-connection trust negotiation
        let trust_protocol = RequestResponse::new(
            [(TrustProtocol, ProtocolSupport::Full)],
            request_response::Config::default().with_request_timeout(config.request_timeout),
        );

        // Recovery M4 T19: Shamir share request-response protocol — optional
        // cryptographic proof layer for key-material recovery. Authorization is
        // verified against the DHT-projected recovery-approval attestation; this
        // channel carries only share material. Codec enforces a 1 MiB ceiling
        // (see `MAX_SHAMIR_MESSAGE_SIZE`) — shares are always ≤ 64 bytes plus
        // signing overhead; the cap is a runaway guard, not a tuning knob.
        let shamir_share = RequestResponse::new(
            [(ShamirShareProtocol, ProtocolSupport::Full)],
            request_response::Config::default().with_request_timeout(config.request_timeout),
        );

        // Identity handshake protocol — Category C session-local projection of
        // AgentPeerBinding DHT entries. Fires on ConnectionEstablished.
        let identity_handshake = RequestResponse::new(
            [(IdentityHandshakeProtocol, ProtocolSupport::Full)],
            request_response::Config::default().with_request_timeout(config.request_timeout),
        );

        // F-T18: View federation protocol — explicit-peer fetch of signed
        // cluster + peer-topology slices on behalf of an agent.
        //
        // Layering constraint: the transport budget is deliberately generous
        // (the general `config.request_timeout`, like every other behaviour),
        // and the CALLERS own the effective deadline — reconcile sweeps pass
        // `PEER_TIMEOUT` (10s) to `view_federate`, the HTTP aggregator passes
        // `ELOHIM_FEDERATION_TIMEOUT_MS` (3s). A short transport-level timeout
        // here would fire BEFORE those caller budgets and tear the stream while
        // a marginal-but-alive responder is still building/signing the slice
        // (the F-T20 "failed to send" race), so the transport must not be the
        // one that gives up first. See `view_federation_request_timeout`.
        let view_federation = RequestResponse::with_codec(
            ViewFederationCodec,
            [(ViewFederationProtocol, ProtocolSupport::Full)],
            request_response::Config::default()
                .with_request_timeout(view_federation_request_timeout(&config)),
        );

        // mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)
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

        // Identify protocol — advertise who we are and what we support.
        //
        // The `agent_version` carries a `http_port=<port>` suffix after the
        // usual build-info user-agent string — this node's OWN real HTTP
        // port (`config.http_port`, threaded from `Config::http_port`), so a
        // receiving peer can populate `DeliveryPeer::http_port` correctly
        // instead of guessing 8090 for everyone (a libp2p multiaddr carries
        // no HTTP-port information at all). See
        // `p2p::mod::parse_http_port_from_agent_version`, the sole decoder,
        // and `genesis/data/timeline/backlog/
        // delivery-peer-row-local-port-and-untyped-capabilities.md`.
        //
        // A `caps=<a,b,c>` token follows it — this node's self-declared
        // delivery capabilities (`identity::DeliveryCapabilities`
        // vocabulary), derived from whether it actually runs an extraction
        // cache (`config.serves_extracted`). Decoded by
        // `p2p::mod::parse_capabilities_from_agent_version` onto the typed
        // `DeliveryPeer` fields, so `serves_extracted` on the HTTP row is a
        // declared fact rather than a permanent default-false.
        let declared = crate::identity::DeliveryCapabilities {
            serves_extracted: config.serves_extracted,
            serves_compressed: true,
            ready_content: vec![],
            cache_tier: if config.serves_extracted {
                crate::identity::CacheTier::Extraction
            } else {
                crate::identity::CacheTier::BlobOnly
            },
        };
        let identify = identify::Behaviour::new(
            identify::Config::new("/elohim/id/1.0.0".to_string(), keypair.public())
                .with_agent_version(format!(
                    "{} http_port={} caps={}",
                    elohim_compute::BuildInfo::new("elohim-storage").user_agent(),
                    config.http_port,
                    declared.to_capability_strings().join(",")
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

        // Ping cadence is a config knob (`P2P_PING_INTERVAL_SECS` /
        // `P2P_PING_TIMEOUT_SECS`); a failed ping closes its connection in
        // `P2PNode::handle_behaviour_event`, so these two numbers bound how
        // long a silent peer can stay in the connected set.
        let ping = ping::Behaviour::new(
            ping::Config::new()
                .with_interval(Duration::from_secs(config.ping_interval_secs.max(1)))
                .with_timeout(Duration::from_secs(config.ping_timeout_secs.max(1))),
        );

        // Gossipsub — pub/sub for recovery invitation broadcast (M3) and
        // identity binding propagation (A.10: mid-session rotation).
        // Uses the node's signing keypair for message authentication.
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("valid gossipsub config");
        let mut gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .expect("gossipsub behaviour init");
        let recovery_topic = gossipsub::IdentTopic::new(RECOVERY_INVITATION_TOPIC);
        gossipsub
            .subscribe(&recovery_topic)
            .expect("subscribe to recovery.invitation");
        // A.10: subscribe to identity binding topic — propagates AgentPeerBinding
        // DHT entries to peers as an operational projection (Category C).
        let identity_binding_topic = gossipsub::IdentTopic::new(IDENTITY_BINDING_TOPIC);
        gossipsub
            .subscribe(&identity_binding_topic)
            .expect("subscribe to elohim/identity/binding");
        // M4: subscribe to key revocation fan-out topic.
        // Subscriber set: emergency contacts, specialist-elohim watchers, security dashboards.
        let revocation_topic = gossipsub::IdentTopic::new(super::RECOVERY_REVOCATION_TOPIC);
        // TODO(M3/M4-migration): remove this legacy subscription once all
        // recovery.revocation publishers (M3/M4) migrate to TOPIC_INTEGRITY_REVOCATION
        // (= "elohim/integrity/revocation"). Tracked by Recovery epic M4. Until then,
        // storage subscribes to BOTH names so it never misses a revocation regardless
        // of which name the publisher uses.
        gossipsub
            .subscribe(&revocation_topic)
            .expect("subscribe to recovery.revocation");
        // D.3: subscribe to the new canonical integrity-revocation topic.
        // Dual-subscription during transition: M3/M4 publishers still use
        // "recovery.revocation" (above); new D.5+ publishers use the canonical
        // "elohim/integrity/revocation" name. Both are subscribed so this node
        // receives revocations from both old and new publishers until M3/M4 is
        // updated to use the canonical name exclusively.
        let integrity_revocation_topic =
            gossipsub::IdentTopic::new(super::TOPIC_INTEGRITY_REVOCATION);
        gossipsub
            .subscribe(&integrity_revocation_topic)
            .expect("subscribe to elohim/integrity/revocation");
        // T14: subscribe to blob inventory topic for peer-to-peer inventory reconciliation.
        // Broadcaster peers publish `BlobInventorySnapshot` and `BlobInventoryDelta` on this topic.
        // The receive arm in p2p/mod.rs projects arrivals into `peer_blob_inventory` (SQLite).
        let inventory_topic = gossipsub::IdentTopic::new(super::inventory_gossip::INVENTORY_TOPIC);
        gossipsub
            .subscribe(&inventory_topic)
            .expect("subscribe to elohim/inventory/blob");
        // P3-4: subscribe to the salvage-capacity topic. Opt-in peers publish
        // `SalvageCapacityAd`; the receive arm in p2p/mod.rs projects arrivals
        // into `salvage_capacity` (SQLite), populating the salvage candidate pool.
        let salvage_topic =
            gossipsub::IdentTopic::new(super::salvage_gossip::SALVAGE_CAPACITY_TOPIC);
        gossipsub
            .subscribe(&salvage_topic)
            .expect("subscribe to elohim/storage/salvage");
        // Custody-announcement topic: peers broadcast shard_locations writes
        // (self-held / push-ack) so the holder footprint converges across pods;
        // the receive arm projects arrivals as `peer-announced` rows under the
        // never-overwrite-local rule (db/shard_locations::apply_peer_announced).
        let custody_topic =
            gossipsub::IdentTopic::new(super::custody_announce::CUSTODY_ANNOUNCE_TOPIC);
        gossipsub
            .subscribe(&custody_topic)
            .expect("subscribe to elohim/storage/custody");
        // Task 4.4: subscribe to all observation namespaces so peers receive
        // CursorAnnouncements.  Role-based subscription override is a follow-on
        // (see spec §5.3 + open question 4).
        for namespace in &["infrastructure", "lamad", "imagodei", "shefa", "qahal"] {
            let obs_topic = gossipsub::IdentTopic::new(format!(
                "{}{}",
                crate::p2p::topics::OBSERVATION_GOSSIP_TOPIC_PREFIX,
                namespace
            ));
            let _ = gossipsub.subscribe(&obs_topic);
        }

        Self {
            kademlia,
            shard_protocol,
            blob_protocol,
            sync_protocol,
            epr_protocol,
            epr_atom_protocol,
            trust_protocol,
            shamir_share,
            identity_handshake,
            view_federation,
            mdns,
            relay_client,
            relay_server,
            dcutr,
            identify,
            autonat,
            ping,
            gossipsub,
        }
    }
}

/// Transport-level request timeout for the view-federation behaviour.
///
/// This MUST track the general `config.request_timeout` — the view-federation
/// callers (reconcile sweep 10s, HTTP aggregator 3s) own the effective deadline,
/// and the transport must not give up before them. Any future desire for a
/// view-federation-specific timeout goes HERE (and must reckon with the
/// stream-tear race documented at the construction site) rather than a hardcoded
/// literal at the call.
pub(crate) fn view_federation_request_timeout(config: &P2PConfig) -> std::time::Duration {
    config.request_timeout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_federation_timeout_tracks_general_and_is_not_the_old_3s_pin() {
        let config = P2PConfig::default();
        // Layering: the transport budget equals the general request timeout…
        assert_eq!(
            view_federation_request_timeout(&config),
            config.request_timeout
        );
        // …and is decisively NOT the retired 3-second pin that fired before the
        // caller budgets and tore marginal-but-alive responders' streams.
        assert_ne!(
            view_federation_request_timeout(&config),
            std::time::Duration::from_secs(3)
        );
        // Default general timeout is 30s (mod.rs P2PConfig::default).
        assert_eq!(
            view_federation_request_timeout(&config),
            std::time::Duration::from_secs(30)
        );
    }
}
