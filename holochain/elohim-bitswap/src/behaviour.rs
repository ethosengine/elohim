//! Bitswap NetworkBehaviour — handles block exchange via libp2p.
//!
//! Ported from rust-ipfs libp2p-bitswap-next:
//! - Removed `StoreParams` generic (simplified to raw CID + bytes)
//! - Removed `libipld::Block` dependency (CID verification done at store layer)
//! - Removed compat feature (not needed for Elohim)
//! - Updated to libp2p 0.54 API

use crate::protocol::{
    BitswapCodec, BitswapProtocol, BitswapRequest, BitswapResponse, RequestType,
};
use crate::query::{QueryEvent, QueryId, QueryManager, Request, Response};
use crate::stats::*;
use cid::Cid;
use fnv::FnvHashMap;
use futures::future::BoxFuture;
use futures::{
    channel::mpsc,
    stream::{Stream, StreamExt},
    task::{Context, Poll},
};
use libp2p::core::{Endpoint, Multiaddr};
use libp2p::identity::PeerId;
use libp2p::swarm::behaviour::ConnectionEstablished;
use libp2p::swarm::{
    derive_prelude::{ConnectionClosed, FromSwarm},
    ConnectionDenied, ConnectionId, THandler, THandlerInEvent,
};
use libp2p::{
    request_response::{
        Behaviour as RequestResponse, Config as RequestResponseConfig,
        Event as RequestResponseEvent, InboundFailure, InboundRequestId,
        Message as RequestResponseMessage, OutboundFailure, OutboundRequestId, ProtocolSupport,
        ResponseChannel,
    },
    swarm::{ConnectionHandler, NetworkBehaviour, ToSwarm},
};
use prometheus::Registry;
use std::{pin::Pin, time::Duration};

/// Bitswap response channel.
pub type Channel = ResponseChannel<BitswapResponse>;

/// Event emitted by the bitswap behaviour.
#[derive(Debug)]
pub enum BitswapEvent {
    /// Progress on a sync query (number of known missing blocks).
    Progress(QueryId, usize),
    /// A get or sync query completed.
    Complete(QueryId, anyhow::Result<()>),
}

/// Trait implemented by a block store for bitswap.
///
/// Simplified from upstream: no `StoreParams` generic, uses raw CID + bytes.
#[async_trait::async_trait]
pub trait BitswapStore: Send + Sync + 'static {
    /// Check if the store contains a block with the given CID.
    async fn contains(&self, cid: &Cid) -> anyhow::Result<bool>;
    /// Retrieve block data by CID.
    async fn get(&self, cid: &Cid) -> anyhow::Result<Option<Vec<u8>>>;
    /// Insert a block (CID + data) into the store.
    async fn insert(&self, cid: Cid, data: Vec<u8>) -> anyhow::Result<()>;
    /// Return CIDs of blocks referenced by the given block that are not in the store.
    async fn missing_blocks(&self, cid: &Cid) -> anyhow::Result<Vec<Cid>>;
}

/// Bitswap configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BitswapConfig {
    /// Timeout of a request.
    pub request_timeout: Duration,
}

impl BitswapConfig {
    /// Creates a new `BitswapConfig`.
    pub fn new() -> Self {
        Self {
            request_timeout: Duration::from_secs(10),
        }
    }
}

impl Default for BitswapConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BitswapId {
    Bitswap(OutboundRequestId),
}

enum BitswapChannel {
    Bitswap(Channel),
}

/// Network behaviour that handles sending and receiving blocks.
pub struct Bitswap {
    inner: RequestResponse<BitswapCodec>,
    query_manager: QueryManager,
    requests: FnvHashMap<BitswapId, QueryId>,
    db_tx: mpsc::UnboundedSender<DbRequest>,
    db_rx: mpsc::UnboundedReceiver<DbResponse>,
}

impl Bitswap {
    /// Creates a new `Bitswap` behaviour.
    pub fn new<S: BitswapStore>(
        config: BitswapConfig,
        store: S,
        executor: Box<dyn FnOnce(BoxFuture<'static, ()>)>,
    ) -> Self {
        let rr_config =
            RequestResponseConfig::default().with_request_timeout(config.request_timeout);
        let inner = RequestResponse::new(
            [(BitswapProtocol, ProtocolSupport::Full)],
            rr_config,
        );
        let (db_tx, db_rx) = start_db_thread(store, executor);
        Self {
            inner,
            query_manager: Default::default(),
            requests: Default::default(),
            db_tx,
            db_rx,
        }
    }

    /// Adds an address for a peer.
    #[allow(deprecated)]
    pub fn add_address(&mut self, peer_id: &PeerId, addr: Multiaddr) {
        self.query_manager.add_peer(peer_id);
        self.inner.add_address(peer_id, addr);
    }

    /// Removes an address for a peer.
    #[allow(deprecated)]
    pub fn remove_address(&mut self, peer_id: &PeerId, addr: &Multiaddr) {
        self.query_manager.remove_peer(peer_id);
        self.inner.remove_address(peer_id, addr);
    }

    /// Starts a get query with an initial guess of providers.
    pub fn get(&mut self, cid: Cid, peers: impl Iterator<Item = PeerId>) -> QueryId {
        self.query_manager.get(None, cid, peers)
    }

    /// Starts a sync query with the initial set of missing blocks.
    pub fn sync(
        &mut self,
        cid: Cid,
        peers: Vec<PeerId>,
        missing: impl Iterator<Item = Cid>,
    ) -> QueryId {
        self.query_manager.sync(cid, peers, missing)
    }

    /// Cancels an in-progress query. Returns true if a query was cancelled.
    pub fn cancel(&mut self, id: QueryId) -> bool {
        let res = self.query_manager.cancel(id);
        if res {
            REQUESTS_CANCELED.inc();
        }
        res
    }

    /// Notify bitswap that new blocks are available locally.
    /// (Currently a no-op — peers must poll. Future: push announcements.)
    pub fn notify_new_blocks(&mut self, _cids: &[Cid]) {
        // TODO: optionally push Have announcements to connected peers
    }

    /// Registers prometheus metrics.
    pub fn register_metrics(&self, registry: &Registry) -> anyhow::Result<()> {
        registry.register(Box::new(REQUESTS_TOTAL.clone()))?;
        registry.register(Box::new(REQUEST_DURATION_SECONDS.clone()))?;
        registry.register(Box::new(REQUESTS_CANCELED.clone()))?;
        registry.register(Box::new(BLOCK_NOT_FOUND.clone()))?;
        registry.register(Box::new(PROVIDERS_TOTAL.clone()))?;
        registry.register(Box::new(MISSING_BLOCKS_TOTAL.clone()))?;
        registry.register(Box::new(RECEIVED_BLOCK_BYTES.clone()))?;
        registry.register(Box::new(RECEIVED_INVALID_BLOCK_BYTES.clone()))?;
        registry.register(Box::new(SENT_BLOCK_BYTES.clone()))?;
        registry.register(Box::new(RESPONSES_TOTAL.clone()))?;
        registry.register(Box::new(THROTTLED_INBOUND.clone()))?;
        registry.register(Box::new(THROTTLED_OUTBOUND.clone()))?;
        registry.register(Box::new(OUTBOUND_FAILURE.clone()))?;
        registry.register(Box::new(INBOUND_FAILURE.clone()))?;
        Ok(())
    }
}

enum DbRequest {
    Bitswap(BitswapChannel, BitswapRequest),
    Insert(Cid, Vec<u8>),
    MissingBlocks(QueryId, Cid),
}

enum DbResponse {
    Bitswap(BitswapChannel, BitswapResponse),
    MissingBlocks(QueryId, anyhow::Result<Vec<Cid>>),
}

fn start_db_thread<S: BitswapStore>(
    store: S,
    executor: Box<dyn FnOnce(BoxFuture<'static, ()>)>,
) -> (
    mpsc::UnboundedSender<DbRequest>,
    mpsc::UnboundedReceiver<DbResponse>,
) {
    let (tx, requests) = mpsc::unbounded();
    let (responses, rx) = mpsc::unbounded();
    executor(Box::pin(async move {
        let mut requests: mpsc::UnboundedReceiver<DbRequest> = requests;
        while let Some(request) = requests.next().await {
            match request {
                DbRequest::Bitswap(channel, request) => {
                    let response = match request.ty {
                        RequestType::Have => {
                            let have = store.contains(&request.cid).await.unwrap_or_default();
                            if have {
                                RESPONSES_TOTAL.with_label_values(&["have"]).inc();
                            } else {
                                RESPONSES_TOTAL.with_label_values(&["dont_have"]).inc();
                            }
                            tracing::trace!("have {}", have);
                            BitswapResponse::Have(have)
                        }
                        RequestType::Block => {
                            let block = store.get(&request.cid).await.unwrap_or_default();
                            if let Some(data) = block {
                                RESPONSES_TOTAL.with_label_values(&["block"]).inc();
                                SENT_BLOCK_BYTES.inc_by(data.len() as u64);
                                tracing::trace!("block {}", data.len());
                                BitswapResponse::Block(data)
                            } else {
                                RESPONSES_TOTAL.with_label_values(&["dont_have"]).inc();
                                tracing::trace!("have false");
                                BitswapResponse::Have(false)
                            }
                        }
                    };
                    responses
                        .unbounded_send(DbResponse::Bitswap(channel, response))
                        .ok();
                }
                DbRequest::Insert(cid, data) => {
                    if let Err(err) = store.insert(cid, data).await {
                        tracing::error!("error inserting block: {}", err);
                    }
                }
                DbRequest::MissingBlocks(id, cid) => {
                    let res = store.missing_blocks(&cid).await;
                    responses
                        .unbounded_send(DbResponse::MissingBlocks(id, res))
                        .ok();
                }
            }
        }
    }));
    (tx, rx)
}

impl Bitswap {
    fn inject_request(&mut self, channel: BitswapChannel, request: BitswapRequest) {
        self.db_tx
            .unbounded_send(DbRequest::Bitswap(channel, request))
            .ok();
    }

    fn inject_response(&mut self, id: BitswapId, peer: PeerId, response: BitswapResponse) {
        if let Some(id) = self.requests.remove(&id) {
            match response {
                BitswapResponse::Have(have) => {
                    self.query_manager
                        .inject_response(id, Response::Have(peer, have));
                }
                BitswapResponse::Block(data) => {
                    if let Some(info) = self.query_manager.query_info(id) {
                        let len = data.len();
                        let cid = info.cid;
                        // Simplified: trust the data, verify at store layer
                        RECEIVED_BLOCK_BYTES.inc_by(len as u64);
                        self.db_tx
                            .unbounded_send(DbRequest::Insert(cid, data))
                            .ok();
                        self.query_manager
                            .inject_response(id, Response::Block(peer, true));
                    }
                }
            }
        }
    }

    fn inject_outbound_failure(
        &mut self,
        peer: &PeerId,
        request_id: OutboundRequestId,
        error: &OutboundFailure,
    ) {
        tracing::debug!(
            "bitswap outbound failure {} {} {:?}",
            peer,
            request_id,
            error
        );
        match error {
            OutboundFailure::DialFailure => {
                OUTBOUND_FAILURE.with_label_values(&["dial_failure"]).inc();
            }
            OutboundFailure::Timeout => {
                OUTBOUND_FAILURE.with_label_values(&["timeout"]).inc();
            }
            OutboundFailure::ConnectionClosed => {
                OUTBOUND_FAILURE
                    .with_label_values(&["connection_closed"])
                    .inc();
            }
            OutboundFailure::UnsupportedProtocols => {
                OUTBOUND_FAILURE
                    .with_label_values(&["unsupported_protocols"])
                    .inc();
            }
            OutboundFailure::Io(_) => {
                OUTBOUND_FAILURE.with_label_values(&["io"]).inc();
            }
        }
    }

    fn inject_inbound_failure(
        &mut self,
        peer: &PeerId,
        request_id: InboundRequestId,
        error: &InboundFailure,
    ) {
        tracing::error!(
            "bitswap inbound failure {} {} {:?}",
            peer,
            request_id,
            error
        );
        match error {
            InboundFailure::Timeout => {
                INBOUND_FAILURE.with_label_values(&["timeout"]).inc();
            }
            InboundFailure::ConnectionClosed => {
                INBOUND_FAILURE
                    .with_label_values(&["connection_closed"])
                    .inc();
            }
            InboundFailure::UnsupportedProtocols => {
                INBOUND_FAILURE
                    .with_label_values(&["unsupported_protocols"])
                    .inc();
            }
            InboundFailure::ResponseOmission => {
                INBOUND_FAILURE
                    .with_label_values(&["response_omission"])
                    .inc();
            }
            InboundFailure::Io(_) => {
                INBOUND_FAILURE.with_label_values(&["io"]).inc();
            }
        }
    }
}

impl NetworkBehaviour for Bitswap {
    type ConnectionHandler =
        <RequestResponse<BitswapCodec> as NetworkBehaviour>::ConnectionHandler;
    type ToSwarm = BitswapEvent;

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.inner.handle_established_inbound_connection(
            connection_id,
            peer,
            local_addr,
            remote_addr,
        )
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role_override: Endpoint,
        port_use: libp2p::core::transport::PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.inner.handle_established_outbound_connection(
            connection_id,
            peer,
            addr,
            role_override,
            port_use,
        )
    }

    fn handle_pending_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        maybe_peer: Option<PeerId>,
        addresses: &[Multiaddr],
        effective_role: Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        self.inner.handle_pending_outbound_connection(
            connection_id,
            maybe_peer,
            addresses,
            effective_role,
        )
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        match event {
            FromSwarm::ConnectionEstablished(ConnectionEstablished {
                peer_id,
                other_established,
                connection_id,
                endpoint,
                failed_addresses,
            }) => {
                if other_established == 0 {
                    self.query_manager.add_peer(&peer_id);
                }
                self.inner.on_swarm_event(FromSwarm::ConnectionEstablished(
                    ConnectionEstablished {
                        peer_id,
                        other_established,
                        connection_id,
                        endpoint,
                        failed_addresses,
                    },
                ));
            }
            FromSwarm::ConnectionClosed(ConnectionClosed {
                peer_id,
                connection_id,
                endpoint,
                remaining_established,
                cause,
            }) => {
                if remaining_established == 0 {
                    self.query_manager.remove_peer(&peer_id);
                }
                self.inner
                    .on_swarm_event(FromSwarm::ConnectionClosed(ConnectionClosed {
                        peer_id,
                        connection_id,
                        endpoint,
                        remaining_established,
                        cause,
                    }));
            }
            ev => self.inner.on_swarm_event(ev),
        }
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        conn: ConnectionId,
        event: <Self::ConnectionHandler as ConnectionHandler>::ToBehaviour,
    ) {
        tracing::trace!(?event, "on_connection_handler_event");
        self.inner
            .on_connection_handler_event(peer_id, conn, event);
    }

    fn poll(&mut self, cx: &mut Context) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        let mut exit = false;
        while !exit {
            exit = true;
            // Process DB responses
            while let Poll::Ready(Some(response)) = Pin::new(&mut self.db_rx).poll_next(cx) {
                exit = false;
                match response {
                    DbResponse::Bitswap(channel, response) => match channel {
                        BitswapChannel::Bitswap(channel) => {
                            self.inner.send_response(channel, response).ok();
                        }
                    },
                    DbResponse::MissingBlocks(id, res) => match res {
                        Ok(missing) => {
                            MISSING_BLOCKS_TOTAL.inc_by(missing.len() as u64);
                            self.query_manager
                                .inject_response(id, Response::MissingBlocks(missing));
                        }
                        Err(err) => {
                            self.query_manager.cancel(id);
                            let event = BitswapEvent::Complete(id, Err(err));
                            return Poll::Ready(ToSwarm::GenerateEvent(event));
                        }
                    },
                }
            }
            // Process query events
            while let Some(query) = self.query_manager.next() {
                exit = false;
                match query {
                    QueryEvent::Request(id, req) => match req {
                        Request::Have(peer_id, cid) => {
                            let req = BitswapRequest {
                                ty: RequestType::Have,
                                cid,
                            };
                            let rid = self.inner.send_request(&peer_id, req);
                            self.requests.insert(BitswapId::Bitswap(rid), id);
                        }
                        Request::Block(peer_id, cid) => {
                            let req = BitswapRequest {
                                ty: RequestType::Block,
                                cid,
                            };
                            let rid = self.inner.send_request(&peer_id, req);
                            self.requests.insert(BitswapId::Bitswap(rid), id);
                        }
                        Request::MissingBlocks(cid) => {
                            self.db_tx
                                .unbounded_send(DbRequest::MissingBlocks(id, cid))
                                .ok();
                        }
                    },
                    QueryEvent::Progress(id, missing) => {
                        let event = BitswapEvent::Progress(id, missing);
                        return Poll::Ready(ToSwarm::GenerateEvent(event));
                    }
                    QueryEvent::Complete(id, res) => {
                        if res.is_err() {
                            BLOCK_NOT_FOUND.inc();
                        }
                        let event = BitswapEvent::Complete(
                            id,
                            res.map_err(|cid| {
                                anyhow::anyhow!("block not found: {}", cid)
                            }),
                        );
                        return Poll::Ready(ToSwarm::GenerateEvent(event));
                    }
                }
            }
            // Process inner request-response events
            while let Poll::Ready(event) = self.inner.poll(cx) {
                exit = false;
                let event = match event {
                    ToSwarm::GenerateEvent(event) => event,
                    ToSwarm::Dial { opts } => {
                        return Poll::Ready(ToSwarm::Dial { opts });
                    }
                    ToSwarm::NotifyHandler {
                        peer_id,
                        handler,
                        event,
                    } => {
                        return Poll::Ready(ToSwarm::NotifyHandler {
                            peer_id,
                            handler,
                            event,
                        });
                    }
                    ToSwarm::ListenOn { opts } => {
                        return Poll::Ready(ToSwarm::ListenOn { opts });
                    }
                    ToSwarm::RemoveListener { id } => {
                        return Poll::Ready(ToSwarm::RemoveListener { id });
                    }
                    ToSwarm::NewExternalAddrCandidate(address) => {
                        return Poll::Ready(ToSwarm::NewExternalAddrCandidate(address));
                    }
                    ToSwarm::ExternalAddrConfirmed(address) => {
                        return Poll::Ready(ToSwarm::ExternalAddrConfirmed(address));
                    }
                    ToSwarm::ExternalAddrExpired(address) => {
                        return Poll::Ready(ToSwarm::ExternalAddrExpired(address));
                    }
                    ToSwarm::CloseConnection {
                        peer_id,
                        connection,
                    } => {
                        return Poll::Ready(ToSwarm::CloseConnection {
                            peer_id,
                            connection,
                        });
                    }
                    _ => continue,
                };
                match event {
                    RequestResponseEvent::Message { peer, message } => match message {
                        RequestResponseMessage::Request {
                            request,
                            channel,
                            ..
                        } => self.inject_request(BitswapChannel::Bitswap(channel), request),
                        RequestResponseMessage::Response {
                            request_id,
                            response,
                        } => self.inject_response(BitswapId::Bitswap(request_id), peer, response),
                    },
                    RequestResponseEvent::ResponseSent { .. } => {}
                    RequestResponseEvent::OutboundFailure {
                        peer,
                        request_id,
                        error,
                    } => {
                        self.inject_outbound_failure(&peer, request_id, &error);
                        if let Some(id) = self.requests.remove(&BitswapId::Bitswap(request_id)) {
                            self.query_manager
                                .inject_response(id, Response::Have(peer, false));
                        }
                    }
                    RequestResponseEvent::InboundFailure {
                        peer,
                        request_id,
                        error,
                    } => {
                        self.inject_inbound_failure(&peer, request_id, &error);
                    }
                }
            }
        }
        Poll::Pending
    }
}
