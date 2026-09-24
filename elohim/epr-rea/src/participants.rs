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

use std::collections::{BTreeSet, HashMap};
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
}

impl ParticipantRow {
    /// The handle this row speaks about.
    pub fn handle(&self) -> &str {
        match self {
            ParticipantRow::Genesis { handle, .. }
            | ParticipantRow::Binding { handle, .. }
            | ParticipantRow::Contest { handle, .. } => handle,
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

    /// The CIDs contested by an EFFECTIVE contest: a contest row whose signature verifies and
    /// which is not itself the target of an effective contest. Content addressing makes the
    /// contest graph acyclic (a row can only name CIDs that existed before it), so the recursion
    /// terminates.
    pub fn contested(&self, verifier: &dyn SignatureVerifier) -> BTreeSet<String> {
        // target CID -> the CIDs of the verified contest rows naming it.
        let mut contests: HashMap<String, Vec<String>> = HashMap::new();
        for (cid, row) in &self.rows {
            if let ParticipantRow::Contest {
                target_cid,
                by,
                signature,
                ..
            } = row
            {
                if row_signature_verifies(verifier, row, by, signature) {
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
        contests
            .iter()
            .filter(|(_, by_rows)| by_rows.iter().any(|c| effective(c, &contests, &mut memo)))
            .map(|(target, _)| target.clone())
            .collect()
    }
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
                ParticipantRef::Agent { .. } => return None,
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

fn validate_handle(handle: &str) -> Result<()> {
    match parse_participant_ref(&format!("human:{handle}"))? {
        ParticipantRef::Human { .. } => Ok(()),
        ParticipantRef::Agent { .. } => Err(FabricError::Decode(format!(
            "`{handle}` is not a human handle"
        ))),
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
        let mut log = crate::sidecar::LockedLog::exclusive(&self.log_path)?;
        let mut roster = Roster::from_jsonl(&self.handle, &log.contents()?)?;
        let line = Roster::line_for(&row)?;
        let cid = roster.append(row)?;
        log.append(line)?;
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

    #[test]
    fn roster_rows_carry_no_private_material() {
        let a = device("DeviceA", 1);
        let w = ActorRecord::Witness(witness("s1"));
        let line = Roster::line_for(&genesis(&w, &a)).unwrap();
        assert!(!line.contains('@'), "no email, no role@model: {line}");
    }
}
