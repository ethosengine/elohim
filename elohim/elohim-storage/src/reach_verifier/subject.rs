//! Subjects whose bytes or metadata may cross an egress plane.

/// The object a reach decision covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// A projected content row and its declared reach.
    Row {
        /// Stable content identifier.
        id: String,
        /// Declared reach word from the projection.
        reach: String,
    },
    /// Content-addressed bytes. The adapter resolves every stored rendering.
    Address {
        /// Canonical or legacy address supplied at the boundary.
        hash: String,
    },
    /// An EPR atom whose signer may retain author-only standing.
    Atom {
        /// Atom CID.
        cid: String,
        /// Reach declared by the atom envelope.
        reach: String,
        /// Canonical agent identifier of the atom signer.
        signer: String,
    },
    /// An eprfs tree root.
    Root {
        /// Root CID.
        cid: String,
    },
    /// An Automerge document inside an application namespace.
    Doc {
        /// Application namespace.
        namespace: String,
        /// Document identifier inside the namespace.
        doc_id: String,
    },
}

impl Subject {
    /// Construct a projected-row subject.
    pub fn row(id: impl Into<String>, reach: impl Into<String>) -> Self {
        Self::Row {
            id: id.into(),
            reach: reach.into(),
        }
    }

    /// Return the subject's stable content identifier when it has one.
    pub fn id(&self) -> &str {
        match self {
            Self::Row { id, .. } => id,
            Self::Address { hash } => hash,
            Self::Atom { cid, .. } | Self::Root { cid } => cid,
            Self::Doc { doc_id, .. } => doc_id,
        }
    }

    /// Return the declared reach carried directly by this subject.
    pub fn reach(&self) -> Option<&str> {
        match self {
            Self::Row { reach, .. } | Self::Atom { reach, .. } => Some(reach),
            Self::Address { .. } | Self::Root { .. } | Self::Doc { .. } => None,
        }
    }

    /// Return the atom signer when author-only standing can apply.
    pub fn signer(&self) -> Option<&str> {
        match self {
            Self::Atom { signer, .. } => Some(signer),
            _ => None,
        }
    }
}
