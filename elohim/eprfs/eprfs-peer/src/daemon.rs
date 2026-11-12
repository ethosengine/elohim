//! The device peer daemon behind `epr peer serve` (spec §3.2): one iroh endpoint serving the tier
//! ALPN over this device's channel tree and the steward topic's gossip, both behind admission.
//!
//! Its loop, every [`TICK`]:
//!
//! - re-reads the roster when the file changed, so a Contest written in this checkout drops its
//!   device at once, with no restart;
//! - re-reads the policy when it changed, keeping the last loaded rules on an unreadable or
//!   malformed replacement and re-projecting when the enforced rules change;
//! - re-indexes the artifact mount, when the daemon has a payload store, from the custody
//!   manifests the flow store records, and records each closed `serve-blob` session as one Use
//!   event ([`BlobMount`]);
//! - re-projects the channel tree when the flow store changed, and installs it in the store the
//!   tier serves (a failure is recorded in the head table, where `epr peer status` reads it, and
//!   the same state of the store is retried with a growing wait: [`ProjectionRetry`]);
//! - announces this device's signed head when one is due ([`Channel::due`]), persisting it first:
//!   a head whose seq could not be persisted is never sent, and the loop ends saying why.
//!
//! Each head heard is judged ([`Channel::judge`]) on the loop. A head to fold is folded by a task
//! of its own, at most one per device: a newer head for a device whose fold is in flight waits,
//! coalesced to the newest, and the result comes back to the loop to be recorded. A slow or
//! silent device therefore never holds back another device's fold, this device's announce, or a
//! roster reload. The head table is persisted after every change so `epr peer status` and the
//! steward lease view read what the daemon knows. Nothing here logs a key: the log sink sees
//! device did:keys, seqs, roots and counts only.

use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use elohim_epr_rea::{FlowRecord, FlowStore, ParticipantRow, SidecarFlowStore, SignatureVerifier};
use eprfs_core::{EprfsStorage, TreeLimits};
use futures_lite::StreamExt;
use iroh::protocol::Router;
use iroh::{Endpoint, NodeAddr, NodeId};
use iroh_gossip::api::{Event, GossipReceiver, GossipSender};
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use url::Url;

use crate::admission::{roster_rows_from_file, Admission};
use crate::announce::{topic_for, Channel, HeadVerdict, REANNOUNCE_EVERY};
use crate::blob_mount::{rfc3339_utc, BlobMount, DeviceScope, ServedDraw, INDEX_RETRY_FIRST};
use crate::channel::{channel_tier_id, project_channel_with_nacks, ChannelStore};
use crate::connections::ConnectionBook;
use crate::endpoint::bind_on;
use crate::error::PeerError;
use crate::fetch::{
    admitted_holders, ensure_queue_dir, fetch_authorized_until, fetch_cancel_path, fetch_done_path,
    fetch_queue_dir, install_fetched, queue_dir_is_private, read_queue_file, write_queue_file,
    FetchDone, FetchFor, FetchGap, FetchRequest, InstallScope,
};
use crate::fold::{fold_root_planned, FoldLimits, FoldPlan, FoldReader, FoldReport, FoldTarget};
use crate::gate::SharedAdmission;
use crate::gossip::GatedGossip;
use crate::head::{ChannelHead, HeadSigner};
use crate::head_table::{HeadTable, StorePlace};
use crate::key::PeerKey;
use crate::local_blob::LocalBlobContext;
use crate::rules::{Rules, RulesState};
use crate::serves::ArtifactServeList;
use crate::tier::{TierLimits, TierProtocol, TIER_ALPN};

/// How often the daemon looks at its roster and flow store and asks whether a head is due.
pub const TICK: Duration = Duration::from_millis(250);

/// How long a projection that failed waits before it re-reads the same state of the flow store.
pub const PROJECTION_RETRY_FIRST: Duration = Duration::from_secs(1);

/// The longest wait between retries of one failing state of the flow store.
pub const PROJECTION_RETRY_MAX: Duration = Duration::from_secs(30);

/// The first wait after a dropped neighbour before another join attempt.
pub const REJOIN_RETRY_FIRST: Duration = Duration::from_secs(2);
/// The longest wait between join attempts for one dropped neighbour.
pub const REJOIN_RETRY_MAX: Duration = Duration::from_secs(60);

/// The first wait before replacing a failed gossip subscription.
pub const STREAM_RETRY_FIRST: Duration = Duration::from_secs(1);
/// The longest wait between attempts to replace a failed gossip subscription.
pub const STREAM_RETRY_MAX: Duration = Duration::from_secs(60);

/// How often stale admitted devices are considered for re-join, and how old their last head may
/// be before the link is refreshed.
const REJOIN_SWEEP_EVERY: Duration = Duration::from_secs(30);
const HEAD_STALE_AFTER: Duration = Duration::from_secs(90);

/// A pure capped retry clock. Scheduling consumes the current delay; resetting restores the
/// first delay. The owner decides what success means (a join call is not yet a neighbour-up).
#[derive(Debug, Clone)]
pub struct RetryBackoff {
    first: Duration,
    max: Duration,
    next_delay: Duration,
    at: Option<Instant>,
}

impl RetryBackoff {
    /// A retry clock beginning at `first` and doubling no farther than `max`.
    pub fn new(first: Duration, max: Duration) -> RetryBackoff {
        RetryBackoff {
            first,
            max,
            next_delay: first,
            at: None,
        }
    }

    /// Schedule after the current delay and double the following one up to the cap.
    pub fn schedule(&mut self, now: Instant) -> Duration {
        let delay = self.next_delay;
        self.at = Some(now + delay);
        self.next_delay = delay.saturating_mul(2).min(self.max);
        delay
    }

    /// Schedule on the current tick without consuming a delay (used by the stale-head sweep).
    pub fn schedule_now(&mut self, now: Instant) {
        self.at = Some(now);
    }

    #[must_use]
    /// Whether the scheduled retry is due at `now`.
    pub fn due(&self, now: Instant) -> bool {
        self.at.is_some_and(|at| now >= at)
    }

    fn scheduled(&self) -> bool {
        self.at.is_some()
    }

    /// A join request was accepted; wait for neighbour-up, a head, or the stale-head sweep.
    pub fn requested(&mut self) {
        self.at = None;
    }

    /// Forget the schedule and restore the first delay.
    pub fn reset(&mut self) {
        self.next_delay = self.first;
        self.at = None;
    }
}

#[derive(Debug)]
struct RejoinState {
    retry: RetryBackoff,
    failure_logged: bool,
}

impl RejoinState {
    fn delayed(now: Instant) -> RejoinState {
        let mut retry = RetryBackoff::new(REJOIN_RETRY_FIRST, REJOIN_RETRY_MAX);
        retry.schedule(now);
        RejoinState {
            retry,
            failure_logged: false,
        }
    }

    fn immediate(now: Instant) -> RejoinState {
        let mut retry = RetryBackoff::new(REJOIN_RETRY_FIRST, REJOIN_RETRY_MAX);
        retry.schedule_now(now);
        RejoinState {
            retry,
            failure_logged: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GossipDisposition {
    Handle,
    ResubscribeLater,
}

/// What a gossip event means for the subscription: a lag, an error or the end of the stream all
/// drop the receiver and resubscribe on the tick with backoff, never end the daemon.
fn gossip_disposition<E>(event: Option<&Result<Event, E>>) -> GossipDisposition {
    match event {
        Some(Ok(Event::Lagged)) | Some(Err(_)) | None => GossipDisposition::ResubscribeLater,
        Some(Ok(_)) => GossipDisposition::Handle,
    }
}

/// In-memory evidence of fold work, deliberately absent from heads and the persisted table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FoldStats {
    /// Fold tasks started; unchanged roots do not increment this.
    pub tasks: u64,
    /// Calls to `folded_from` made to build a device's lazy held-CID index.
    pub store_parses: u64,
}

#[derive(Debug, Default)]
struct RunStats {
    tasks: AtomicU64,
    store_parses: AtomicU64,
}

/// When a projection that failed is tried again. A projection re-parses the whole flow store, so a
/// state of the store that keeps failing is retried after [`PROJECTION_RETRY_FIRST`], then after
/// twice as long each time up to [`PROJECTION_RETRY_MAX`], not on every tick. A change to the
/// store (a new key) is projected at once and starts the wait over; a success forgets the failure.
#[derive(Debug, Clone)]
pub struct ProjectionRetry<K> {
    failing: Option<(K, Duration, Instant)>,
}

impl<K> Default for ProjectionRetry<K> {
    fn default() -> Self {
        ProjectionRetry { failing: None }
    }
}

impl<K: PartialEq> ProjectionRetry<K> {
    /// Whether the state `key` is to be projected at `now`: always for a state that has not
    /// failed, and for the failing one once its wait is over.
    #[must_use]
    pub fn due(&self, key: &K, now: Instant) -> bool {
        match &self.failing {
            Some((failed, _, next)) if failed == key => now >= *next,
            _ => true,
        }
    }

    /// Record that projecting `key` failed at `now`. True when this is the state's first failure
    /// (count and report it once); the wait doubles while the same state keeps failing.
    pub fn failed(&mut self, key: K, now: Instant) -> bool {
        let (first, wait) = match &self.failing {
            Some((failed, wait, _)) if *failed == key => {
                (false, wait.saturating_mul(2).min(PROJECTION_RETRY_MAX))
            }
            _ => (true, PROJECTION_RETRY_FIRST),
        };
        self.failing = Some((key, wait, now + wait));
        first
    }

    /// Forget the failure: the store projected.
    pub fn succeeded(&mut self) {
        self.failing = None;
    }
}

/// How long one fold may take before it is abandoned (and retried on the device's next head). A
/// fold runs off the loop, so this bounds only that device's own fold.
pub const FOLD_DEADLINE: Duration = Duration::from_secs(30);

/// What a device daemon needs. The keys stay with their owners: the peer key moves into the
/// endpoint, and the device key is reached only through `signer`.
pub struct PeerConfig {
    /// This device's iroh key.
    pub key: PeerKey,
    /// This device's did:key.
    pub device: String,
    /// The lineage's chain root: the topic and admission are this lineage's.
    pub chain_root: String,
    /// The checkout whose `.eprfs/status/flows.jsonl` is this device's flow store.
    pub flow_root: PathBuf,
    /// The roster file admission is read from, in this device's own checkout.
    pub roster: PathBuf,
    /// Where the head table is persisted.
    pub state_dir: PathBuf,
    /// The crossing and steward-scope rules this device enforces.
    pub rules: Rules,
    /// The `pool-policy.json` the rules are reloaded from when it changes; `None` holds them fixed
    /// and records no reloadable rules state in the head table.
    pub policy: Option<PathBuf>,
    /// The relay to home to; `None` for direct addresses only.
    pub relay: Option<Url>,
    /// A fixed address to listen on; an ephemeral port when `None`.
    pub listen: Option<SocketAddrV4>,
    /// Direct addresses of other devices (`--peer <node_id>@<addr>`). The roster names node ids
    /// only.
    pub peers: Vec<NodeAddr>,
    /// The device key's signature over a head.
    pub signer: Arc<HeadSigner>,
    /// The did:key verifier (epr-cli's device-key verifier).
    pub verifier: Arc<dyn SignatureVerifier + Send + Sync>,
    /// Where the daemon reports what it did, one line at a time.
    pub log: Arc<dyn Fn(&str) + Send + Sync>,
    /// The device's payload store, whose custody artifacts the tier serves to the devices
    /// `rules.serves()` grants; `None` for a channel-only daemon, which serves no artifact.
    pub payload_store: Option<Arc<dyn EprfsStorage>>,
    /// Where a queued sibling fetch may install its artifact; the default admits nothing.
    pub install: InstallScope,
}

/// A running device daemon.
pub struct PeerDaemon {
    router: Router,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<Result<(), PeerError>>>,
    stats: Arc<RunStats>,
}

impl PeerDaemon {
    /// Bind the endpoint, mount the tier and gossip behind admission, join the steward topic and
    /// start the loop.
    ///
    /// # Errors
    ///
    /// The roster, head table, flow store and bind errors, and [`PeerError::Gossip`] when the
    /// topic cannot be joined.
    pub async fn start(config: PeerConfig) -> Result<PeerDaemon, PeerError> {
        let rows = roster_rows_from_file(&config.roster)?;
        let admission: SharedAdmission = Arc::new(RwLock::new(Admission::from_roster(
            &rows,
            &config.chain_root,
            &*config.verifier,
        )));
        let mut table = HeadTable::load(&config.state_dir)?;
        let flow = SidecarFlowStore::open(&config.flow_root).map_err(PeerError::FlowStore)?;
        // The one flow store this device's channel is projected from: recorded before anything
        // is announced, so the lease view and berth read and write this store, not their own.
        table.store = Some(store_place(&config.flow_root, flow.log_path()));
        if let Some(policy) = &config.policy {
            table.rules = Some(RulesState {
                digest: config.rules.digest(),
                policy: std::fs::canonicalize(policy).unwrap_or_else(|_| policy.clone()),
                loaded_at: unix_ms(),
                pending: None,
            });
        }
        table.save(&config.state_dir)?;
        // The fetch queue is this device's own: created 0700 before anything can be queued.
        ensure_queue_dir(&fetch_queue_dir(&config.state_dir))
            .map_err(|e| ended(format!("the fetch queue cannot be created: {e}")))?;
        let store = Arc::new(ChannelStore::default());
        let limits = TierLimits::default();
        let book = ConnectionBook::default();
        book.name(roster_named_nodes(&rows, &config.chain_root));
        let serves = Arc::new(RwLock::new(config.rules.serves().clone()));
        let (mount, served_rx) = match &config.payload_store {
            Some(payload) => {
                let (mount, served_rx) = BlobMount::new(payload.clone(), config.device.clone());
                (Some(Arc::new(mount)), Some(served_rx))
            }
            None => (None, None),
        };
        let scope = DeviceScope::new(
            store.clone(),
            mount.clone(),
            serves.clone(),
            admission.clone(),
        );
        let tier = TierProtocol::with_scope(
            channel_tier_id(),
            store.clone(),
            admission.clone(),
            limits.clone(),
            Arc::new(scope),
            mount.clone(),
        )
        .map_err(|e| PeerError::ChannelTree {
            reason: e.to_string(),
        })?
        .with_book(book.clone());

        let endpoint = bind_on(
            &config.key,
            config.relay.clone(),
            vec![TIER_ALPN.to_vec(), iroh_gossip::ALPN.to_vec()],
            config.listen,
        )
        .await?;
        let gossip = Gossip::builder().spawn(endpoint.clone());
        let router = Router::builder(endpoint.clone())
            .accept(TIER_ALPN, tier)
            .accept(
                iroh_gossip::ALPN,
                GatedGossip::new(gossip.clone(), admission.clone()).with_book(book.clone()),
            )
            .spawn();

        let mut peers = HashMap::new();
        for addr in &config.peers {
            peers.insert(addr.node_id, addr.clone());
        }
        let me = endpoint.node_id();
        let bootstrap = others(&admission, me);
        // Before the topic is joined: the first head is found through these addresses.
        seed_endpoint(
            &endpoint,
            config.relay.as_ref(),
            &bootstrap,
            &peers,
            &*config.log,
        );
        let topic = topic_for(&config.chain_root);
        let (sender, receiver) = subscribe(&gossip, topic, bootstrap.clone()).await?;
        let (folds_tx, folds_rx) = mpsc::unbounded_channel();

        let stats = Arc::new(RunStats::default());
        let run = Run {
            channel: Channel::new(config.device.clone(), table),
            admission,
            book,
            refused_saved: None,
            flow,
            store,
            endpoint,
            gossip,
            topic,
            sender,
            peers,
            joined: bootstrap,
            rejoins: HashMap::new(),
            last_heard: HashMap::new(),
            last_rejoin_sweep: Instant::now(),
            resubscriptions: 0,
            roster_stamp: stamp(&config.roster),
            policy_stamp: config.policy.as_deref().and_then(stamp),
            flow_stamp: None,
            reproject: true,
            retry: ProjectionRetry::default(),
            tree: limits.tree,
            folds_tx,
            in_flight: HashMap::new(),
            waiting: HashMap::new(),
            held: HashMap::new(),
            stats: stats.clone(),
            mount,
            served_rx,
            index_stamp: None,
            indexed_once: false,
            index_task: None,
            index_retry_at: None,
            serves,
            fetch_task: None,
            queue_pruned: None,
            queue_open_logged: false,
            config,
        };
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(run.run(receiver, folds_rx, stopped));
        Ok(PeerDaemon {
            router,
            stop: Some(stop),
            task: Some(task),
            stats,
        })
    }

    /// This daemon's node id.
    pub fn node_id(&self) -> NodeId {
        self.router.endpoint().node_id()
    }

    /// The sockets the endpoint is bound to.
    pub fn bound_sockets(&self) -> Vec<SocketAddr> {
        self.router.endpoint().bound_sockets()
    }

    /// In-memory fold work since this daemon started. This is a test/diagnostic surface only;
    /// neither counter enters a head or the persisted head table.
    pub fn fold_stats(&self) -> FoldStats {
        FoldStats {
            tasks: self.stats.tasks.load(Ordering::Relaxed),
            store_parses: self.stats.store_parses.load(Ordering::Relaxed),
        }
    }

    /// Run until the loop ends. It ends on its own only for a reason worth a non-zero exit.
    /// Cancelling this wait retains the loop's join handle so [`Self::shutdown`] can still join it.
    ///
    /// # Errors
    ///
    /// [`PeerError::DaemonEnded`] naming a local fault (a head could not be persisted before it
    /// was announced, or the loop task failed). Gossip loss is recovered in-process.
    pub async fn wait(&mut self) -> Result<(), PeerError> {
        wait_for_task(&mut self.task).await
    }

    /// Stop the loop, then the router (which closes the endpoint and both protocols).
    pub async fn shutdown(mut self) {
        if let Some(stop) = self.stop.take() {
            // The loop may already have ended; either way it is stopping.
            let _ = stop.send(());
        }
        // Stopped on request: how the loop ended is no longer news.
        let _ = self.wait().await;
        // Best effort: the endpoint is closing either way.
        let _ = self.router.shutdown().await;
    }
}

async fn wait_for_task(
    task: &mut Option<JoinHandle<Result<(), PeerError>>>,
) -> Result<(), PeerError> {
    let Some(task_handle) = task.as_mut() else {
        return Ok(());
    };
    let joined = task_handle.await;
    *task = None;
    match joined {
        Ok(ended) => ended,
        Err(error) => Err(PeerError::DaemonEnded {
            reason: format!("the loop task failed: {error}"),
        }),
    }
}

async fn subscribe(
    gossip: &Gossip,
    topic: TopicId,
    bootstrap: Vec<NodeId>,
) -> Result<(GossipSender, GossipReceiver), PeerError> {
    gossip
        .subscribe(topic, bootstrap)
        .await
        .map(|topic| topic.split())
        .map_err(|e| PeerError::Gossip {
            reason: e.to_string(),
        })
}

/// The admitted node ids other than this device's own.
fn others(admission: &SharedAdmission, me: NodeId) -> Vec<NodeId> {
    admission
        .read()
        .map(|admission| admission.nodes())
        .unwrap_or_default()
        .into_iter()
        .filter(|node| *node != me)
        .collect()
}

/// Tell `endpoint` how to reach every device in `admitted`, and every `--peer` address given.
///
/// With a relay configured, each admitted node id becomes a [`NodeAddr`] carrying that relay URL
/// (merged with its `--peer` direct addresses, when it has them), so a gossip join or a tier
/// dial to a bare roster node id finds the peer through our own relay with no `--peer`. With no
/// relay a node known only by its id stays unaddressed, as before.
fn seed_endpoint(
    endpoint: &Endpoint,
    relay: Option<&Url>,
    admitted: &[NodeId],
    peers: &HashMap<NodeId, NodeAddr>,
    log: &(dyn Fn(&str) + Send + Sync),
) {
    let mut addrs: HashMap<NodeId, NodeAddr> = peers.clone();
    if let Some(relay) = relay {
        for node in admitted {
            addrs.entry(*node).or_insert_with(|| NodeAddr::new(*node));
        }
        for addr in addrs.values_mut() {
            addr.relay_url = Some(relay.clone().into());
        }
    }
    for addr in addrs.into_values() {
        let node = addr.node_id;
        if let Err(e) = endpoint.add_node_addr(addr) {
            log(&format!("peer {node}: address not added: {e}"));
        }
    }
}

/// How long answered and withdrawn queue files are kept.
const QUEUE_RETAIN: Duration = Duration::from_secs(3600);

/// The oldest-named queued fetch request in `queue` and its stem (`<key>-<nonce>`): a `.json`
/// entry, never an answer (`.done.json`). An entry that is not a regular file (a link, a FIFO, a
/// device) is removed without being opened.
fn next_request(
    queue: &std::path::Path,
    log: &(dyn Fn(&str) + Send + Sync),
) -> Option<(PathBuf, String)> {
    let mut requests: Vec<(PathBuf, String)> = Vec::new();
    for entry in std::fs::read_dir(queue).ok()?.filter_map(Result::ok) {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Some(stem) = name.strip_suffix(".json") else {
            continue;
        };
        if stem.ends_with(".done") {
            continue;
        }
        let path = entry.path();
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_file() => requests.push((path, stem.to_string())),
            _ => {
                log(&format!(
                    "fetch queue entry {name} is not a regular file; removed unread"
                ));
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    requests.sort();
    requests.into_iter().next()
}

/// A file's length and modification time, to notice it changed.
type Stamp = Option<(u64, SystemTime)>;

fn stamp(path: &std::path::Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()?))
}

/// Where the channel store is, by canonical path (as given when a path cannot be resolved).
fn store_place(root: &std::path::Path, log: &std::path::Path) -> StorePlace {
    let canonical =
        |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    StorePlace {
        root: canonical(root),
        log: canonical(log),
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

fn ended(reason: impl Into<String>) -> PeerError {
    PeerError::DaemonEnded {
        reason: reason.into(),
    }
}

/// A fold task's result, back on the loop.
struct FoldDone {
    head: ChannelHead,
    result: Result<FoldReport, PeerError>,
}

struct Run {
    config: PeerConfig,
    channel: Channel,
    admission: SharedAdmission,
    /// The connections the tier and gossip accepted, and those they refused.
    book: ConnectionBook,
    /// When refused-connection counts were last written on their own.
    refused_saved: Option<Instant>,
    flow: SidecarFlowStore,
    store: Arc<ChannelStore>,
    endpoint: Endpoint,
    gossip: Gossip,
    topic: TopicId,
    sender: GossipSender,
    peers: HashMap<NodeId, NodeAddr>,
    joined: Vec<NodeId>,
    /// Dropped or stale admitted nodes awaiting another join request.
    rejoins: HashMap<NodeId, RejoinState>,
    /// Last verified head received per admitted device, deliberately not persisted.
    last_heard: HashMap<String, Instant>,
    last_rejoin_sweep: Instant,
    /// Successful in-process replacements of the gossip subscription.
    resubscriptions: u64,
    roster_stamp: Stamp,
    policy_stamp: Stamp,
    flow_stamp: Stamp,
    /// A changed rule or nack set reprojects even when the flow log itself did not move.
    reproject: bool,
    /// The state of the log the last failed projection read, counted once and retried with a
    /// growing wait rather than re-parsed every tick.
    retry: ProjectionRetry<Stamp>,
    tree: TreeLimits,
    folds_tx: mpsc::UnboundedSender<FoldDone>,
    /// Per device, the seq of the head whose fold is in flight, and its task (aborted when a
    /// reload drops the device).
    in_flight: HashMap<String, (u64, tokio::task::AbortHandle)>,
    /// Per device, the newest head that arrived while its fold was in flight.
    waiting: HashMap<String, ChannelHead>,
    /// Original event CIDs held from each device, rebuilt lazily after start or any uncertain fold.
    held: HashMap<String, Arc<HashSet<cid::Cid>>>,
    stats: Arc<RunStats>,
    /// The artifact mount, when this device has a payload store.
    mount: Option<Arc<BlobMount>>,
    /// Closed `serve-blob` sessions, recorded on the tick.
    served_rx: Option<mpsc::Receiver<ServedDraw>>,
    /// The state of the flow log the mount was last indexed from.
    index_stamp: Stamp,
    indexed_once: bool,
    /// The one rebuild of the mount in flight, off the loop: reading and hashing artifacts never
    /// delays a tick (a roster or policy reload, a projection, an announce).
    index_task: Option<JoinHandle<Vec<PeerError>>>,
    /// When a rebuild next falls due with no change to the log, so a manifest whose backoff has
    /// run out is tried again.
    index_retry_at: Option<Instant>,
    /// The artifact grant the tier's scope reads; replaced when the rules reload.
    serves: Arc<RwLock<ArtifactServeList>>,
    /// The one queued sibling fetch in flight, off the loop (artifact-store design §3.b.2): its
    /// task, its queue stem and who its answer is for.
    fetch_task: Option<(JoinHandle<()>, String, FetchFor)>,
    /// When answered and withdrawn queue files were last pruned.
    queue_pruned: Option<Instant>,
    /// Whether a queue directory that is not private has been reported.
    queue_open_logged: bool,
}

impl Run {
    async fn run(
        mut self,
        receiver: GossipReceiver,
        mut folds: mpsc::UnboundedReceiver<FoldDone>,
        mut stopped: oneshot::Receiver<()>,
    ) -> Result<(), PeerError> {
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut receiver = Some(receiver);
        let mut stream_retry = RetryBackoff::new(STREAM_RETRY_FIRST, STREAM_RETRY_MAX);
        let mut stream_failure_logged = false;
        let mut resubscribe_failure_logged = false;
        let ended = loop {
            tokio::select! {
                _ = &mut stopped => break Ok(()),
                _ = tick.tick() => {
                    let now = Instant::now();
                    if receiver.is_none() && stream_retry.due(now) {
                        match self.resubscribe().await {
                            Ok(fresh) => {
                                receiver = Some(fresh);
                                stream_retry.reset();
                                stream_failure_logged = false;
                                resubscribe_failure_logged = false;
                                self.resubscriptions += 1;
                                self.log(&format!(
                                    "gossip resubscribed ({} in this run)",
                                    self.resubscriptions
                                ));
                            }
                            Err(e) => {
                                if !resubscribe_failure_logged {
                                    self.log(&format!("gossip resubscribe failed: {e}"));
                                    resubscribe_failure_logged = true;
                                }
                                stream_retry.schedule(now);
                            }
                        }
                    }
                    if let Err(e) = self.tick().await {
                        break Err(e);
                    }
                }
                Some(done) = folds.recv() => {
                    if let Err(e) = self.fold_done(done) {
                        break Err(e);
                    }
                }
                event = async {
                    match receiver.as_mut() {
                        Some(receiver) => receiver.next().await,
                        None => std::future::pending().await,
                    }
                } => {
                    let disposition = gossip_disposition(event.as_ref());
                    match event {
                    Some(Ok(Event::Received(message))) => {
                        if let Err(e) = self.heard(&message.content) {
                            break Err(e);
                        }
                    }
                    Some(Ok(Event::NeighborUp(node))) => self.neighbor_up(node),
                    Some(Ok(Event::Lagged)) => {
                        if !stream_failure_logged {
                            self.log("gossip lagged; resubscribing to the steward topic");
                        }
                    }
                    Some(Ok(Event::NeighborDown(node))) => self.neighbor_down(node),
                    Some(Err(e)) => {
                        if !stream_failure_logged {
                            self.log(&format!("the steward topic's gossip failed: {e}"));
                        }
                    }
                    None => {
                        if !stream_failure_logged {
                            self.log("the steward topic's gossip stream ended");
                        }
                    }
                    }
                    if disposition == GossipDisposition::ResubscribeLater {
                        receiver = None;
                        stream_failure_logged = true;
                        resubscribe_failure_logged = false;
                        stream_retry.reset();
                        stream_retry.schedule(Instant::now());
                    }
                },
            }
        };
        if let Some(task) = self.index_task.take() {
            task.abort();
        }
        if let Some((task, _, _)) = self.fetch_task.take() {
            task.abort();
        }
        // Best effort on the way out: the counters are worth keeping, the head is already saved.
        let _ = self.save();
        ended
    }

    fn log(&self, line: &str) {
        (self.config.log)(line);
    }

    /// Persist the head table. Every caller treats a failure as the end of the loop: a daemon
    /// that cannot persist what it announced must not announce.
    fn save(&self) -> Result<(), PeerError> {
        self.channel
            .table()
            .save(&self.config.state_dir)
            .map_err(|e| ended(format!("the head table cannot be persisted: {e}")))
    }

    async fn resubscribe(&mut self) -> Result<GossipReceiver, PeerError> {
        let bootstrap = others(&self.admission, self.endpoint.node_id());
        let (sender, receiver) = subscribe(&self.gossip, self.topic, bootstrap.clone()).await?;
        self.sender = sender;
        for node in &bootstrap {
            if let Some(state) = self.rejoins.get_mut(node) {
                state.retry.requested();
            }
        }
        self.joined = bootstrap;
        Ok(receiver)
    }

    fn neighbor_up(&mut self, node: NodeId) {
        let admitted = self
            .admission
            .read()
            .is_ok_and(|admission| admission.admits(&node).is_some());
        if admitted {
            self.rejoins.remove(&node);
            if !self.joined.contains(&node) {
                self.joined.push(node);
            }
        }
        self.channel.neighbor_up(Instant::now());
    }

    fn neighbor_down(&mut self, node: NodeId) {
        let admitted = self
            .admission
            .read()
            .is_ok_and(|admission| admission.admits(&node).is_some());
        if !admitted {
            return;
        }
        self.joined.retain(|joined| *joined != node);
        let now = Instant::now();
        let state = self
            .rejoins
            .entry(node)
            .or_insert_with(|| RejoinState::delayed(now));
        if !state.retry.scheduled() {
            state.retry.schedule(now);
        }
    }

    fn heard_device(&mut self, device: &str, now: Instant) {
        self.last_heard.insert(device.to_string(), now);
        let nodes = self
            .admission
            .read()
            .map(|admission| {
                admission
                    .nodes()
                    .into_iter()
                    .filter(|node| admission.admits(node) == Some(device))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for node in nodes {
            self.rejoins.remove(&node);
            if !self.joined.contains(&node) {
                self.joined.push(node);
            }
        }
    }

    async fn maintain_rejoins(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_rejoin_sweep) >= REJOIN_SWEEP_EVERY {
            self.last_rejoin_sweep = now;
            let stale: Vec<NodeId> = self
                .admission
                .read()
                .map(|admission| {
                    admission
                        .nodes()
                        .into_iter()
                        .filter(|node| *node != self.endpoint.node_id())
                        .filter(|node| {
                            admission.admits(node).is_some_and(|device| {
                                self.last_heard.get(device).is_none_or(|heard| {
                                    now.duration_since(*heard) >= HEAD_STALE_AFTER
                                })
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            for node in stale {
                self.joined.retain(|joined| *joined != node);
                let state = self
                    .rejoins
                    .entry(node)
                    .or_insert_with(|| RejoinState::immediate(now));
                if !state.retry.scheduled() {
                    state.retry.schedule_now(now);
                }
            }
        }

        let due: Vec<NodeId> = self
            .rejoins
            .iter()
            .filter_map(|(node, state)| state.retry.due(now).then_some(*node))
            .collect();
        for node in due {
            let admitted = self
                .admission
                .read()
                .is_ok_and(|admission| admission.admits(&node).is_some());
            if !admitted {
                self.rejoins.remove(&node);
                continue;
            }
            match self.sender.join_peers(vec![node]).await {
                Ok(()) => {
                    if !self.joined.contains(&node) {
                        self.joined.push(node);
                    }
                    if let Some(state) = self.rejoins.get_mut(&node) {
                        state.retry.requested();
                    }
                }
                Err(e) => {
                    let first_failure = {
                        let state = self
                            .rejoins
                            .entry(node)
                            .or_insert_with(|| RejoinState::delayed(now));
                        let first = !state.failure_logged;
                        state.failure_logged = true;
                        state.retry.schedule(now);
                        first
                    };
                    if first_failure {
                        self.log(&format!("neighbour {node} not re-joined: {e}"));
                    }
                }
            }
        }
    }

    async fn tick(&mut self) -> Result<(), PeerError> {
        self.reload_roster().await;
        self.reload_rules()?;
        self.reindex().await;
        self.record_serves();
        self.fetch_queued();
        self.maintain_rejoins().await;
        // Refused connections reach the head table `epr peer status` reads: kept in memory every
        // tick, written with the next announce, or on their own at most once per re-announce
        // interval — never once per refused connection.
        let (refused, strangers) = (self.book.refused_counts(), self.book.refused_strangers());
        let counters = &self.channel.table().channel;
        if counters.refused_connections != refused || counters.refused_strangers != strangers {
            let counters = self.channel.counters_mut();
            counters.refused_connections = refused;
            counters.refused_strangers = strangers;
            let now = Instant::now();
            if self
                .refused_saved
                .is_none_or(|at| now.duration_since(at) >= REANNOUNCE_EVERY)
            {
                self.refused_saved = Some(now);
                self.save()?;
            }
        }
        self.reproject()?;
        let now = Instant::now();
        if !self.channel.due(now) {
            return Ok(());
        }
        let head = match self.channel.announce(now, unix_ms(), &*self.config.signer) {
            Ok(Some(head)) => head,
            Ok(None) => return Ok(()),
            Err(e) => {
                self.log(&format!("head not signed: {e}"));
                return Ok(());
            }
        };
        // Persisted before it is sent: a seq that did not reach the disk is never announced.
        self.save()?;
        self.broadcast(&head).await;
        Ok(())
    }

    async fn broadcast(&self, head: &ChannelHead) {
        let sent = match head.to_wire() {
            Ok(wire) => self
                .sender
                .broadcast(wire.into())
                .await
                .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        if let Err(e) = sent {
            self.log(&format!("head seq {} not announced: {e}", head.seq));
        }
    }

    /// Re-read admission when the roster file changed. A roster that no longer reads admits
    /// nobody (fail closed) until it does.
    async fn reload_roster(&mut self) {
        let now = stamp(&self.config.roster);
        if now == self.roster_stamp {
            return;
        }
        self.roster_stamp = now;
        let (rows, read) = match roster_rows_from_file(&self.config.roster) {
            Ok(rows) => (rows, true),
            Err(e) => {
                self.log(&format!("roster unreadable, admitting nobody: {e}"));
                (Vec::new(), false)
            }
        };
        self.book
            .name(roster_named_nodes(&rows, &self.config.chain_root));
        if let Ok(mut admission) = self.admission.write() {
            admission.reload(&rows, &*self.config.verifier);
            // A device the reload dropped keeps nothing it had open here.
            let closed = self.book.close_unadmitted(&admission);
            if closed > 0 {
                self.log(&format!(
                    "closed {closed} connection(s) of devices no longer admitted"
                ));
            }
        }
        self.drop_unadmitted_folds();
        let admission = self.admission.clone();
        let admitted_device = |device: &str| {
            admission
                .read()
                .is_ok_and(|admission| admission.admits_device(device))
        };
        self.held.retain(|device, _| admitted_device(device));
        self.last_heard.retain(|device, _| admitted_device(device));
        if read {
            self.forget_unadmitted_heads();
        }
        let me = self.endpoint.node_id();
        let admitted = others(&self.admission, me);
        self.joined.retain(|node| admitted.contains(node));
        self.rejoins.retain(|node, _| admitted.contains(node));
        seed_endpoint(
            &self.endpoint,
            self.config.relay.as_ref(),
            &admitted,
            &self.peers,
            &*self.config.log,
        );
        let fresh: Vec<NodeId> = admitted
            .into_iter()
            .filter(|node| !self.joined.contains(node))
            .collect();
        if fresh.is_empty() {
            return;
        }
        match self.sender.join_peers(fresh.clone()).await {
            Ok(()) => {
                for node in fresh {
                    self.joined.push(node);
                    if let Some(state) = self.rejoins.get_mut(&node) {
                        state.retry.requested();
                    }
                }
            }
            Err(e) => {
                self.log(&format!("newly admitted devices not joined: {e}"));
                let now = Instant::now();
                for node in fresh {
                    self.rejoins
                        .entry(node)
                        .or_insert_with(|| RejoinState::delayed(now));
                }
            }
        }
    }

    /// Re-read the enforced rule sets when their policy file changed. A bad replacement keeps
    /// the loaded crossing and steward rules and is recorded for status readers, but revokes
    /// serving: the artifact grant is empty (deny) until a policy that reads loads again. A grant
    /// is never carried past a policy that no longer says it.
    fn reload_rules(&mut self) -> Result<(), PeerError> {
        let Some(policy) = self.config.policy.clone() else {
            return Ok(());
        };
        let now = stamp(&policy);
        if now == self.policy_stamp {
            return Ok(());
        }
        self.policy_stamp = now;
        match Rules::from_policy_file(&policy) {
            Ok(rules) => {
                // Restored on every policy that reads, a revocation's included.
                if let Ok(mut serves) = self.serves.write() {
                    *serves = rules.serves().clone();
                }
                let old = self.config.rules.digest();
                let new = rules.digest();
                if old == new {
                    let pending = self
                        .channel
                        .rules_state_mut()
                        .as_mut()
                        .is_some_and(|state| state.pending.take().is_some());
                    if pending {
                        self.save()?;
                    }
                    return Ok(());
                }
                self.log(&format!(
                    "rules reloaded: {} -> {}",
                    old.short(),
                    new.short()
                ));
                self.config.rules = rules;
                self.reproject = true;
                *self.channel.rules_state_mut() = Some(RulesState {
                    digest: new,
                    policy: std::fs::canonicalize(&policy).unwrap_or(policy),
                    loaded_at: unix_ms(),
                    pending: None,
                });
                self.save()
            }
            Err(error) => {
                // Fail closed for serving, every time the file fails to read.
                if let Ok(mut serves) = self.serves.write() {
                    *serves = ArtifactServeList::default();
                }
                let reason = error.to_string();
                let digest = self.config.rules.digest();
                let already_pending = self
                    .channel
                    .table()
                    .rules
                    .as_ref()
                    .is_some_and(|state| state.pending.as_deref() == Some(reason.as_str()));
                if already_pending {
                    return Ok(());
                }
                self.log(&format!(
                    "rules not reloaded, keeping {} and serving no artifact until a policy \
                     reads: {reason}",
                    digest.short()
                ));
                if let Some(state) = self.channel.rules_state_mut().as_mut() {
                    state.pending = Some(reason);
                }
                self.save()
            }
        }
    }

    /// Re-index the artifact mount from the flow store when the log moved, on a task of its own
    /// and one at a time: a finished rebuild reports what it could not index, and a log that
    /// moved meanwhile is rebuilt next. The tick itself only reads the log.
    async fn reindex(&mut self) {
        let Some(mount) = self.mount.clone() else {
            return;
        };
        if self
            .index_task
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
        {
            if let Some(task) = self.index_task.take() {
                match task.await {
                    Ok(unindexed) => {
                        for error in unindexed {
                            self.log(&error.to_string());
                        }
                    }
                    Err(e) => self.log(&format!("artifact index rebuild failed: {e}")),
                }
            }
        }
        if self.index_task.is_some() {
            return;
        }
        let now = stamp(self.flow.log_path());
        // A rebuild also falls due when a failed manifest's backoff may have run out.
        let retry_due = self.index_retry_at.is_some_and(|at| Instant::now() >= at);
        if self.indexed_once && now == self.index_stamp && !retry_due {
            return;
        }
        // Settled per state of the log, failed or not: the next append is the next try.
        self.index_stamp = now;
        self.indexed_once = true;
        self.index_retry_at = Some(Instant::now() + INDEX_RETRY_FIRST);
        let events = match self.flow.events() {
            Ok(events) => events,
            Err(e) => {
                self.log(&format!(
                    "artifacts not indexed: the flow store failed: {e}"
                ));
                return;
            }
        };
        self.index_task = Some(tokio::spawn(async move {
            mount.rebuild_from_flow(&events).await
        }));
    }

    /// Close idle `serve-blob` sessions and record every closed one as one Use event provided by
    /// this device to its requester. A record that cannot be appended is reported, never retried
    /// into a second event.
    fn record_serves(&mut self) {
        let (Some(mount), Some(served_rx)) = (self.mount.as_ref(), self.served_rx.as_mut()) else {
            return;
        };
        mount.sweep_idle(Instant::now());
        let mut draws = Vec::new();
        while let Ok(draw) = served_rx.try_recv() {
            draws.push(draw);
        }
        for draw in draws {
            let event = draw.use_event(&self.config.device, rfc3339_utc(SystemTime::now()));
            if let Err(source) = self.flow.append(FlowRecord::Event(event)) {
                let error = PeerError::ServeUnrecorded {
                    artifact: Box::new(draw.artifact.clone()),
                    source,
                };
                self.log(&error.to_string());
            }
        }
    }

    /// Start the next fetch `epr artifact fetch` queued in `<state dir>/fetch-queue`, one at a
    /// time and on a task of its own (design §3.b.2): only this daemon holds the device's peer key,
    /// so only it dials.
    ///
    /// The queue is read only while it is private (0700). An entry is read only when it is a
    /// regular file within the size cap, opened without following a link and without blocking;
    /// anything else is removed unread. A request is claimed (removed) before it runs, is answered
    /// in `<key>-<nonce>.done.json` naming its nonce, key, output and destination, and is withdrawn
    /// by `<key>-<nonce>.cancel`. Holders are re-resolved under this daemon's admission now, the
    /// destination must lie in its install scope, and every integrity rule is `fetch_artifact`'s
    /// and `install_fetched`'s, under one deadline from the first dial through the install.
    fn fetch_queued(&mut self) {
        let queue = fetch_queue_dir(&self.config.state_dir);
        if let Some((task, stem, for_request)) = &self.fetch_task {
            if !task.is_finished() {
                let cancel = fetch_cancel_path(&queue, stem);
                if std::fs::symlink_metadata(&cancel).is_ok() {
                    task.abort();
                    let _ = std::fs::remove_file(&cancel);
                    let (stem, for_request) = (stem.clone(), for_request.clone());
                    self.write_done(
                        &fetch_done_path(&queue, &stem),
                        &FetchDone::gap(for_request, FetchGap::Cancelled),
                    );
                    self.log(&format!("fetch {stem} withdrawn by its requester"));
                    self.fetch_task = None;
                }
                return;
            }
        }
        self.fetch_task = None;
        if !queue_dir_is_private(&queue) {
            if !self.queue_open_logged {
                self.queue_open_logged = true;
                self.log(&format!(
                    "the fetch queue {} is not a private (0700) directory; nothing in it is read",
                    queue.display()
                ));
            }
            return;
        }
        self.queue_open_logged = false;
        self.prune_queue(&queue);
        let Some((path, stem)) = next_request(&queue, &*self.config.log) else {
            return;
        };
        let read = read_queue_file(&path);
        // Claimed: a request runs once, whatever it holds.
        let _ = std::fs::remove_file(&path);
        let request = match read.and_then(|bytes| {
            serde_json::from_slice::<FetchRequest>(&bytes).map_err(|e| e.to_string())
        }) {
            Ok(request) => request,
            Err(e) => {
                // Unreadable: nobody can be answered, because nothing names who asked.
                self.log(&format!("fetch request {stem} dropped: {e}"));
                return;
            }
        };
        let for_request = FetchFor::of(&request);
        let done = fetch_done_path(&queue, &stem);
        let refused = |reason: String| {
            FetchDone::gap(
                for_request.clone(),
                FetchGap::InstallRefused {
                    dest: request.dest.display().to_string(),
                    reason,
                },
            )
        };
        if request.stem() != stem || request.authority.validate(&request.output).is_err() {
            let answer =
                refused("the request's key and nonce do not name the file it was queued in".into());
            self.write_done(&done, &answer);
            return;
        }
        if request
            .expect_output
            .as_ref()
            .is_some_and(|output| output != &request.output)
        {
            let answer =
                refused("the selected output differs from the caller's expected output".into());
            self.write_done(&done, &answer);
            return;
        }
        let Some(mount) = self.mount.clone() else {
            self.write_done(
                &done,
                &FetchDone::gap(for_request.clone(), FetchGap::DaemonDown),
            );
            return;
        };
        let holders = admitted_holders(&request.holders, &self.config.device, |device| {
            let node = self.admission.read().ok()?.node_for(device)?;
            Some(self.peers.get(&node).cloned().unwrap_or_else(|| {
                let addr = NodeAddr::new(node);
                match &self.config.relay {
                    Some(relay) => addr.with_relay_url(relay.clone().into()),
                    None => addr,
                }
            }))
        });
        if request.inputs_only {
            let Some(key) = request.authority.key().cloned() else {
                self.write_done(
                    &done,
                    &refused("input documents require a derivation key".into()),
                );
                return;
            };
            let endpoint = self.endpoint.clone();
            let log = self.config.log.clone();
            let answer_for = for_request.clone();
            let task = tokio::spawn(async move {
                let outcome = crate::fetch::fetch_input_document(
                    &endpoint,
                    &key,
                    &request.output,
                    &holders,
                    &request.budget(),
                )
                .await;
                let result = match outcome {
                    Ok(bytes) => crate::fetch::put_input_document(
                        mount.store().as_ref(),
                        key.input.cid(),
                        bytes,
                    )
                    .await
                    .map_err(|e| FetchGap::LocalStore {
                        reason: e.to_string(),
                    }),
                    Err(gap) => Err(gap),
                };
                let answer = match result {
                    Ok(cid) => FetchDone::InputDocument {
                        for_request: answer_for,
                        cid,
                    },
                    Err(gap) => FetchDone::gap(answer_for, gap),
                };
                if let Err(error) = write_queue_file(&done, &answer) {
                    log(&format!(
                        "input document answer {} not written: {error}",
                        done.display()
                    ));
                }
            });
            self.fetch_task = Some((task, stem, for_request));
            return;
        }
        let dest = match self.config.install.admits(&request.dest) {
            Ok(dest) => dest,
            Err(reason) => {
                let answer = refused(reason);
                self.write_done(&done, &answer);
                return;
            }
        };
        let Ok(scope) = request.scope.parse::<cid::Cid>() else {
            let answer = refused(format!("the queued scope {} is not a CID", request.scope));
            self.write_done(&done, &answer);
            return;
        };
        let endpoint = self.endpoint.clone();
        let flow_root = self.config.flow_root.clone();
        let device = self.config.device.clone();
        let log = self.config.log.clone();
        self.log(&format!(
            "fetching {} from {} holder(s) into {}",
            request.output,
            holders.len(),
            dest.display()
        ));
        let answer_for = for_request.clone();
        let task = tokio::spawn(async move {
            let store = mount.store().clone();
            let budget = request.budget();
            let deadline = Instant::now() + budget.total;
            let outcome = async {
                let fetched = fetch_authorized_until(
                    &endpoint,
                    store.as_ref(),
                    &request.authority,
                    &request.output,
                    &holders,
                    &budget,
                    deadline,
                )
                .await?;
                let mut flow =
                    SidecarFlowStore::open(&flow_root).map_err(|e| FetchGap::LocalStore {
                        reason: e.to_string(),
                    })?;
                let context = LocalBlobContext {
                    custodian: elohim_epr_rea::AgentRef(device),
                    in_scope_of: scope,
                    occurred_at: rfc3339_utc(SystemTime::now()),
                    crossing: None,
                };
                let installing =
                    install_fetched(store.as_ref(), &mut flow, &fetched, &context, &dest);
                let until = tokio::time::Instant::from_std(deadline);
                match tokio::time::timeout_at(until, installing).await {
                    Ok(installed) => installed?,
                    Err(_) => {
                        return Err(FetchGap::DeadlineExceeded {
                            fetched: fetched.manifest.byte_len,
                            total: fetched.manifest.byte_len,
                        })
                    }
                }
                Ok::<_, FetchGap>(fetched)
            }
            .await;
            let written = match &outcome {
                Ok(fetched) => FetchDone::installed(answer_for.clone(), fetched),
                Err(gap) => FetchDone::gap(answer_for.clone(), gap.clone()),
            };
            match &outcome {
                Ok(fetched) => log(&format!(
                    "fetched {} ({} chunks, {} resumed, {} bytes wasted)",
                    request.output, fetched.chunks_fetched, fetched.chunks_resumed, fetched.wasted
                )),
                Err(gap) => log(&format!("fetch of {} gap: {gap}", request.output)),
            }
            if let Err(e) = write_queue_file(&done, &written) {
                log(&format!("fetch answer {} not written: {e}", done.display()));
            }
        });
        self.fetch_task = Some((task, stem, for_request));
    }

    /// Remove answers and withdrawals older than [`QUEUE_RETAIN`] (nobody waits that long), at
    /// most once per [`REANNOUNCE_EVERY`].
    fn prune_queue(&mut self, queue: &std::path::Path) {
        let now = Instant::now();
        if self
            .queue_pruned
            .is_some_and(|at| now.duration_since(at) < REANNOUNCE_EVERY)
        {
            return;
        }
        self.queue_pruned = Some(now);
        let Ok(entries) = std::fs::read_dir(queue) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !(name.ends_with(".done.json") || name.ends_with(".cancel")) {
                continue;
            }
            let old = entry
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|at| at.elapsed().ok())
                .is_some_and(|age| age >= QUEUE_RETAIN);
            if old {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    fn write_done(&self, path: &std::path::Path, done: &FetchDone) {
        if let Err(e) = write_queue_file(path, done) {
            self.log(&format!("fetch answer {} not written: {e}", path.display()));
        }
    }

    /// Whether `device` is admitted now. Fail closed: a poisoned admission admits nobody.
    fn admitted(&self, device: &str) -> bool {
        self.admission
            .read()
            .is_ok_and(|admission| admission.admits_device(device))
    }

    /// Abort the in-flight fold, and forget the waiting head, of every device admission no
    /// longer admits: a Contest stops what that device was still sending.
    fn drop_unadmitted_folds(&mut self) {
        let dropped: Vec<String> = self
            .in_flight
            .keys()
            .chain(self.waiting.keys())
            .filter(|device| !self.admitted(device))
            .cloned()
            .collect();
        for device in dropped {
            if let Some((seq, task)) = self.in_flight.remove(&device) {
                task.abort();
                self.log(&format!(
                    "fold of {device} seq {seq} stopped: the device is no longer admitted"
                ));
            }
            self.waiting.remove(&device);
            self.held.remove(&device);
            self.last_heard.remove(&device);
        }
    }

    /// Drop from the head table every device the roster no longer admits, and persist it: its
    /// last seq is no longer carried in this device's heads (spent of `MAX_HEAD_BYTES`) or kept.
    fn forget_unadmitted_heads(&mut self) {
        let admission = self.admission.clone();
        let admitted = |device: &str| {
            admission
                .read()
                .is_ok_and(|admission| admission.admits_device(device))
        };
        let forgotten = self.channel.forget_unadmitted(admitted);
        if forgotten.is_empty() {
            return;
        }
        self.log(&format!(
            "forgot the last head of {} device(s) no longer admitted: {}",
            forgotten.len(),
            forgotten.join(", ")
        ));
        if let Err(e) = self.save() {
            self.log(&format!("head table not saved after forgetting: {e}"));
        }
    }

    /// Re-project the channel tree when the flow store changed. A failure is recorded where
    /// `epr peer status` reads it; the last good root stays served and announced.
    fn reproject(&mut self) -> Result<(), PeerError> {
        let now = stamp(self.flow.log_path());
        if !self.reproject && now.is_some() && now == self.flow_stamp {
            return Ok(());
        }
        if !self.retry.due(&now, Instant::now()) {
            return Ok(());
        }
        let nacks = self
            .channel
            .table()
            .nacks
            .iter()
            .map(|(device, state)| (device.clone(), state.leaf()))
            .collect();
        match project_channel_with_nacks(&self.flow, self.config.rules.allow(), &nacks) {
            Ok(tree) => {
                // Only a projection that succeeded settles this state of the log; a failed one
                // is retried after a growing wait, or at once when the log changes.
                self.flow_stamp = now;
                self.reproject = false;
                self.retry.succeeded();
                self.store.install(&tree);
                self.channel.changed(tree.root().clone(), Instant::now());
                self.channel
                    .covers(tree.read(), tree.last().map(ToString::to_string));
                if self
                    .channel
                    .counters_mut()
                    .last_projection_error
                    .take()
                    .is_some()
                {
                    self.save()?;
                }
            }
            Err(e) => {
                // Counted and saved once per state of the log, though retried.
                if self.retry.failed(now, Instant::now()) {
                    self.log(&format!("channel not projected: {e}"));
                    let counters = self.channel.counters_mut();
                    counters.projection_failures += 1;
                    counters.last_projection_error = Some(e.to_string());
                    self.save()?;
                }
            }
        }
        Ok(())
    }

    fn heard(&mut self, bytes: &[u8]) -> Result<(), PeerError> {
        let Ok(head) = ChannelHead::from_wire(bytes) else {
            self.channel.malformed();
            return self.save();
        };
        // Fail closed: a poisoned admission admits nobody.
        let admission = self.admission.read().map_or_else(
            |_| Admission::from_roster(&[], &self.config.chain_root, &*self.config.verifier),
            |admission| admission.clone(),
        );
        let verdict = self
            .channel
            .judge(&head, &admission, &*self.config.verifier);
        if matches!(
            verdict,
            HeadVerdict::Fold | HeadVerdict::Rollback | HeadVerdict::Replayed
        ) {
            self.heard_device(&head.device, Instant::now());
        }
        match verdict {
            HeadVerdict::Fold => self.fold(head),
            HeadVerdict::Own => return Ok(()),
            refused => self.log(&format!(
                "head from {} seq {} dropped: {refused:?}",
                head.device, head.seq
            )),
        }
        self.save()
    }

    /// Fold `head` on a task of its own, or keep it as the device's next fold when one is in
    /// flight (only the newest waits).
    fn fold(&mut self, head: ChannelHead) {
        if let Some(&(in_flight, _)) = self.in_flight.get(&head.device) {
            let newer = head.seq > in_flight
                && self
                    .waiting
                    .get(&head.device)
                    .is_none_or(|waiting| head.seq > waiting.seq);
            if newer {
                self.waiting.insert(head.device.clone(), head);
            }
            return;
        }
        let unchanged = self
            .channel
            .last_folded(&head.device)
            .is_some_and(|(_, root)| root == &head.root);
        if unchanged {
            let nack = self
                .channel
                .last_peer_nack(&head.device)
                .cloned()
                .unwrap_or_default();
            self.channel
                .folded(&head, 0, nack, unix_ms(), Instant::now());
            return;
        }
        let node = self
            .admission
            .read()
            .ok()
            .and_then(|admission| admission.node_for(&head.device));
        let Some(node) = node else {
            return;
        };
        let peer = self.peers.get(&node).cloned().unwrap_or_else(|| {
            let addr = NodeAddr::new(node);
            match &self.config.relay {
                Some(relay) => addr.with_relay_url(relay.clone().into()),
                None => addr,
            }
        });
        let since = self
            .channel
            .last_folded(&head.device)
            .map(|(_, root)| root.clone());
        let seq = head.seq;
        let device = head.device.clone();
        let admission = self.admission.clone();
        let endpoint = self.endpoint.clone();
        let flow_root = self.config.flow_root.clone();
        let reader = self.config.device.clone();
        let state_dir = self.config.state_dir.clone();
        let allow = self.config.rules.allow().clone();
        let tree = self.tree;
        let done = self.folds_tx.clone();
        let known = self.held.get(&device).cloned();
        self.stats.tasks.fetch_add(1, Ordering::Relaxed);
        let task = tokio::spawn(async move {
            let folding = async {
                let store = SidecarFlowStore::open(&flow_root).map_err(PeerError::FlowStore)?;
                let target = FoldTarget {
                    device: &head.device,
                    since: since.as_ref(),
                    root: &head.root,
                    // Re-checked under the append lock: a Contest folded meanwhile appends nothing.
                    admission: Some(&admission),
                    reader: FoldReader {
                        device: &reader,
                        state_dir: &state_dir,
                    },
                };
                fold_root_planned(
                    &endpoint,
                    peer,
                    target,
                    &store,
                    &allow,
                    &tree,
                    FoldPlan {
                        limits: FoldLimits::default(),
                        known: known.as_deref(),
                    },
                )
                .await
            };
            let result = tokio::time::timeout(FOLD_DEADLINE, folding)
                .await
                .unwrap_or_else(|_| {
                    Err(PeerError::FoldUnanswered {
                        what: format!("root {} (timed out)", head.root),
                    })
                });
            // The loop has stopped when nobody receives; nothing is left to record.
            let _ = done.send(FoldDone { head, result });
        });
        self.in_flight.insert(device, (seq, task.abort_handle()));
    }

    /// Record a finished fold, then start the device's waiting head, if any.
    fn fold_done(&mut self, FoldDone { head, result }: FoldDone) -> Result<(), PeerError> {
        self.in_flight.remove(&head.device);
        if !self.admitted(&head.device) {
            // Contested while the fold ran: no head is recorded and no acknowledgement advances.
            self.waiting.remove(&head.device);
            self.log(&format!(
                "fold of {} seq {} discarded: the device is no longer admitted",
                head.device, head.seq
            ));
            return Ok(());
        }
        match result {
            Ok(report) => {
                self.stats
                    .store_parses
                    .fetch_add(report.store_parses, Ordering::Relaxed);
                let counters = self.channel.counters_mut();
                counters.oversized_events += report.oversized;
                counters.over_budget_folds += u64::from(report.over_budget);
                let nack_changed = self.channel.nack(
                    &head.device,
                    report.outcome.refused_cids.iter().map(ToString::to_string),
                    report.quarantined.iter().map(|entry| entry.cid.to_string()),
                );
                self.reproject |= nack_changed;
                if report.complete {
                    self.held
                        .insert(head.device.clone(), Arc::new(report.known.clone()));
                } else {
                    self.held.remove(&head.device);
                }
                let outcome = report.outcome;
                if report.complete {
                    self.channel.folded(
                        &head,
                        outcome.appended,
                        report.peer_nack,
                        unix_ms(),
                        Instant::now(),
                    );
                    if outcome.appended > 0 || outcome.refused > 0 {
                        self.log(&format!(
                            "folded {} seq {}: {} appended, {} already, {} refused",
                            head.device,
                            head.seq,
                            outcome.appended,
                            outcome.already,
                            outcome.refused
                        ));
                    }
                } else if report.quarantine_blocks {
                    self.log(&format!(
                        "folded {} seq {} in part ({} appended, {} quarantined); held at the \
                         prior head until that device's daemon reads nacks",
                        head.device,
                        head.seq,
                        outcome.appended,
                        report.quarantined.len()
                    ));
                } else if report.over_budget {
                    self.log(&format!(
                        "folded {} seq {} in part ({} appended, byte budget reached); the rest \
                         on its next head",
                        head.device, head.seq, outcome.appended
                    ));
                } else {
                    self.log(&format!(
                        "folded {} seq {} in part ({} appended, event bound reached); the rest \
                         on its next head",
                        head.device, head.seq, outcome.appended
                    ));
                }
            }
            Err(e) => {
                self.held.remove(&head.device);
                self.channel.counters_mut().fold_failures += 1;
                self.log(&format!(
                    "fold of {} seq {} failed, retried on its next head: {e}",
                    head.device, head.seq
                ));
            }
        }
        let folded = self
            .channel
            .last_folded(&head.device)
            .map_or(0, |(seq, _)| seq);
        if let Some(next) = self.waiting.remove(&head.device) {
            if next.seq > folded {
                self.fold(next);
            }
        }
        self.save()
    }
}

/// Every node id a `PeerEndpoint` row of `chain_root`'s lineage names (64 lowercase hex), admitted
/// or not: the node ids whose refused connections are counted one by one. Bounded by the roster.
fn roster_named_nodes(rows: &[ParticipantRow], chain_root: &str) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            ParticipantRow::PeerEndpoint {
                chain_root: root,
                node_id,
                ..
            } if root == chain_root => Some(node_id.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelling_wait_keeps_the_daemon_task_joinable() {
        let (release, held) = oneshot::channel();
        let mut task = Some(tokio::spawn(async move {
            let _ = held.await;
            Ok(())
        }));

        assert!(
            tokio::time::timeout(Duration::from_millis(1), wait_for_task(&mut task))
                .await
                .is_err(),
            "the first wait remains pending"
        );
        assert!(task.is_some(), "cancellation retains the join handle");

        release.send(()).expect("release daemon task");
        wait_for_task(&mut task).await.expect("join daemon task");
        assert!(task.is_none(), "a completed wait consumes the join handle");
    }

    fn delays(first: Duration) -> Vec<Duration> {
        let now = Instant::now();
        let mut retry = RetryBackoff::new(first, Duration::from_secs(60));
        (0..8).map(|_| retry.schedule(now)).collect()
    }

    #[test]
    fn retry_backoffs_double_to_sixty_seconds_and_reset() {
        assert_eq!(
            delays(REJOIN_RETRY_FIRST),
            [2, 4, 8, 16, 32, 60, 60, 60].map(Duration::from_secs)
        );
        assert_eq!(
            delays(STREAM_RETRY_FIRST),
            [1, 2, 4, 8, 16, 32, 60, 60].map(Duration::from_secs)
        );

        let now = Instant::now();
        let mut retry = RetryBackoff::new(REJOIN_RETRY_FIRST, REJOIN_RETRY_MAX);
        retry.schedule(now);
        retry.schedule(now);
        retry.reset();
        assert_eq!(retry.schedule(now), REJOIN_RETRY_FIRST);
    }

    #[test]
    fn broken_gossip_streams_always_resubscribe_later() {
        let failed: Result<Event, ()> = Err(());
        let lagged: Result<Event, ()> = Ok(Event::Lagged);
        let ended: Option<&Result<Event, ()>> = None;
        assert_eq!(
            gossip_disposition(Some(&failed)),
            GossipDisposition::ResubscribeLater
        );
        assert_eq!(
            gossip_disposition(Some(&lagged)),
            GossipDisposition::ResubscribeLater
        );
        assert_eq!(
            gossip_disposition(ended),
            GossipDisposition::ResubscribeLater
        );
    }
}
