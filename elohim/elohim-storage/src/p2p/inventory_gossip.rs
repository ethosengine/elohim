//! Inventory gossip: signed delta envelopes on `elohim/inventory/blob/2`.
//!
//! A holder announces what it adds to and removes from its byte store as an
//! [`InventoryEnvelope`]: a delta body, the holder's key, the transport peer that
//! authored it, a level and an epoch, signed over
//!
//! ```text
//! b"elohim:inventory:v2:" ‖ u16be(epoch) ‖ u16be(scheme) ‖ key ‖ dag-cbor({body, epoch, level, holder, author_peer})
//! ```
//!
//! so every consequential field is inside the signature. Full-state convergence is
//! range reconciliation ([`crate::p2p::inventory_recon`]), not a snapshot: there is no
//! snapshot message and no unsigned message on this topic.
//!
//! ## Admission ([`admit`])
//!
//! 1. At most [`INVENTORY_PAGE_LIMIT`] bytes, else refused before decoding.
//! 2. An epoch or signature scheme this build does not know is **Held**: the bytes are
//!    retained opaque in a bounded table ([`HeldEnvelope`]), never projected into
//!    `peer_blob_inventory` and never counted as custody.
//! 3. The authenticated transport author must equal `author_peer`: on libp2p the
//!    gossipsub `message.source` (signed by the author's own key under gossipsub's
//!    `Signed` validation, NOT the `propagation_source` that relayed it); on iroh the
//!    envelope signature is the end-to-end authentication, because `delivered_from` is
//!    only the neighbour that relayed it.
//! 4. The holder key is the one independently expected for `author_peer`
//!    ([`HolderResolver`]: the key a libp2p PeerId or iroh NodeId embeds, admitted only
//!    when `peer_transport_manifest` binds the peer), and the envelope's signing key
//!    must be that key.
//! 5. The signature verifies.
//!
//! A replayed old delta (valid signature, old sequence) is a `Replay` at the sink and
//! never moves a cursor backwards; only an authenticated recon session re-bases one.

use elohim_epr::address::ByteCid;
use elohim_epr::proof::AgentKeypair;
use elohim_epr::signature::{Multikey, SignatureCheck, SignatureEnvelope, SignatureScheme};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Gossip topic for inventory envelopes (epoch 0 of the v2 wire).
pub const INVENTORY_TOPIC: &str = "elohim/inventory/blob/2";

/// The retired v1 topic. Peers still publishing here cannot be read; they are reported
/// unavailable, never as holding nothing. Bodies on it are never decoded.
pub const LEGACY_INVENTORY_TOPIC: &str = "elohim/inventory/blob";

/// Signature domain of an inventory envelope.
pub const INVENTORY_DOMAIN: &[u8] = b"elohim:inventory:v2:";

/// The envelope epoch this build reads and writes.
pub const INVENTORY_EPOCH: u16 = 0;

/// Largest encoded envelope accepted or emitted (C3). iroh-gossip caps a message at
/// 4096 encoded bytes; libp2p carries the same pages.
pub const INVENTORY_PAGE_LIMIT: usize = 4096;

/// How many held envelopes the in-memory table keeps (oldest evicted first).
pub const HELD_ENVELOPE_CAP: usize = 256;

/// A byte CID on the inventory wire, parsed by full decode on read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlobAddress(#[serde(with = "crate::blob_store::byte_cid_serde")] pub ByteCid);

impl BlobAddress {
    /// Parse wire text by full decode ([`ByteCid::parse`]); every other spelling fails.
    pub fn parse(text: &str) -> Result<Self, elohim_epr::address::AddressError> {
        ByteCid::parse(text).map(Self)
    }

    pub fn cid(&self) -> &ByteCid {
        &self.0
    }
}

impl From<ByteCid> for BlobAddress {
    fn from(cid: ByteCid) -> Self {
        Self(cid)
    }
}

impl std::fmt::Display for BlobAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Optional, advisory per-blob enrichment (never authority).
///
/// Field order is the dag-cbor canonical key order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobHint {
    /// Tiered-quilt storage class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    pub address: BlobAddress,
    /// `EprKind` entry-type of the blob's EPR.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epr_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// Collective CID of the advertising peer's dwelling hub.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_hub_id: Option<String>,
}

/// What an envelope's body claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InventoryLevel {
    /// `added`/`removed` are custody facts about the holder's own store.
    Full,
    /// Advisory hints only: `added` and `removed` are empty and nothing is projected
    /// as custody.
    Hints,
}

/// The delta body. Field order is the dag-cbor canonical key order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryDelta {
    pub added: Vec<BlobAddress>,
    pub hints: Vec<BlobHint>,
    pub removed: Vec<BlobAddress>,
    /// Per-holder monotonic sequence (restart epoch in the high bits; see
    /// `p2p::sync_state::split_sequence`).
    pub sequence: u64,
    /// Microseconds since the Unix epoch.
    pub emitted_at: i64,
}

/// A signed inventory announcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryEnvelope {
    pub sig: SignatureEnvelope,
    pub body: InventoryDelta,
    pub epoch: u16,
    pub level: InventoryLevel,
    #[serde(with = "multikey_bytes")]
    pub holder: Multikey,
    /// The transport peer (libp2p PeerId or iroh NodeId text) that authored it.
    pub author_peer: String,
}

/// The signed portion, in dag-cbor canonical key order.
#[derive(Serialize)]
struct SignedInventory<'a> {
    body: &'a InventoryDelta,
    epoch: u16,
    level: InventoryLevel,
    #[serde(with = "multikey_bytes")]
    holder: &'a Multikey,
    author_peer: &'a str,
}

mod multikey_bytes {
    use elohim_epr::signature::Multikey;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::borrow::Borrow;

    pub fn serialize<K: Borrow<Multikey>, S: Serializer>(
        key: &K,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(key.borrow().as_bytes())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Multikey, D::Error> {
        let bytes = serde_bytes::ByteBuf::deserialize(deserializer)?;
        Multikey::from_bytes(bytes.into_vec())
            .map_err(|error| serde::de::Error::custom(format!("holder multikey: {error:?}")))
    }
}

/// Just enough of an envelope to decide whether this build can read the rest.
#[derive(Deserialize)]
struct Header {
    epoch: u16,
    sig: SchemeHeader,
}

#[derive(Deserialize)]
struct SchemeHeader {
    scheme: u16,
}

impl InventoryEnvelope {
    /// The bytes the signature covers (after the domain/epoch/scheme/key prefix).
    pub fn signed_body(
        body: &InventoryDelta,
        epoch: u16,
        level: InventoryLevel,
        holder: &Multikey,
        author_peer: &str,
    ) -> Vec<u8> {
        let signed = SignedInventory {
            body,
            epoch,
            level,
            holder,
            author_peer,
        };
        // Plain data (strings, integers, byte strings, vectors): encoding cannot fail.
        serde_ipld_dagcbor::to_vec(&signed).unwrap_or_default()
    }

    /// Sign `body` as `author_peer` with `keypair` (whose public key is the holder).
    pub fn sign(
        keypair: &AgentKeypair,
        author_peer: &str,
        level: InventoryLevel,
        body: InventoryDelta,
    ) -> Self {
        let holder = Multikey::ed25519(&keypair.public_key_bytes());
        let signed = Self::signed_body(&body, INVENTORY_EPOCH, level, &holder, author_peer);
        let sig =
            SignatureEnvelope::sign_ed25519(keypair, INVENTORY_DOMAIN, INVENTORY_EPOCH, &signed);
        Self {
            sig,
            body,
            epoch: INVENTORY_EPOCH,
            level,
            holder,
            author_peer: author_peer.to_string(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Plain data: encoding cannot fail.
        serde_ipld_dagcbor::to_vec(self).unwrap_or_default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        serde_ipld_dagcbor::from_slice(bytes).map_err(|error| error.to_string())
    }

    /// The signature check against the key the receiver expects for the holder.
    pub fn check_for(&self, expected: &Multikey) -> SignatureCheck {
        let signed = Self::signed_body(
            &self.body,
            self.epoch,
            self.level,
            &self.holder,
            &self.author_peer,
        );
        self.sig
            .check_for(expected, INVENTORY_DOMAIN, self.epoch, &signed)
    }
}

/// Who the transport says authored a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorWitness<'a> {
    /// libp2p gossipsub `message.source` (validated under `Signed` mode); `None` when the
    /// message carried no source.
    Gossipsub(Option<&'a str>),
    /// iroh-gossip: no transport-level origin; the envelope signature over a holder key
    /// derived from `author_peer` is the end-to-end authentication.
    EndToEnd,
}

/// Resolves the holder key a receiver independently expects for a transport author.
pub trait HolderResolver {
    /// `Some(key)` when `author_peer` is bound in `peer_transport_manifest` and its key
    /// can be derived from the peer id itself; `None` otherwise.
    fn expected_holder(&self, author_peer: &str) -> Option<Multikey>;
}

/// Why an envelope was held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldReason {
    Epoch(u16),
    Scheme(u16),
}

/// Why an envelope was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    Oversize(usize),
    Malformed(String),
    /// `message.source` absent, or not `author_peer`.
    AuthorMismatch,
    /// `author_peer` has no bound, derivable key.
    UnboundAuthor,
    /// The envelope's holder is not the key expected for `author_peer`.
    HolderMismatch,
    BadSignature,
    /// A `full` delta with nothing in it, or a `hints` delta that claims custody.
    LevelShape,
}

impl RefuseReason {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Oversize(_) => "oversize",
            Self::Malformed(_) => "malformed",
            Self::AuthorMismatch => "author_mismatch",
            Self::UnboundAuthor => "unbound_author",
            Self::HolderMismatch => "holder_mismatch",
            Self::BadSignature => "bad_signature",
            Self::LevelShape => "level_shape",
        }
    }
}

/// An envelope this build cannot read, kept opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldEnvelope {
    pub topic: &'static str,
    pub reason: HeldReason,
    pub bytes: Vec<u8>,
}

/// The admission verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admission {
    Admitted(InventoryEnvelope),
    Held(HeldEnvelope),
    Refused(RefuseReason),
}

/// Judge one received envelope. See the module docs for the order of checks.
pub fn admit(raw: &[u8], witness: AuthorWitness<'_>, resolver: &dyn HolderResolver) -> Admission {
    if raw.len() > INVENTORY_PAGE_LIMIT {
        return Admission::Refused(RefuseReason::Oversize(raw.len()));
    }
    let header: Header = match serde_ipld_dagcbor::from_slice(raw) {
        Ok(header) => header,
        Err(error) => return Admission::Refused(RefuseReason::Malformed(error.to_string())),
    };
    let held = |reason| {
        Admission::Held(HeldEnvelope {
            topic: INVENTORY_TOPIC,
            reason,
            bytes: raw.to_vec(),
        })
    };
    if header.epoch != INVENTORY_EPOCH {
        return held(HeldReason::Epoch(header.epoch));
    }
    if SignatureScheme::from_code(header.sig.scheme).is_none() {
        return held(HeldReason::Scheme(header.sig.scheme));
    }
    let envelope = match InventoryEnvelope::from_bytes(raw) {
        Ok(envelope) => envelope,
        Err(error) => return Admission::Refused(RefuseReason::Malformed(error)),
    };
    if let AuthorWitness::Gossipsub(source) = witness {
        if source != Some(envelope.author_peer.as_str()) {
            return Admission::Refused(RefuseReason::AuthorMismatch);
        }
    }
    let Some(expected) = resolver.expected_holder(&envelope.author_peer) else {
        return Admission::Refused(RefuseReason::UnboundAuthor);
    };
    if envelope.holder != expected {
        return Admission::Refused(RefuseReason::HolderMismatch);
    }
    match envelope.check_for(&expected) {
        SignatureCheck::Verified => {}
        SignatureCheck::Invalid => return Admission::Refused(RefuseReason::BadSignature),
        SignatureCheck::Held { scheme } => return held(HeldReason::Scheme(scheme)),
    }
    let custody = !envelope.body.added.is_empty() || !envelope.body.removed.is_empty();
    let shape_ok = match envelope.level {
        InventoryLevel::Full => custody,
        InventoryLevel::Hints => !custody && !envelope.body.hints.is_empty(),
    };
    if !shape_ok {
        return Admission::Refused(RefuseReason::LevelShape);
    }
    Admission::Admitted(envelope)
}

/// What a cursor-keeping sink did with a delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaOutcome {
    Applied,
    /// At or below the cursor: dropped; the cursor does not move.
    Replay,
    /// Ahead of the cursor: not applied (stash it, or reconcile).
    Gap {
        expected: u64,
    },
}

/// Where admitted inventory goes. Production: `peer_blob_inventory` + `held_envelopes`.
pub trait InventorySink {
    /// Apply a `full` delta from `peer` under the cursor rules: a fresh peer's first
    /// delta is accepted, `sequence <= cursor` is a replay, `!= cursor + 1` a gap. A
    /// delta never moves a cursor backwards.
    fn apply_delta(&mut self, peer: &str, delta: &InventoryDelta) -> Result<DeltaOutcome, String>;
    /// Keep an envelope this build cannot read, bounded, opaque.
    fn retain_held(&mut self, held: HeldEnvelope);
    /// A peer is publishing where this build cannot read it.
    fn mark_unavailable(&mut self, peer: &str, reason: &'static str);
}

/// What [`receive`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Receipt {
    Applied {
        peer: String,
        envelope: InventoryEnvelope,
    },
    HintsOnly {
        peer: String,
        envelope: InventoryEnvelope,
    },
    Replay {
        peer: String,
    },
    Gap {
        peer: String,
        expected: u64,
        envelope: InventoryEnvelope,
    },
    Held(HeldReason),
    Refused(RefuseReason),
    Unavailable {
        peer: String,
    },
    SinkFailed(String),
}

/// Transport-neutral receive: route by topic, admit, project. `peer_hint` is the
/// transport's best identification of the sender, used only for the legacy topic.
pub fn receive(
    topic: &str,
    raw: &[u8],
    witness: AuthorWitness<'_>,
    peer_hint: &str,
    resolver: &dyn HolderResolver,
    sink: &mut dyn InventorySink,
) -> Receipt {
    if topic == LEGACY_INVENTORY_TOPIC {
        let peer = match witness {
            AuthorWitness::Gossipsub(Some(source)) => source,
            _ => peer_hint,
        };
        sink.mark_unavailable(peer, "legacy-inventory-topic");
        return Receipt::Unavailable {
            peer: peer.to_string(),
        };
    }
    match admit(raw, witness, resolver) {
        Admission::Refused(reason) => Receipt::Refused(reason),
        Admission::Held(held) => {
            let reason = held.reason;
            sink.retain_held(held);
            Receipt::Held(reason)
        }
        Admission::Admitted(envelope) => {
            let peer = envelope.author_peer.clone();
            if envelope.level == InventoryLevel::Hints {
                return Receipt::HintsOnly { peer, envelope };
            }
            match sink.apply_delta(&peer, &envelope.body) {
                Ok(DeltaOutcome::Applied) => Receipt::Applied { peer, envelope },
                Ok(DeltaOutcome::Replay) => Receipt::Replay { peer },
                Ok(DeltaOutcome::Gap { expected }) => Receipt::Gap {
                    peer,
                    expected,
                    envelope,
                },
                Err(error) => Receipt::SinkFailed(error),
            }
        }
    }
}

/// Cut an announcement into envelopes of at most [`INVENTORY_PAGE_LIMIT`] encoded bytes,
/// each signed with its own sequence from `next_sequence`. Every address is kept;
/// hints that would push a page over the limit are dropped first (they are advisory).
pub fn signed_pages(
    keypair: &AgentKeypair,
    author_peer: &str,
    added: &[BlobAddress],
    removed: &[BlobAddress],
    hints: &[BlobHint],
    emitted_at: i64,
    mut next_sequence: impl FnMut() -> u64,
) -> Vec<InventoryEnvelope> {
    let mut hints_by_address: HashMap<BlobAddress, Vec<BlobHint>> = HashMap::new();
    for hint in hints {
        hints_by_address
            .entry(hint.address)
            .or_default()
            .push(hint.clone());
    }
    let items: Vec<(BlobAddress, bool)> = added
        .iter()
        .map(|a| (*a, true))
        .chain(removed.iter().map(|r| (*r, false)))
        .collect();
    let mut pages = Vec::new();
    let mut page = InventoryDelta {
        added: Vec::new(),
        hints: Vec::new(),
        removed: Vec::new(),
        sequence: u64::MAX,
        emitted_at,
    };
    // A signature's encoded length is fixed by its scheme, so a probe with a zero
    // signature measures exactly what the signed page will.
    let holder = Multikey::ed25519(&keypair.public_key_bytes());
    let fits = |delta: &InventoryDelta| {
        let probe = InventoryEnvelope {
            sig: SignatureEnvelope::ed25519(&keypair.public_key_bytes(), [0u8; 64]),
            body: delta.clone(),
            epoch: INVENTORY_EPOCH,
            level: InventoryLevel::Full,
            holder: holder.clone(),
            author_peer: author_peer.to_string(),
        };
        probe.to_bytes().len() <= INVENTORY_PAGE_LIMIT
    };
    for (address, is_add) in items {
        let item_hints = if is_add {
            hints_by_address.remove(&address).unwrap_or_default()
        } else {
            Vec::new()
        };
        let mut trial = page.clone();
        push(&mut trial, address, is_add);
        trial.hints.extend(item_hints.iter().cloned());
        if fits(&trial) {
            page = trial;
            continue;
        }
        let mut bare = page.clone();
        push(&mut bare, address, is_add);
        if fits(&bare) {
            page = bare;
            continue;
        }
        if !page.added.is_empty() || !page.removed.is_empty() {
            pages.push(std::mem::replace(
                &mut page,
                InventoryDelta {
                    added: Vec::new(),
                    hints: Vec::new(),
                    removed: Vec::new(),
                    sequence: u64::MAX,
                    emitted_at,
                },
            ));
        }
        push(&mut page, address, is_add);
        let mut with_hints = page.clone();
        with_hints.hints.extend(item_hints);
        if fits(&with_hints) {
            page = with_hints;
        }
    }
    if !page.added.is_empty() || !page.removed.is_empty() {
        pages.push(page);
    }
    pages
        .into_iter()
        .map(|mut delta| {
            delta.sequence = next_sequence();
            InventoryEnvelope::sign(keypair, author_peer, InventoryLevel::Full, delta)
        })
        .collect()
}

fn push(delta: &mut InventoryDelta, address: BlobAddress, is_add: bool) {
    if is_add {
        delta.added.push(address);
    } else {
        delta.removed.push(address);
    }
}

/// Peers seen only on a topic this build cannot read, process-wide (one node per
/// process), keyed by the transport author. A diagnostics read reports them
/// `unavailable`, never as holding nothing.
static UNAVAILABLE_PEERS: std::sync::LazyLock<std::sync::Mutex<HashMap<String, &'static str>>> =
    std::sync::LazyLock::new(Default::default);

/// Most peers the unavailable registry remembers (a full registry ignores new ones).
pub const UNAVAILABLE_PEERS_CAP: usize = 4096;

/// Record that `peer` publishes where this build cannot read it.
pub fn mark_unavailable(peer: &str, reason: &'static str) {
    let mut peers = UNAVAILABLE_PEERS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if peers.len() < UNAVAILABLE_PEERS_CAP || peers.contains_key(peer) {
        peers.insert(peer.to_string(), reason);
    }
}

/// Forget a peer's unavailability once a readable envelope from it applies.
pub fn clear_unavailable(peer: &str) {
    UNAVAILABLE_PEERS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(peer);
}

/// Why `peer` is unavailable, if it is.
pub fn unavailable_reason(peer: &str) -> Option<&'static str> {
    UNAVAILABLE_PEERS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(peer)
        .copied()
}

/// An in-memory [`InventorySink`] with the production cursor rules: the reference the
/// DB-backed sink is tested against, and the store tests and diagnostics use.
#[derive(Debug, Default)]
pub struct MemoryInventory {
    pub held: VecDeque<HeldEnvelope>,
    sets: HashMap<String, std::collections::BTreeSet<ByteCid>>,
    cursors: HashMap<String, u64>,
    unavailable: HashMap<String, &'static str>,
}

/// What a receiver knows about a peer's inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryStatus {
    Known {
        count: usize,
    },
    /// The peer publishes where this build cannot read: unknown, NOT empty.
    Unavailable {
        reason: &'static str,
    },
    Unknown,
}

impl MemoryInventory {
    pub fn status(&self, peer: &str) -> InventoryStatus {
        if let Some(set) = self.sets.get(peer) {
            return InventoryStatus::Known { count: set.len() };
        }
        match self.unavailable.get(peer) {
            Some(reason) => InventoryStatus::Unavailable { reason },
            None => InventoryStatus::Unknown,
        }
    }

    /// Custody rows across all peers (what `custody.rs` would count).
    pub fn custody_count(&self) -> usize {
        self.sets.values().map(|set| set.len()).sum()
    }

    pub fn cursor(&self, peer: &str) -> Option<u64> {
        self.cursors.get(peer).copied()
    }

    /// Re-base a peer's cursor after an authenticated recon session with that peer.
    pub fn rebase_after_recon(&mut self, peer: &str, set: Vec<ByteCid>, sequence: u64) {
        self.sets
            .insert(peer.to_string(), set.into_iter().collect());
        self.cursors.insert(peer.to_string(), sequence);
        self.unavailable.remove(peer);
    }
}

impl InventorySink for MemoryInventory {
    fn apply_delta(&mut self, peer: &str, delta: &InventoryDelta) -> Result<DeltaOutcome, String> {
        match self.cursors.get(peer) {
            Some(&cursor) if delta.sequence <= cursor => return Ok(DeltaOutcome::Replay),
            Some(&cursor) if delta.sequence != cursor + 1 => {
                return Ok(DeltaOutcome::Gap {
                    expected: cursor + 1,
                })
            }
            _ => {}
        }
        let set = self.sets.entry(peer.to_string()).or_default();
        set.extend(delta.added.iter().map(|a| a.0));
        for removed in &delta.removed {
            set.remove(&removed.0);
        }
        self.cursors.insert(peer.to_string(), delta.sequence);
        self.unavailable.remove(peer);
        Ok(DeltaOutcome::Applied)
    }

    fn retain_held(&mut self, held: HeldEnvelope) {
        if self.held.len() == HELD_ENVELOPE_CAP {
            self.held.pop_front();
        }
        self.held.push_back(held);
    }

    fn mark_unavailable(&mut self, peer: &str, reason: &'static str) {
        if !self.sets.contains_key(peer) {
            self.unavailable.insert(peer.to_string(), reason);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use elohim_epr::address::compute_bytes_cid;

    pub(crate) fn keypair(seed: u8) -> AgentKeypair {
        AgentKeypair::from_secret(&[seed; 32]).unwrap()
    }

    pub(crate) fn addr(n: u32) -> BlobAddress {
        BlobAddress(compute_bytes_cid(&n.to_le_bytes()))
    }

    /// Binds author peer names to keys, as the transport manifest would.
    pub(crate) struct Bound(pub HashMap<String, Multikey>);

    impl HolderResolver for Bound {
        fn expected_holder(&self, author_peer: &str) -> Option<Multikey> {
            self.0.get(author_peer).cloned()
        }
    }

    fn bound(peer: &str, kp: &AgentKeypair) -> Bound {
        Bound(HashMap::from([(
            peer.to_string(),
            Multikey::ed25519(&kp.public_key_bytes()),
        )]))
    }

    fn delta(sequence: u64, added: Vec<BlobAddress>) -> InventoryDelta {
        InventoryDelta {
            added,
            hints: vec![],
            removed: vec![],
            sequence,
            emitted_at: 1_700_000_000_000_000,
        }
    }

    fn signed(kp: &AgentKeypair, author: &str, seq: u64) -> InventoryEnvelope {
        InventoryEnvelope::sign(
            kp,
            author,
            InventoryLevel::Full,
            delta(seq, vec![addr(seq as u32)]),
        )
    }

    #[test]
    fn envelope_round_trips_and_verifies() {
        let kp = keypair(7);
        let env = signed(&kp, "peerA", 1);
        let bytes = env.to_bytes();
        assert!(bytes.len() <= INVENTORY_PAGE_LIMIT);
        assert_eq!(InventoryEnvelope::from_bytes(&bytes).unwrap(), env);
        assert!(matches!(
            admit(
                &bytes,
                AuthorWitness::Gossipsub(Some("peerA")),
                &bound("peerA", &kp)
            ),
            Admission::Admitted(_)
        ));
    }

    #[test]
    fn envelope_with_unknown_scheme_is_held_not_dropped_and_changes_no_count() {
        let kp = keypair(7);
        let resolver = bound("peerA", &kp);
        let mut sink = MemoryInventory::default();
        let mut env = signed(&kp, "peerA", 1);
        env.sig.scheme = 9;
        let raw = env.to_bytes();
        let receipt = receive(
            INVENTORY_TOPIC,
            &raw,
            AuthorWitness::Gossipsub(Some("peerA")),
            "relay",
            &resolver,
            &mut sink,
        );
        assert_eq!(receipt, Receipt::Held(HeldReason::Scheme(9)));
        assert_eq!(sink.held.len(), 1, "retained, not dropped");
        assert_eq!(sink.held[0].bytes, raw, "kept opaque, byte for byte");
        assert_eq!(sink.custody_count(), 0, "never projected as custody");
        assert_eq!(sink.cursor("peerA"), None, "no cursor moved");
        assert_eq!(sink.status("peerA"), InventoryStatus::Unknown);

        // An unknown envelope epoch is held the same way, before its body is decoded.
        let mut future = serde_ipld_dagcbor::to_vec(&serde_json::json!({
            "epoch": 1, "sig": {"scheme": 0}, "body": "a shape this build has never seen"
        }))
        .unwrap();
        let receipt = receive(
            INVENTORY_TOPIC,
            &future,
            AuthorWitness::Gossipsub(Some("peerA")),
            "relay",
            &resolver,
            &mut sink,
        );
        assert_eq!(receipt, Receipt::Held(HeldReason::Epoch(1)));
        assert_eq!(sink.held.len(), 2);
        assert_eq!(sink.custody_count(), 0);

        // The table is bounded.
        future.push(0);
        for _ in 0..HELD_ENVELOPE_CAP + 10 {
            sink.retain_held(HeldEnvelope {
                topic: INVENTORY_TOPIC,
                reason: HeldReason::Epoch(1),
                bytes: future.clone(),
            });
        }
        assert_eq!(sink.held.len(), HELD_ENVELOPE_CAP);
    }

    #[test]
    fn envelope_author_mismatch_is_refused() {
        let kp = keypair(7);
        let resolver = bound("peerA", &kp);
        let raw = signed(&kp, "peerA", 1).to_bytes();
        // The relay that propagated it is not its author.
        assert_eq!(
            admit(&raw, AuthorWitness::Gossipsub(Some("peerB")), &resolver),
            Admission::Refused(RefuseReason::AuthorMismatch)
        );
        // A gossipsub message with no source has no authenticated author.
        assert_eq!(
            admit(&raw, AuthorWitness::Gossipsub(None), &resolver),
            Admission::Refused(RefuseReason::AuthorMismatch)
        );
    }

    #[test]
    fn envelope_holder_key_must_match_expected_signer() {
        let honest = keypair(7);
        let forger = keypair(8);
        let resolver = bound("peerA", &honest);
        // A forger signs a perfectly valid envelope under its own key, claiming peerA.
        let forged = signed(&forger, "peerA", 1).to_bytes();
        assert_eq!(
            admit(&forged, AuthorWitness::Gossipsub(Some("peerA")), &resolver),
            Admission::Refused(RefuseReason::HolderMismatch)
        );
        // The forger names the honest holder but signs with its own key.
        let mut swapped = signed(&forger, "peerA", 1);
        swapped.holder = Multikey::ed25519(&honest.public_key_bytes());
        assert_eq!(
            admit(
                &swapped.to_bytes(),
                AuthorWitness::Gossipsub(Some("peerA")),
                &resolver
            ),
            Admission::Refused(RefuseReason::BadSignature)
        );
        // Any field changed after signing breaks the signature.
        let mut edited = signed(&honest, "peerA", 1);
        edited.body.added.push(addr(99));
        assert_eq!(
            admit(
                &edited.to_bytes(),
                AuthorWitness::Gossipsub(Some("peerA")),
                &resolver
            ),
            Admission::Refused(RefuseReason::BadSignature)
        );
        // An author the manifest does not bind is not admitted.
        let unbound = signed(&honest, "peerZ", 1).to_bytes();
        assert_eq!(
            admit(&unbound, AuthorWitness::Gossipsub(Some("peerZ")), &resolver),
            Admission::Refused(RefuseReason::UnboundAuthor)
        );
    }

    #[test]
    fn replayed_old_delta_does_not_rebase_cursor() {
        let kp = keypair(7);
        let resolver = bound("peerA", &kp);
        let mut sink = MemoryInventory::default();
        let witness = AuthorWitness::Gossipsub(Some("peerA"));
        let first = signed(&kp, "peerA", 10).to_bytes();
        for seq in 10..=12 {
            let raw = signed(&kp, "peerA", seq).to_bytes();
            assert!(matches!(
                receive(INVENTORY_TOPIC, &raw, witness, "x", &resolver, &mut sink),
                Receipt::Applied { .. }
            ));
        }
        assert_eq!(sink.cursor("peerA"), Some(12));
        // The genuine, validly signed delta 10 replayed later.
        assert_eq!(
            receive(INVENTORY_TOPIC, &first, witness, "x", &resolver, &mut sink),
            Receipt::Replay {
                peer: "peerA".into()
            }
        );
        assert_eq!(
            sink.cursor("peerA"),
            Some(12),
            "the cursor never moves backwards"
        );
        assert_eq!(sink.status("peerA"), InventoryStatus::Known { count: 3 });
        // Only an authenticated recon session re-bases.
        sink.rebase_after_recon("peerA", vec![addr(1).0], 3);
        assert_eq!(sink.cursor("peerA"), Some(3));
    }

    #[test]
    fn relayed_delta_still_applies() {
        let kp = keypair(7);
        let resolver = bound("peerA", &kp);
        let mut sink = MemoryInventory::default();
        let raw = signed(&kp, "peerA", 1).to_bytes();
        // libp2p: relayed by peerC, but message.source is the author.
        assert!(matches!(
            receive(
                INVENTORY_TOPIC,
                &raw,
                AuthorWitness::Gossipsub(Some("peerA")),
                "peerC",
                &resolver,
                &mut sink
            ),
            Receipt::Applied { .. }
        ));
        // iroh: delivered by a neighbour; the signature authenticates end to end.
        let raw = signed(&kp, "peerA", 2).to_bytes();
        assert!(matches!(
            receive(
                INVENTORY_TOPIC,
                &raw,
                AuthorWitness::EndToEnd,
                "nodeC",
                &resolver,
                &mut sink
            ),
            Receipt::Applied { .. }
        ));
        assert_eq!(sink.status("peerA"), InventoryStatus::Known { count: 2 });
    }

    #[test]
    fn old_topic_peers_are_reported_unavailable_not_empty() {
        let resolver = Bound(HashMap::new());
        let mut sink = MemoryInventory::default();
        let receipt = receive(
            LEGACY_INVENTORY_TOPIC,
            b"\x85\xa7peer_id\xa5peerL",
            AuthorWitness::Gossipsub(Some("peerL")),
            "peerL",
            &resolver,
            &mut sink,
        );
        assert_eq!(
            receipt,
            Receipt::Unavailable {
                peer: "peerL".into()
            }
        );
        assert_eq!(
            sink.status("peerL"),
            InventoryStatus::Unavailable {
                reason: "legacy-inventory-topic"
            }
        );
        assert_ne!(sink.status("peerL"), InventoryStatus::Known { count: 0 });
        assert_eq!(sink.custody_count(), 0);
    }

    #[test]
    fn levels_bind_their_shape() {
        let kp = keypair(7);
        let resolver = bound("peerA", &kp);
        let w = AuthorWitness::Gossipsub(Some("peerA"));
        let empty_full =
            InventoryEnvelope::sign(&kp, "peerA", InventoryLevel::Full, delta(1, vec![]));
        assert_eq!(
            admit(&empty_full.to_bytes(), w, &resolver),
            Admission::Refused(RefuseReason::LevelShape)
        );
        let custody_hints =
            InventoryEnvelope::sign(&kp, "peerA", InventoryLevel::Hints, delta(1, vec![addr(1)]));
        assert_eq!(
            admit(&custody_hints.to_bytes(), w, &resolver),
            Admission::Refused(RefuseReason::LevelShape)
        );
        let mut hints = delta(1, vec![]);
        hints.hints.push(BlobHint {
            tier: None,
            address: addr(1),
            epr_kind: Some("Content".into()),
            size_bytes: Some(4),
            recipient_hub_id: None,
        });
        let hints_only = InventoryEnvelope::sign(&kp, "peerA", InventoryLevel::Hints, hints);
        let mut sink = MemoryInventory::default();
        assert!(matches!(
            receive(
                INVENTORY_TOPIC,
                &hints_only.to_bytes(),
                w,
                "x",
                &resolver,
                &mut sink
            ),
            Receipt::HintsOnly { .. }
        ));
        assert_eq!(sink.custody_count(), 0);
    }

    #[test]
    fn oversize_and_sha256_spellings_are_refused() {
        let kp = keypair(7);
        let resolver = bound("peerA", &kp);
        let w = AuthorWitness::Gossipsub(Some("peerA"));
        assert!(matches!(
            admit(&vec![0u8; INVENTORY_PAGE_LIMIT + 1], w, &resolver),
            Admission::Refused(RefuseReason::Oversize(_))
        ));
        // Rewrite one address to a sha256 spelling: the body no longer decodes.
        let env = signed(&kp, "peerA", 1);
        let tampered = env
            .to_bytes()
            .windows(59)
            .position(|w| w == addr(1).to_string().as_bytes())
            .map(|at| {
                let mut bytes = env.to_bytes();
                let sha = format!("sha256-{}", "a".repeat(52));
                bytes[at..at + 59].copy_from_slice(sha.as_bytes());
                bytes
            })
            .unwrap();
        assert!(matches!(
            admit(&tampered, w, &resolver),
            Admission::Refused(RefuseReason::Malformed(_))
        ));
    }

    #[test]
    fn signed_pages_stay_under_the_page_limit_and_keep_every_address() {
        let kp = keypair(7);
        let added: Vec<BlobAddress> = (0..500).map(addr).collect();
        let removed: Vec<BlobAddress> = (500..520).map(addr).collect();
        let hints: Vec<BlobHint> = added
            .iter()
            .map(|a| BlobHint {
                tier: Some("stocked".into()),
                address: *a,
                epr_kind: Some("Content".into()),
                size_bytes: Some(1024),
                recipient_hub_id: Some("bafyreihub".into()),
            })
            .collect();
        let mut seq = 100u64;
        let pages = signed_pages(&kp, "peerA", &added, &removed, &hints, 1, || {
            seq += 1;
            seq
        });
        assert!(pages.len() > 1);
        let resolver = bound("peerA", &kp);
        let mut sink = MemoryInventory::default();
        for page in &pages {
            let bytes = page.to_bytes();
            assert!(bytes.len() <= INVENTORY_PAGE_LIMIT, "{} bytes", bytes.len());
            assert!(matches!(
                receive(
                    INVENTORY_TOPIC,
                    &bytes,
                    AuthorWitness::Gossipsub(Some("peerA")),
                    "x",
                    &resolver,
                    &mut sink
                ),
                Receipt::Applied { .. }
            ));
        }
        let all_added: usize = pages.iter().map(|p| p.body.added.len()).sum();
        let all_removed: usize = pages.iter().map(|p| p.body.removed.len()).sum();
        assert_eq!((all_added, all_removed), (500, 20));
        let seqs: Vec<u64> = pages.iter().map(|p| p.body.sequence).collect();
        assert_eq!(seqs, (101..101 + pages.len() as u64).collect::<Vec<_>>());
    }
}
