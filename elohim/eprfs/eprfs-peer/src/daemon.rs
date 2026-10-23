//! The device peer daemon behind `epr peer serve` (spec §3.2): one iroh endpoint serving the tier
//! ALPN over this device's channel tree and the steward topic's gossip, both behind admission.
//!
//! Its loop, every [`TICK`]:
//!
//! - re-reads the roster when the file changed, so a Contest written in this checkout drops its
//!   device at once, with no restart;
//! - re-projects the channel tree when the flow store changed, and installs it in the store the
//!   tier serves;
//! - announces this device's signed head when one is due ([`Channel::due`]).
//!
//! Each head heard is judged ([`Channel::judge`]); a head to fold is folded from its device's
//! tier and recorded, and the head table is persisted after every change so `epr peer status` and
//! the steward lease view read what the daemon knows. Nothing here logs a key: the log sink sees
//! device did:keys, seqs, roots and counts only.

use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use elohim_epr_rea::{SidecarFlowStore, SignatureVerifier};
use eprfs_core::TreeLimits;
use futures_lite::StreamExt;
use iroh::protocol::Router;
use iroh::{Endpoint, NodeAddr, NodeId};
use iroh_gossip::api::{Event, GossipSender};
use iroh_gossip::net::Gossip;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use url::Url;

use crate::admission::{roster_rows_from_file, Admission};
use crate::allow::CrossingAllowList;
use crate::announce::{topic_for, Channel, HeadVerdict};
use crate::channel::{channel_tier_id, project_channel, ChannelStore};
use crate::endpoint::bind_on;
use crate::error::PeerError;
use crate::fold::{fold_root, FoldTarget};
use crate::gate::SharedAdmission;
use crate::gossip::GatedGossip;
use crate::head::{ChannelHead, HeadSigner};
use crate::head_table::HeadTable;
use crate::key::PeerKey;
use crate::tier::{TierLimits, TierProtocol, TIER_ALPN};

/// How often the daemon looks at its roster and flow store and asks whether a head is due.
pub const TICK: Duration = Duration::from_millis(250);

/// How long one fold may take before it is abandoned (and retried on the next head).
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
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
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
        let table = HeadTable::load(&config.state_dir)?;
        let flow = SidecarFlowStore::open(&config.flow_root).map_err(PeerError::FlowStore)?;
        let store = Arc::new(ChannelStore::default());
        let limits = TierLimits::default();
        let tier = TierProtocol::new(
            channel_tier_id(),
            store.clone(),
            admission.clone(),
            limits.clone(),
        )
        .map_err(|e| PeerError::ChannelTree {
            reason: e.to_string(),
        })?;

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
                GatedGossip::new(gossip.clone(), admission.clone()),
            )
            .spawn();

        let mut peers = HashMap::new();
        for addr in &config.peers {
            if let Err(e) = endpoint.add_node_addr(addr.clone()) {
                (config.log)(&format!("peer {}: address not added: {e}", addr.node_id));
            }
            peers.insert(addr.node_id, addr.clone());
        }
        let me = endpoint.node_id();
        let bootstrap = others(&admission, me);
        let topic = gossip
            .subscribe(topic_for(&config.chain_root), bootstrap.clone())
            .await
            .map_err(|e| PeerError::Gossip {
                reason: e.to_string(),
            })?;
        let (sender, receiver) = topic.split();

        let run = Run {
            channel: Channel::new(config.device.clone(), table),
            admission,
            flow,
            store,
            endpoint,
            sender,
            peers,
            joined: bootstrap,
            roster_stamp: stamp(&config.roster),
            flow_stamp: None,
            tree: limits.tree,
            config,
        };
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(run.run(receiver, stopped));
        Ok(PeerDaemon { router, stop, task })
    }

    /// This daemon's node id.
    pub fn node_id(&self) -> NodeId {
        self.router.endpoint().node_id()
    }

    /// The sockets the endpoint is bound to.
    pub fn bound_sockets(&self) -> Vec<SocketAddr> {
        self.router.endpoint().bound_sockets()
    }

    /// Run until the loop ends (it ends only when gossip closes).
    pub async fn wait(&mut self) {
        // A loop that panicked has ended too.
        let _ = (&mut self.task).await;
    }

    /// Stop the loop, then the router (which closes the endpoint and both protocols).
    pub async fn shutdown(self) {
        // The loop may already have ended; either way it is stopping.
        let _ = self.stop.send(());
        let _ = self.task.await;
        // Best effort: the endpoint is closing either way.
        let _ = self.router.shutdown().await;
    }
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

/// A file's length and modification time, to notice it changed.
type Stamp = Option<(u64, SystemTime)>;

fn stamp(path: &std::path::Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()?))
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

struct Run {
    config: PeerConfig,
    channel: Channel,
    admission: SharedAdmission,
    flow: SidecarFlowStore,
    store: Arc<ChannelStore>,
    endpoint: Endpoint,
    sender: GossipSender,
    peers: HashMap<NodeId, NodeAddr>,
    joined: Vec<NodeId>,
    roster_stamp: Stamp,
    flow_stamp: Stamp,
    tree: TreeLimits,
}

impl Run {
    async fn run(
        mut self,
        mut receiver: iroh_gossip::api::GossipReceiver,
        mut stopped: oneshot::Receiver<()>,
    ) {
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = &mut stopped => break,
                _ = tick.tick() => self.tick().await,
                event = receiver.next() => match event {
                    Some(Ok(Event::Received(message))) => self.heard(&message.content).await,
                    Some(Ok(Event::NeighborUp(_))) => self.channel.neighbor_up(Instant::now()),
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        self.log(&format!("gossip ended: {e}"));
                        break;
                    }
                    None => break,
                },
            }
        }
        self.save();
    }

    fn log(&self, line: &str) {
        (self.config.log)(line);
    }

    fn save(&self) {
        if let Err(e) = self.channel.table().save(&self.config.state_dir) {
            self.log(&format!("head table not saved: {e}"));
        }
    }

    async fn tick(&mut self) {
        self.reload_roster().await;
        self.reproject();
        let now = Instant::now();
        if !self.channel.due(now) {
            return;
        }
        match self.channel.announce(now, unix_ms(), &*self.config.signer) {
            Ok(Some(head)) => {
                self.broadcast(&head).await;
                self.save();
            }
            Ok(None) => {}
            Err(e) => self.log(&format!("head not signed: {e}")),
        }
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
        let rows = match roster_rows_from_file(&self.config.roster) {
            Ok(rows) => rows,
            Err(e) => {
                self.log(&format!("roster unreadable, admitting nobody: {e}"));
                Vec::new()
            }
        };
        if let Ok(mut admission) = self.admission.write() {
            admission.reload(&rows, &*self.config.verifier);
        }
        let me = self.endpoint.node_id();
        let fresh: Vec<NodeId> = others(&self.admission, me)
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

    /// Re-project the channel tree when the flow store changed.
    fn reproject(&mut self) {
        let now = stamp(self.flow.log_path());
        if now.is_some() && now == self.flow_stamp {
            return;
        }
        match project_channel(&self.flow, &self.config.allow) {
            Ok(tree) => {
                self.flow_stamp = now;
                self.store.install(&tree);
                self.channel.changed(tree.root().clone(), Instant::now());
            }
            Err(e) => self.log(&format!("channel not projected: {e}")),
        }
    }

    async fn heard(&mut self, bytes: &[u8]) {
        let head = match ChannelHead::from_wire(bytes) {
            Ok(head) => head,
            Err(_) => {
                self.channel.malformed();
                self.save();
                return;
            }
        };
        // Fail closed: a poisoned admission admits nobody.
        let admission = self.admission.read().map_or_else(
            |_| Admission::from_roster(&[], &self.config.chain_root, &*self.config.verifier),
            |admission| admission.clone(),
        );
        let verdict = self
            .channel
            .judge(&head, &admission, &*self.config.verifier);
        match verdict {
            HeadVerdict::Fold => self.fold(&head).await,
            HeadVerdict::Own => return,
            refused => self.log(&format!(
                "head from {} seq {} dropped: {refused:?}",
                head.device, head.seq
            )),
        }
        self.save();
    }

    async fn fold(&mut self, head: &ChannelHead) {
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
        let target = FoldTarget {
            device: &head.device,
            since: since.as_ref(),
            root: &head.root,
        };
        let folding = fold_root(
            &self.endpoint,
            peer,
            target,
            &self.flow,
            &self.config.allow,
            &self.tree,
        );
        match tokio::time::timeout(FOLD_DEADLINE, folding).await {
            Ok(Ok((outcome, true))) => {
                self.channel
                    .folded(head, outcome.appended, unix_ms(), Instant::now());
                if outcome.appended > 0 || outcome.refused > 0 {
                    self.log(&format!(
                        "folded {} seq {}: {} appended, {} already, {} refused",
                        head.device, head.seq, outcome.appended, outcome.already, outcome.refused
                    ));
                }
            }
            Ok(Ok((outcome, false))) => self.log(&format!(
                "folded {} seq {} in part ({} appended); the rest on its next head",
                head.device, head.seq, outcome.appended
            )),
            Ok(Err(e)) => self.log(&format!(
                "fold of {} seq {} failed, retried on its next head: {e}",
                head.device, head.seq
            )),
            Err(_) => self.log(&format!(
                "fold of {} seq {} timed out, retried on its next head",
                head.device, head.seq
            )),
        }
    }
}
