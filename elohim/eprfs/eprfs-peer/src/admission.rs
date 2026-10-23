//! Which authenticated iroh node ids this device admits, read from the roster in its own
//! checkout.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io;
use std::path::Path;

use elohim_epr_rea::{ParticipantRow, Roster, SignatureVerifier};
use iroh::NodeId;

use crate::error::PeerError;

/// The node ids this device admits for one lineage: node id (64 lowercase hex) → device
/// `did:key`, as [`elohim_epr_rea::admitted_endpoints`] reads the roster.
///
/// Default deny: a node id no valid `PeerEndpoint` row of `chain_root`'s lineage names is not
/// admitted, and neither is a conflicting, equivocating, superseded or contested one. Admission
/// is pure over the rows given — never asked over the network — so a Contest written in this
/// checkout takes effect at the next [`Self::reload`], with no pull (spec §3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    chain_root: String,
    admitted: BTreeMap<String, String>,
}

impl Admission {
    /// Admission for the lineage rooted at `chain_root`, from `rows` checked with `verifier`
    /// (epr-cli supplies the device-key verifier; this crate never decodes a did:key).
    pub fn from_roster(
        rows: &[ParticipantRow],
        chain_root: &str,
        verifier: &dyn SignatureVerifier,
    ) -> Admission {
        Admission {
            chain_root: chain_root.to_string(),
            admitted: elohim_epr_rea::admitted_endpoints(rows, chain_root, verifier),
        }
    }

    /// The lineage this admission is for.
    pub fn chain_root(&self) -> &str {
        &self.chain_root
    }

    /// The device `did:key` an authenticated `node` belongs to, or `None` when it is not
    /// admitted.
    ///
    /// Pass only a node id iroh authenticated (`Connection::remote_node_id`): the handshake
    /// proves the peer holds that node's secret key, so admission here is the whole possession
    /// proof.
    pub fn admits(&self, node: &NodeId) -> Option<&str> {
        self.admitted.get(&node_id_hex(node)).map(String::as_str)
    }

    /// Whether `device` (a `did:key`) currently has an admitted node id. The tier gate asks this
    /// on every request, so a [`Self::reload`] that drops a device takes effect at its next
    /// request, even on a connection admitted earlier.
    pub fn admits_device(&self, device: &str) -> bool {
        self.admitted.values().any(|admitted| admitted == device)
    }

    /// Every device (a `did:key`) admitted now, this device included.
    pub fn devices(&self) -> BTreeSet<&str> {
        self.admitted.values().map(String::as_str).collect()
    }

    /// The node id `device` (a `did:key`) is admitted on now, to dial it; `None` when the device
    /// has no admitted endpoint.
    pub fn node_for(&self, device: &str) -> Option<NodeId> {
        self.admitted
            .iter()
            .find(|(_, admitted)| admitted.as_str() == device)
            .and_then(|(node, _)| node.parse().ok())
    }

    /// Every node id admitted now.
    pub fn nodes(&self) -> Vec<NodeId> {
        self.admitted
            .keys()
            .filter_map(|node| node.parse().ok())
            .collect()
    }

    /// Re-read admission for the same lineage from the current `rows`. Rotations and
    /// revocations in `rows` take effect here; nothing earlier is kept.
    pub fn reload(&mut self, rows: &[ParticipantRow], verifier: &dyn SignatureVerifier) {
        self.admitted = elohim_epr_rea::admitted_endpoints(rows, &self.chain_root, verifier);
    }
}

/// A node id in the roster's canonical form: its 32 bytes as lowercase hex. Computed from the
/// bytes rather than iroh's `Display`, so the comparison cannot drift with a printing change.
fn node_id_hex(node: &NodeId) -> String {
    node.as_bytes()
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            // Writing to a String cannot fail.
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// The rows of one handle's roster file, `<…>/.eprfs/status/participants/<handle>.jsonl`, read
/// from this device's own checkout with epr-rea's verifying reader ([`Roster::from_jsonl`]):
/// every stored CID is recomputed and every row shape-checked. The handle is the file stem.
///
/// An absent file is an empty roster, which admits nobody.
///
/// # Errors
///
/// [`PeerError::RosterIo`] when the file cannot be read; [`PeerError::RosterInvalid`] when its
/// name is not `<handle>.jsonl` or a line does not verify.
pub fn roster_rows_from_file(path: &Path) -> Result<Vec<ParticipantRow>, PeerError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(PeerError::RosterIo {
                path: path.to_path_buf(),
                source,
            })
        }
    };
    let invalid = |reason: String| PeerError::RosterInvalid {
        path: path.to_path_buf(),
        reason,
    };
    let handle = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|_| path.extension().is_some_and(|ext| ext == "jsonl"))
        .ok_or_else(|| invalid("the file is not named <handle>.jsonl".into()))?;
    let roster = Roster::from_jsonl(handle, &text).map_err(|e| invalid(e.to_string()))?;
    Ok(roster.rows().iter().map(|(_, row)| row.clone()).collect())
}
