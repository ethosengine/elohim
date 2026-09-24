//! READER — who an answer is shaped for.
//!
//! A reader is an agent (named by its CID when one is claimed; `None` for an anonymous visitor)
//! reading at a lens tier (`minimal`, `standard`, `whole`, or a tier a recipe's own lens table
//! declares). It is the input a reach gate and a lens cut read; it is never on the wire — an
//! answer prints the lens it resolved, not the reader it was asked by.

/// The agent an answer is for, and the tier it reads at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reader {
    /// The reader's agent CID; `None` when no agent is claimed.
    pub agent_cid: Option<String>,
    /// The lens tier the reader asked for (resolved against a declared lens table by the caller).
    pub tier: String,
}
