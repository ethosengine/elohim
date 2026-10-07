//! What the approving node remembers about an ask whose code it holds, apart
//! from the held delivery itself.
//!
//! A terminal that can reach this node over plain HTTP asks with
//! `returnPath: {kind: "hold"}`. When the person agrees, the delivery is held
//! in the ceremony's store exactly as any other, bound at issuance to the
//! request's state, challenge, client and device, and the person carries
//! nothing. The terminal collects it with its verifier
//! (`MemoryStore::collect`, `POST /auth/consent/collect`). That store never
//! keeps a code, only its digest, and this module keeps no code either.
//!
//! What it does keep, by state, so a terminal that collects before there is
//! anything to collect, or after it is gone, is told the truth rather than
//! "unknown":
//!
//! - **shown**: the consent screen was shown for the ask (`view`). Used only
//!   to tell the terminal to keep waiting, and how long, never to admit it to
//!   anything. The waiting window starts at the first showing; showing the
//!   ask again does not extend it.
//! - **expired**, **spent**, **declined**: the terminal answers. Once one is
//!   given, every later collect for that state is given the same one until it
//!   is forgotten, whatever the delivery store holds by then, and showing the
//!   ask again changes nothing. A decline comes only from an authenticated
//!   decision on a waiting ask (`/auth/consent/pending/decide`); there is no
//!   route that declines by state alone.
//!
//! A shown ask never answered becomes expired when its wait ends. A terminal
//! answer is remembered for one window from when it was given (an expired
//! wait, one window from when the wait ended), then forgotten.
//!
//! Ephemeral (class C): nothing here is written to the DHT, the content
//! database or the disk. A node that restarts forgets every ask, and the
//! terminal asks again.
//!
//! Bounded: spent and declined answers come only from the person's own
//! agreements and decisions. Shown asks come from `view`, which anyone may
//! call, so shown and expired asks together are kept to [`MAX_SHOWN`] and the
//! oldest gives way.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// How long a wait lasts, and how long a terminal answer is remembered: the
/// code's window.
pub const WINDOW_MICROS: i64 = super::device_consent::CODE_TTL_MICROS;

/// How many shown or expired asks are kept.
pub const MAX_SHOWN: usize = 256;

/// What this node can tell a terminal about an ask from what it remembers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NothingHeld {
    /// The person has the ask in front of them and has not answered.
    Waiting { seconds_left: u64 },
    /// The ask was not answered, or not collected, in time.
    Expired,
    /// The person said no.
    Declined,
    /// What was held was collected.
    Spent,
    /// Nothing is known here for that state.
    Unknown,
}

impl NothingHeld {
    /// An answer every later collect is given again.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Expired | Self::Declined | Self::Spent)
    }
}

/// A terminal answer, as recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled {
    Expired,
    Declined,
    Spent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Shown,
    Settled(Settled),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    status: Status,
    /// Shown: when the wait ends. Settled: when the answer is forgotten.
    until_micros: i64,
}

impl Entry {
    /// Past this the state is forgotten. A shown ask is answered "expired"
    /// for one window after its wait.
    fn forgotten_at(&self) -> i64 {
        match self.status {
            Status::Shown => self.until_micros.saturating_add(WINDOW_MICROS),
            Status::Settled(_) => self.until_micros,
        }
    }

    /// Whether anyone may cause this entry (`view`, or a lapsed wait), so it
    /// counts against [`MAX_SHOWN`] and may give way.
    fn evictable(&self) -> bool {
        matches!(
            self.status,
            Status::Shown | Status::Settled(Settled::Expired)
        )
    }
}

/// Asks shown or answered on this node, by state.
#[derive(Debug, Default)]
pub struct AskStates {
    entries: Mutex<HashMap<String, Entry>>,
}

/// The one this node runs with.
pub fn ask_states() -> &'static AskStates {
    static STATES: OnceLock<AskStates> = OnceLock::new();
    STATES.get_or_init(AskStates::new)
}

impl AskStates {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        // Every critical section is a few map operations, so a panic inside
        // one cannot leave the map half-written.
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Forget everything past its remembered window, and settle every wait
    /// that has ended as expired.
    fn sweep(map: &mut HashMap<String, Entry>, now_micros: i64) {
        // bounded-work: one pass over at most MAX_SHOWN shown or expired asks
        // plus the person's own spent and declined ones, each living at most
        // two windows.
        map.retain(|_, e| now_micros < e.forgotten_at());
        for e in map.values_mut() {
            if e.status == Status::Shown && now_micros >= e.until_micros {
                *e = Entry {
                    status: Status::Settled(Settled::Expired),
                    until_micros: e.forgotten_at(),
                };
            }
        }
    }

    /// Make room for one more evictable entry.
    fn make_room(map: &mut HashMap<String, Entry>) {
        if map.values().filter(|e| e.evictable()).count() < MAX_SHOWN {
            return;
        }
        // bounded-work: one pass over at most MAX_SHOWN evictable entries.
        let oldest = map
            .iter()
            .filter(|(_, e)| e.evictable())
            .min_by_key(|(_, e)| e.forgotten_at())
            .map(|(s, _)| s.clone());
        if let Some(oldest) = oldest {
            map.remove(&oldest);
        }
    }

    /// The consent screen was shown for `state`. Only the first showing
    /// starts the wait: showing it again neither extends the wait nor reopens
    /// an ask already answered.
    pub fn shown(&self, state: &str, now_micros: i64) {
        let mut map = self.lock();
        Self::sweep(&mut map, now_micros);
        if map.contains_key(state) {
            return;
        }
        Self::make_room(&mut map);
        map.insert(
            state.to_string(),
            Entry {
                status: Status::Shown,
                until_micros: now_micros.saturating_add(WINDOW_MICROS),
            },
        );
    }

    /// Record a terminal answer for `state`, remembered for one window. The
    /// first terminal answer stands: a later one changes nothing.
    pub fn settle(&self, state: &str, answer: Settled, now_micros: i64) {
        let mut map = self.lock();
        Self::sweep(&mut map, now_micros);
        if matches!(map.get(state), Some(e) if matches!(e.status, Status::Settled(_))) {
            return;
        }
        let entry = Entry {
            status: Status::Settled(answer),
            until_micros: now_micros.saturating_add(WINDOW_MICROS),
        };
        if entry.evictable() && !map.contains_key(state) {
            Self::make_room(&mut map);
        }
        map.insert(state.to_string(), entry);
    }

    /// The person said no to the ask with `state`, by an authenticated
    /// decision.
    pub fn declined(&self, state: &str, now_micros: i64) {
        self.settle(state, Settled::Declined, now_micros);
    }

    /// What this node remembers about `state`.
    pub fn answer(&self, state: &str, now_micros: i64) -> NothingHeld {
        let mut map = self.lock();
        Self::sweep(&mut map, now_micros);
        match map.get(state).map(|e| (e.status, e.until_micros)) {
            None => NothingHeld::Unknown,
            Some((Status::Settled(Settled::Expired), _)) => NothingHeld::Expired,
            Some((Status::Settled(Settled::Declined), _)) => NothingHeld::Declined,
            Some((Status::Settled(Settled::Spent), _)) => NothingHeld::Spent,
            Some((Status::Shown, until)) => {
                // Positive here: the sweep settled every ended wait. Rounded
                // up, so a terminal is never told 0 while it may still wait.
                let left = until - now_micros;
                NothingHeld::Waiting {
                    seconds_left: u64::try_from((left + 999_999) / 1_000_000).unwrap_or(0),
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_791_000_000_000_000;

    #[test]
    fn a_shown_ask_waits_then_expires_then_is_forgotten() {
        let states = AskStates::new();
        states.shown("s1", NOW);
        assert_eq!(
            states.answer("s1", NOW + 1_500_000),
            NothingHeld::Waiting { seconds_left: 299 }
        );
        assert_eq!(
            states.answer("s1", NOW + WINDOW_MICROS),
            NothingHeld::Expired
        );
        assert_eq!(
            states.answer("s1", NOW + 2 * WINDOW_MICROS),
            NothingHeld::Unknown
        );
        assert!(states.is_empty());
    }

    #[test]
    fn showing_an_ask_again_does_not_undo_a_decline() {
        let states = AskStates::new();
        states.shown("s1", NOW);
        states.declined("s1", NOW + 1);
        states.shown("s1", NOW + 2);
        assert_eq!(states.answer("s1", NOW + 3), NothingHeld::Declined);
        assert_eq!(
            states.answer("s1", NOW + 1 + WINDOW_MICROS),
            NothingHeld::Unknown
        );
    }

    #[test]
    fn only_the_first_showing_starts_the_wait() {
        let states = AskStates::new();
        states.shown("s1", NOW);
        states.shown("s1", NOW + 200_000_000);
        assert_eq!(
            states.answer("s1", NOW + 200_000_000),
            NothingHeld::Waiting { seconds_left: 100 }
        );
        assert_eq!(
            states.answer("s1", NOW + WINDOW_MICROS),
            NothingHeld::Expired
        );
    }

    #[test]
    fn showing_an_answered_ask_again_never_reopens_it() {
        let states = AskStates::new();
        // A wait that ended is expired; showing the link again does not
        // restart it.
        states.shown("lapsed", NOW);
        states.shown("lapsed", NOW + WINDOW_MICROS + 1);
        assert_eq!(
            states.answer("lapsed", NOW + WINDOW_MICROS + 2),
            NothingHeld::Expired
        );
        for (state, settled, told) in [
            ("spent", Settled::Spent, NothingHeld::Spent),
            ("expired", Settled::Expired, NothingHeld::Expired),
            ("declined", Settled::Declined, NothingHeld::Declined),
        ] {
            states.settle(state, settled, NOW);
            states.shown(state, NOW + 1);
            // A later, different answer does not replace the first.
            states.settle(state, Settled::Expired, NOW + 2);
            states.declined(state, NOW + 3);
            assert_eq!(states.answer(state, NOW + 4), told, "{state}");
            assert!(told.is_terminal());
            assert_eq!(
                states.answer(state, NOW + WINDOW_MICROS),
                NothingHeld::Unknown,
                "{state}"
            );
        }
    }

    #[test]
    fn asks_shown_by_anyone_are_kept_to_a_bound() {
        let states = AskStates::new();
        states.declined("mine", NOW);
        for i in 0..(MAX_SHOWN + 10) {
            states.shown(&format!("s{i}"), NOW + i as i64);
        }
        assert_eq!(states.len(), MAX_SHOWN + 1);
        // The oldest gave way; the newest and the person's decline stay.
        assert_eq!(states.answer("s0", NOW + 1_000), NothingHeld::Unknown);
        assert!(matches!(
            states.answer(&format!("s{}", MAX_SHOWN + 9), NOW + 1_000),
            NothingHeld::Waiting { .. }
        ));
        assert_eq!(states.answer("mine", NOW + 1_000), NothingHeld::Declined);
    }
}
