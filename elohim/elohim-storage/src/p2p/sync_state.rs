//! The sync-state contract: where a receiver is in a publisher's stream, and
//! whether that is the whole of what the publisher has declared.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-08-29-sync-state-contract-design.md`.
//! Four convergence defects measured on 2026-08-29 were one defect: streams
//! with an implicit position, no epoch, and a guessed "caught up". This module
//! is the vocabulary every stream declares against — `epoch` before
//! `position`, position monotone per epoch, caught-up a comparison and never a
//! timer, unknown declared end = not caught up (honest absence, C4).
//!
//! Station 1 (this file): the inventory publisher's sequence carries its boot
//! epoch in the high 32 bits, so a restarted publisher is strictly AHEAD of
//! its old run on the wire and no receiver has to detect the restart. Nothing
//! changes in the message shape — `sequence` is the same `u64`.

/// Seconds since 2026-01-01T00:00:00Z at which epochs are counted. 31 bits of
/// headroom above it = 68 years before `epoch << 32` could touch the sign bit
/// of the `i64` the cursor row stores.
pub const EPOCH_ORIGIN_UNIX_SECS: u64 = 1_767_225_600;

/// The publisher's boot epoch for a process that started at `unix_secs`.
/// Clamped so a clock before the origin still yields a valid (zero) epoch.
pub fn boot_epoch(unix_secs: u64) -> u32 {
    unix_secs
        .saturating_sub(EPOCH_ORIGIN_UNIX_SECS)
        .min(u32::MAX as u64) as u32
}

/// The first sequence of an epoch: counters allocate upward from here.
pub fn epoch_base(epoch: u32) -> u64 {
    (epoch as u64) << 32
}

/// `(epoch, counter)` of a sequence. A pre-epoch publisher (counters from 0)
/// decodes as epoch 0, which is exactly what its `PUBLISHER_RESTART_GAP`
/// fallback assumes.
pub fn split_sequence(sequence: u64) -> (u32, u32) {
    ((sequence >> 32) as u32, (sequence & 0xffff_ffff) as u32)
}

/// One receiver's state in one publisher's stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStreamState {
    /// Publisher epoch the `position` belongs to.
    pub epoch: u32,
    /// Last position applied in order within `epoch`.
    pub position: u32,
    /// The publisher's declared end of this epoch so far, if the receiver has
    /// learned it. `None` is honest absence: never read as "nothing to do".
    pub declared: Option<u32>,
}

impl SyncStreamState {
    /// The contract's one predicate. `None` when the declared end is unknown —
    /// a rollup must publish `null`, never `true`, for that stream.
    pub fn caught_up(&self) -> Option<bool> {
        self.declared.map(|d| self.position >= d)
    }

    /// How a position from the wire relates to this state.
    pub fn classify(&self, sequence: u64) -> PositionClass {
        let (epoch, counter) = split_sequence(sequence);
        if epoch > self.epoch {
            PositionClass::NewerEpoch
        } else if epoch < self.epoch {
            PositionClass::OlderEpoch
        } else if counter <= self.position {
            PositionClass::Replay
        } else if counter == self.position + 1 {
            PositionClass::Next
        } else {
            PositionClass::Ahead
        }
    }
}

/// Where a wire position lands relative to a receiver's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionClass {
    /// A newer publisher epoch: supersedes everything held for the old one.
    NewerEpoch,
    /// An older epoch: a replay of a run that is over.
    OlderEpoch,
    /// Same epoch, at or below the cursor.
    Replay,
    /// Same epoch, exactly the next position.
    Next,
    /// Same epoch, past the next position: hold until contiguous.
    Ahead,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_restarted_publisher_is_strictly_ahead_of_its_old_run() {
        let old_epoch = boot_epoch(EPOCH_ORIGIN_UNIX_SECS + 1_000);
        let new_epoch = boot_epoch(EPOCH_ORIGIN_UNIX_SECS + 1_001);
        // The old run published 5 million pages; the new run's FIRST page
        // still sorts above every one of them.
        let old_high = epoch_base(old_epoch) + 5_000_000;
        let new_first = epoch_base(new_epoch) + 1;
        assert!(new_first > old_high);
        assert_eq!(split_sequence(new_first), (new_epoch, 1));
    }

    #[test]
    fn a_pre_epoch_publisher_decodes_as_epoch_zero() {
        assert_eq!(split_sequence(937), (0, 937));
        assert_eq!(boot_epoch(EPOCH_ORIGIN_UNIX_SECS - 5), 0);
    }

    #[test]
    fn the_cursor_stays_inside_i64_for_68_years() {
        let far = boot_epoch(EPOCH_ORIGIN_UNIX_SECS + 68 * 365 * 24 * 3600);
        assert!(epoch_base(far) + u32::MAX as u64 <= i64::MAX as u64);
    }

    #[test]
    fn caught_up_is_a_comparison_and_unknown_is_not_true() {
        let mut s = SyncStreamState {
            epoch: 7,
            position: 76,
            declared: None,
        };
        assert_eq!(
            s.caught_up(),
            None,
            "unknown declared end is honest absence"
        );
        s.declared = Some(77);
        assert_eq!(s.caught_up(), Some(false));
        s.position = 77;
        assert_eq!(s.caught_up(), Some(true));
    }

    #[test]
    fn positions_classify_by_epoch_before_counter() {
        let s = SyncStreamState {
            epoch: 7,
            position: 10,
            declared: None,
        };
        assert_eq!(s.classify(epoch_base(8) + 1), PositionClass::NewerEpoch);
        assert_eq!(s.classify(epoch_base(6) + 999), PositionClass::OlderEpoch);
        assert_eq!(s.classify(epoch_base(7) + 10), PositionClass::Replay);
        assert_eq!(s.classify(epoch_base(7) + 11), PositionClass::Next);
        assert_eq!(s.classify(epoch_base(7) + 12), PositionClass::Ahead);
    }
}
