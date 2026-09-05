//! Per-space partition probe (Holochain Evolution Epic MVP, Task 32).
//!
//! # The thing this makes visible
//!
//! When a peer authors an action on a chain the network has already seen
//! closed, every neighbour warrants the author and turns the accepted warrant
//! into a **permanent cell block** (`Timestamp::max()`; holochain 0.7 has no
//! unblock). Task 30 read that state straight out of the household's conductor
//! databases: three block rows, one warrant each, all citing *"No more actions
//! are allowed after a chain close"* over two `CapGrant` creates.
//!
//! From the outside the symptom is silence. The conductors report "ready", the
//! relay is up, the app answers — and **zero gossip rounds ever complete**,
//! because each peer is executing a correct-by-its-own-rules refusal of the
//! others. Nobody was told. matthew's own database held no warrant and no
//! rejected op, so matthew never learned why it had gone quiet.
//!
//! [`ClosedChainFence`](crate::closed_chain_fence) stops storage from EARNING
//! that state. This module makes the state SAYABLE while it lasts, so a driver
//! (the a2o `@concern:happ-lineage-migration` pre-flight, an operator reading
//! `/version`) never has to infer a partition from an absence.
//!
//! # The classifier
//!
//! Three facts, read per space out of `dump_network_metrics`, across **two**
//! samples taken after a boot grace:
//!
//! 1. **`storage_arc` is empty on every local agent.** A node that has declared
//!    no arc is authority for nothing and will be gossiped nothing.
//! 2. **No peer has ever completed a gossip round with us.** A blocked cell
//!    never gets past round setup, so `completed_rounds` stays absent (or nailed
//!    at zero — the predicate takes `None` and `Some(0)` as the same fact,
//!    because a peer stuck at zero is exactly as partitioned as one that never
//!    reported).
//! 3. **Liveness, in ONE of two shapes.** Either
//!    `peer_timeouts` is RISING between the two samples (`shape: "timeouts"`),
//!    or this space's `peer_meta` has been EMPTY across both samples while
//!    another space on the same conductor holds peers (`shape: "isolated"`).
//!
//!    The first separates a partition from a quiet, healthy, idle node: an idle
//!    node's counters do not move; a node whose neighbours are refusing it keeps
//!    trying and keeps timing out.
//!
//!    The second was MEASURED (2026-09-05, the poisoned household). matthew and
//!    jessica — the two peers that were blocked — named the node-registry space
//!    four minutes after restart on the rising-timeouts shape (377 → 378, arc
//!    null, no completed rounds). **james, the peer that BLOCKED both of them,
//!    was never named**: he knows no peers in that space at all, so nothing
//!    initiates, nothing times out, and the counter never rises. The blocker
//!    was the one peer the probe could not see. Emptiness is only evidence when
//!    the conductor is otherwise fine, which is why the other-spaces clause is
//!    load-bearing rather than decorative — a conductor with no peers anywhere
//!    is offline, not partitioned, and that is a different fault.
//!
//! Conditions 1, 2 and one arm of 3, on two consecutive samples, or nothing is
//! claimed. The probe is
//! deliberately conservative — a false "you are partitioned" would send an
//! operator hunting a fault that is not there, and the honest cost of missing
//! one cycle is a single extra probe interval.
//!
//! # Why the sample is taken from JSON and not from the typed struct
//!
//! The conductor's metrics types (`holochain_types::network::Kitsune2NetworkMetrics`
//! over `kitsune2_api::GossipStateSummary`) are deep, are moving under the 0.7
//! upgrade, and are impractical to construct in a unit test. Serializing them
//! once with `serde_json` gives the classifier a shape it can be tested
//! against with a realistic fixture, and it lets the extractor degrade to
//! "unknown" on a field that moves rather than failing to compile or panicking.
//! Both snake_case and camelCase spellings are accepted for the same reason.

use std::collections::BTreeMap;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;

/// Seconds between probe samples.
pub const PARTITION_PROBE_SECS: u64 = 60;
/// Environment override for [`PARTITION_PROBE_SECS`], read ONCE at construction.
pub const PARTITION_PROBE_SECS_ENV: &str = "LINEAGE_PARTITION_PROBE_SECS";

/// How long after process start the probe declines to classify anything.
///
/// A conductor legitimately shows an empty arc and no completed rounds for the
/// first minutes of its life; naming that a partition would make the signal
/// worthless. Two samples must both fall AFTER this grace.
pub const PARTITION_BOOT_GRACE_SECS: u64 = 180;
/// Environment override for [`PARTITION_BOOT_GRACE_SECS`], read ONCE at construction.
pub const PARTITION_BOOT_GRACE_SECS_ENV: &str = "LINEAGE_PARTITION_BOOT_GRACE_SECS";

/// Minimum interval between WARN lines for the SAME space. The passport field
/// is always current; the log is rate-limited so a partition that lasts an hour
/// does not produce sixty identical lines.
pub const PARTITION_WARN_EVERY_SECS: u64 = 900;

/// The three facts one sample carries about one space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceSample {
    /// How many local agents this node runs in the space.
    pub local_agents: usize,
    /// True when EVERY local agent declares an empty (`null`) storage arc.
    /// False when any agent declares a real arc, and false when there are no
    /// local agents at all (nothing is being claimed either way).
    pub arc_empty: bool,
    /// The highest `completed_rounds` any peer reports, with `None` meaning no
    /// peer reported one.
    pub completed_rounds: Option<u32>,
    /// The sum of `peer_timeouts` across every peer we hold metadata for.
    pub peer_timeouts: u64,
    /// How many peers we hold metadata for.
    pub peers: usize,
    /// The arc the FIRST local agent declares, when it declares one at all.
    /// Carried through to [`SpacePartition::arc`] so that field reports what was
    /// sampled rather than restating the classifier's own precondition.
    pub arc: Option<[u32; 2]>,
}

impl SpaceSample {
    /// No peer has ever completed a gossip round with us. `None` and `Some(0)`
    /// are the same fact — see the module docs.
    fn no_completed_rounds(&self) -> bool {
        self.completed_rounds.unwrap_or(0) == 0
    }

    /// Positive evidence that this space is REACHABLE again: somebody completed
    /// a round with us, or we are declaring an arc. Either negates a condition
    /// the classification rests on.
    ///
    /// A plateau in `peer_timeouts` is deliberately NOT here — see
    /// [`LineagePartitionProbe::observe`].
    fn cleared(&self) -> bool {
        !self.no_completed_rounds() || (!self.arc_empty && self.local_agents > 0)
    }
}

/// WHICH of the two liveness shapes named this space. Rendered on the passport
/// entry, because the two are read differently by whoever is holding the
/// evidence: `timeouts` is a peer that keeps reaching and keeps being refused;
/// `isolated` is a peer that has nobody left to reach — the BLOCKER's own view
/// of the same event, and the one the first classifier could not see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionShape {
    /// `peer_timeouts` rose between the two samples.
    Timeouts,
    /// This space held NO peer metadata across both samples, while another
    /// space on the same conductor held some.
    Isolated,
}

impl PartitionShape {
    /// The name as it appears on the passport and in the WARN's `shape` field.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Timeouts => "timeouts",
            Self::Isolated => "isolated",
        }
    }
}

/// The WARN's evidence clause — ONE SENTENCE PER SHAPE.
///
/// The two preconditions (empty arc, no completed round) are shared and stay in
/// the message body; the evidence is what differs. Saying "peer timeouts are
/// still rising" over the ISOLATED shape would be a false statement about the
/// one peer the widening exists to name — the BLOCKER, who knows nobody in the
/// space and therefore times nothing out. The remediation is the same for both,
/// so it stays shared.
///
/// Pure and public so the two clauses can be pinned by a test; a `tracing`
/// macro's rendered output is not otherwise reachable.
pub fn shape_evidence(shape: PartitionShape) -> &'static str {
    match shape {
        PartitionShape::Timeouts => {
            "and peer timeouts are still RISING — it keeps reaching for peers that refuse it"
        }
        PartitionShape::Isolated => {
            "and it holds NO peer metadata for this space at all while it still knows peers in \
             others — nothing left to reach, which is the BLOCKER's own view of this event"
        }
    }
}

/// A space this node believes it is partitioned from, as rendered on the
/// passport under `passport.lineage.partition`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpacePartition {
    /// The space — the DNA hash, canonically rendered.
    pub space: String,
    /// RFC3339 UTC timestamp of the SECOND (confirming) sample.
    pub sampled_at: String,
    /// Total `peer_timeouts` at the confirming sample.
    pub peer_timeouts: u64,
    /// Total `peer_timeouts` at the previous sample — the rise is the signal,
    /// so both ends of it are reported rather than the delta alone.
    pub peer_timeouts_prev: u64,
    /// The declared storage arc AS SAMPLED, carried from
    /// [`SpaceSample::arc`]. An empty arc is a precondition of the
    /// classification, so today this is always `null` on a row that appears
    /// here — but it is carried, not hardcoded, so a future classifier that
    /// admits a narrow arc reports the truth without touching this field.
    /// Serialized rather than skipped: an absent key would read as "not
    /// measured".
    pub arc: Option<[u32; 2]>,
    /// The highest `completed_rounds` any peer reported, `null` when none did.
    pub completed_rounds: Option<u32>,
    /// How many peers we hold gossip metadata for. Zero on the `isolated`
    /// shape — that emptiness IS the signal there.
    pub peers: usize,
    /// Which liveness arm named this space. See [`PartitionShape`].
    pub shape: PartitionShape,
}

/// Extract one space's sample from the JSON projection of that space's
/// `Kitsune2NetworkMetrics`.
///
/// Returns `None` only when the value is not an object at all. Every field is
/// individually tolerant: a missing `local_agents` yields `arc_empty == false`
/// (nothing claimed), a missing `peer_meta` yields zero peers and zero
/// timeouts. Tolerance here is deliberate — a metrics shape that moves under
/// the 0.7 upgrade must make the probe go QUIET, never make it lie.
pub fn sample_from_metrics_json(metrics: &serde_json::Value) -> Option<SpaceSample> {
    let obj = metrics.as_object()?;

    let local_agents = get_either(obj, "local_agents", "localAgents")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let arc_empty = !local_agents.is_empty()
        && local_agents
            .iter()
            .all(|agent| agent_arc(agent).is_none_or(serde_json::Value::is_null));
    let arc = local_agents
        .first()
        .and_then(agent_arc)
        .and_then(|v| v.as_array())
        .and_then(|bounds| match bounds.as_slice() {
            [lo, hi] => Some([lo.as_u64()? as u32, hi.as_u64()? as u32]),
            _ => None,
        });

    let gossip =
        get_either(obj, "gossip_state_summary", "gossipStateSummary").and_then(|v| v.as_object());
    let peer_meta = gossip
        .and_then(|g| get_either(g, "peer_meta", "peerMeta"))
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut completed_rounds: Option<u32> = None;
    let mut peer_timeouts: u64 = 0;
    for peer in peer_meta.values() {
        let Some(peer) = peer.as_object() else {
            continue;
        };
        if let Some(rounds) = get_either(peer, "completed_rounds", "completedRounds")
            .and_then(serde_json::Value::as_u64)
        {
            let rounds = rounds.min(u32::MAX as u64) as u32;
            completed_rounds = Some(completed_rounds.map_or(rounds, |best| best.max(rounds)));
        }
        peer_timeouts = peer_timeouts.saturating_add(
            get_either(peer, "peer_timeouts", "peerTimeouts")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        );
    }

    Some(SpaceSample {
        local_agents: local_agents.len(),
        arc_empty,
        completed_rounds,
        peer_timeouts,
        peers: peer_meta.len(),
        arc,
    })
}

/// Is this space partitioned, given two consecutive samples — and if so, in
/// which shape?
///
/// `prev` is the earlier one. `peers_elsewhere` is whether ANOTHER space on
/// this conductor holds peer metadata; it gates the `isolated` arm alone, so
/// that a conductor which knows nobody anywhere reads as offline (a different
/// fault) rather than as partitioned everywhere.
///
/// The two preconditions — empty arc, no completed round — are shared. The two
/// liveness arms are mutually exclusive by construction (`peers > 0` versus
/// `peers == 0`), so there is never a shape to choose between.
pub fn classify(
    prev: &SpaceSample,
    next: &SpaceSample,
    peers_elsewhere: bool,
) -> Option<PartitionShape> {
    let preconditions = prev.arc_empty
        && next.arc_empty
        && prev.no_completed_rounds()
        && next.no_completed_rounds();
    if !preconditions {
        return None;
    }
    if next.peers > 0 && next.peer_timeouts > prev.peer_timeouts {
        return Some(PartitionShape::Timeouts);
    }
    if peers_elsewhere && prev.peers == 0 && next.peers == 0 {
        return Some(PartitionShape::Isolated);
    }
    None
}

/// One local agent's declared storage arc, in either spelling.
fn agent_arc(agent: &serde_json::Value) -> Option<&serde_json::Value> {
    agent
        .as_object()
        .and_then(|a| get_either(a, "storage_arc", "storageArc"))
}

fn get_either<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    snake: &str,
    camel: &str,
) -> Option<&'a serde_json::Value> {
    obj.get(snake).or_else(|| obj.get(camel))
}

/// The running probe: one sample per interval, per space, with the previous
/// sample kept so the classifier always has two.
#[derive(Debug)]
pub struct LineagePartitionProbe {
    /// `None` on a node with no conductor admin connection (and in the unit
    /// tests, which drive [`LineagePartitionProbe::observe`] directly). A tick
    /// with no handle takes no sample and classifies nothing.
    admin: Option<holochain_client::AdminWebsocket>,
    interval: Duration,
    boot_grace: Duration,
    started: Instant,
    previous: Mutex<BTreeMap<String, SpaceSample>>,
    partitions: RwLock<BTreeMap<String, SpacePartition>>,
    last_warn: Mutex<BTreeMap<String, Instant>>,
}

impl LineagePartitionProbe {
    /// Both intervals are read from the environment ONCE, here — never on the
    /// tick path, so a test that sets one cannot race a test that reads one.
    pub fn new(admin: holochain_client::AdminWebsocket) -> Self {
        Self::with_admin(Some(admin))
    }

    /// The constructor the tests use, with no conductor behind it.
    pub fn detached() -> Self {
        Self::with_admin(None)
    }

    fn with_admin(admin: Option<holochain_client::AdminWebsocket>) -> Self {
        Self {
            admin,
            interval: env_secs(PARTITION_PROBE_SECS_ENV, PARTITION_PROBE_SECS),
            boot_grace: env_secs(PARTITION_BOOT_GRACE_SECS_ENV, PARTITION_BOOT_GRACE_SECS),
            started: Instant::now(),
            previous: Mutex::new(BTreeMap::new()),
            partitions: RwLock::new(BTreeMap::new()),
            last_warn: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn boot_grace(&self) -> Duration {
        self.boot_grace
    }

    /// Every space currently classified partitioned, sorted by space. Empty —
    /// and so absent from `/version` — on a healthy node, which is byte-identical
    /// to the pre-Task-32 passport.
    pub fn snapshot(&self) -> Vec<SpacePartition> {
        self.partitions
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Fold one freshly-taken sample in, returning the space's verdict.
    ///
    /// Split out from [`Self::tick`] so the two-sample rule is testable without
    /// a conductor: the FIRST call for a space can never classify (there is no
    /// `prev`), the second can.
    ///
    /// # A named partition is STICKY until it is positively cleared
    ///
    /// The returned bool is "did THIS pair of samples classify"; the passport
    /// entry is not. Once named, a space stays named until a sample shows a
    /// completed gossip round or a declared arc ([`SpaceSample::cleared`]).
    ///
    /// The reason is that condition 3 (rising timeouts) is a liveness signal,
    /// and a partitioned peer can go quiet without recovering: neighbours give
    /// up retrying, a backoff saturates, and the counter plateaus. Dropping the
    /// entry there would clear the field while the partition holds — and the
    /// a2o pre-flight reads it at ONE instant, so that false negative lands
    /// exactly where it costs most. A stale-but-true entry is the safer error:
    /// it carries `sampledAt` from its last CONFIRMING sample, so a reader can
    /// always see how old the confirmation is.
    pub fn observe(&self, space: &str, sample: SpaceSample, sampled_at: String) -> bool {
        // The `isolated` arm needs one fact from OUTSIDE this space: does the
        // conductor hold peers anywhere else? It is read from the probe's own
        // sample map rather than plumbed through `tick`, because the map
        // already IS "the most recent sample of every space" — and a second
        // path to the same fact is how the two would come to disagree.
        //
        // Taken BEFORE this space's sample is inserted, and excluding this
        // space either way, so a space can never vouch for its own emptiness.
        let (prev, peers_elsewhere) = {
            let mut previous = self.previous.lock().unwrap_or_else(|e| e.into_inner());
            let peers_elsewhere = previous
                .iter()
                .any(|(other, s)| other != space && s.peers > 0);
            (previous.insert(space.to_string(), sample), peers_elsewhere)
        };
        let Some(prev) = prev else {
            return false;
        };
        let shape = classify(&prev, &sample, peers_elsewhere);

        let mut partitions = self.partitions.write().unwrap_or_else(|e| e.into_inner());
        if let Some(shape) = shape {
            partitions.insert(
                space.to_string(),
                SpacePartition {
                    space: space.to_string(),
                    sampled_at,
                    peer_timeouts: sample.peer_timeouts,
                    peer_timeouts_prev: prev.peer_timeouts,
                    arc: sample.arc,
                    completed_rounds: sample.completed_rounds,
                    peers: sample.peers,
                    shape,
                },
            );
        } else if sample.cleared() {
            // Only a POSITIVE recovery un-names a space; see the doc above.
            partitions.remove(space);
        }
        shape.is_some()
    }

    /// One WARN per space per [`PARTITION_WARN_EVERY_SECS`].
    fn warn_rate_limited(&self, partition: &SpacePartition) {
        let mut last = self.last_warn.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let due = last
            .get(&partition.space)
            .is_none_or(|prev| now.duration_since(*prev).as_secs() >= PARTITION_WARN_EVERY_SECS);
        if !due {
            return;
        }
        last.insert(partition.space.clone(), now);
        let evidence = shape_evidence(partition.shape);
        tracing::warn!(
            space = %partition.space,
            peer_timeouts = partition.peer_timeouts,
            peer_timeouts_prev = partition.peer_timeouts_prev,
            peers = partition.peers,
            shape = partition.shape.as_str(),
            "SPACE PARTITIONED — this node declares an empty storage arc, no peer has completed a \
             gossip round with it, {evidence}. The known cause of this shape is a permanent cell \
             block earned by an action authored after a chain close (holochain 0.7 cannot lift \
             one). Read the blocks with `hc-mesh.sh blocks`; the partition is named on /version \
             under passport.lineage.partition."
        );
    }

    async fn tick(&self) {
        if self.started.elapsed() < self.boot_grace {
            return;
        }
        let Some(admin) = self.admin.as_ref() else {
            return;
        };
        let metrics = match admin.dump_network_metrics(None, false).await {
            Ok(metrics) => metrics,
            Err(e) => {
                tracing::debug!(error = %e, "partition probe: dump_network_metrics unavailable");
                return;
            }
        };
        let sampled_at = now_rfc3339();
        for (dna, m) in metrics {
            let Ok(value) = serde_json::to_value(&m) else {
                continue;
            };
            let Some(sample) = sample_from_metrics_json(&value) else {
                continue;
            };
            let space = dna.to_string();
            if self.observe(&space, sample, sampled_at.clone()) {
                if let Some(partition) = self
                    .partitions
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(&space)
                    .cloned()
                {
                    self.warn_rate_limited(&partition);
                }
            }
        }
    }

    /// Spawn the ticker. One task for the life of the process, shaped exactly
    /// like [`crate::services::lineage_bridge::LineageBridge::spawn`].
    pub fn spawn(self: std::sync::Arc<Self>, mut shutdown: tokio::sync::broadcast::Receiver<()>) {
        let interval = self.interval;
        let grace = self.boot_grace;
        tokio::spawn(async move {
            tracing::info!(
                interval_secs = interval.as_secs(),
                boot_grace_secs = grace.as_secs(),
                "lineage partition probe armed (two samples confirm; silent until then)"
            );
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = shutdown.recv() => {
                        tracing::info!("lineage partition probe exiting (shutdown)");
                        return;
                    }
                }
                self.tick().await;
            }
        });
    }
}

fn env_secs(key: &str, default: u64) -> Duration {
    let secs = std::env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(default);
    Duration::from_secs(secs)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(arc_empty: bool, rounds: Option<u32>, timeouts: u64, peers: usize) -> SpaceSample {
        SpaceSample {
            local_agents: 1,
            arc_empty,
            completed_rounds: rounds,
            peer_timeouts: timeouts,
            peers,
            arc: if arc_empty { None } else { Some([0, u32::MAX]) },
        }
    }

    /// The exact shape a partitioned space produces, in the conductor's own
    /// snake_case spelling.
    fn partitioned_metrics_json() -> serde_json::Value {
        serde_json::json!({
            "fetch_state_summary": { "pending_requests": {} },
            "gossip_state_summary": {
                "initiated_round": null,
                "accepted_rounds": [],
                "dht_summary": {},
                "local_op_count": 12,
                "peer_meta": {
                    "wss://relay/a": {
                        "completed_rounds": null,
                        "peer_timeouts": 7,
                        "is_tombstone": false,
                        "storage_arc": null
                    },
                    "wss://relay/b": {
                        "completed_rounds": null,
                        "peer_timeouts": 4,
                        "is_tombstone": false,
                        "storage_arc": null
                    }
                }
            },
            "local_agents": [
                { "agent": "uhCAk…", "storage_arc": null, "target_arc": [0, 4294967295u32] }
            ]
        })
    }

    #[test]
    fn the_partitioned_shape_extracts_all_three_facts() {
        let s = sample_from_metrics_json(&partitioned_metrics_json()).expect("object");
        assert_eq!(s.local_agents, 1);
        assert!(s.arc_empty, "a null storage_arc is the empty arc");
        assert_eq!(s.completed_rounds, None);
        assert_eq!(s.peer_timeouts, 11, "timeouts sum across peers");
        assert_eq!(s.peers, 2);
    }

    #[test]
    fn a_declared_arc_is_not_the_partitioned_shape() {
        let mut v = partitioned_metrics_json();
        v["local_agents"][0]["storage_arc"] = serde_json::json!([0, 4294967295u32]);
        let s = sample_from_metrics_json(&v).expect("object");
        assert!(!s.arc_empty);
    }

    #[test]
    fn camel_case_metrics_extract_identically() {
        let v = serde_json::json!({
            "gossipStateSummary": {
                "peerMeta": {
                    "wss://relay/a": { "completedRounds": null, "peerTimeouts": 7 },
                    "wss://relay/b": { "completedRounds": null, "peerTimeouts": 4 }
                }
            },
            "localAgents": [ { "storageArc": null } ]
        });
        let s = sample_from_metrics_json(&v).expect("object");
        assert!(s.arc_empty);
        assert_eq!(s.peer_timeouts, 11);
        assert_eq!(s.peers, 2);
    }

    #[test]
    fn an_unrecognised_metrics_shape_goes_quiet_rather_than_lying() {
        let s = sample_from_metrics_json(&serde_json::json!({})).expect("object");
        assert!(!s.arc_empty, "nothing claimed means no arc claim either");
        assert_eq!(s.peers, 0);
        assert!(
            classify(&s, &s, false).is_none(),
            "an unknown shape can never be classified"
        );
        assert!(
            classify(&s, &s, true).is_none(),
            "and not even when the conductor has peers elsewhere: an unknown shape's arc is not \
             empty, so the preconditions never hold"
        );
        assert!(sample_from_metrics_json(&serde_json::json!("nope")).is_none());
    }

    #[test]
    fn all_three_conditions_are_required() {
        let prev = sample(true, None, 10, 2);
        assert_eq!(
            classify(&prev, &sample(true, None, 11, 2), false),
            Some(PartitionShape::Timeouts),
            "empty arc + no rounds + rising timeouts"
        );
        assert!(
            classify(&prev, &sample(false, None, 11, 2), false).is_none(),
            "a declared arc is not a partition"
        );
        assert!(
            classify(&prev, &sample(true, Some(3), 11, 2), false).is_none(),
            "a completed round proves the space is reachable"
        );
        assert!(
            classify(&prev, &sample(true, None, 10, 2), false).is_none(),
            "flat timeouts are an idle node, not a partitioned one"
        );
        assert!(
            classify(&prev, &sample(true, None, 9, 2), false).is_none(),
            "falling timeouts are not rising"
        );
        assert!(
            classify(&prev, &sample(true, None, 11, 0), false).is_none(),
            "no peers at all is not evidence of refusal — unless the conductor has peers \
             ELSEWHERE, which is the isolated shape and needs BOTH samples empty"
        );
    }

    #[test]
    fn zero_completed_rounds_reads_the_same_as_none() {
        let prev = sample(true, Some(0), 10, 2);
        assert_eq!(
            classify(&prev, &sample(true, Some(0), 11, 2), false),
            Some(PartitionShape::Timeouts)
        );
    }

    /// **MEASURED 2026-09-05 — the BLOCKER's own view.** matthew and jessica,
    /// the two blocked peers, named the space on rising timeouts. james, the
    /// peer that blocked them both, knows nobody in that space at all: nothing
    /// initiates, nothing times out, and the rising-timeouts arm can never
    /// fire. Emptiness across two samples, while the conductor holds peers in
    /// another space, IS the signal — and the other-spaces clause is what keeps
    /// an offline conductor (no peers anywhere) from reading as partitioned
    /// everywhere.
    #[test]
    fn the_isolated_shape_names_the_peer_that_knows_nobody() {
        // Both samples empty, and the conductor has peers elsewhere.
        let prev = sample(true, None, 0, 0);
        assert_eq!(
            classify(&prev, &sample(true, None, 0, 0), true),
            Some(PartitionShape::Isolated),
            "no peers, twice, while other spaces have some"
        );
        // The same pair on a conductor that knows nobody ANYWHERE is offline,
        // not partitioned — a different fault, and not this probe's to claim.
        assert!(
            classify(&prev, &sample(true, None, 0, 0), false).is_none(),
            "a conductor with no peers in any space is offline, not partitioned"
        );
        // One empty sample is not two: a space that has just lost its peers
        // waits a full interval like every other classification here.
        assert!(
            classify(&sample(true, None, 3, 1), &sample(true, None, 3, 0), true).is_none(),
            "the space held a peer one sample ago"
        );
        // The shared preconditions still gate it.
        assert!(
            classify(&prev, &sample(false, None, 0, 0), true).is_none(),
            "a declared arc is not a partition, in either shape"
        );
        assert!(
            classify(&prev, &sample(true, Some(2), 0, 0), true).is_none(),
            "a completed round proves the space is reachable, in either shape"
        );

        // End to end through `observe`, where `peers_elsewhere` is read off the
        // probe's own sample map: the healthy space is sampled FIRST so the
        // isolated one has something to be measured against.
        let probe = probe_without_conductor();
        assert!(!probe.observe("space-healthy", sample(false, Some(4), 0, 3), "t0".into()));
        assert!(!probe.observe("space-blocked", sample(true, None, 0, 0), "t0".into()));
        assert!(!probe.observe("space-healthy", sample(false, Some(5), 0, 3), "t1".into()));
        assert!(
            probe.observe("space-blocked", sample(true, None, 0, 0), "t1".into()),
            "the blocker's own space is named on the second empty sample"
        );

        let named = probe.snapshot();
        assert_eq!(named.len(), 1, "only the blocked space is named");
        assert_eq!(named[0].space, "space-blocked");
        assert_eq!(named[0].shape, PartitionShape::Isolated);
        assert_eq!(named[0].peers, 0, "the emptiness IS the signal here");
        assert_eq!(
            serde_json::to_value(&named[0]).expect("serializable")["shape"],
            serde_json::json!("isolated"),
            "the passport entry carries the shape name"
        );
    }

    #[test]
    fn one_sample_never_classifies_and_the_second_one_can() {
        let probe = probe_without_conductor();
        assert!(
            !probe.observe("space-a", sample(true, None, 10, 2), "t0".into()),
            "the first sample has nothing to compare against"
        );
        assert!(probe.snapshot().is_empty());

        assert!(probe.observe("space-a", sample(true, None, 11, 2), "t1".into()));
        let named = probe.snapshot();
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].space, "space-a");
        assert_eq!(named[0].peer_timeouts, 11);
        assert_eq!(named[0].peer_timeouts_prev, 10);
        assert_eq!(named[0].arc, None);
        assert_eq!(named[0].sampled_at, "t1");
    }

    #[test]
    fn a_space_that_recovers_is_dropped_from_the_passport() {
        let probe = probe_without_conductor();
        probe.observe("space-a", sample(true, None, 10, 2), "t0".into());
        probe.observe("space-a", sample(true, None, 11, 2), "t1".into());
        assert_eq!(probe.snapshot().len(), 1);
        // A completed round: the space is reachable again.
        probe.observe("space-a", sample(true, Some(1), 12, 2), "t2".into());
        assert!(
            probe.snapshot().is_empty(),
            "the passport must never keep naming a partition that has cleared"
        );
    }

    #[test]
    fn a_plateaued_partition_stays_named() {
        // M4. Rising timeouts are a LIVENESS signal, and a partitioned peer can
        // go quiet without recovering — neighbours stop retrying, a backoff
        // saturates. Dropping the entry there clears the field while the
        // partition holds, and the pre-flight reads it at one instant.
        let probe = probe_without_conductor();
        probe.observe("space-a", sample(true, None, 10, 2), "t0".into());
        assert!(probe.observe("space-a", sample(true, None, 11, 2), "t1".into()));

        // Timeouts plateau: this pair does not classify …
        assert!(!probe.observe("space-a", sample(true, None, 11, 2), "t2".into()));
        // … but the space is still named, still carrying its last CONFIRMING
        // timestamp so a reader can see how old the confirmation is.
        let named = probe.snapshot();
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].sampled_at, "t1");
    }

    #[test]
    fn a_declared_arc_alone_clears_a_named_partition() {
        let probe = probe_without_conductor();
        probe.observe("space-a", sample(true, None, 10, 2), "t0".into());
        probe.observe("space-a", sample(true, None, 11, 2), "t1".into());
        assert_eq!(probe.snapshot().len(), 1);
        // No completed round yet, but we are declaring an arc again.
        probe.observe("space-a", sample(false, None, 11, 2), "t2".into());
        assert!(probe.snapshot().is_empty());
    }

    #[test]
    fn the_sampled_arc_is_carried_not_invented() {
        // M6. `arc` reports what was sampled. It is null on every row that can
        // appear today (an empty arc is a precondition), but it is carried, so
        // it cannot silently become a lie if the classifier ever widens.
        let mut v = partitioned_metrics_json();
        let s = sample_from_metrics_json(&v).expect("object");
        assert_eq!(s.arc, None);

        v["local_agents"][0]["storage_arc"] = serde_json::json!([7u32, 9u32]);
        let s = sample_from_metrics_json(&v).expect("object");
        assert_eq!(s.arc, Some([7, 9]));
    }

    #[test]
    fn spaces_are_tracked_independently() {
        let probe = probe_without_conductor();
        probe.observe("space-a", sample(true, None, 10, 2), "t0".into());
        probe.observe("space-b", sample(false, Some(4), 0, 2), "t0".into());
        probe.observe("space-a", sample(true, None, 11, 2), "t1".into());
        probe.observe("space-b", sample(false, Some(5), 0, 2), "t1".into());
        let named = probe.snapshot();
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].space, "space-a");
    }

    /// **Fix round 1, M2.** The evidence clause is per shape, and the
    /// `isolated` one must not claim rising timeouts — that is exactly what is
    /// NOT true of the blocker.
    #[test]
    fn the_warn_evidence_clause_says_one_true_thing_per_shape() {
        let timeouts = shape_evidence(PartitionShape::Timeouts);
        let isolated = shape_evidence(PartitionShape::Isolated);
        assert_ne!(timeouts, isolated);
        assert!(timeouts.contains("RISING"), "{timeouts}");
        assert!(
            !isolated.to_lowercase().contains("rising"),
            "the blocker times nothing out; claiming rising timeouts about it is a lie: {isolated}"
        );
        assert!(
            isolated.contains("NO peer metadata"),
            "the isolated clause names the emptiness that IS its signal: {isolated}"
        );
    }

    #[test]
    fn the_partition_view_serializes_the_null_arc_rather_than_omitting_it() {
        let view = SpacePartition {
            space: "uhC0k…".into(),
            sampled_at: "2026-09-05T00:00:00Z".into(),
            peer_timeouts: 11,
            peer_timeouts_prev: 10,
            arc: None,
            completed_rounds: None,
            peers: 2,
            shape: PartitionShape::Timeouts,
        };
        let json = serde_json::to_value(&view).expect("serialize");
        assert!(
            json.get("arc").is_some(),
            "an absent key would read as unmeasured"
        );
        assert!(json["arc"].is_null());
        assert_eq!(json["sampledAt"], "2026-09-05T00:00:00Z");
        assert_eq!(json["peerTimeouts"], 11);
        assert_eq!(json["shape"], "timeouts");
    }

    /// A probe with no conductor behind it: every test above drives
    /// `observe`/`classify`, which touch no conductor.
    fn probe_without_conductor() -> LineagePartitionProbe {
        LineagePartitionProbe::detached()
    }
}
