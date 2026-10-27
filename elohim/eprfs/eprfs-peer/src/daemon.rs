//! The device peer daemon behind `epr peer serve` (spec §3.2): one iroh endpoint serving the tier
//! ALPN over this device's channel tree and the steward topic's gossip, both behind admission.
//!
//! Its loop, every [`TICK`]:
//!
//! - re-reads the roster when the file changed, so a Contest written in this checkout drops its
//!   device at once, with no restart;
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

use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use elohim_epr_rea::{ParticipantRow, SidecarFlowStore, SignatureVerifier};
use eprfs_core::TreeLimits;
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
use crate::allow::CrossingAllowList;
use crate::announce::{topic_for, Channel, HeadVerdict, REANNOUNCE_EVERY};
use crate::channel::{channel_tier_id, project_channel_with_nacks, ChannelStore};
use crate::connections::ConnectionBook;
use crate::endpoint::bind_on;
use crate::error::PeerError;
use crate::fold::{fold_root, FoldLimits, FoldReader, FoldReport, FoldTarget};
use crate::gate::SharedAdmission;
use crate::gossip::GatedGossip;
use crate::head::{ChannelHead, HeadSigner};
use crate::head_table::{HeadTable, StorePlace};
use crate::key::PeerKey;
use crate::tier::{TierLimits, TierProtocol, TIER_ALPN};

/// How often the daemon looks at its roster and flow store and asks whether a head is due.
pub const TICK: Duration = Duration::from_millis(250);

/// How long a projection that failed waits before it re-reads the same state of the flow store.
pub const PROJECTION_RETRY_FIRST: Duration = Duration::from_secs(1);

/// The longest wait between retries of one failing state of the flow store.
pub const PROJECTION_RETRY_MAX: Duration = Duration::from_secs(30);

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
    pub allow: CrossingAllowList,
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
}

/// A running device daemon.
pub struct PeerDaemon {
    router: Router,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<Result<(), PeerError>>>,
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
        table.save(&config.state_dir)?;
        let store = Arc::new(ChannelStore::default());
        let limits = TierLimits::default();
        let book = ConnectionBook::default();
        book.name(roster_named_nodes(&rows, &config.chain_root));
        let tier = TierProtocol::new(
            channel_tier_id(),
            store.clone(),
            admission.clone(),
            limits.clone(),
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
            roster_stamp: stamp(&config.roster),
            flow_stamp: None,
            nacks_dirty: true,
            retry: ProjectionRetry::default(),
            tree: limits.tree,
            folds_tx,
            in_flight: HashMap::new(),
            waiting: HashMap::new(),
            config,
        };
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(run.run(receiver, folds_rx, stopped));
        Ok(PeerDaemon {
            router,
            stop: Some(stop),
            task: Some(task),
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

    /// Run until the loop ends. It ends on its own only for a reason worth a non-zero exit.
    ///
    /// # Errors
    ///
    /// [`PeerError::DaemonEnded`] naming why the loop ended (its gossip stream ended, a head could
    /// not be persisted before it was announced, or the loop task failed).
    pub async fn wait(&mut self) -> Result<(), PeerError> {
        let Some(task) = self.task.take() else {
            return Ok(());
        };
        match task.await {
            Ok(ended) => ended,
            Err(e) => Err(PeerError::DaemonEnded {
                reason: format!("the loop task failed: {e}"),
            }),
        }
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
    roster_stamp: Stamp,
    flow_stamp: Stamp,
    /// A changed nack set reprojects even when the flow log itself did not move.
    nacks_dirty: bool,
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
}

impl Run {
    async fn run(
        mut self,
        mut receiver: GossipReceiver,
        mut folds: mpsc::UnboundedReceiver<FoldDone>,
        mut stopped: oneshot::Receiver<()>,
    ) -> Result<(), PeerError> {
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let ended = loop {
            tokio::select! {
                _ = &mut stopped => break Ok(()),
                _ = tick.tick() => {
                    if let Err(e) = self.tick().await {
                        break Err(e);
                    }
                }
                Some(done) = folds.recv() => {
                    if let Err(e) = self.fold_done(done) {
                        break Err(e);
                    }
                }
                event = receiver.next() => match event {
                    Some(Ok(Event::Received(message))) => {
                        if let Err(e) = self.heard(&message.content) {
                            break Err(e);
                        }
                    }
                    Some(Ok(Event::NeighborUp(_))) => self.channel.neighbor_up(Instant::now()),
                    Some(Ok(Event::Lagged)) => {
                        // iroh-gossip closes a lagging subscription; it is to be re-opened.
                        self.log("gossip lagged; resubscribing to the steward topic");
                        match self.resubscribe().await {
                            Ok(fresh) => receiver = fresh,
                            Err(e) => break Err(ended(format!("resubscribing after lag: {e}"))),
                        }
                    }
                    Some(Ok(Event::NeighborDown(_))) => {}
                    Some(Err(e)) => break Err(ended(format!("the steward topic's gossip failed: {e}"))),
                    None => break Err(ended("the steward topic's gossip stream ended")),
                },
            }
        };
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
        self.joined = bootstrap;
        Ok(receiver)
    }

    async fn tick(&mut self) -> Result<(), PeerError> {
        self.reload_roster().await;
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
        if read {
            self.forget_unadmitted_heads();
        }
        let me = self.endpoint.node_id();
        let admitted = others(&self.admission, me);
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
            Ok(()) => self.joined.extend(fresh),
            Err(e) => self.log(&format!("newly admitted devices not joined: {e}")),
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
        if !self.nacks_dirty && now.is_some() && now == self.flow_stamp {
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
        match project_channel_with_nacks(&self.flow, &self.config.allow, &nacks) {
            Ok(tree) => {
                // Only a projection that succeeded settles this state of the log; a failed one
                // is retried after a growing wait, or at once when the log changes.
                self.flow_stamp = now;
                self.nacks_dirty = false;
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
        match self
            .channel
            .judge(&head, &admission, &*self.config.verifier)
        {
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
        let allow = self.config.allow.clone();
        let tree = self.tree;
        let done = self.folds_tx.clone();
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
                fold_root(
                    &endpoint,
                    peer,
                    target,
                    &store,
                    &allow,
                    &tree,
                    &FoldLimits::default(),
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
                let counters = self.channel.counters_mut();
                counters.oversized_events += report.oversized;
                counters.over_budget_folds += u64::from(report.over_budget);
                let nack_changed = self.channel.nack(
                    &head.device,
                    report.outcome.refused_cids.iter().map(ToString::to_string),
                    report.quarantined.iter().map(|entry| entry.cid.to_string()),
                );
                self.nacks_dirty |= nack_changed;
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
                } else {
                    self.log(&format!(
                        "folded {} seq {} in part ({} appended, {} oversized); the rest on its \
                         next head",
                        head.device, head.seq, outcome.appended, report.oversized
                    ));
                }
            }
            Err(e) => {
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
