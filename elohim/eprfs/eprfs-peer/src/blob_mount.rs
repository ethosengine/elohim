//! The artifact mount: the second answer path of a device's tier, beside its channel tree
//! (artifact-store design §1.1, §1.5).
//!
//! The channel store stays the only tree store the tier endpoint reads. This mount answers
//! [`TierRequest::Record`] (and legacy output-only requests) from the payload store, for the
//! derivation artifacts this device holds custody of, and only to a device the artifact grant
//! admits ([`crate::ArtifactServeList`]). It keeps:
//!
//! - an [`ArtifactIndex`] of what it may serve: each artifact's manifest, method name and chunks.
//!   The index's source of truth is this device's own flow store — the `custody-blob` Produce
//!   events with `manifest:<cid>` and `copy:self` that [`crate::put_local_blob`] writes, daemon or
//!   no daemon — rebuilt when the store moves. Only this device's own custody counts: a Produce
//!   whose custodian (its receiver) is another device, or that was folded from another device
//!   (`via:`), is never local custody. A manifest whose method record is not held here is
//!   [`Unindexed::MethodUnknown`], one whose chunks do not reconstruct its output here is
//!   [`Unindexed::OutputUnverified`] or [`Unindexed::ManifestInvalid`], and neither is served. For
//!   an explicit request, the named custody manifest is served; an indexed manifest that no longer reads is dropped at the next rebuild. A failed manifest is
//!   retried with a backoff, an invalid one only after a new custody event;
//! - one `serve-blob` session per (requester, manifest), closed on the last distinct chunk, when
//!   the connection closes, or after [`SERVE_IDLE`]; each closed session (and each refusal of an
//!   admitted device) becomes one [`ServedDraw`], which the daemon records as one `Use` event.
//!   That event carries no `measure:` slot, so it never crosses. Recording is bounded: at most
//!   [`REFUSALS_PER_WINDOW`] refusals per requester per [`SERVE_IDLE`] are recorded (the rest are
//!   counted), and at most [`SERVED_QUEUE`] draws wait for the daemon (the rest are dropped and
//!   counted).
//!
//! A stranger is only counted. Every refusal it receives is the uniform form, byte-identical to
//! the tree endpoint's, whether the artifact is granted, held ungranted, or unknown.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use elohim_epr_rea::{AgentRef, FlowEvent, ReaVerb};
use eprfs_core::{
    BlobCid, EprfsError, EprfsStorage, FetchPolicy, ItemAbsence, MethodDescriptor, ReachGate,
    ReachReason, ReachRefusal, Requester, TierId, TreePath,
};
use eprfs_storage::{
    read_tier_manifest, stream_tier_blob, DirectoryStorage, OffloadedDirectory, TierBlobChunk,
    TierBlobStreamDraw, TierBlobUnavailable,
};
use tokio::sync::mpsc;

use crate::allow::via_device;
use crate::channel::ChannelStore;
use crate::error::PeerError;
use crate::gate::{ServeScope, Served, SharedAdmission, StewardDevicesGate};
use crate::local_blob::{custody_use_event, LocalBlobContext};
use crate::serves::ArtifactServeList;
use crate::tier::{TierAnswer, TierRequest};

/// How long a `serve-blob` session waits for its next chunk before it is closed as partial.
pub const SERVE_IDLE: Duration = Duration::from_secs(60);

/// The tag a serve's `Use` event carries after its subject.
pub const SERVE_BLOB: &str = "serve-blob";

/// The most refusals of one requester recorded as events per [`SERVE_IDLE`] window; further
/// refusals in the window are only counted ([`BlobMount::suppressed_refusals`]).
pub const REFUSALS_PER_WINDOW: usize = 16;

/// The most closed sessions and refusals waiting for the daemon to record them; past it a draw is
/// dropped and counted ([`BlobMount::dropped_draws`]), never queued without bound.
pub const SERVED_QUEUE: usize = 1024;

/// Why a custody manifest is held but not served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unindexed {
    /// The method descriptor its derivation record keys is not held here, or does not decode to
    /// that CID: the method's name, and so the grant, cannot be known.
    MethodUnknown,
    /// The manifest does not read, does not validate, disagrees with its Produce event, or its
    /// chunks do not reconstruct its output.
    ManifestInvalid { reason: String },
    /// Its chunks are not all held here, so whether they reconstruct its output cannot be
    /// checked yet: deferred, and tried again at the next rebuild.
    OutputUnverified { reason: String },
}

impl std::fmt::Display for Unindexed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unindexed::MethodUnknown => f.write_str("its method record is not held here"),
            Unindexed::ManifestInvalid { reason } => write!(f, "the manifest is invalid: {reason}"),
            Unindexed::OutputUnverified { reason } => {
                write!(f, "its output is not verified here yet: {reason}")
            }
        }
    }
}

/// One artifact the mount may serve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    /// The manifest CID.
    pub manifest: BlobCid,
    /// The complete artifact's CID (`record.output`): the root a requester names.
    pub output: BlobCid,
    /// The method CID the derivation record keys.
    pub method: BlobCid,
    /// The derivation key CID, checked for record-bound requests.
    pub key: BlobCid,
    /// The input document named by the derivation key (self reach, like its method).
    pub input: BlobCid,
    /// The method descriptor's name: what the grant is judged on.
    pub method_name: String,
    /// The chunks the manifest names, in order (a repeated chunk appears once per use).
    pub chunks: Vec<BlobCid>,
    /// The complete artifact's length.
    pub byte_len: u64,
    /// The scope of the Produce event that put it in custody.
    pub in_scope_of: cid::Cid,
}

impl Held {
    fn distinct_chunks(&self) -> usize {
        self.chunks.iter().collect::<BTreeSet<_>>().len()
    }
}

/// What the mount may serve, by custody manifest and by object.
#[derive(Debug, Default)]
pub struct ArtifactIndex {
    by_manifest: BTreeMap<BlobCid, Held>,
    /// Latest custody manifest per output, for legacy output-only requests.
    latest: BTreeMap<BlobCid, BlobCid>,
    /// Object CIDs → the manifests naming them.
    by_object: BTreeMap<BlobCid, BTreeSet<BlobCid>>,
    /// Designated manifests held and not served, with why.
    unindexed: BTreeMap<BlobCid, Unindexed>,
    /// When each designated manifest that failed may be tried again.
    retry: BTreeMap<BlobCid, Retry>,
}

impl ArtifactIndex {
    /// Index one custody record without replacing other records of the same output.
    fn insert(&mut self, held: Held) {
        self.unindexed.remove(&held.manifest);
        self.retry.remove(&held.manifest);
        self.remove_manifest(&held.manifest);
        for object in std::iter::once(&held.manifest)
            .chain(std::iter::once(&held.method))
            .chain(std::iter::once(&held.input))
            .chain(&held.chunks)
        {
            self.by_object
                .entry(object.clone())
                .or_default()
                .insert(held.manifest.clone());
        }
        self.by_manifest.insert(held.manifest.clone(), held);
    }

    /// Drop one manifest and every membership it gave its objects.
    fn remove_manifest(&mut self, manifest: &BlobCid) -> Option<Held> {
        let old = self.by_manifest.remove(manifest)?;
        for object in std::iter::once(&old.manifest)
            .chain(std::iter::once(&old.method))
            .chain(std::iter::once(&old.input))
            .chain(&old.chunks)
        {
            let emptied = self.by_object.get_mut(object).is_some_and(|outputs| {
                outputs.remove(manifest);
                outputs.is_empty()
            });
            if emptied {
                self.by_object.remove(object);
            }
        }
        Some(old)
    }

    /// Retain the distinct manifests named by local custody, including older records.
    fn retain_designated(&mut self, designated: &BTreeMap<BlobCid, Designated>) {
        let stale: Vec<_> = self
            .by_manifest
            .keys()
            .filter(|manifest| !designated.contains_key(*manifest))
            .cloned()
            .collect();
        for manifest in stale {
            self.remove_manifest(&manifest);
        }
        self.unindexed
            .retain(|manifest, _| designated.contains_key(manifest));
        self.retry
            .retain(|manifest, _| designated.contains_key(manifest));
    }

    /// How many manifests the index tracks, served or not.
    fn tracked(&self) -> usize {
        let mut manifests: BTreeSet<&BlobCid> = self
            .by_manifest
            .values()
            .map(|held| &held.manifest)
            .collect();
        manifests.extend(self.unindexed.keys());
        manifests.extend(self.retry.keys());
        manifests.len()
    }
}

/// The latest local custody event for one manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Designated {
    manifest: BlobCid,
    /// The custody event's own CID: a new event (a re-put) is a new chance to index.
    event: cid::Cid,
    in_scope_of: cid::Cid,
    resource: cid::Cid,
}

/// The retry state of a designated manifest that failed to index.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Retry {
    /// The custody event the failure was for; a different designating event retries at once.
    event: cid::Cid,
    /// `None`: never again for this event (an invalid manifest or output mismatch).
    next: Option<Instant>,
    wait: Duration,
}

/// The first wait before a failed manifest is tried again.
pub const INDEX_RETRY_FIRST: Duration = Duration::from_secs(30);

/// The longest wait between tries of one failing manifest.
pub const INDEX_RETRY_MAX: Duration = Duration::from_secs(3600);

/// How a `serve-blob` session ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawOutcome {
    /// Every distinct chunk of the artifact was served.
    Completed,
    /// The session closed (idle, or its connection closed) before every chunk was served.
    Partial,
    /// An admitted device was refused, for `reason`.
    Refused { reason: String },
}

impl DrawOutcome {
    fn slot(&self) -> &'static str {
        match self {
            DrawOutcome::Completed => "draw:completed",
            DrawOutcome::Partial => "draw:partial",
            DrawOutcome::Refused { .. } => "draw:refused",
        }
    }
}

/// One closed `serve-blob` session, or one refusal of an admitted device: one `Use` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedDraw {
    /// The requesting device's did:key.
    pub requester: String,
    /// The artifact asked for (the output CID named in the request).
    pub artifact: BlobCid,
    /// Its manifest, when the artifact is indexed.
    pub manifest: Option<BlobCid>,
    /// Chunk bytes served in the session.
    pub bytes: u64,
    /// Distinct chunks served.
    pub chunks: usize,
    /// Distinct chunks the artifact has.
    pub of: usize,
    pub outcome: DrawOutcome,
    /// The custody Produce event's scope, when the artifact is indexed.
    pub in_scope_of: Option<cid::Cid>,
}

impl ServedDraw {
    /// The `Use` event this draw is recorded as, provided by `custodian` (this device) to the
    /// requester, at `occurred_at`. No `measure:` slot: it never crosses.
    pub fn use_event(&self, custodian: &str, occurred_at: String) -> FlowEvent {
        let resource = *self.artifact.as_cid();
        let context = LocalBlobContext {
            custodian: AgentRef(custodian.to_string()),
            in_scope_of: self.in_scope_of.unwrap_or(resource),
            occurred_at,
            crossing: None,
        };
        let mut slots = vec![SERVE_BLOB.to_string(), format!("copy:{}", self.requester)];
        if let Some(manifest) = &self.manifest {
            slots.push(format!("manifest:{manifest}"));
        }
        slots.push(self.outcome.slot().to_string());
        if let DrawOutcome::Refused { reason } = &self.outcome {
            slots.push(format!("reason:{reason}"));
        }
        let mut event = custody_use_event(
            &context,
            &self.artifact.to_string(),
            resource,
            self.bytes as f64,
            slots,
        );
        event.receiver = AgentRef(self.requester.clone());
        event
    }
}

/// An open session: what one requester has drawn of one artifact.
#[derive(Debug)]
struct Session {
    held: Held,
    connection: usize,
    served: BTreeSet<BlobCid>,
    bytes: u64,
    last: Instant,
}

impl Session {
    fn draw(self, requester: String, outcome: DrawOutcome) -> ServedDraw {
        ServedDraw {
            requester,
            artifact: self.held.output.clone(),
            manifest: Some(self.held.manifest.clone()),
            bytes: self.bytes,
            chunks: self.served.len(),
            of: self.held.distinct_chunks(),
            outcome,
            in_scope_of: Some(self.held.in_scope_of),
        }
    }
}

/// What one answer of the mount was, for the tier's counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answered {
    Manifest,
    Chunk,
    Refused,
    Other,
}

/// One requester's refusals recorded in the current [`SERVE_IDLE`] window.
#[derive(Debug)]
struct RefusalWindow {
    opened: Instant,
    /// The (artifact, reason) pairs recorded in this window: each once, and at most
    /// [`REFUSALS_PER_WINDOW`] of them.
    recorded: BTreeSet<(BlobCid, String)>,
}

/// The artifact mount. See the module docs.
pub struct BlobMount {
    store: Arc<dyn EprfsStorage>,
    /// This device's did:key: the only custodian whose custody Produce events are indexed.
    custodian: String,
    index: RwLock<ArtifactIndex>,
    served: mpsc::Sender<ServedDraw>,
    sessions: Mutex<HashMap<(String, BlobCid), Session>>,
    /// Per requester, the refusals recorded in its current window.
    refused: Mutex<HashMap<String, RefusalWindow>>,
    suppressed_refusals: AtomicU64,
    dropped_draws: AtomicU64,
    reconstructions: AtomicU64,
}

impl std::fmt::Debug for BlobMount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobMount").finish_non_exhaustive()
    }
}

impl BlobMount {
    /// A mount over `store` for the device `custodian` (its did:key), and the receiver its closed
    /// sessions arrive on (at most [`SERVED_QUEUE`] waiting).
    pub fn new(
        store: Arc<dyn EprfsStorage>,
        custodian: impl Into<String>,
    ) -> (BlobMount, mpsc::Receiver<ServedDraw>) {
        let (served, receiver) = mpsc::channel(SERVED_QUEUE);
        (
            BlobMount {
                store,
                custodian: custodian.into(),
                index: RwLock::new(ArtifactIndex::default()),
                served,
                sessions: Mutex::new(HashMap::new()),
                refused: Mutex::new(HashMap::new()),
                suppressed_refusals: AtomicU64::new(0),
                dropped_draws: AtomicU64::new(0),
                reconstructions: AtomicU64::new(0),
            },
            receiver,
        )
    }

    /// Refusals of admitted devices counted but not recorded: past [`REFUSALS_PER_WINDOW`] in a
    /// requester's window.
    pub fn suppressed_refusals(&self) -> u64 {
        self.suppressed_refusals.load(Ordering::Relaxed)
    }

    /// Draws dropped because [`SERVED_QUEUE`] draws were already waiting.
    pub fn dropped_draws(&self) -> u64 {
        self.dropped_draws.load(Ordering::Relaxed)
    }

    /// Hand `draw` to the daemon, or count it dropped when the queue is full. A closed receiver
    /// means the daemon is gone: nothing is left to record.
    fn send(&self, draw: ServedDraw) {
        if let Err(mpsc::error::TrySendError::Full(_)) = self.served.try_send(draw) {
            self.dropped_draws.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// The payload store this mount serves from.
    pub fn store(&self) -> &Arc<dyn EprfsStorage> {
        &self.store
    }

    /// Try to index `designated`, recording a failure with its retry state.
    async fn try_index(
        &self,
        designated: &Designated,
        now: Instant,
        previous: Option<Retry>,
    ) -> Result<Held, PeerError> {
        self.reconstructions.fetch_add(1, Ordering::Relaxed);
        let manifest = &designated.manifest;
        match self
            .read_held(manifest, designated.in_scope_of, Some(&designated.resource))
            .await
        {
            Ok(held) => {
                if let Ok(mut index) = self.index.write() {
                    index.insert(held.clone());
                }
                Ok(held)
            }
            Err(reason) => {
                let retry = match &reason {
                    // Corrupt, invalid or not reconstructing its output: content addressed, so
                    // only a new custody event (a re-put) is worth another try.
                    Unindexed::ManifestInvalid { .. } => Retry {
                        event: designated.event,
                        next: None,
                        wait: INDEX_RETRY_MAX,
                    },
                    Unindexed::MethodUnknown | Unindexed::OutputUnverified { .. } => {
                        let wait = previous
                            .filter(|retry| retry.event == designated.event)
                            .map_or(INDEX_RETRY_FIRST, |retry| {
                                retry.wait.saturating_mul(2).min(INDEX_RETRY_MAX)
                            });
                        Retry {
                            event: designated.event,
                            next: Some(now + wait),
                            wait,
                        }
                    }
                };
                if let Ok(mut index) = self.index.write() {
                    index.unindexed.insert(manifest.clone(), reason.clone());
                    index.retry.insert(manifest.clone(), retry);
                }
                Err(PeerError::ArtifactUnindexed {
                    manifest: Box::new(manifest.clone()),
                    reason,
                })
            }
        }
    }

    async fn read_held(
        &self,
        manifest_cid: &BlobCid,
        in_scope_of: cid::Cid,
        output: Option<&cid::Cid>,
    ) -> Result<Held, Unindexed> {
        let invalid = |reason: String| Unindexed::ManifestInvalid { reason };
        let manifest = read_tier_manifest(self.store.as_ref(), manifest_cid)
            .await
            .map_err(|why| invalid(why.to_string()))?;
        if output.is_some_and(|output| output != manifest.record.output.as_cid()) {
            return Err(invalid(format!(
                "its Produce event names another output than {}",
                manifest.record.output
            )));
        }
        let method = manifest.record.key.method.clone();
        let descriptor = match self.store.fetch_blob(&method, FetchPolicy::LocalOnly).await {
            Ok(handle) if handle.cid == method && method.verifies(&handle.bytes) => {
                serde_ipld_dagcbor::from_slice::<MethodDescriptor>(&handle.bytes)
                    .map_err(|_| Unindexed::MethodUnknown)?
            }
            Ok(_) | Err(_) => return Err(Unindexed::MethodUnknown),
        };
        if descriptor.cid().ok().as_ref() != Some(&method) {
            return Err(Unindexed::MethodUnknown);
        }
        if descriptor.deterministic != manifest.record.deterministic {
            return Err(invalid(
                "the record's deterministic flag disagrees with its method's".into(),
            ));
        }
        // The chunks must reconstruct the output here, verified chunk by chunk and as a whole,
        // before anything of it is served. Nothing is retained: the sink only discards.
        let mut discard = |_: &TierBlobChunk, _: Bytes| Ok(());
        match stream_tier_blob(self.store.as_ref(), manifest_cid, &mut discard).await {
            TierBlobStreamDraw::Completed { .. } => {}
            TierBlobStreamDraw::Unavailable {
                why: why @ TierBlobUnavailable::ChunkMissing { .. },
                ..
            } => {
                return Err(Unindexed::OutputUnverified {
                    reason: why.to_string(),
                })
            }
            TierBlobStreamDraw::Unavailable { why, .. } => return Err(invalid(why.to_string())),
            TierBlobStreamDraw::Miss { why, .. } => return Err(invalid(why.to_string())),
        }
        Ok(Held {
            manifest: manifest_cid.clone(),
            output: manifest.record.output.clone(),
            key: crate::key_cid(&manifest.record.key).map_err(|e| invalid(e.to_string()))?,
            method,
            input: manifest.record.key.input.cid().clone(),
            method_name: descriptor.name,
            chunks: manifest
                .chunks
                .iter()
                .map(|chunk| chunk.cid.clone())
                .collect(),
            byte_len: manifest.byte_len,
            in_scope_of,
        })
    }

    /// Rebuild the index from `events` (this device's flow store, in append order), now.
    pub async fn rebuild_from_flow(&self, events: &[(cid::Cid, FlowEvent)]) -> Vec<PeerError> {
        self.rebuild_from_flow_at(events, Instant::now()).await
    }

    /// Rebuild the index from `events` as of `now`.
    ///
    /// - Every distinct local custody manifest is indexed. Event order selects only the
    ///   legacy output-only default; late indexing never changes that default.
    /// - Everything else leaves the index: entries of undesignated manifests and their
    ///   memberships, and failure records of manifests no longer designated.
    /// - A designated manifest already indexed is re-read, and dropped when it no longer reads.
    /// - One not indexed is tried, unless its last failure for the same custody event is waiting
    ///   out its backoff ([`INDEX_RETRY_FIRST`] doubling to [`INDEX_RETRY_MAX`]) or was an
    ///   invalid manifest, which only a new custody event (a re-put) tries again.
    ///
    /// Returns what could not be indexed this time, for the caller to report.
    pub async fn rebuild_from_flow_at(
        &self,
        events: &[(cid::Cid, FlowEvent)],
        now: Instant,
    ) -> Vec<PeerError> {
        let mut designated: BTreeMap<BlobCid, Designated> = BTreeMap::new();
        let mut latest = BTreeMap::new();
        for (event_cid, event) in events {
            let Some(manifest) = custody_manifest(event, &self.custodian) else {
                continue;
            };
            let Ok(output) = BlobCid::parse(&event.resource.to_string()) else {
                continue;
            };
            latest.insert(output, manifest.clone());
            designated.insert(
                manifest.clone(),
                Designated {
                    manifest,
                    event: *event_cid,
                    in_scope_of: event.in_scope_of,
                    resource: event.resource,
                },
            );
        }
        if let Ok(mut index) = self.index.write() {
            index.retain_designated(&designated);
            index.latest = latest;
        }
        let mut unindexed = Vec::new();
        for (manifest, designation) in &designated {
            let (current, previous) = self.index.read().map_or((false, None), |index| {
                (
                    index
                        .by_manifest
                        .get(manifest)
                        .is_some_and(|held| held.manifest == designation.manifest),
                    index.retry.get(&designation.manifest).cloned(),
                )
            });
            if current {
                // Revalidate: the manifest must still read as the one indexed.
                if let Err(why) =
                    read_tier_manifest(self.store.as_ref(), &designation.manifest).await
                {
                    let reason = Unindexed::ManifestInvalid {
                        reason: why.to_string(),
                    };
                    if let Ok(mut index) = self.index.write() {
                        index.remove_manifest(manifest);
                        index
                            .unindexed
                            .insert(designation.manifest.clone(), reason.clone());
                        index.retry.insert(
                            designation.manifest.clone(),
                            Retry {
                                event: designation.event,
                                next: None,
                                wait: INDEX_RETRY_MAX,
                            },
                        );
                    }
                    unindexed.push(PeerError::ArtifactUnindexed {
                        manifest: Box::new(designation.manifest.clone()),
                        reason,
                    });
                }
                continue;
            }
            let waiting = previous.as_ref().is_some_and(|retry| {
                retry.event == designation.event && retry.next.is_none_or(|next| now < next)
            });
            if waiting {
                continue;
            }
            if let Err(error) = self.try_index(designation, now, previous).await {
                unindexed.push(error);
            }
        }
        unindexed
    }

    /// How many index attempts (each one reads, and may hash, a whole artifact) this mount made.
    pub fn reconstructions(&self) -> u64 {
        self.reconstructions.load(Ordering::Relaxed)
    }

    /// How many manifests the index tracks, served, unindexed or waiting to retry: bounded by
    /// the distinct manifests of this device's custody events.
    pub fn tracked_manifests(&self) -> usize {
        self.index.read().map_or(0, |index| index.tracked())
    }

    /// A specific custody record, never substituted by another record of its output.
    fn held_record(&self, manifest: &BlobCid) -> Option<Held> {
        self.index.read().ok()?.by_manifest.get(manifest).cloned()
    }

    /// The latest custody record of `output`, when indexed (legacy output-only lookup).
    pub fn held(&self, output: &BlobCid) -> Option<Held> {
        let index = self.index.read().ok()?;
        index.by_manifest.get(index.latest.get(output)?).cloned()
    }

    /// Why the manifest `manifest` is held and not served, when it is.
    pub fn unindexed(&self, manifest: &BlobCid) -> Option<Unindexed> {
        self.index.read().ok()?.unindexed.get(manifest).cloned()
    }

    fn method_of(&self, output: &BlobCid) -> Option<String> {
        self.held(output).map(|held| held.method_name)
    }

    fn object_of(&self, artifact: &BlobCid, cid: &BlobCid) -> bool {
        self.index.read().is_ok_and(|index| {
            index.by_object.get(cid).is_some_and(|manifests| {
                manifests.iter().any(|manifest| {
                    index
                        .by_manifest
                        .get(manifest)
                        .is_some_and(|held| held.output == *artifact)
                })
            })
        })
    }

    fn methods_of(&self, cid: &BlobCid, serves: &ArtifactServeList) -> Vec<String> {
        let Ok(index) = self.index.read() else {
            return Vec::new();
        };
        index
            .by_object
            .get(cid)
            .into_iter()
            .flatten()
            .filter_map(|manifest| index.by_manifest.get(manifest))
            .filter(|held| {
                (held.method != *cid && held.input != *cid)
                    || serves.discloses_method(&held.method_name)
            })
            .map(|held| held.method_name.clone())
            .collect()
    }

    /// Close every session idle past [`SERVE_IDLE`] at `now`, as partial.
    pub fn sweep_idle(&self, now: Instant) {
        self.close_where(|session| now.duration_since(session.last) >= SERVE_IDLE);
        if let Ok(mut refused) = self.refused.lock() {
            refused.retain(|_, window| now.duration_since(window.opened) < SERVE_IDLE);
        }
    }

    /// Close every session last drawn on the connection `connection` (its stable id), as
    /// partial: the requester went away.
    pub fn connection_closed(&self, connection: usize) {
        self.close_where(|session| session.connection == connection);
    }

    fn close_where(&self, close: impl Fn(&Session) -> bool) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let closing: Vec<(String, BlobCid)> = sessions
            .iter()
            .filter(|(_, session)| close(session))
            .map(|(key, _)| key.clone())
            .collect();
        for key in closing {
            if let Some(session) = sessions.remove(&key) {
                self.send(session.draw(key.0, DrawOutcome::Partial));
            }
        }
    }

    fn record_served(
        &self,
        device: &str,
        held: &Held,
        object: &BlobCid,
        len: usize,
        connection: usize,
    ) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let key = (device.to_string(), held.manifest.clone());
        let session = sessions.entry(key.clone()).or_insert_with(|| Session {
            held: held.clone(),
            connection,
            served: BTreeSet::new(),
            bytes: 0,
            last: Instant::now(),
        });
        session.connection = connection;
        session.last = Instant::now();
        if held.chunks.contains(object) {
            session.served.insert(object.clone());
            session.bytes += len as u64;
        }
        if session.served.len() >= held.distinct_chunks() {
            if let Some(session) = sessions.remove(&key) {
                self.send(session.draw(key.0, DrawOutcome::Completed));
            }
        }
    }

    /// Record one refusal of `device`, unless the same (artifact, reason) was already recorded in
    /// its window, or its window already holds [`REFUSALS_PER_WINDOW`] (then it is only
    /// counted): a device asking for ever-new CIDs cannot grow memory, disk or the flow log.
    fn record_refused(&self, device: &str, artifact: &BlobCid, reason: String) {
        let now = Instant::now();
        let Ok(mut refused) = self.refused.lock() else {
            return;
        };
        let window = refused
            .entry(device.to_string())
            .or_insert_with(|| RefusalWindow {
                opened: now,
                recorded: BTreeSet::new(),
            });
        if now.duration_since(window.opened) >= SERVE_IDLE {
            window.opened = now;
            window.recorded.clear();
        }
        let key = (artifact.clone(), reason.clone());
        if window.recorded.contains(&key) {
            return;
        }
        if window.recorded.len() >= REFUSALS_PER_WINDOW {
            self.suppressed_refusals.fetch_add(1, Ordering::Relaxed);
            return;
        }
        window.recorded.insert(key);
        drop(refused);
        let held = self.held(artifact);
        self.send(ServedDraw {
            requester: device.to_string(),
            artifact: artifact.clone(),
            manifest: held.as_ref().map(|held| held.manifest.clone()),
            bytes: 0,
            chunks: 0,
            of: held.as_ref().map_or(0, Held::distinct_chunks),
            outcome: DrawOutcome::Refused { reason },
            in_scope_of: held.map(|held| held.in_scope_of),
        });
    }
}

/// The manifest a custody Produce event of `custodian`'s own names: `custody-blob`,
/// `copy:self`, `manifest:<cid>`, received into custody by `custodian` (this device), and not
/// folded from another device (no `via:` slot). A sibling's Produce, crossed or folded, is that
/// sibling's custody, never this device's: it names no authority to serve from here.
fn custody_manifest(event: &FlowEvent, custodian: &str) -> Option<BlobCid> {
    if event.action != ReaVerb::Produce
        || event.receiver.0 != custodian
        || via_device(event).is_some()
        || event.classified_as.first().map(String::as_str) != Some("custody-blob")
        || !event.classified_as.iter().any(|slot| slot == "copy:self")
    {
        return None;
    }
    event
        .classified_as
        .iter()
        .find_map(|slot| slot.strip_prefix("manifest:"))
        .and_then(|text| BlobCid::parse(text).ok())
}

/// The refusal reason a `draw:refused` event carries: the standing lacked, else the reason.
fn refusal_reason(refusal: &ReachRefusal) -> String {
    match &refusal.needs_standing {
        Some(standing) => standing.clone(),
        None => match refusal.reason {
            ReachReason::OutsideStanding => "outside-standing",
            ReachReason::NoRecord => "no-record",
            ReachReason::NotUnderRoot => "not-under-root",
            ReachReason::IdentityRequired => "identity-required",
            ReachReason::AuthorizerUnavailable => "authorizer-unavailable",
        }
        .to_string(),
    }
}

/// A refusal of `artifact` as the tier answers it: `why` names `refusal` as this tier's.
fn refused(self_id: &TierId, artifact: &BlobCid, refusal: ReachRefusal) -> TierAnswer {
    TierAnswer::Unanswered {
        cid: artifact.clone(),
        path: TreePath::root(),
        why: ItemAbsence::refused(self_id, refusal),
    }
}

/// The uniform refusal: what the tree endpoint answers a requester outside reach, for any root.
pub(crate) fn uniform_refusal(self_id: &TierId, artifact: &BlobCid) -> TierAnswer {
    refused(
        self_id,
        artifact,
        ReachRefusal::new(ReachReason::OutsideStanding, None),
    )
}

/// Answer an artifact request: admit the selected record's method (or legacy root), then
/// `admit_object`, then membership, before any byte is read (the tree endpoint never calls
/// `admit_object`, so this path does). A requester the gate does not admit gets the uniform
/// refusal whatever failed; an admitted device gets the typed one, and is recorded.
pub(crate) async fn answer(
    mount: Option<&BlobMount>,
    gate: &StewardDevicesGate,
    self_id: &TierId,
    who: &Requester,
    connection: usize,
    request: &TierRequest,
) -> (TierAnswer, Answered) {
    let (artifact, asked) = match request {
        TierRequest::Manifest { artifact } => (artifact, None),
        TierRequest::Blob { artifact, cid } => (artifact, Some(cid)),
        TierRequest::Record { artifact, cid, .. } => (artifact, cid.as_ref()),
        _ => {
            return (
                TierAnswer::Failed {
                    reason: "not an artifact request".into(),
                },
                Answered::Other,
            )
        }
    };
    let refuse = |refusal: ReachRefusal| -> (TierAnswer, Answered) {
        match gate.admitted(who) {
            Some(device) => {
                if let Some(mount) = mount {
                    mount.record_refused(device, artifact, refusal_reason(&refusal));
                }
                (refused(self_id, artifact, refusal), Answered::Refused)
            }
            None => (uniform_refusal(self_id, artifact), Answered::Refused),
        }
    };
    let (mount, held) = if let TierRequest::Record { manifest, key, .. } = request {
        // Admission precedes record lookup so strangers cannot inventory custody.
        if gate.admitted(who).is_none() {
            return refuse(ReachRefusal::new(ReachReason::OutsideStanding, None));
        }
        let selected = mount.and_then(|mount| Some((mount, mount.held_record(manifest)?)));
        let Some((mount, held)) = selected.filter(|(_, held)| {
            held.output == *artifact && key.as_ref().is_none_or(|key| *key == held.key)
        }) else {
            return refuse(ReachRefusal::new(
                ReachReason::NoRecord,
                Some(format!("manifest:{manifest}")),
            ));
        };
        if let Err(refusal) = gate.admit_artifact(who, &held.method_name) {
            return refuse(refusal);
        }
        (mount, held)
    } else {
        if let Err(refusal) = gate.admit_root(who, artifact) {
            return refuse(refusal);
        }
        let Some(selected) = mount.and_then(|mount| Some((mount, mount.held(artifact)?))) else {
            return refuse(ReachRefusal::new(ReachReason::OutsideStanding, None));
        };
        selected
    };
    let object = asked.unwrap_or(&held.manifest);
    if *object == held.method || *object == held.input {
        if let Err(refusal) = gate.admit_method(who, &held.method_name) {
            return refuse(refusal);
        }
    }
    if let Err(refusal) = gate.admit_object(who, object) {
        return refuse(refusal);
    }
    // Membership in the very entry whose method admitted the root, and in the index now.
    let in_held = *object == held.manifest
        || *object == held.method
        || *object == held.input
        || held.chunks.contains(object);
    if !in_held || !mount.object_of(artifact, object) {
        return refuse(ReachRefusal::new(ReachReason::NotUnderRoot, None));
    }
    // Admitted by every rule: only now is a byte read.
    let Some(device) = gate.admitted(who) else {
        return refuse(ReachRefusal::new(ReachReason::OutsideStanding, None));
    };
    match mount.store.fetch_blob(object, FetchPolicy::LocalOnly).await {
        Ok(handle) if handle.cid == *object && object.verifies(&handle.bytes) => {
            let kind = if *object == held.manifest {
                Answered::Manifest
            } else {
                Answered::Chunk
            };
            mount.record_served(device, &held, object, handle.bytes.len(), connection);
            (
                TierAnswer::Blob {
                    cid: object.clone(),
                    bytes: handle.bytes,
                },
                kind,
            )
        }
        Err(EprfsError::BlobNotFound(_) | EprfsError::BlobNotLocal(_)) => (
            TierAnswer::Unanswered {
                cid: artifact.clone(),
                path: TreePath::root(),
                why: ItemAbsence::not_held(Vec::new()),
            },
            Answered::Other,
        ),
        Ok(_) | Err(EprfsError::Corrupt(_)) => (
            TierAnswer::Failed {
                reason: format!("the held object {object} fails its address"),
            },
            Answered::Other,
        ),
        Err(other) => (
            TierAnswer::Failed {
                reason: other.to_string(),
            },
            Answered::Other,
        ),
    }
}

/// The scope a device daemon serves: its channel roots, the artifacts its mount indexes, and the
/// artifact grant, judged against admission now.
pub struct DeviceScope {
    channel: Arc<ChannelStore>,
    mount: Option<Arc<BlobMount>>,
    serves: Arc<RwLock<ArtifactServeList>>,
    admission: SharedAdmission,
}

impl DeviceScope {
    pub fn new(
        channel: Arc<ChannelStore>,
        mount: Option<Arc<BlobMount>>,
        serves: Arc<RwLock<ArtifactServeList>>,
        admission: SharedAdmission,
    ) -> DeviceScope {
        DeviceScope {
            channel,
            mount,
            serves,
            admission,
        }
    }
}

impl ServeScope for DeviceScope {
    fn classify(&self, root: &BlobCid) -> Served {
        if self.channel.roots().contains(root) {
            return Served::Channel;
        }
        match self.mount.as_ref().and_then(|mount| mount.method_of(root)) {
            Some(method) => Served::Artifact { method },
            None => Served::Unknown,
        }
    }

    fn object_of(&self, artifact: &BlobCid, cid: &BlobCid) -> bool {
        self.mount
            .as_ref()
            .is_some_and(|mount| mount.object_of(artifact, cid))
    }

    fn methods_of(&self, cid: &BlobCid) -> Vec<String> {
        self.serves
            .read()
            .map(|serves| {
                self.mount
                    .as_ref()
                    .map(|mount| mount.methods_of(cid, &serves))
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    fn discloses_method(&self, method: &str) -> bool {
        self.serves
            .read()
            .is_ok_and(|serves| serves.discloses_method(method))
    }

    fn grants(&self, device: &str, method_name: &str) -> bool {
        // Fail closed: a poisoned lock grants nothing.
        self.serves.read().is_ok_and(|serves| {
            self.admission
                .read()
                .is_ok_and(|admission| serves.grants(device, method_name, &admission))
        })
    }
}

/// Open (creating it, and any missing parent, if absent) the directory payload store at `path`,
/// as `epr peer serve` mounts it. Parents it creates are private to this account (0700), as the
/// store itself is.
///
/// # Errors
///
/// [`PeerError::LocalBlob`] when a parent or the store cannot be created, or the store opened.
pub fn open_payload_store(path: &Path) -> Result<Arc<dyn EprfsStorage>, PeerError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(parent).map_err(|source| {
            PeerError::LocalBlob(EprfsError::Io {
                path: parent.to_path_buf(),
                source,
            })
        })?;
    }
    let store = DirectoryStorage::create(path).map_err(PeerError::LocalBlob)?;
    Ok(Arc::new(OffloadedDirectory::new(store)))
}

/// `now` as an RFC 3339 UTC timestamp, to the second (`2026-11-07T19:00:00Z`).
pub(crate) fn rfc3339_utc(now: SystemTime) -> String {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_utc_reads_known_instants() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        let at = UNIX_EPOCH + Duration::from_secs(1_794_077_742);
        assert_eq!(rfc3339_utc(at), "2026-11-07T18:55:42Z");
        let leap = UNIX_EPOCH + Duration::from_secs(951_782_400);
        assert_eq!(rfc3339_utc(leap), "2000-02-29T00:00:00Z");
    }
}
