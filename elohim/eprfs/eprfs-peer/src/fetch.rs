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
use std::time::{Duration, Instant};

use bytes::Bytes;
use elohim_epr_rea::FlowStore;
use eprfs_core::{
    BlobCid, BlobPresence, DerivationKey, EprfsStorage, ItemAbsence, MethodDescriptor, TreeLimits,
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
use crate::tier::{tier_request_capped, TierAnswer, TierRequest, TIER_ALPN};

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
    /// Admitted siblings recorded different outputs for one key: the method is not reproducible
    /// enough to pick one, so nothing is fetched.
    Mismatch { outputs: Vec<BlobCid> },
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
}

impl FetchGap {
    /// The gap's name, as the CLI prints it before its reason.
    pub fn kind(&self) -> &'static str {
        match self {
            FetchGap::NoRecord => "no-record",
            FetchGap::Mismatch { .. } => "mismatch",
            FetchGap::NoRoute { .. } => "no-route",
            FetchGap::Refused { .. } => "refused",
            FetchGap::ManifestTooLarge { .. } => "manifest-too-large",
            FetchGap::ManifestRejected { .. } => "manifest-rejected",
            FetchGap::DeadlineExceeded { .. } => "deadline-exceeded",
            FetchGap::IntegrityFailed { .. } => "integrity-failed",
            FetchGap::DaemonDown => "daemon-down",
            FetchGap::TransportCannotDeliver { .. } => "transport-cannot-deliver",
            FetchGap::LocalStore { .. } => "local-store",
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
                "admitted siblings recorded {} different outputs for this key ({}); not \
                 reproducible enough to pick one, build here",
                outputs.len(),
                outputs
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
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

async fn dial(endpoint: &Endpoint, holder: &Holder, stall: Duration) -> Option<Connection> {
    match tokio::time::timeout(stall, endpoint.connect(holder.addr.clone(), TIER_ALPN)).await {
        Ok(Ok(connection)) => Some(connection),
        Ok(Err(_)) | Err(_) => None,
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

/// Ask the holders for the manifest in order, until one serves a manifest this device accepts.
async fn accept_manifest(
    endpoint: &Endpoint,
    key: &DerivationKey,
    output: &BlobCid,
    holders: &[Holder],
    budget: &FetchBudget,
    tree: &TreeLimits,
    unreachable: &mut BTreeSet<usize>,
) -> Result<Accepted, FetchGap> {
    let mut last = FetchGap::NoRecord;
    for (index, holder) in holders.iter().enumerate() {
        let Some(connection) = dial(endpoint, holder, budget.stall).await else {
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
        let answer = tokio::time::timeout(
            budget.stall,
            tier_request_capped(&connection, &request, tree, budget.manifest_cap),
        )
        .await;
        let rejected = |reason: String| FetchGap::ManifestRejected { reason };
        let gap = match answer {
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
            Ok(Err(_)) | Err(_) => {
                if relay_only(&holder.addr) {
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
        connection.close(VarInt::from_u32(0), b"");
        unreachable.insert(index);
        last = gap;
    }
    Err(last)
}

/// Fetch the artifact `output` derived under `key` from `holders` into `store` (design §3.b.3).
/// The install read is [`install_fetched`]'s.
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
    let started = Instant::now();
    let deadline = started + budget.total;
    if budget.transport == FetchTransport::IrohBlobs {
        return Err(FetchGap::TransportCannotDeliver {
            mode: FetchTransport::IrohBlobs.name().into(),
        });
    }
    if holders.is_empty() {
        return Err(FetchGap::NoRecord);
    }
    let tree = TreeLimits::default();
    let mut unreachable = BTreeSet::new();
    let accepting = tokio::time::timeout(
        budget.total,
        accept_manifest(
            endpoint,
            key,
            output,
            holders,
            budget,
            &tree,
            &mut unreachable,
        ),
    )
    .await
    .unwrap_or(Err(FetchGap::DeadlineExceeded {
        fetched: 0,
        total: 0,
    }))?;
    let Accepted {
        holder: manifest_holder,
        connection,
        cid: manifest_cid,
        bytes: manifest_bytes,
        manifest,
    } = accepting;

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

    // One connection per holder for the whole artifact: the manifest's, then every other holder
    // not already found unreachable or refusing.
    let mut links = vec![Link {
        device: holders[manifest_holder].device.clone(),
        connection,
        relay_only: relay_only(&holders[manifest_holder].addr),
        in_flight: 0,
        abandoned: None,
    }];
    if !queue.is_empty() {
        for (index, holder) in holders.iter().enumerate() {
            if index == manifest_holder || unreachable.contains(&index) {
                continue;
            }
            if let Some(connection) = dial(endpoint, holder, budget.stall).await {
                links.push(Link {
                    device: holder.device.clone(),
                    connection,
                    relay_only: relay_only(&holder.addr),
                    in_flight: 0,
                    abandoned: None,
                });
            }
        }
    }

    let cap = budget.in_flight_per_holder.max(1);
    let mut fetched_from = BTreeSet::from([holders[manifest_holder].device.clone()]);
    let mut tasks: JoinSet<(usize, TierBlobChunk, ChunkOutcome)> = JoinSet::new();
    let mut next = 0usize;
    let (mut chunks_fetched, mut bytes_fetched, mut wasted) = (0usize, 0u64, 0u64);
    let held_bytes: u64 = manifest
        .chunks
        .iter()
        .filter(|chunk| !queue.iter().any(|missing| missing.cid == chunk.cid))
        .map(|chunk| u64::from(chunk.byte_len))
        .sum();
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
            let stall = budget.stall;
            // A chunk is asked under the artifact (its output), never alone.
            let request = TierRequest::Blob {
                artifact: output.clone(),
                cid: chunk.cid.clone(),
            };
            tasks.spawn(async move {
                let cap = chunk.byte_len.saturating_add(CHUNK_FRAME_OVERHEAD);
                let answer = tokio::time::timeout(
                    stall,
                    tier_request_capped(&connection, &request, &TreeLimits::default(), cap),
                )
                .await;
                (chosen, chunk, answer)
            });
        }
        if tasks.is_empty() {
            if queue.is_empty() {
                break Ok(());
            }
            break Err(exhausted(&links, holders, manifest_holder));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let joined = match tokio::time::timeout(remaining, tasks.join_next()).await {
            Err(_) => {
                tasks.abort_all();
                break Err(FetchGap::DeadlineExceeded {
                    fetched: held_bytes + bytes_fetched,
                    total: manifest.byte_len,
                });
            }
            Ok(None) => continue,
            Ok(Some(joined)) => joined,
        };
        let Ok((index, chunk, answer)) = joined else {
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
        match answer {
            Ok(Ok(TierAnswer::Blob { cid, bytes })) => {
                // `tier_request` checked the CID asked and the bytes against it; checked again,
                // with the manifest's length, before anything is stored.
                if cid != chunk.cid
                    || !chunk.cid.verifies(&bytes)
                    || bytes.len() != chunk.byte_len as usize
                {
                    wasted += bytes.len() as u64;
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
            }
            Ok(Err(PeerError::TierLeafIntegrity { .. })) => {
                // The answer arrived and failed its address: its declared length moved for nothing.
                wasted += u64::from(chunk.byte_len);
                abandon(&mut links, Abandon::Integrity(chunk.cid.clone()));
                queue.push_front(chunk);
            }
            Ok(Ok(TierAnswer::Unanswered { why, .. })) => {
                abandon(&mut links, Abandon::Refused(refusal_text(&why.why)));
                queue.push_front(chunk);
            }
            Ok(Ok(TierAnswer::Failed { reason })) => {
                abandon(&mut links, Abandon::Refused(reason));
                queue.push_front(chunk);
            }
            Ok(Ok(_)) | Ok(Err(_)) => {
                abandon(&mut links, Abandon::Stream);
                queue.push_front(chunk);
            }
            Err(_) => {
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
        wasted,
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

/// The temporary sibling `dest` is streamed into before the rename.
fn staging_path(dest: &Path) -> PathBuf {
    let name = dest
        .file_name()
        .map_or_else(|| "artifact".into(), |n| n.to_string_lossy().into_owned());
    dest.with_file_name(format!(".{name}.fetching"))
}

/// The install read (design §3.b.3): stream the fetched artifact out of `store` into `dest` through
/// [`stream_local_blob_with`], whose incremental whole-blob verify against the record's output is
/// what admits the bytes, and whose one `Use` event records the draw: `draw:hit`, `copy:self`,
/// `basis:sibling`, `fetched-from:<did>` (one per device that served), `wasted:<n>`. Only a
/// completed, verified stream is renamed onto `dest`; anything else removes the staging file and
/// leaves `dest` untouched, and the `Use` records `draw:unavailable` or `draw:miss` with its reason.
///
/// # Errors
///
/// [`FetchGap::IntegrityFailed`] (naming the manifest) when the stream does not complete;
/// [`FetchGap::LocalStore`] when the file cannot be written or the `Use` event appended.
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
    let local = |e: std::io::Error| FetchGap::LocalStore {
        reason: format!("{}: {e}", dest.display()),
    };
    if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(local)?;
    }
    let staging = staging_path(dest);
    let file = std::fs::File::create(&staging).map_err(local)?;
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
    let finish = |sink: FileSink| -> std::io::Result<()> {
        let file = sink.file.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()
    };
    match draw {
        Ok(TierBlobStreamDraw::Completed { .. }) => {
            if let Err(e) = finish(sink).and_then(|()| std::fs::rename(&staging, dest)) {
                let _ = std::fs::remove_file(&staging);
                return Err(local(e));
            }
            Ok(())
        }
        Ok(TierBlobStreamDraw::Miss { .. } | TierBlobStreamDraw::Unavailable { .. }) => {
            drop(sink);
            let _ = std::fs::remove_file(&staging);
            Err(FetchGap::IntegrityFailed {
                cid: fetched.manifest_cid.clone(),
            })
        }
        Err(e) => {
            drop(sink);
            let _ = std::fs::remove_file(&staging);
            Err(FetchGap::LocalStore {
                reason: e.to_string(),
            })
        }
    }
}

/// One admitted sibling's custody record for a key, in flow order, as resolution reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiblingRecord {
    pub device: String,
    pub output: BlobCid,
    pub manifest: BlobCid,
}

/// The holders for one key (design §3.b.1): each device's newest record, and the one output they
/// all name. No record is [`FetchGap::NoRecord`]; two outputs for one key is
/// [`FetchGap::Mismatch`], not a fetch: nothing is asked of anyone.
///
/// # Errors
///
/// [`FetchGap::NoRecord`] and [`FetchGap::Mismatch`].
pub fn resolve_holders(
    records: &[SiblingRecord],
) -> Result<(BlobCid, Vec<RequestedHolder>), FetchGap> {
    let mut newest: std::collections::BTreeMap<&str, &SiblingRecord> =
        std::collections::BTreeMap::new();
    for record in records {
        newest.insert(&record.device, record);
    }
    let outputs: BTreeSet<&BlobCid> = newest.values().map(|record| &record.output).collect();
    let output = match outputs.len() {
        0 => return Err(FetchGap::NoRecord),
        1 => outputs
            .into_iter()
            .next()
            .cloned()
            .ok_or(FetchGap::NoRecord)?,
        _ => {
            return Err(FetchGap::Mismatch {
                outputs: outputs.into_iter().cloned().collect(),
            })
        }
    };
    let holders = newest
        .values()
        .map(|record| RequestedHolder {
            device: record.device.clone(),
            manifest: record.manifest.clone(),
        })
        .collect();
    Ok((output, holders))
}

/// One holder named in a queued request: a device and the manifest its crossed record names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestedHolder {
    pub device: String,
    pub manifest: BlobCid,
}

/// A fetch `epr artifact fetch` queues for the daemon (design §3.b.2): the daemon holds the
/// device's peer key, so only it dials. The daemon re-checks admission and every integrity rule
/// itself; the request names what to look for, never what to trust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchRequest {
    /// The key this device computed.
    pub key: DerivationKey,
    /// [`key_cid`] of `key`; the request's file name.
    pub key_cid: BlobCid,
    pub output: BlobCid,
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
        key_cid: BlobCid,
        manifest: BlobCid,
        output: BlobCid,
        manifest_from: String,
        fetched_from: Vec<String>,
        chunks_fetched: usize,
        chunks_resumed: usize,
        bytes_fetched: u64,
        wasted: u64,
        seconds: f64,
    },
    Gap {
        key_cid: BlobCid,
        gap: FetchGap,
        reason: String,
    },
}

impl FetchDone {
    /// The written-back gap for `key_cid`.
    pub fn gap(key_cid: BlobCid, gap: FetchGap) -> FetchDone {
        FetchDone::Gap {
            key_cid,
            reason: gap.to_string(),
            gap,
        }
    }

    /// The written-back installation of `fetched`.
    pub fn installed(key_cid: BlobCid, fetched: &FetchedArtifact) -> FetchDone {
        FetchDone::Installed {
            key_cid,
            manifest: fetched.manifest_cid.clone(),
            output: fetched.manifest.record.output.clone(),
            manifest_from: fetched.manifest_from.clone(),
            fetched_from: fetched.fetched_from.iter().cloned().collect(),
            chunks_fetched: fetched.chunks_fetched,
            chunks_resumed: fetched.chunks_resumed,
            bytes_fetched: fetched.bytes_fetched,
            wasted: fetched.wasted,
            seconds: fetched.elapsed.as_secs_f64(),
        }
    }
}

/// `<state dir>/fetch-queue`.
pub fn fetch_queue_dir(state_dir: &Path) -> PathBuf {
    state_dir.join(FETCH_QUEUE)
}

/// The request file for `key_cid` in `queue`.
pub fn fetch_request_path(queue: &Path, key_cid: &BlobCid) -> PathBuf {
    queue.join(format!("{key_cid}.json"))
}

/// The answer file for `key_cid` in `queue`.
pub fn fetch_done_path(queue: &Path, key_cid: &BlobCid) -> PathBuf {
    queue.join(format!("{key_cid}.done.json"))
}

/// Write `value` as JSON to `path` through a sibling temporary file and a rename, so a reader
/// never sees half of it.
///
/// # Errors
///
/// The I/O and encoding errors.
pub fn write_queue_file<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
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
