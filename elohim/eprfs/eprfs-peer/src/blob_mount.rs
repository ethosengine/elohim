//! The artifact mount: the second answer path of a device's tier, beside its channel tree
//! (artifact-store design §1.1, §1.5).
//!
//! The channel store stays the only tree store the tier endpoint reads. This mount answers
//! [`TierRequest::Manifest`] and [`TierRequest::Blob`] from the device's payload store, for the
//! derivation artifacts this device holds custody of, and only to a device the artifact grant
//! admits ([`crate::ArtifactServeList`]). It keeps:
//!
//! - an [`ArtifactIndex`] of what it may serve: each artifact's manifest, method name and chunks.
//!   The index's source of truth is this device's own flow store — the `custody-blob` Produce
//!   events with `manifest:<cid>` and `copy:self` that [`crate::put_local_blob`] writes, daemon or
//!   no daemon — rebuilt when the store moves. A manifest whose method record is not held here is
//!   [`Unindexed::MethodUnknown`] and is never served;
//! - one `serve-blob` session per (requester, artifact), closed on the last distinct chunk, when
//!   the connection closes, or after [`SERVE_IDLE`]; each closed session (and each refusal of an
//!   admitted device) becomes one [`ServedDraw`], which the daemon records as one `Use` event.
//!   That event carries no `measure:` slot, so it never crosses.
//!
//! A stranger is only counted. Every refusal it receives is the uniform form, byte-identical to
//! the tree endpoint's, whether the artifact is granted, held ungranted, or unknown.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use elohim_epr_rea::{AgentRef, FlowEvent, ReaVerb};
use eprfs_core::{
    BlobCid, EprfsError, EprfsStorage, FetchPolicy, ItemAbsence, MethodDescriptor, ReachGate,
    ReachReason, ReachRefusal, Requester, TierId, TreePath,
};
use eprfs_storage::{read_tier_manifest, DirectoryStorage, OffloadedDirectory};
use tokio::sync::mpsc;

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

/// Why a custody manifest is held but not served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unindexed {
    /// The method descriptor its derivation record keys is not held here, or does not decode to
    /// that CID: the method's name, and so the grant, cannot be known.
    MethodUnknown,
    /// The manifest does not read, does not validate, or disagrees with its Produce event.
    ManifestInvalid { reason: String },
}

impl std::fmt::Display for Unindexed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unindexed::MethodUnknown => f.write_str("its method record is not held here"),
            Unindexed::ManifestInvalid { reason } => write!(f, "the manifest is invalid: {reason}"),
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

/// What the mount may serve, by output and by object.
#[derive(Debug, Default)]
pub struct ArtifactIndex {
    by_output: BTreeMap<BlobCid, Held>,
    /// Manifest and chunk CIDs → the outputs naming them.
    by_object: BTreeMap<BlobCid, BTreeSet<BlobCid>>,
    /// Manifests held and never served, with why.
    unindexed: BTreeMap<BlobCid, Unindexed>,
}

impl ArtifactIndex {
    fn insert(&mut self, held: Held) {
        self.unindexed.remove(&held.manifest);
        for object in std::iter::once(&held.manifest).chain(&held.chunks) {
            self.by_object
                .entry(object.clone())
                .or_default()
                .insert(held.output.clone());
        }
        self.by_output.insert(held.output.clone(), held);
    }

    fn indexed_manifest(&self, manifest: &BlobCid) -> bool {
        self.by_object.get(manifest).is_some_and(|outputs| {
            outputs.iter().any(|output| {
                self.by_output
                    .get(output)
                    .is_some_and(|h| &h.manifest == manifest)
            })
        })
    }
}

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

/// The artifact mount. See the module docs.
pub struct BlobMount {
    store: Arc<dyn EprfsStorage>,
    index: RwLock<ArtifactIndex>,
    served: mpsc::UnboundedSender<ServedDraw>,
    sessions: Mutex<HashMap<(String, BlobCid), Session>>,
    /// The last time each (requester, artifact, reason) refusal was recorded, so a device that
    /// keeps asking is recorded once per [`SERVE_IDLE`], not once per request.
    refused: Mutex<HashMap<(String, BlobCid, String), Instant>>,
}

impl std::fmt::Debug for BlobMount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobMount").finish_non_exhaustive()
    }
}

impl BlobMount {
    /// A mount over `store`, and the receiver its closed sessions arrive on.
    pub fn new(store: Arc<dyn EprfsStorage>) -> (BlobMount, mpsc::UnboundedReceiver<ServedDraw>) {
        let (served, receiver) = mpsc::unbounded_channel();
        (
            BlobMount {
                store,
                index: RwLock::new(ArtifactIndex::default()),
                served,
                sessions: Mutex::new(HashMap::new()),
                refused: Mutex::new(HashMap::new()),
            },
            receiver,
        )
    }

    /// The payload store this mount serves from.
    pub fn store(&self) -> &Arc<dyn EprfsStorage> {
        &self.store
    }

    /// Index the artifact whose manifest is `manifest_cid`, put in custody in `in_scope_of`.
    /// When `output` is given (the Produce event's resource) the manifest must name it. Fails
    /// closed: a manifest that does not read, or whose method record is not held, is recorded as
    /// unindexed and never served.
    ///
    /// # Errors
    ///
    /// [`PeerError::ArtifactUnindexed`] with [`Unindexed::MethodUnknown`] or
    /// [`Unindexed::ManifestInvalid`].
    pub async fn index_manifest(
        &self,
        manifest_cid: &BlobCid,
        in_scope_of: cid::Cid,
        output: Option<&cid::Cid>,
    ) -> Result<Held, PeerError> {
        match self.read_held(manifest_cid, in_scope_of, output).await {
            Ok(held) => {
                if let Ok(mut index) = self.index.write() {
                    index.insert(held.clone());
                }
                Ok(held)
            }
            Err(reason) => {
                if let Ok(mut index) = self.index.write() {
                    index.unindexed.insert(manifest_cid.clone(), reason.clone());
                }
                Err(PeerError::ArtifactUnindexed {
                    manifest: Box::new(manifest_cid.clone()),
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
        Ok(Held {
            manifest: manifest_cid.clone(),
            output: manifest.record.output.clone(),
            method,
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

    /// Index every custody manifest `events` (this device's flow store, in append order) put in
    /// custody here and that is not indexed yet; an unindexed one is tried again. Returns what
    /// could not be indexed, for the caller to report.
    pub async fn rebuild_from_flow(&self, events: &[(cid::Cid, FlowEvent)]) -> Vec<PeerError> {
        let mut unindexed = Vec::new();
        let mut seen = BTreeSet::new();
        for (_, event) in events {
            let Some(manifest) = custody_manifest(event) else {
                continue;
            };
            if !seen.insert(manifest.clone()) {
                continue;
            }
            let indexed = self
                .index
                .read()
                .is_ok_and(|index| index.indexed_manifest(&manifest));
            if indexed {
                continue;
            }
            if let Err(error) = self
                .index_manifest(&manifest, event.in_scope_of, Some(&event.resource))
                .await
            {
                unindexed.push(error);
            }
        }
        unindexed
    }

    /// The artifact `output` names, when indexed.
    pub fn held(&self, output: &BlobCid) -> Option<Held> {
        self.index.read().ok()?.by_output.get(output).cloned()
    }

    /// Why the manifest `manifest` is held and not served, when it is.
    pub fn unindexed(&self, manifest: &BlobCid) -> Option<Unindexed> {
        self.index.read().ok()?.unindexed.get(manifest).cloned()
    }

    fn method_of(&self, output: &BlobCid) -> Option<String> {
        self.index
            .read()
            .ok()?
            .by_output
            .get(output)
            .map(|held| held.method_name.clone())
    }

    fn object_of(&self, artifact: &BlobCid, cid: &BlobCid) -> bool {
        self.index.read().is_ok_and(|index| {
            index
                .by_object
                .get(cid)
                .is_some_and(|outputs| outputs.contains(artifact))
        })
    }

    fn methods_of(&self, cid: &BlobCid) -> Vec<String> {
        let Ok(index) = self.index.read() else {
            return Vec::new();
        };
        index
            .by_object
            .get(cid)
            .into_iter()
            .flatten()
            .filter_map(|output| index.by_output.get(output))
            .map(|held| held.method_name.clone())
            .collect()
    }

    /// Close every session idle past [`SERVE_IDLE`] at `now`, as partial.
    pub fn sweep_idle(&self, now: Instant) {
        self.close_where(|session| now.duration_since(session.last) >= SERVE_IDLE);
        if let Ok(mut refused) = self.refused.lock() {
            refused.retain(|_, at| now.duration_since(*at) < SERVE_IDLE);
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
                // The receiver is gone only when the daemon is: nothing is left to record.
                let _ = self.served.send(session.draw(key.0, DrawOutcome::Partial));
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
        let key = (device.to_string(), held.output.clone());
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
                // The receiver is gone only when the daemon is.
                let _ = self
                    .served
                    .send(session.draw(key.0, DrawOutcome::Completed));
            }
        }
    }

    fn record_refused(&self, device: &str, artifact: &BlobCid, reason: String) {
        let key = (device.to_string(), artifact.clone(), reason.clone());
        let now = Instant::now();
        let fresh = self.refused.lock().is_ok_and(|mut refused| {
            let fresh = refused
                .get(&key)
                .is_none_or(|at| now.duration_since(*at) >= SERVE_IDLE);
            if fresh {
                refused.insert(key, now);
            }
            fresh
        });
        if !fresh {
            return;
        }
        let held = self.held(artifact);
        // The receiver is gone only when the daemon is.
        let _ = self.served.send(ServedDraw {
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

/// The manifest a custody Produce event names: `custody-blob`, `copy:self`, `manifest:<cid>`.
fn custody_manifest(event: &FlowEvent) -> Option<BlobCid> {
    if event.action != ReaVerb::Produce
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

/// Answer one [`TierRequest::Manifest`] or [`TierRequest::Blob`]: `admit_root`, then
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
    if let Err(refusal) = gate.admit_root(who, artifact) {
        return refuse(refusal);
    }
    let Some((mount, held)) = mount.and_then(|mount| Some((mount, mount.held(artifact)?))) else {
        return refuse(ReachRefusal::new(ReachReason::OutsideStanding, None));
    };
    let object = asked.unwrap_or(&held.manifest);
    if let Err(refusal) = gate.admit_object(who, object) {
        return refuse(refusal);
    }
    if !mount.object_of(artifact, object) {
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
        self.mount
            .as_ref()
            .map(|mount| mount.methods_of(cid))
            .unwrap_or_default()
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

/// Open (creating it if absent) the directory payload store at `path`, as `epr peer serve`
/// mounts it.
///
/// # Errors
///
/// [`PeerError::LocalBlob`] when the directory cannot be created or opened.
pub fn open_payload_store(path: &Path) -> Result<Arc<dyn EprfsStorage>, PeerError> {
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
