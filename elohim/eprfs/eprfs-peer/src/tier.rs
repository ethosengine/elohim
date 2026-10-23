//! The tier ALPN: a device serves its own eprfs store to the person's other admitted devices
//! (spec §3.2), and asks theirs.
//!
//! One request per bidirectional stream. Each frame is a big-endian `u32` length and then that
//! many bytes of canonical DAG-CBOR (the codec eprfs-core encodes trees with): a [`TierRequest`]
//! one way, a [`TierAnswer`] back. A length past the cap is refused before anything is
//! allocated for it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eprfs_core::{
    BatchOptions, BlobCid, DirListing, EprfsStorage, ItemAbsence, ReadLeaf, TierId, TreeChange,
    TreeError, TreeLimits, TreePath, TreeResult, View,
};
use eprfs_serve::TierEndpoint;
use iroh::endpoint::{Connection, RecvStream, SendStream, VarInt};
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh::{Endpoint, NodeAddr, NodeId};
use seam_contracts::Unanswered;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::error::PeerError;
use crate::gate::{SharedAdmission, StewardDevicesGate};

/// The ALPN a device's tier answers on.
pub const TIER_ALPN: &[u8] = b"/elohim/eprfs-tier/1.0.0";

/// The largest request frame accepted, in bytes. A request names roots, paths and a view; a view
/// of 256 paths at `TreeLimits::max_path_bytes` (4096) fits.
pub const MAX_REQUEST_FRAME_BYTES: u32 = 1024 * 1024;

/// The largest answer frame, in bytes: four `TreeLimits::max_node_bytes` (4 MiB). An answer
/// past it (a leaf or a diff that large) is sent as [`TierAnswer::Failed`], never truncated.
pub const MAX_ANSWER_FRAME_BYTES: u32 = 16 * 1024 * 1024;

/// How many streams of one connection are answered at once. Further streams wait in QUIC's own
/// flow control until one finishes (RS-CON-02: bounded concurrent work).
pub const MAX_CONCURRENT_STREAMS: usize = 8;

/// The QUIC error code a refused frame resets its stream with.
const REFUSED_FRAME: u32 = 1;

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
    oversized_answers: AtomicU64,
}

/// A snapshot of a [`TierProtocol`]'s counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TierCountersSnapshot {
    /// Requests decoded and answered.
    pub requests: u64,
    /// Of those, requests from a node id admission did not map to a device.
    pub unadmitted: u64,
    /// Streams reset for a request frame past the cap or one that did not decode.
    pub refused_frames: u64,
    /// Answers sent as [`TierAnswer::Failed`] for exceeding [`MAX_ANSWER_FRAME_BYTES`].
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
    /// node-id lookup share the one `admission`, so they cannot disagree.
    ///
    /// # Errors
    ///
    /// The [`TierEndpoint::new`] refusals: an indexical `self_id`.
    pub fn new(
        self_id: TierId,
        store: Arc<dyn EprfsStorage>,
        admission: SharedAdmission,
        limits: TreeLimits,
    ) -> eprfs_core::Result<TierProtocol> {
        let gate = StewardDevicesGate::new(admission);
        let endpoint = TierEndpoint::new(self_id, store, gate.clone(), None, limits)?;
        Ok(TierProtocol {
            inner: Arc::new(Inner {
                gate,
                endpoint,
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
        let mut streams = JoinSet::new();
        loop {
            while streams.try_join_next().is_some() {}
            while streams.len() >= MAX_CONCURRENT_STREAMS {
                streams.join_next().await;
            }
            let Ok((send, recv)) = connection.accept_bi().await else {
                break;
            };
            let inner = Arc::clone(&self.inner);
            streams.spawn(async move { inner.answer_stream(remote, send, recv).await });
        }
        while streams.join_next().await.is_some() {}
        Ok(())
    }
}

impl Inner {
    /// Answer one stream. Stream failures end it quietly: the peer sees its stream fail.
    async fn answer_stream(&self, remote: NodeId, mut send: SendStream, mut recv: RecvStream) {
        let request = match read_frame::<TierRequest>(&mut recv, MAX_REQUEST_FRAME_BYTES).await {
            Ok(request) => request,
            Err(_) => {
                TierCounters::bump(&self.counters.refused_frames);
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
        let frame = match encode_frame(&answer, MAX_ANSWER_FRAME_BYTES) {
            Ok(frame) => frame,
            Err(e) => {
                TierCounters::bump(&self.counters.oversized_answers);
                let failed = TierAnswer::Failed {
                    reason: e.to_string(),
                };
                match encode_frame(&failed, MAX_ANSWER_FRAME_BYTES) {
                    Ok(frame) => frame,
                    Err(_) => return,
                }
            }
        };
        if send.write_all(&frame).await.is_ok() {
            // Best effort: a peer that went away has nothing left to read.
            let _ = send.finish();
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
) -> Result<TierAnswer, PeerError> {
    let connection = endpoint
        .connect(peer, TIER_ALPN)
        .await
        .map_err(|e| PeerError::TierConnect(Box::new(e)))?;
    let answer = tier_request(&connection, &req).await;
    connection.close(VarInt::from_u32(0), b"");
    answer
}

/// Ask one question on an open tier connection, on a stream of its own.
///
/// A [`TierAnswer::Read`] is checked here: a leaf whose bytes fail its CID is an error, never an
/// answer.
///
/// # Errors
///
/// - [`PeerError::TierStream`] when the stream fails or the peer resets it.
/// - [`PeerError::TierFrame`] when either frame is past its cap or the answer does not decode.
/// - [`PeerError::TierLeafIntegrity`] when a served leaf fails its address.
pub async fn tier_request(
    connection: &Connection,
    req: &TierRequest,
) -> Result<TierAnswer, PeerError> {
    let frame = encode_frame(req, MAX_REQUEST_FRAME_BYTES)?;
    let (mut send, mut recv) = connection.open_bi().await.map_err(stream_error)?;
    send.write_all(&frame).await.map_err(stream_error)?;
    send.finish().map_err(stream_error)?;
    let answer: TierAnswer = read_frame(&mut recv, MAX_ANSWER_FRAME_BYTES).await?;
    if let TierAnswer::Read(leaf) = &answer {
        if !leaf.cid.verifies(&leaf.bytes) {
            return Err(PeerError::TierLeafIntegrity {
                cid: leaf.cid.clone(),
            });
        }
    }
    Ok(answer)
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

/// Read one frame: its length first, checked against `cap` before the body is allocated.
async fn read_frame<T: for<'de> Deserialize<'de>>(
    recv: &mut RecvStream,
    cap: u32,
) -> Result<T, PeerError> {
    let mut header = [0u8; 4];
    recv.read_exact(&mut header).await.map_err(stream_error)?;
    let len = u32::from_be_bytes(header);
    if len > cap {
        return Err(frame_error(format!(
            "a frame of {len} bytes exceeds the {cap}-byte cap"
        )));
    }
    let len = usize::try_from(len).map_err(|e| frame_error(e.to_string()))?;
    let mut body = vec![0u8; len];
    recv.read_exact(&mut body).await.map_err(stream_error)?;
    serde_ipld_dagcbor::from_slice(&body).map_err(|e| frame_error(e.to_string()))
}
