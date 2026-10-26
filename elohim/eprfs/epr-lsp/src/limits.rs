//! Transport bounds (design D9), in one place.
//!
//! No single client can make the daemon allocate without bound or stall the loop that reads the
//! server: a client over its frame, queue or pending-request bound is answered or disconnected,
//! and its editor's restart-on-crash reattaches it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// The largest LSP frame body accepted from a client or the server. A larger declared length
/// closes that connection with a logged reason.
pub const MAX_FRAME: usize = 64 << 20;

/// The most messages queued to one client before it is disconnected as too slow.
pub const CLIENT_QUEUE_MESSAGES: usize = 1024;

/// The most bytes queued to one client before it is disconnected as too slow.
pub const CLIENT_QUEUE_BYTES: usize = 32 << 20;

/// The idle reap and a pin change wait this long for the server's `shutdown` reply …
pub const SHUTDOWN_WAIT: Duration = Duration::from_secs(5);

/// … then send `exit` and wait this long before killing the child (SIGKILL).
pub const EXIT_WAIT: Duration = Duration::from_secs(2);

/// What a client is told when it is disconnected for falling behind.
pub const TOO_SLOW: &str = "epr lsp: too slow; reconnecting";

/// One client's outbound queue accounting, shared by the loop (which admits) and the client's
/// writer (which retires). The loop is the only producer, so admission never races itself.
#[derive(Debug, Default)]
pub struct Outbox {
    messages: AtomicUsize,
    bytes: AtomicUsize,
}

impl Outbox {
    /// Count one message of `len` bytes into the queue, or refuse it when either bound would be
    /// exceeded (the client is then too slow).
    pub fn admit(&self, len: usize) -> bool {
        let messages = self.messages.load(Ordering::Acquire);
        let bytes = self.bytes.load(Ordering::Acquire);
        if messages + 1 > CLIENT_QUEUE_MESSAGES || bytes + len > CLIENT_QUEUE_BYTES {
            return false;
        }
        self.messages.fetch_add(1, Ordering::AcqRel);
        self.bytes.fetch_add(len, Ordering::AcqRel);
        true
    }

    /// Retire one written message of `len` bytes.
    pub fn retire(&self, len: usize) {
        self.messages.fetch_sub(1, Ordering::AcqRel);
        self.bytes.fetch_sub(len, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outbox_refuses_past_either_bound_and_recovers_as_it_drains() {
        let outbox = Outbox::default();
        for _ in 0..CLIENT_QUEUE_MESSAGES {
            assert!(outbox.admit(1));
        }
        assert!(!outbox.admit(1), "message bound");
        outbox.retire(1);
        assert!(outbox.admit(1));

        let outbox = Outbox::default();
        assert!(outbox.admit(CLIENT_QUEUE_BYTES - 10));
        assert!(!outbox.admit(11), "byte bound");
        assert!(outbox.admit(10));
    }
}
