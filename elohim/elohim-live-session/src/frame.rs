//! The presence frame. Encoding (MessagePack with a length prefix) belongs to the transport
//! codec; this module fixes the shape and the version rule: the format version is the first
//! field, and a frame with an unknown version is dropped.
//!
//! Frames carry sender, session and count but no signature (plan section 3): the two ends of a
//! direct stream are already authenticated to each other.

use crate::session_id::SessionId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The only format version this build understands.
pub const FORMAT_VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    /// Always first on the wire.
    pub version: u8,
    /// The sending machine's agent key, as the admission step established it.
    pub sender: String,
    pub session: SessionId,
    /// Strictly increasing per sender within a session; starts at 1.
    pub count: u64,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Body {
    /// "I am here", with this machine's small in-the-moment values. Also the renewal.
    Present { values: BTreeMap<String, String> },
    /// A signalled clean exit: shown as left, not unknown.
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    UnknownVersion(u8),
}

impl Frame {
    pub fn new(sender: impl Into<String>, session: SessionId, count: u64, body: Body) -> Self {
        Frame {
            version: FORMAT_VERSION,
            sender: sender.into(),
            session,
            count,
            body,
        }
    }

    /// Refuse a frame whose version this build does not know.
    pub fn check_version(&self) -> Result<(), FrameError> {
        if self.version == FORMAT_VERSION {
            Ok(())
        } else {
            Err(FrameError::UnknownVersion(self.version))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_version_refused() {
        let mut f = Frame::new("a", SessionId::for_document(b"d"), 1, Body::Leave);
        assert_eq!(f.check_version(), Ok(()));
        f.version = 2;
        assert_eq!(f.check_version(), Err(FrameError::UnknownVersion(2)));
    }
}
