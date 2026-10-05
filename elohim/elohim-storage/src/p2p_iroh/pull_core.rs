//! The pull leg for pure-iroh mode: inventory discovery, replication gaps, pin
//! acquisition, and the `/p2p/status.pull` block — without a libp2p `P2PNode`.
//!
//! Measured 2026-08-28 (`homo-iroh` warm recovery): the iroh doc-sync driver
//! refilled a wiped DocStore (P0 green, 15 changes applied) and NOTHING turned
//! the docs into content rows (P1–P4 red, `contentCount 0`) because every
//! consumer of a peer's inventory — the replication scheduler, the gap queue,
//! the acquisition reconcile and dispatch, the pull status — lives inside the
//! libp2p `P2PNode`, which `transport_backend = Iroh` never constructs. This
//! module is that loop, driven by the iroh shard ALPN instead of the libp2p
//! shard protocol, over the SAME state types (`ReplicationState`,
//! `AcquisitionState`, the shared `store_acquired_record` ingest) so the two
//! planes settle the same trackers the same way. It exists only when no
//! libp2p node does; in `dual` mode `P2PNode` owns the loop and dispatches its
//! iroh targets through the same `acquire_over_iroh`.
//!
//! What it mirrors, step by step (libp2p original in `p2p/mod.rs`):
//! - `hydrate_replication_state` → [`IrohPullCore::hydrate_local_ids`]
//! - `run_replication_cycle` + the `ContentList` arm → [`IrohPullCore::discover_from_peers`]
//!   (ListContent pages per book peer → `readmit_pins_named_by_inventory` →
//!   `ReplicationState::discover` → gap queue)
//! - `drain_gap_queue` → [`IrohPullCore::drain_gaps`] (`GetContent` over iroh, `PullKind::Gap`)
//! - `run_acquisition_reconcile` → [`IrohPullCore::reconcile_pins`]
//! - `drain_acquisition_queue` → [`IrohPullCore::drain_pins`] (`PullKind::Pin`)
//! - `refresh_status` (the `replication` + `pull` fields) → [`IrohPullCore::status`]
//!
//! Bounded work (C6a): one ListContent page in flight per peer, gap dispatch
//! capped at [`MAX_REPLICATION_INFLIGHT`], pin dispatch at
//! `acquisition::MAX_ACQUISITION_INFLIGHT`, every request under a timeout, and a
//! per-peer backoff after a failed ListContent so an unreachable peer costs one
//! request per cadence, not a storm.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use iroh::{NodeAddr, NodeId};
use tokio::sync::{watch, Mutex};
use tracing::{debug, info, warn};

use crate::blob_store::BlobStore;
use crate::db::DbPool;
use crate::p2p::acquisition::{self, AcquisitionState, PullStatusInfo};
use crate::p2p::announce_fetch::AnnounceFetch;
use crate::p2p::replication::{ReplicationState, ReplicationStatus};
use crate::p2p::shard_protocol::{ShardRequest, ShardResponse};
use crate::p2p::{acquire_over_iroh, AcquisitionIngestCtx, PullKind};
use crate::p2p_iroh::{iroh_fetch_leg, IrohShardClient};
use crate::services::head_adoption_trigger::TriggerGate;

/// Same cap as the libp2p `drain_gap_queue`.
pub const MAX_REPLICATION_INFLIGHT: usize = 50;
/// Same page size as the libp2p replication cycle.
const LIST_PAGE_LIMIT: u32 = 1000;
const LIST_TIMEOUT: Duration = Duration::from_secs(30);
/// After a failed ListContent, leave the peer alone for this long.
const LIST_BACKOFF: Duration = Duration::from_secs(120);

/// The `/p2p/status` projection of the pull leg on iroh — the fields the
/// recovery harness (P3) and the seeder's caughtUp poll read.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrohPullStatus {
    pub replication: ReplicationStatus,
    pub pull: Option<PullStatusInfo>,
    pub iroh_peers_known: usize,
    pub gap_queue: usize,
    pub acquisition_queue: usize,
}

pub struct IrohPullCore {
    ctx: AcquisitionIngestCtx,
    gap_queue: Mutex<VecDeque<String>>,
    acquisition_queue: Mutex<VecDeque<String>>,
    gap_in_flight: Arc<AtomicUsize>,
    pin_in_flight: Arc<AtomicUsize>,
    rotation: AtomicUsize,
    // bounded-work: one ListContent walk per book peer per `list_every` tick, each walk
    // page-bounded by `next_doc_list_offset` (MAX_SYNC_LIST_OFFSET) and per-request by
    // LIST_TIMEOUT; a failed walk parks the peer for LIST_BACKOFF so an unreachable peer
    // costs one request per cadence. Dispatch is capped by MAX_REPLICATION_INFLIGHT /
    // MAX_ACQUISITION_INFLIGHT; nothing here retries an uncancellable conductor call.
    list_backoff: Mutex<HashMap<NodeId, Instant>>,
    status_tx: watch::Sender<IrohPullStatus>,
    /// Late-bound: the trigger worker is built after this core. Read into every
    /// dispatched ingest context ([`Self::ingest_ctx`]) so a stored row is
    /// offered to the head-adoption trigger, as the libp2p ingest does.
    head_adoption: OnceLock<Arc<TriggerGate>>,
}

impl IrohPullCore {
    /// `custody_standing` — station 3b (M9) receiver-side pre-authorization.
    /// Pure-iroh mode has no `P2PNode`/`ShardService` to share the resolver
    /// from, so the composition root threads the SAME process-lifetime
    /// instance it built for the shared `ShardService` straight in here —
    /// never a second one. `None` when reach-gating is not wired at boot;
    /// `store_acquired_record` fails a `private` record closed in that case.
    pub fn new(
        db_pool: Option<DbPool>,
        blob_store: Arc<BlobStore>,
        self_cid: String,
        custody_standing: Option<Arc<dyn crate::services::custody_standing::CustodyStanding>>,
    ) -> Arc<Self> {
        let replication_state = ReplicationState::new();
        let acquisition = AcquisitionState::new();
        let (status_tx, _) = watch::channel(IrohPullStatus {
            replication: ReplicationStatus {
                pending: 0,
                completed: 0,
                failed: 0,
                caught_up: false,
            },
            pull: None,
            iroh_peers_known: 0,
            gap_queue: 0,
            acquisition_queue: 0,
        });
        Arc::new(Self {
            ctx: AcquisitionIngestCtx {
                db_pool,
                replication_state,
                acquisition,
                blob_store,
                self_cid,
                write_gate: Arc::new(Mutex::new(())),
                custody_standing,
                // Filled per dispatch from the late-bound slot — see `ingest_ctx`.
                head_adoption: None,
            },
            gap_queue: Mutex::new(VecDeque::new()),
            acquisition_queue: Mutex::new(VecDeque::new()),
            gap_in_flight: Arc::new(AtomicUsize::new(0)),
            pin_in_flight: Arc::new(AtomicUsize::new(0)),
            rotation: AtomicUsize::new(0),
            list_backoff: Mutex::new(HashMap::new()),
            status_tx,
            head_adoption: OnceLock::new(),
        })
    }

    /// Wire the head-adoption trigger gate a stored row is offered to. Set
    /// once; `false` if already set.
    pub fn set_head_adoption_trigger(&self, gate: Arc<TriggerGate>) -> bool {
        self.head_adoption.set(gate).is_ok()
    }

    /// The ingest context for ONE dispatch, carrying whatever trigger gate has
    /// been wired by now. Cheap (Arc/handle clones).
    fn ingest_ctx(&self) -> AcquisitionIngestCtx {
        let mut ctx = self.ctx.clone();
        ctx.head_adoption = self.head_adoption.get().cloned();
        ctx
    }

    /// The fetch-on-announce leg over this core's trackers: the same
    /// `ReplicationState` (so an id is asked for once across the announce and
    /// the inventory walk) and the same gap in-flight budget `drain_gaps` spends.
    pub fn announce_row_fetch(self: &Arc<Self>) -> IrohAnnounceRowFetch {
        let core = Arc::clone(self);
        IrohAnnounceRowFetch::new(
            move || core.ingest_ctx(),
            self.gap_in_flight.clone(),
            MAX_REPLICATION_INFLIGHT,
        )
    }

    pub fn status(&self) -> IrohPullStatus {
        self.status_tx.borrow().clone()
    }

    pub fn acquisition(&self) -> &AcquisitionState {
        &self.ctx.acquisition
    }

    /// Drive the loop: discovery every `list_every`, dispatch every 5 s, pin
    /// reconcile every `reconcile_every`, status refresh after each. Returns
    /// when the task is dropped (process exit).
    pub fn spawn(self: Arc<Self>, list_every: Duration, reconcile_every: Duration) {
        // Pure-iroh mode: this core owns the replication trackers, so it is the
        // one that answers an announced doc with no local row.
        register_announce_row_fetch(self.announce_row_fetch());
        tokio::spawn(async move {
            self.hydrate_local_ids().await;
            let mut list_iv = tokio::time::interval(list_every);
            let mut dispatch_iv = tokio::time::interval(Duration::from_secs(5));
            let mut reconcile_iv = tokio::time::interval(reconcile_every);
            let mut status_iv = tokio::time::interval(Duration::from_secs(15));
            info!(
                target: "elohim_storage::iroh_pull",
                list_every_s = list_every.as_secs(),
                reconcile_every_s = reconcile_every.as_secs(),
                "iroh pull core: running (pure-iroh pull leg)"
            );
            loop {
                tokio::select! {
                    _ = list_iv.tick() => { self.discover_from_peers().await; }
                    _ = dispatch_iv.tick() => { self.drain_gaps().await; self.drain_pins().await; }
                    _ = reconcile_iv.tick() => { self.reconcile_pins().await; }
                    _ = status_iv.tick() => { self.refresh_status().await; }
                }
            }
        });
    }

    fn book_peers(&self) -> Vec<(String, NodeAddr)> {
        let Some(leg) = iroh_fetch_leg() else {
            return Vec::new();
        };
        let me = leg.endpoint().node_id();
        leg.book()
            .snapshot(Some(&me))
            .into_iter()
            .map(|e| {
                let label = e
                    .libp2p_peer_id
                    .clone()
                    .or(e.agent_cid.clone())
                    .unwrap_or_else(|| e.addr.node_id.to_string());
                (label, e.addr)
            })
            .collect()
    }

    /// Mirror of `hydrate_replication_state`: rows already here are not gaps.
    async fn hydrate_local_ids(&self) {
        const PAGE: i64 = 5_000;
        const CEILING: usize = 10_000_000;
        let Some(pool) = self.ctx.db_pool.as_ref() else {
            return;
        };
        let app_ctx = crate::db::AppContext::default_lamad();
        let mut ids: HashSet<String> = HashSet::new();
        let mut offset: i64 = 0;
        loop {
            let Ok(mut conn) = pool.get() else { break };
            let query = crate::db::content_diesel::ContentQuery {
                limit: PAGE,
                offset,
                ..Default::default()
            };
            let page = match crate::db::content_diesel::list_content(
                &mut conn,
                &app_ctx,
                &query,
                crate::db::content_diesel::MinTrust::Invisible,
            ) {
                Ok(p) => p,
                Err(e) => {
                    warn!(target: "elohim_storage::iroh_pull", error = %e, offset, "hydrate: list_content failed");
                    break;
                }
            };
            let n = page.len();
            for item in &page {
                ids.insert(item.content.id.clone());
            }
            if ids.len() >= CEILING || (n as i64) < PAGE {
                break;
            }
            offset += PAGE;
        }
        info!(target: "elohim_storage::iroh_pull", count = ids.len(), "hydrated local content ids");
        self.ctx.replication_state.set_local_ids(ids).await;
    }

    /// One ListContent walk per book peer not backing off; every page's ids go
    /// through pin readmission and `ReplicationState::discover`.
    pub async fn discover_from_peers(&self) {
        let peers = self.book_peers();
        if peers.is_empty() {
            debug!(target: "elohim_storage::iroh_pull", "discover: no iroh peers in the book yet");
            return;
        }
        let Some(leg) = iroh_fetch_leg() else { return };
        let client = IrohShardClient::new(leg.endpoint());
        let now = Instant::now();
        for (label, addr) in peers {
            let node_id = addr.node_id;
            if let Some(until) = self.list_backoff.lock().await.get(&node_id) {
                if *until > now {
                    continue;
                }
            }
            let mut offset: u32 = 0;
            let mut pages = 0usize;
            loop {
                let req = ShardRequest::ListContent {
                    reach_filter: None,
                    offset,
                    limit: LIST_PAGE_LIMIT,
                };
                let t0 = Instant::now();
                let res =
                    tokio::time::timeout(LIST_TIMEOUT, client.request(addr.clone(), &req)).await;
                match res {
                    Ok(Ok(ShardResponse::ContentList {
                        items,
                        total,
                        has_more,
                    })) => {
                        crate::metrics::observe_atom_duration(
                            crate::metrics::ConvergenceAtom::InventoryRequest,
                            t0.elapsed(),
                        );
                        pages += 1;
                        let page_len = items.len();
                        let remote_ids: Vec<String> = items.into_iter().map(|i| i.id).collect();
                        self.readmit_pins_named_by_inventory(&remote_ids);
                        let new_gaps = self.ctx.replication_state.discover(remote_ids).await;
                        if new_gaps.is_empty() {
                            self.ctx.replication_state.update_caught_up().await;
                        } else {
                            info!(
                                target: "elohim_storage::iroh_pull",
                                peer = %label, gaps = new_gaps.len(), total, "queued content gaps from iroh inventory"
                            );
                            self.gap_queue.lock().await.extend(new_gaps);
                        }
                        match crate::p2p::sync_protocol::next_doc_list_offset(
                            offset, page_len, has_more,
                        ) {
                            Some(next) => offset = next,
                            None => break,
                        }
                    }
                    Ok(Ok(other)) => {
                        debug!(target: "elohim_storage::iroh_pull", peer = %label, response = ?std::mem::discriminant(&other), "ListContent: unexpected response");
                        self.list_backoff
                            .lock()
                            .await
                            .insert(node_id, Instant::now() + LIST_BACKOFF);
                        break;
                    }
                    Ok(Err(e)) => {
                        debug!(target: "elohim_storage::iroh_pull", peer = %label, error = %e, "ListContent failed");
                        self.list_backoff
                            .lock()
                            .await
                            .insert(node_id, Instant::now() + LIST_BACKOFF);
                        break;
                    }
                    Err(_) => {
                        debug!(target: "elohim_storage::iroh_pull", peer = %label, "ListContent timed out");
                        self.list_backoff
                            .lock()
                            .await
                            .insert(node_id, Instant::now() + LIST_BACKOFF);
                        break;
                    }
                }
            }
            if pages > 0 {
                self.list_backoff.lock().await.remove(&node_id);
            }
        }
        self.refresh_status().await;
    }

    /// Mirror of `drain_gap_queue`: bounded `GetContent` dispatch over iroh.
    pub async fn drain_gaps(&self) {
        let peers = self.book_peers();
        if peers.is_empty() {
            return;
        }
        let in_flight = self.gap_in_flight.load(Ordering::Relaxed);
        let available = MAX_REPLICATION_INFLIGHT.saturating_sub(in_flight);
        if available == 0 {
            return;
        }
        let to_dispatch: Vec<String> = {
            let mut q = self.gap_queue.lock().await;
            if q.is_empty() {
                return;
            }
            let n = q.len();
            q.drain(..available.min(n)).collect()
        };
        debug!(target: "elohim_storage::iroh_pull", dispatching = to_dispatch.len(), in_flight, "draining replication gaps over iroh");
        let rotation = self.rotation.fetch_add(1, Ordering::Relaxed);
        for (i, id) in to_dispatch.into_iter().enumerate() {
            let (label, addr) = peers[(rotation.wrapping_add(i)) % peers.len()].clone();
            crate::metrics::inc_acquisition_dispatch("iroh");
            let ctx = self.ingest_ctx();
            let counter = self.gap_in_flight.clone();
            counter.fetch_add(1, Ordering::Relaxed);
            tokio::spawn(async move {
                acquire_over_iroh(ctx, id, label, addr, PullKind::Gap).await;
                counter.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }

    /// Mirror of `drain_acquisition_queue` for pin wants.
    pub async fn drain_pins(&self) {
        let peers = self.book_peers();
        if peers.is_empty() {
            return;
        }
        let in_flight = self.pin_in_flight.load(Ordering::Relaxed);
        let available = acquisition::MAX_ACQUISITION_INFLIGHT.saturating_sub(in_flight);
        if available == 0 {
            return;
        }
        let to_dispatch: Vec<String> = {
            let mut q = self.acquisition_queue.lock().await;
            if q.is_empty() {
                return;
            }
            let n = q.len();
            q.drain(..available.min(n)).collect()
        };
        let rotation = self.rotation.fetch_add(1, Ordering::Relaxed);
        for (i, id) in to_dispatch.into_iter().enumerate() {
            if !self.ctx.acquisition.wants(&id).await {
                continue;
            }
            let (label, addr) = peers[(rotation.wrapping_add(i)) % peers.len()].clone();
            crate::metrics::inc_acquisition_dispatch("iroh");
            let ctx = self.ingest_ctx();
            let counter = self.pin_in_flight.clone();
            counter.fetch_add(1, Ordering::Relaxed);
            tokio::spawn(async move {
                acquire_over_iroh(ctx, id, label, addr, PullKind::Pin).await;
                counter.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }

    /// Mirror of `run_acquisition_reconcile`: active item pins → wants → queue.
    pub async fn reconcile_pins(&self) -> crate::metrics::AcquisitionReconcileOutcome {
        use crate::metrics::AcquisitionReconcileOutcome;
        let first_pass = self.ctx.acquisition.rollup_if_initialized().await.is_none();
        let Some(pool) = self.ctx.db_pool.as_ref() else {
            return AcquisitionReconcileOutcome::DbPoolMissing;
        };
        let Ok(mut conn) = pool.get() else {
            return AcquisitionReconcileOutcome::DbPoolUnavailable;
        };
        readmit_cooled_down_pins(&mut conn);
        let pins = match crate::db::acquisition_pins::list_active_pins(&mut conn) {
            Ok(p) => p,
            Err(e) => {
                warn!(target: "elohim_storage::iroh_pull", error = %e, "pin census failed");
                return AcquisitionReconcileOutcome::PinLoadFailed;
            }
        };
        let active_pin_count = pins.len();
        let pin_wants: Vec<(i32, Vec<String>)> = pins
            .iter()
            .filter(|p| p.kind == "item")
            .map(|p| (p.id, vec![p.head_ref.clone()]))
            .collect();
        let want_ids: Vec<String> = pin_wants
            .iter()
            .flat_map(|(_, ids)| ids.iter().cloned())
            .collect();
        let app_ctx = crate::db::AppContext::default_lamad();
        let local_has = match crate::db::content_diesel::content_ids_present(
            &mut conn, &app_ctx, &want_ids,
        ) {
            Ok(set) => set,
            Err(e) => {
                warn!(target: "elohim_storage::iroh_pull", error = %e, "local-presence query failed");
                return AcquisitionReconcileOutcome::PresenceQueryFailed;
            }
        };
        drop(conn);
        let peer_count = self.book_peers().len();
        let to_dispatch = self
            .ctx
            .acquisition
            .reconcile(
                pin_wants,
                &local_has,
                acquisition::retry_budget_for_peers(peer_count),
            )
            .await;
        if !to_dispatch.is_empty() {
            let mut q = self.acquisition_queue.lock().await;
            for id in to_dispatch {
                if !q.contains(&id) {
                    q.push_back(id);
                }
            }
        }
        crate::metrics::set_acquisition_reconcile_completed(active_pin_count);
        if first_pass {
            info!(target: "elohim_storage::iroh_pull", active_pins = active_pin_count, desired = want_ids.len(), local = local_has.len(), "acquisition reconcile first pass completed (iroh)");
        }
        self.refresh_status().await;
        AcquisitionReconcileOutcome::Completed
    }

    async fn refresh_status(&self) {
        let replication = self.ctx.replication_state.status().await;
        let pull = self.ctx.acquisition.rollup_if_initialized().await;
        let status = IrohPullStatus {
            replication,
            pull,
            iroh_peers_known: self.book_peers().len(),
            gap_queue: self.gap_queue.lock().await.len(),
            acquisition_queue: self.acquisition_queue.lock().await.len(),
        };
        self.status_tx.send_replace(status);
    }

    fn readmit_pins_named_by_inventory(&self, remote_ids: &[String]) {
        if remote_ids.is_empty() {
            return;
        }
        let Some(pool) = self.ctx.db_pool.as_ref() else {
            return;
        };
        let Ok(mut conn) = pool.get() else { return };
        let retired = match crate::db::acquisition_pins::list_pins_by_status(
            &mut conn,
            acquisition::PIN_STATUS_RETIRED,
        ) {
            Ok(p) if !p.is_empty() => p,
            _ => return,
        };
        let advertised: HashSet<&str> = remote_ids.iter().map(String::as_str).collect();
        let mut readmitted = 0u64;
        for pin in &retired {
            if !advertised.contains(pin.head_ref.as_str()) {
                continue;
            }
            if let Ok(n) = crate::db::acquisition_pins::set_pin_status(&mut conn, pin.id, "active")
            {
                if n > 0 {
                    readmitted += 1;
                }
            }
        }
        if readmitted > 0 {
            info!(target: "elohim_storage::iroh_pull", readmitted, "readmitted retired pins named by an iroh inventory");
        }
    }
}

/// Fetch-the-row-on-announce over the iroh shard ALPN — the iroh twin of
/// `P2PNode::fetch_announced_row`. Decision and rationale:
/// [`crate::p2p::announce_fetch`].
///
/// One shape for both modes that run an iroh sync plane: in pure-iroh mode it
/// is built over [`IrohPullCore`]'s trackers, in `dual` mode over the libp2p
/// node's (`P2PNode::iroh_announce_row_fetch`) — whichever owns the
/// `ReplicationState`, so the announce and the inventory walk never both ask
/// for one id. The record lands through the shared `acquire_over_iroh` →
/// `store_acquired_record`, with every receive-side check that path carries.
///
/// bounded-work: at most `cap` pulls in flight (a reservation taken BEFORE the
/// tracker is touched, so a refused fetch leaves nothing pending); one
/// `GetContent` per id, to the announcing peer, under `acquire_over_iroh`'s own
/// timeouts; `ReplicationState::discover` refuses an id already local, pending,
/// completed or retry-exhausted. No queue and no retry of its own — a dropped
/// fetch is the next inventory walk's.
pub struct IrohAnnounceRowFetch {
    ctx: Box<dyn Fn() -> AcquisitionIngestCtx + Send + Sync>,
    in_flight: Arc<AtomicUsize>,
    cap: usize,
}

impl IrohAnnounceRowFetch {
    pub(crate) fn new(
        ctx: impl Fn() -> AcquisitionIngestCtx + Send + Sync + 'static,
        in_flight: Arc<AtomicUsize>,
        cap: usize,
    ) -> Self {
        Self {
            ctx: Box::new(ctx),
            in_flight,
            cap,
        }
    }

    /// Reserve one in-flight slot, or refuse. Reserve-then-verify, the same
    /// idiom `TriggerGate::plan_retry` uses.
    fn reserve(&self) -> bool {
        if self.in_flight.fetch_add(1, Ordering::AcqRel) >= self.cap {
            self.in_flight.fetch_sub(1, Ordering::AcqRel);
            return false;
        }
        true
    }

    /// Ask `peer` — the node whose doc apply named `content_id` — for the
    /// record. The caller has already established that no local row exists.
    /// Never blocks and never fails the sync apply: the pull is a detached task.
    pub(crate) fn fetch(&self, content_id: &str, peer: NodeId) {
        if !self.reserve() {
            return AnnounceFetch::Saturated.record();
        }
        let ctx = (self.ctx)();
        let in_flight = self.in_flight.clone();
        let content_id = content_id.to_string();
        tokio::spawn(async move {
            if ctx
                .replication_state
                .discover(vec![content_id.clone()])
                .await
                .is_empty()
            {
                AnnounceFetch::AlreadyTracked.record();
            } else {
                // The book is the verified dial target; a bare NodeId is only
                // reachable when discovery can resolve it — never wrong, just
                // possibly unreachable (the same fallback the announce pull takes).
                let entry = iroh_fetch_leg().and_then(|leg| leg.book().get(&peer));
                let label = entry
                    .as_ref()
                    .and_then(|e| e.libp2p_peer_id.clone().or(e.agent_cid.clone()))
                    .unwrap_or_else(|| peer.to_string());
                let addr = entry.map(|e| e.addr).unwrap_or_else(|| NodeAddr::new(peer));
                AnnounceFetch::Enqueued.record();
                crate::metrics::inc_acquisition_dispatch("iroh");
                info!(
                    target: "elohim_storage::iroh_pull",
                    content_id = %content_id, peer = %label,
                    "announced content doc has no local row — fetching the record from the announcer"
                );
                acquire_over_iroh(ctx, content_id, label, addr, PullKind::Gap).await;
            }
            in_flight.fetch_sub(1, Ordering::AcqRel);
        });
    }
}

static ANNOUNCE_ROW_FETCH: OnceLock<IrohAnnounceRowFetch> = OnceLock::new();

/// Publish the process's fetch-on-announce leg. First registration wins (one
/// owner of the replication trackers per process: the libp2p node in `dual`
/// mode, [`IrohPullCore`] in pure-iroh mode); returns `false` otherwise.
pub fn register_announce_row_fetch(fetch: IrohAnnounceRowFetch) -> bool {
    ANNOUNCE_ROW_FETCH.set(fetch).is_ok()
}

/// The registered fetch-on-announce leg, or `None` when no tracker owner has
/// started (the iroh sync plane then leaves an absent row to the inventory walk).
pub(crate) fn announce_row_fetch() -> Option<&'static IrohAnnounceRowFetch> {
    ANNOUNCE_ROW_FETCH.get()
}

/// Mirror of `P2PNode::readmit_cooled_down_pins`.
fn readmit_cooled_down_pins(conn: &mut diesel::SqliteConnection) {
    let retired = match crate::db::acquisition_pins::list_pins_by_status(
        conn,
        acquisition::PIN_STATUS_RETIRED,
    ) {
        Ok(p) => p,
        Err(_) => return,
    };
    if retired.is_empty() {
        return;
    }
    let now = chrono::Utc::now();
    for pin in &retired {
        if !acquisition::pin_cooled_down(&pin.updated_at, now) {
            continue;
        }
        let _ = crate::db::acquisition_pins::set_pin_status(conn, pin.id, "active");
    }
}

#[cfg(test)]
mod announce_fetch_tests {
    use super::*;

    async fn fixture(cap: usize) -> (IrohAnnounceRowFetch, ReplicationState, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AcquisitionIngestCtx {
            db_pool: None,
            replication_state: ReplicationState::new(),
            acquisition: AcquisitionState::new(),
            blob_store: Arc::new(BlobStore::new(dir.path()).await.unwrap()),
            self_cid: String::new(),
            write_gate: Arc::new(Mutex::new(())),
            custody_standing: None,
            head_adoption: None,
        };
        let state = ctx.replication_state.clone();
        (
            IrohAnnounceRowFetch::new(move || ctx.clone(), Arc::new(AtomicUsize::new(0)), cap),
            state,
            dir,
        )
    }

    fn a_peer() -> NodeId {
        iroh::SecretKey::generate(&mut rand::rngs::OsRng).public()
    }

    /// The in-flight reservation is taken BEFORE the tracker is touched: a
    /// saturated fetch leaves the id unknown to `ReplicationState`, so the next
    /// inventory walk still discovers it (a pending id nobody dispatches would
    /// hold `caught_up` false forever).
    #[tokio::test]
    async fn a_saturated_fetch_leaves_the_tracker_untouched() {
        let (fetch, state, _dir) = fixture(0).await;
        fetch.fetch("wanted", a_peer());
        tokio::task::yield_now().await;
        assert_eq!(state.status().await.pending, 0);
        assert_eq!(state.discover(vec!["wanted".into()]).await, vec!["wanted"]);
        assert_eq!(fetch.in_flight.load(Ordering::Acquire), 0);
    }

    #[test]
    fn the_reservation_is_capped_and_returned() {
        let fetch = IrohAnnounceRowFetch::new(
            || unreachable!("reserve never builds a context"),
            Arc::new(AtomicUsize::new(0)),
            2,
        );
        assert!(fetch.reserve());
        assert!(fetch.reserve());
        assert!(!fetch.reserve(), "the third pull is refused at a cap of 2");
        assert_eq!(fetch.in_flight.load(Ordering::Acquire), 2);
    }

    /// An id the tracker already holds (pending from the inventory walk, or
    /// already local) is not asked for again, and the slot is given back.
    #[tokio::test]
    async fn an_id_already_tracked_is_not_fetched_twice() {
        let (fetch, state, _dir) = fixture(4).await;
        assert_eq!(state.discover(vec!["queued".into()]).await.len(), 1);
        fetch.fetch("queued", a_peer());
        for _ in 0..50 {
            if fetch.in_flight.load(Ordering::Acquire) == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(fetch.in_flight.load(Ordering::Acquire), 0);
        assert_eq!(
            state.status().await.pending,
            1,
            "still the one pending entry the inventory walk made"
        );
    }
}
