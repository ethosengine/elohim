//! The tier ALPN: a device serves its own eprfs store to the person's other admitted devices
//! (spec §3.2), and asks theirs.
//!
//! One request per bidirectional stream. Each frame is a big-endian `u32` length and then that
//! many bytes of canonical DAG-CBOR (the codec eprfs-core encodes trees with): a [`TierRequest`]
//! one way, a [`TierAnswer`] back. A length past the cap is refused, and a body is read as it
//! arrives, so memory follows the bytes a peer actually sent, never the length it claimed.
//!
//! Serving work is bounded three ways ([`TierLimits`]): every frame must arrive (or leave) within
//! a deadline, one connection has at most a few streams answered at once, and every connection of
//! one [`TierProtocol`] shares a bound on streams in flight. That bound is two pools: the node id
//! iroh authenticated is classified when each stream is accepted, and admitted devices draw on
//! one pool, everyone else on a smaller other, so a stranger flood never sheds the person's own
//! devices. A stream finding its pool empty is reset at once, before any byte of it is read. The
//! request itself is still judged on decode, so a device dropped by a reload mid-connection gets
//! the uniform refusal, and a stranger's refusal is unchanged.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use eprfs_core::{
    encode_tree, BatchOptions, BlobCid, DirListing, EprfsStorage, ItemAbsence, ReadLeaf, TierId,
    TreeChange, TreeError, TreeLimits, TreeNode, TreePath, TreeResult, View,
};
use eprfs_serve::TierEndpoint;
use iroh::endpoint::{Connection, RecvStream, SendStream, VarInt};
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh::{Endpoint, NodeAddr, NodeId};
use seam_contracts::Unanswered;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::error::PeerError;
use crate::gate::{SharedAdmission, StewardDevicesGate};

/// The ALPN a device's tier answers on.
pub const TIER_ALPN: &[u8] = b"/elohim/eprfs-tier/1.0.0";

/// The default largest request frame, in bytes. A request names roots, paths and a view; a view
/// of 256 paths at `TreeLimits::max_path_bytes` (4096) fits.
pub const MAX_REQUEST_FRAME_BYTES: u32 = 1024 * 1024;

/// The largest answer frame, in bytes: four `TreeLimits::max_node_bytes` (4 MiB). The client
/// refuses an answer past it; a server never sends one larger, whatever its [`TierLimits`] say.
/// An answer past the server's cap (a leaf or a diff that large) is sent as
/// [`TierAnswer::Failed`], never truncated.
pub const MAX_ANSWER_FRAME_BYTES: u32 = 16 * 1024 * 1024;

/// The default number of streams of one connection answered at once. Further streams wait in
/// QUIC's own flow control until one finishes (RS-CON-02: bounded concurrent work).
pub const MAX_CONCURRENT_STREAMS: usize = 8;

/// The default number of admitted devices' streams in flight across every connection of one
/// [`TierProtocol`].
pub const MAX_IN_FLIGHT_STREAMS: usize = 64;

/// The default number of unadmitted streams in flight across every connection of one
/// [`TierProtocol`]: enough for a stranger to receive its refusal, too few to crowd anything.
pub const MAX_UNADMITTED_IN_FLIGHT_STREAMS: usize = 8;

/// The default time one frame may take to arrive or to leave.
pub const FRAME_DEADLINE: Duration = Duration::from_secs(10);

/// The QUIC error code a refused or timed-out frame resets its stream with.
const REFUSED_FRAME: u32 = 1;

/// The QUIC error code a stream shed at the in-flight bound is reset with.
const BUSY: u32 = 2;

/// How a [`TierProtocol`] bounds its work. `Clone` shares the two in-flight pools: every protocol
/// cloned from one set of limits draws on the same bounds.
#[derive(Debug, Clone)]
pub struct TierLimits {
    /// The tree limits the endpoint reads under.
    pub tree: TreeLimits,
    /// A request frame past this is refused and its stream reset.
    pub max_request_frame_bytes: u32,
    /// An answer past this is sent as [`TierAnswer::Failed`]. Never above
    /// [`MAX_ANSWER_FRAME_BYTES`], the client's own cap.
    pub max_answer_frame_bytes: u32,
    /// Each request frame must arrive, and each answer frame leave, within this; a stream that
    /// stalls past it is reset and its permits returned.
    pub frame_deadline: Duration,
    /// Streams of one connection answered at once; at least one.
    pub max_streams_per_connection: usize,
    /// Admitted devices' streams in flight across every connection, from accept to answer. A
    /// stream that finds no permit is reset at once (shed), before any byte of it is read.
    pub admitted_in_flight: Arc<Semaphore>,
    /// Every other node id's streams in flight across every connection, shed alike. Strangers
    /// draw only on this pool, never on the admitted one.
    pub unadmitted_in_flight: Arc<Semaphore>,
}

impl Default for TierLimits {
    fn default() -> TierLimits {
        TierLimits {
            tree: TreeLimits::default(),
            max_request_frame_bytes: MAX_REQUEST_FRAME_BYTES,
            max_answer_frame_bytes: MAX_ANSWER_FRAME_BYTES,
            frame_deadline: FRAME_DEADLINE,
            max_streams_per_connection: MAX_CONCURRENT_STREAMS,
            admitted_in_flight: Arc::new(Semaphore::new(MAX_IN_FLIGHT_STREAMS)),
            unadmitted_in_flight: Arc::new(Semaphore::new(MAX_UNADMITTED_IN_FLIGHT_STREAMS)),
        }
    }
}

/// One question to a device's tier: [`TierEndpoint`]'s read, list and diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum TierRequest {
    /// [`TierEndpoint::serve_read`]: one file or symlink and its bytes.
    Read { root: BlobCid, path: TreePath },
    /// [`TierEndpoint::serve_list`]: one directory's entries.
    List { root: BlobCid, path: TreePath },
    /// [`TierEndpoint::serve_diff`]: the changes from `old` to `new` under `view`.
    Diff {
        old: BlobCid,
        new: BlobCid,
        view: View,
    },
}

/// A device tier's answer to one [`TierRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum TierAnswer {
    Read(ReadLeaf),
    List(DirListing),
    Diff(Vec<TreeChange>),
    /// The endpoint did not answer for the object at `path` under `cid`, with ruling 7's three
    /// parts ([`TreeError::Unanswered`]). To a requester outside reach this is the uniform
    /// refusal, alike for a held root and an unknown one.
    Unanswered {
        cid: BlobCid,
        path: TreePath,
        why: Unanswered<ItemAbsence>,
    },
    /// Any other tree error, by its message, or an answer too large for one frame. Only an
    /// admitted device can receive one: the gate refuses everyone else first.
    Failed {
        reason: String,
    },
}

impl TierAnswer {
    fn from_tree<T>(answer: TreeResult<T>, wrap: impl FnOnce(T) -> TierAnswer) -> TierAnswer {
        match answer {
            Ok(value) => wrap(value),
            Err(TreeError::Unanswered { cid, path, why }) => TierAnswer::Unanswered {
                cid,
                path,
                why: *why,
            },
            Err(other) => TierAnswer::Failed {
                reason: other.to_string(),
            },
        }
    }
}

/// What a [`TierProtocol`] has seen. The answers keep apart less than this does: a stranger's
/// refusal and a missing root look the same to the stranger, not here.
#[derive(Debug, Default)]
struct TierCounters {
    requests: AtomicU64,
    unadmitted: AtomicU64,
    refused_frames: AtomicU64,
    timed_out_frames: AtomicU64,
    shed_streams: AtomicU64,
    oversized_answers: AtomicU64,
}

/// A snapshot of a [`TierProtocol`]'s counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TierCountersSnapshot {
    /// Requests decoded and answered.
    pub requests: u64,
    /// Of those, requests from a node id admission did not map to a device.
    pub unadmitted: u64,
    /// Streams reset for a request frame past the cap, one that did not decode, or a stream that
    /// failed or closed before its frame was whole.
    pub refused_frames: u64,
    /// Streams reset because a frame did not arrive or leave within the deadline.
    pub timed_out_frames: u64,
    /// Streams reset unread because the in-flight bound was full.
    pub shed_streams: u64,
    /// Answers sent as [`TierAnswer::Failed`] for exceeding the answer frame cap.
    pub oversized_answers: u64,
}

impl TierCounters {
    fn bump(counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> TierCountersSnapshot {
        let read = |c: &AtomicU64| c.load(Ordering::Relaxed);
        TierCountersSnapshot {
            requests: read(&self.requests),
            unadmitted: read(&self.unadmitted),
            refused_frames: read(&self.refused_frames),
            timed_out_frames: read(&self.timed_out_frames),
            shed_streams: read(&self.shed_streams),
            oversized_answers: read(&self.oversized_answers),
        }
    }
}

/// The tier ALPN's [`ProtocolHandler`]: a [`TierEndpoint`] over this device's own store, gated by
/// [`StewardDevicesGate`]. Mount it on a router under [`TIER_ALPN`].
///
/// It serves this store only: no upstream chain, so it never forwards a miss on a peer's behalf.
#[derive(Clone)]
pub struct TierProtocol {
    inner: Arc<Inner>,
}

struct Inner {
    gate: StewardDevicesGate,
    endpoint: TierEndpoint<StewardDevicesGate>,
    limits: TierLimits,
    counters: TierCounters,
}

impl std::fmt::Debug for TierProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TierProtocol")
            .field("self_id", self.inner.endpoint.self_id())
            .finish_non_exhaustive()
    }
}

impl TierProtocol {
    /// Serve `store` as `self_id`, answering only the devices `admission` admits. The gate and the
    /// node-id lookup share the one `admission`, so they cannot disagree. Work is bounded by
    /// `limits` ([`TierLimits::default`] for the defaults).
    ///
    /// # Errors
    ///
    /// The [`TierEndpoint::new`] refusals: an indexical `self_id`.
    pub fn new(
        self_id: TierId,
        store: Arc<dyn EprfsStorage>,
        admission: SharedAdmission,
        limits: TierLimits,
    ) -> eprfs_core::Result<TierProtocol> {
        let gate = StewardDevicesGate::new(admission);
        let endpoint = TierEndpoint::new(self_id, store, gate.clone(), None, limits.tree)?;
        Ok(TierProtocol {
            inner: Arc::new(Inner {
                gate,
                endpoint,
                limits,
                counters: TierCounters::default(),
            }),
        })
    }

    pub fn counters(&self) -> TierCountersSnapshot {
        self.inner.counters.snapshot()
    }

    /// The endpoint's own serving counters.
    pub fn serve_counters(&self) -> eprfs_serve::ServeCountersSnapshot {
        self.inner.endpoint.counters()
    }
}

impl ProtocolHandler for TierProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        // The handshake proved the peer holds this node id's key; nothing it sends names it.
        let remote = connection.remote_node_id()?;
        let per_connection = self.inner.limits.max_streams_per_connection.max(1);
        let mut streams = JoinSet::new();
        loop {
            while streams.try_join_next().is_some() {}
            while streams.len() >= per_connection {
                if streams.join_next().await.is_none() {
                    break;
                }
            }
            let Ok((mut send, mut recv)) = connection.accept_bi().await else {
                break;
            };
            // The pool is chosen by the authenticated node id's admission now, read once and fail
            // closed; the request is still judged on decode. The permit is taken before any byte
            // is read and held until the stream's task ends, on every path (a dropped or aborted
            // task drops its permit).
            let pool = if self.inner.gate.requester(&remote).agent().is_some() {
                &self.inner.limits.admitted_in_flight
            } else {
                &self.inner.limits.unadmitted_in_flight
            };
            let Ok(permit) = Arc::clone(pool).try_acquire_owned() else {
                TierCounters::bump(&self.inner.counters.shed_streams);
                // Best effort: the stream is being refused either way.
                let _ = recv.stop(VarInt::from_u32(BUSY));
                let _ = send.reset(VarInt::from_u32(BUSY));
                continue;
            };
            let inner = Arc::clone(&self.inner);
            streams.spawn(async move {
                inner.answer_stream(remote, send, recv).await;
                drop(permit);
            });
        }
        while streams.join_next().await.is_some() {}
        Ok(())
    }
}

impl Inner {
    /// Answer one stream. Stream failures end it quietly: the peer sees its stream fail.
    async fn answer_stream(&self, remote: NodeId, mut send: SendStream, mut recv: RecvStream) {
        let deadline = self.limits.frame_deadline;
        let cap = self.limits.max_request_frame_bytes;
        let read = tokio::time::timeout(deadline, read_frame::<TierRequest>(&mut recv, cap)).await;
        let request = match read {
            Ok(Ok(request)) => request,
            Ok(Err(_)) | Err(_) => {
                let counter = if read.is_err() {
                    &self.counters.timed_out_frames
                } else {
                    &self.counters.refused_frames
                };
                TierCounters::bump(counter);
                // Best effort: the stream is being abandoned either way.
                let _ = recv.stop(VarInt::from_u32(REFUSED_FRAME));
                let _ = send.reset(VarInt::from_u32(REFUSED_FRAME));
                return;
            }
        };
        TierCounters::bump(&self.counters.requests);
        // Admission is read per request, so a reload reaches an already-open connection.
        let who = self.gate.requester(&remote);
        if who.agent().is_none() {
            TierCounters::bump(&self.counters.unadmitted);
        }
        let answer = self.answer(&who, request).await;
        let cap = self
            .limits
            .max_answer_frame_bytes
            .min(MAX_ANSWER_FRAME_BYTES);
        let frame = match encode_frame(&answer, cap) {
            Ok(frame) => frame,
            Err(e) => {
                TierCounters::bump(&self.counters.oversized_answers);
                let failed = TierAnswer::Failed {
                    reason: e.to_string(),
                };
                // A failure names only the cap and the size; it fits the client's own cap.
                match encode_frame(&failed, MAX_ANSWER_FRAME_BYTES) {
                    Ok(frame) => frame,
                    Err(_) => return,
                }
            }
        };
        match tokio::time::timeout(deadline, send.write_all(&frame)).await {
            // Best effort: a peer that went away has nothing left to read.
            Ok(Ok(())) => {
                let _ = send.finish();
            }
            Ok(Err(_)) => {}
            Err(_) => {
                TierCounters::bump(&self.counters.timed_out_frames);
                let _ = send.reset(VarInt::from_u32(REFUSED_FRAME));
            }
        }
    }

    async fn answer(&self, who: &eprfs_core::Requester, request: TierRequest) -> TierAnswer {
        // No upstream: the endpoint reads its own store only, so a requester's options are moot.
        let opts = BatchOptions::default();
        let endpoint = &self.endpoint;
        match request {
            TierRequest::Read { root, path } => TierAnswer::from_tree(
                endpoint.serve_read(who, &root, &path, &opts).await,
                TierAnswer::Read,
            ),
            TierRequest::List { root, path } => TierAnswer::from_tree(
                endpoint.serve_list(who, &root, &path, &opts).await,
                TierAnswer::List,
            ),
            TierRequest::Diff { old, new, view } => TierAnswer::from_tree(
                endpoint.serve_diff(who, &old, &new, &view, &opts).await,
                TierAnswer::Diff,
            ),
        }
    }
}

/// Ask `peer`'s tier one question on a new connection, then close it.
///
/// The dialer builds `peer` itself (a relay URL or direct addresses): the roster carries node ids
/// only. The node id `peer` names is the one iroh authenticates, so the answer is from that
/// device.
///
/// # Errors
///
/// As [`tier_request`], and [`PeerError::TierConnect`] when the peer cannot be reached.
pub async fn tier_call(
    endpoint: &Endpoint,
    peer: NodeAddr,
    req: TierRequest,
    tree: &TreeLimits,
) -> Result<TierAnswer, PeerError> {
    let connection = endpoint
        .connect(peer, TIER_ALPN)
        .await
        .map_err(|e| PeerError::TierConnect(Box::new(e)))?;
    let answer = tier_request(&connection, &req, tree).await;
    connection.close(VarInt::from_u32(0), b"");
    answer
}

/// Ask one question on an open tier connection, on a stream of its own.
///
/// Read and List answers are checked here for self-consistency and for the path asked: a leaf
/// must be at the requested path and its bytes must hash to its CID; a listing must be at the
/// requested path and its entries must encode to its CID under `tree`, the caller's own tree
/// limits (a listing the caller would refuse to encode is refused here too). Anything else is an
/// error, never an answer. Membership under the requested root is not checked here: that would
/// need the spine.
///
/// # Errors
///
/// - [`PeerError::TierStream`] when the stream fails or the peer resets it.
/// - [`PeerError::TierFrame`] when the request is past its cap or the answer does not decode;
///   [`PeerError::TierFrameTooLarge`] when the answer frame is past the client's cap.
/// - [`PeerError::TierLeafIntegrity`] when a served leaf or listing fails its address or answers
///   another path.
pub async fn tier_request(
    connection: &Connection,
    req: &TierRequest,
    tree: &TreeLimits,
) -> Result<TierAnswer, PeerError> {
    tier_request_capped(connection, req, tree, MAX_ANSWER_FRAME_BYTES).await
}

/// [`tier_request`] with a smaller answer cap (never above [`MAX_ANSWER_FRAME_BYTES`]): an answer
/// past `answer_cap` is [`PeerError::TierFrameTooLarge`], refused before its body is read.
pub(crate) async fn tier_request_capped(
    connection: &Connection,
    req: &TierRequest,
    tree: &TreeLimits,
    answer_cap: u32,
) -> Result<TierAnswer, PeerError> {
    let frame = encode_frame(req, MAX_REQUEST_FRAME_BYTES)?;
    let (mut send, mut recv) = connection.open_bi().await.map_err(stream_error)?;
    send.write_all(&frame).await.map_err(stream_error)?;
    send.finish().map_err(stream_error)?;
    let cap = answer_cap.min(MAX_ANSWER_FRAME_BYTES);
    let answer: TierAnswer = read_frame(&mut recv, cap).await?;
    check_answer(req, &answer, tree)?;
    Ok(answer)
}

/// The client's checks on a served leaf or listing (see [`tier_request`]).
fn check_answer(
    req: &TierRequest,
    answer: &TierAnswer,
    tree: &TreeLimits,
) -> Result<(), PeerError> {
    let (cid, consistent) = match (req, answer) {
        (TierRequest::Read { path, .. }, TierAnswer::Read(leaf)) => (
            &leaf.cid,
            leaf.path == *path && leaf.cid.verifies(&leaf.bytes),
        ),
        (TierRequest::List { path, .. }, TierAnswer::List(listing)) => (
            &listing.cid,
            listing.path == *path && listing_cid(listing, tree).as_ref() == Some(&listing.cid),
        ),
        (TierRequest::Diff { .. }, TierAnswer::Diff(_))
        | (_, TierAnswer::Unanswered { .. } | TierAnswer::Failed { .. }) => return Ok(()),
        _ => return Err(frame_error("the answer is not of the kind asked")),
    };
    if consistent {
        Ok(())
    } else {
        Err(PeerError::TierLeafIntegrity { cid: cid.clone() })
    }
}

/// The CID a listing's entries encode to under `tree`, or `None` when they are not a valid tree
/// node within those limits.
fn listing_cid(listing: &DirListing, tree: &TreeLimits) -> Option<BlobCid> {
    let node = TreeNode::new(listing.entries.clone()).ok()?;
    encode_tree(&node, tree).ok().map(|(cid, _)| cid)
}

fn stream_error(e: impl std::error::Error + Send + Sync + 'static) -> PeerError {
    PeerError::TierStream(Box::new(e))
}

fn frame_error(reason: impl Into<String>) -> PeerError {
    PeerError::TierFrame {
        reason: reason.into(),
    }
}

/// The length prefix and DAG-CBOR body of `value`, refused past `cap` bytes of body.
fn encode_frame<T: Serialize>(value: &T, cap: u32) -> Result<Vec<u8>, PeerError> {
    let body = serde_ipld_dagcbor::to_vec(value).map_err(|e| frame_error(e.to_string()))?;
    let len = u32::try_from(body.len())
        .ok()
        .filter(|len| *len <= cap)
        .ok_or_else(|| {
            frame_error(format!(
                "a frame of {} bytes exceeds the {cap}-byte cap",
                body.len()
            ))
        })?;
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

/// The most a frame body read reserves ahead of the bytes that arrived.
const READ_CHUNK: usize = 64 * 1024;

/// Read one frame, server request or client answer alike: its length first, refused past `cap`,
/// then its body chunk by chunk as it arrives, so memory follows the bytes received, never the
/// length claimed. The caller bounds the time it may take.
async fn read_frame<T: for<'de> Deserialize<'de>>(
    recv: &mut RecvStream,
    cap: u32,
) -> Result<T, PeerError> {
    let mut header = [0u8; 4];
    recv.read_exact(&mut header).await.map_err(stream_error)?;
    let len = u32::from_be_bytes(header);
    if len > cap {
        return Err(PeerError::TierFrameTooLarge { len, cap });
    }
    let len = usize::try_from(len).map_err(|e| frame_error(e.to_string()))?;
    let mut body = Vec::with_capacity(len.min(READ_CHUNK));
    while body.len() < len {
        let want = (len - body.len()).min(READ_CHUNK);
        let chunk = recv
            .read_chunk(want, true)
            .await
            .map_err(stream_error)?
            .ok_or_else(|| frame_error("the stream ended inside a frame"))?;
        body.extend_from_slice(&chunk.bytes);
    }
    serde_ipld_dagcbor::from_slice(&body).map_err(|e| frame_error(e.to_string()))
}
