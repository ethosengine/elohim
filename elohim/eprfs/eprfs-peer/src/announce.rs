//! The channel's announce rhythm and how a device judges the heads it hears (spec §3.2–§3.3).
//!
//! A device announces after each change of its own channel root (debounced, [`ANNOUNCE_DEBOUNCE`])
//! and every [`REANNOUNCE_EVERY`] regardless, each time with a fresh `seq`. Folding another
//! device's head whose root moved schedules a re-announce, so its acknowledgement (`folded`)
//! reaches the sender within a couple of seconds. A head whose root did not move (a periodic
//! re-announce, or the other device acknowledging us) is recorded but not answered at once:
//! acknowledging an acknowledgement would have two devices re-announce to each other forever.
//! The periodic re-announce carries it.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use elohim_epr_rea::SignatureVerifier;
use eprfs_core::BlobCid;
use iroh_gossip::proto::TopicId;

use crate::admission::Admission;
use crate::error::PeerError;
use crate::head::{ChannelHead, HeadSigner};
use crate::head_table::{
    ChannelCounters, DeviceHead, HeadTable, NackLeaf, OwnAnnounce, OWN_LOG_LEN,
};

/// How long a change waits for others before it is announced.
pub const ANNOUNCE_DEBOUNCE: Duration = Duration::from_secs(1);

/// How often a device announces with nothing changed: its liveness for the others' freshness.
pub const REANNOUNCE_EVERY: Duration = Duration::from_secs(30);

/// The topic prefix the steward channel's topic id is derived from.
pub const TOPIC_PREFIX: &str = "elohim/steward-channel/1/";

/// The gossip topic of one person's devices: `blake3("elohim/steward-channel/1/" ‖ chain_root)`.
pub fn topic_for(chain_root: &str) -> TopicId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TOPIC_PREFIX.as_bytes());
    hasher.update(chain_root.as_bytes());
    TopicId::from_bytes(*hasher.finalize().as_bytes())
}

/// What a device does with a head it heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadVerdict {
    /// From an admitted device, signed by it, newer than the last folded: fold it.
    Fold,
    /// This device's own head heard back: ignored, not counted.
    Own,
    /// From a device admission does not admit now (unknown, superseded or contested): dropped
    /// and counted.
    Unadmitted,
    /// Its signature does not verify against the device it names: dropped and counted.
    Forged,
    /// A lower `seq` than the last folded from that device: dropped and counted.
    Rollback,
    /// The same `seq` as the last folded: dropped and counted.
    Replayed,
}

/// One device's side of the channel: its own head's rhythm and what it has folded from the others.
#[derive(Debug, Clone)]
pub struct Channel {
    device: String,
    table: HeadTable,
    /// This device's current channel root, once projected.
    root: Option<BlobCid>,
    /// How many flow events the current root was projected from, and the CID of the last.
    covers: u64,
    covers_last: Option<String>,
    /// When a change first waited to be announced.
    pending_since: Option<Instant>,
    last_announced: Option<Instant>,
    /// The highest of this device's seqs another admitted device has acknowledged folding. A
    /// device whose head table was lost resumes above it, so its next head is not refused as a
    /// rollback; no clock is compared.
    acked_floor: u64,
}

impl Channel {
    /// The channel of `device` (its did:key), resuming from a persisted `table`.
    pub fn new(device: String, table: HeadTable) -> Channel {
        Channel {
            device,
            table,
            root: None,
            covers: 0,
            covers_last: None,
            pending_since: None,
            last_announced: None,
            acked_floor: 0,
        }
    }

    pub fn table(&self) -> &HeadTable {
        &self.table
    }

    /// The persisted rules state for daemon reload bookkeeping.
    pub(crate) fn rules_state_mut(&mut self) -> &mut Option<crate::rules::RulesState> {
        &mut self.table.rules
    }

    /// Add refusals for `author`. True only when the projected nack set changed.
    pub fn nack(
        &mut self,
        author: &str,
        refused: impl IntoIterator<Item = String>,
        quarantined: impl IntoIterator<Item = String>,
    ) -> bool {
        let state = self.table.nacks.entry(author.to_string()).or_default();
        let before = state.clone();
        state.refused.extend(refused);
        state.quarantined.extend(quarantined);
        *state != before
    }

    /// The channel-health counters, for the daemon to record projection and fold failures in.
    pub fn counters_mut(&mut self) -> &mut ChannelCounters {
        &mut self.table.channel
    }

    /// The last folded `(seq, root)` from `device`.
    pub fn last_folded(&self, device: &str) -> Option<(u64, &BlobCid)> {
        self.table
            .devices
            .get(device)
            .map(|head| (head.seq, &head.root))
    }

    /// The nack leaf read from `device`'s last folded head. An unchanged root carries the same
    /// leaf, so a periodic head can advance its acknowledgement without reading the tier again.
    pub fn last_peer_nack(&self, device: &str) -> Option<&NackLeaf> {
        self.table.devices.get(device).map(|head| &head.nack)
    }

    /// Judge a head heard on the topic, counting every refusal in the table. The signature is
    /// checked before the order, so a forged head with a high `seq` counts as forged.
    pub fn judge(
        &mut self,
        head: &ChannelHead,
        admission: &Admission,
        verifier: &dyn SignatureVerifier,
    ) -> HeadVerdict {
        let verdict = if head.device == self.device {
            HeadVerdict::Own
        } else if !admission.admits_device(&head.device) {
            HeadVerdict::Unadmitted
        } else if head.verify(&head.device, verifier).is_err() {
            HeadVerdict::Forged
        } else {
            match self.last_folded(&head.device) {
                Some((last, _)) if head.seq < last => HeadVerdict::Rollback,
                Some((last, _)) if head.seq == last => HeadVerdict::Replayed,
                _ => HeadVerdict::Fold,
            }
        };
        if matches!(
            verdict,
            HeadVerdict::Fold | HeadVerdict::Rollback | HeadVerdict::Replayed
        ) {
            self.acknowledged(head);
        }
        let dropped = &mut self.table.dropped;
        match verdict {
            HeadVerdict::Unadmitted => dropped.unadmitted += 1,
            HeadVerdict::Forged => dropped.forged += 1,
            HeadVerdict::Rollback => dropped.rollback += 1,
            HeadVerdict::Replayed => dropped.replayed += 1,
            HeadVerdict::Fold | HeadVerdict::Own => {}
        }
        verdict
    }

    /// A verified head from another device acknowledges this device up to `folded[me]`. When that
    /// is at or past this device's next seq (its head table was lost), the next head goes above it,
    /// soon.
    fn acknowledged(&mut self, head: &ChannelHead) {
        let Some(&acked) = head.folded.get(&self.device) else {
            return;
        };
        if acked > self.own_seq() && acked > self.acked_floor {
            self.acked_floor = acked;
            self.schedule(Instant::now());
        }
    }

    fn own_seq(&self) -> u64 {
        self.table.own.as_ref().map_or(0, |own| own.seq)
    }

    /// Count a message on the topic that is not a head.
    pub fn malformed(&mut self) {
        self.table.dropped.malformed += 1;
    }

    /// Record that `head` (judged [`HeadVerdict::Fold`]) was folded, appending `appended` events,
    /// at `folded_at` (this device's clock, unix milliseconds). When its root moved, a
    /// re-announce is scheduled so the acknowledgement reaches its device.
    pub fn folded(
        &mut self,
        head: &ChannelHead,
        appended: u64,
        nack: NackLeaf,
        folded_at: u64,
        now: Instant,
    ) {
        let previous = self.table.devices.get(&head.device);
        let moved = previous.is_none_or(|previous| previous.root != head.root);
        let events_folded = previous.map_or(0, |previous| previous.events_folded) + appended;
        self.table.devices.insert(
            head.device.clone(),
            DeviceHead {
                seq: head.seq,
                root: head.root.clone(),
                at: head.at,
                folded: head.folded.clone(),
                folded_at,
                events_folded,
                nack,
            },
        );
        if moved {
            self.schedule(now);
        }
    }

    /// Forget the last head of every device `admitted` no longer admits, returning their
    /// did:keys: a contested device's seq is no longer recorded or carried in this device's
    /// heads. Called after a roster reload that read the roster (an unreadable roster admits
    /// nobody, and forgetting every device on it would cost every acknowledgement).
    pub fn forget_unadmitted(&mut self, admitted: impl Fn(&str) -> bool) -> Vec<String> {
        let forgotten: Vec<String> = self
            .table
            .devices
            .keys()
            .filter(|device| !admitted(device))
            .cloned()
            .collect();
        for device in &forgotten {
            self.table.devices.remove(device);
        }
        forgotten
    }

    /// This device's channel root is now `root`; a change is announced after the debounce.
    pub fn changed(&mut self, root: BlobCid, now: Instant) {
        if self.root.as_ref() != Some(&root) {
            self.root = Some(root);
            self.schedule(now);
        }
    }

    /// The current root was projected from the store's first `events` flow events, the last of
    /// them `last` (its CID). Recorded in [`HeadTable::own_log`] with the first head that
    /// announces each new root, so the lease view can tell which head first carried a claim, and
    /// over which store. Called after every projection, whether or not the root moved.
    pub fn covers(&mut self, events: u64, last: Option<String>) {
        self.covers = events;
        self.covers_last = last;
    }

    /// A neighbour joined the topic: it hears the current head within the debounce.
    pub fn neighbor_up(&mut self, now: Instant) {
        self.schedule(now);
    }

    fn schedule(&mut self, now: Instant) {
        self.pending_since.get_or_insert(now);
    }

    /// Whether a head is due now: a change has waited out the debounce, or the periodic
    /// re-announce has come round. Never before this device has a root.
    pub fn due(&self, now: Instant) -> bool {
        if self.root.is_none() {
            return false;
        }
        let debounced = self
            .pending_since
            .is_some_and(|since| now.saturating_duration_since(since) >= ANNOUNCE_DEBOUNCE);
        // Before the first announcement the change itself is pending, so only the debounce
        // applies.
        let periodic = self
            .last_announced
            .is_some_and(|last| now.saturating_duration_since(last) >= REANNOUNCE_EVERY);
        debounced || periodic
    }

    /// Sign this device's next head (`seq` one past the last announced, `folded` from every
    /// device folded so far) at `at` (unix milliseconds), and record it as announced. `None`
    /// before this device has a root.
    ///
    /// # Errors
    ///
    /// [`PeerError::HeadFrame`] when the head does not encode.
    pub fn announce(
        &mut self,
        now: Instant,
        at: u64,
        signer: &HeadSigner,
    ) -> Result<Option<ChannelHead>, PeerError> {
        let Some(root) = self.root.clone() else {
            return Ok(None);
        };
        // Saturating: an admitted device acknowledging u64::MAX cannot overflow this device's seq.
        let seq = self.own_seq().max(self.acked_floor).saturating_add(1);
        let folded: BTreeMap<String, u64> = self
            .table
            .devices
            .iter()
            .map(|(device, head)| (device.clone(), head.seq))
            .collect();
        let head = ChannelHead::sign(self.device.clone(), root, seq, at, folded, signer)?;
        // Only a new root is logged: the entry that first carried each event is kept, and the
        // periodic re-announce of an unchanged root never pushes it out.
        if self.table.own_log.last().map(|logged| &logged.root) != Some(&head.root) {
            self.table.own_log.push(OwnAnnounce {
                seq,
                root: head.root.clone(),
                at,
                events: self.covers,
                last: self.covers_last.clone(),
            });
            let excess = self.table.own_log.len().saturating_sub(OWN_LOG_LEN);
            self.table.own_log.drain(..excess);
        }
        self.table.own = Some(head.clone());
        self.pending_since = None;
        self.last_announced = Some(now);
        Ok(Some(head))
    }
}
