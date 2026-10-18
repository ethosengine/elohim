//! The per-session register of who is here, and the sender-side outbox that feeds it.
//!
//! Rules (plan section 2 seam 1, concern canon C3, C4, C6a, C6b, C11):
//! - a frame whose count is not greater than the last count seen from its sender is dropped and
//!   changes nothing (idempotent effect, C6b);
//! - a drop of any kind changes nothing at all, including the rate window;
//! - presence expires `EXPIRY_MS` after the last accepted frame and then reads `Unknown`, never
//!   `Left`; only a signalled `Leave` reads `Left` (honest absence, C4);
//! - the sender renews at half the expiry (C3);
//! - size, value-shape, per-sender rate and sender-count limits are enforced on receipt (C6a);
//! - a slow receiver gets the latest values, not a queue: each sender holds one slot (C11).
//!
//! Time is milliseconds passed in by the caller; this module never reads a clock.

use crate::frame::{Body, Frame, FrameError};
use crate::session_id::SessionId;
use std::collections::BTreeMap;

/// Silence for this long reads as unknown. The story's 30 seconds.
pub const EXPIRY_MS: u64 = 30_000;
/// A sender renews at half the expiry.
pub const RENEW_INTERVAL_MS: u64 = EXPIRY_MS / 2;
/// The adapter polls an `Outbox` at least this often, so a coalesced final update lands well
/// inside the story's 5 second bound.
pub const POLL_INTERVAL_MS: u64 = 250;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Largest accepted frame on the wire, in bytes (the caller passes the wire length).
    pub max_frame_bytes: usize,
    pub max_values: usize,
    pub max_name_bytes: usize,
    pub max_value_bytes: usize,
    pub max_sender_bytes: usize,
    /// Accepted frames per sender per window; more are dropped.
    pub max_frames_per_window: u32,
    /// Frames an `Outbox` may emit per window. Lower than the receive cap so that two sender
    /// windows straddling one receiver window still fit under it.
    pub send_frames_per_window: u32,
    pub window_ms: u64,
    /// Distinct senders held in one register.
    pub max_senders: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_frame_bytes: 4096,
            max_values: 16,
            max_name_bytes: 64,
            max_value_bytes: 512,
            max_sender_bytes: 128,
            max_frames_per_window: 20,
            send_frames_per_window: 10,
            window_ms: 1_000,
            max_senders: 16,
        }
    }
}

/// Why a frame was dropped. A drop never changes the register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dropped {
    UnknownVersion(u8),
    TooLarge,
    WrongSession,
    BadSender,
    BadValues,
    /// Count not greater than the last seen from this sender (repeat or older).
    Stale,
    TooManySenders,
    RateCapped,
}

/// What an accepted frame did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    /// New sender, or one that was left or unknown is here again.
    Arrived,
    /// Here, with different values than before.
    Updated,
    /// Here, nothing new to show.
    Renewed,
    /// Signalled clean exit.
    Left,
    /// A newer Leave from a sender already Left: nothing shown changes, but the sender's
    /// count watermark advances so a delayed older Present stays stale.
    StillLeft,
}

/// What this machine shows for another one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeerState {
    Here(BTreeMap<String, String>),
    Left,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Slot {
    Here(BTreeMap<String, String>),
    Left,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    last_count: u64,
    last_seen: u64,
    slot: Slot,
    window_start: u64,
    window_n: u32,
}

impl Entry {
    fn expired(&self, now: u64) -> bool {
        matches!(self.slot, Slot::Here(_)) && now.saturating_sub(self.last_seen) >= EXPIRY_MS
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Register {
    session: SessionId,
    limits: Limits,
    peers: BTreeMap<String, Entry>,
}

impl Register {
    pub fn new(session: SessionId) -> Self {
        Self::with_limits(session, Limits::default())
    }

    pub fn with_limits(session: SessionId, limits: Limits) -> Self {
        Register {
            session,
            limits,
            peers: BTreeMap::new(),
        }
    }

    pub fn session(&self) -> SessionId {
        self.session
    }

    /// Apply a received frame. `wire_len` is the encoded length as received. Every check runs
    /// before any mutation, so an `Err` leaves the register exactly as it was.
    pub fn apply(&mut self, frame: &Frame, wire_len: usize, now: u64) -> Result<Applied, Dropped> {
        let l = self.limits;
        frame
            .check_version()
            .map_err(|FrameError::UnknownVersion(v)| Dropped::UnknownVersion(v))?;
        if wire_len > l.max_frame_bytes {
            return Err(Dropped::TooLarge);
        }
        if frame.session != self.session {
            return Err(Dropped::WrongSession);
        }
        if frame.sender.is_empty() || frame.sender.len() > l.max_sender_bytes {
            return Err(Dropped::BadSender);
        }
        if let Body::Present { values } = &frame.body {
            let ok = values.len() <= l.max_values
                && values
                    .iter()
                    .all(|(k, v)| k.len() <= l.max_name_bytes && v.len() <= l.max_value_bytes);
            if !ok {
                return Err(Dropped::BadValues);
            }
        }
        if frame.count == 0 {
            return Err(Dropped::Stale);
        }
        let existing = self.peers.get(&frame.sender);
        if let Some(e) = existing {
            if frame.count <= e.last_count {
                return Err(Dropped::Stale);
            }
        } else if self.peers.len() >= l.max_senders {
            return Err(Dropped::TooManySenders);
        }
        // Rate window for this sender, computed without mutating.
        let (window_start, window_n) = match existing {
            Some(e) if now.saturating_sub(e.window_start) < l.window_ms => {
                (e.window_start, e.window_n)
            }
            _ => (now, 0),
        };
        // Only the transition into Left is exempt from the cap, so a clean close reads as Left
        // even after a burst of presence; further Leaves from a Left sender go through the normal cap.
        let is_leave_transition = matches!(frame.body, Body::Leave)
            && existing.is_some_and(|e| !matches!(e.slot, Slot::Left));
        if window_n >= l.max_frames_per_window && !is_leave_transition {
            return Err(Dropped::RateCapped);
        }

        // Accepted: mutate.
        let was_here =
            matches!(existing, Some(e) if matches!(e.slot, Slot::Here(_)) && !e.expired(now));
        let prev_values = match existing {
            Some(Entry {
                slot: Slot::Here(v),
                ..
            }) if was_here => Some(v.clone()),
            _ => None,
        };
        let (slot, applied) = match &frame.body {
            Body::Present { values } => {
                let applied = match prev_values {
                    None => Applied::Arrived,
                    Some(p) if &p == values => Applied::Renewed,
                    Some(_) => Applied::Updated,
                };
                (Slot::Here(values.clone()), applied)
            }
            Body::Leave if matches!(existing, Some(e) if matches!(e.slot, Slot::Left)) => {
                (Slot::Left, Applied::StillLeft)
            }
            Body::Leave => (Slot::Left, Applied::Left),
        };
        self.peers.insert(
            frame.sender.clone(),
            Entry {
                last_count: frame.count,
                last_seen: now,
                slot,
                window_start,
                window_n: window_n.saturating_add(1),
            },
        );
        Ok(applied)
    }

    /// Turn every presence that has gone quiet for `EXPIRY_MS` into `Unknown`, dropping its
    /// values. Returns the senders that changed. `view` and `peer` already read expired
    /// presence as unknown; this makes the memory match and tells the caller what to announce.
    pub fn expire(&mut self, now: u64) -> Vec<String> {
        let mut changed = Vec::new();
        for (sender, e) in self.peers.iter_mut() {
            if e.expired(now) {
                e.slot = Slot::Unknown;
                changed.push(sender.clone());
            }
        }
        changed
    }

    /// The transport calls this when an authenticated connection from `sender` ends. It forgets
    /// that sender's dedup state and presence and frees its slot, so a reconnecting outbox
    /// (which restarts at count 1) is heard. Silence or expiry must NOT call this: forgetting
    /// counts on mere silence would let a stale frame be replayed. Returns whether it was held.
    pub fn connection_closed(&mut self, sender: &str) -> bool {
        self.peers.remove(sender).is_some()
    }

    /// What this machine shows for `sender` at `now`; `None` if never heard from.
    pub fn peer(&self, sender: &str, now: u64) -> Option<PeerState> {
        self.peers.get(sender).map(|e| Self::state_of(e, now))
    }

    /// Everyone heard from, in sender order.
    pub fn view(&self, now: u64) -> Vec<(String, PeerState)> {
        self.peers
            .iter()
            .map(|(s, e)| (s.clone(), Self::state_of(e, now)))
            .collect()
    }

    fn state_of(e: &Entry, now: u64) -> PeerState {
        match &e.slot {
            Slot::Here(_) if e.expired(now) => PeerState::Unknown,
            Slot::Here(v) => PeerState::Here(v.clone()),
            Slot::Left => PeerState::Left,
            Slot::Unknown => PeerState::Unknown,
        }
    }
}

/// Why an `Outbox` refused to send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutboxError {
    /// Values exceed the shape limits (count, name length, value length).
    BadValues,
    /// The frame counter is exhausted; the connection must be restarted.
    CountExhausted,
}

/// Sender side: numbers frames, remembers the latest values, says when a renewal is due, and
/// caps its own send rate. Over the cap, `present` keeps only the latest values (no queue,
/// C11) and `poll` emits them as soon as the cap allows. A `leave` is never held back.
///
/// Time is a caller-supplied monotonic millisecond clock; a clock that runs backwards is
/// outside this contract.
#[derive(Clone, Debug)]
pub struct Outbox {
    sender: String,
    session: SessionId,
    limits: Limits,
    count: u64,
    values: BTreeMap<String, String>,
    pending: bool,
    /// A Present frame has been emitted since the last Leave.
    announced: bool,
    last_sent: Option<u64>,
    window_start: u64,
    window_n: u32,
    open: bool,
}

impl Outbox {
    pub fn new(sender: impl Into<String>, session: SessionId) -> Self {
        Self::with_limits(sender, session, Limits::default())
    }

    pub fn with_limits(sender: impl Into<String>, session: SessionId, limits: Limits) -> Self {
        Outbox {
            sender: sender.into(),
            session,
            limits,
            count: 0,
            values: BTreeMap::new(),
            pending: false,
            announced: false,
            last_sent: None,
            window_start: 0,
            window_n: 0,
            open: false,
        }
    }

    fn slot_free(&self, now: u64) -> bool {
        self.last_sent.is_none()
            || now.saturating_sub(self.window_start) >= self.limits.window_ms
            || self.window_n < self.limits.send_frames_per_window
    }

    fn take_slot(&mut self, now: u64) {
        if self.last_sent.is_none()
            || now.saturating_sub(self.window_start) >= self.limits.window_ms
        {
            self.window_start = now;
            self.window_n = 0;
        }
        self.window_n = self.window_n.saturating_add(1);
    }

    fn next(&mut self, body: Body, now: u64) -> Result<Frame, OutboxError> {
        let count = self
            .count
            .checked_add(1)
            .ok_or(OutboxError::CountExhausted)?;
        self.count = count;
        self.take_slot(now);
        self.last_sent = Some(now);
        Ok(Frame::new(self.sender.clone(), self.session, count, body))
    }

    fn emit_values(&mut self, now: u64) -> Result<Frame, OutboxError> {
        let values = self.values.clone();
        let f = self.next(Body::Present { values }, now)?;
        self.pending = false;
        self.announced = true;
        Ok(f)
    }

    /// Set the current values (also opens the session). Returns the frame to send now, or
    /// `None` when the send cap holds it back; the latest values are kept and `poll` sends them.
    pub fn present(
        &mut self,
        values: BTreeMap<String, String>,
        now: u64,
    ) -> Result<Option<Frame>, OutboxError> {
        let l = self.limits;
        let ok = values.len() <= l.max_values
            && values
                .iter()
                .all(|(k, v)| k.len() <= l.max_name_bytes && v.len() <= l.max_value_bytes);
        if !ok {
            return Err(OutboxError::BadValues);
        }
        self.values = values;
        self.open = true;
        self.pending = true;
        if self.slot_free(now) {
            self.emit_values(now).map(Some)
        } else {
            Ok(None)
        }
    }

    /// True when the session is open and half the expiry has passed since the last send.
    pub fn renewal_due(&self, now: u64) -> bool {
        self.open
            && self
                .last_sent
                .is_some_and(|t| now.saturating_sub(t) >= RENEW_INTERVAL_MS)
    }

    /// The next frame owed, if any: held-back latest values as soon as the cap allows, else a
    /// renewal when due. Call on a timer and after each `present`.
    pub fn poll(&mut self, now: u64) -> Result<Option<Frame>, OutboxError> {
        if !self.open || !self.slot_free(now) {
            return Ok(None);
        }
        if self.pending || self.renewal_due(now) {
            return self.emit_values(now).map(Some);
        }
        Ok(None)
    }

    /// Clean exit. Never held back by the send cap. Emits at most once per announced period:
    /// `None` when no Present was emitted since the last Leave. After this nothing is sent until the next `present`.
    pub fn leave(&mut self, now: u64) -> Result<Option<Frame>, OutboxError> {
        self.open = false;
        self.pending = false;
        if !self.announced {
            // The receiver never saw this period's presence; its state is already right.
            return Ok(None);
        }
        let f = self.next(Body::Leave, now)?;
        self.announced = false;
        Ok(Some(f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid() -> SessionId {
        SessionId::for_document(b"sheet-1")
    }

    fn vals(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn present(sender: &str, count: u64, v: &[(&str, &str)]) -> Frame {
        Frame::new(sender, sid(), count, Body::Present { values: vals(v) })
    }

    #[test]
    fn arrival_update_renew() {
        let mut r = Register::new(sid());
        assert_eq!(
            r.apply(&present("a", 1, &[("sel", "A1")]), 50, 0),
            Ok(Applied::Arrived)
        );
        assert_eq!(
            r.apply(&present("a", 2, &[("sel", "A1")]), 50, 100),
            Ok(Applied::Renewed)
        );
        assert_eq!(
            r.apply(&present("a", 3, &[("sel", "B2")]), 50, 200),
            Ok(Applied::Updated)
        );
        assert_eq!(
            r.peer("a", 200),
            Some(PeerState::Here(vals(&[("sel", "B2")])))
        );
        assert_eq!(r.peer("nobody", 200), None);
    }

    #[test]
    fn repeated_and_older_counts_are_dropped() {
        let mut r = Register::new(sid());
        r.apply(&present("a", 5, &[("x", "1")]), 10, 0).unwrap();
        assert_eq!(
            r.apply(&present("a", 5, &[("x", "2")]), 10, 1),
            Err(Dropped::Stale)
        );
        assert_eq!(
            r.apply(&present("a", 4, &[("x", "3")]), 10, 2),
            Err(Dropped::Stale)
        );
        assert_eq!(r.peer("a", 3), Some(PeerState::Here(vals(&[("x", "1")]))));
    }

    #[test]
    fn silence_is_unknown_never_left() {
        let mut r = Register::new(sid());
        r.apply(&present("a", 1, &[("x", "1")]), 10, 0).unwrap();
        assert!(matches!(
            r.peer("a", EXPIRY_MS - 1),
            Some(PeerState::Here(_))
        ));
        assert_eq!(r.peer("a", EXPIRY_MS), Some(PeerState::Unknown));
        assert_eq!(r.expire(EXPIRY_MS), vec!["a".to_string()]);
        assert_eq!(
            r.peer("a", EXPIRY_MS + 10_000_000),
            Some(PeerState::Unknown)
        );
        assert!(r.expire(EXPIRY_MS * 3).is_empty());
    }

    #[test]
    fn renewal_keeps_presence_alive() {
        let mut r = Register::new(sid());
        let mut t = 0;
        r.apply(&present("a", 1, &[]), 10, t).unwrap();
        for n in 2..10 {
            t += RENEW_INTERVAL_MS;
            r.apply(&present("a", n, &[]), 10, t).unwrap();
            assert!(matches!(r.peer("a", t), Some(PeerState::Here(_))));
        }
    }

    #[test]
    fn leave_reads_left_and_clears_values_then_rejoin() {
        let mut r = Register::new(sid());
        r.apply(&present("a", 1, &[("x", "1")]), 10, 0).unwrap();
        let leave = Frame::new("a", sid(), 2, Body::Leave);
        assert_eq!(r.apply(&leave, 10, 5), Ok(Applied::Left));
        assert_eq!(r.peer("a", 5), Some(PeerState::Left));
        assert_eq!(r.peer("a", 5 + EXPIRY_MS * 10), Some(PeerState::Left));
        assert_eq!(r.apply(&present("a", 3, &[]), 10, 6), Ok(Applied::Arrived));
    }

    #[test]
    fn revival_after_expiry_is_an_arrival() {
        let mut r = Register::new(sid());
        r.apply(&present("a", 1, &[]), 10, 0).unwrap();
        assert_eq!(
            r.apply(&present("a", 2, &[]), 10, EXPIRY_MS + 1),
            Ok(Applied::Arrived)
        );
    }

    #[test]
    fn size_and_shape_limits() {
        let mut r = Register::new(sid());
        assert_eq!(
            r.apply(&present("a", 1, &[]), 4097, 0),
            Err(Dropped::TooLarge)
        );
        assert_eq!(
            r.apply(&present("a", 1, &[]), 4096, 0),
            Ok(Applied::Arrived)
        );
        let big = "x".repeat(513);
        assert_eq!(
            r.apply(&present("a", 2, &[("k", &big)]), 10, 1),
            Err(Dropped::BadValues)
        );
        let many: Vec<(String, String)> = (0..17).map(|i| (format!("k{i}"), "v".into())).collect();
        let mut f = present("a", 2, &[]);
        f.body = Body::Present {
            values: many.into_iter().collect(),
        };
        assert_eq!(r.apply(&f, 10, 1), Err(Dropped::BadValues));
        assert_eq!(
            r.apply(&present("", 1, &[]), 10, 1),
            Err(Dropped::BadSender)
        );
    }

    #[test]
    fn wrong_session_and_unknown_version() {
        let mut r = Register::new(sid());
        let other = Frame::new("a", SessionId::for_document(b"other"), 1, Body::Leave);
        assert_eq!(r.apply(&other, 10, 0), Err(Dropped::WrongSession));
        let mut f = present("a", 1, &[]);
        f.version = 9;
        assert_eq!(r.apply(&f, 10, 0), Err(Dropped::UnknownVersion(9)));
        assert!(r.view(0).is_empty());
    }

    #[test]
    fn rate_cap_per_sender_and_window_reopens() {
        let mut r = Register::new(sid());
        for n in 1..=20 {
            assert!(r.apply(&present("a", n, &[]), 10, 0).is_ok());
        }
        assert_eq!(
            r.apply(&present("a", 21, &[]), 10, 999),
            Err(Dropped::RateCapped)
        );
        // another sender is unaffected
        assert!(r.apply(&present("b", 1, &[]), 10, 999).is_ok());
        // the window reopens
        assert!(r.apply(&present("a", 21, &[]), 10, 1_000).is_ok());
    }

    #[test]
    fn sender_count_cap() {
        let mut r = Register::new(sid());
        for i in 0..16 {
            r.apply(&present(&format!("s{i}"), 1, &[]), 10, 0).unwrap();
        }
        assert_eq!(
            r.apply(&present("late", 1, &[]), 10, 0),
            Err(Dropped::TooManySenders)
        );
        assert!(r.apply(&present("s0", 2, &[]), 10, 1).is_ok());
    }

    #[test]
    fn outbox_numbers_renews_at_half_and_leaves() {
        let mut o = Outbox::new("a", sid());
        assert!(!o.renewal_due(0));
        let f1 = o.present(vals(&[("sel", "A1")]), 0).unwrap().unwrap();
        assert_eq!(f1.count, 1);
        assert!(o.poll(RENEW_INTERVAL_MS - 1).unwrap().is_none());
        let f2 = o.poll(RENEW_INTERVAL_MS).unwrap().unwrap();
        assert_eq!(f2.count, 2);
        assert_eq!(
            f2.body,
            Body::Present {
                values: vals(&[("sel", "A1")])
            }
        );
        let f3 = o.leave(RENEW_INTERVAL_MS + 1).unwrap().unwrap();
        assert!(o.leave(RENEW_INTERVAL_MS + 2).unwrap().is_none());
        assert_eq!(f3.body, Body::Leave);
        assert!(o.poll(RENEW_INTERVAL_MS * 10).unwrap().is_none());
    }

    #[test]
    fn leave_passes_the_receive_rate_cap() {
        let mut r = Register::new(sid());
        for n in 1..=20 {
            r.apply(&present("a", n, &[]), 10, 0).unwrap();
        }
        assert_eq!(
            r.apply(&present("a", 21, &[]), 10, 1),
            Err(Dropped::RateCapped)
        );
        let leave = Frame::new("a", sid(), 22, Body::Leave);
        assert_eq!(r.apply(&leave, 10, 2), Ok(Applied::Left));
        assert_eq!(r.peer("a", 2), Some(PeerState::Left));
        let stale = Frame::new("a", sid(), 22, Body::Leave);
        assert_eq!(r.apply(&stale, 10, 3), Err(Dropped::Stale));
        let big = Frame::new("a", sid(), 23, Body::Leave);
        assert_eq!(r.apply(&big, 99_999, 3), Err(Dropped::TooLarge));
    }

    #[test]
    fn outbox_send_cap_coalesces_to_latest_and_flushes_within_window() {
        let mut o = Outbox::new("a", sid());
        let mut sent = 0;
        for i in 0..25 {
            let v = vals(&[("sel", &format!("c{i}"))]);
            if o.present(v, 100).unwrap().is_some() {
                sent += 1;
            }
        }
        assert_eq!(sent, 10);
        assert!(o.poll(500).unwrap().is_none());
        let f = o.poll(1_100).unwrap().expect("pending latest flushes");
        assert_eq!(
            f.body,
            Body::Present {
                values: vals(&[("sel", "c24")])
            }
        );
        assert_eq!(f.count, 11);
        assert!(o.poll(1_101).unwrap().is_none());
        for i in 0..12 {
            let _ = o.present(vals(&[("sel", &format!("d{i}"))]), 1_200);
        }
        assert_eq!(o.leave(1_201).unwrap().unwrap().body, Body::Leave);
    }

    #[test]
    fn connection_closed_lets_a_reconnecting_outbox_be_heard() {
        let mut r = Register::new(sid());
        let mut o = Outbox::new("a", sid());
        let f = o.present(vals(&[]), 0).unwrap().unwrap();
        r.apply(&f, 10, 0).unwrap();
        let again = Outbox::new("a", sid())
            .present(vals(&[]), 1)
            .unwrap()
            .unwrap();
        assert_eq!(r.apply(&again, 10, 1), Err(Dropped::Stale));
        assert!(r.connection_closed("a"));
        assert_eq!(r.peer("a", 1), None);
        assert_eq!(r.apply(&again, 10, 1), Ok(Applied::Arrived));
        assert!(!r.connection_closed("nobody"));
    }

    #[test]
    fn expiry_does_not_forget_counts() {
        let mut r = Register::new(sid());
        r.apply(&present("a", 5, &[]), 10, 0).unwrap();
        r.expire(EXPIRY_MS * 2);
        assert_eq!(
            r.apply(&present("a", 5, &[]), 10, EXPIRY_MS * 2),
            Err(Dropped::Stale)
        );
    }

    #[test]
    fn slots_are_reclaimed_only_by_connection_closed() {
        let mut r = Register::new(sid());
        for i in 0..16 {
            r.apply(&present(&format!("s{i}"), 1, &[]), 10, i).unwrap();
        }
        assert_eq!(
            r.apply(&present("n1", 1, &[]), 10, 20),
            Err(Dropped::TooManySenders)
        );
        // a Left sender does not give up its slot
        r.apply(&Frame::new("s5", sid(), 2, Body::Leave), 10, 22)
            .unwrap();
        assert_eq!(
            r.apply(&present("n1", 1, &[]), 10, 23),
            Err(Dropped::TooManySenders)
        );
        r.connection_closed("s3");
        assert!(r.apply(&present("n1", 1, &[]), 10, 24).is_ok());
        assert_eq!(r.view(24).len(), 16);
    }

    #[test]
    fn full_register_with_a_left_sender_does_not_resurrect_stale_presence() {
        let limits = Limits {
            max_senders: 1,
            ..Limits::default()
        };
        let mut r = Register::with_limits(sid(), limits);
        r.apply(&present("A", 1, &[]), 10, 0).unwrap();
        r.apply(&Frame::new("A", sid(), 2, Body::Leave), 10, 1)
            .unwrap();
        assert_eq!(
            r.apply(&Frame::new("B", sid(), 1, Body::Leave), 10, 2),
            Err(Dropped::TooManySenders)
        );
        assert!(r.apply(&present("A", 1, &[]), 10, 3).is_err());
        assert_eq!(r.peer("A", 3), Some(PeerState::Left));
        assert_eq!(r.peer("B", 3), None);
    }

    #[test]
    fn leave_burst_after_the_first_is_not_exempt_and_never_reports_left_again() {
        let mut r = Register::new(sid());
        r.apply(&present("a", 1, &[]), 10, 0).unwrap();
        assert_eq!(
            r.apply(&Frame::new("a", sid(), 2, Body::Leave), 10, 0),
            Ok(Applied::Left)
        );
        let mut still = 0;
        let mut capped = 0;
        for n in 3..60 {
            match r.apply(&Frame::new("a", sid(), n, Body::Leave), 10, 0) {
                Ok(Applied::StillLeft) => still += 1,
                Err(Dropped::RateCapped) => capped += 1,
                other => panic!("unexpected {other:?}"),
            }
        }
        assert!(still > 0 && capped > 0);
        // a never-present sender's Leaves go through the normal cap
        let mut q = Register::new(sid());
        let mut accepted = 0;
        for n in 1..40 {
            if q.apply(&Frame::new("z", sid(), n, Body::Leave), 10, 0)
                .is_ok()
            {
                accepted += 1;
            }
        }
        assert_eq!(accepted, 20);
    }

    #[test]
    fn later_leave_advances_the_watermark_so_a_delayed_present_stays_stale() {
        let mut r = Register::new(sid());
        r.apply(&present("a", 1, &[]), 10, 0).unwrap();
        assert_eq!(
            r.apply(&Frame::new("a", sid(), 2, Body::Leave), 10, 1),
            Ok(Applied::Left)
        );
        assert_eq!(
            r.apply(&Frame::new("a", sid(), 4, Body::Leave), 10, 2),
            Ok(Applied::StillLeft)
        );
        assert_eq!(r.apply(&present("a", 3, &[]), 10, 3), Err(Dropped::Stale));
        assert_eq!(r.peer("a", 3), Some(PeerState::Left));
    }

    #[test]
    fn present_leave_loop_under_an_exhausted_send_window_emits_no_leave() {
        let mut o = Outbox::new("a", sid());
        for _ in 0..10 {
            assert!(o.present(vals(&[]), 0).unwrap().is_some());
        }
        // announced: the first leave goes out
        assert!(o.leave(0).unwrap().is_some());
        // window now exhausted; reopen attempts are capped and never announced
        for _ in 0..50 {
            assert!(o.present(vals(&[]), 0).unwrap().is_none());
            assert!(o.leave(0).unwrap().is_none());
        }
    }

    #[test]
    fn outbox_refuses_oversized_values_and_exhausted_counter() {
        let mut o = Outbox::new("a", sid());
        let big = "x".repeat(513);
        assert_eq!(
            o.present(vals(&[("k", &big)]), 0),
            Err(OutboxError::BadValues)
        );
        let many: BTreeMap<String, String> =
            (0..17).map(|i| (format!("k{i}"), "v".into())).collect();
        assert_eq!(o.present(many, 0), Err(OutboxError::BadValues));
        assert!(o.poll(0).unwrap().is_none());
        o.count = u64::MAX;
        assert_eq!(o.present(vals(&[]), 0), Err(OutboxError::CountExhausted));
        assert_eq!(o.leave(0), Ok(None));
    }

    /// Fixed-seed xorshift; proptest is not in the workspace lock.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    /// A repeated or older count never changes the register, whatever else the frame holds.
    /// More strongly: any dropped frame leaves the register exactly as it was.
    #[test]
    fn property_stale_count_and_any_drop_never_change_register() {
        let senders = ["a", "b", "c"];
        for seed in 1..=200u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let mut r = Register::new(sid());
            let mut last: BTreeMap<&str, u64> = BTreeMap::new();
            let mut now = 0u64;
            for _ in 0..300 {
                now += rng.below(3_000);
                let s = senders[rng.below(3) as usize];
                let known = last.get(s).copied();
                // bias toward repeats and older counts
                let count = match (known, rng.below(4)) {
                    (Some(k), 0) => k,
                    (Some(k), 1) => rng.below(k + 1),
                    (Some(k), _) => k + 1 + rng.below(3),
                    (None, _) => rng.below(5),
                };
                let body = if rng.below(8) == 0 {
                    Body::Leave
                } else {
                    let mut v = BTreeMap::new();
                    v.insert("sel".to_string(), format!("c{}", rng.below(5)));
                    Body::Present { values: v }
                };
                let frame = Frame::new(s, sid(), count, body);
                let before = r.clone();
                let stale = known.is_some_and(|k| count <= k);
                let res = r.apply(&frame, 64, now);
                if stale {
                    assert_eq!(res, Err(Dropped::Stale), "seed {seed}");
                }
                if res.is_err() {
                    assert_eq!(r, before, "a drop changed the register (seed {seed})");
                } else {
                    last.insert(s, count);
                }
            }
        }
    }
}
