//! Inventory range reconciliation: `negentropy-v1`, in-crate.
//!
//! Two peers converge on the difference between their byte-CID sets in a logarithmic
//! number of round trips, each message at most [`RECON_PAGE_LIMIT`] bytes. It replaces
//! the periodic full-inventory snapshot: a receiver learns a holder's whole set by
//! reconciling against its own view of that holder, over an authenticated
//! request-response stream ([`RECON_PROTOCOL`]) to the holder itself.
//!
//! ## Wire (declared as `recon: "negentropy-v1"`)
//!
//! Items are the sorted 36-byte binary forms of byte CIDs (`01 55 1e 20 ‖ digest`).
//! A message is a version byte `0x61` followed by ranges; each range is
//! `bound ‖ varint(mode) ‖ payload`, its lower bound the previous range's upper bound
//! (the first starts at the minimum):
//!
//! - `bound`: `varint(0)` is infinity; `varint(n)` (1..=36) is followed by an `n`-byte
//!   prefix. An item is below the bound iff its first `n` bytes sort before the prefix.
//! - mode `0` Skip (no payload); mode `1` Fingerprint (16 bytes); mode `2` IdList
//!   (`varint(count)` then `count` 36-byte items).
//!
//! The fingerprint of a range is the first 16 bytes of
//! `sha256(Σ items mod 2^256 ‖ varint(count))`, each item read as a little-endian
//! integer. Unlike upstream negentropy there is no timestamp field: items are
//! content addresses and carry no order but their bytes.

use elohim_epr::address::ByteCid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeSet;

/// The request-response protocol that carries recon messages.
pub const RECON_PROTOCOL: &str = "/elohim/inventory-recon/1.0.0";

/// The reconciliation algorithm id every request declares.
pub const RECON_ID: &str = "negentropy-v1";

/// Largest recon message (C3).
pub const RECON_PAGE_LIMIT: usize = 4096;

const VERSION: u8 = 0x61;
const ID_LEN: usize = 36;
const BUCKETS: usize = 16;
const FINGERPRINT_LEN: usize = 16;
/// Room kept for the closing remainder fingerprint (bound ≤ 38 + mode + 16).
const CLOSING_ROOM: usize = 64;

type Id = [u8; ID_LEN];

/// Why a recon message was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReconError {
    #[error("recon message over {RECON_PAGE_LIMIT} bytes")]
    Oversize,
    #[error("recon protocol version {0:#x} unsupported")]
    Version(u8),
    #[error("recon message truncated or malformed")]
    Malformed,
    #[error("recon item is not a byte cid")]
    NotByteCid,
    #[error("recon algorithm {0} unsupported")]
    Algorithm(String),
}

/// A request on [`RECON_PROTOCOL`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconRequest {
    /// Must be [`RECON_ID`].
    pub recon: String,
    #[serde(with = "serde_bytes")]
    pub msg: Vec<u8>,
}

/// A response on [`RECON_PROTOCOL`]. `sequence` is the responder's inventory sequence at
/// the time it answered: a receiver that completes a session re-bases its cursor for the
/// responder to it (the only cursor re-base there is).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconResponse {
    #[serde(with = "serde_bytes")]
    pub msg: Vec<u8>,
    pub sequence: u64,
}

impl ReconRequest {
    pub fn new(msg: Vec<u8>) -> Self {
        Self {
            recon: RECON_ID.to_string(),
            msg,
        }
    }

    /// Check the declared algorithm and size before acting on it.
    pub fn admit(&self) -> Result<(), ReconError> {
        if self.recon != RECON_ID {
            return Err(ReconError::Algorithm(self.recon.clone()));
        }
        if self.msg.len() > RECON_PAGE_LIMIT {
            return Err(ReconError::Oversize);
        }
        Ok(())
    }
}

/// One side's sorted, de-duplicated items.
#[derive(Debug, Clone, Default)]
pub struct ReconSet {
    ids: Vec<Id>,
}

impl ReconSet {
    pub fn new<'a>(cids: impl IntoIterator<Item = &'a ByteCid>) -> Self {
        let set: BTreeSet<Id> = cids.into_iter().map(id_of).collect();
        Self {
            ids: set.into_iter().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// First index whose item is not below `bound`.
    fn lower_index(&self, bound: &Bound) -> usize {
        self.ids.partition_point(|id| bound.above(id))
    }

    fn fingerprint(&self, from: usize, to: usize) -> [u8; FINGERPRINT_LEN] {
        fingerprint(&self.ids[from..to])
    }
}

fn id_of(cid: &ByteCid) -> Id {
    let bytes = cid.to_bytes();
    let mut id = [0u8; ID_LEN];
    id.copy_from_slice(&bytes[..ID_LEN]);
    id
}

fn cid_of(id: &Id) -> Result<ByteCid, ReconError> {
    let cid = cid::Cid::try_from(&id[..]).map_err(|_| ReconError::NotByteCid)?;
    ByteCid::try_from(cid).map_err(|_| ReconError::NotByteCid)
}

/// `sha256(Σ ids mod 2^256 ‖ varint(count))[..16]`.
fn fingerprint(ids: &[Id]) -> [u8; FINGERPRINT_LEN] {
    let mut sum = [0u8; 32];
    for id in ids {
        let mut carry = 0u16;
        for (acc, byte) in sum.iter_mut().zip(&id[..32]) {
            let total = u16::from(*acc) + u16::from(*byte) + carry;
            *acc = (total & 0xff) as u8;
            carry = total >> 8;
        }
    }
    let mut hasher = Sha256::new();
    hasher.update(sum);
    let mut count = Vec::new();
    put_varint(&mut count, ids.len() as u64);
    hasher.update(count);
    let digest = hasher.finalize();
    let mut out = [0u8; FINGERPRINT_LEN];
    out.copy_from_slice(&digest[..FINGERPRINT_LEN]);
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Bound {
    Prefix(Vec<u8>),
    Infinity,
}

impl Bound {
    /// Is `id` below this bound?
    fn above(&self, id: &Id) -> bool {
        match self {
            Bound::Infinity => true,
            Bound::Prefix(prefix) => id[..prefix.len()].cmp(prefix) == Ordering::Less,
        }
    }

    /// The shortest bound that puts `prev` below and `next` at or above it.
    fn between(prev: Option<&Id>, next: &Id) -> Bound {
        let shared = prev.map_or(0, |prev| {
            prev.iter().zip(next).take_while(|(a, b)| a == b).count()
        });
        Bound::Prefix(next[..(shared + 1).min(ID_LEN)].to_vec())
    }

    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Bound::Infinity => put_varint(out, 0),
            Bound::Prefix(prefix) => {
                put_varint(out, prefix.len() as u64);
                out.extend_from_slice(prefix);
            }
        }
    }
}

const MODE_SKIP: u64 = 0;
const MODE_FINGERPRINT: u64 = 1;
const MODE_ID_LIST: u64 = 2;

fn put_varint(out: &mut Vec<u8>, mut value: u64) {
    // bounded-work: at most 10 iterations (64 bits, 7 per byte).
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn done(&self) -> bool {
        self.at >= self.bytes.len()
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], ReconError> {
        let end = self.at.checked_add(n).ok_or(ReconError::Malformed)?;
        let slice = self.bytes.get(self.at..end).ok_or(ReconError::Malformed)?;
        self.at = end;
        Ok(slice)
    }

    fn varint(&mut self) -> Result<u64, ReconError> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = self.take(1)?[0];
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ReconError::Malformed)
    }

    fn bound(&mut self) -> Result<Bound, ReconError> {
        match self.varint()? {
            0 => Ok(Bound::Infinity),
            n if n as usize <= ID_LEN => Ok(Bound::Prefix(self.take(n as usize)?.to_vec())),
            _ => Err(ReconError::Malformed),
        }
    }
}

/// Builds one outgoing message, coalescing skips and respecting the page limit.
struct Writer {
    out: Vec<u8>,
    pending_skip: Option<Bound>,
    meaningful: bool,
}

impl Writer {
    fn new() -> Self {
        Self {
            out: vec![VERSION],
            pending_skip: None,
            meaningful: false,
        }
    }

    fn skip(&mut self, upper: Bound) {
        self.pending_skip = Some(upper);
    }

    fn flush_skip(&mut self) {
        if let Some(bound) = self.pending_skip.take() {
            bound.encode(&mut self.out);
            put_varint(&mut self.out, MODE_SKIP);
        }
    }

    fn range(&mut self, piece: &[u8]) {
        self.flush_skip();
        self.out.extend_from_slice(piece);
        self.meaningful = true;
    }

    fn room_for(&self, piece: &[u8]) -> bool {
        let skip = self.pending_skip.as_ref().map_or(0, |_| ID_LEN + 2);
        self.out.len() + skip + piece.len() + CLOSING_ROOM <= RECON_PAGE_LIMIT
    }

    /// The finished message; a trailing skip is implied and omitted.
    fn finish(self) -> (Vec<u8>, bool) {
        (self.out, self.meaningful)
    }
}

fn fingerprint_piece(upper: &Bound, fp: &[u8; FINGERPRINT_LEN]) -> Vec<u8> {
    let mut piece = Vec::new();
    upper.encode(&mut piece);
    put_varint(&mut piece, MODE_FINGERPRINT);
    piece.extend_from_slice(fp);
    piece
}

fn id_list_piece(upper: &Bound, ids: &[Id]) -> Vec<u8> {
    let mut piece = Vec::new();
    upper.encode(&mut piece);
    put_varint(&mut piece, MODE_ID_LIST);
    put_varint(&mut piece, ids.len() as u64);
    for id in ids {
        piece.extend_from_slice(id);
    }
    piece
}

/// One side of a reconciliation.
pub struct Reconciler<'a> {
    set: &'a ReconSet,
    initiator: bool,
}

/// What a reconciliation step produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Step {
    /// The next message to send; `None` when the initiator is done.
    pub next: Option<Vec<u8>>,
    /// Items this side holds and the other lacks (initiator only).
    pub have: Vec<ByteCid>,
    /// Items the other side holds and this side lacks (initiator only).
    pub need: Vec<ByteCid>,
}

impl<'a> Reconciler<'a> {
    pub fn initiator(set: &'a ReconSet) -> Self {
        Self {
            set,
            initiator: true,
        }
    }

    pub fn responder(set: &'a ReconSet) -> Self {
        Self {
            set,
            initiator: false,
        }
    }

    /// The opening message: the whole range, split as the responder would split it.
    pub fn initiate(&self) -> Vec<u8> {
        let mut writer = Writer::new();
        self.split(&mut writer, 0, self.set.len(), &Bound::Infinity);
        writer.finish().0
    }

    /// Answer one received message.
    pub fn reconcile(&self, msg: &[u8]) -> Result<Step, ReconError> {
        if msg.len() > RECON_PAGE_LIMIT {
            return Err(ReconError::Oversize);
        }
        let mut reader = Reader { bytes: msg, at: 0 };
        let version = reader.take(1)?[0];
        if version != VERSION {
            return Err(ReconError::Version(version));
        }
        let mut writer = Writer::new();
        let mut step = Step::default();
        let mut lower = 0usize;
        // bounded-work: every range consumes at least two bytes of a message capped at
        // RECON_PAGE_LIMIT.
        while !reader.done() {
            let upper_bound = reader.bound()?;
            let upper = self.set.lower_index(&upper_bound);
            if upper < lower {
                return Err(ReconError::Malformed);
            }
            let mode = reader.varint()?;
            let mut piece_out = Writer::new();
            match mode {
                MODE_SKIP => piece_out.skip(upper_bound.clone()),
                MODE_FINGERPRINT => {
                    let theirs = reader.take(FINGERPRINT_LEN)?;
                    if self.set.fingerprint(lower, upper)[..] == theirs[..] {
                        piece_out.skip(upper_bound.clone());
                    } else {
                        self.split(&mut piece_out, lower, upper, &upper_bound);
                    }
                }
                MODE_ID_LIST => {
                    let count = reader.varint()? as usize;
                    if count > RECON_PAGE_LIMIT / ID_LEN {
                        return Err(ReconError::Malformed);
                    }
                    let mut theirs = BTreeSet::new();
                    for _ in 0..count {
                        let mut id = [0u8; ID_LEN];
                        id.copy_from_slice(reader.take(ID_LEN)?);
                        theirs.insert(id);
                    }
                    let ours = &self.set.ids[lower..upper];
                    if self.initiator {
                        for id in ours {
                            if !theirs.contains(id) {
                                step.have.push(cid_of(id)?);
                            }
                        }
                        for id in &theirs {
                            if ours.binary_search(id).is_err() {
                                step.need.push(cid_of(id)?);
                            }
                        }
                        piece_out.skip(upper_bound.clone());
                    } else {
                        // Answer with our side of the range: an id list when it is small,
                        // fingerprinted buckets otherwise (a large list would not fit a
                        // page, and the initiator splits again from its fingerprints).
                        self.split(&mut piece_out, lower, upper, &upper_bound);
                    }
                }
                _ => return Err(ReconError::Malformed),
            }
            let (piece, meaningful) = piece_out.finish();
            let piece = &piece[1..];
            if meaningful {
                if !writer.room_for(piece) {
                    // Out of room: everything from this range on is one fingerprint.
                    let fp = self.set.fingerprint(lower, self.set.len());
                    writer.range(&fingerprint_piece(&Bound::Infinity, &fp));
                    return Ok(self.finish(writer, step));
                }
                writer.range(piece);
            } else {
                writer.skip(upper_bound.clone());
            }
            lower = upper;
        }
        Ok(self.finish(writer, step))
    }

    fn finish(&self, writer: Writer, mut step: Step) -> Step {
        let (out, meaningful) = writer.finish();
        step.next = if meaningful || !self.initiator {
            Some(out)
        } else {
            None
        };
        step
    }

    /// Describe `[from, to)` ending at `upper`: an id list when small, else
    /// [`BUCKETS`] fingerprinted sub-ranges.
    fn split(&self, writer: &mut Writer, from: usize, to: usize, upper: &Bound) {
        let ids = &self.set.ids[from..to];
        if ids.len() < 2 * BUCKETS {
            writer.range(&id_list_piece(upper, ids));
            return;
        }
        let per = ids.len() / BUCKETS;
        let extra = ids.len() % BUCKETS;
        let mut start = 0usize;
        for bucket in 0..BUCKETS {
            let size = per + usize::from(bucket < extra);
            let end = start + size;
            let bound = if bucket + 1 == BUCKETS {
                upper.clone()
            } else {
                Bound::between(Some(&ids[end - 1]), &ids[end])
            };
            writer.range(&fingerprint_piece(&bound, &fingerprint(&ids[start..end])));
            start = end;
        }
    }
}

/// Round trips after which a session is abandoned as non-converging.
pub const RECON_MAX_ROUNDS: usize = 4096;

/// Run a whole session between two in-process sets (tests, and the shape a transport
/// driver follows): returns `(have, need, round_trips, largest_message)`.
pub fn reconcile_in_process(
    initiator: &ReconSet,
    responder: &ReconSet,
) -> Result<(Vec<ByteCid>, Vec<ByteCid>, usize, usize), ReconError> {
    let a = Reconciler::initiator(initiator);
    let b = Reconciler::responder(responder);
    let mut msg = a.initiate();
    let mut largest = msg.len();
    let mut have = Vec::new();
    let mut need = Vec::new();
    let mut rounds = 0usize;
    // bounded-work: RECON_MAX_ROUNDS round trips of at most RECON_PAGE_LIMIT bytes each.
    loop {
        rounds += 1;
        if rounds > RECON_MAX_ROUNDS {
            return Err(ReconError::Malformed);
        }
        let reply = b.reconcile(&msg)?.next.unwrap_or_else(|| vec![VERSION]);
        largest = largest.max(reply.len());
        let step = a.reconcile(&reply)?;
        have.extend(step.have);
        need.extend(step.need);
        match step.next {
            Some(next) => {
                largest = largest.max(next.len());
                msg = next;
            }
            None => return Ok((have, need, rounds, largest)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr::address::compute_bytes_cid;

    fn cids(range: std::ops::Range<u32>) -> Vec<ByteCid> {
        range.map(|n| compute_bytes_cid(&n.to_le_bytes())).collect()
    }

    fn sorted(mut v: Vec<ByteCid>) -> Vec<ByteCid> {
        v.sort();
        v
    }

    /// Two sets sharing `n` items, with `only_a` and `only_b` extra items each.
    fn sets(
        n: u32,
        only_a: u32,
        only_b: u32,
    ) -> (Vec<ByteCid>, Vec<ByteCid>, Vec<ByteCid>, Vec<ByteCid>) {
        let shared = cids(0..n);
        let a_extra = cids(1_000_000..1_000_000 + only_a);
        let b_extra = cids(2_000_000..2_000_000 + only_b);
        let a: Vec<ByteCid> = shared.iter().chain(&a_extra).copied().collect();
        let b: Vec<ByteCid> = shared.iter().chain(&b_extra).copied().collect();
        (a, b, a_extra, b_extra)
    }

    #[test]
    fn recon_converges_in_log_rounds() {
        for n in [1_000u32, 10_000, 100_000] {
            let (a, b, a_extra, b_extra) = sets(n, 6, 7);
            let (have, need, rounds, largest) =
                reconcile_in_process(&ReconSet::new(&a), &ReconSet::new(&b)).unwrap();
            assert_eq!(sorted(have), sorted(a_extra), "n={n}");
            assert_eq!(sorted(need), sorted(b_extra), "n={n}");
            assert!(largest <= RECON_PAGE_LIMIT, "n={n}: {largest} byte message");
            // log16(n) rounds of splitting, plus the id-list exchange.
            let log16 = ((n as f64).ln() / 16f64.ln()).ceil() as usize;
            assert!(rounds <= log16 + 2, "n={n}: {rounds} rounds, log16={log16}");
        }
    }

    #[test]
    fn identical_sets_finish_in_one_round_trip() {
        let a = cids(0..50_000);
        let (have, need, rounds, _) =
            reconcile_in_process(&ReconSet::new(&a), &ReconSet::new(&a)).unwrap();
        assert!(have.is_empty() && need.is_empty());
        assert_eq!(rounds, 1);
    }

    #[test]
    fn many_differences_stay_paged_and_exact() {
        let (a, b, a_extra, b_extra) = sets(20_000, 900, 1_100);
        let (have, need, _rounds, largest) =
            reconcile_in_process(&ReconSet::new(&a), &ReconSet::new(&b)).unwrap();
        assert!(largest <= RECON_PAGE_LIMIT);
        assert_eq!(sorted(have), sorted(a_extra));
        assert_eq!(sorted(need), sorted(b_extra));
    }

    #[test]
    fn empty_sides_reconcile() {
        // 5,000 items cannot fit one id-list page: the responder must split, not loop.
        for n in [300u32, 5_000] {
            let a = cids(0..n);
            let (have, need, _, largest) =
                reconcile_in_process(&ReconSet::new(&a), &ReconSet::default()).unwrap();
            assert_eq!(sorted(have), sorted(a.clone()));
            assert!(need.is_empty());
            let (have, need, _, largest_rev) =
                reconcile_in_process(&ReconSet::default(), &ReconSet::new(&a)).unwrap();
            assert!(have.is_empty());
            assert_eq!(sorted(need), sorted(a));
            assert!(largest.max(largest_rev) <= RECON_PAGE_LIMIT);
        }
    }

    #[test]
    fn hostile_messages_are_refused() {
        let set = ReconSet::new(&cids(0..10));
        let r = Reconciler::responder(&set);
        assert_eq!(r.reconcile(&[0x60]), Err(ReconError::Version(0x60)));
        assert_eq!(
            r.reconcile(&vec![VERSION; RECON_PAGE_LIMIT + 1]),
            Err(ReconError::Oversize)
        );
        // An id list that claims more items than the page could carry.
        let mut msg = vec![VERSION];
        Bound::Infinity.encode(&mut msg);
        put_varint(&mut msg, MODE_ID_LIST);
        put_varint(&mut msg, 1_000_000);
        assert_eq!(r.reconcile(&msg), Err(ReconError::Malformed));
        // A truncated fingerprint.
        let mut msg = vec![VERSION];
        Bound::Infinity.encode(&mut msg);
        put_varint(&mut msg, MODE_FINGERPRINT);
        msg.extend_from_slice(&[0; 5]);
        assert_eq!(r.reconcile(&msg), Err(ReconError::Malformed));
        // A non-byte-cid item reaches the initiator as an error, not a need.
        let a = Reconciler::initiator(&set);
        let mut msg = vec![VERSION];
        Bound::Infinity.encode(&mut msg);
        put_varint(&mut msg, MODE_ID_LIST);
        put_varint(&mut msg, 1);
        msg.extend_from_slice(&[0xab; ID_LEN]);
        assert_eq!(a.reconcile(&msg), Err(ReconError::NotByteCid));
    }

    #[test]
    fn requests_declare_the_algorithm() {
        let req = ReconRequest::new(vec![VERSION]);
        assert_eq!(req.recon, "negentropy-v1");
        req.admit().unwrap();
        let other = ReconRequest {
            recon: "negentropy-v2".into(),
            msg: vec![VERSION],
        };
        assert!(matches!(other.admit(), Err(ReconError::Algorithm(_))));
        assert_eq!(RECON_PROTOCOL, "/elohim/inventory-recon/1.0.0");
    }

    #[test]
    fn fingerprint_is_order_free_and_counts() {
        let ids: Vec<Id> = cids(0..5).iter().map(id_of).collect();
        let mut reversed = ids.clone();
        reversed.reverse();
        assert_eq!(fingerprint(&ids), fingerprint(&reversed));
        assert_ne!(fingerprint(&ids), fingerprint(&ids[..4]));
        assert_eq!(fingerprint(&[]).len(), 16);
    }
}
