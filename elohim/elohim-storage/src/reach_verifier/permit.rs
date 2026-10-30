//! Permit typestate minted only by the reach verifier.

use super::Subject;

/// Proof that one exact subject may cross one judged egress plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permit {
    subject: Subject,
}

impl Permit {
    /// Mint a permit after a verifier rule grants the subject.
    pub(super) fn new(subject: Subject) -> Self {
        Self { subject }
    }

    /// The exact subject the permit authorizes a serving function to read.
    pub fn subject(&self) -> &Subject {
        &self.subject
    }
}
