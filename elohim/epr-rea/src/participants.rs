//! The participant roster — which devices speak for a human handle, in rows only the devices
//! themselves can write.
//!
//! **A handle is a name; the chain root is the identity.** `human:<handle>` stays the display
//! grammar. The durable identity is the `did:key` of the first device the handle was established
//! on — the genesis of a key lineage, the repository node's rehearsal of the identity-head chain
//! root. A second device mints its OWN key and joins through a [`ParticipantRow::Binding`] both
//! devices sign; a key is never copied, so every device stays distinguishable.
//!
//! **Only public material.** A row carries handles, did:keys, CIDs, nonces and signatures — never
//! an email, a git name or a workspace namespace. The roster is meant to be tracked and pushed;
//! nothing in it may say more about a person than the person chose to say.
//!
//! **Standing, not a gate.** [`standing_human`] answers "which human does this device speak for,
//! right now?" for attribution. Nothing here refuses an act for want of standing — the floor stays
//! honor-system; a device signature only makes a claim portable and checkable.
//!
//! **Pure, with one seam.** Verification is the caller's ([`SignatureVerifier`]): this crate never
//! decodes a did:key and never holds a key. The file-backed [`SidecarRoster`] sits behind the
//! default-on `sidecar` feature like every other `std::fs` surface here.

use std::collections::{BTreeMap, BTreeSet, HashMap};
#[cfg(feature = "sidecar")]
use std::path::{Path, PathBuf};

use cid::Cid;
use serde::{Deserialize, Serialize};

use crate::actor::{
    hex_decode, parse_participant_ref, validate_did_key_shape, ActorRecord, ParticipantRef,
    SignatureVerifier, ED25519_SIGNATURE_LEN,
};
use crate::error::{FabricError, Result};
use crate::model::{atom_cid, AgentRef};

/// The domain tag a roster row's signatures cover, ahead of the CID of the row's unsigned body.
const ROW_SIGNING_DOMAIN: &str = "elohim:participant-roster-row:v1:";

/// An iroh NodeId as iroh prints it: the 32-byte ed25519 public key in lowercase hex.
const NODE_ID_HEX_LEN: usize = 64;

/// One appended roster row. Append-only; a correction is a new row (a `Contest`), never an edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ParticipantRow {
    /// The handle's genesis on its first device: the founding record (a witness of the human by
    /// a present agent, or the human's own claim) and the device that signed it. `chain_root` IS
    /// that device's did:key, so `signer` must equal it.
    Genesis {
        handle: String,
        chain_root: String,
        /// CID of the founding [`ActorRecord`] (a `Witness` or a human `Claim`).
        record_cid: String,
        signer: String,
        /// Lowercase hex of the signer's signature over [`ParticipantRow::signing_message`].
        signature: String,
    },
    /// A second device joining the lineage: `controller` (the new device, B) authorized by
    /// `authorized_by` (a device already in the roster, A). BOTH halves sign the same body —
    /// A's authorization and B's countersignature — the two-halves shape of storage's
    /// agent/peer binding proof. One half is no binding.
    Binding {
        handle: String,
        chain_root: String,
        controller: String,
        authorized_by: String,
        /// `authorized_by`'s signature (A).
        sig_a: String,
        /// `controller`'s signature (B).
        sig_b: String,
        /// Minted by B at enrolment, so an authorization cannot be replayed onto another request.
        nonce: String,
    },
    /// An adverse attestation against an actor record (a witness, a claim, or the signature over
    /// one): the record stops conferring standing. `by` is the contesting device's did:key, the
    /// signer of this row; a contest may itself be contested.
    Contest {
        handle: String,
        /// CID of the contested record — an actor record, its `Signed` record, or another
        /// `Contest` row.
        target_cid: String,
        by: String,
        /// One line the contester stands behind. A contest with no reason is refused.
        basis: String,
        signature: String,
    },
    /// A device's peer endpoint: `device`, a member of `chain_root`'s lineage, answers on the
    /// iroh NodeId `node_id`. Signed by `device` alone — a device names its own endpoint, nobody
    /// names it for the device. The verifying row with the device's highest `seq` supersedes
    /// its others, which is how a key rotates; line order never decides, so replaying an old
    /// row cannot undo a rotation. [`admitted_endpoints`] is the read.
    PeerEndpoint {
        handle: String,
        chain_root: String,
        /// The device the endpoint belongs to, `did:key:…`, and the row's signer.
        device: String,
        /// The iroh NodeId in its canonical printed form (64 lowercase hex characters).
        node_id: String,
        /// The device's own sequence over its endpoint rows, under its signature: the highest
        /// verifying `seq` is the device's endpoint.
        seq: u64,
        /// Minted by the device per row, so two rows naming the same node id stay distinct.
        nonce: String,
        /// `device`'s signature over [`ParticipantRow::signing_message`], lowercase hex.
        signature: String,
    },
}

/// The unsigned body of each row kind — the thing its signatures are over. Kept private and
/// hashed with the crate's one canonical codec, so a signature covers every field except the
/// signatures themselves.
#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum RowBody<'a> {
    Genesis {
        handle: &'a str,
        chain_root: &'a str,
        record_cid: &'a str,
        signer: &'a str,
    },
    Binding {
        handle: &'a str,
        chain_root: &'a str,
        controller: &'a str,
        authorized_by: &'a str,
        nonce: &'a str,
    },
    Contest {
        handle: &'a str,
        target_cid: &'a str,
        by: &'a str,
        basis: &'a str,
    },
    PeerEndpoint {
        handle: &'a str,
        chain_root: &'a str,
        device: &'a str,
        node_id: &'a str,
        seq: u64,
        nonce: &'a str,
    },
}

impl ParticipantRow {
    /// The handle this row speaks about.
    pub fn handle(&self) -> &str {
        match self {
            ParticipantRow::Genesis { handle, .. }
            | ParticipantRow::Binding { handle, .. }
            | ParticipantRow::Contest { handle, .. }
            | ParticipantRow::PeerEndpoint { handle, .. } => handle,
        }
    }

    /// The lineage a row names, when its kind names one (a contest names a record, not a
    /// lineage).
    fn named_chain_root(&self) -> Option<&str> {
        match self {
            ParticipantRow::Genesis { chain_root, .. }
            | ParticipantRow::Binding { chain_root, .. }
            | ParticipantRow::PeerEndpoint { chain_root, .. } => Some(chain_root),
            ParticipantRow::Contest { .. } => None,
        }
    }

    /// The row's identity: the atom CID of the whole row, signatures included.
    pub fn cid(&self) -> Result<Cid> {
        atom_cid(self)
    }

    /// The exact bytes every signature on this row covers: the domain tag followed by the CID of
    /// the row's unsigned body. For a binding, A and B sign these same bytes.
    pub fn signing_message(&self) -> Result<Vec<u8>> {
        let body = match self {
            ParticipantRow::Genesis {
                handle,
                chain_root,
                record_cid,
                signer,
                ..
            } => RowBody::Genesis {
                handle,
                chain_root,
                record_cid,
                signer,
            },
            ParticipantRow::Binding {
                handle,
                chain_root,
                controller,
                authorized_by,
                nonce,
                ..
            } => RowBody::Binding {
                handle,
                chain_root,
                controller,
                authorized_by,
                nonce,
            },
            ParticipantRow::Contest {
                handle,
                target_cid,
                by,
                basis,
                ..
            } => RowBody::Contest {
                handle,
                target_cid,
                by,
                basis,
            },
            ParticipantRow::PeerEndpoint {
                handle,
                chain_root,
                device,
                node_id,
                seq,
                nonce,
                ..
            } => RowBody::PeerEndpoint {
                handle,
                chain_root,
                device,
                node_id,
                seq: *seq,
                nonce,
            },
        };
        Ok(format!("{ROW_SIGNING_DOMAIN}{}", atom_cid(&body)?).into_bytes())
    }

    /// Shape check, no keys needed: a valid handle, did:key-shaped keys, present 64-byte hex
    /// signatures, a genesis whose signer is its chain root, a nonce, a contest basis. Every row a
    /// [`Roster`] accepts has passed this.
    pub fn validate_shape(&self) -> Result<()> {
        validate_handle(self.handle())?;
        match self {
            ParticipantRow::Genesis {
                chain_root,
                record_cid,
                signer,
                signature,
                ..
            } => {
                validate_did_key_shape(chain_root)?;
                validate_did_key_shape(signer)?;
                if signer != chain_root {
                    return Err(FabricError::Decode(format!(
                        "a genesis row is signed by its chain root: signer `{signer}` is not \
                         chain_root `{chain_root}`"
                    )));
                }
                validate_cid(record_cid)?;
                validate_signature("genesis signature", signature)
            }
            ParticipantRow::Binding {
                chain_root,
                controller,
                authorized_by,
                sig_a,
                sig_b,
                nonce,
                ..
            } => {
                validate_did_key_shape(chain_root)?;
                validate_did_key_shape(controller)?;
                validate_did_key_shape(authorized_by)?;
                if controller == authorized_by {
                    return Err(FabricError::Decode(
                        "a binding joins a NEW device: controller and authorized_by are the same \
                         key"
                        .into(),
                    ));
                }
                if nonce.trim().is_empty() {
                    return Err(FabricError::Decode(
                        "a binding needs the enrolling device's nonce".into(),
                    ));
                }
                validate_signature("binding half A (authorized_by)", sig_a)?;
                validate_signature("binding half B (controller)", sig_b)
            }
            ParticipantRow::Contest {
                target_cid,
                by,
                basis,
                signature,
                ..
            } => {
                validate_cid(target_cid)?;
                validate_did_key_shape(by)?;
                if basis.trim().is_empty() {
                    return Err(FabricError::Decode(
                        "a contest needs a basis — one line the contester stands behind".into(),
                    ));
                }
                validate_signature("contest signature", signature)
            }
            ParticipantRow::PeerEndpoint {
                chain_root,
                device,
                node_id,
                nonce,
                signature,
                ..
            } => {
                validate_did_key_shape(chain_root)?;
                validate_did_key_shape(device)?;
                validate_node_id(node_id)?;
                if nonce.trim().is_empty() {
                    return Err(FabricError::Decode(
                        "a peer endpoint needs the device's nonce".into(),
                    ));
                }
                validate_signature("peer endpoint signature", signature)
            }
        }
    }
}

/// Verify a binding row: BOTH halves present and valid — A's (`authorized_by`) and B's
/// (`controller`) signatures over the same body. One half is never a binding: A alone would let
/// an existing device conscript any key, B alone would let any key join itself.
///
/// Whether `authorized_by` is itself in the roster is the [`Roster`]'s question, not this one.
pub fn verify_binding(row: &ParticipantRow, verifier: &dyn SignatureVerifier) -> Result<()> {
    let ParticipantRow::Binding {
        controller,
        authorized_by,
        sig_a,
        sig_b,
        ..
    } = row
    else {
        return Err(FabricError::Decode("not a binding row".into()));
    };
    row.validate_shape()?;
    let message = row.signing_message()?;
    if !signature_verifies(verifier, authorized_by, &message, sig_a) {
        return Err(FabricError::Decode(format!(
            "binding half A does not verify as `{authorized_by}`'s signature"
        )));
    }
    if !signature_verifies(verifier, controller, &message, sig_b) {
        return Err(FabricError::Decode(format!(
            "binding half B does not verify as `{controller}`'s signature"
        )));
    }
    Ok(())
}

/// Verify a peer endpoint row: its shape, and `device`'s own signature over its body (which
/// covers `seq`). Whether the device stands on the roster is [`endpoint_admission`]'s question.
pub fn verify_peer_endpoint(row: &ParticipantRow, verifier: &dyn SignatureVerifier) -> Result<()> {
    let ParticipantRow::PeerEndpoint {
        device, signature, ..
    } = row
    else {
        return Err(FabricError::Decode("not a peer endpoint row".into()));
    };
    row.validate_shape()?;
    if !row_signature_verifies(verifier, row, device, signature) {
        return Err(FabricError::Decode(format!(
            "peer endpoint does not verify as `{device}`'s signature"
        )));
    }
    Ok(())
}

/// One admitted endpoint: the device it belongs to and the row it stands on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedEndpoint {
    /// The device, `did:key:…`.
    pub device: String,
    /// CID of the verified `PeerEndpoint` row that admits it.
    pub row_cid: String,
}

/// What a roster admits as peer endpoints for one lineage — see [`endpoint_admission`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EndpointAdmission {
    /// Admitted iroh node id → the device it belongs to and the row that admits it.
    pub admitted: BTreeMap<String, AdmittedEndpoint>,
    /// Node ids that two or more standing devices claim, with every claimant. None of them is
    /// admitted on that id: one node id answers for one device or for nobody.
    pub conflicts: BTreeMap<String, BTreeSet<String>>,
    /// Standing devices whose highest-`seq` verifying rows name different node ids, with those
    /// ids. Such a device admits nothing — no older row stands in for it.
    pub equivocations: BTreeMap<String, BTreeSet<String>>,
}

/// The peer endpoints `rows` admit for the lineage rooted at `chain_root`: iroh node id →
/// device did:key. The node-id → device projection of [`endpoint_admission`]'s `admitted`.
pub fn admitted_endpoints(
    rows: &[ParticipantRow],
    chain_root: &str,
    verifier: &dyn SignatureVerifier,
) -> BTreeMap<String, String> {
    endpoint_admission(rows, chain_root, verifier)
        .admitted
        .into_iter()
        .map(|(node_id, endpoint)| (node_id, endpoint.device))
        .collect()
}

/// Which iroh node ids the roster `rows` admit for the lineage rooted at `chain_root`, and
/// which it refuses. Pure over the rows: each device reads admission from the roster in its own
/// checkout, never over the network.
///
/// A device's endpoint is decided by its verifying [`ParticipantRow::PeerEndpoint`] rows
/// ([`verify_peer_endpoint`]) with the HIGHEST `seq` — never by line order, so re-appending or
/// reordering an older validly signed row cannot roll a rotation back. It is admitted only when
/// all of these hold:
///
/// - the row names `chain_root`, and the lineage has a verifying genesis with that root —
///   rows naming any other root (a rival genesis, an endpoint filed under another lineage) are
///   ignored, so a pinned root cannot be displaced by line order;
/// - the device stands by the roster ([`Roster::device_standing_row`]): it is the chain root
///   and neither its genesis row nor the founding record is effectively contested, or it is
///   the controller of a both-halves binding authorized from inside the lineage and that
///   binding is not effectively contested;
/// - every row at the device's highest `seq` names the same node id — otherwise the device is
///   listed in [`EndpointAdmission::equivocations`] and admits nothing;
/// - none of those rows is effectively contested. A contested highest row does NOT revive a
///   lower one: a superseded key stays superseded;
/// - no other standing device's endpoint claims the same node id. Both claimants are refused
///   and reported in [`EndpointAdmission::conflicts`].
pub fn endpoint_admission(
    rows: &[ParticipantRow],
    chain_root: &str,
    verifier: &dyn SignatureVerifier,
) -> EndpointAdmission {
    let mut admission = EndpointAdmission::default();
    let Some(roster) = lineage_roster(rows, chain_root, verifier) else {
        return admission;
    };
    let contested = roster.contested(verifier);

    // Per device: its highest verifying seq, and every row at it.
    let mut highest: BTreeMap<&str, HighestEndpointRows<'_>> = BTreeMap::new();
    for (cid, row) in roster.rows() {
        let ParticipantRow::PeerEndpoint {
            device,
            node_id,
            seq,
            ..
        } = row
        else {
            continue;
        };
        if verify_peer_endpoint(row, verifier).is_err() {
            continue;
        }
        let entry = highest.entry(device).or_insert(HighestEndpointRows {
            seq: *seq,
            rows: Vec::new(),
        });
        if *seq > entry.seq {
            *entry = HighestEndpointRows {
                seq: *seq,
                rows: Vec::new(),
            };
        }
        if *seq == entry.seq {
            entry.rows.push((cid.to_string(), node_id));
        }
    }

    let mut claims: BTreeMap<String, Vec<AdmittedEndpoint>> = BTreeMap::new();
    for (
        device,
        HighestEndpointRows {
            rows: at_highest, ..
        },
    ) in highest
    {
        if roster.device_standing_row(device, verifier).is_none() {
            continue;
        }
        let named: BTreeSet<String> = at_highest
            .iter()
            .map(|(_, node_id)| (*node_id).to_string())
            .collect();
        if named.len() > 1 {
            admission.equivocations.insert(device.to_string(), named);
            continue;
        }
        if at_highest
            .iter()
            .any(|(row_cid, _)| contested.contains(row_cid))
        {
            continue;
        }
        // Every row at the highest seq names the one node id; the first in line order stands.
        let Some((row_cid, node_id)) = at_highest.into_iter().next() else {
            continue;
        };
        claims
            .entry(node_id.to_string())
            .or_default()
            .push(AdmittedEndpoint {
                device: device.to_string(),
                row_cid,
            });
    }
    for (node_id, mut claimants) in claims {
        if claimants.len() == 1 {
            if let Some(endpoint) = claimants.pop() {
                admission.admitted.insert(node_id, endpoint);
            }
        } else {
            admission.conflicts.insert(
                node_id,
                claimants
                    .into_iter()
                    .map(|endpoint| endpoint.device)
                    .collect(),
            );
        }
    }
    admission
}

/// One device's verifying endpoint rows at its highest `seq`: `(row CID, node id)` each.
struct HighestEndpointRows<'a> {
    seq: u64,
    rows: Vec<(String, &'a str)>,
}

/// The roster of the lineage rooted at `chain_root`, built from `rows`: the handle of that
/// root's first verifying genesis, and every shape-valid row about that handle that names this
/// root or no root at all (a contest). `None` when no genesis with this root verifies.
fn lineage_roster(
    rows: &[ParticipantRow],
    chain_root: &str,
    verifier: &dyn SignatureVerifier,
) -> Option<Roster> {
    let handle = rows.iter().find_map(|row| match row {
        ParticipantRow::Genesis {
            handle,
            chain_root: root,
            signer,
            signature,
            ..
        } if root == chain_root
            && row.validate_shape().is_ok()
            && row_signature_verifies(verifier, row, signer, signature) =>
        {
            Some(handle.as_str())
        }
        _ => None,
    })?;
    let mut roster = Roster::new(handle).ok()?;
    let in_lineage = rows.iter().filter(|row| {
        row.handle() == handle
            && row.named_chain_root().is_none_or(|root| root == chain_root)
            // A row that fails its shape check counts for nothing.
            && row.validate_shape().is_ok()
    });
    for row in in_lineage {
        // Cannot fail after the filter above; if it ever does, admit nobody rather than guess.
        roster.append(row.clone()).ok()?;
    }
    (roster.chain_root(verifier).as_deref() == Some(chain_root)).then_some(roster)
}

/// One handle's roster: its rows in append order, each with its CID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roster {
    handle: String,
    rows: Vec<(Cid, ParticipantRow)>,
}

/// One roster line: the row and its CID, re-verified on read.
#[derive(Serialize, Deserialize)]
struct RosterLine {
    cid: String,
    row: ParticipantRow,
}

impl Roster {
    /// An empty roster for `handle` (the `<handle>` of `human:<handle>`).
    pub fn new(handle: &str) -> Result<Self> {
        validate_handle(handle)?;
        Ok(Self {
            handle: handle.to_string(),
            rows: Vec::new(),
        })
    }

    pub fn handle(&self) -> &str {
        &self.handle
    }

    pub fn rows(&self) -> &[(Cid, ParticipantRow)] {
        &self.rows
    }

    /// Append a row after its shape check; refuses a row about another handle. Returns its CID.
    pub fn append(&mut self, row: ParticipantRow) -> Result<Cid> {
        row.validate_shape()?;
        if row.handle() != self.handle {
            return Err(FabricError::Decode(format!(
                "row is about `{}` but this is `{}`'s roster",
                row.handle(),
                self.handle
            )));
        }
        let cid = row.cid()?;
        self.rows.push((cid, row));
        Ok(cid)
    }

    /// The JSONL line for a row, exactly as [`Self::from_jsonl`] reads it back.
    pub fn line_for(row: &ParticipantRow) -> Result<String> {
        serde_json::to_string(&RosterLine {
            cid: row.cid()?.to_string(),
            row: row.clone(),
        })
        .map_err(|e| FabricError::Encode(e.to_string()))
    }

    /// Read a roster from its JSONL text. Every stored CID is recomputed — a line whose payload
    /// no longer matches its CID is an integrity error, never silent drift — and every row
    /// passes the same shape check [`Self::append`] applies.
    pub fn from_jsonl(handle: &str, text: &str) -> Result<Self> {
        let mut roster = Self::new(handle)?;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let parsed: RosterLine =
                serde_json::from_str(line).map_err(|e| FabricError::Decode(e.to_string()))?;
            let computed = parsed.row.cid()?;
            if computed.to_string() != parsed.cid {
                return Err(FabricError::Integrity {
                    stored: parsed.cid,
                    computed: computed.to_string(),
                });
            }
            roster.append(parsed.row)?;
        }
        Ok(roster)
    }

    /// The handle's chain root: the did:key of the first genesis row that verifies. `None` until
    /// the handle has a genesis.
    pub fn chain_root(&self, verifier: &dyn SignatureVerifier) -> Option<String> {
        self.rows.iter().find_map(|(_, row)| match row {
            ParticipantRow::Genesis {
                chain_root,
                signer,
                signature,
                ..
            } if row_signature_verifies(verifier, row, signer, signature) => {
                Some(chain_root.clone())
            }
            _ => None,
        })
    }

    /// The devices (did:keys) that speak for this handle: the chain root, then every controller
    /// of a binding that verifies on both halves, names this chain root, and is authorized by a
    /// device ALREADY a member at that row — append order is the authority order, so no binding
    /// can authorize itself or a key admitted only later. A genesis naming a different chain root
    /// admits nobody: a second device joins by binding, never by founding a rival lineage.
    pub fn members(&self, verifier: &dyn SignatureVerifier) -> BTreeSet<String> {
        let mut members = BTreeSet::new();
        let Some(root) = self.chain_root(verifier) else {
            return members;
        };
        members.insert(root.clone());
        for (_, row) in &self.rows {
            if let ParticipantRow::Binding {
                chain_root,
                controller,
                authorized_by,
                ..
            } = row
            {
                if chain_root == &root
                    && members.contains(authorized_by)
                    && verify_binding(row, verifier).is_ok()
                {
                    members.insert(controller.clone());
                }
            }
        }
        members
    }

    /// The CIDs contested by an EFFECTIVE contest: an ADMISSIBLE contest row — signed by a
    /// device that is a member of this roster, and whose signature verifies — which is not itself
    /// the target of an effective contest. Content addressing makes the contest graph acyclic (a
    /// row can only name CIDs that existed before it), so the recursion terminates.
    ///
    /// Membership is the admissibility rule (ruling R-P14), for contests of contests too: a
    /// contest signed by a throwaway key names nothing, so a stranger can neither void the
    /// human's standing nor void the human's contest by counter-contesting it. A witness agent
    /// signs from the device key, which is a member by genesis or by binding.
    pub fn contested(&self, verifier: &dyn SignatureVerifier) -> BTreeSet<String> {
        let members = self.members(verifier);
        // target CID -> the CIDs of the admissible contest rows naming it.
        let mut contests: HashMap<String, Vec<String>> = HashMap::new();
        for (cid, row) in &self.rows {
            if let ParticipantRow::Contest {
                target_cid,
                by,
                signature,
                ..
            } = row
            {
                if members.contains(by) && row_signature_verifies(verifier, row, by, signature) {
                    contests
                        .entry(target_cid.clone())
                        .or_default()
                        .push(cid.to_string());
                }
            }
        }
        fn effective(
            contest_cid: &str,
            contests: &HashMap<String, Vec<String>>,
            memo: &mut HashMap<String, bool>,
        ) -> bool {
            if let Some(known) = memo.get(contest_cid) {
                return *known;
            }
            let voided = contests
                .get(contest_cid)
                .map(|counters| counters.iter().any(|c| effective(c, contests, memo)))
                .unwrap_or(false);
            memo.insert(contest_cid.to_string(), !voided);
            !voided
        }

        let mut memo = HashMap::new();
        let mut contested: BTreeSet<String> = contests
            .iter()
            .filter(|(_, by_rows)| by_rows.iter().any(|c| effective(c, &contests, &mut memo)))
            .map(|(target, _)| target.clone())
            .collect();

        // The device-binding plane (spec §3.1: a Contest is valid "only when signed by one of the
        // person's admitted devices") is decided by its own grounded labelling over rows, which
        // the general rule above cannot express: membership ignores contests.
        let devices = self.device_rows(verifier, &members);
        let (plane, rejected) = device_plane(&self.rows, &devices, &members, verifier);
        contested.retain(|cid| !plane.contains(cid));
        contested.extend(rejected);
        contested
    }

    /// Each verifying device-standing row of this lineage, by CID, and the device it stands:
    /// the chain root's genesis rows, and every member's bindings under this root.
    fn device_rows(
        &self,
        verifier: &dyn SignatureVerifier,
        members: &BTreeSet<String>,
    ) -> HashMap<String, String> {
        let Some(root) = self.chain_root(verifier) else {
            return HashMap::new();
        };
        self.rows
            .iter()
            .filter_map(|(cid, row)| match row {
                ParticipantRow::Genesis {
                    chain_root,
                    signer,
                    signature,
                    ..
                } if chain_root == &root
                    && row_signature_verifies(verifier, row, signer, signature) =>
                {
                    Some((cid.to_string(), root.clone()))
                }
                ParticipantRow::Binding {
                    chain_root,
                    controller,
                    ..
                } if chain_root == &root
                    && members.contains(controller)
                    && verify_binding(row, verifier).is_ok() =>
                {
                    Some((cid.to_string(), controller.clone()))
                }
                _ => None,
            })
            .collect()
    }

    /// The admissible contest rows (by CID) that are EFFECTIVE and name `target` — the contests a
    /// re-witness of `target` must answer.
    pub fn effective_contests_of(
        &self,
        target: &str,
        verifier: &dyn SignatureVerifier,
    ) -> Vec<String> {
        let contested = self.contested(verifier);
        let members = self.members(verifier);
        self.rows
            .iter()
            .filter_map(|(cid, row)| match row {
                ParticipantRow::Contest {
                    target_cid,
                    by,
                    signature,
                    ..
                } if target_cid == target
                    && members.contains(by)
                    && row_signature_verifies(verifier, row, by, signature)
                    && !contested.contains(&cid.to_string()) =>
                {
                    Some(cid.to_string())
                }
                _ => None,
            })
            .collect()
    }

    /// Whether this roster's verified chain root is `pin` — the root a device recorded the first
    /// time it read this handle's roster (ruling R-P15).
    ///
    /// The chain root is otherwise trust-on-first-use by line order: whoever writes the first
    /// verifying genesis owns the lineage. A device that has already seen the lineage keeps its
    /// own record of it, so a history rewritten to start from another key reads as a DIFFERENT
    /// root on that device — contested, never standing. `false` for a roster with no verified
    /// root at all. The pin itself lives with the device (the caller's); this crate never holds
    /// it.
    pub fn chain_root_matches(&self, pin: &str, verifier: &dyn SignatureVerifier) -> bool {
        self.chain_root(verifier).as_deref() == Some(pin)
    }

    /// Every row `device` stands on that no accepted contest names: the root's verifying genesis
    /// rows, or the device's verifying bindings under this root, by CID in line order. A device
    /// stands while any is accepted, so retiring it means contesting each of them.
    pub fn uncontested_standing_rows(
        &self,
        device: &str,
        verifier: &dyn SignatureVerifier,
    ) -> Vec<String> {
        let members = self.members(verifier);
        if !members.contains(device) {
            return Vec::new();
        }
        let contested = self.contested(verifier);
        self.device_rows(verifier, &members)
            .into_iter()
            .filter(|(cid, owner)| owner == device && !contested.contains(cid))
            .map(|(cid, _)| cid)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// The roster row `signer_did` stands on by the roster ALONE — for a device reading a
    /// checkout that lacks the actor records (a fresh worktree on a witnessed device, ruling
    /// R-P16): the chain root stands on its genesis row, a bound controller on its latest
    /// binding row. `None` when the device is not a member, or when that row (or, for the chain
    /// root, the founding record it names) is effectively contested.
    ///
    /// The genesis row is the chain root's own signature over the founding record's CID, and a
    /// binding row carries the controller's countersignature — each is the device's signed
    /// statement that it speaks for the handle, exactly the evidence a `Signed` actor record is.
    pub fn device_standing_row(
        &self,
        signer_did: &str,
        verifier: &dyn SignatureVerifier,
    ) -> Option<RosterStanding> {
        let members = self.members(verifier);
        if !members.contains(signer_did) {
            return None;
        }
        let root = self.chain_root(verifier)?;
        let contested = self.contested(verifier);
        // Any accepted standing row stands the device (a re-binding restores a retired device,
        // whatever the line order); the latest such binding is the one reported.
        let found = if signer_did == root {
            self.rows.iter().find_map(|(cid, row)| match row {
                ParticipantRow::Genesis {
                    chain_root,
                    record_cid,
                    signer,
                    signature,
                    ..
                } if chain_root == &root
                    && !contested.contains(&cid.to_string())
                    && row_signature_verifies(verifier, row, signer, signature) =>
                {
                    Some(RosterStanding {
                        row_cid: cid.to_string(),
                        record_cid: record_cid.clone(),
                        via: RosterVia::Genesis,
                    })
                }
                _ => None,
            })
        } else {
            self.rows.iter().rev().find_map(|(cid, row)| match row {
                ParticipantRow::Binding {
                    chain_root,
                    controller,
                    ..
                } if chain_root == &root
                    && controller == signer_did
                    && !contested.contains(&cid.to_string())
                    && verify_binding(row, verifier).is_ok() =>
                {
                    Some(RosterStanding {
                        row_cid: cid.to_string(),
                        record_cid: cid.to_string(),
                        via: RosterVia::Binding,
                    })
                }
                _ => None,
            })
        }?;
        if contested.contains(&found.row_cid) || contested.contains(&found.record_cid) {
            return None;
        }
        Some(found)
    }
}

/// The device-binding plane of a roster and the rows of it rejected (spec §3.1). Its rows are
/// every device-standing row (`devices`: genesis rows of the root, members' bindings, each mapped
/// to its device) and every admissible contest (a member's, verifying) of one of them or of
/// another such contest. It is decided by a grounded labelling:
///
/// - a contest attacks the row it names (a standing row, a contest it counters, or its author's
///   own contest it withdraws);
/// - a device stands while ANY of its standing rows is accepted, and a contest stands only while
///   its signer stands: a contest is rejected once every standing row of its signer is;
/// - no self-defence: a contest naming a contest of one of its own signer's standing rows attacks
///   nothing (the retired device cannot counter the contest of itself);
/// - a row is accepted when its attackers are all rejected (and, for a contest, some standing row
///   of its signer is accepted); rejected when an accepted row attacks it (or, for a contest, every
///   standing row of its signer is rejected); to a fixed point. A row still undecided (a pure
///   cycle: mutual contests, a self-contest) is rejected — fail closed.
///
/// Retirement is per row: re-binding (a new binding no contest names), a withdrawal by the
/// contest's author, or a standing member's counter-contest restores the device. The order of
/// the rows never changes the answer. Returns (plane, rejected).
fn device_plane(
    rows: &[(Cid, ParticipantRow)],
    devices: &HashMap<String, String>,
    members: &BTreeSet<String>,
    verifier: &dyn SignatureVerifier,
) -> (BTreeSet<String>, BTreeSet<String>) {
    // Every admissible contest: CID -> (target, signer).
    let admissible: BTreeMap<String, (String, String)> = rows
        .iter()
        .filter_map(|(cid, row)| match row {
            ParticipantRow::Contest {
                target_cid,
                by,
                signature,
                ..
            } if members.contains(by) && row_signature_verifies(verifier, row, by, signature) => {
                Some((cid.to_string(), (target_cid.clone(), by.clone())))
            }
            _ => None,
        })
        .collect();
    // The plane's contests: those naming a standing row, then those naming a plane contest.
    let mut contests: BTreeMap<String, (String, String)> = BTreeMap::new();
    loop {
        let before = contests.len();
        for (cid, (target, by)) in &admissible {
            if !contests.contains_key(cid)
                && (devices.contains_key(target) || contests.contains_key(target))
            {
                contests.insert(cid.clone(), (target.clone(), by.clone()));
            }
        }
        if contests.len() == before {
            break;
        }
    }
    // Each device's standing rows.
    let mut standing: HashMap<&str, Vec<&str>> = HashMap::new();
    for (cid, device) in devices {
        standing
            .entry(device.as_str())
            .or_default()
            .push(cid.as_str());
    }
    let plane: BTreeSet<String> = devices.keys().chain(contests.keys()).cloned().collect();
    let mut attackers: BTreeMap<&str, BTreeSet<&str>> = plane
        .iter()
        .map(|cid| (cid.as_str(), BTreeSet::new()))
        .collect();
    for (cid, (target, by)) in &contests {
        // No self-defence: a contest of a contest that names one of its signer's own rows.
        let defends_itself = contests
            .get(target)
            .is_some_and(|(named, _)| devices.get(named).is_some_and(|device| device == by));
        if defends_itself {
            continue;
        }
        if let Some(set) = attackers.get_mut(target.as_str()) {
            set.insert(cid.as_str());
        }
    }
    let mut accepted: BTreeSet<&str> = BTreeSet::new();
    let mut rejected: BTreeSet<&str> = BTreeSet::new();
    loop {
        let mut changed = false;
        for (&cid, by) in &attackers {
            if accepted.contains(cid) || rejected.contains(cid) {
                continue;
            }
            let signer_rows: &[&str] = contests
                .get(cid)
                .and_then(|(_, signer)| standing.get(signer.as_str()))
                .map_or(&[], Vec::as_slice);
            let is_contest = contests.contains_key(cid);
            let signer_in = !is_contest || signer_rows.iter().any(|r| accepted.contains(r));
            let signer_out = is_contest && signer_rows.iter().all(|r| rejected.contains(r));
            if by.iter().any(|a| accepted.contains(a)) || signer_out {
                rejected.insert(cid);
                changed = true;
            } else if signer_in && by.iter().all(|a| rejected.contains(a)) {
                accepted.insert(cid);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // Undecided rows (pure cycles) are rejected: fail closed.
    let rejected = plane
        .iter()
        .filter(|cid| !accepted.contains(cid.as_str()))
        .cloned()
        .collect();
    (plane, rejected)
}

/// How a device stands by the roster alone — see [`Roster::device_standing_row`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterStanding {
    /// The roster row the device signed: its genesis, or its binding.
    pub row_cid: String,
    /// The record the standing pins: the founding record a genesis names, or the binding row
    /// itself for a bound device.
    pub record_cid: String,
    pub via: RosterVia,
}

/// Which roster row a [`RosterStanding`] rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterVia {
    Genesis,
    Binding,
}

/// Which human `signer_did` (a device) speaks for right now, per this roster and these actor
/// records — or `None`.
///
/// Standing is per DEVICE, not per session: the latest human record — a `Witness` of the handle
/// or the human's own `Claim` of it — that carries a `Signed` record by `signer_did` which
/// verifies, provided `signer_did` is a member of the handle's roster and neither the record nor
/// its signature is the target of an effective `Contest`. A contested record simply stops
/// counting; an earlier uncontested one (the human's own claim beside a contested witness, say)
/// still stands, and a re-witness — a new record — stands again.
///
/// `None` is honest absence — unsigned, unrostered or contested — and never a refusal: the
/// floor stays honor-system, and an unsigned human claim is exactly as valid as it always was.
pub fn standing_human(
    roster: &Roster,
    records: &[ActorRecord],
    signer_did: &str,
    verifier: &dyn SignatureVerifier,
) -> Option<AgentRef> {
    if !roster.members(verifier).contains(signer_did) {
        return None;
    }
    let contested = roster.contested(verifier);

    // Signed-record CID for each record CID this device verifiably signed (latest wins).
    let mut signed_by_device: HashMap<String, String> = HashMap::new();
    for record in records {
        if let ActorRecord::Signed(signature) = record {
            if signature.signer == signer_did && signature.verify(verifier) {
                if let Ok(cid) = record.cid() {
                    signed_by_device.insert(signature.claim_cid.clone(), cid.to_string());
                }
            }
        }
    }

    records.iter().rev().find_map(|record| {
        let handle = match record {
            ActorRecord::Witness(witness) => witness.handle().ok()?,
            ActorRecord::Claim(claim) => match claim.participant().ok()? {
                ParticipantRef::Human { handle } => handle,
                ParticipantRef::Agent { .. } | ParticipantRef::Service { .. } => return None,
            },
            ActorRecord::Signed(_) => return None,
        };
        if handle != roster.handle() {
            return None;
        }
        let record_cid = record.cid().ok()?.to_string();
        let signed_cid = signed_by_device.get(&record_cid)?;
        if contested.contains(&record_cid) || contested.contains(signed_cid) {
            return None;
        }
        Some(AgentRef(format!("human:{handle}")))
    })
}

/// [`standing_human`] on a device that has pinned this handle's chain root (ruling R-P15):
/// `None` — contested, never standing — when the roster's verified root is not `pin`. A `None`
/// pin (the device has never read this roster) defers to [`standing_human`]; recording the pin on
/// that first verified read is the caller's, beside the device key.
pub fn standing_human_pinned(
    roster: &Roster,
    records: &[ActorRecord],
    signer_did: &str,
    verifier: &dyn SignatureVerifier,
    pin: Option<&str>,
) -> Option<AgentRef> {
    if let Some(pin) = pin {
        if !roster.chain_root_matches(pin, verifier) {
            return None;
        }
    }
    standing_human(roster, records, signer_did, verifier)
}

fn validate_handle(handle: &str) -> Result<()> {
    match parse_participant_ref(&format!("human:{handle}"))? {
        ParticipantRef::Human { .. } => Ok(()),
        ParticipantRef::Agent { .. } | ParticipantRef::Service { .. } => Err(FabricError::Decode(
            format!("`{handle}` is not a human handle"),
        )),
    }
}

fn validate_cid(value: &str) -> Result<()> {
    let parsed = Cid::try_from(value)
        .map_err(|e| FabricError::Decode(format!("`{value}` is not a CID: {e}")))?;
    if parsed.to_string() != value {
        return Err(FabricError::Decode(format!(
            "`{value}` is not a CID in its canonical string form"
        )));
    }
    Ok(())
}

/// An iroh NodeId in the one form iroh prints: 64 lowercase hex characters. iroh also parses a
/// base32 spelling, but admission compares strings, so only the printed form is canonical.
fn validate_node_id(value: &str) -> Result<()> {
    let canonical = value.len() == NODE_ID_HEX_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if canonical {
        Ok(())
    } else {
        Err(FabricError::Decode(format!(
            "`{value}` is not an iroh node id: {NODE_ID_HEX_LEN} lowercase hex characters, as \
             iroh prints a NodeId"
        )))
    }
}

fn validate_signature(what: &str, hex: &str) -> Result<()> {
    match hex_decode(hex) {
        Some(bytes) if bytes.len() == ED25519_SIGNATURE_LEN => Ok(()),
        _ if hex.is_empty() => Err(FabricError::Decode(format!(
            "{what} is missing — the row is refused"
        ))),
        _ => Err(FabricError::Decode(format!(
            "{what} is not {ED25519_SIGNATURE_LEN} bytes of lowercase hex"
        ))),
    }
}

fn signature_verifies(
    verifier: &dyn SignatureVerifier,
    signer: &str,
    message: &[u8],
    hex: &str,
) -> bool {
    hex_decode(hex).is_some_and(|bytes| verifier.verify(signer, message, &bytes))
}

fn row_signature_verifies(
    verifier: &dyn SignatureVerifier,
    row: &ParticipantRow,
    signer: &str,
    hex: &str,
) -> bool {
    row.signing_message()
        .is_ok_and(|message| signature_verifies(verifier, signer, &message, hex))
}

/// The tracked, append-only roster file for one handle:
/// `<root>/.eprfs/status/participants/<handle>.jsonl`.
///
/// The same discipline as [`crate::actor::SidecarActorStore`]: one `{cid, row}` line per append,
/// synced, under the log's OS lock; CIDs re-verified on every read. Unlike the actor sidecar it is
/// meant to be COMMITTED — git is the roster's transport at the repository node, which is why a
/// row carries only public material.
#[cfg(feature = "sidecar")]
#[derive(Debug)]
pub struct SidecarRoster {
    handle: String,
    log_path: PathBuf,
}

#[cfg(feature = "sidecar")]
impl SidecarRoster {
    /// Open (creating directories and the log as needed) `handle`'s roster under `root`.
    pub fn open(root: &Path, handle: &str) -> Result<Self> {
        validate_handle(handle)?;
        std::fs::create_dir_all(root.join(".eprfs").join("status").join("participants"))?;
        let log_path = crate::sidecar::open_log(root, &format!("participants/{handle}.jsonl"))?;
        Ok(Self {
            handle: handle.to_string(),
            log_path,
        })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Read and verify the whole roster.
    pub fn read(&self) -> Result<Roster> {
        let log = crate::sidecar::LockedLog::shared(&self.log_path)?;
        Roster::from_jsonl(&self.handle, &log.contents()?)
    }

    /// Append one row under the exclusive lock, after re-verifying every existing line and the
    /// new row's shape. Returns the row's CID.
    pub fn append(&self, row: ParticipantRow) -> Result<Cid> {
        self.transaction()?.append(row)
    }

    /// Hold the roster's exclusive lock across a read, a decision and an append — so "the handle
    /// has no roster yet, so this is its genesis" is decided on the same bytes the genesis is
    /// appended to. The lock releases when the handle drops.
    pub fn transaction(&self) -> Result<RosterTransaction> {
        Ok(RosterTransaction {
            handle: self.handle.clone(),
            log: crate::sidecar::LockedLog::exclusive(&self.log_path)?,
        })
    }
}

/// One handle's roster under its exclusive lock — see [`SidecarRoster::transaction`].
#[cfg(feature = "sidecar")]
pub struct RosterTransaction {
    handle: String,
    log: crate::sidecar::LockedLog,
}

#[cfg(feature = "sidecar")]
impl RosterTransaction {
    /// Read and verify the whole roster, under the held lock.
    pub fn read(&self) -> Result<Roster> {
        Roster::from_jsonl(&self.handle, &self.log.contents()?)
    }

    /// Append one row, re-verifying every existing line and the new row's shape first.
    pub fn append(&mut self, row: ParticipantRow) -> Result<Cid> {
        let mut roster = self.read()?;
        let line = Roster::line_for(&row)?;
        let cid = roster.append(row)?;
        self.log.append(line)?;
        Ok(cid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{
        hex_encode, record_signing_message, ActorClaim, ActorStore, ActorWitness, MemoryActorStore,
        RecordSignature,
    };
    use elohim_epr::proof::{self, AgentKeypair};

    const AT: &str = "2026-09-24T00:00:00Z";
    const AGENT: &str = "agent:orchestrator@claude-fable-5-1";
    const BASIS: &str = "operator of this stewarded device; present in this session";

    /// A test device: a fixed-seed keypair under a fake (but did:key-shaped) name. The verifier
    /// below resolves these names; real did:key decoding is the caller's, never this crate's.
    struct Device {
        did: String,
        key: AgentKeypair,
    }

    fn device(name: &str, seed: u8) -> Device {
        Device {
            did: format!("did:key:z{name}"),
            key: AgentKeypair::from_secret(&[seed; 32]).unwrap(),
        }
    }

    fn verifier(devices: &[&Device]) -> impl Fn(&str, &[u8], &[u8]) -> bool {
        let keys: HashMap<String, [u8; 32]> = devices
            .iter()
            .map(|d| (d.did.clone(), d.key.public_key_bytes()))
            .collect();
        move |did, message, signature| {
            keys.get(did)
                .is_some_and(|pk| proof::verify(pk, message, signature))
        }
    }

    fn sign(device: &Device, message: &[u8]) -> String {
        hex_encode(&proof::sign(&device.key, message))
    }

    fn witness(session: &str) -> ActorWitness {
        ActorWitness::new("human:matthew", AGENT, session, BASIS, AT).unwrap()
    }

    fn signed(record: &ActorRecord, by: &Device) -> ActorRecord {
        let cid = record.cid().unwrap();
        let sig = proof::sign(&by.key, &record_signing_message(&cid.to_string()));
        ActorRecord::Signed(RecordSignature::new(&cid, &by.did, &sig).unwrap())
    }

    fn genesis(founding: &ActorRecord, by: &Device) -> ParticipantRow {
        let mut row = ParticipantRow::Genesis {
            handle: "matthew".into(),
            chain_root: by.did.clone(),
            record_cid: founding.cid().unwrap().to_string(),
            signer: by.did.clone(),
            signature: String::new(),
        };
        let message = row.signing_message().unwrap();
        if let ParticipantRow::Genesis { signature, .. } = &mut row {
            *signature = sign(by, &message);
        }
        row
    }

    fn binding(root: &Device, a: &Device, b: &Device, halves: (bool, bool)) -> ParticipantRow {
        let mut row = ParticipantRow::Binding {
            handle: "matthew".into(),
            chain_root: root.did.clone(),
            controller: b.did.clone(),
            authorized_by: a.did.clone(),
            sig_a: String::new(),
            sig_b: String::new(),
            nonce: "nonce-1".into(),
        };
        let message = row.signing_message().unwrap();
        if let ParticipantRow::Binding { sig_a, sig_b, .. } = &mut row {
            if halves.0 {
                *sig_a = sign(a, &message);
            }
            if halves.1 {
                *sig_b = sign(b, &message);
            }
        }
        row
    }

    fn contest(target: &str, by: &Device) -> ParticipantRow {
        let mut row = ParticipantRow::Contest {
            handle: "matthew".into(),
            target_cid: target.into(),
            by: by.did.clone(),
            basis: "that was not me at the keyboard".into(),
            signature: String::new(),
        };
        let message = row.signing_message().unwrap();
        if let ParticipantRow::Contest { signature, .. } = &mut row {
            *signature = sign(by, &message);
        }
        row
    }

    // ── the floor ───────────────────────────────────────────────────────────────────────

    #[test]
    fn unsigned_human_claim_still_valid() {
        let claim = ActorClaim::new("human:matthew", "s1", AT, None).expect("the floor admits it");
        let mut store = MemoryActorStore::new();
        let cid = store.append(ActorRecord::Claim(claim.clone())).unwrap();
        assert_eq!(
            store.current_for("s1").unwrap(),
            Some((cid, claim.clone())),
            "an unsigned human claim is exactly as current as it always was"
        );

        // It confers no device standing — honest absence, never a refusal.
        let a = device("DeviceA", 1);
        let roster = Roster::new("matthew").unwrap();
        let records = vec![ActorRecord::Claim(claim)];
        assert_eq!(
            standing_human(&roster, &records, &a.did, &verifier(&[&a])),
            None
        );
    }

    // ── witness shape ───────────────────────────────────────────────────────────────────

    #[test]
    fn witness_with_empty_basis_refused() {
        for basis in ["", "   ", "\n\t"] {
            let err = ActorWitness::new("human:matthew", AGENT, "s1", basis, AT)
                .expect_err("a witness that names nothing it knows is refused");
            assert!(err.to_string().contains("basis"), "got: {err}");
        }
    }

    #[test]
    fn witness_by_non_agent_refused() {
        let err = ActorWitness::new("human:matthew", "human:someone", "s1", BASIS, AT)
            .expect_err("a human's own act is a claim, not a witness");
        assert!(err.to_string().contains("not an agent"), "got: {err}");
        assert!(
            ActorWitness::new("human:matthew", "agent:matthew@human", "s1", BASIS, AT).is_err()
        );
        assert!(ActorWitness::new("human:matthew", "orchestrator", "s1", BASIS, AT).is_err());
    }

    #[test]
    fn witness_of_non_human_subject_refused() {
        let err = ActorWitness::new("agent:scribe@opus-5", AGENT, "s1", BASIS, AT)
            .expect_err("only a human is witnessed");
        assert!(err.to_string().contains("not a human"), "got: {err}");
        assert!(ActorWitness::new("human:Matthew", AGENT, "s1", BASIS, AT).is_err());
    }

    // ── standing ────────────────────────────────────────────────────────────────────────

    #[test]
    fn a_rostered_signed_witness_is_standing() {
        let a = device("DeviceA", 1);
        let w = ActorRecord::Witness(witness("s1"));
        let records = vec![w.clone(), signed(&w, &a)];
        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w, &a)).unwrap();

        assert_eq!(
            standing_human(&roster, &records, &a.did, &verifier(&[&a])),
            Some(AgentRef("human:matthew".into()))
        );
    }

    #[test]
    fn signed_row_with_unrostered_signer_not_standing() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let w = ActorRecord::Witness(witness("s1"));
        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w, &a)).unwrap();
        let v = verifier(&[&a, &b]);

        // B signed a witness of matthew, but B is not in matthew's roster.
        let records = vec![w.clone(), signed(&w, &b)];
        assert_eq!(standing_human(&roster, &records, &b.did, &v), None);
        // And A's standing is not conferred by a signature A never made.
        assert_eq!(standing_human(&roster, &records, &a.did, &v), None);
        // A self-founded rival genesis by B admits nobody: a second device binds, never founds.
        roster.append(genesis(&w, &b)).unwrap();
        assert_eq!(standing_human(&roster, &records, &b.did, &v), None);
    }

    #[test]
    fn binding_missing_half_refused() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let v = verifier(&[&a, &b]);

        let both = binding(&a, &a, &b, (true, true));
        verify_binding(&both, &v).expect("both halves verify");

        for halves in [(true, false), (false, true)] {
            let row = binding(&a, &a, &b, halves);
            let err = verify_binding(&row, &v).expect_err("one half is no binding");
            assert!(err.to_string().contains("missing"), "got: {err}");
            let mut roster = Roster::new("matthew").unwrap();
            assert!(roster.append(row).is_err(), "the roster refuses it too");
        }

        // Two present halves where one is not the named key's signature: refused as well.
        let mut forged = binding(&a, &a, &b, (true, true));
        if let ParticipantRow::Binding { sig_b, .. } = &mut forged {
            *sig_b = sign(&a, &both.signing_message().unwrap());
        }
        assert!(verify_binding(&forged, &v).is_err());
    }

    #[test]
    fn a_bound_second_device_is_standing() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let v = verifier(&[&a, &b]);
        let w = ActorRecord::Witness(witness("s1"));
        let w2 = ActorRecord::Witness(witness("s2"));
        let records = vec![w.clone(), signed(&w, &a), w2.clone(), signed(&w2, &b)];

        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w, &a)).unwrap();
        assert_eq!(standing_human(&roster, &records, &b.did, &v), None);
        roster.append(binding(&a, &a, &b, (true, true))).unwrap();
        assert_eq!(
            standing_human(&roster, &records, &b.did, &v),
            Some(AgentRef("human:matthew".into()))
        );
    }

    #[test]
    fn contest_makes_witness_non_standing_until_rewitnessed() {
        let a = device("DeviceA", 1);
        let v = verifier(&[&a]);
        let w1 = ActorRecord::Witness(witness("s1"));
        let mut records = vec![w1.clone(), signed(&w1, &a)];
        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w1, &a)).unwrap();
        assert!(standing_human(&roster, &records, &a.did, &v).is_some());

        let contest_row = contest(&w1.cid().unwrap().to_string(), &a);
        let contest_cid = roster.append(contest_row).unwrap();
        assert_eq!(
            standing_human(&roster, &records, &a.did, &v),
            None,
            "a contested witness confers no standing"
        );

        // Re-witnessed: a NEW record (another session), signed — it stands again.
        let w2 = ActorRecord::Witness(witness("s2"));
        records.push(w2.clone());
        records.push(signed(&w2, &a));
        assert_eq!(
            standing_human(&roster, &records, &a.did, &v),
            Some(AgentRef("human:matthew".into()))
        );

        // And a contest that is itself contested stops counting.
        let records_w1_only = vec![w1.clone(), signed(&w1, &a)];
        roster
            .append(contest(&contest_cid.to_string(), &a))
            .unwrap();
        assert!(standing_human(&roster, &records_w1_only, &a.did, &v).is_some());
    }

    // ── W1: only a member's contest counts (R-P14) ──────────────────────────────────────

    #[test]
    fn w1_non_member_contest_is_inadmissible() {
        let a = device("DeviceA", 1);
        let eve = device("DeviceEve", 9);
        let v = verifier(&[&a, &eve]);
        let w = ActorRecord::Witness(witness("s1"));
        let records = vec![w.clone(), signed(&w, &a)];
        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w, &a)).unwrap();

        // A well-formed, verifying contest — signed by a key that is not in the lineage.
        roster
            .append(contest(&w.cid().unwrap().to_string(), &eve))
            .unwrap();
        assert!(
            roster.contested(&v).is_empty(),
            "a stranger's contest names nothing"
        );
        assert_eq!(
            standing_human(&roster, &records, &a.did, &v),
            Some(AgentRef("human:matthew".into())),
            "a throwaway key cannot void the human's standing"
        );
    }

    #[test]
    fn w1_counter_contest_by_stranger_does_not_restore_standing() {
        let a = device("DeviceA", 1);
        let eve = device("DeviceEve", 9);
        let v = verifier(&[&a, &eve]);
        let w = ActorRecord::Witness(witness("s1"));
        let records = vec![w.clone(), signed(&w, &a)];
        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w, &a)).unwrap();

        // The member (the human, on their device) contests the witness.
        let contest_cid = roster
            .append(contest(&w.cid().unwrap().to_string(), &a))
            .unwrap();
        assert_eq!(standing_human(&roster, &records, &a.did, &v), None);

        // A stranger counter-contests the human's contest: inadmissible, so the contest stands.
        roster
            .append(contest(&contest_cid.to_string(), &eve))
            .unwrap();
        assert_eq!(
            standing_human(&roster, &records, &a.did, &v),
            None,
            "a stranger cannot void the human's contest right"
        );
        assert_eq!(
            roster.effective_contests_of(&w.cid().unwrap().to_string(), &v),
            vec![contest_cid.to_string()]
        );

        // A MEMBER's counter-contest still voids it — the rule is membership, not "no counters".
        roster
            .append(contest(&contest_cid.to_string(), &a))
            .unwrap();
        assert!(standing_human(&roster, &records, &a.did, &v).is_some());
        assert!(roster
            .effective_contests_of(&w.cid().unwrap().to_string(), &v)
            .is_empty());
    }

    // ── W2: the chain root is pinned per device (R-P15) ─────────────────────────────────

    #[test]
    fn w2_roster_with_a_different_root_is_contested_not_standing() {
        let a = device("DeviceA", 1);
        let eve = device("DeviceEve", 9);
        let v = verifier(&[&a, &eve]);
        let w = ActorRecord::Witness(witness("s1"));
        let records = vec![w.clone(), signed(&w, &a), signed(&w, &eve)];

        let mut honest = Roster::new("matthew").unwrap();
        honest.append(genesis(&w, &a)).unwrap();
        assert!(honest.chain_root_matches(&a.did, &v));
        assert_eq!(
            standing_human_pinned(&honest, &records, &a.did, &v, Some(&a.did)),
            Some(AgentRef("human:matthew".into()))
        );
        assert_eq!(
            standing_human_pinned(&honest, &records, &a.did, &v, None),
            Some(AgentRef("human:matthew".into())),
            "an unpinned device defers to the ordinary resolver (and pins on that read)"
        );

        // A rewritten history whose first verifying genesis is Eve's: by line order Eve is the
        // root and Eve stands — but a device that pinned A reads it as contested.
        let mut hijacked = Roster::new("matthew").unwrap();
        hijacked.append(genesis(&w, &eve)).unwrap();
        hijacked.append(genesis(&w, &a)).unwrap();
        assert_eq!(hijacked.chain_root(&v), Some(eve.did.clone()));
        assert!(standing_human(&hijacked, &records, &eve.did, &v).is_some());
        assert!(!hijacked.chain_root_matches(&a.did, &v));
        for signer in [&a.did, &eve.did] {
            assert_eq!(
                standing_human_pinned(&hijacked, &records, signer, &v, Some(&a.did)),
                None,
                "a root that differs from the pin is contested, never standing"
            );
        }
        // A roster with no verified root matches no pin.
        assert!(!Roster::new("matthew")
            .unwrap()
            .chain_root_matches(&a.did, &v));
    }

    // ── W3: standing by the roster alone (R-P16) ────────────────────────────────────────

    #[test]
    fn w3_the_roster_alone_stands_the_chain_root_and_a_bound_device() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let c = device("DeviceC", 3);
        let v = verifier(&[&a, &b, &c]);
        let w = ActorRecord::Witness(witness("s1"));
        let mut roster = Roster::new("matthew").unwrap();
        let genesis_cid = roster.append(genesis(&w, &a)).unwrap();
        let binding_cid = roster.append(binding(&a, &a, &b, (true, true))).unwrap();

        let root = roster.device_standing_row(&a.did, &v).expect("the root");
        assert_eq!(root.via, RosterVia::Genesis);
        assert_eq!(root.row_cid, genesis_cid.to_string());
        assert_eq!(root.record_cid, w.cid().unwrap().to_string());

        let bound = roster.device_standing_row(&b.did, &v).expect("bound");
        assert_eq!(bound.via, RosterVia::Binding);
        assert_eq!(bound.row_cid, binding_cid.to_string());

        assert_eq!(roster.device_standing_row(&c.did, &v), None, "not a member");

        // A member's contest of the founding record takes the root's roster standing away.
        roster
            .append(contest(&w.cid().unwrap().to_string(), &a))
            .unwrap();
        assert_eq!(roster.device_standing_row(&a.did, &v), None);
        assert!(roster.device_standing_row(&b.did, &v).is_some());
    }

    #[test]
    fn a_witness_without_answers_keeps_its_address() {
        let w = witness("s1");
        let json = serde_json::to_string(&w).unwrap();
        assert!(!json.contains("answers"), "{json}");
        let answering = w
            .clone()
            .answering(&ActorRecord::Witness(w.clone()).cid().unwrap().to_string())
            .unwrap();
        assert_ne!(
            ActorRecord::Witness(answering).cid().unwrap(),
            ActorRecord::Witness(w.clone()).cid().unwrap()
        );
        assert!(w.answering("not-a-cid").is_err());
    }

    // ── identity + integrity ────────────────────────────────────────────────────────────

    /// Two rows copied verbatim from `.eprfs/status/actors.jsonl` (2026-08-15): one without and
    /// one with a `definitionCid`. Adding the `Signed` and `Witness` kinds must re-address none
    /// of the claims already written.
    #[test]
    fn existing_agent_claim_cids_unchanged() {
        const ROWS: [&str; 2] = [
            r#"{"cid":"bafyreifa6atceormur2io4frex7q7m2gp2pjazo3mvdo2mpklvasuf7kza","record":{"kind":"claim","claimed":"agent:orchestrator@claude-fable-5","session":"99062e06-c626-4f7e-b55d-41f24754ced5","claimedAt":"2026-08-15T14:41:24Z"}}"#,
            r#"{"cid":"bafyreigfjneabvlkyt7h2n6b6eiaf6ex2ptt7ol2h25dq35zdkg555h66y","record":{"kind":"claim","claimed":"agent:scribe@claude-opus-4-6","session":"99062e06-c626-4f7e-b55d-41f24754ced5","claimedAt":"2026-08-15T15:07:05Z","definitionCid":"sha256:e117a5a9266ae62bc62d075349cd8a92a84db2bb657aea16302ee9491d3bce30"}}"#,
        ];
        #[derive(Deserialize)]
        struct Line {
            cid: String,
            record: ActorRecord,
        }
        for row in ROWS {
            let line: Line = serde_json::from_str(row).unwrap();
            assert!(matches!(line.record, ActorRecord::Claim(_)));
            assert_eq!(line.record.cid().unwrap().to_string(), line.cid, "{row}");
            // And the envelope still writes the same line shape back.
            let rewritten = serde_json::to_value(&line.record).unwrap();
            let original: serde_json::Value = serde_json::from_str(row).unwrap();
            assert_eq!(rewritten, original["record"]);
        }
    }

    #[test]
    fn roster_rejects_row_with_bad_cid() {
        let a = device("DeviceA", 1);
        let w = ActorRecord::Witness(witness("s1"));
        let row = genesis(&w, &a);
        let line = Roster::line_for(&row).unwrap();

        let roster = Roster::from_jsonl("matthew", &format!("{line}\n")).unwrap();
        assert_eq!(roster.rows().len(), 1);

        let tampered = line.replace(&a.did, "did:key:zDeviceEve");
        let err = Roster::from_jsonl("matthew", &tampered).expect_err("payload no longer matches");
        assert!(matches!(err, FabricError::Integrity { .. }), "got: {err:?}");

        // A row about another handle does not read into this roster.
        assert!(Roster::from_jsonl("someone-else", &line).is_err());
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn sidecar_roster_round_trips_and_refuses_tampering() {
        let dir = tempfile::TempDir::new().unwrap();
        let a = device("DeviceA", 1);
        let w = ActorRecord::Witness(witness("s1"));
        let roster = SidecarRoster::open(dir.path(), "matthew").unwrap();
        assert_eq!(
            roster.log_path(),
            dir.path().join(".eprfs/status/participants/matthew.jsonl")
        );
        let cid = roster.append(genesis(&w, &a)).unwrap();
        let read = roster.read().unwrap();
        assert_eq!(read.rows()[0].0, cid);
        assert_eq!(read.chain_root(&verifier(&[&a])), Some(a.did.clone()));

        let text = std::fs::read_to_string(roster.log_path()).unwrap();
        std::fs::write(roster.log_path(), text.replace("matthew\"", "matthew2\"")).unwrap();
        assert!(roster.read().is_err());
    }

    // ── peer endpoints: admission of a device's iroh node id (spec §3.1) ────────────────

    const NODE_A: &str = "a1a2a3a4a5a6a7a8a9b0b1b2b3b4b5b6b7b8b9c0c1c2c3c4c5c6c7c8c9d0d1d2";
    const NODE_B: &str = "b1b2b3b4b5b6b7b8b9c0c1c2c3c4c5c6c7c8c9d0d1d2d3d4d5d6d7d8d9e0e1e2";
    const NODE_B2: &str = "0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b";

    fn endpoint(root: &Device, dev: &Device, node_id: &str, seq: u64) -> ParticipantRow {
        let mut row = ParticipantRow::PeerEndpoint {
            handle: "matthew".into(),
            chain_root: root.did.clone(),
            device: dev.did.clone(),
            node_id: node_id.into(),
            seq,
            nonce: format!("nonce-{seq}"),
            signature: String::new(),
        };
        let message = row.signing_message().unwrap();
        if let ParticipantRow::PeerEndpoint { signature, .. } = &mut row {
            *signature = sign(dev, &message);
        }
        row
    }

    fn rows_of(roster: &Roster) -> Vec<ParticipantRow> {
        roster.rows().iter().map(|(_, r)| r.clone()).collect()
    }

    /// A genesis on A plus a both-halves binding of B — the two-device lineage every admission
    /// test starts from.
    fn two_device_roster(a: &Device, b: &Device) -> (Roster, Cid) {
        let w = ActorRecord::Witness(witness("s1"));
        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w, a)).unwrap();
        let binding_cid = roster.append(binding(a, a, b, (true, true))).unwrap();
        (roster, binding_cid)
    }

    #[test]
    fn peer_endpoint_signing_message_is_stable() {
        let row = ParticipantRow::PeerEndpoint {
            handle: "matthew".into(),
            chain_root: "did:key:zDeviceA".into(),
            device: "did:key:zDeviceB".into(),
            node_id: NODE_B.into(),
            seq: 7,
            nonce: "nonce-1".into(),
            signature: String::new(),
        };
        let message = String::from_utf8(row.signing_message().unwrap()).unwrap();
        assert_eq!(
            message, "elohim:participant-roster-row:v1:bafyreietixh7hduwgh5pg7eubyqvr4kn7hopqwipswdz7necdf3fes6scm",
            "the bytes a device signs for its endpoint are fixed: changing them orphans every \
             PeerEndpoint row already written"
        );
        // The signature itself is never under the signature.
        let mut signed = row.clone();
        if let ParticipantRow::PeerEndpoint { signature, .. } = &mut signed {
            *signature = "00".repeat(64);
        }
        assert_eq!(
            signed.signing_message().unwrap(),
            row.signing_message().unwrap()
        );
        // And the wire shape follows the enum's tag and camelCase.
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["kind"], "peerEndpoint");
        assert_eq!(json["chainRoot"], "did:key:zDeviceA");
        assert_eq!(json["nodeId"], NODE_B);
        assert_eq!(json["seq"], 7);
        // seq is under the signature: a row re-sequenced by anyone but the device stops verifying.
        let mut resequenced = row.clone();
        if let ParticipantRow::PeerEndpoint { seq, .. } = &mut resequenced {
            *seq = 8;
        }
        assert_ne!(
            resequenced.signing_message().unwrap(),
            row.signing_message().unwrap()
        );
    }

    #[test]
    fn peer_endpoint_shape_refuses_a_node_id_iroh_would_not_print() {
        let a = device("DeviceA", 1);
        for bad in [
            "",
            "abc",
            &NODE_A.to_uppercase(),
            &format!("{NODE_A}00"),
            &NODE_A.replace('a', "g"),
            // iroh also PARSES base32, but prints hex: only the printed form is canonical.
            "ksxwu3zbnr4sk2dbmr4xg3lfnrsws2dfnzsxa3dfnr2xg5dfnzsxa3dfnr2q",
        ] {
            let row = endpoint(&a, &a, bad, 1);
            let err = row.validate_shape().expect_err("not a canonical node id");
            assert!(err.to_string().contains("node id"), "{bad}: {err}");
        }
        let mut no_nonce = endpoint(&a, &a, NODE_A, 1);
        if let ParticipantRow::PeerEndpoint { nonce, .. } = &mut no_nonce {
            *nonce = " ".into();
        }
        assert!(
            no_nonce.validate_shape().is_err(),
            "an endpoint needs a nonce"
        );
        let mut unsigned = endpoint(&a, &a, NODE_A, 1);
        if let ParticipantRow::PeerEndpoint { signature, .. } = &mut unsigned {
            signature.clear();
        }
        assert!(
            unsigned.validate_shape().is_err(),
            "an unsigned endpoint is refused"
        );
        endpoint(&a, &a, NODE_A, 1)
            .validate_shape()
            .expect("a canonical endpoint passes");
    }

    #[test]
    fn peer_endpoint_valid_for_genesis_device() {
        let a = device("DeviceA", 1);
        let v = verifier(&[&a]);
        let w = ActorRecord::Witness(witness("s1"));
        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w, &a)).unwrap();
        roster.append(endpoint(&a, &a, NODE_A, 1)).unwrap();

        let admitted = admitted_endpoints(&rows_of(&roster), &a.did, &v);
        assert_eq!(
            admitted,
            BTreeMap::from([(NODE_A.to_string(), a.did.clone())])
        );
    }

    #[test]
    fn peer_endpoint_valid_for_bound_controller() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let v = verifier(&[&a, &b]);
        let (mut roster, _) = two_device_roster(&a, &b);
        roster.append(endpoint(&a, &a, NODE_A, 1)).unwrap();
        roster.append(endpoint(&a, &b, NODE_B, 1)).unwrap();

        let admitted = admitted_endpoints(&rows_of(&roster), &a.did, &v);
        assert_eq!(
            admitted,
            BTreeMap::from([
                (NODE_A.to_string(), a.did.clone()),
                (NODE_B.to_string(), b.did.clone()),
            ])
        );
    }

    #[test]
    fn peer_endpoint_refused_for_unbound_device() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let c = device("DeviceC", 3);
        let v = verifier(&[&a, &b, &c]);
        let w = ActorRecord::Witness(witness("s1"));
        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w, &a)).unwrap();
        // C was never bound: its own well-signed endpoint admits nothing.
        roster.append(endpoint(&a, &c, NODE_B, 1)).unwrap();
        // Half a binding is no binding: B signed alone.
        let half = binding(&a, &a, &b, (false, true));
        assert!(roster.append(half.clone()).is_err());
        roster.append(endpoint(&a, &b, NODE_B2, 1)).unwrap();
        // A row naming A as its device but signed by C is forged, not A's.
        let mut forged = endpoint(&a, &a, NODE_A, 1);
        let message = forged.signing_message().unwrap();
        if let ParticipantRow::PeerEndpoint { signature, .. } = &mut forged {
            *signature = sign(&c, &message);
        }
        roster.append(forged).unwrap();

        let mut rows = rows_of(&roster);
        rows.push(half);
        assert!(
            admitted_endpoints(&rows, &a.did, &v).is_empty(),
            "no unbound device and no forged row is admitted"
        );
    }

    #[test]
    fn peer_endpoint_refused_after_binding_contest() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let v = verifier(&[&a, &b]);
        let (mut roster, binding_cid) = two_device_roster(&a, &b);
        roster.append(endpoint(&a, &a, NODE_A, 1)).unwrap();
        roster.append(endpoint(&a, &b, NODE_B, 1)).unwrap();
        assert_eq!(admitted_endpoints(&rows_of(&roster), &a.did, &v).len(), 2);

        // A contests B's binding, AFTER B's endpoint was written: B loses its endpoint.
        roster
            .append(contest(&binding_cid.to_string(), &a))
            .unwrap();
        assert_eq!(
            admitted_endpoints(&rows_of(&roster), &a.did, &v),
            BTreeMap::from([(NODE_A.to_string(), a.did.clone())]),
            "a contested binding revokes the device's endpoint"
        );

        // A contest of the root's founding record takes the root's endpoint away the same way.
        let w = ActorRecord::Witness(witness("s1"));
        roster
            .append(contest(&w.cid().unwrap().to_string(), &a))
            .unwrap();
        assert!(admitted_endpoints(&rows_of(&roster), &a.did, &v).is_empty());
    }

    #[test]
    fn a_contested_endpoint_row_admits_nothing_and_does_not_revive_an_older_one() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let v = verifier(&[&a, &b]);
        let (mut roster, _) = two_device_roster(&a, &b);
        roster.append(endpoint(&a, &b, NODE_B, 1)).unwrap();
        let newer = roster.append(endpoint(&a, &b, NODE_B2, 2)).unwrap();
        roster.append(contest(&newer.to_string(), &a)).unwrap();
        assert!(
            admitted_endpoints(&rows_of(&roster), &a.did, &v).is_empty(),
            "the newest row is contested; the superseded key stays superseded"
        );
    }

    #[test]
    fn newest_peer_endpoint_supersedes() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let c = device("DeviceC", 3);
        let v = verifier(&[&a, &b, &c]);
        let (mut roster, _) = two_device_roster(&a, &b);
        roster.append(endpoint(&a, &b, NODE_B, 1)).unwrap();
        roster.append(endpoint(&a, &b, NODE_B2, 2)).unwrap();
        let admitted = admitted_endpoints(&rows_of(&roster), &a.did, &v);
        assert_eq!(
            admitted,
            BTreeMap::from([(NODE_B2.to_string(), b.did.clone())]),
            "a rotated key replaces the old one; the old node id is no longer admitted"
        );

        // A newer row that does not verify as B's never displaces B's valid one.
        let mut forged = endpoint(&a, &b, NODE_A, 3);
        let message = forged.signing_message().unwrap();
        if let ParticipantRow::PeerEndpoint { signature, .. } = &mut forged {
            *signature = sign(&c, &message);
        }
        roster.append(forged).unwrap();
        assert_eq!(admitted_endpoints(&rows_of(&roster), &a.did, &v), admitted);
    }

    #[test]
    fn a_replayed_older_endpoint_row_does_not_roll_back() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let v = verifier(&[&a, &b]);
        let (mut roster, _) = two_device_roster(&a, &b);
        let old = endpoint(&a, &b, NODE_B, 1);
        let new = endpoint(&a, &b, NODE_B2, 2);
        roster.append(old.clone()).unwrap();
        let rotated = roster.append(new.clone()).unwrap();
        // Someone re-appends B's old, validly signed row after the rotation.
        roster.append(old.clone()).unwrap();

        let expected = BTreeMap::from([(NODE_B2.to_string(), b.did.clone())]);
        let rows = rows_of(&roster);
        assert_eq!(
            admitted_endpoints(&rows, &a.did, &v),
            expected,
            "line order never decides: the retired node id stays retired"
        );
        // A history rewritten so the old row simply comes last: still the highest seq wins.
        let mut reordered: Vec<ParticipantRow> = rows.iter().take(2).cloned().collect();
        reordered.push(new);
        reordered.push(old);
        assert_eq!(admitted_endpoints(&reordered, &a.did, &v), expected);
        // The admitted entry names the row it stands on.
        let admission = endpoint_admission(&rows, &a.did, &v);
        assert_eq!(
            admission.admitted.get(NODE_B2).map(|e| e.row_cid.clone()),
            Some(rotated.to_string())
        );
    }

    #[test]
    fn same_seq_different_node_ids_refuses_device() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let v = verifier(&[&a, &b]);
        let (mut roster, _) = two_device_roster(&a, &b);
        roster.append(endpoint(&a, &a, NODE_A, 1)).unwrap();
        roster.append(endpoint(&a, &b, NODE_B, 1)).unwrap();
        roster.append(endpoint(&a, &b, NODE_B, 3)).unwrap();
        roster.append(endpoint(&a, &b, NODE_B2, 3)).unwrap();

        let admission = endpoint_admission(&rows_of(&roster), &a.did, &v);
        assert_eq!(
            admission
                .admitted
                .iter()
                .map(|(node, e)| (node.clone(), e.device.clone()))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([(NODE_A.to_string(), a.did.clone())]),
            "a device that names two node ids at one seq admits neither, nor an older one"
        );
        assert_eq!(
            admission.equivocations,
            BTreeMap::from([(
                b.did.clone(),
                BTreeSet::from([NODE_B.to_string(), NODE_B2.to_string()])
            )])
        );

        // The same node id twice at one seq (a re-run) is not an equivocation.
        roster.append(endpoint(&a, &b, NODE_B2, 4)).unwrap();
        let mut again = endpoint(&a, &b, NODE_B2, 4);
        if let ParticipantRow::PeerEndpoint {
            nonce, signature, ..
        } = &mut again
        {
            *nonce = "another-nonce".into();
            signature.clear();
        }
        let message = again.signing_message().unwrap();
        if let ParticipantRow::PeerEndpoint { signature, .. } = &mut again {
            *signature = sign(&b, &message);
        }
        roster.append(again).unwrap();
        let admission = endpoint_admission(&rows_of(&roster), &a.did, &v);
        assert!(admission.equivocations.is_empty());
        assert_eq!(
            admission.admitted.get(NODE_B2).map(|e| e.device.clone()),
            Some(b.did.clone())
        );
    }

    #[test]
    fn a_node_id_claimed_by_two_devices_admits_neither() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let v = verifier(&[&a, &b]);
        let (mut roster, _) = two_device_roster(&a, &b);
        roster.append(endpoint(&a, &a, NODE_A, 1)).unwrap();
        roster.append(endpoint(&a, &b, NODE_A, 1)).unwrap();

        let admission = endpoint_admission(&rows_of(&roster), &a.did, &v);
        assert!(admission.admitted.is_empty(), "both claimants are refused");
        assert_eq!(
            admission.conflicts,
            BTreeMap::from([(
                NODE_A.to_string(),
                BTreeSet::from([a.did.clone(), b.did.clone()])
            )])
        );
        assert!(admitted_endpoints(&rows_of(&roster), &a.did, &v).is_empty());

        // B rotates away from the shared id: both are admitted again.
        roster.append(endpoint(&a, &b, NODE_B, 2)).unwrap();
        assert_eq!(admitted_endpoints(&rows_of(&roster), &a.did, &v).len(), 2);
    }

    #[test]
    fn peer_endpoint_rows_for_another_chain_root_are_ignored() {
        let a = device("DeviceA", 1);
        let b = device("DeviceB", 2);
        let eve = device("DeviceEve", 9);
        let v = verifier(&[&a, &b, &eve]);
        let w = ActorRecord::Witness(witness("s1"));
        // Eve's rival genesis is FIRST in line order, then A's lineage.
        let mut roster = Roster::new("matthew").unwrap();
        roster.append(genesis(&w, &eve)).unwrap();
        roster.append(endpoint(&eve, &eve, NODE_B2, 1)).unwrap();
        roster.append(genesis(&w, &a)).unwrap();
        roster.append(binding(&a, &a, &b, (true, true))).unwrap();
        roster.append(endpoint(&a, &b, NODE_B, 1)).unwrap();
        // An endpoint naming Eve's root, signed by B, counts for nothing in A's lineage.
        roster.append(endpoint(&eve, &b, NODE_A, 2)).unwrap();

        let rows = rows_of(&roster);
        assert_eq!(
            admitted_endpoints(&rows, &a.did, &v),
            BTreeMap::from([(NODE_B.to_string(), b.did.clone())])
        );
        assert_eq!(
            admitted_endpoints(&rows, &eve.did, &v),
            BTreeMap::from([(NODE_B2.to_string(), eve.did.clone())]),
            "each lineage reads only its own rows"
        );
        let stranger = device("DeviceC", 3);
        assert!(admitted_endpoints(&rows, &stranger.did, &v).is_empty());
    }

    #[test]
    fn roster_rows_carry_no_private_material() {
        let a = device("DeviceA", 1);
        let w = ActorRecord::Witness(witness("s1"));
        let line = Roster::line_for(&genesis(&w, &a)).unwrap();
        assert!(!line.contains('@'), "no email, no role@model: {line}");
    }

    // ── device-binding contests: only a standing device's contest retires a device ────────

    /// A three-device lineage (A the root, B and C bound by A), each with an endpoint, and the
    /// CIDs of A's genesis and of B's and C's bindings.
    fn three_device_rows(a: &Device, b: &Device, c: &Device) -> (Vec<ParticipantRow>, [String; 3]) {
        let w = ActorRecord::Witness(witness("s1"));
        let mut roster = Roster::new("matthew").unwrap();
        let gen = roster.append(genesis(&w, a)).unwrap().to_string();
        let bind_b = roster
            .append(binding(a, a, b, (true, true)))
            .unwrap()
            .to_string();
        let bind_c = roster
            .append(binding(a, a, c, (true, true)))
            .unwrap()
            .to_string();
        roster.append(endpoint(a, a, NODE_A, 1)).unwrap();
        roster.append(endpoint(a, b, NODE_B, 1)).unwrap();
        roster.append(endpoint(a, c, NODE_B2, 1)).unwrap();
        (rows_of(&roster), [gen, bind_b, bind_c])
    }

    fn roster_of(rows: &[ParticipantRow]) -> Roster {
        let mut roster = Roster::new("matthew").unwrap();
        for row in rows {
            roster.append(row.clone()).unwrap();
        }
        roster
    }

    fn admitted_devices(
        rows: &[ParticipantRow],
        root: &Device,
        v: &dyn SignatureVerifier,
    ) -> BTreeSet<String> {
        admitted_endpoints(rows, &root.did, v)
            .into_values()
            .collect()
    }

    /// `rows` in a pseudo-random order (xorshift from `seed`), keeping the relative order of
    /// `pinned` (indices into `rows`) — T2's current binding of a device is its latest by line.
    fn shuffled(rows: &[ParticipantRow], seed: u64, pinned: &[usize]) -> Vec<ParticipantRow> {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut order: Vec<usize> = (0..rows.len()).collect();
        for i in (1..order.len()).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let j = usize::try_from(state % (i as u64 + 1)).unwrap();
            order.swap(i, j);
        }
        // Put the pinned rows back into their original relative order, in the slots they took.
        let mut slots: Vec<usize> = order
            .iter()
            .enumerate()
            .filter(|(_, i)| pinned.contains(i))
            .map(|(slot, _)| slot)
            .collect();
        slots.sort_unstable();
        for (slot, &row) in slots.iter().zip(pinned) {
            order[*slot] = row;
        }
        order.into_iter().map(|i| rows[i].clone()).collect()
    }

    /// The admitted devices of `rows` under 24 orders: every order must give the same answer.
    fn admitted_in_every_order(
        rows: &[ParticipantRow],
        root: &Device,
        v: &dyn SignatureVerifier,
        pinned: &[usize],
    ) -> BTreeSet<String> {
        let first = admitted_devices(rows, root, v);
        for seed in 1..=24 {
            let order = shuffled(rows, seed, pinned);
            assert_eq!(
                admitted_devices(&order, root, v),
                first,
                "order {seed} disagrees"
            );
        }
        first
    }

    fn rebinding(root: &Device, joiner: &Device, nonce: &str) -> ParticipantRow {
        let mut row = binding(root, root, joiner, (true, true));
        if let ParticipantRow::Binding { nonce: n, .. } = &mut row {
            *n = nonce.into();
        }
        let message = row.signing_message().unwrap();
        if let ParticipantRow::Binding { sig_a, sig_b, .. } = &mut row {
            *sig_a = sign(root, &message);
            *sig_b = sign(joiner, &message);
        }
        row
    }

    fn cid_of(row: &ParticipantRow) -> String {
        row.cid().unwrap().to_string()
    }

    fn three() -> (Device, Device, Device) {
        (
            device("DeviceA", 1),
            device("DeviceB", 2),
            device("DeviceC", 3),
        )
    }

    fn set(devices: &[&Device]) -> BTreeSet<String> {
        devices.iter().map(|d| d.did.clone()).collect()
    }

    #[test]
    fn n1_1_a_retired_devices_contest_retires_no_one() {
        let (a, b, c) = three();
        let v = verifier(&[&a, &b, &c]);
        let (mut rows, [_, bind_b, bind_c]) = three_device_rows(&a, &b, &c);
        rows.push(contest(&bind_b, &a));
        rows.push(contest(&bind_c, &b));
        assert_eq!(admitted_in_every_order(&rows, &a, &v, &[]), set(&[&a, &c]));
    }

    #[test]
    fn n1_2_a_corroborator_is_not_retired_by_the_device_it_corroborated_against() {
        let (a, b, c) = three();
        let v = verifier(&[&a, &b, &c]);
        let (mut rows, [_, bind_b, bind_c]) = three_device_rows(&a, &b, &c);
        rows.push(contest(&bind_b, &a));
        rows.push(contest(&bind_b, &c));
        rows.push(contest(&bind_c, &b));
        assert_eq!(admitted_in_every_order(&rows, &a, &v, &[]), set(&[&a, &c]));
    }

    #[test]
    fn n1_3_two_devices_contesting_each_other_alone_are_both_retired() {
        let (a, b, c) = three();
        let v = verifier(&[&a, &b, &c]);
        let (mut rows, [gen, bind_b, _]) = three_device_rows(&a, &b, &c);
        rows.push(contest(&bind_b, &a));
        rows.push(contest(&gen, &b));
        assert_eq!(admitted_in_every_order(&rows, &a, &v, &[]), set(&[&c]));
    }

    #[test]
    fn n1_4_a_retired_device_rebound_stands_again() {
        let (a, b, c) = three();
        let v = verifier(&[&a, &b, &c]);
        let (mut rows, [_, bind_b, _]) = three_device_rows(&a, &b, &c);
        rows.push(contest(&bind_b, &a));
        assert_eq!(admitted_devices(&rows, &a, &v), set(&[&a, &c]));
        rows.push(rebinding(&a, &b, "nonce-2"));
        // A device stands while any of its bindings is accepted: line order never decides.
        assert_eq!(
            admitted_in_every_order(&rows, &a, &v, &[]),
            set(&[&a, &b, &c])
        );
    }

    #[test]
    fn n1_4b_a_contest_written_before_a_rebind_never_retires_the_new_binding() {
        let (a, b, c) = three();
        let v = verifier(&[&a, &b, &c]);
        let (mut rows, [_, bind_b, _]) = three_device_rows(&a, &b, &c);
        let retire = contest(&bind_b, &a);
        let rebind = rebinding(&a, &b, "nonce-2");
        let rebind_cid = cid_of(&rebind);
        rows.push(retire);
        rows.push(rebind);
        for seed in 0..=24 {
            let order = if seed == 0 {
                rows.clone()
            } else {
                shuffled(&rows, seed, &[])
            };
            let roster = roster_of(&order);
            let standing = roster.device_standing_row(&b.did, &v).expect("B stands");
            assert_eq!(
                standing.row_cid, rebind_cid,
                "order {seed}: B stands on its new binding"
            );
        }
    }

    #[test]
    fn n1_5_a_withdrawn_contest_restores_the_device() {
        let (a, b, c) = three();
        let v = verifier(&[&a, &b, &c]);
        let (mut rows, [_, bind_b, _]) = three_device_rows(&a, &b, &c);
        let retire = contest(&bind_b, &a);
        rows.push(contest(&cid_of(&retire), &a));
        rows.push(retire);
        assert_eq!(
            admitted_in_every_order(&rows, &a, &v, &[]),
            set(&[&a, &b, &c])
        );
    }

    #[test]
    fn n1_6_a_standing_members_counter_contest_restores_the_device() {
        let (a, b, c) = three();
        let v = verifier(&[&a, &b, &c]);
        let (mut rows, [_, bind_b, _]) = three_device_rows(&a, &b, &c);
        let retire = contest(&bind_b, &a);
        rows.push(contest(&cid_of(&retire), &c));
        rows.push(retire);
        assert_eq!(
            admitted_in_every_order(&rows, &a, &v, &[]),
            set(&[&a, &b, &c])
        );
    }

    #[test]
    fn n1_7_the_retired_device_cannot_counter_the_contest_of_itself() {
        let (a, b, c) = three();
        let v = verifier(&[&a, &b, &c]);
        let (mut rows, [_, bind_b, _]) = three_device_rows(&a, &b, &c);
        let retire = contest(&bind_b, &a);
        rows.push(contest(&cid_of(&retire), &b));
        rows.push(retire);
        assert_eq!(admitted_in_every_order(&rows, &a, &v, &[]), set(&[&a, &c]));
    }

    #[test]
    fn n1_8_a_cycle_of_three_with_no_outside_contest_retires_all_three() {
        let (a, b, c) = three();
        let v = verifier(&[&a, &b, &c]);
        let (mut rows, [gen, bind_b, bind_c]) = three_device_rows(&a, &b, &c);
        rows.push(contest(&bind_b, &a));
        rows.push(contest(&bind_c, &b));
        rows.push(contest(&gen, &c));
        assert!(admitted_in_every_order(&rows, &a, &v, &[]).is_empty());
    }

    #[test]
    fn n1_a_contest_signed_outside_the_lineage_is_ignored() {
        let (a, b, c) = three();
        let x = device("DeviceX", 9);
        let v = verifier(&[&a, &b, &c, &x]);
        let (mut rows, [_, bind_b, _]) = three_device_rows(&a, &b, &c);
        rows.push(contest(&bind_b, &x));
        assert_eq!(
            admitted_in_every_order(&rows, &a, &v, &[]),
            set(&[&a, &b, &c])
        );
    }
}
