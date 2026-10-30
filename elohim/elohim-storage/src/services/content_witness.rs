//! The content chrome's witness reading — whether the network has witnessed a
//! record — derived per read, never written.
//!
//! **Witnessed** means a conductor OTHER than the record's author holds the
//! record's op as valid. Authoring stamps a record's action hash (the
//! projection's `dht_anchor_hash`) the moment it commits, before any validator
//! has seen it, so the anchor alone reads amber.
//!
//! How this peer finds out, from the record's anchor:
//!
//! 1. One zome call, `content_store::get_record_witness_facts`, returns the
//!    author, the validation receipts other conductors sent back, and whether
//!    this conductor holds the record as valid.
//! 2. **A valid receipt on the record's op** → witnessed. Receipts are sent only
//!    to the authoring conductor, by a remote validator.
//! 3. **Not authored here** (the author is none of this conductor's agents) and
//!    held as valid → witnessed: this conductor holds the record only because
//!    DHT authorities served it, and it validated the record itself.
//! 4. **Authored here, no receipt** → ONE bounded ask to ONE other peer's
//!    storage over the view-federation plane (`ViewKind::ContentHeadRecord`,
//!    the plane the head-adoption arm already rides): witnessed iff that peer
//!    serves this exact action with its record bytes. Needed because a
//!    validator sends a receipt only for an op that first reached it by
//!    publish, and never later — measured 2026-10-16, 31% of matthew's
//!    created records had no receipt from either other household conductor
//!    while both held them.
//!
//! Anything unanswered — no conductor, a timeout, an error, an unreachable
//! peer — is amber. Nothing here reads green from silence.
//!
//! **Caching.** Green is monotonic for one action, so a green result is kept in
//! memory (keyed by app, record and anchor; an update moves the anchor and is
//! read afresh). Amber is not cached: the next read asks again.
//!
//! **Cost and the chip budget.** A read of a not-yet-green record starts at most
//! one check (concurrent reads of the same record join the check in flight):
//! one zome call, plus the one peer ask in case 4. The facing waits for it no
//! longer than [`INLINE_BUDGET`] — the doorway gives the whole facing 300 ms and
//! falls back to its own descriptor past that — and a check still running then
//! finishes in the background, so its green is ready for the next read.
//!
//! **The work is bounded before the call is made.** The extern reads one
//! action's receipt rows and one local `get_details` — no network await, no
//! link walk, no chain scan — so the caller's timeout bounds only how long this
//! read waits, never an unbounded conductor body left running.
//!
//! **Source of truth:** none. Ephemeral (Category C): the facts are the
//! conductor's and the peer's; this module stores no reading.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use elohim_facings::folds::content_chrome::WitnessEvidence;
use serde::Deserialize;
use tokio::sync::watch;

use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::hc_client_registry::HcClientRegistry;
use crate::p2p::reconcile_peers::ReconcilePeers;
use crate::services::head_adoption::{CarriedHeadRecord, HeadRecordFetcher};

/// How long a facing read waits for the witness check. Kept well inside the
/// doorway's 300 ms facing budget (`CHROME_FACING_TIMEOUT`), so a slow
/// conductor never pushes the doorway onto its descriptor fallback.
pub const INLINE_BUDGET: Duration = Duration::from_millis(120);

/// Bound on the zome call inside a check (it may outlive [`INLINE_BUDGET`] in
/// the background, never this).
pub const FACTS_TIMEOUT: Duration = Duration::from_secs(2);

/// Bound on the one peer ask inside a check.
pub const PEER_ASK_TIMEOUT: Duration = Duration::from_secs(3);

/// Bound on refreshing the set of this conductor's agents.
pub const LOCAL_AGENTS_TIMEOUT: Duration = Duration::from_secs(1);

/// How long the set of this conductor's agents is reused before it is read
/// again (hosted agents arrive while the node runs).
pub const LOCAL_AGENTS_TTL: Duration = Duration::from_secs(60);

/// Ceiling on cached green readings; past it the cache is cleared (each entry
/// is re-derivable at the cost of one check).
pub const GREEN_CACHE_CAP: usize = 50_000;

/// The zome the witness facts come from.
pub const ZOME: &str = "content_store";
/// The extern (coordinator-only, read-only; classified READ in
/// `chain_write_gate`).
pub const FACTS_FN: &str = "get_record_witness_facts";

/// `content_store::get_record_witness_facts`'s answer. The author arrives as
/// its base64 string (`AgentPubKeyB64` is serde-transparent), never raw hash
/// bytes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WitnessFacts {
    pub author: String,
    pub authored_by_caller: bool,
    pub held_valid: bool,
    pub record_op_receipts: u32,
    pub record_op_valid_receipts: u32,
    #[serde(default)]
    pub valid_receipts_all_ops: u32,
}

/// What the local facts decide on their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalVerdict {
    /// A non-author is known to hold the record as valid.
    Witnessed,
    /// Authored on this conductor and no receipt yet: ask one other peer.
    AskPeer,
    /// Nothing proves a non-author holder (yet).
    Awaiting,
}

/// Pure: the verdict from the conductor's facts and whether the author is one
/// of this conductor's agents (`None` = that could not be read).
pub fn local_verdict(facts: Option<&WitnessFacts>, author_is_local: Option<bool>) -> LocalVerdict {
    let Some(facts) = facts else {
        // This conductor holds no record for the anchor: nothing to say.
        return LocalVerdict::Awaiting;
    };
    if facts.record_op_valid_receipts > 0 {
        return LocalVerdict::Witnessed;
    }
    if facts.authored_by_caller || author_is_local == Some(true) {
        return LocalVerdict::AskPeer;
    }
    match author_is_local {
        Some(false) if facts.held_valid => LocalVerdict::Witnessed,
        // Unknown locality: a held copy might be the author's own, and an asked
        // peer might be the author — neither proves a non-author holder.
        _ => LocalVerdict::Awaiting,
    }
}

/// Pure: whether a peer's answer proves it holds the record for `anchor`. Only
/// a served record for exactly this action counts; a different head, a
/// hash-only answer, an absence or no answer does not.
pub fn peer_holds(answer: &seam_contracts::Answer<CarriedHeadRecord>, anchor: &str) -> bool {
    match answer {
        seam_contracts::Answer::Present(carried) => {
            carried.head_action_hash == anchor && carried.record.is_some()
        }
        _ => false,
    }
}

/// The conductor- and peer-facing reads a check makes. One production
/// implementation ([`NodeWitnessProbe`]); tests supply their own.
#[async_trait]
pub trait WitnessProbe: Send + Sync {
    /// `content_store::get_record_witness_facts` for `anchor`.
    async fn facts(&self, anchor: &str) -> Result<Option<WitnessFacts>, StorageError>;
    /// The agents of this conductor's installed apps, base64.
    async fn local_agents(&self) -> Option<HashSet<String>>;
    /// Ask ONE other peer whether it holds `content_id`'s record for `anchor`.
    /// `false` for any answer that does not prove it (including no peer).
    async fn ask_one_peer(&self, content_id: &str, anchor: &str) -> bool;
}

/// One check: the facts, then (only when they say so) the one peer ask.
pub async fn check(probe: &dyn WitnessProbe, content_id: &str, anchor: &str) -> WitnessEvidence {
    let facts = match tokio::time::timeout(FACTS_TIMEOUT, probe.facts(anchor)).await {
        Ok(Ok(facts)) => facts,
        Ok(Err(e)) => {
            tracing::debug!(record = %content_id, error = %e, "witness: facts unread");
            return WitnessEvidence::Unread;
        }
        Err(_) => {
            tracing::debug!(record = %content_id, "witness: facts timed out");
            return WitnessEvidence::Unread;
        }
    };
    let author_is_local = match facts.as_ref() {
        Some(f) if f.record_op_valid_receipts == 0 && !f.authored_by_caller => {
            match tokio::time::timeout(LOCAL_AGENTS_TIMEOUT, probe.local_agents()).await {
                Ok(Some(agents)) => Some(agents.contains(&f.author)),
                _ => None,
            }
        }
        _ => None,
    };
    match local_verdict(facts.as_ref(), author_is_local) {
        LocalVerdict::Witnessed => WitnessEvidence::Witnessed,
        LocalVerdict::Awaiting => WitnessEvidence::Awaiting,
        LocalVerdict::AskPeer => {
            match tokio::time::timeout(PEER_ASK_TIMEOUT, probe.ask_one_peer(content_id, anchor))
                .await
            {
                Ok(true) => WitnessEvidence::Witnessed,
                Ok(false) => WitnessEvidence::Awaiting,
                Err(_) => WitnessEvidence::Unread,
            }
        }
    }
}

/// The witness evidence for one record's anchor, within [`INLINE_BUDGET`].
///
/// Green comes from the cache when held. Otherwise the read joins a check in
/// flight for the same key, or starts one; a check that outlives the budget
/// keeps running and fills the cache for the next read. A blank anchor is not
/// checked (the fold reads it amber regardless).
pub async fn witness_for(
    probe: Arc<dyn WitnessProbe>,
    h_app_id: &str,
    content_id: &str,
    anchor: Option<&str>,
) -> WitnessEvidence {
    witness_for_within(probe, h_app_id, content_id, anchor, INLINE_BUDGET).await
}

/// [`witness_for`] with an explicit budget (tests).
pub async fn witness_for_within(
    probe: Arc<dyn WitnessProbe>,
    h_app_id: &str,
    content_id: &str,
    anchor: Option<&str>,
    budget: Duration,
) -> WitnessEvidence {
    let Some(anchor) = anchor.map(str::trim).filter(|a| !a.is_empty()) else {
        return WitnessEvidence::Unread;
    };
    let key = format!("{h_app_id}|{content_id}|{anchor}");
    if is_green(&key) {
        return WitnessEvidence::Witnessed;
    }
    let mut rx = join_or_start(probe, key, content_id.to_string(), anchor.to_string());
    let waited = tokio::time::timeout(budget, rx.wait_for(Option::is_some)).await;
    match waited {
        Ok(Ok(result)) => result.unwrap_or(WitnessEvidence::Unread),
        // Budget spent, or the check's task ended without a result: amber now.
        _ => WitnessEvidence::Unread,
    }
}

type Slot = watch::Receiver<Option<WitnessEvidence>>;

fn green_cache() -> &'static Mutex<HashSet<String>> {
    static GREEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    GREEN.get_or_init(|| Mutex::new(HashSet::new()))
}

fn in_flight() -> &'static Mutex<HashMap<String, Slot>> {
    static IN_FLIGHT: OnceLock<Mutex<HashMap<String, Slot>>> = OnceLock::new();
    IN_FLIGHT.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_green(key: &str) -> bool {
    green_cache()
        .lock()
        .map(|g| g.contains(key))
        .unwrap_or(false)
}

fn remember_green(key: &str) {
    if let Ok(mut g) = green_cache().lock() {
        if g.len() >= GREEN_CACHE_CAP {
            g.clear();
        }
        g.insert(key.to_string());
    }
}

fn join_or_start(
    probe: Arc<dyn WitnessProbe>,
    key: String,
    content_id: String,
    anchor: String,
) -> Slot {
    let mut map = match in_flight().lock() {
        Ok(m) => m,
        Err(p) => p.into_inner(),
    };
    if let Some(rx) = map.get(&key) {
        // A check whose task ended without answering (it panicked) is not
        // joined: its slot would read nothing forever. Start a fresh one.
        if rx.has_changed().is_ok() {
            return rx.clone();
        }
    }
    let (tx, rx) = watch::channel(None);
    map.insert(key.clone(), rx.clone());
    drop(map);
    tokio::spawn(async move {
        let evidence = check(probe.as_ref(), &content_id, &anchor).await;
        if evidence == WitnessEvidence::Witnessed {
            remember_green(&key);
        }
        if let Ok(mut m) = in_flight().lock() {
            m.remove(&key);
        }
        let _ = tx.send(Some(evidence));
    });
    rx
}

// ---------------------------------------------------------------------------
// The node's probe: its conductor and one peer.
// ---------------------------------------------------------------------------

static NODE_REGISTRY: OnceLock<Arc<HcClientRegistry>> = OnceLock::new();
static NODE_PEERS: OnceLock<Arc<dyn ReconcilePeers>> = OnceLock::new();

/// Publish this node's conductor registry to the witness reading. First wins.
pub fn register_conductor(registry: Arc<HcClientRegistry>) -> bool {
    NODE_REGISTRY.set(registry).is_ok()
}

/// Publish this node's peer source (the view-federation plane, libp2p and/or
/// iroh) to the witness reading. First wins.
pub fn register_peers(peers: Arc<dyn ReconcilePeers>) -> bool {
    NODE_PEERS.set(peers).is_ok()
}

/// The probe over whatever the node registered; a missing conductor or peer
/// source reads as no answer (amber).
pub fn node_probe() -> Arc<dyn WitnessProbe> {
    static PROBE: OnceLock<Arc<dyn WitnessProbe>> = OnceLock::new();
    PROBE
        .get_or_init(|| Arc::new(NodeWitnessProbe::default()))
        .clone()
}

/// The production [`WitnessProbe`].
#[derive(Default)]
pub struct NodeWitnessProbe {
    agents: tokio::sync::Mutex<Option<(Instant, HashSet<String>)>>,
    next_peer: std::sync::atomic::AtomicUsize,
}

impl NodeWitnessProbe {
    fn lamad(&self) -> Option<Arc<HcClient>> {
        NODE_REGISTRY.get().and_then(|r| r.lamad_client())
    }
}

#[async_trait]
impl WitnessProbe for NodeWitnessProbe {
    async fn facts(&self, anchor: &str) -> Result<Option<WitnessFacts>, StorageError> {
        let hc = self
            .lamad()
            .ok_or_else(|| StorageError::Connection("no lamad conductor client".into()))?;
        let payload = rmp_serde::to_vec_named(&anchor.to_string())
            .map_err(|e| StorageError::Internal(format!("witness: encode anchor: {e}")))?;
        let bytes = hc.call_zome(ZOME, FACTS_FN, payload).await?;
        rmp_serde::from_slice(&bytes)
            .map_err(|e| StorageError::Serialization(format!("witness: decode facts: {e}")))
    }

    async fn local_agents(&self) -> Option<HashSet<String>> {
        let mut cached = self.agents.lock().await;
        if let Some((at, agents)) = cached.as_ref() {
            if at.elapsed() < LOCAL_AGENTS_TTL {
                return Some(agents.clone());
            }
        }
        let hc = self.lamad()?;
        let apps = hc.admin_websocket().list_apps(None).await.ok()?;
        let mut agents: HashSet<String> =
            apps.iter().map(|a| a.agent_pub_key.to_string()).collect();
        agents.insert(hc.agent_key_uhcak());
        *cached = Some((Instant::now(), agents.clone()));
        Some(agents)
    }

    async fn ask_one_peer(&self, content_id: &str, anchor: &str) -> bool {
        let Some(peers) = NODE_PEERS.get() else {
            return false;
        };
        let listed = peers.list_peers().await;
        if listed.is_empty() {
            return false;
        }
        // One peer per check, rotating so a re-check of an amber record asks a
        // different peer than the last one.
        let i = self
            .next_peer
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            % listed.len();
        let fetcher = crate::p2p::head_record_client::PeerHeadRecordFetcher::new(peers.as_ref());
        let answer = fetcher.fetch(&listed[i].peer_id, content_id).await;
        peer_holds(&answer, anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn facts(receipts: u32, by_caller: bool, held: bool) -> WitnessFacts {
        WitnessFacts {
            author: "uhCAkauthor".into(),
            authored_by_caller: by_caller,
            held_valid: held,
            record_op_receipts: receipts,
            record_op_valid_receipts: receipts,
            valid_receipts_all_ops: receipts,
        }
    }

    #[test]
    fn a_remote_receipt_is_a_witness() {
        assert_eq!(
            local_verdict(Some(&facts(1, true, true)), None),
            LocalVerdict::Witnessed
        );
    }

    #[test]
    fn authored_here_without_a_receipt_asks_a_peer() {
        assert_eq!(
            local_verdict(Some(&facts(0, true, true)), None),
            LocalVerdict::AskPeer
        );
        // A hosted agent on this conductor: the caller is not the author, but
        // the conductor's agent set holds it.
        assert_eq!(
            local_verdict(Some(&facts(0, false, true)), Some(true)),
            LocalVerdict::AskPeer
        );
    }

    #[test]
    fn a_non_authors_valid_copy_is_a_witness() {
        assert_eq!(
            local_verdict(Some(&facts(0, false, true)), Some(false)),
            LocalVerdict::Witnessed
        );
        assert_eq!(
            local_verdict(Some(&facts(0, false, false)), Some(false)),
            LocalVerdict::Awaiting,
            "held but not valid is no witness"
        );
    }

    #[test]
    fn unknown_locality_or_no_record_awaits() {
        assert_eq!(
            local_verdict(Some(&facts(0, false, true)), None),
            LocalVerdict::Awaiting
        );
        assert_eq!(local_verdict(None, Some(false)), LocalVerdict::Awaiting);
    }

    #[test]
    fn a_rejecting_receipt_alone_is_not_a_witness() {
        let mut f = facts(0, true, true);
        f.record_op_receipts = 2;
        assert_eq!(local_verdict(Some(&f), None), LocalVerdict::AskPeer);
    }

    #[test]
    fn only_a_served_record_for_this_exact_action_proves_a_peer_holds_it() {
        use seam_contracts::Answer;
        let carried = |head: &str, record: Option<Vec<u8>>| CarriedHeadRecord {
            head_action_hash: head.into(),
            record,
            record_absent_reason: None,
            election_link_record: None,
        };
        assert!(peer_holds(
            &Answer::Present(carried("uhCkkA", Some(vec![1]))),
            "uhCkkA"
        ));
        assert!(!peer_holds(
            &Answer::Present(carried("uhCkkA", None)),
            "uhCkkA"
        ));
        assert!(!peer_holds(
            &Answer::Present(carried("uhCkkB", Some(vec![1]))),
            "uhCkkA"
        ));
        assert!(!peer_holds(&Answer::Absent, "uhCkkA"));
        assert!(!peer_holds(&Answer::Unreachable, "uhCkkA"));
    }

    #[test]
    fn facts_decode_from_the_zome_wire_with_the_author_as_a_string() {
        #[derive(serde::Serialize)]
        struct ZomeSide<'a> {
            author: &'a str,
            authored_by_caller: bool,
            held_valid: bool,
            record_op_receipts: u32,
            record_op_valid_receipts: u32,
            valid_receipts_all_ops: u32,
        }
        let bytes = rmp_serde::to_vec_named(&Some(ZomeSide {
            author: "uhCAkzyTps",
            authored_by_caller: true,
            held_valid: true,
            record_op_receipts: 2,
            record_op_valid_receipts: 2,
            valid_receipts_all_ops: 6,
        }))
        .unwrap();
        let read: Option<WitnessFacts> = rmp_serde::from_slice(&bytes).unwrap();
        let read = read.expect("facts");
        assert_eq!(read.author, "uhCAkzyTps");
        assert_eq!(read.record_op_valid_receipts, 2);
        let none: Option<WitnessFacts> =
            rmp_serde::from_slice(&rmp_serde::to_vec_named(&None::<()>).unwrap()).unwrap();
        assert!(none.is_none());
    }

    /// A probe with scripted answers and call counters.
    struct Scripted {
        facts: Result<Option<WitnessFacts>, ()>,
        facts_delay: Duration,
        local: Option<HashSet<String>>,
        peer: bool,
        facts_calls: AtomicUsize,
        peer_calls: AtomicUsize,
    }

    impl Scripted {
        fn new(facts: Option<WitnessFacts>) -> Self {
            Self {
                facts: Ok(facts),
                facts_delay: Duration::ZERO,
                local: None,
                peer: false,
                facts_calls: AtomicUsize::new(0),
                peer_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl WitnessProbe for Scripted {
        async fn facts(&self, _anchor: &str) -> Result<Option<WitnessFacts>, StorageError> {
            self.facts_calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.facts_delay).await;
            self.facts
                .clone()
                .map_err(|_| StorageError::Connection("scripted".into()))
        }
        async fn local_agents(&self) -> Option<HashSet<String>> {
            self.local.clone()
        }
        async fn ask_one_peer(&self, _content_id: &str, _anchor: &str) -> bool {
            self.peer_calls.fetch_add(1, Ordering::SeqCst);
            self.peer
        }
    }

    #[tokio::test]
    async fn authored_here_reads_amber_until_a_peer_holds_it_then_green_is_cached() {
        let probe_arc = Arc::new(Scripted::new(Some(facts(0, true, true))));
        let dynp: Arc<dyn WitnessProbe> = probe_arc.clone();
        let budget = Duration::from_secs(5);
        let first = witness_for_within(dynp.clone(), "t1", "rec", Some("uhCkkA1"), budget).await;
        assert_eq!(first, WitnessEvidence::Awaiting);
        assert_eq!(probe_arc.peer_calls.load(Ordering::SeqCst), 1);

        // The peer now holds it: the next read asks again and turns green.
        let probe2 = Arc::new(Scripted {
            peer: true,
            ..Scripted::new(Some(facts(0, true, true)))
        });
        let dyn2: Arc<dyn WitnessProbe> = probe2.clone();
        let second = witness_for_within(dyn2.clone(), "t1", "rec", Some("uhCkkA1"), budget).await;
        assert_eq!(second, WitnessEvidence::Witnessed);

        // Green is kept: no further zome call for this anchor.
        let third = witness_for_within(dyn2, "t1", "rec", Some("uhCkkA1"), budget).await;
        assert_eq!(third, WitnessEvidence::Witnessed);
        assert_eq!(probe2.facts_calls.load(Ordering::SeqCst), 1);

        // A new anchor (an update) is read afresh.
        let fresh = witness_for_within(dynp, "t1", "rec", Some("uhCkkA2"), budget).await;
        assert_eq!(fresh, WitnessEvidence::Awaiting);
    }

    #[tokio::test]
    async fn a_receipt_needs_no_peer_ask() {
        let probe = Arc::new(Scripted::new(Some(facts(2, true, true))));
        let dynp: Arc<dyn WitnessProbe> = probe.clone();
        let got =
            witness_for_within(dynp, "t2", "rec", Some("uhCkkB"), Duration::from_secs(5)).await;
        assert_eq!(got, WitnessEvidence::Witnessed);
        assert_eq!(probe.peer_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_non_author_peer_is_green_from_its_own_valid_copy() {
        let probe = Arc::new(Scripted {
            local: Some(HashSet::from(["uhCAkme".to_string()])),
            ..Scripted::new(Some(facts(0, false, true)))
        });
        let dynp: Arc<dyn WitnessProbe> = probe.clone();
        let got =
            witness_for_within(dynp, "t3", "rec", Some("uhCkkC"), Duration::from_secs(5)).await;
        assert_eq!(got, WitnessEvidence::Witnessed);
        assert_eq!(probe.peer_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn an_error_or_a_slow_conductor_reads_amber_never_green() {
        let failing = Arc::new(Scripted {
            facts: Err(()),
            ..Scripted::new(None)
        });
        let got =
            witness_for_within(failing, "t4", "rec", Some("uhCkkD"), Duration::from_secs(5)).await;
        assert_eq!(got, WitnessEvidence::Unread);

        // Slower than the budget: amber now; the check finishes in the
        // background and its green serves the next read.
        let slow = Arc::new(Scripted {
            facts_delay: Duration::from_millis(200),
            ..Scripted::new(Some(facts(1, true, true)))
        });
        let dyns: Arc<dyn WitnessProbe> = slow.clone();
        let now = witness_for_within(
            dyns.clone(),
            "t5",
            "rec",
            Some("uhCkkE"),
            Duration::from_millis(20),
        )
        .await;
        assert_eq!(now, WitnessEvidence::Unread);
        tokio::time::sleep(Duration::from_millis(400)).await;
        let later =
            witness_for_within(dyns, "t5", "rec", Some("uhCkkE"), Duration::from_millis(20)).await;
        assert_eq!(later, WitnessEvidence::Witnessed);
        assert_eq!(slow.facts_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_reads_share_one_check() {
        let probe = Arc::new(Scripted {
            facts_delay: Duration::from_millis(50),
            ..Scripted::new(Some(facts(0, true, true)))
        });
        let dynp: Arc<dyn WitnessProbe> = probe.clone();
        let budget = Duration::from_secs(5);
        let (a, b) = tokio::join!(
            witness_for_within(dynp.clone(), "t6", "rec", Some("uhCkkF"), budget),
            witness_for_within(dynp.clone(), "t6", "rec", Some("uhCkkF"), budget),
        );
        assert_eq!(
            (a, b),
            (WitnessEvidence::Awaiting, WitnessEvidence::Awaiting)
        );
        assert_eq!(probe.facts_calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.peer_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_blank_anchor_is_not_checked() {
        let probe = Arc::new(Scripted::new(Some(facts(2, true, true))));
        let dynp: Arc<dyn WitnessProbe> = probe.clone();
        for anchor in [None, Some(""), Some("  ")] {
            let got =
                witness_for_within(dynp.clone(), "t7", "rec", anchor, Duration::from_secs(5)).await;
            assert_eq!(got, WitnessEvidence::Unread);
        }
        assert_eq!(probe.facts_calls.load(Ordering::SeqCst), 0);
    }
}
