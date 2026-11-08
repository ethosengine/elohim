//! DHT confirmation of cross-signed bindings learned by gossip or handshake.
//!
//! A cross-signed `AgentPeerBinding` that arrives by gossip proves only that its sender
//! holds both keys; anyone can mint any number of agent↔PeerId pairs and gossip them.
//! It becomes AUTHORITY (`p2p::transport_authority`) only when this node finds, on the
//! DHT, a current `AgentPeerBinding` entry whose action hash is the row's
//! `dht_anchor_hash` and whose peer, agent and validity window are the row's. Then the
//! confirmation is recorded (`db::binding_confirmations`) and the claims held for that
//! author are replayed (`p2p::gossip_dispatch::schedule_replay`).
//!
//! The read is the imagodei `get_bindings_for_peer` (legacy mode: the bindings linked
//! from the peer's anchor, superseded ones filtered out). This node's OWN bindings
//! arrive as DHT signals and are confirmed by the projection itself
//! (`reconcile::controller::on_agent_peer_binding`); every other one comes here.
//!
//! What the legacy view does not carry: the entry's author and signature. A
//! confirmation therefore proves that a current entry with these fields exists at this
//! anchor on the DHT, not that the named agent authored it (filed: author-level
//! confirmation needs a zome query returning the record).
//!
//! ## Never starved, never unbounded
//!
//! - One worker owns a bounded queue ([`CONFIRM_QUEUE_CAP`]). A duplicate request is
//!   coalesced; a request past the cap, past [`PER_SOURCE_CAP`] for its relaying
//!   source or past [`PER_PEER_CAP`] for its PeerId is dropped and counted
//!   ([`Confirmer::dropped`]) — the slow sweep re-requests whatever is still
//!   unconfirmed.
//! - Each ATTEMPT takes a concurrency slot ([`CONFIRM_CONCURRENCY`]) and releases it
//!   when the read returns; waiting for the next attempt
//!   ([`CONFIRM_BACKOFF_SECS`]) holds nothing. Bindings that can never confirm cannot
//!   starve one that can.
//! - Due requests whose PeerId has a verified transport link go first.
//! - The conductor client is resolved on every attempt ([`Reader`]), so a bridge that
//!   connects after boot still confirms.
//! - [`Confirmer::sweep`] re-requests every cross-signed, current, unconfirmed row at
//!   start and every [`SWEEP_INTERVAL_SECS`], so a binding whose attempts ran out (or
//!   whose request was dropped) is retried.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use diesel::prelude::*;
use futures_util::future::BoxFuture;
use tokio::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::hc_client::HcClient;

/// Backoff between DHT reads for one binding, in seconds (about 33 minutes in all).
pub const CONFIRM_BACKOFF_SECS: [u64; 7] = [5, 15, 45, 120, 300, 600, 900];

/// DHT reads in flight at once.
pub const CONFIRM_CONCURRENCY: usize = 8;

/// Requests the confirmer holds (queued or between attempts).
pub const CONFIRM_QUEUE_CAP: usize = 512;

/// Requests one relaying source may have outstanding.
pub const PER_SOURCE_CAP: usize = 16;

/// Requests one PeerId may have outstanding (one per anchor it claims).
pub const PER_PEER_CAP: usize = 4;

/// How often unconfirmed rows are swept back in.
pub const SWEEP_INTERVAL_SECS: u64 = 600;

/// Rows one sweep re-requests at most.
pub const SWEEP_LIMIT: i64 = 256;

/// One binding to confirm.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfirmRequest {
    pub peer_id: String,
    pub dht_anchor_hash: String,
    /// The transport that relayed the binding to us (gossip source, handshake peer),
    /// or `None` for the sweep and this node's own projection.
    pub source: Option<String>,
}

/// Reads the current DHT bindings for a PeerId. `Ok(None)`: no conductor client
/// right now (not an attempt that found nothing).
pub type Reader = Arc<
    dyn Fn(String) -> BoxFuture<'static, Result<Option<Vec<DhtView>>, StorageError>> + Send + Sync,
>;

/// One entry of the legacy `get_bindings_for_peer` view.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhtView {
    action_hash: crate::signals::HoloHashB64,
    peer_id: String,
    agent_cid: String,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    #[serde(default)]
    superseded_by: Option<crate::signals::HoloHashB64>,
}

/// The production reader: the imagodei client resolved from `resolve` on every call.
pub fn conductor_reader(resolve: Arc<dyn Fn() -> Option<Arc<HcClient>> + Send + Sync>) -> Reader {
    Arc::new(move |peer_id: String| {
        let resolve = resolve.clone();
        Box::pin(async move {
            let Some(hc) = resolve() else {
                return Ok(None);
            };
            let payload = rmp_serde::to_vec_named(&peer_id)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            let bytes = hc
                .call_zome_imagodei("imagodei", "get_bindings_for_peer", payload)
                .await?;
            let views: Vec<DhtView> = rmp_serde::from_slice(&bytes)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            Ok(Some(views))
        })
    })
}

struct Entry {
    request: ConfirmRequest,
    attempt: usize,
    due: Instant,
    linked: bool,
}

#[derive(Default)]
struct Queue {
    entries: Vec<Entry>,
    /// Every request held, waiting or in flight (coalescing and the caps).
    held: HashSet<(String, String)>,
}

/// The confirmer: a bounded queue and one worker. Production uses one per process
/// ([`spawn_binding_confirmer`], [`request_confirmation`]); tests make their own.
pub struct Confirmer {
    pool: DbPool,
    reader: Reader,
    queue: Mutex<Queue>,
    wake: tokio::sync::Notify,
    dropped: AtomicU64,
}

impl Confirmer {
    pub fn new(pool: DbPool, reader: Reader) -> Arc<Self> {
        Arc::new(Self {
            pool,
            reader,
            queue: Mutex::new(Queue::default()),
            wake: tokio::sync::Notify::new(),
            dropped: AtomicU64::new(0),
        })
    }

    /// Requests dropped by the caps so far.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Requests currently held.
    pub fn held(&self) -> usize {
        self.lock().held.len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Queue one binding for confirmation (coalesced; dropped and counted past a cap).
    pub fn request(&self, request: ConfirmRequest) {
        let linked = self.pool.get().ok().is_some_and(|mut conn| {
            crate::db::transport_links::iroh_for_libp2p(
                &mut conn,
                std::slice::from_ref(&request.peer_id),
            )
            .is_ok_and(|ids| !ids.is_empty())
        });
        let key = (request.peer_id.clone(), request.dht_anchor_hash.clone());
        {
            let mut queue = self.lock();
            if queue.held.contains(&key) {
                return;
            }
            let per_source = request.source.as_ref().map_or(0, |source| {
                queue
                    .entries
                    .iter()
                    .filter(|e| e.request.source.as_ref() == Some(source))
                    .count()
            });
            let per_peer = queue
                .held
                .iter()
                .filter(|(peer, _)| *peer == request.peer_id)
                .count();
            if queue.held.len() >= CONFIRM_QUEUE_CAP
                || per_source >= PER_SOURCE_CAP
                || per_peer >= PER_PEER_CAP
            {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                debug!(peer_id = %request.peer_id, "binding confirmation request dropped (cap) — the sweep re-requests it");
                return;
            }
            queue.held.insert(key);
            queue.entries.push(Entry {
                request,
                attempt: 0,
                due: Instant::now(),
                linked,
            });
        }
        self.wake.notify_one();
    }

    /// Re-request every cross-signed, current, not-superseded row that has no
    /// confirmation (bounded by [`SWEEP_LIMIT`]). Returns how many were requested.
    pub fn sweep(&self) -> usize {
        use crate::db::diesel_schema::peer_identity_bindings::dsl;
        let Ok(mut conn) = self.pool.get() else {
            return 0;
        };
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let rows: Vec<crate::db::models::PeerIdentityBindingRow> = dsl::peer_identity_bindings
            .filter(dsl::superseded_by.is_null())
            .filter(dsl::valid_until.gt(&now))
            .filter(dsl::proof_status.eq(crate::p2p::binding_proof_wire::PROOF_STATUS_CROSS_SIGNED))
            .limit(SWEEP_LIMIT)
            .load(&mut conn)
            .unwrap_or_default();
        let unconfirmed: Vec<_> = rows
            .into_iter()
            .filter(|row| row.is_cross_signed())
            .filter(|row| !crate::db::binding_confirmations::confirms(&mut conn, row))
            .collect();
        drop(conn);
        let count = unconfirmed.len();
        for row in unconfirmed {
            self.request(ConfirmRequest {
                peer_id: row.peer_id,
                dht_anchor_hash: row.dht_anchor_hash,
                source: None,
            });
        }
        count
    }

    /// Take up to [`CONFIRM_CONCURRENCY`] due entries, linked PeerIds first; or the
    /// instant the next one falls due.
    fn take_due(&self) -> (Vec<Entry>, Option<Instant>) {
        let now = Instant::now();
        let mut queue = self.lock();
        queue
            .entries
            .sort_by(|a, b| b.linked.cmp(&a.linked).then(a.due.cmp(&b.due)));
        let mut due = Vec::new();
        let mut rest = Vec::new();
        for entry in std::mem::take(&mut queue.entries) {
            if entry.due <= now && due.len() < CONFIRM_CONCURRENCY {
                due.push(entry);
            } else {
                rest.push(entry);
            }
        }
        let next = rest.iter().map(|e| e.due).min();
        queue.entries = rest;
        (due, next)
    }

    /// One attempt for one entry, then reschedule it or let it go.
    async fn attempt(self: &Arc<Self>, mut entry: Entry) {
        let settled = match confirm_once(&self.pool, &self.reader, &entry.request).await {
            Ok(settled) => settled,
            Err(e) => {
                debug!(peer_id = %entry.request.peer_id, error = %e, "binding confirmation read failed");
                false
            }
        };
        let key = (
            entry.request.peer_id.clone(),
            entry.request.dht_anchor_hash.clone(),
        );
        let mut queue = self.lock();
        if settled || entry.attempt + 1 >= CONFIRM_BACKOFF_SECS.len() {
            // Settled, or out of attempts: the sweep re-requests it if still unconfirmed.
            queue.held.remove(&key);
            return;
        }
        entry.due = Instant::now() + Duration::from_secs(CONFIRM_BACKOFF_SECS[entry.attempt]);
        entry.attempt += 1;
        queue.entries.push(entry);
    }

    /// The worker: run due attempts concurrently (each holds its slot only while its
    /// read runs), then wait — holding nothing — until the next is due or a request
    /// arrives.
    pub async fn run(self: Arc<Self>) {
        loop {
            let (due, next) = self.take_due();
            if !due.is_empty() {
                futures_util::future::join_all(due.into_iter().map(|entry| self.attempt(entry)))
                    .await;
                continue;
            }
            match next {
                Some(at) => {
                    tokio::select! {
                        _ = tokio::time::sleep_until(at) => {}
                        _ = self.wake.notified() => {}
                    }
                }
                None => self.wake.notified().await,
            }
        }
    }
}

static CONFIRMER: OnceLock<Arc<Confirmer>> = OnceLock::new();

/// Ask this process's confirmer to look for this binding on the DHT. Before the
/// confirmer starts the request is lost, but the startup sweep picks the row up.
pub fn request_confirmation(peer_id: &str, dht_anchor_hash: &str, source: Option<&str>) {
    if let Some(confirmer) = CONFIRMER.get() {
        confirmer.request(ConfirmRequest {
            peer_id: peer_id.to_string(),
            dht_anchor_hash: dht_anchor_hash.to_string(),
            source: source.map(str::to_string),
        });
    }
}

/// Sweep unconfirmed bindings into this process's confirmer now (the conductor app
/// just became available). A no-op before the confirmer starts.
pub fn sweep_now() {
    if let Some(confirmer) = CONFIRMER.get() {
        let swept = confirmer.sweep();
        if swept > 0 {
            info!(
                swept,
                "binding confirmer: unconfirmed bindings re-requested (conductor app available)"
            );
        }
    }
}

/// Start this process's confirmer: the worker, plus the sweep at start and every
/// [`SWEEP_INTERVAL_SECS`]. The client is resolved per attempt, so the confirmer
/// runs even when the conductor bridge connects later.
pub fn spawn_binding_confirmer(pool: DbPool, reader: Reader) {
    let confirmer = Confirmer::new(pool, reader);
    if CONFIRMER.set(confirmer.clone()).is_err() {
        warn!("binding confirmer already running — second start ignored");
        return;
    }
    tokio::spawn(confirmer.clone().run());
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(SWEEP_INTERVAL_SECS));
        loop {
            tick.tick().await;
            let swept = confirmer.sweep();
            if swept > 0 {
                info!(
                    swept,
                    dropped = confirmer.dropped(),
                    "binding confirmer: unconfirmed bindings re-requested"
                );
            }
        }
    });
}

fn iso_of_micros(micros: i64) -> Option<String> {
    chrono::DateTime::<chrono::Utc>::from_timestamp_micros(micros)
        .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// Does `view` carry exactly the row's anchor, peer, agent and window, and is it
/// current? Pure, so the matching rule is testable without a conductor.
fn view_matches(view: &DhtView, row: &crate::db::models::PeerIdentityBindingRow) -> bool {
    view.superseded_by.is_none()
        && view.action_hash.as_str() == row.dht_anchor_hash
        && view.peer_id == row.peer_id
        && view.agent_cid == row.agent_cid
        && iso_of_micros(view.valid_from_micros).as_deref() == Some(row.valid_from.as_str())
        && view.valid_until_micros.and_then(iso_of_micros) == row.valid_until
}

/// One DHT read. `Ok(true)` when the request is settled (confirmed, or nothing left to
/// confirm); `Ok(false)` when it is not found yet or no client is connected.
async fn confirm_once(
    pool: &DbPool,
    reader: &Reader,
    request: &ConfirmRequest,
) -> Result<bool, StorageError> {
    use crate::db::diesel_schema::peer_identity_bindings::dsl;
    let row = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        dsl::peer_identity_bindings
            .filter(dsl::peer_id.eq(&request.peer_id))
            .filter(dsl::dht_anchor_hash.eq(&request.dht_anchor_hash))
            .first::<crate::db::models::PeerIdentityBindingRow>(&mut conn)
            .optional()
            .map_err(|e| StorageError::Database(e.to_string()))?
    };
    let Some(row) = row.filter(|row| row.is_cross_signed()) else {
        return Ok(true);
    };
    let Some(views) = reader(request.peer_id.clone()).await? else {
        return Ok(false);
    };
    if !views.iter().any(|view| view_matches(view, &row)) {
        return Ok(false);
    }
    {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        crate::db::binding_confirmations::confirm(
            &mut conn,
            &crate::db::binding_confirmations::DhtBinding {
                peer_id: &row.peer_id,
                dht_anchor_hash: &row.dht_anchor_hash,
                agent_cid: &row.agent_cid,
                valid_from: &row.valid_from,
                valid_until: row.valid_until.as_deref(),
            },
        )?;
    }
    info!(
        peer_id = %row.peer_id, agent_cid = %row.agent_cid,
        "cross-signed binding confirmed on the DHT — now authority"
    );
    crate::p2p::gossip_dispatch::schedule_replay(
        pool,
        crate::p2p::gossip_dispatch::authors_bound_by_peer(pool, &row.peer_id),
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn row_of(pool: &DbPool, peer: &str) -> crate::db::models::PeerIdentityBindingRow {
        use crate::db::diesel_schema::peer_identity_bindings::dsl;
        let mut conn = pool.get().unwrap();
        dsl::peer_identity_bindings
            .filter(dsl::peer_id.eq(peer))
            .first(&mut conn)
            .unwrap()
    }

    fn view_of(row: &crate::db::models::PeerIdentityBindingRow) -> DhtView {
        let micros = |iso: &str| {
            chrono::DateTime::parse_from_rfc3339(iso)
                .unwrap()
                .timestamp_micros()
        };
        let bytes = rmp_serde::to_vec_named(&serde_json::json!({
            "actionHash": row.dht_anchor_hash,
            "peerId": row.peer_id,
            "agentCid": row.agent_cid,
            "validFromMicros": micros(&row.valid_from),
            "validUntilMicros": row.valid_until.as_deref().map(micros),
            "deviceArchetype": "node",
            "supersededBy": null,
        }))
        .unwrap();
        rmp_serde::from_slice(&bytes).unwrap()
    }

    fn unconfirmed(
        pool: &DbPool,
        name: &str,
        secret: [u8; 32],
    ) -> crate::db::models::PeerIdentityBindingRow {
        let mut conn = pool.get().unwrap();
        let peer = crate::test_util::bind_cross_signed_unconfirmed(&mut conn, name, secret);
        drop(conn);
        row_of(pool, &peer)
    }

    fn confirmed(pool: &DbPool, row: &crate::db::models::PeerIdentityBindingRow) -> bool {
        let mut conn = pool.get().unwrap();
        crate::db::binding_confirmations::confirms(&mut conn, row)
    }

    /// A reader that finds exactly the views in `found` (by PeerId), counting reads.
    fn reader_finding(found: Vec<DhtView>, reads: Arc<AtomicUsize>) -> Reader {
        let found = Arc::new(found);
        Arc::new(move |peer: String| {
            let found = found.clone();
            let reads = reads.clone();
            Box::pin(async move {
                reads.fetch_add(1, Ordering::SeqCst);
                Ok(Some(
                    found
                        .iter()
                        .filter(|v| v.peer_id == peer)
                        .cloned()
                        .collect(),
                ))
            })
        })
    }

    fn request_of(row: &crate::db::models::PeerIdentityBindingRow, source: &str) -> ConfirmRequest {
        ConfirmRequest {
            peer_id: row.peer_id.clone(),
            dht_anchor_hash: row.dht_anchor_hash.clone(),
            source: Some(source.to_string()),
        }
    }

    #[test]
    fn only_an_exact_current_dht_entry_confirms() {
        let pool = crate::test_util::test_pool();
        let row = unconfirmed(&pool, "uhCAkDht", [91u8; 32]);
        assert!(view_matches(&view_of(&row), &row));
        let mut other = row.clone();
        other.agent_cid = crate::test_util::test_agent("uhCAkSomeoneElse");
        assert!(!view_matches(&view_of(&other), &row), "another agent");
        let mut anchor = row.clone();
        anchor.dht_anchor_hash = "uhCkkAnotherAnchor".into();
        assert!(!view_matches(&view_of(&anchor), &row), "another anchor");
        let mut window = row.clone();
        window.valid_until = Some("2099-01-01T00:00:00Z".into());
        assert!(!view_matches(&view_of(&window), &row), "another window");
    }

    /// Bindings that can never confirm — more than the concurrency, from many
    /// sources — do not hold the confirmer: an honest binding requested after them is
    /// confirmed on its first attempt, while they wait out their backoff holding
    /// nothing. A flood from one source is capped and counted.
    #[tokio::test(start_paused = true)]
    async fn the_confirmer_is_not_starved_by_unconfirmable_bindings() {
        let pool = crate::test_util::test_pool();
        let honest = unconfirmed(&pool, "uhCAkHonest", [140u8; 32]);
        let reads = Arc::new(AtomicUsize::new(0));
        let confirmer = Confirmer::new(
            pool.clone(),
            reader_finding(vec![view_of(&honest)], reads.clone()),
        );
        for i in 0..(CONFIRM_CONCURRENCY as u8 * 3) {
            let junk = unconfirmed(&pool, &format!("uhCAkFree{i}"), [150 + i; 32]);
            confirmer.request(request_of(&junk, &format!("12D3KooWSource{i}")));
        }
        tokio::spawn(confirmer.clone().run());
        // Let the first batch run and fall into backoff.
        tokio::time::sleep(Duration::from_millis(50)).await;
        confirmer.request(request_of(&honest, "12D3KooWHonestRelay"));
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(
            confirmed(&pool, &honest),
            "confirmed within seconds, not after ~33 min"
        );

        // One relaying source cannot hold more than its cap.
        let before = confirmer.dropped();
        for i in 0..(PER_SOURCE_CAP as u8 + 5) {
            let flood = unconfirmed(
                &pool,
                &format!("uhCAkFlood{i}"),
                [200u8.wrapping_add(i); 32],
            );
            confirmer.request(request_of(&flood, "12D3KooWFlooder"));
        }
        assert!(
            confirmer.dropped() >= before + 5,
            "the flood past the source cap is dropped and counted"
        );
        assert!(confirmer.held() <= CONFIRM_QUEUE_CAP);
    }

    /// No conductor client at first: attempts find no client and back off; once a
    /// client connects (resolved per attempt) the binding confirms.
    #[tokio::test(start_paused = true)]
    async fn a_late_connecting_client_still_confirms() {
        let pool = crate::test_util::test_pool();
        let row = unconfirmed(&pool, "uhCAkLate", [141u8; 32]);
        let connected = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let view = view_of(&row);
        let reader: Reader = {
            let connected = connected.clone();
            Arc::new(move |_peer: String| {
                let connected = connected.load(Ordering::SeqCst);
                let view = view.clone();
                Box::pin(async move { Ok(connected.then(|| vec![view])) })
            })
        };
        let confirmer = Confirmer::new(pool.clone(), reader);
        confirmer.request(request_of(&row, "12D3KooWRelay"));
        tokio::spawn(confirmer.clone().run());
        tokio::time::sleep(Duration::from_secs(30)).await;
        assert!(!confirmed(&pool, &row), "no client yet");
        connected.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(200)).await;
        assert!(
            confirmed(&pool, &row),
            "confirmed after the client connected"
        );
    }

    /// A binding whose attempts ran out (the DHT never showed it in time) is picked
    /// up again by the sweep and confirms once the entry is found.
    #[tokio::test(start_paused = true)]
    async fn unconfirmed_rows_are_swept_and_retried() {
        let pool = crate::test_util::test_pool();
        let row = unconfirmed(&pool, "uhCAkSwept", [142u8; 32]);
        let published = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let view = view_of(&row);
        let reader: Reader = {
            let published = published.clone();
            Arc::new(move |_peer: String| {
                let found = if published.load(Ordering::SeqCst) {
                    vec![view.clone()]
                } else {
                    Vec::new()
                };
                Box::pin(async move { Ok(Some(found)) })
            })
        };
        let confirmer = Confirmer::new(pool.clone(), reader);
        tokio::spawn(confirmer.clone().run());
        assert_eq!(
            confirmer.sweep(),
            1,
            "the unconfirmed cross-signed row is swept in"
        );
        let all: u64 = CONFIRM_BACKOFF_SECS.iter().sum();
        tokio::time::sleep(Duration::from_secs(all + 60)).await;
        assert!(!confirmed(&pool, &row));
        assert_eq!(
            confirmer.held(),
            0,
            "attempts ran out and the request let go"
        );

        published.store(true, Ordering::SeqCst);
        assert_eq!(confirmer.sweep(), 1, "swept again");
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(confirmed(&pool, &row));
        assert_eq!(confirmer.sweep(), 0, "nothing left to sweep");
    }
}
