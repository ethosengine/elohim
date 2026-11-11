//! The sibling fetch (artifact-store design §3.b): a device draws a derivation artifact one of
//! its person's other devices holds, over the tier ALPN, instead of computing it.
//!
//! The order is fixed and every step fails closed:
//!
//! 1. **Manifest first, capped.** [`TierRequest::Manifest`] from the first holder that answers,
//!    its answer frame capped at [`FetchBudget::manifest_cap`] so an oversized manifest is refused
//!    before any chunk moves. The served bytes must be the manifest the holder's crossed record
//!    named, must decode under this reader's epoch, and must carry a record whose `key` equals the
//!    key *this* device computed and whose `output` equals the output asked: a manifest keyed for
//!    another closure is rejected however well it verifies.
//! 2. **Resume.** A chunk already held here is not asked again.
//! 3. **Race.** Missing chunks are spread round-robin across every holder, at most
//!    [`FetchBudget::in_flight_per_holder`] on one connection, each under [`FetchBudget::stall`].
//!    A stalled, refused, failed or corrupt answer abandons that holder and re-queues the chunk to
//!    the next one. Each chunk is verified by CID and length before it is stored; the bytes of a
//!    rejected answer are metered as wasted. [`FetchBudget::total`] bounds the whole fetch.
//! 4. **Manifest last.** The manifest is stored only once every chunk is held, so a held manifest
//!    always means a complete set of chunks was once here.
//! 5. **Install read.** [`install_fetched`] streams the artifact out of the local store with
//!    [`stream_local_blob_with`], whose whole-blob verify is the install read and whose one `Use`
//!    event records the draw (`draw:hit`, `copy:self`, `basis:sibling`, `fetched-from:<did>`,
//!    `wasted:<n>`); only a verified stream is renamed into place.
//!
//! One connection per holder carries every request of one artifact, because a holder's
//! `serve-blob` session closes with its connection (E3): one artifact drawn from one holder is one
//! `draw:completed` Use on that holder, never one partial per chunk.
//!
//! Every way this does not end in an installed artifact is a [`FetchGap`] with a printable reason;
//! the caller builds locally. No record crosses here: holders are found from the
//! `artifact-produce@1` Produce events folded from the person's other devices, which crossed on the
//! existing channel; only bytes move on the tier ALPN.
//!
//! The received manifest bytes are stored exactly as they verified against their CID and are
//! never re-encoded, so a record extension (`ext`) a newer writer put inside the manifest's record
//! keeps its bytes, and the manifest keeps its CID, on every device it reaches (follow-up F-E2-1).

use std::collections::{BTreeSet, VecDeque};
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use bytes::Bytes;
use elohim_epr_rea::FlowStore;
use eprfs_core::{
    BlobCid, BlobPresence, DerivationKey, EprfsStorage, ItemAbsence, MethodDescriptor,
};
use eprfs_storage::{
    read_tier_manifest, MemoryStorage, TierBlobChunk, TierBlobManifest, TierBlobSink,
    TierBlobStreamDraw,
};
use iroh::endpoint::{Connection, VarInt};
use iroh::{Endpoint, NodeAddr};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::error::PeerError;
use crate::local_blob::{stream_local_blob_with, LocalBlobContext};
use crate::tier::{tier_request_counted, TierAnswer, TierRequest, FRAME_DEADLINE, TIER_ALPN};

/// The crossing measure an artifact's custody Produce carries, so it reaches the person's other
/// devices (`berth.channel.crosses` in `pool-policy.json`).
pub const ARTIFACT_PRODUCE_MEASURE: &str = "artifact-produce@1";

/// The one kind of [`ARTIFACT_PRODUCE_MEASURE`] that crosses.
pub const ARTIFACT_PRODUCE_KIND: &str = "produce";

/// The directory, under a daemon's state directory, that `epr artifact fetch` queues requests in.
pub const FETCH_QUEUE: &str = "fetch-queue";

/// The default per-request stall bound.
pub const FETCH_STALL: Duration = Duration::from_secs(30);

/// The default bound on a whole fetch.
pub const FETCH_TOTAL: Duration = Duration::from_secs(1800);

/// The default manifest cap: 1 MiB admits an artifact of about 13 GB at the 1 MiB average chunk.
pub const FETCH_MANIFEST_CAP: u32 = 1024 * 1024;

/// The default chunk requests in flight on one holder's connection: the tier answers eight
/// streams of one connection at once.
pub const FETCH_IN_FLIGHT: usize = 8;

/// What a chunk answer frame may hold beyond the chunk's bytes: the answer's tag and its CID.
const CHUNK_FRAME_OVERHEAD: u32 = 4096;

/// How the bytes are to move.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FetchTransport {
    /// The tier ALPN: direct addresses, or the person's relay.
    #[default]
    Tier,
    /// Pure iroh-blobs. It has no byte path this sprint (`NoBytePath` by design), so it is always
    /// [`FetchGap::TransportCannotDeliver`].
    IrohBlobs,
}

impl FetchTransport {
    /// The mode's name, as a gap reports it.
    pub fn name(self) -> &'static str {
        match self {
            FetchTransport::Tier => "tier",
            FetchTransport::IrohBlobs => "iroh-blobs",
        }
    }
}

/// How long and how much a fetch may take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchBudget {
    /// Each request (and each dial) must be answered within this; a stalled request abandons
    /// its holder and its chunk is re-raced.
    pub stall: Duration,
    /// The whole fetch must finish within this.
    pub total: Duration,
    /// The largest manifest answer frame accepted, in bytes.
    pub manifest_cap: u32,
    /// Chunk requests in flight on one holder's connection.
    pub in_flight_per_holder: usize,
    /// How the bytes are to move.
    pub transport: FetchTransport,
}

impl Default for FetchBudget {
    fn default() -> FetchBudget {
        FetchBudget {
            stall: FETCH_STALL,
            total: FETCH_TOTAL,
            manifest_cap: FETCH_MANIFEST_CAP,
            in_flight_per_holder: FETCH_IN_FLIGHT,
            transport: FetchTransport::Tier,
        }
    }
}

/// One device holding the artifact: its did:key, how to dial it, and the manifest its crossed
/// custody record names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub device: String,
    pub addr: NodeAddr,
    pub manifest: BlobCid,
}

/// A completed fetch: every chunk is held here and the manifest was stored last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedArtifact {
    pub manifest_cid: BlobCid,
    pub manifest: TierBlobManifest,
    /// The device whose manifest was accepted.
    pub manifest_from: String,
    /// Every device that served at least one chunk (or the manifest).
    pub fetched_from: BTreeSet<String>,
    /// Distinct chunks moved in this fetch.
    pub chunks_fetched: usize,
    /// Distinct chunks already held here, not asked again.
    pub chunks_resumed: usize,
    /// Chunk bytes moved and stored.
    pub bytes_fetched: u64,
    /// Bytes received and discarded: answers that failed their CID or length.
    pub wasted: u64,
    pub elapsed: Duration,
}

/// Why a fetch did not end in an installed artifact. Every one is printable, and every one means
/// the caller builds locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "gap",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum FetchGap {
    /// No admitted sibling has recorded a build for this key.
    NoRecord,
    /// No expected output, own output or strict plurality resolves the differing outputs.
    Mismatch { outputs: Vec<BlobCid> },
    /// The explicitly requested output has no holder for this key.
    ExpectedOutputUnavailable { output: BlobCid },
    /// The holder could not be dialled, or its connection failed.
    NoRoute { device: String },
    /// The holder refused or did not answer what was asked.
    Refused { device: String, refusal: String },
    /// The manifest is past the cap; refused before any chunk moved.
    ManifestTooLarge { len: u32, cap: u32 },
    /// The manifest is not the one recorded, does not decode, or names another key or output.
    ManifestRejected { reason: String },
    /// The total deadline passed: `fetched` of `total` bytes are held here.
    DeadlineExceeded { fetched: u64, total: u64 },
    /// A chunk, or the installed whole, failed its address, and no holder remains to re-fetch it.
    IntegrityFailed { cid: BlobCid },
    /// No `epr peer serve` holds this device's key, so nothing can dial a sibling.
    DaemonDown,
    /// The transport cannot carry the bytes (`relay` too slow for a 4 MiB frame per 10 s;
    /// `iroh-blobs` has no byte path this sprint).
    TransportCannotDeliver { mode: String },
    /// This device's own payload store or flow store failed.
    LocalStore { reason: String },
    /// The destination is not one this daemon installs into, or the request does not read.
    InstallRefused { dest: String, reason: String },
    /// The requester withdrew the fetch before it finished.
    Cancelled,
}

impl FetchGap {
    /// The gap's name, as the CLI prints it before its reason.
    pub fn kind(&self) -> &'static str {
        match self {
            FetchGap::NoRecord => "no-record",
            FetchGap::Mismatch { .. } => "mismatch",
            FetchGap::ExpectedOutputUnavailable { .. } => "expected-output-unavailable",
            FetchGap::NoRoute { .. } => "no-route",
            FetchGap::Refused { .. } => "refused",
            FetchGap::ManifestTooLarge { .. } => "manifest-too-large",
            FetchGap::ManifestRejected { .. } => "manifest-rejected",
            FetchGap::DeadlineExceeded { .. } => "deadline-exceeded",
            FetchGap::IntegrityFailed { .. } => "integrity-failed",
            FetchGap::DaemonDown => "daemon-down",
            FetchGap::TransportCannotDeliver { .. } => "transport-cannot-deliver",
            FetchGap::LocalStore { .. } => "local-store",
            FetchGap::InstallRefused { .. } => "install-refused",
            FetchGap::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for FetchGap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchGap::NoRecord => f.write_str(
                "no admitted sibling device has recorded a build for this key; build here",
            ),
            FetchGap::Mismatch { outputs } => write!(
                f,
                "admitted siblings recorded {} different outputs for this key ({}); no \
                 expected output, own output or strict plurality decides; build here",
                outputs.len(),
                outputs
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            FetchGap::ExpectedOutputUnavailable { output } => {
                write!(
                    f,
                    "no holder of expected output {output} for this key; build here"
                )
            }
            FetchGap::NoRoute { device } => {
                write!(f, "the holder {device} could not be reached")
            }
            FetchGap::Refused { device, refusal } => {
                write!(f, "the holder {device} refused: {refusal}")
            }
            FetchGap::ManifestTooLarge { len, cap } => write!(
                f,
                "the manifest is {len} bytes, past the {cap}-byte cap; no chunk moved"
            ),
            FetchGap::ManifestRejected { reason } => write!(f, "manifest rejected: {reason}"),
            FetchGap::DeadlineExceeded { fetched, total } => write!(
                f,
                "the fetch deadline passed with {fetched} of {total} bytes held here; a later \
                 fetch resumes from them"
            ),
            FetchGap::IntegrityFailed { cid } => write!(
                f,
                "{cid} failed its address and no holder remains to fetch it again"
            ),
            FetchGap::DaemonDown => f.write_str(
                "no `epr peer serve` is running for this device, and only the daemon holding the \
                 device's peer key can dial a sibling; cure: start `epr peer serve` on this \
                 device's checkout (or build here)",
            ),
            FetchGap::TransportCannotDeliver { mode } => write!(
                f,
                "the {mode} transport cannot deliver this artifact (relay: below one 4 MiB frame \
                 per 10 s; iroh-blobs: no byte path this sprint)"
            ),
            FetchGap::LocalStore { reason } => write!(f, "this device's store failed: {reason}"),
            FetchGap::InstallRefused { dest, reason } => {
                write!(f, "the daemon refused to install at {dest}: {reason}")
            }
            FetchGap::Cancelled => f.write_str("the fetch was withdrawn before it finished"),
        }
    }
}

/// The derivation key's CID: the DAG-CBOR CID of the key itself. It is the `inputs_hash` a build
/// attestation records and the `key` a crossing record names; never a sha256 hex.
///
/// # Errors
///
/// [`PeerError::ChannelTree`] when the key does not encode.
pub fn key_cid(key: &DerivationKey) -> Result<BlobCid, PeerError> {
    let bytes = serde_ipld_dagcbor::to_vec(key).map_err(|e| PeerError::ChannelTree {
        reason: format!("derivation key encoding: {e}"),
    })?;
    Ok(BlobCid::compute(&bytes))
}

/// Store `descriptor`'s canonical record under its CID in `store`, so a mount indexing an artifact
/// of this method can read the method it names (E3 fails closed without it). The view is
/// normalized first, so the stored bytes hash to [`MethodDescriptor::cid`].
///
/// # Errors
///
/// [`PeerError::ChannelTree`] when the descriptor does not encode or the store refuses it.
pub async fn put_method_record<S: EprfsStorage + ?Sized>(
    store: &S,
    descriptor: &MethodDescriptor,
) -> Result<BlobCid, PeerError> {
    let failed = |reason: String| PeerError::ChannelTree { reason };
    let mut canonical = descriptor.clone();
    canonical.view = descriptor.view().paths().cloned().collect();
    let cid = descriptor.cid().map_err(|e| failed(e.to_string()))?;
    let bytes = serde_ipld_dagcbor::to_vec(&canonical)
        .map_err(|e| failed(format!("method descriptor encoding: {e}")))?;
    if BlobCid::compute(&bytes) != cid {
        return Err(failed(
            "the method descriptor's canonical bytes do not hash to its CID".into(),
        ));
    }
    store
        .put_blob_verified(&cid, Bytes::from(bytes))
        .await
        .map_err(|e| failed(e.to_string()))?;
    Ok(cid)
}

/// Why a holder was abandoned in the race.
#[derive(Debug, Clone)]
enum Abandon {
    Stalled,
    Stream,
    Refused(String),
    Integrity(BlobCid),
}

/// One holder's connection in the race.
struct Link {
    device: String,
    connection: Connection,
    relay_only: bool,
    in_flight: usize,
    abandoned: Option<Abandon>,
}

/// What one chunk request came back with.
type ChunkOutcome = Result<Result<TierAnswer, PeerError>, tokio::time::error::Elapsed>;

fn relay_only(addr: &NodeAddr) -> bool {
    addr.direct_addresses().next().is_none() && addr.relay_url().is_some()
}

fn refusal_text(why: &ItemAbsence) -> String {
    match why {
        ItemAbsence::Refused { refusal, .. } => match &refusal.needs_standing {
            Some(standing) => format!("{:?} (needs {standing})", refusal.reason),
            None => format!("{:?}", refusal.reason),
        },
        ItemAbsence::NotHeld { .. } => "not held there".into(),
        other => format!("{other:?}"),
    }
}

/// The relay floor (design §3.b.5): over a relay, an answer of `len` bytes must arrive within one
/// [`FRAME_DEADLINE`] per 4 MiB (at least one), measured here on the client, not trusted to the
/// holder's own write deadline.
fn relay_bound(len: u64) -> Duration {
    let frames = len.div_ceil(RELAY_FRAME_BYTES).max(1);
    FRAME_DEADLINE.saturating_mul(u32::try_from(frames).unwrap_or(u32::MAX))
}

/// The bytes one frame deadline must carry over a relay: 4 MiB.
const RELAY_FRAME_BYTES: u64 = 4 * 1024 * 1024;

/// How long one request or dial may take, and which limit set that: the holder's own bound (the
/// stall bound, tightened to the relay floor on a relay-only path) or the fetch's deadline. When
/// it elapses, the limit that set it is the one that fired, so a deadline-cut request is the
/// deadline's gap and a holder's own stall is the holder's: no tolerance window decides.
#[derive(Debug, Clone, Copy)]
struct Bound {
    wait: Duration,
    by_deadline: bool,
}

fn request_bound(relay: bool, len: u64, budget: &FetchBudget, deadline: Instant) -> Bound {
    let own = if relay {
        budget.stall.min(relay_bound(len))
    } else {
        budget.stall
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining <= own {
        Bound {
            wait: remaining,
            by_deadline: true,
        }
    } else {
        Bound {
            wait: own,
            by_deadline: false,
        }
    }
}

/// Why a dial did not connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialFailed {
    /// The endpoint answered with an error.
    Refused,
    /// The bound elapsed, set by the fetch's deadline or not.
    Elapsed { by_deadline: bool },
}

async fn dial(
    endpoint: &Endpoint,
    addr: &NodeAddr,
    bound: Bound,
) -> Result<Connection, DialFailed> {
    match tokio::time::timeout(bound.wait, endpoint.connect(addr.clone(), TIER_ALPN)).await {
        Ok(Ok(connection)) => Ok(connection),
        Ok(Err(_)) => Err(DialFailed::Refused),
        Err(_) => Err(DialFailed::Elapsed {
            by_deadline: bound.by_deadline,
        }),
    }
}

/// Decode `bytes` (which hashed to `cid`) as a tier manifest under this reader's rules, through a
/// scratch store: the device's own store receives the manifest only last.
async fn decode_manifest(cid: &BlobCid, bytes: &Bytes) -> Result<TierBlobManifest, String> {
    let scratch = MemoryStorage::default();
    scratch.insert_blob(cid.clone(), bytes.clone()).await;
    read_tier_manifest(&scratch, cid)
        .await
        .map_err(|why| why.to_string())
}

/// The accepted manifest and the connection it came on.
struct Accepted {
    holder: usize,
    connection: Connection,
    cid: BlobCid,
    bytes: Bytes,
    manifest: TierBlobManifest,
}

/// What a fetch has moved so far, readable when its deadline cancels it.
#[derive(Default)]
struct Progress {
    /// Bytes of the artifact held here (resumed plus fetched).
    held: AtomicU64,
    /// The artifact's length, once its manifest is accepted.
    total: AtomicU64,
    /// Bytes received and discarded.
    wasted: AtomicU64,
}

/// One fetch's budget, absolute deadline and progress.
struct Bounds<'a> {
    budget: &'a FetchBudget,
    deadline: Instant,
    progress: &'a Progress,
}

/// Ask the holders for the manifest in order, until one serves a manifest this device accepts.
async fn accept_manifest(
    endpoint: &Endpoint,
    key: &DerivationKey,
    output: &BlobCid,
    holders: &[Holder],
    bounds: &Bounds<'_>,
    unreachable: &mut BTreeSet<usize>,
) -> Result<Accepted, FetchGap> {
    let Bounds {
        budget,
        deadline,
        progress,
    } = *bounds;
    let mut last = FetchGap::NoRecord;
    for (index, holder) in holders.iter().enumerate() {
        let relay = relay_only(&holder.addr);
        let bound = request_bound(relay, u64::from(budget.manifest_cap), budget, deadline);
        let dialled = dial(endpoint, &holder.addr, bound).await;
        if matches!(dialled, Err(DialFailed::Elapsed { by_deadline: true })) {
            return Err(FetchGap::DeadlineExceeded {
                fetched: 0,
                total: 0,
            });
        }
        let Ok(connection) = dialled else {
            // Unreachable is a route gap; only a reached relay too slow for a frame is the
            // transport's.
            unreachable.insert(index);
            last = FetchGap::NoRoute {
                device: holder.device.clone(),
            };
            continue;
        };
        let request = TierRequest::Manifest {
            artifact: output.clone(),
        };
        let received = AtomicU64::new(0);
        let bound = request_bound(relay, u64::from(budget.manifest_cap), budget, deadline);
        let answer = tokio::time::timeout(
            bound.wait,
            tier_request_counted(&connection, &request, budget.manifest_cap, &received),
        )
        .await;
        let rejected = |reason: String| FetchGap::ManifestRejected { reason };
        let gap = match answer {
            Ok(Ok(TierAnswer::Blob { cid, bytes })) if !cid.verifies(&bytes) => rejected(format!(
                "{} served bytes that do not hash to {cid}",
                holder.device
            )),
            Ok(Ok(TierAnswer::Blob { cid, bytes })) => {
                if cid != holder.manifest {
                    rejected(format!(
                        "{} served manifest {cid}; its crossed record names {}",
                        holder.device, holder.manifest
                    ))
                } else {
                    match decode_manifest(&cid, &bytes).await {
                        Err(reason) => rejected(format!("{cid}: {reason}")),
                        Ok(manifest) if manifest.record.key != *key => rejected(format!(
                            "{cid} is keyed for another closure (method {}, input {}), not the \
                             one computed here",
                            manifest.record.key.method,
                            manifest.record.key.input.cid()
                        )),
                        Ok(manifest) if manifest.record.output != *output => rejected(format!(
                            "{cid} names output {}, not {output}",
                            manifest.record.output
                        )),
                        Ok(manifest) => {
                            return Ok(Accepted {
                                holder: index,
                                connection,
                                cid,
                                bytes,
                                manifest,
                            })
                        }
                    }
                }
            }
            Ok(Ok(TierAnswer::Unanswered { why, .. })) => FetchGap::Refused {
                device: holder.device.clone(),
                refusal: refusal_text(&why.why),
            },
            Ok(Ok(TierAnswer::Failed { reason })) => FetchGap::Refused {
                device: holder.device.clone(),
                refusal: reason,
            },
            Ok(Ok(_)) => FetchGap::Refused {
                device: holder.device.clone(),
                refusal: "answered another kind than a manifest".into(),
            },
            Ok(Err(PeerError::TierFrameTooLarge { len, cap })) => {
                FetchGap::ManifestTooLarge { len, cap }
            }
            Err(_) if bound.by_deadline => {
                return Err(FetchGap::DeadlineExceeded {
                    fetched: 0,
                    total: 0,
                })
            }
            Ok(Err(_)) | Err(_) => {
                if relay {
                    FetchGap::TransportCannotDeliver {
                        mode: "relay".into(),
                    }
                } else {
                    FetchGap::NoRoute {
                        device: holder.device.clone(),
                    }
                }
            }
        };
        // Whatever arrived for a manifest that was not accepted was moved for nothing.
        progress
            .wasted
            .fetch_add(received.load(Ordering::Relaxed), Ordering::Relaxed);
        connection.close(VarInt::from_u32(0), b"");
        unreachable.insert(index);
        last = gap;
    }
    Err(last)
}

/// Fetch the artifact `output` derived under `key` from `holders` into `store` (design §3.b.3),
/// within `budget.total` from now. The install read is [`install_fetched`]'s.
///
/// # Errors
///
/// A [`FetchGap`] for every outcome that does not leave every chunk and then the manifest held
/// here. Chunks already stored stay stored: they are verified content-addressed custody, and the
/// next fetch resumes from them.
pub async fn fetch_artifact<S: EprfsStorage + ?Sized>(
    endpoint: &Endpoint,
    store: &S,
    key: &DerivationKey,
    output: &BlobCid,
    holders: &[Holder],
    budget: &FetchBudget,
) -> Result<FetchedArtifact, FetchGap> {
    let deadline = Instant::now() + budget.total;
    fetch_artifact_until(endpoint, store, key, output, holders, budget, deadline).await
}

/// [`fetch_artifact`] under one absolute `deadline`: every dial, request, store write and resume
/// check falls inside it, and when it passes the fetch is dropped where it stands (`total` is then
/// how long it had, measured from the caller's start).
///
/// # Errors
///
/// As [`fetch_artifact`]; [`FetchGap::DeadlineExceeded`] when `deadline` passes.
pub async fn fetch_artifact_until<S: EprfsStorage + ?Sized>(
    endpoint: &Endpoint,
    store: &S,
    key: &DerivationKey,
    output: &BlobCid,
    holders: &[Holder],
    budget: &FetchBudget,
    deadline: Instant,
) -> Result<FetchedArtifact, FetchGap> {
    if budget.transport == FetchTransport::IrohBlobs {
        return Err(FetchGap::TransportCannotDeliver {
            mode: FetchTransport::IrohBlobs.name().into(),
        });
    }
    if holders.is_empty() {
        return Err(FetchGap::NoRecord);
    }
    let progress = Progress::default();
    let bounds = Bounds {
        budget,
        deadline,
        progress: &progress,
    };
    let fetching = fetch_within(endpoint, store, key, output, holders, &bounds);
    match tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), fetching).await {
        Ok(outcome) => outcome,
        Err(_) => Err(FetchGap::DeadlineExceeded {
            fetched: progress.held.load(Ordering::Relaxed),
            total: progress.total.load(Ordering::Relaxed),
        }),
    }
}

async fn fetch_within<S: EprfsStorage + ?Sized>(
    endpoint: &Endpoint,
    store: &S,
    key: &DerivationKey,
    output: &BlobCid,
    holders: &[Holder],
    bounds: &Bounds<'_>,
) -> Result<FetchedArtifact, FetchGap> {
    let Bounds {
        budget,
        deadline,
        progress,
    } = *bounds;
    let started = Instant::now();
    let mut unreachable = BTreeSet::new();
    let Accepted {
        holder: manifest_holder,
        connection,
        cid: manifest_cid,
        bytes: manifest_bytes,
        manifest,
    } = accept_manifest(endpoint, key, output, holders, bounds, &mut unreachable).await?;
    progress.total.store(manifest.byte_len, Ordering::Relaxed);

    // Resume: only the distinct chunks not held here are asked for.
    let mut queue: VecDeque<TierBlobChunk> = VecDeque::new();
    let mut seen = BTreeSet::new();
    let mut resumed = 0usize;
    for chunk in &manifest.chunks {
        if !seen.insert(chunk.cid.clone()) {
            continue;
        }
        match store.has_blob(&chunk.cid).await {
            Ok(BlobPresence::Local) => resumed += 1,
            _ => queue.push_back(chunk.clone()),
        }
    }
    let held_bytes: u64 = manifest
        .chunks
        .iter()
        .filter(|chunk| !queue.iter().any(|missing| missing.cid == chunk.cid))
        .map(|chunk| u64::from(chunk.byte_len))
        .sum();
    progress.held.store(held_bytes, Ordering::Relaxed);

    // One connection per holder for the whole artifact: the manifest's, then every other holder
    // not already found unreachable or refusing, dialled together within the deadline.
    let mut links = vec![Link {
        device: holders[manifest_holder].device.clone(),
        connection,
        relay_only: relay_only(&holders[manifest_holder].addr),
        in_flight: 0,
        abandoned: None,
    }];
    if !queue.is_empty() {
        let mut dials = JoinSet::new();
        for (index, holder) in holders.iter().enumerate() {
            if index == manifest_holder || unreachable.contains(&index) {
                continue;
            }
            let (endpoint, addr) = (endpoint.clone(), holder.addr.clone());
            let bound = request_bound(false, 0, budget, deadline);
            dials.spawn(async move { (index, dial(&endpoint, &addr, bound).await.ok()) });
        }
        let mut dialled = Vec::new();
        while let Some(joined) = dials.join_next().await {
            if let Ok((index, Some(connection))) = joined {
                dialled.push((index, connection));
            }
        }
        dialled.sort_by_key(|(index, _)| *index);
        for (index, connection) in dialled {
            links.push(Link {
                device: holders[index].device.clone(),
                connection,
                relay_only: relay_only(&holders[index].addr),
                in_flight: 0,
                abandoned: None,
            });
        }
    }

    let cap = budget.in_flight_per_holder.max(1);
    let mut fetched_from = BTreeSet::from([holders[manifest_holder].device.clone()]);
    let mut tasks: JoinSet<(usize, TierBlobChunk, ChunkOutcome, u64, bool)> = JoinSet::new();
    let mut next = 0usize;
    let (mut chunks_fetched, mut bytes_fetched) = (0usize, 0u64);
    let close_all = |links: &[Link]| {
        for link in links {
            link.connection.close(VarInt::from_u32(0), b"");
        }
    };

    let outcome = loop {
        // Fill every live link up to its cap, round-robin, from the front of the queue.
        while !queue.is_empty() {
            let live: Vec<usize> = (0..links.len())
                .map(|offset| (next + offset) % links.len())
                .filter(|&i| links[i].abandoned.is_none() && links[i].in_flight < cap)
                .collect();
            let Some(&chosen) = live.first() else {
                break;
            };
            next = (chosen + 1) % links.len();
            let Some(chunk) = queue.pop_front() else {
                break;
            };
            links[chosen].in_flight += 1;
            let connection = links[chosen].connection.clone();
            let bound = request_bound(
                links[chosen].relay_only,
                u64::from(chunk.byte_len),
                budget,
                deadline,
            );
            // A chunk is asked under the artifact (its output), never alone.
            let request = TierRequest::Blob {
                artifact: output.clone(),
                cid: chunk.cid.clone(),
            };
            tasks.spawn(async move {
                let frame_cap = chunk.byte_len.saturating_add(CHUNK_FRAME_OVERHEAD);
                let received = AtomicU64::new(0);
                let answer = tokio::time::timeout(
                    bound.wait,
                    tier_request_counted(&connection, &request, frame_cap, &received),
                )
                .await;
                let received = received.load(Ordering::Relaxed);
                (chosen, chunk, answer, received, bound.by_deadline)
            });
        }
        if tasks.is_empty() {
            if queue.is_empty() {
                break Ok(());
            }
            break Err(exhausted(&links, holders, manifest_holder));
        }
        let Some(joined) = tasks.join_next().await else {
            continue;
        };
        let Ok((index, chunk, answer, received, by_deadline)) = joined else {
            // A request task cannot panic short of a bug; its chunk is unaccounted for, so the
            // fetch stops rather than install a partial set.
            break Err(FetchGap::LocalStore {
                reason: "a chunk request task failed".into(),
            });
        };
        links[index].in_flight -= 1;
        let abandon = |links: &mut Vec<Link>, why: Abandon| {
            if links[index].abandoned.is_none() {
                links[index].abandoned = Some(why);
            }
        };
        let discard = |received: u64| {
            progress.wasted.fetch_add(received, Ordering::Relaxed);
        };
        match answer {
            Ok(Ok(TierAnswer::Blob { cid, bytes })) => {
                // Checked here against the manifest: the object asked, its address, its length.
                if cid != chunk.cid
                    || !chunk.cid.verifies(&bytes)
                    || bytes.len() != chunk.byte_len as usize
                {
                    discard(received);
                    abandon(&mut links, Abandon::Integrity(chunk.cid.clone()));
                    queue.push_front(chunk);
                    continue;
                }
                if let Err(e) = store.put_blob_verified(&chunk.cid, bytes).await {
                    break Err(FetchGap::LocalStore {
                        reason: e.to_string(),
                    });
                }
                fetched_from.insert(links[index].device.clone());
                chunks_fetched += 1;
                bytes_fetched += u64::from(chunk.byte_len);
                progress
                    .held
                    .fetch_add(u64::from(chunk.byte_len), Ordering::Relaxed);
            }
            Ok(Ok(TierAnswer::Unanswered { why, .. })) => {
                discard(received);
                abandon(&mut links, Abandon::Refused(refusal_text(&why.why)));
                queue.push_front(chunk);
            }
            Ok(Ok(TierAnswer::Failed { reason })) => {
                discard(received);
                abandon(&mut links, Abandon::Refused(reason));
                queue.push_front(chunk);
            }
            Ok(Ok(_)) | Ok(Err(_)) => {
                discard(received);
                abandon(&mut links, Abandon::Stream);
                queue.push_front(chunk);
            }
            Err(_) => {
                // Stalled: whatever had arrived before the bound is discarded with the request.
                discard(received);
                if by_deadline {
                    // The fetch's own deadline set this bound and fired: not the holder's pace.
                    break Err(FetchGap::DeadlineExceeded {
                        fetched: progress.held.load(Ordering::Relaxed),
                        total: manifest.byte_len,
                    });
                }
                abandon(&mut links, Abandon::Stalled);
                queue.push_front(chunk);
            }
        }
    };
    close_all(&links);
    outcome?;

    // Every chunk is held: only now the manifest, exactly as it verified.
    store
        .put_blob_verified(&manifest_cid, manifest_bytes)
        .await
        .map_err(|e| FetchGap::LocalStore {
            reason: e.to_string(),
        })?;
    Ok(FetchedArtifact {
        manifest_cid,
        manifest,
        manifest_from: holders[manifest_holder].device.clone(),
        fetched_from,
        chunks_fetched,
        chunks_resumed: resumed,
        bytes_fetched,
        wasted: progress.wasted.load(Ordering::Relaxed),
        elapsed: started.elapsed(),
    })
}

/// The gap when no live holder remains and chunks are still missing, named by why the holders
/// were abandoned: corrupt bytes first, then a relay that could not carry the frames, then a
/// refusal, then the holder that stopped answering.
fn exhausted(links: &[Link], holders: &[Holder], manifest_holder: usize) -> FetchGap {
    let reasons: Vec<(&Link, &Abandon)> = links
        .iter()
        .filter_map(|link| link.abandoned.as_ref().map(|why| (link, why)))
        .collect();
    if let Some((_, Abandon::Integrity(cid))) = reasons
        .iter()
        .find(|(_, why)| matches!(why, Abandon::Integrity(_)))
    {
        return FetchGap::IntegrityFailed { cid: cid.clone() };
    }
    let carried = |why: &Abandon| matches!(why, Abandon::Stalled | Abandon::Stream);
    if !reasons.is_empty()
        && reasons
            .iter()
            .all(|(link, why)| link.relay_only && carried(why))
    {
        return FetchGap::TransportCannotDeliver {
            mode: "relay".into(),
        };
    }
    if let Some((link, Abandon::Refused(refusal))) = reasons
        .iter()
        .find(|(_, why)| matches!(why, Abandon::Refused(_)))
    {
        return FetchGap::Refused {
            device: link.device.clone(),
            refusal: refusal.clone(),
        };
    }
    let device = reasons.last().map_or_else(
        || holders[manifest_holder].device.clone(),
        |(link, _)| link.device.clone(),
    );
    FetchGap::NoRoute { device }
}

/// The caller's file, one verified chunk at a time.
struct FileSink {
    file: std::io::BufWriter<std::fs::File>,
}

impl TierBlobSink for FileSink {
    fn chunk(&mut self, _chunk: &TierBlobChunk, bytes: Bytes) -> std::io::Result<()> {
        self.file.write_all(&bytes)
    }
}

/// A fresh staging directory beside the destination, on its filesystem: created exclusively
/// (`mkdir` fails on any existing name), mode 0700, named `.fetch-<destination name>.<128 random
/// bits>`, and removed with everything in it when dropped, whatever path the install took.
///
/// It holds an advisory lock on its own `lock` file for as long as it lives, so a later install
/// of the same destination can tell a live staging directory from one a killed run left behind
/// ([`sweep_stale_staging`]); the kernel releases the lock however the process ends.
struct Staging {
    dir: PathBuf,
    _lock: std::fs::File,
}

/// The file inside a staging directory its installer holds locked while it lives.
const STAGING_LOCK: &str = "lock";

/// How long a staging directory without its lock file is presumed to be in its first moment
/// (created, lock not yet taken) rather than orphaned.
const STAGING_LOCKLESS_GRACE: Duration = Duration::from_secs(60);

fn staging_prefix(name: &std::ffi::OsStr) -> String {
    format!(".fetch-{}.", name.to_string_lossy())
}

impl Staging {
    fn beside(parent: &Path, name: &std::ffi::OsStr) -> std::io::Result<Staging> {
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        let mut last = None;
        for _ in 0..8 {
            let dir = parent.join(format!(
                "{}{:016x}{:016x}",
                staging_prefix(name),
                rand::random::<u64>(),
                rand::random::<u64>()
            ));
            match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
                Ok(()) => {
                    let lock = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(0o600)
                        .open(dir.join(STAGING_LOCK))?;
                    lock.try_lock().map_err(std::io::Error::other)?;
                    return Ok(Staging { dir, _lock: lock });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or_else(|| std::io::Error::other("no staging name was free")))
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        // Best effort: an install that published renamed its file out already.
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Remove the staging directories a killed install of `name` left in `parent` (review item 12),
/// before a new install stages its own. Lock based: a directory is removed only when nobody holds
/// its lock now (a live installer always does), or, lacking a lock file, when it is older than
/// [`STAGING_LOCKLESS_GRACE`]. Only directories named for this destination are looked at, never
/// followed through a link. Returns how many were removed.
fn sweep_stale_staging(parent: &Path, name: &std::ffi::OsStr) -> usize {
    let prefix = staging_prefix(name);
    let Ok(entries) = std::fs::read_dir(parent) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.filter_map(Result::ok) {
        let file_name = entry.file_name();
        let Some(rest) = file_name.to_str().and_then(|n| n.strip_prefix(&prefix)) else {
            continue;
        };
        if rest.len() != 32 || !rest.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let dir = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&dir) else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }
        let lock_path = dir.join(STAGING_LOCK);
        let orphaned = match std::fs::symlink_metadata(&lock_path) {
            Ok(lock_meta) if lock_meta.is_file() => std::fs::OpenOptions::new()
                .write(true)
                .open(&lock_path)
                .is_ok_and(|lock| lock.try_lock().is_ok()),
            Ok(_) => false,
            Err(_) => meta
                .modified()
                .ok()
                .and_then(|at| at.elapsed().ok())
                .is_some_and(|age| age >= STAGING_LOCKLESS_GRACE),
        };
        if orphaned && std::fs::remove_dir_all(&dir).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// The (device, inode) of an open file or a path that is a regular file, never a link.
fn identity(meta: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (meta.dev(), meta.ino())
}

/// The install read (design §3.b.3): stream the fetched artifact out of `store` into `dest` through
/// [`stream_local_blob_with`], whose incremental whole-blob verify against the record's output is
/// what admits the bytes, and whose one `Use` event records the draw: `draw:hit`, `copy:self`,
/// `basis:sibling`, `fetched-from:<did>` (one per device that served), `wasted:<n>`.
///
/// The bytes are staged in a file created exclusively (`create_new`) in a fresh private directory
/// beside `dest` (see [`Staging`]), so no precreated name or link can receive them. Only a
/// completed, verified stream is published: the file is made executable (0755) and fsynced through
/// its own descriptor, its path is checked to still name that inode, it is renamed onto `dest`,
/// `dest` is checked to name that inode, and the directory is fsynced. Anything else leaves `dest`
/// untouched, removes the staging directory, and the `Use` records `draw:unavailable` or
/// `draw:miss` with its reason.
///
/// # Errors
///
/// [`FetchGap::IntegrityFailed`] (naming the manifest) when the stream does not complete or the
/// staged path stopped naming the verified file; [`FetchGap::LocalStore`] when the file cannot be
/// written or the `Use` event appended.
pub async fn install_fetched<S, F>(
    store: &S,
    flow: &mut F,
    fetched: &FetchedArtifact,
    context: &LocalBlobContext,
    dest: &Path,
) -> Result<(), FetchGap>
where
    S: EprfsStorage + ?Sized,
    F: FlowStore + ?Sized,
{
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let local = |e: std::io::Error| FetchGap::LocalStore {
        reason: format!("{}: {e}", dest.display()),
    };
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| FetchGap::InstallRefused {
            dest: dest.display().to_string(),
            reason: "the destination names no directory".into(),
        })?;
    std::fs::create_dir_all(parent).map_err(local)?;
    let name = dest.file_name().ok_or_else(|| FetchGap::InstallRefused {
        dest: dest.display().to_string(),
        reason: "the destination names no file".into(),
    })?;
    sweep_stale_staging(parent, name);
    let staging = Staging::beside(parent, name).map_err(local)?;
    let staged = staging.dir.join("artifact");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&staged)
        .map_err(local)?;
    let mut sink = FileSink {
        file: std::io::BufWriter::new(file),
    };
    let mut slots = vec!["basis:sibling".to_string()];
    slots.extend(
        fetched
            .fetched_from
            .iter()
            .map(|device| format!("fetched-from:{device}")),
    );
    slots.push(format!("wasted:{}", fetched.wasted));
    let draw = stream_local_blob_with(
        store,
        flow,
        &fetched.manifest_cid,
        context,
        &mut sink,
        slots,
    )
    .await;
    match draw {
        Ok(TierBlobStreamDraw::Completed { .. }) => {}
        Ok(TierBlobStreamDraw::Miss { .. } | TierBlobStreamDraw::Unavailable { .. }) => {
            return Err(FetchGap::IntegrityFailed {
                cid: fetched.manifest_cid.clone(),
            })
        }
        Err(e) => {
            return Err(FetchGap::LocalStore {
                reason: e.to_string(),
            })
        }
    }
    let file = sink.file.into_inner().map_err(|e| local(e.into_error()))?;
    file.set_permissions(std::fs::Permissions::from_mode(0o755))
        .map_err(local)?;
    file.sync_all().map_err(local)?;
    let verified = identity(&file.metadata().map_err(local)?);
    let substituted = || FetchGap::IntegrityFailed {
        cid: fetched.manifest_cid.clone(),
    };
    match std::fs::symlink_metadata(&staged) {
        Ok(meta) if meta.file_type().is_file() && identity(&meta) == verified => {}
        _ => return Err(substituted()),
    }
    std::fs::rename(&staged, dest).map_err(local)?;
    match std::fs::symlink_metadata(dest) {
        Ok(meta) if meta.file_type().is_file() && identity(&meta) == verified => {}
        _ => return Err(substituted()),
    }
    std::fs::File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(local)?;
    drop(staging);
    Ok(())
}

/// Where a daemon may install a fetched artifact (design §3.b.2, review H5): under one of
/// `roots`, or directly in a `mesh-bin` directory under `mesh_bin_pool` (the cargo pool the mesh
/// build installs into). The default admits nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallScope {
    pub roots: Vec<PathBuf>,
    pub mesh_bin_pool: Option<PathBuf>,
}

impl InstallScope {
    /// The destination `dest` resolves to, when this scope admits it: absolute, with no `.` or
    /// `..` component, a file name, and its directory (resolved through every link to the nearest
    /// existing ancestor) under an admitted place.
    ///
    /// # Errors
    ///
    /// A printable reason when it is not admitted.
    pub fn admits(&self, dest: &Path) -> Result<PathBuf, String> {
        use std::path::Component;
        if !dest.is_absolute() {
            return Err("the destination is not an absolute path".into());
        }
        if dest
            .components()
            .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
        {
            return Err("the destination has a `.` or `..` component".into());
        }
        let name = dest
            .file_name()
            .ok_or("the destination names no file")?
            .to_owned();
        let parent = dest.parent().ok_or("the destination names no directory")?;
        let resolved = resolve_existing(parent)?;
        let canonical = |path: &Path| path.canonicalize().ok();
        let under_root = self
            .roots
            .iter()
            .filter_map(|root| canonical(root))
            .any(|root| resolved.starts_with(&root));
        let in_mesh_bin = self
            .mesh_bin_pool
            .as_deref()
            .and_then(canonical)
            .is_some_and(|pool| {
                resolved.starts_with(&pool) && resolved.file_name().is_some_and(|n| n == "mesh-bin")
            });
        if under_root || in_mesh_bin {
            Ok(resolved.join(name))
        } else {
            Err(format!(
                "{} is outside this daemon's install roots (a mesh-bin directory under the cargo \
                 pool, or an --install-root given at serve start)",
                dest.display()
            ))
        }
    }
}

/// `path` with its nearest existing ancestor canonicalized and the rest appended.
fn resolve_existing(path: &Path) -> Result<PathBuf, String> {
    let mut existing = path.to_path_buf();
    let mut rest = Vec::new();
    loop {
        if let Ok(canonical) = existing.canonicalize() {
            let mut resolved = canonical;
            for part in rest.iter().rev() {
                resolved.push(part);
            }
            return Ok(resolved);
        }
        let Some(name) = existing.file_name().map(std::ffi::OsStr::to_owned) else {
            return Err(format!("{} has no existing ancestor", path.display()));
        };
        rest.push(name);
        if !existing.pop() {
            return Err(format!("{} has no existing ancestor", path.display()));
        }
    }
}

/// One local or admitted sibling custody record for a key, in flow order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiblingRecord {
    pub key: BlobCid,
    pub device: String,
    pub output: BlobCid,
    pub manifest: BlobCid,
}

/// The newest custody claim per (device, key), in deterministic order. Both key and output
/// lookups filter this fold, so an obsolete claim cannot survive by querying its old output.
pub fn fold_holders(records: &[SiblingRecord]) -> Vec<&SiblingRecord> {
    let mut newest = std::collections::BTreeMap::new();
    for record in records {
        newest.insert((&record.device, &record.key), record);
    }
    newest.into_values().collect()
}

/// A local choice among custody claims; it does not grant admission or prove byte integrity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolderSelection {
    pub output: BlobCid,
    pub holders: Vec<RequestedHolder>,
    /// The first applicable rule: expected, own, or plurality.
    pub rule: &'static str,
    /// Sorted distinct outputs, including those not chosen.
    pub outputs: Vec<BlobCid>,
}

/// Choose holders for a key, preferring an explicit output, then this device's own newest
/// Produce, then a strict plurality of distinct devices. A tie remains a mismatch.
///
/// # Errors
///
/// No record, no holder of the expected output, or an unresolved tie.
pub fn resolve_holders(
    key: &BlobCid,
    records: &[SiblingRecord],
    own_device: &str,
    expected: Option<&BlobCid>,
) -> Result<HolderSelection, FetchGap> {
    let current: Vec<_> = fold_holders(records)
        .into_iter()
        .filter(|r| &r.key == key)
        .collect();
    let mut by_output: std::collections::BTreeMap<BlobCid, Vec<RequestedHolder>> =
        std::collections::BTreeMap::new();
    for record in &current {
        by_output
            .entry(record.output.clone())
            .or_default()
            .push(RequestedHolder {
                device: record.device.clone(),
                manifest: record.manifest.clone(),
            });
    }
    let outputs: Vec<_> = by_output.keys().cloned().collect();
    let (output, rule) = if let Some(expected) = expected {
        if !by_output.contains_key(expected) {
            return Err(FetchGap::ExpectedOutputUnavailable {
                output: expected.clone(),
            });
        }
        (expected.clone(), "expected")
    } else if let Some(own) = current.iter().find(|record| record.device == own_device) {
        (own.output.clone(), "own")
    } else {
        let max = by_output
            .values()
            .map(Vec::len)
            .max()
            .ok_or(FetchGap::NoRecord)?;
        let mut winners = by_output.iter().filter(|(_, holders)| holders.len() == max);
        let output = winners.next().ok_or(FetchGap::NoRecord)?.0.clone();
        if winners.next().is_some() {
            return Err(FetchGap::Mismatch { outputs });
        }
        (output, "plurality")
    };
    let holders = by_output.remove(&output).ok_or(FetchGap::NoRecord)?;
    Ok(HolderSelection {
        output,
        holders,
        rule,
        outputs,
    })
}

/// One holder named in a queued request: a device and the manifest its crossed record names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestedHolder {
    pub device: String,
    pub manifest: BlobCid,
}

/// A fetch `epr artifact fetch` queues for the daemon (design §3.b.2): the daemon holds the
/// device's peer key, so only it dials. The daemon re-checks admission, the install scope and
/// every integrity rule itself; the request names what to look for, never what to trust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FetchRequest {
    /// This request alone: its file names and its answer carry it, so no other request's answer is
    /// ever taken for it.
    pub nonce: String,
    /// The key this device computed.
    pub key: DerivationKey,
    /// [`key_cid`] of `key`.
    pub key_cid: BlobCid,
    pub output: BlobCid,
    /// Optional caller constraint, rechecked by the daemon before any bytes move.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_output: Option<BlobCid>,
    pub holders: Vec<RequestedHolder>,
    /// Where the verified artifact is installed.
    pub dest: PathBuf,
    /// The flow scope the install's `Use` event is recorded in.
    pub scope: String,
    pub transport: FetchTransport,
    pub stall_s: u64,
    pub total_s: u64,
}

impl FetchRequest {
    /// The budget the request asks for, with the default caps.
    pub fn budget(&self) -> FetchBudget {
        FetchBudget {
            stall: Duration::from_secs(self.stall_s.max(1)),
            total: Duration::from_secs(self.total_s.max(1)),
            transport: self.transport,
            ..FetchBudget::default()
        }
    }

    /// The file stem this request is queued under: `<key CID>-<nonce>`.
    pub fn stem(&self) -> String {
        format!("{}-{}", self.key_cid, self.nonce)
    }

    /// A fresh nonce: 128 random bits, hex.
    pub fn fresh_nonce() -> String {
        format!(
            "{:016x}{:016x}",
            rand::random::<u64>(),
            rand::random::<u64>()
        )
    }
}

/// Who an answer is for: the request's nonce, key, output and destination. A requester accepts an
/// answer only when all four are its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FetchFor {
    pub nonce: String,
    pub key_cid: BlobCid,
    pub output: BlobCid,
    pub dest: PathBuf,
}

impl FetchFor {
    /// The identity of `request`.
    pub fn of(request: &FetchRequest) -> FetchFor {
        FetchFor {
            nonce: request.nonce.clone(),
            key_cid: request.key_cid.clone(),
            output: request.output.clone(),
            dest: request.dest.clone(),
        }
    }
}

/// What the daemon writes back for one request: the installed artifact, or the gap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "outcome",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum FetchDone {
    Installed {
        #[serde(rename = "for")]
        for_request: FetchFor,
        manifest: BlobCid,
        manifest_from: String,
        fetched_from: Vec<String>,
        chunks_fetched: usize,
        chunks_resumed: usize,
        bytes_fetched: u64,
        wasted: u64,
        seconds: f64,
    },
    Gap {
        #[serde(rename = "for")]
        for_request: FetchFor,
        gap: FetchGap,
        reason: String,
    },
}

impl FetchDone {
    /// The written-back gap for `request`.
    pub fn gap(request: FetchFor, gap: FetchGap) -> FetchDone {
        FetchDone::Gap {
            for_request: request,
            reason: gap.to_string(),
            gap,
        }
    }

    /// The written-back installation of `fetched` for `request`.
    pub fn installed(request: FetchFor, fetched: &FetchedArtifact) -> FetchDone {
        FetchDone::Installed {
            for_request: request,
            manifest: fetched.manifest_cid.clone(),
            manifest_from: fetched.manifest_from.clone(),
            fetched_from: fetched.fetched_from.iter().cloned().collect(),
            chunks_fetched: fetched.chunks_fetched,
            chunks_resumed: fetched.chunks_resumed,
            bytes_fetched: fetched.bytes_fetched,
            wasted: fetched.wasted,
            seconds: fetched.elapsed.as_secs_f64(),
        }
    }

    /// The request this answer is for.
    pub fn for_request(&self) -> &FetchFor {
        match self {
            FetchDone::Installed { for_request, .. } | FetchDone::Gap { for_request, .. } => {
                for_request
            }
        }
    }
}

/// The largest queue file a daemon or requester reads.
pub const MAX_QUEUE_FILE_BYTES: u64 = 64 * 1024;

/// `<state dir>/fetch-queue`.
pub fn fetch_queue_dir(state_dir: &Path) -> PathBuf {
    state_dir.join(FETCH_QUEUE)
}

/// The request file for `stem` (see [`FetchRequest::stem`]) in `queue`.
pub fn fetch_request_path(queue: &Path, stem: &str) -> PathBuf {
    queue.join(format!("{stem}.json"))
}

/// The answer file for `stem` in `queue`.
pub fn fetch_done_path(queue: &Path, stem: &str) -> PathBuf {
    queue.join(format!("{stem}.done.json"))
}

/// The withdrawal marker for `stem` in `queue`: its presence cancels the fetch.
pub fn fetch_cancel_path(queue: &Path, stem: &str) -> PathBuf {
    queue.join(format!("{stem}.cancel"))
}

/// Create `queue` (and its parents) with mode 0700, and hold it there.
///
/// # Errors
///
/// The I/O errors.
pub fn ensure_queue_dir(queue: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(queue)?;
    std::fs::set_permissions(queue, std::fs::Permissions::from_mode(0o700))
}

/// Whether `queue` is a directory only its owner can reach (mode 0700, no group or other bits).
pub fn queue_dir_is_private(queue: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::symlink_metadata(queue)
        .is_ok_and(|meta| meta.is_dir() && meta.permissions().mode() & 0o077 == 0)
}

/// Write `value` as JSON to `path` through a sibling temporary file and a rename, so a reader
/// never sees half of it. The temporary is created exclusively, mode 0600.
///
/// # Errors
///
/// The I/O and encoding errors.
pub fn write_queue_file<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let tmp = path.with_extension(format!("tmp-{:016x}", rand::random::<u64>()));
    let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    std::fs::rename(&tmp, path)
}

/// Read one queue file: a regular file (never a link, FIFO or device; the open itself never
/// blocks and never follows a link) of at most [`MAX_QUEUE_FILE_BYTES`].
///
/// # Errors
///
/// A printable reason for anything else.
pub fn read_queue_file(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() {
        return Err("not a regular file".into());
    }
    if meta.len() > MAX_QUEUE_FILE_BYTES {
        return Err(format!(
            "{} bytes, past the {MAX_QUEUE_FILE_BYTES}-byte cap",
            meta.len()
        ));
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    // Checked again on the descriptor: the name may have changed since the first look.
    let opened = file.metadata().map_err(|e| e.to_string())?;
    if !opened.file_type().is_file() || opened.len() > MAX_QUEUE_FILE_BYTES {
        return Err("not a regular file within the cap".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_QUEUE_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_QUEUE_FILE_BYTES {
        return Err("grew past the cap while read".into());
    }
    Ok(bytes)
}

/// Holders resolved to addresses: the queued holders `addr_of` resolves (the daemon's admission
/// now), other than this device, each device once, in the order queued.
pub(crate) fn admitted_holders(
    requested: &[RequestedHolder],
    me: &str,
    addr_of: impl Fn(&str) -> Option<NodeAddr>,
) -> Vec<Holder> {
    let mut seen = BTreeSet::new();
    requested
        .iter()
        .filter(|holder| holder.device != me && seen.insert(holder.device.clone()))
        .filter_map(|holder| {
            Some(Holder {
                device: holder.device.clone(),
                addr: addr_of(&holder.device)?,
                manifest: holder.manifest.clone(),
            })
        })
        .collect()
}
