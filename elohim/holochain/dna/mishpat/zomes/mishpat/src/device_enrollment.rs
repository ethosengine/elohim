//! Additive device relationships, notarized as existing Mishpat Commitments.
//!
//! Every reader repeats authorization: generic Commitment integrity validation
//! does not confer identity authority. Lifecycle links are discovery, not proof.
//! No device enters the controller set, and no SQL row can authorize enrollment.
//!
//! Every device a person has joined speaks for them, and any of them may
//! approve the next. The authority record stays the root and the person's
//! policy and is never rewritten by a join; each joining record stands alone
//! and is verified by walking its approvers' own joining records back to the
//! authority ([`binding_stands`]). The rule, in four sentences:
//!
//! 1. A joining record stands when it names a current authority of its
//!    identity, the joining device signed it, and the authority's root
//!    controllers have not revoked it.
//! 2. At least as many distinct approvers as the person's policy requires
//!    (one by default) signed it, never the joining device itself, and each
//!    approver is a root controller of the authority or a device whose own
//!    joining record, which this record names, stands by this same rule for
//!    the same identity.
//! 3. The walk back to the authority goes at most `MAX_APPROVAL_DEPTH`
//!    records deep and reads at most `MAX_APPROVAL_VISITS` records; a record
//!    met again on its own way back refuses as a cycle, and one reached by
//!    two approvers' ways is read once.
//! 4. Read now, an approver whose joining record was revoked no longer
//!    counts, so what it approved stops verifying too; read at an
//!    authenticated earlier moment (`verify_historical_device_binding`), it
//!    counts if its revocation came after that moment.
use crate::commitment_record::get_commitment_record;
use crate::commitments::CommitmentOutput;
use hdk::prelude::*;
use mishpat_integrity::{Commitment, EntryTypes, LinkTypes};

const DOMAIN: &str = "elohim:device-enrollment:v1";
const MAX_LIFECYCLE: usize = 64;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ControllerPolicy {
    pub kind: String,
    pub m: Option<usize>,
    pub n: Option<usize>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HumanRootEvidence {
    pub human_action_hash: ActionHash,
    pub human_id: String,
    pub author: AgentPubKey,
    pub dna_hash: DnaHash,
    pub timestamp: Timestamp,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Authority {
    pub action: String,
    pub binding_kind: String,
    pub chain_root: ActionHash,
    pub human_id: String,
    pub human_dna: DnaHash,
    pub network_dna: DnaHash,
    pub head_key: AgentPubKey,
    pub controllers: Vec<AgentPubKey>,
    pub controller_policy: ControllerPolicy,
    pub authorization: String,
    pub previous_authority: Option<ActionHash>,
    pub signatures: Vec<Proof>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Proof {
    pub agent: AgentPubKey,
    pub signature: Signature,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceIntent {
    pub domain: String,
    pub authority: ActionHash,
    pub identity_root: ActionHash,
    pub device_key: AgentPubKey,
    pub network_dna: DnaHash,
    pub content_dna: DnaHash,
    pub issued_at: Timestamp,
    /// A revoked device binding remains a fact. Reenrollment must name it.
    pub supersedes: Option<ActionHash>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceBinding {
    pub action: String,
    pub binding_kind: String,
    pub intent: DeviceIntent,
    /// The approvals: each a signature over the intent by a root controller
    /// of the authority or by a device that speaks for the person.
    pub controllers: Vec<Proof>,
    pub possession: Proof,
    /// For each approver that is a device rather than a root controller, the
    /// joining record by which it speaks for the person. Unsigned evidence:
    /// the verifier checks each one names that approver and stands. Absent
    /// (and absent from the encoding) when every approver is a root
    /// controller, so earlier records keep their exact bytes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approved_via: Vec<ApprovedVia>,
}
/// A device approver and the joining record by which it speaks.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ApprovedVia {
    pub agent: AgentPubKey,
    pub binding: ActionHash,
}

/// How deep the walk from a joining record back to its authority may go.
pub const MAX_APPROVAL_DEPTH: usize = 8;
/// How many joining records one walk may read.
pub const MAX_APPROVAL_VISITS: usize = 24;
/// How many discovery links one read of an identity's devices follows.
const MAX_DEVICE_LINKS: usize = 64;
/// How many affirmations of one device a read follows.
const MAX_AFFIRMATION_LINKS: usize = 32;
pub use qahal_types::{VerifiedDevice, VerifyDeviceInput};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceRevocation {
    pub action: String,
    pub binding_kind: String,
    pub target: ActionHash,
    pub authority: ActionHash,
    pub network_dna: DnaHash,
    pub signatures: Vec<Proof>,
}

fn refuse(message: &str) -> WasmError {
    wasm_error!(WasmErrorInner::Guest(message.to_string()))
}
fn bytes<T: Serialize>(value: &T) -> ExternResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| refuse("identity proof serialization failed"))
}
fn human_root(hash: ActionHash) -> ExternResult<HumanRootEvidence> {
    match call(
        CallTargetCell::OtherRole("imagodei".into()),
        ZomeName::from("imagodei"),
        "get_human_root_evidence".into(),
        None,
        hash,
    )? {
        ZomeCallResponse::Ok(result) => result
            .decode()
            .map_err(|_| refuse("Human evidence decode failed")),
        _ => Err(refuse("Human evidence unavailable")),
    }
}
fn record<T: serde::de::DeserializeOwned>(
    hash: ActionHash,
    action: &str,
) -> ExternResult<(Record, Commitment, T)> {
    let record =
        get_commitment_record(hash)?.ok_or_else(|| refuse("identity commitment unavailable"))?;
    let entry = record
        .entry()
        .to_app_option::<Commitment>()
        .map_err(|_| refuse("invalid Commitment entry encoding"))?
        .ok_or_else(|| refuse("identity commitment entry unavailable"))?;
    if entry.action != action {
        return Err(refuse("wrong identity commitment action"));
    }
    let value = serde_json::from_str(&entry.payload_json)
        .map_err(|_| refuse("invalid identity commitment payload"))?;
    Ok((record, entry, value))
}
fn threshold(policy: &ControllerPolicy, controllers: &[AgentPubKey]) -> Result<usize, String> {
    let unique: std::collections::HashSet<_> = controllers.iter().collect();
    if unique.is_empty() || unique.len() != controllers.len() {
        return Err("invalid controller set".into());
    }
    match policy.kind.as_str() {
        "self" if controllers.len() == 1 => Ok(1),
        "steward-set" => Ok(controllers.len()),
        "recovery-quorum" => match (policy.m, policy.n) {
            (Some(m), Some(n)) if m > 0 && m <= n && n == controllers.len() => Ok(m),
            _ => Err("invalid recovery quorum".into()),
        },
        _ => Err("unsupported controller policy".into()),
    }
}
fn authorized_signers(authority: &Authority, proofs: &[Proof]) -> Result<(), String> {
    let required = threshold(&authority.controller_policy, &authority.controllers)?;
    let mut seen = std::collections::HashSet::new();
    for proof in proofs {
        if !authority.controllers.contains(&proof.agent) || !seen.insert(proof.agent.clone()) {
            return Err("unauthorized or duplicate identity signer".into());
        }
    }
    if seen.len() < required {
        return Err("identity controller quorum not met".into());
    }
    Ok(())
}
/// Who an approval's signer is to the person.
#[derive(Debug, PartialEq)]
enum Approver {
    /// A root controller named by the authority.
    Root,
    /// A device that speaks through the joining record named.
    Device(ActionHash),
}

/// The pure part of the approval rule (rule 2): who signed, that each signer
/// counts once, that the joining device did not approve itself, that each
/// device approver names the record it speaks through, and that enough of
/// them signed. Signatures and the named records are checked by the caller.
fn classify_approvers(
    authority: &Authority,
    intent: &DeviceIntent,
    proofs: &[Proof],
    via: &[ApprovedVia],
) -> Result<Vec<Approver>, String> {
    let required = threshold(&authority.controller_policy, &authority.controllers)?;
    // The unsigned part of a record says exactly one thing per device
    // approver, in the order they approved, and nothing else: a record cannot
    // be republished with other unsigned contents and still stand.
    let devices: Vec<&AgentPubKey> = proofs
        .iter()
        .map(|p| &p.agent)
        .filter(|a| !authority.controllers.contains(a))
        .collect();
    if via.len() != devices.len() || via.iter().zip(&devices).any(|(v, a)| &v.agent != *a) {
        return Err(
            "approved_via must name exactly the device approvers, in the order they approved"
                .into(),
        );
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(proofs.len());
    for proof in proofs {
        if proof.agent == intent.device_key {
            return Err("a device cannot approve its own joining".into());
        }
        if !seen.insert(proof.agent.clone()) {
            return Err("one device cannot approve twice".into());
        }
        if authority.controllers.contains(&proof.agent) {
            out.push(Approver::Root);
        } else {
            match via.iter().find(|v| v.agent == proof.agent) {
                Some(v) => out.push(Approver::Device(v.binding.clone())),
                None => {
                    return Err("an approver does not show that it speaks for this person".into())
                }
            }
        }
    }
    if seen.len() < required {
        return Err("not enough of the person's devices approved".into());
    }
    Ok(out)
}

/// When a walk reads its records: now, at an authenticated earlier moment,
/// or for their structure alone (that they were made by the rule, revoked or
/// not, which is what naming a record as superseded or revoked needs).
#[derive(Clone, Copy, Debug)]
enum Mode {
    Now,
    At(i64),
    Structure,
}

/// One verification walk: the records on the way back from the one being
/// verified (so a record met again on its own way back refuses as a cycle),
/// the records already found standing (read once, however many approvers
/// lead to them), and how many records it has read (rule 3).
struct Walk {
    mode: Mode,
    path: Vec<ActionHash>,
    stood: std::collections::HashMap<ActionHash, (DeviceBinding, Authority)>,
    visits: usize,
}
impl Walk {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            path: Vec::new(),
            stood: std::collections::HashMap::new(),
            visits: 0,
        }
    }
    fn enter(&mut self, hash: &ActionHash, depth: usize) -> ExternResult<()> {
        if depth > MAX_APPROVAL_DEPTH {
            return Err(refuse("device approval chain exceeds depth bound"));
        }
        if self.path.contains(hash) {
            return Err(refuse("device approval chain cycles"));
        }
        self.visits += 1;
        if self.visits > MAX_APPROVAL_VISITS {
            return Err(refuse("device approval walk exceeds work bound"));
        }
        self.path.push(hash.clone());
        Ok(())
    }
    fn stood(&mut self, hash: ActionHash, found: &(DeviceBinding, Authority)) {
        self.path.retain(|h| h != &hash);
        self.stood.insert(hash, found.clone());
    }
    fn authority(&self, hash: ActionHash) -> ExternResult<Authority> {
        match self.mode {
            Mode::Now => current_authority(hash),
            Mode::At(at) => authority_at(hash, at),
            Mode::Structure => authority_history(hash, 0).map(|(a, _)| a),
        }
    }
}

/// Rule 2 on one binding: every approval's signature, and that each device
/// approver's own joining record stands under the same walk.
fn check_approvals(
    authority: &Authority,
    binding: &DeviceBinding,
    walk: &mut Walk,
    depth: usize,
) -> ExternResult<()> {
    let classes = classify_approvers(
        authority,
        &binding.intent,
        &binding.controllers,
        &binding.approved_via,
    )
    .map_err(|e| refuse(&e))?;
    let message = bytes(&binding.intent)?;
    for (proof, class) in binding.controllers.iter().zip(classes) {
        if !verify_signature_raw(
            proof.agent.clone(),
            proof.signature.clone(),
            message.clone(),
        )? {
            return Err(refuse("forged identity proof"));
        }
        if let Approver::Device(via) = class {
            let (approver, _) = binding_stands(via, walk, depth + 1)?;
            if approver.intent.device_key != proof.agent
                || approver.intent.identity_root != binding.intent.identity_root
            {
                return Err(refuse(
                    "an approver's joining record is for another device or identity",
                ));
            }
        }
    }
    Ok(())
}

/// Rules 1 and 2 on a binding's content, under an authority already read.
fn binding_proofs(
    binding: &DeviceBinding,
    authority: &Authority,
    walk: &mut Walk,
    depth: usize,
) -> ExternResult<()> {
    if binding.action != "binds-identity" || binding.binding_kind != "device-v1" {
        return Err(refuse("not an additive device binding"));
    }
    check_intent_context(&binding.intent)?;
    check_intent_identity(&binding.intent, authority)?;
    let message = bytes(&binding.intent)?;
    if binding.possession.agent != binding.intent.device_key
        || !verify_signature_raw(
            binding.possession.agent.clone(),
            binding.possession.signature.clone(),
            message,
        )?
    {
        return Err(refuse("device key possession not proven"));
    }
    check_approvals(authority, binding, walk, depth)
}

/// Where a device's revocations are found: one anchor per device of an
/// identity, from the identity root and the device key, the two facts every
/// joining record of that device signs. Revocation withdraws the device's
/// voice for the identity, not one record's bytes: a record republished with
/// other unsigned parts (another `approved_via`, so another entry hash), or a
/// second joining record of the same key, finds the same revocations. A
/// revoked device does not come back by being joined again; re-enrolment is
/// a design of its own (`supersedes`, not built).
fn revocation_anchor(
    identity_root: &ActionHash,
    device_key: &AgentPubKey,
) -> ExternResult<AnyLinkableHash> {
    // The address of a Commitment that is never written: deterministic from
    // the two signed facts, in the integrity zome's own entry encoding.
    let anchor = Commitment {
        action: "device-revocation-anchor".into(),
        payload_json: String::from_utf8(bytes(&(identity_root, device_key))?)
            .map_err(|_| refuse("identity encoding"))?,
        signed_at: "device-revocation-anchor-v1".into(),
    };
    Ok(hash_entry(&EntryTypes::Commitment(anchor))?.into())
}

/// Whether `binding`'s device has been revoked for its identity by the
/// controllers of `authority`, read under `mode`: now, any revocation
/// refuses; at an authenticated moment, only one made at or before it.
/// Revocations are found on the device's anchor and, for revocations made
/// before the anchor existed, on the record's own entry; each is checked
/// against the device and identity its target signed, never against bytes.
fn device_revoked(
    binding: &DeviceBinding,
    entry: &Commitment,
    authority: &Authority,
    mode: Mode,
) -> ExternResult<()> {
    if matches!(mode, Mode::Structure) {
        return Ok(());
    }
    let mut found = lifecycle(
        revocation_anchor(&binding.intent.identity_root, &binding.intent.device_key)?,
        &authority.controllers,
    )?;
    for target in lifecycle(hash_entry(entry)?, &authority.controllers)? {
        if !found.contains(&target) {
            found.push(target);
        }
    }
    for target in found {
        // A discovered lifecycle prerequisite that cannot be fetched is
        // pending, never evidence of absence.
        let (revocation_record, _, revocation): (_, _, DeviceRevocation) =
            record(target, "revokes-commitment")?;
        let (_, revoked) = binding_record(revocation.target.clone())?;
        if revoked.intent.identity_root != binding.intent.identity_root
            || revoked.intent.device_key != binding.intent.device_key
        {
            continue;
        }
        verify_revocation(&revocation, &revoked)?;
        match mode {
            Mode::At(at) if revocation_record.action().timestamp().as_micros() > at => {}
            Mode::At(_) => return Err(refuse("device withdrawal preceded witnessed exercise")),
            _ => return Err(refuse("device binding revoked")),
        }
    }
    Ok(())
}

/// Whether the joining record at `hash` stands by the module's rule, under
/// `walk`'s mode. Returns the record and the authority it stands under.
fn binding_stands(
    hash: ActionHash,
    walk: &mut Walk,
    depth: usize,
) -> ExternResult<(DeviceBinding, Authority)> {
    if let Some(found) = walk.stood.get(&hash) {
        return Ok(found.clone());
    }
    walk.enter(&hash, depth)?;
    let (native, entry, binding): (_, _, DeviceBinding) = record(hash.clone(), "binds-identity")?;
    if entry.payload_json.as_bytes() != bytes(&binding)?
        || entry.signed_at != binding.intent.issued_at.as_micros().to_string()
    {
        return Err(refuse("noncanonical device binding envelope"));
    }
    // A device joins itself: its record is on its own chain. A copy published
    // by anyone else is not a joining record.
    if native.action().author() != &binding.intent.device_key {
        return Err(refuse("a joining record is published by its own device"));
    }
    if let Mode::At(at) = walk.mode {
        if native.action().timestamp().as_micros() > at {
            return Err(refuse("device joining postdates the witnessed moment"));
        }
    }
    let authority = walk.authority(binding.intent.authority.clone())?;
    binding_proofs(&binding, &authority, walk, depth)?;
    device_revoked(&binding, &entry, &authority, walk.mode)?;
    let found = (binding, authority);
    walk.stood(hash, &found);
    Ok(found)
}

/// This cell's own joining records, newest first, at most 16: the device
/// enrolls itself, so its records are on its own chain.
fn my_bindings() -> ExternResult<Vec<(ActionHash, DeviceBinding)>> {
    let me = agent_info()?.agent_initial_pubkey;
    let expected: ScopedEntryDefIndex = mishpat_integrity::UnitEntryTypes::Commitment.try_into()?;
    let records = query(
        ChainQueryFilter::new()
            .entry_type(EntryType::App(AppEntryDef::new(
                expected.zome_type,
                expected.zome_index,
                EntryVisibility::Public,
            )))
            .include_entries(true),
    )?;
    let mut out = Vec::new();
    for record in records.into_iter().rev() {
        let Ok(Some(entry)) = record.entry().to_app_option::<Commitment>() else {
            continue;
        };
        if entry.action != "binds-identity" {
            continue;
        }
        let Ok(binding) = serde_json::from_str::<DeviceBinding>(&entry.payload_json) else {
            continue;
        };
        if binding.binding_kind == "device-v1" && binding.intent.device_key == me {
            out.push((record.action_address().clone(), binding));
            if out.len() >= 16 {
                break;
            }
        }
    }
    Ok(out)
}

/// How this cell speaks for the identity `authority` belongs to: as one of
/// its root controllers (`Some(None)`), through its own joining record that
/// stands now (`Some(Some(record))`), or not at all (`None`).
fn my_voice(authority: &Authority) -> ExternResult<Option<Option<ActionHash>>> {
    let me = agent_info()?.agent_initial_pubkey;
    if authority.controllers.contains(&me) {
        return Ok(Some(None));
    }
    for (hash, binding) in my_bindings()? {
        if binding.intent.identity_root != authority.chain_root {
            continue;
        }
        if binding_stands(hash.clone(), &mut Walk::new(Mode::Now), 0).is_ok() {
            return Ok(Some(Some(hash)));
        }
    }
    Ok(None)
}

fn verify_quorum(authority: &Authority, proofs: &[Proof], message: Vec<u8>) -> ExternResult<()> {
    authorized_signers(authority, proofs).map_err(|e| refuse(&e))?;
    for proof in proofs {
        if !verify_signature_raw(
            proof.agent.clone(),
            proof.signature.clone(),
            message.clone(),
        )? {
            return Err(refuse("forged identity proof"));
        }
    }
    Ok(())
}
fn authority_message(authority: &Authority) -> ExternResult<Vec<u8>> {
    let mut unsigned = authority.clone();
    unsigned.signatures.clear();
    bytes(&unsigned)
}
fn lifecycle(
    hash: impl Into<AnyLinkableHash>,
    controllers: &[AgentPubKey],
) -> ExternResult<Vec<ActionHash>> {
    // Deleting a discoverability link cannot erase an authorized revocation or
    // policy successor. Generic link integrity does not protect deletion.
    let details = get_links_details(
        LinkQuery::try_new(hash.into(), LinkTypes::CommitmentByState)?,
        GetStrategy::Network,
    )?
    .into_inner();
    let details: Vec<_> = details
        .into_iter()
        .filter(|(create, _)| controllers.contains(create.action().author()))
        .collect();
    if details.len() > MAX_LIFECYCLE {
        return Err(refuse("identity lifecycle exceeds verification bound"));
    }
    Ok(details
        .into_iter()
        .filter_map(|(create, _deletes)| match &create.action().data {
            ActionData::CreateLink(link) => link.target_address.clone().into_action_hash(),
            _ => None,
        })
        .collect())
}

// Bootstrap bytes are deterministic for the immutable Human root. Repeating it
// produces the SAME EntryHash and therefore the SAME lifecycle, never a fresh
// policy history that bypasses a controller change.
fn bootstrap_payload(human: &HumanRootEvidence, network: DnaHash) -> Authority {
    Authority {
        action: "binds-identity".into(),
        binding_kind: "authority-v1".into(),
        chain_root: human.human_action_hash.clone(),
        human_id: human.human_id.clone(),
        human_dna: human.dna_hash.clone(),
        network_dna: network,
        head_key: human.author.clone(),
        controllers: vec![human.author.clone()],
        controller_policy: ControllerPolicy {
            kind: "self".into(),
            m: None,
            n: None,
        },
        authorization: "development-network".into(),
        previous_authority: None,
        signatures: vec![],
    }
}
fn authority_history(hash: ActionHash, depth: usize) -> ExternResult<(Authority, EntryHash)> {
    if depth >= 32 {
        return Err(refuse("identity authority ancestry exceeds bound"));
    }
    let (record, entry, authority): (_, _, Authority) = record(hash, "binds-identity")?;
    if authority.binding_kind != "authority-v1" || authority.network_dna != dna_info()?.hash {
        return Err(refuse("identity authority context mismatch"));
    }
    threshold(&authority.controller_policy, &authority.controllers).map_err(|e| refuse(&e))?;
    if entry.payload_json.as_bytes() != bytes(&authority)? {
        return Err(refuse("noncanonical identity authority encoding"));
    }
    if let Some(previous) = authority.previous_authority.clone() {
        if entry.signed_at != "authority-v1" {
            return Err(refuse("noncanonical successor authority envelope"));
        }
        let (parent, _) = authority_history(previous, depth + 1)?;
        if authority.chain_root != parent.chain_root
            || authority.human_dna != parent.human_dna
            || authority.human_id != parent.human_id
        {
            return Err(refuse("identity authority changed its Human root"));
        }
        verify_quorum(
            &parent,
            &authority.signatures,
            authority_message(&authority)?,
        )?;
    } else {
        let human = human_root(authority.chain_root.clone())?;
        let expected = bootstrap_payload(&human, dna_info()?.hash);
        if bytes(&authority)? != bytes(&expected)?
            || record.action().author() != &human.author
            || entry.signed_at != human.timestamp.as_micros().to_string()
        {
            return Err(refuse(
                "identity bootstrap is not authorized by exact Human author",
            ));
        }
    }
    Ok((authority, hash_entry(&entry)?))
}
fn current_authority(hash: ActionHash) -> ExternResult<Authority> {
    let (authority, anchor) = authority_history(hash.clone(), 0)?;
    let human = human_root(authority.chain_root.clone())?;
    let current_key: AgentPubKey = match call(
        CallTargetCell::OtherRole("imagodei".into()),
        ZomeName::from("imagodei"),
        "identity_head".into(),
        None,
        human.author,
    )? {
        ZomeCallResponse::Ok(result) => result
            .decode()
            .map_err(|_| refuse("current identity head decode failed"))?,
        _ => return Err(refuse("current identity head unavailable")),
    };
    if current_key != authority.head_key {
        return Err(refuse(
            "identity key changed; current controller authority required",
        ));
    }
    // A successor is checked independently of the untrusted link's tag/author.
    // Any valid successor fences the old policy; competing successors fail closed.
    for target in lifecycle(anchor.clone(), &authority.controllers)? {
        let (_, _, candidate): (_, _, Authority) = record(target.clone(), "binds-identity")?;
        if let Some(previous) = &candidate.previous_authority {
            let (_, previous_entry) = authority_history(previous.clone(), 0)?;
            if previous_entry == anchor {
                authority_history(target, 0)?;
                return Err(refuse(
                    "identity authority superseded; resolve current controller policy",
                ));
            }
        }
    }
    // Every predecessor must select this branch, not just the immediate one.
    // Deduplicate by entry identity so replaying a signed action is not a fork.
    let mut child = authority.clone();
    let mut child_anchor = anchor;
    while let Some(previous) = child.previous_authority.clone() {
        let (parent, parent_anchor) = authority_history(previous, 0)?;
        let mut accepted = std::collections::HashSet::new();
        for target in lifecycle(parent_anchor.clone(), &parent.controllers)? {
            let (_, _, candidate): (_, _, Authority) = record(target.clone(), "binds-identity")?;
            if let Some(candidate_parent) = candidate.previous_authority {
                let (_, candidate_parent_entry) = authority_history(candidate_parent, 0)?;
                if candidate_parent_entry == parent_anchor {
                    let (_, entry) = authority_history(target, 0)?;
                    accepted.insert(entry);
                }
            }
        }
        if accepted.len() != 1 || !accepted.contains(&child_anchor) {
            return Err(refuse("identity controller history contested or unlinked"));
        }
        child = parent;
        child_anchor = parent_anchor;
    }
    Ok(authority)
}
/// Resolve the controller policy at an already authenticated native witness.
/// Later successors do not erase history; an earlier successor or fork fences it.
fn authority_at(hash: ActionHash, witnessed_at: i64) -> ExternResult<Authority> {
    let (native, _, _): (_, _, Authority) = record(hash.clone(), "binds-identity")?;
    if native.action().timestamp().as_micros() > witnessed_at {
        return Err(refuse("controller authority postdates witnessed exercise"));
    }
    let (authority, anchor) = authority_history(hash, 0)?;
    for target in lifecycle(anchor.clone(), &authority.controllers)? {
        let (native, _, candidate): (_, _, Authority) = record(target.clone(), "binds-identity")?;
        if native.action().timestamp().as_micros() <= witnessed_at {
            if let Some(previous) = candidate.previous_authority {
                let (_, previous_anchor) = authority_history(previous, 0)?;
                if previous_anchor == anchor {
                    authority_history(target, 0)?;
                    return Err(refuse(
                        "controller policy superseded before witnessed exercise",
                    ));
                }
            }
        }
    }
    let mut child = authority.clone();
    let mut child_anchor = anchor;
    while let Some(previous) = child.previous_authority.clone() {
        let (parent, parent_anchor) = authority_history(previous, 0)?;
        let mut accepted = std::collections::HashSet::new();
        for target in lifecycle(parent_anchor.clone(), &parent.controllers)? {
            let (native, _, candidate): (_, _, Authority) =
                record(target.clone(), "binds-identity")?;
            if native.action().timestamp().as_micros() > witnessed_at {
                continue;
            }
            if let Some(candidate_parent) = candidate.previous_authority {
                let (_, candidate_anchor) = authority_history(candidate_parent, 0)?;
                if candidate_anchor == parent_anchor {
                    let (_, entry) = authority_history(target, 0)?;
                    accepted.insert(entry);
                }
            }
        }
        if accepted.len() != 1 || !accepted.contains(&child_anchor) {
            return Err(refuse("historical controller branch contested or unlinked"));
        }
        child = parent;
        child_anchor = parent_anchor;
    }
    Ok(authority)
}
fn notarize<T: Serialize>(
    payload: &T,
    action: &str,
    signed_at: String,
) -> ExternResult<CommitmentOutput> {
    let entry = Commitment {
        action: action.into(),
        payload_json: String::from_utf8(bytes(payload)?)
            .map_err(|_| refuse("identity encoding"))?,
        signed_at,
    };
    let action_hash = create_entry(&EntryTypes::Commitment(entry.clone()))?;
    Ok(CommitmentOutput {
        action_hash,
        entry_hash: hash_entry(&entry)?,
    })
}

#[hdk_extern]
pub fn bootstrap_device_identity(human_action_hash: ActionHash) -> ExternResult<CommitmentOutput> {
    crate::invocation::authorize("bootstrap_device_identity", &human_action_hash)?;
    let human = human_root(human_action_hash)?;
    if human.author != agent_info()?.agent_initial_pubkey {
        return Err(refuse(
            "only exact Human author can bootstrap device identity",
        ));
    }
    let authority = bootstrap_payload(&human, dna_info()?.hash);
    // A repeat creates the same entry; current_authority subsequently checks its
    // shared lifecycle before this reference may confer authority.
    notarize(
        &authority,
        "binds-identity",
        human.timestamp.as_micros().to_string(),
    )
}
/// What this cell's own person stands on when they agree to a device consent:
/// the identity they are, and the authority under which its controllers act.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsentStanding {
    /// The Human record the identity is rooted in.
    pub identity_root: ActionHash,
    /// The identity's current authority record. `None` until
    /// `bootstrap_device_identity` has run for this person.
    pub authority: Option<ActionHash>,
    /// Who may agree for the identity, and how many of them must.
    pub controllers: Vec<AgentPubKey>,
    pub required: usize,
    /// The network the authority is kept on, which a device must name.
    pub network_dna: DnaHash,
    /// When this cell speaks for the identity as a device rather than a root
    /// controller, the joining record it speaks through. It is then counted
    /// in `controllers`, whose meaning is "devices that speak for the person"
    /// as far as this cell can say without reading the network's list
    /// (`identity_devices` reads that).
    #[serde(default)]
    pub speaks_via: Option<ActionHash>,
    /// Other identities this cell also speaks for through a joining record
    /// (a node that began an identity of its own and joined another as it
    /// is). Approvals made here are for `identity_root`, never these.
    #[serde(default)]
    pub also_speaks_for: Vec<ActionHash>,
}

#[derive(Deserialize, Debug)]
struct MyHuman {
    action_hash: ActionHash,
}

fn my_human() -> ExternResult<Option<ActionHash>> {
    match call(
        CallTargetCell::OtherRole("imagodei".into()),
        ZomeName::from("imagodei"),
        "get_my_human".into(),
        None,
        (),
    )? {
        ZomeCallResponse::Ok(result) => Ok(result
            .decode::<Option<MyHuman>>()
            .map_err(|_| refuse("Human decode failed"))?
            .map(|human| human.action_hash)),
        _ => Err(refuse("Human unavailable")),
    }
}

/// The action that bootstrapped `human`'s authority, if one exists.
///
/// The bootstrap entry is deterministic for its Human, so its address is
/// computed rather than searched for. The author's own node finds it in its
/// own store; another node asks the network. A repeated bootstrap is the same
/// entry under another action; the earliest is taken, and every one of them
/// resolves to the same authority.
fn bootstrap_action(
    human: &HumanRootEvidence,
    network: DnaHash,
) -> ExternResult<Option<ActionHash>> {
    let entry = Commitment {
        action: "binds-identity".into(),
        payload_json: String::from_utf8(bytes(&bootstrap_payload(human, network))?)
            .map_err(|_| refuse("identity encoding"))?,
        signed_at: human.timestamp.as_micros().to_string(),
    };
    let options = if human.author == agent_info()?.agent_initial_pubkey {
        GetOptions::local()
    } else {
        GetOptions::network()
    };
    let Some(Details::Entry(details)) = get_details(hash_entry(&entry)?, options)? else {
        return Ok(None);
    };
    if details.actions.len() > MAX_LIFECYCLE {
        return Err(refuse("identity bootstrap exceeds verification bound"));
    }
    Ok(details
        .actions
        .iter()
        .filter(|a| {
            a.action().author() == &human.author && matches!(a.action().data, ActionData::Create(_))
        })
        .min_by_key(|a| (a.action().timestamp(), a.as_hash().clone()))
        .map(|a| a.as_hash().clone()))
}

/// Follow an authority to the successor that currently governs it.
///
/// Nothing creates a successor today, so this returns `start`. When controller
/// policies can change, a person must agree under the current one; this walk
/// finds it, and `current_authority` then checks it in full. Competing
/// successors are refused rather than chosen between.
fn current_successor(start: ActionHash) -> ExternResult<ActionHash> {
    let mut current = start;
    for _ in 0..32 {
        let (authority, anchor) = authority_history(current.clone(), 0)?;
        let mut next: Option<(ActionHash, EntryHash)> = None;
        for target in lifecycle(anchor.clone(), &authority.controllers)? {
            let (_, _, candidate): (_, _, Authority) = record(target.clone(), "binds-identity")?;
            let Some(previous) = candidate.previous_authority else {
                continue;
            };
            if authority_history(previous, 0)?.1 != anchor {
                continue;
            }
            let (_, successor) = authority_history(target.clone(), 0)?;
            match &next {
                // The same successor published again is not a fork.
                Some((_, seen)) if seen == &successor => {}
                Some(_) => {
                    return Err(refuse(
                        "identity authority contested; resolve current controller policy",
                    ))
                }
                None => next = Some((target, successor)),
            }
        }
        match next {
            Some((successor, _)) => current = successor,
            None => return Ok(current),
        }
    }
    Err(refuse("identity authority succession exceeds bound"))
}

/// What this node's person needs in order to agree to a device consent.
///
/// `None` when this cell's agent has no Human. A read: nothing is written to
/// any chain, and an absent authority is reported, never created.
#[hdk_extern]
pub fn my_consent_standing(_: ()) -> ExternResult<Option<ConsentStanding>> {
    let network_dna = dna_info()?.hash;
    // The identities this cell speaks for through its own joining records.
    let mut joined: Vec<(ActionHash, ActionHash)> = Vec::new();
    for (hash, binding) in my_bindings()? {
        if joined
            .iter()
            .any(|(root, _)| root == &binding.intent.identity_root)
        {
            continue;
        }
        if binding_stands(hash.clone(), &mut Walk::new(Mode::Now), 0).is_ok() {
            joined.push((binding.intent.identity_root.clone(), hash));
        }
    }
    // The identity approvals are for: the one this cell's key resolves to (its
    // own, or the person's it registered as a device of), else the first it
    // joined.
    let identity_root = match my_human()? {
        Some(root) => root,
        None => match joined.first() {
            Some((root, _)) => root.clone(),
            None => return Ok(None),
        },
    };
    let also_speaks_for: Vec<ActionHash> = joined
        .iter()
        .filter(|(root, _)| root != &identity_root)
        .map(|(root, _)| root.clone())
        .collect();
    let human = human_root(identity_root.clone())?;
    let Some(bootstrap) = bootstrap_action(&human, network_dna.clone())? else {
        return Ok(Some(ConsentStanding {
            identity_root,
            authority: None,
            controllers: Vec::new(),
            required: 0,
            network_dna,
            speaks_via: None,
            also_speaks_for,
        }));
    };
    let current = current_successor(bootstrap)?;
    let authority = current_authority(current.clone())?;
    let required =
        threshold(&authority.controller_policy, &authority.controllers).map_err(|e| refuse(&e))?;
    let me = agent_info()?.agent_initial_pubkey;
    let mut controllers = authority.controllers.clone();
    let speaks_via = if controllers.contains(&me) {
        None
    } else {
        let via = joined
            .iter()
            .find(|(root, _)| root == &authority.chain_root)
            .map(|(_, hash)| hash.clone());
        if via.is_some() {
            controllers.push(me);
        }
        via
    };
    Ok(Some(ConsentStanding {
        identity_root: authority.chain_root,
        authority: Some(current),
        controllers,
        required,
        network_dna,
        speaks_via,
        also_speaks_for,
    }))
}

/// One device that speaks for a person, as the network shows it.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityDevice {
    pub device_key: AgentPubKey,
    pub binding: ActionHash,
    /// The content network the device joined on, which verifying or affirming
    /// it names.
    pub content_dna: DnaHash,
    /// Who approved it: root controllers or devices of the person.
    pub approved_by: Vec<AgentPubKey>,
    /// When it joined (its joining record's own time).
    pub joined_at: Timestamp,
    /// The person's other devices that have affirmed it since: distinct,
    /// each speaking for the person, neither the device nor its approvers.
    pub affirmed_by: Vec<AgentPubKey>,
}

/// The devices that speak for a person, found from the identity's root and
/// each verified by the module's rule.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityDevices {
    pub identity_root: ActionHash,
    /// The authority's root controllers.
    pub roots: Vec<AgentPubKey>,
    pub devices: Vec<IdentityDevice>,
    /// Discovery links followed whose record did not stand (or could not be
    /// read yet): never counted, never shown as a device.
    pub not_standing: u32,
    /// Whether more links existed than one read follows.
    pub truncated: bool,
}

/// Discovery links under `base` whose tag starts with `prefix`, with their
/// authors: only links whose author `admits` names, at most `per_author`
/// from one author and `cap` in all, earliest first. Discovery is not proof;
/// filtering here keeps a flood from anyone from hiding the links that
/// matter or costing a walk each.
fn discovery(
    base: AnyLinkableHash,
    prefix: &str,
    admits: &dyn Fn(&AgentPubKey) -> bool,
    per_author: usize,
    cap: usize,
) -> ExternResult<(Vec<(AgentPubKey, ActionHash)>, bool)> {
    let links = get_links(
        LinkQuery::try_new(base, LinkTypes::CommitmentByState)?,
        GetStrategy::Network,
    )?;
    let mut targets: Vec<(Timestamp, AgentPubKey, ActionHash)> = links
        .into_iter()
        .filter(|l| l.tag.0.starts_with(prefix.as_bytes()) && admits(&l.author))
        .filter_map(|l| {
            l.target
                .into_action_hash()
                .map(|t| (l.timestamp, l.author, t))
        })
        .collect();
    targets.sort_by(|a, b| a.0.cmp(&b.0));
    let mut kept: Vec<(AgentPubKey, ActionHash)> = Vec::new();
    let mut truncated = false;
    for (_, author, target) in targets {
        if kept.iter().any(|(_, t)| t == &target) {
            continue;
        }
        if kept.iter().filter(|(a, _)| a == &author).count() >= per_author {
            continue;
        }
        if kept.len() >= cap {
            truncated = true;
            break;
        }
        kept.push((author, target));
    }
    Ok((kept, truncated))
}

/// How many discovery links one author's are followed: a device re-joins
/// rarely; one affirmation per witness counts.
const DEVICE_LINKS_PER_AUTHOR: usize = 2;
const AFFIRMATIONS_PER_AUTHOR: usize = 1;

/// Every device that speaks for the person rooted at `identity_root`, with
/// who approved it and who has affirmed it. A read; bounded by
/// `MAX_DEVICE_LINKS` records (each walked within the rule's bounds) and
/// `MAX_AFFIRMATION_LINKS` affirmations per device.
#[hdk_extern]
pub fn identity_devices(identity_root: ActionHash) -> ExternResult<IdentityDevices> {
    let human = human_root(identity_root.clone())?;
    let bootstrap = bootstrap_action(&human, dna_info()?.hash)?
        .ok_or_else(|| refuse("identity has no authority yet"))?;
    let authority = current_authority(current_successor(bootstrap)?)?;
    // A device links its own joining record; anyone may write a link, so only
    // a link whose author is the device the record joins is followed.
    let (targets, truncated) = discovery(
        identity_root.clone().into(),
        "device|",
        &|_| true,
        DEVICE_LINKS_PER_AUTHOR,
        MAX_DEVICE_LINKS,
    )?;
    let mut devices: Vec<IdentityDevice> = Vec::new();
    let mut not_standing = 0u32;
    for (author, hash) in targets {
        match binding_stands(hash.clone(), &mut Walk::new(Mode::Now), 0) {
            Ok((binding, _))
                if binding.intent.identity_root == identity_root
                    && binding.intent.device_key == author =>
            {
                if devices
                    .iter()
                    .any(|d| d.device_key == binding.intent.device_key)
                {
                    continue;
                }
                let joined_at = get_commitment_record(hash.clone())?
                    .map(|r| r.action().timestamp())
                    .unwrap_or(binding.intent.issued_at);
                devices.push(IdentityDevice {
                    device_key: binding.intent.device_key.clone(),
                    binding: hash,
                    content_dna: binding.intent.content_dna.clone(),
                    approved_by: binding
                        .controllers
                        .iter()
                        .map(|p| p.agent.clone())
                        .collect(),
                    joined_at,
                    affirmed_by: Vec::new(),
                });
            }
            _ => not_standing += 1,
        }
    }
    let speakers: Vec<AgentPubKey> = authority
        .controllers
        .iter()
        .cloned()
        .chain(devices.iter().map(|d| d.device_key.clone()))
        .collect();
    for device in devices.iter_mut() {
        // Only a speaker's affirmation counts, so only a speaker's link is
        // followed: links from anyone else cannot crowd them out.
        let (affirmations, _) = discovery(
            device.binding.clone().into(),
            "affirmed|",
            &|author| speakers.contains(author),
            AFFIRMATIONS_PER_AUTHOR,
            MAX_AFFIRMATION_LINKS,
        )?;
        for (_, action) in affirmations {
            let Ok(affirmation) = verify_device_affirmation(action) else {
                continue;
            };
            let witness = affirmation.witness;
            if affirmation.device.binding_action_hash == device.binding
                && speakers.contains(&witness)
                && witness != device.device_key
                && !device.approved_by.contains(&witness)
                && !device.affirmed_by.contains(&witness)
            {
                device.affirmed_by.push(witness);
            }
        }
    }
    Ok(IdentityDevices {
        identity_root,
        roots: authority.controllers,
        devices,
        not_standing,
        truncated,
    })
}

fn binding_record(hash: ActionHash) -> ExternResult<(Commitment, DeviceBinding)> {
    let (_, entry, binding): (_, _, DeviceBinding) = record(hash, "binds-identity")?;
    if entry.payload_json.as_bytes() != bytes(&binding)?
        || entry.signed_at != binding.intent.issued_at.as_micros().to_string()
    {
        return Err(refuse("noncanonical device binding envelope"));
    }
    Ok((entry, binding))
}
/// That a binding was made by the rule, revoked or not: its approvals and
/// their approvers' records checked for structure alone (rule 2, rule 3).
fn historical_binding_proofs(binding: &DeviceBinding) -> ExternResult<()> {
    let (authority, _) = authority_history(binding.intent.authority.clone(), 0)?;
    if binding.intent.identity_root != authority.chain_root {
        return Err(refuse("invalid historical device binding context"));
    }
    binding_proofs(binding, &authority, &mut Walk::new(Mode::Structure), 0)
}
fn validate_intent(intent: &DeviceIntent) -> ExternResult<Authority> {
    check_intent_context(intent)?;
    let authority = current_authority(intent.authority.clone())?;
    check_intent_identity(intent, &authority)?;
    Ok(authority)
}
fn check_intent_context(intent: &DeviceIntent) -> ExternResult<()> {
    if intent.domain != DOMAIN || intent.network_dna != dna_info()?.hash {
        return Err(refuse("device binding network/domain mismatch"));
    }
    Ok(())
}
fn check_intent_identity(intent: &DeviceIntent, authority: &Authority) -> ExternResult<()> {
    if intent.identity_root != authority.chain_root {
        return Err(refuse("device binding identity mismatch"));
    }
    if let Some(previous) = &intent.supersedes {
        let (_, binding) = binding_record(previous.clone())?;
        historical_binding_proofs(&binding)?;
        if binding.intent.identity_root != intent.identity_root
            || binding.intent.device_key != intent.device_key
        {
            return Err(refuse("superseded device binding has different subject"));
        }
    }
    Ok(())
}
/// This cell's signature on an intent already checked against `authority`:
/// the joining device's possession, or an approval from a root controller or
/// a device that speaks for the person, with the record it speaks through.
fn enrollment_proof(
    intent: &DeviceIntent,
    authority: &Authority,
) -> ExternResult<(Proof, Option<ActionHash>)> {
    let me = agent_info()?.agent_initial_pubkey;
    let via = if me == intent.device_key {
        None
    } else {
        match my_voice(authority)? {
            Some(via) => via,
            None => return Err(refuse("caller cannot authorize this device binding")),
        }
    };
    Ok((
        Proof {
            signature: hdk::ed25519::sign_raw(me.clone(), bytes(intent)?)?,
            agent: me,
        },
        via,
    ))
}
/// Explicit ceremony signing only. Ordinary publication never calls this.
#[hdk_extern]
pub fn sign_device_enrollment(intent: DeviceIntent) -> ExternResult<Proof> {
    crate::invocation::authorize("sign_device_enrollment", &intent)?;
    let authority = validate_intent(&intent)?;
    enrollment_proof(&intent, &authority).map(|(proof, _)| proof)
}
/// What a controller signs to put their agreement on a device consent.
///
/// A consent record is the content-addressed statement of what a device asked
/// for and what the person agreed to. It is evidence, kept apart from the
/// binding that actually recognizes the device. The controller signs the
/// record's address, which commits to every byte of it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DeviceConsent {
    pub authority: ActionHash,
    pub identity_root: ActionHash,
    /// The consent record's content address (CIDv1, dag-cbor).
    pub consent_cid: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedDeviceConsent {
    pub consent: DeviceConsent,
    pub proof: Proof,
    /// When the signer is a device rather than a root controller, the joining
    /// record it speaks through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<ActionHash>,
}

const CONSENT_DOMAIN: &str = "elohim:device-consent:v1:";
/// A CIDv1 with a sha2-256 digest is 59 characters in base32; the ceiling
/// leaves room for another codec and no room for anything else.
const MAX_CONSENT_CID_LEN: usize = 96;

/// The exact bytes signed. Domain-separated, so a signature gathered for a
/// consent can never be replayed as an enrollment or any other identity act.
fn consent_message(consent: &DeviceConsent) -> Result<Vec<u8>, &'static str> {
    let cid = consent.consent_cid.as_bytes();
    let plain = !cid.is_empty()
        && cid.len() <= MAX_CONSENT_CID_LEN
        && cid
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    if !plain {
        return Err("consent address is not a base32 content address");
    }
    let mut message = Vec::with_capacity(CONSENT_DOMAIN.len() + cid.len());
    message.extend_from_slice(CONSENT_DOMAIN.as_bytes());
    message.extend_from_slice(cid);
    Ok(message)
}

/// Whether `signer` may put this identity's agreement on `consent` as a root
/// controller: the consent must name the identity the authority belongs to.
/// A device that speaks for the person agrees through its joining record
/// instead ([`consent_signer_speaks`]).
#[cfg(test)]
fn consent_signer_stands(
    authority: &Authority,
    consent: &DeviceConsent,
    signer: &AgentPubKey,
) -> Result<(), &'static str> {
    if authority.chain_root != consent.identity_root {
        return Err("consent names a different identity");
    }
    if !authority.controllers.contains(signer) {
        return Err("only a controller of this identity may agree for it");
    }
    Ok(())
}

/// Whether `signer` speaks for the identity `consent` names: a root
/// controller, or a device whose joining record `via` names it and stands now.
fn consent_signer_speaks(
    authority: &Authority,
    consent: &DeviceConsent,
    signer: &AgentPubKey,
    via: Option<&ActionHash>,
) -> ExternResult<bool> {
    if authority.chain_root != consent.identity_root {
        return Ok(false);
    }
    if authority.controllers.contains(signer) {
        return Ok(true);
    }
    let Some(via) = via else {
        return Ok(false);
    };
    let (binding, _) = binding_stands(via.clone(), &mut Walk::new(Mode::Now), 0)?;
    Ok(&binding.intent.device_key == signer
        && binding.intent.identity_root == consent.identity_root)
}

/// This cell's agreement on `consent`, under an authority already resolved,
/// and the joining record it speaks through when it is a device.
fn consent_proof(
    consent: &DeviceConsent,
    authority: &Authority,
) -> ExternResult<(Proof, Option<ActionHash>)> {
    let message = consent_message(consent).map_err(refuse)?;
    let me = agent_info()?.agent_initial_pubkey;
    if authority.chain_root != consent.identity_root {
        return Err(refuse("consent names a different identity"));
    }
    let Some(via) = my_voice(authority)? else {
        return Err(refuse(
            "only a device that speaks for this identity may agree for it",
        ));
    };
    Ok((
        Proof {
            signature: hdk::ed25519::sign_raw(me.clone(), message)?,
            agent: me,
        },
        via,
    ))
}

/// Explicit ceremony signing only: a controller agrees to one consent record.
#[hdk_extern]
pub fn sign_device_consent(consent: DeviceConsent) -> ExternResult<Proof> {
    crate::invocation::authorize("sign_device_consent", &consent)?;
    // A malformed address is refused before anything is read from the network.
    consent_message(&consent).map_err(refuse)?;
    let authority = current_authority(consent.authority.clone())?;
    consent_proof(&consent, &authority).map(|(proof, _)| proof)
}

/// Everything a controller signs when they approve a device, asked for once.
///
/// Approving puts the controller's agreement on the consent record and, when
/// enrollment was agreed, signs the enrollment the device will complete. Both
/// are signed under one mandate, so an approval costs the controller's chain one
/// capability grant rather than one per signature. Each signature is still over
/// its own domain-separated bytes, so neither can stand in for the other.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceApproval {
    pub consent: DeviceConsent,
    /// Present exactly when the person agreed to enroll the device.
    pub enrollment: Option<DeviceIntent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceApprovalProofs {
    pub consent: Proof,
    pub enrollment: Option<Proof>,
    /// When this cell approved as a device rather than a root controller, the
    /// joining record it speaks through; the device names it in its binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<ActionHash>,
}

/// An enrollment signed in an approval must be for the identity and authority
/// the consent names, so one approval cannot carry agreement for two people.
fn approval_coheres(approval: &DeviceApproval) -> Result<(), &'static str> {
    match &approval.enrollment {
        Some(intent)
            if intent.authority != approval.consent.authority
                || intent.identity_root != approval.consent.identity_root =>
        {
            Err("approval enrollment names a different identity than its consent")
        }
        _ => Ok(()),
    }
}

/// Explicit ceremony signing only: a controller approves a device in one step.
#[hdk_extern]
pub fn sign_device_approval(approval: DeviceApproval) -> ExternResult<DeviceApprovalProofs> {
    crate::invocation::authorize("sign_device_approval", &approval)?;
    approval_coheres(&approval).map_err(refuse)?;
    consent_message(&approval.consent).map_err(refuse)?;
    if let Some(intent) = &approval.enrollment {
        check_intent_context(intent)?;
    }
    let authority = current_authority(approval.consent.authority.clone())?;
    let (consent, via) = consent_proof(&approval.consent, &authority)?;
    let enrollment = match &approval.enrollment {
        Some(intent) => {
            check_intent_identity(intent, &authority)?;
            if intent.device_key == agent_info()?.agent_initial_pubkey {
                return Err(refuse("a device cannot approve its own joining"));
            }
            Some(enrollment_proof(intent, &authority)?.0)
        }
        None => None,
    };
    Ok(DeviceApprovalProofs {
        consent,
        enrollment,
        via,
    })
}

/// Whether a controller of the named identity signed this consent. Any holder
/// of a consent can ask; the answer is recomputed here and trusts nothing the
/// caller says about who the controllers are.
/// A statement a node makes over the device-consent carrier, signed by its
/// agent key so the receiver can tell which key says it from where
/// (`consent_grant::carrier`). The bytes signed are fixed here and in
/// `consent_grant::carrier::statement` alike: the domain, then kind, state,
/// transport id and challenge, one per line.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarrierStatement {
    pub kind: String,
    pub state: String,
    pub transport_id: String,
    pub challenge: String,
}

const CARRIER_DOMAIN: &str = "elohim:device-carrier:v1:";
const MAX_CARRIER_FIELD: usize = 128;

fn carrier_statement_bytes(statement: &CarrierStatement) -> Result<Vec<u8>, &'static str> {
    if !matches!(
        statement.kind.as_str(),
        "ask" | "listed" | "code" | "declined"
    ) {
        return Err("not a carrier statement kind");
    }
    let fits = |f: &str| f.len() <= MAX_CARRIER_FIELD && !f.contains('\n') && !f.contains('\r');
    if !(fits(&statement.state) && fits(&statement.transport_id) && fits(&statement.challenge))
        || statement.transport_id.is_empty()
    {
        return Err("carrier statement field malformed");
    }
    Ok(format!(
        "{CARRIER_DOMAIN}{}\n{}\n{}\n{}",
        statement.kind, statement.state, statement.transport_id, statement.challenge
    )
    .into_bytes())
}

/// Sign a carrier statement with this cell's key. Under the carrier's own
/// domain only, so it cannot stand for an enrollment, an approval, a consent
/// or anything else this key signs. Commits nothing.
#[hdk_extern]
pub fn sign_carrier_statement(statement: CarrierStatement) -> ExternResult<Proof> {
    crate::invocation::authorize("sign_carrier_statement", &statement)?;
    let message = carrier_statement_bytes(&statement).map_err(refuse)?;
    let me = agent_info()?.agent_initial_pubkey;
    Ok(Proof {
        signature: hdk::ed25519::sign_raw(me.clone(), message)?,
        agent: me,
    })
}

#[hdk_extern]
pub fn verify_device_consent(signed: SignedDeviceConsent) -> ExternResult<bool> {
    let message = consent_message(&signed.consent).map_err(refuse)?;
    let authority = current_authority(signed.consent.authority.clone())?;
    if !consent_signer_speaks(
        &authority,
        &signed.consent,
        &signed.proof.agent,
        signed.via.as_ref(),
    )? {
        return Ok(false);
    }
    verify_signature_raw(signed.proof.agent, signed.proof.signature, message)
}

/// A binding about to be notarized, checked now (rules 1 to 3).
fn verify_binding(binding: &DeviceBinding) -> ExternResult<Authority> {
    check_intent_context(&binding.intent)?;
    let authority = current_authority(binding.intent.authority.clone())?;
    binding_proofs(binding, &authority, &mut Walk::new(Mode::Now), 0)?;
    Ok(authority)
}
#[hdk_extern]
pub fn enroll_identity_device(binding: DeviceBinding) -> ExternResult<CommitmentOutput> {
    if agent_info()?.agent_initial_pubkey != binding.intent.device_key {
        return Err(refuse("a device enrolls itself, from its own chain"));
    }
    let authority = verify_binding(&binding)?;
    // A revoked device is not joined again by a new record of the same key.
    let unsigned = Commitment {
        action: "binds-identity".into(),
        payload_json: String::from_utf8(bytes(&binding)?)
            .map_err(|_| refuse("identity encoding"))?,
        signed_at: binding.intent.issued_at.as_micros().to_string(),
    };
    device_revoked(&binding, &unsigned, &authority, Mode::Now)?;
    let out = notarize(
        &binding,
        "binds-identity",
        binding.intent.issued_at.as_micros().to_string(),
    )?;
    // Discovery, not proof: the person's other devices find this joining
    // record from the identity's root and verify it themselves.
    create_link(
        binding.intent.identity_root.clone(),
        out.action_hash.clone(),
        LinkTypes::CommitmentByState,
        LinkTag::new(format!("device|{}", binding.intent.issued_at.as_micros())),
    )?;
    Ok(out)
}
fn revocation_message(revocation: &DeviceRevocation) -> ExternResult<Vec<u8>> {
    let mut unsigned = revocation.clone();
    unsigned.signatures.clear();
    bytes(&unsigned)
}
fn verify_revocation(revocation: &DeviceRevocation, binding: &DeviceBinding) -> ExternResult<()> {
    if revocation.action != "revokes-commitment"
        || revocation.binding_kind != "device-revocation-v1"
        || revocation.network_dna != dna_info()?.hash
    {
        return Err(refuse("invalid device revocation context"));
    }
    let (authority, _) = authority_history(revocation.authority.clone(), 0)?;
    if revocation.authority != binding.intent.authority {
        return Err(refuse("revocation must name binding controller policy"));
    }
    if authority.chain_root != binding.intent.identity_root {
        return Err(refuse("revocation names another identity"));
    }
    verify_quorum(
        &authority,
        &revocation.signatures,
        revocation_message(revocation)?,
    )
}
#[hdk_extern]
pub fn sign_device_revocation(revocation: DeviceRevocation) -> ExternResult<Proof> {
    crate::invocation::authorize("sign_device_revocation", &revocation)?;
    let (_, binding) = binding_record(revocation.target.clone())?;
    historical_binding_proofs(&binding)?;
    let authority = current_authority(revocation.authority.clone())?;
    let me = agent_info()?.agent_initial_pubkey;
    if authority.chain_root != binding.intent.identity_root || !authority.controllers.contains(&me)
    {
        return Err(refuse("caller cannot revoke this device"));
    }
    Ok(Proof {
        signature: hdk::ed25519::sign_raw(me.clone(), revocation_message(&revocation)?)?,
        agent: me,
    })
}
#[hdk_extern]
pub fn revoke_identity_device(revocation: DeviceRevocation) -> ExternResult<CommitmentOutput> {
    crate::invocation::authorize("revoke_identity_device", &revocation)?;
    let (entry, binding) = binding_record(revocation.target.clone())?;
    historical_binding_proofs(&binding)?;
    let authority = current_authority(revocation.authority.clone())?;
    if !authority
        .controllers
        .contains(&agent_info()?.agent_initial_pubkey)
    {
        return Err(refuse(
            "only an identity controller may publish revocation lifecycle",
        ));
    }
    verify_revocation(&revocation, &binding)?;
    // Preserve the existing revokes-commitment integrity contract. The signed
    // device proof binds the exact target action; target_cid is its derived
    // canonical entry address for existing Commitment projections.
    let mut payload =
        serde_json::to_value(&revocation).map_err(|_| refuse("revocation encoding"))?;
    payload["target_cid"] = serde_json::Value::String(hash_entry(&entry)?.to_string());
    let out = notarize(
        &payload,
        "revokes-commitment",
        sys_time()?.as_micros().to_string(),
    )?;
    create_link(
        hash_entry(&entry)?,
        out.action_hash.clone(),
        LinkTypes::CommitmentByState,
        LinkTag::new(format!("revoked|{}", sys_time()?.as_micros())),
    )?;
    // The device's own anchor: what every record of this device for this
    // identity is read against (`device_revoked`).
    create_link(
        revocation_anchor(&binding.intent.identity_root, &binding.intent.device_key)?,
        out.action_hash.clone(),
        LinkTypes::CommitmentByState,
        LinkTag::new(format!("revoked|{}", sys_time()?.as_micros())),
    )?;
    Ok(out)
}
#[hdk_extern]
pub fn verify_device_binding(input: VerifyDeviceInput) -> ExternResult<VerifiedDevice> {
    let (_, binding) = binding_record(input.binding.clone())?;
    if binding.intent.device_key != input.expected_device
        || binding.intent.content_dna != input.expected_content_dna
    {
        return Err(refuse(
            "device binding does not match actual device/content DNA",
        ));
    }
    let (binding, authority) = binding_stands(input.binding.clone(), &mut Walk::new(Mode::Now), 0)?;
    Ok(VerifiedDevice {
        controllers: authority.controllers,
        human_id: authority.human_id,
        human_action_hash: authority.chain_root.clone(),
        identity_root: authority.chain_root,
        device_key: binding.intent.device_key,
        authority_action_hash: binding.intent.authority,
        binding_action_hash: input.binding,
        network_dna: binding.intent.network_dna,
        content_dna: binding.intent.content_dna,
    })
}

/// Historical evidence does not reauthorize future exercise. This read is used
/// only after the content receiver has verified the exact native acceptance.
#[hdk_extern]
pub fn verify_historical_device_binding(
    input: qahal_types::VerifyHistoricalDeviceInput,
) -> ExternResult<VerifiedDevice> {
    let (_, binding) = binding_record(input.device.binding.clone())?;
    if binding.intent.device_key != input.device.expected_device
        || binding.intent.content_dna != input.device.expected_content_dna
    {
        return Err(refuse(
            "historical device context or native ordering differs",
        ));
    }
    // Rule 4: every record on the walk, and its approvers', is read at the
    // witnessed moment: made before it, revoked (if ever) only after it.
    let (binding, authority) = binding_stands(
        input.device.binding.clone(),
        &mut Walk::new(Mode::At(input.witnessed_at)),
        0,
    )?;
    Ok(VerifiedDevice {
        controllers: authority.controllers,
        human_id: authority.human_id,
        human_action_hash: authority.chain_root.clone(),
        identity_root: authority.chain_root,
        device_key: binding.intent.device_key,
        authority_action_hash: binding.intent.authority,
        binding_action_hash: input.device.binding,
        network_dna: binding.intent.network_dna,
        content_dna: binding.intent.content_dna,
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceAffirmation {
    pub domain: String,
    pub device: VerifiedDevice,
    pub witness: AgentPubKey,
    pub requester: AgentPubKey,
    pub observed_at: Timestamp,
    pub signature: Signature,
}

/// A peer independently affirms the verified relationship. This is bounded
/// evidence, not collective admission or a grant of publishing authority.
#[hdk_extern]
pub fn affirm_identity_device(input: VerifyDeviceInput) -> ExternResult<CommitmentOutput> {
    crate::invocation::authorize("affirm_identity_device", &input)?;
    let device = verify_device_binding(input)?;
    let witness = agent_info()?.agent_initial_pubkey;
    let requester = call_info()?.provenance;
    let observed_at = sys_time()?;
    let unsigned = (
        "elohim:device-affirmation:v1",
        &device,
        &witness,
        &requester,
        observed_at,
    );
    let signature = hdk::ed25519::sign_raw(witness.clone(), bytes(&unsigned)?)?;
    let binding = device.binding_action_hash.clone();
    let out = notarize(
        &DeviceAffirmation {
            domain: "elohim:device-affirmation:v1".into(),
            device,
            witness,
            requester,
            observed_at,
            signature,
        },
        "affirms-device",
        observed_at.as_micros().to_string(),
    )?;
    // Discovery, not proof: who has affirmed a joining record is found from
    // the record's action (never its entry, whose links are its lifecycle).
    create_link(
        binding,
        out.action_hash.clone(),
        LinkTypes::CommitmentByState,
        LinkTag::new(format!("affirmed|{}", observed_at.as_micros())),
    )?;
    Ok(out)
}

#[hdk_extern]
pub fn verify_device_affirmation(action: ActionHash) -> ExternResult<DeviceAffirmation> {
    let (record, entry, proof): (_, _, DeviceAffirmation) = record(action, "affirms-device")?;
    if proof.domain != "elohim:device-affirmation:v1"
        || record.action().author() != &proof.witness
        || entry.signed_at != proof.observed_at.as_micros().to_string()
        || record.action().timestamp() < proof.observed_at
        || !verify_signature_raw(
            proof.witness.clone(),
            proof.signature.clone(),
            bytes(&(
                proof.domain.as_str(),
                &proof.device,
                &proof.witness,
                &proof.requester,
                proof.observed_at,
            ))?,
        )?
    {
        return Err(refuse("invalid device affirmation"));
    }
    let verified = verify_historical_device_binding(qahal_types::VerifyHistoricalDeviceInput {
        device: VerifyDeviceInput {
            binding: proof.device.binding_action_hash.clone(),
            expected_device: proof.device.device_key.clone(),
            expected_content_dna: proof.device.content_dna.clone(),
        },
        witnessed_at: record.action().timestamp().as_micros(),
    })?;
    if bytes(&verified)? != bytes(&proof.device)? {
        return Err(refuse(
            "affirmation relationship differs from native evidence",
        ));
    }
    Ok(proof)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(n: u8) -> AgentPubKey {
        AgentPubKey::from_raw_32(vec![n; 32])
    }
    fn action(n: u8) -> ActionHash {
        ActionHash::from_raw_32(vec![n; 32])
    }
    fn consent(cid: &str) -> DeviceConsent {
        DeviceConsent {
            authority: action(2),
            identity_root: action(1),
            consent_cid: cid.into(),
        }
    }
    fn authority_of(controllers: Vec<AgentPubKey>) -> Authority {
        Authority {
            action: "binds-identity".into(),
            binding_kind: "authority-v1".into(),
            chain_root: action(1),
            human_id: "human".into(),
            human_dna: DnaHash::from_raw_32(vec![3; 32]),
            network_dna: DnaHash::from_raw_32(vec![4; 32]),
            head_key: key(1),
            controllers,
            controller_policy: ControllerPolicy {
                kind: "self".into(),
                m: None,
                n: None,
            },
            authorization: "development-network".into(),
            previous_authority: None,
            signatures: vec![],
        }
    }
    const CID: &str = "bafyreigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";

    #[test]
    fn a_consent_signature_is_bound_to_its_own_domain_and_address() {
        let message = consent_message(&consent(CID)).unwrap();
        assert!(message.starts_with(CONSENT_DOMAIN.as_bytes()));
        assert!(message.ends_with(CID.as_bytes()));
        assert_ne!(
            message,
            consent_message(&consent(&CID.replace("bafy", "bafk"))).unwrap()
        );
        // Never the bytes an enrollment signs.
        assert!(!CONSENT_DOMAIN.contains(DOMAIN));
    }

    #[test]
    fn a_consent_address_is_a_plain_content_address() {
        for bad in [
            "",
            "BAFYREI",
            "bafy rei",
            "bafyrei/../x",
            &"a".repeat(MAX_CONSENT_CID_LEN + 1),
        ] {
            assert!(consent_message(&consent(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn only_a_controller_of_the_named_identity_agrees_for_it() {
        let authority = authority_of(vec![key(1)]);
        assert!(consent_signer_stands(&authority, &consent(CID), &key(1)).is_ok());
        // A device, or anyone else, is not a controller.
        assert!(consent_signer_stands(&authority, &consent(CID), &key(7)).is_err());
        // A controller of some other identity cannot agree for this one.
        let mut other = consent(CID);
        other.identity_root = action(9);
        assert!(consent_signer_stands(&authority, &other, &key(1)).is_err());
    }

    #[test]
    fn policy_threshold_is_owned_by_controller_policy_not_device() {
        let policy = |kind: &str, m, n| ControllerPolicy {
            kind: kind.into(),
            m,
            n,
        };
        assert_eq!(
            threshold(&policy("self", None, None), &[key(1)]).unwrap(),
            1
        );
        assert!(threshold(&policy("self", None, None), &[key(1), key(2)]).is_err());
        assert_eq!(
            threshold(&policy("steward-set", None, None), &[key(1), key(2)]).unwrap(),
            2
        );
        assert_eq!(
            threshold(
                &policy("recovery-quorum", Some(2), Some(3)),
                &[key(1), key(2), key(3)]
            )
            .unwrap(),
            2
        );
        assert!(threshold(&policy("recovery-quorum", Some(1), Some(3)), &[key(1)]).is_err());
        assert!(threshold(&policy("steward-set", None, None), &[key(1), key(1)]).is_err());
        assert!(threshold(&policy("self", None, None), &[]).is_err());
    }
    #[test]
    fn possession_key_cannot_self_authorize_and_duplicate_proofs_do_not_form_quorum() {
        let human = HumanRootEvidence {
            human_action_hash: ActionHash::from_raw_32(vec![3; 32]),
            human_id: "real-matthew".into(),
            author: key(1),
            dna_hash: DnaHash::from_raw_32(vec![4; 32]),
            timestamp: Timestamp::from_micros(123),
        };
        let mut authority = bootstrap_payload(&human, DnaHash::from_raw_32(vec![5; 32]));
        let proof = |n| Proof {
            agent: key(n),
            signature: Signature([0; 64]),
        };
        assert!(authorized_signers(&authority, &[]).is_err());
        assert!(authorized_signers(&authority, &[proof(2)]).is_err());
        assert!(authorized_signers(&authority, &[proof(1), proof(1)]).is_err());
        authority.controllers.push(key(2));
        authority.controller_policy = ControllerPolicy {
            kind: "recovery-quorum".into(),
            m: Some(2),
            n: Some(2),
        };
        assert!(authorized_signers(&authority, &[proof(1)]).is_err());
        assert!(authorized_signers(&authority, &[proof(1), proof(3)]).is_err());
        assert!(authorized_signers(&authority, &[proof(1), proof(2)]).is_ok());
    }
    fn intent() -> DeviceIntent {
        DeviceIntent {
            domain: DOMAIN.into(),
            authority: action(2),
            identity_root: action(1),
            device_key: key(7),
            network_dna: DnaHash::from_raw_32(vec![4; 32]),
            content_dna: DnaHash::from_raw_32(vec![5; 32]),
            issued_at: Timestamp::from_micros(123),
            supersedes: None,
        }
    }

    /// The JSON keys of `json`, in the order they appear.
    fn keys_in_order(json: &str) -> Vec<String> {
        json.split('"')
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[1].starts_with(':'))
            .map(|w| w[0].to_string())
            .collect()
    }

    #[test]
    fn one_approval_cannot_carry_agreement_for_two_identities() {
        let ok = DeviceApproval {
            consent: consent(CID),
            enrollment: Some(intent()),
        };
        assert!(approval_coheres(&ok).is_ok());
        assert!(approval_coheres(&DeviceApproval {
            consent: consent(CID),
            enrollment: None,
        })
        .is_ok());
        let mut other = intent();
        other.identity_root = action(9);
        assert!(approval_coheres(&DeviceApproval {
            consent: consent(CID),
            enrollment: Some(other),
        })
        .is_err());
        let mut other = intent();
        other.authority = action(9);
        assert!(approval_coheres(&DeviceApproval {
            consent: consent(CID),
            enrollment: Some(other),
        })
        .is_err());
    }

    /// A mandate carries the exact JSON of the payload it authorizes, so a
    /// client that builds the payload must produce these keys in this order.
    /// `elohim-storage`'s `device_consent_cell` pins the same list.
    #[test]
    fn the_approval_payload_keeps_its_key_order() {
        let approval = DeviceApproval {
            consent: consent(CID),
            enrollment: Some(intent()),
        };
        assert_eq!(
            keys_in_order(&serde_json::to_string(&approval).unwrap()),
            [
                "consent",
                "authority",
                "identity_root",
                "consent_cid",
                "enrollment",
                "domain",
                "authority",
                "identity_root",
                "device_key",
                "network_dna",
                "content_dna",
                "issued_at",
                "supersedes",
            ]
        );
        assert!(serde_json::to_string(&approval)
            .unwrap()
            .contains("\"issued_at\":123,"));
    }

    fn approval(n: u8) -> Proof {
        Proof {
            agent: key(n),
            signature: Signature::from([n; 64]),
        }
    }
    fn via(n: u8) -> ApprovedVia {
        ApprovedVia {
            agent: key(n),
            binding: action(100 + n),
        }
    }

    #[test]
    fn any_device_of_the_person_counts_once_and_never_for_itself() {
        // Root controller key(1); the joining device is key(7).
        let authority = authority_of(vec![key(1)]);
        let i = intent();
        assert_eq!(
            classify_approvers(&authority, &i, &[approval(1)], &[]),
            Ok(vec![Approver::Root])
        );
        // A device approver names the record it speaks through.
        assert_eq!(
            classify_approvers(&authority, &i, &[approval(2)], &[via(2)]),
            Ok(vec![Approver::Device(action(102))])
        );
        assert!(classify_approvers(&authority, &i, &[approval(2)], &[]).is_err());
        // Never itself, never twice, never more named than approved.
        assert!(classify_approvers(&authority, &i, &[approval(7)], &[via(7)]).is_err());
        assert!(
            classify_approvers(&authority, &i, &[approval(2), approval(2)], &[via(2)]).is_err()
        );
        assert!(classify_approvers(&authority, &i, &[approval(1)], &[via(2), via(3)]).is_err());
        assert!(classify_approvers(&authority, &i, &[], &[]).is_err());
        // The rewrap (adversarial review, critical 1): a record approved by a
        // root alone, republished with an unsigned junk approver record, is a
        // new entry with none of the original's lifecycle. It no longer
        // classifies: approved_via names exactly the device approvers.
        assert!(classify_approvers(&authority, &i, &[approval(1)], &[via(2)]).is_err());
        assert!(classify_approvers(&authority, &i, &[approval(1)], &[via(1)]).is_err());
        // In the order they approved, and only theirs.
        assert!(classify_approvers(
            &authority,
            &i,
            &[approval(2), approval(3)],
            &[via(3), via(2)]
        )
        .is_err());
        assert!(classify_approvers(
            &authority,
            &i,
            &[approval(1), approval(2)],
            &[via(2), via(3)]
        )
        .is_err());
        assert!(classify_approvers(&authority, &i, &[approval(1), approval(2)], &[via(2)]).is_ok());
    }

    #[test]
    fn a_required_number_counts_distinct_devices_that_speak() {
        // The person's policy asks for two: one approval is not enough, two
        // distinct speakers are (a root and a device, or two devices).
        let mut authority = authority_of(vec![key(1), key(2)]);
        authority.controller_policy = ControllerPolicy {
            kind: "recovery-quorum".into(),
            m: Some(2),
            n: Some(2),
        };
        let i = intent();
        assert!(classify_approvers(&authority, &i, &[approval(1)], &[]).is_err());
        assert!(classify_approvers(&authority, &i, &[approval(3)], &[via(3)]).is_err());
        assert!(classify_approvers(&authority, &i, &[approval(1), approval(3)], &[via(3)]).is_ok());
        assert!(classify_approvers(
            &authority,
            &i,
            &[approval(3), approval(4)],
            &[via(3), via(4)]
        )
        .is_ok());
    }

    #[test]
    fn a_carrier_statement_signs_only_its_own_shape() {
        let st = |kind: &str, state: &str| CarrierStatement {
            kind: kind.into(),
            state: state.into(),
            transport_id: "12D3KooWpeer".into(),
            challenge: String::new(),
        };
        assert_eq!(
            carrier_statement_bytes(&st("code", "s1")).unwrap(),
            b"elohim:device-carrier:v1:code\ns1\n12D3KooWpeer\n".to_vec()
        );
        assert!(carrier_statement_bytes(&st("enroll", "s1")).is_err());
        assert!(carrier_statement_bytes(&st("code", "a\nb")).is_err());
        let mut no_peer = st("ask", "s1");
        no_peer.transport_id.clear();
        assert!(carrier_statement_bytes(&no_peer).is_err());
    }

    #[test]
    fn the_walk_back_is_bounded_refuses_a_cycle_and_reads_a_shared_approver_once() {
        // Depth: a record deeper than the bound refuses.
        let mut walk = Walk::new(Mode::Now);
        assert!(walk.enter(&action(1), MAX_APPROVAL_DEPTH).is_ok());
        assert!(walk.enter(&action(2), MAX_APPROVAL_DEPTH + 1).is_err());
        // A cycle: a record met again on its own way back.
        let mut walk = Walk::new(Mode::Now);
        walk.enter(&action(1), 0).unwrap();
        walk.enter(&action(2), 1).unwrap();
        let cycle = walk.enter(&action(1), 2).unwrap_err();
        assert!(format!("{cycle:?}").contains("cycles"), "{cycle:?}");
        // Two approvers whose ways meet (a diamond, possible above a policy
        // of one) is no cycle: once a record stands it leaves the path and
        // is read from the walk, not again.
        let mut walk = Walk::new(Mode::Now);
        let found = (
            DeviceBinding {
                action: "binds-identity".into(),
                binding_kind: "device-v1".into(),
                intent: intent(),
                controllers: vec![approval(1)],
                possession: approval(7),
                approved_via: vec![],
            },
            authority_of(vec![key(1)]),
        );
        walk.enter(&action(3), 0).unwrap();
        walk.enter(&action(4), 1).unwrap();
        walk.stood(action(4), &found);
        assert!(walk.stood.contains_key(&action(4)));
        assert!(!walk.path.contains(&action(4)));
        // Work: at most MAX_APPROVAL_VISITS records are read on one walk.
        let mut walk = Walk::new(Mode::Now);
        for n in 0..MAX_APPROVAL_VISITS {
            walk.enter(&action(n as u8), 0).unwrap();
            walk.path.clear();
        }
        let bound = walk.enter(&action(200), 0).unwrap_err();
        assert!(format!("{bound:?}").contains("work bound"), "{bound:?}");
    }

    #[test]
    fn a_binding_approved_only_by_roots_keeps_its_exact_bytes() {
        let binding = DeviceBinding {
            action: "binds-identity".into(),
            binding_kind: "device-v1".into(),
            intent: intent(),
            controllers: vec![approval(1)],
            possession: approval(7),
            approved_via: vec![],
        };
        let json = String::from_utf8(bytes(&binding).unwrap()).unwrap();
        assert!(!json.contains("approved_via"));
        let mut by_device = binding.clone();
        by_device.approved_via = vec![via(2)];
        assert!(String::from_utf8(bytes(&by_device).unwrap())
            .unwrap()
            .contains("approved_via"));
    }

    #[test]
    fn bootstrap_is_deterministic_and_does_not_name_a_device() {
        let human = HumanRootEvidence {
            human_action_hash: ActionHash::from_raw_32(vec![3; 32]),
            human_id: "matthew-real".into(),
            author: key(1),
            dna_hash: DnaHash::from_raw_32(vec![4; 32]),
            timestamp: Timestamp::from_micros(123),
        };
        let a = bootstrap_payload(&human, DnaHash::from_raw_32(vec![5; 32]));
        assert_eq!(
            bytes(&a).unwrap(),
            bytes(&bootstrap_payload(&human, a.network_dna.clone())).unwrap()
        );
        assert_eq!(a.controllers, vec![human.author]);
        assert_eq!(a.chain_root, human.human_action_hash);
        assert_eq!(a.authorization, "development-network");
    }
}

// Publication witnesses are attributes of the existing binding. They reuse
// Commitment without creating a content head or a reconciliation inventory row.
// The initial implementation supports the existing self-controller bootstrap;
// quorum changes need checkpoints on every participating controller chain.
const MAX_PUBLICATION_HISTORY: u32 = 4096;
const PUBLICATION_DOMAIN: &str = "elohim:device-publication:v1";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DevicePublication {
    domain: String,
    publication: qahal_types::DevicePublicationInput,
    authority: ActionHash,
    root_approval: SignedActionHashed,
}

fn publication_authority(binding: &DeviceBinding) -> ExternResult<(Authority, Record)> {
    historical_binding_proofs(binding)?;
    let checkpoint = binding.intent.authority.clone();
    let (authority, _) = authority_history(checkpoint.clone(), 0)?;
    let (native, _, _): (_, _, Authority) = record(checkpoint, "binds-identity")?;
    if authority.controller_policy.kind != "self"
        || authority.controllers.len() != 1
        || authority.previous_authority.is_some()
        || native.action().author() != &authority.controllers[0]
    {
        return Err(refuse(
            "publication requires a self-controller bootstrap checkpoint",
        ));
    }
    Ok((authority, native))
}

fn publication_history_size(checkpoint: u32, top: u32) -> Option<u32> {
    top.checked_sub(checkpoint)?
        .checked_add(1)
        .filter(|count| *count <= MAX_PUBLICATION_HISTORY)
}

fn publication_predecessor(prior: &Action, next: &Action, prior_hash: &ActionHash) -> bool {
    prior.author() == next.author()
        && prior.action_seq().checked_add(1) == Some(next.action_seq())
        && next.prev_action() == Some(prior_hash)
        && prior.timestamp() < next.timestamp()
}

fn check_publication_commitment(
    entry: &Commitment,
    binding: &DeviceBinding,
    binding_entry: &Commitment,
    authority: &Authority,
) -> ExternResult<()> {
    if entry.action == "revokes-commitment" {
        // Any unreadable lifecycle payload leaves the claim pending. A generic
        // Commitment revocation cannot hide a device withdrawal by omitting a link.
        let payload: serde_json::Value = serde_json::from_str(&entry.payload_json)
            .map_err(|_| refuse("controller revocation payload unavailable — PENDING"))?;
        if payload.get("binding_kind").and_then(|v| v.as_str()) == Some("device-revocation-v1") {
            let withdrawal: DeviceRevocation = serde_json::from_value(payload)
                .map_err(|_| refuse("controller withdrawal payload invalid"))?;
            let (target, _) = binding_record(withdrawal.target.clone())?;
            if &target == binding_entry {
                verify_revocation(&withdrawal, binding)?;
                return Err(refuse("device withdrawal preceded publication witness"));
            }
        }
    } else if entry.action == "binds-identity" {
        let payload: serde_json::Value = serde_json::from_str(&entry.payload_json)
            .map_err(|_| refuse("controller authority payload unavailable — PENDING"))?;
        if payload.get("binding_kind").and_then(|v| v.as_str()) == Some("authority-v1") {
            let policy: Authority = serde_json::from_value(payload)
                .map_err(|_| refuse("controller authority payload invalid"))?;
            if policy.chain_root == authority.chain_root && policy.previous_authority.is_some() {
                return Err(refuse(
                    "controller policy changed before publication witness",
                ));
            }
        }
    }
    Ok(())
}

fn inspect_publication_action(
    signed: &SignedActionHashed,
    cached: Option<Entry>,
    binding: &DeviceBinding,
    binding_entry: &Commitment,
    authority: &Authority,
) -> ExternResult<()> {
    let action = signed.action();
    if action.author() != &authority.controllers[0]
        || hash_action(action.clone())? != *signed.as_hash()
        || !verify_signature(action.author().clone(), signed.signature().clone(), action)?
    {
        return Err(refuse(
            "controller publication history signature or identity differs",
        ));
    }
    let expected: ScopedEntryDefIndex = mishpat_integrity::UnitEntryTypes::Commitment.try_into()?;
    let app = match &action.data {
        ActionData::Create(create) => Some((&create.entry_type, &create.entry_hash)),
        ActionData::Update(update) => Some((&update.entry_type, &update.entry_hash)),
        _ => None,
    };
    if let Some((EntryType::App(def), entry_hash)) = app {
        if def.zome_index == expected.zome_index && def.entry_index == expected.zome_type {
            if def.visibility != EntryVisibility::Public {
                return Err(refuse("controller lifecycle commitment is private"));
            }
            let entry = match cached {
                Some(entry) => entry,
                None => get(signed.as_hash().clone(), GetOptions::default())?
                    .and_then(|record| record.entry().as_option().cloned())
                    .ok_or_else(|| refuse("controller lifecycle entry unavailable — PENDING"))?,
            };
            if hash_entry(entry.clone())? != *entry_hash {
                return Err(refuse("controller lifecycle entry identity differs"));
            }
            let commitment = RecordEntry::Present(entry)
                .to_app_option::<Commitment>()
                .map_err(|_| refuse("controller lifecycle entry undecodable"))?
                .ok_or_else(|| refuse("controller lifecycle entry missing — PENDING"))?;
            check_publication_commitment(&commitment, binding, binding_entry, authority)?;
        }
    }
    Ok(())
}

#[hdk_extern]
pub fn witness_device_publication(
    input: qahal_types::DevicePublicationInput,
) -> ExternResult<CommitmentOutput> {
    crate::invocation::authorize("witness_device_publication", &input)?;
    let (entry, binding) = binding_record(input.device.binding.clone())?;
    let (authority, checkpoint) = publication_authority(&binding)?;
    if agent_info()?.agent_initial_pubkey != authority.controllers[0] {
        return Err(refuse(
            "only the binding controller may witness publication",
        ));
    }
    verify_device_binding(input.device.clone())?;
    let approval: Record = match call(
        CallTargetCell::OtherRole("lamad".into()),
        ZomeName::from("content_store"),
        "get_device_publication_approval".into(),
        None,
        input.clone(),
    )? {
        ZomeCallResponse::Ok(result) => result
            .decode()
            .map_err(|_| refuse("root approval response malformed"))?,
        _ => return Err(refuse("root approval unavailable — PENDING")),
    };
    let top = agent_info()?.chain_head;
    publication_history_size(
        checkpoint.action().action_seq(),
        top.1
            .checked_add(1)
            .ok_or_else(|| refuse("controller chain sequence exhausted"))?,
    )
    .ok_or_else(|| refuse("controller publication history exceeds bound — PENDING"))?;
    let records = query(
        ChainQueryFilter::new()
            .sequence_range(ChainQueryFilterRange::ActionSeqRange(
                checkpoint.action().action_seq(),
                top.1,
            ))
            .include_entries(true),
    )?;
    if records.len() >= MAX_PUBLICATION_HISTORY as usize {
        return Err(refuse(
            "controller publication history exceeds bound — PENDING",
        ));
    }
    for native in records {
        inspect_publication_action(
            native.signed_action(),
            native.entry().as_option().cloned(),
            &binding,
            &entry,
            &authority,
        )?;
    }
    notarize(
        &DevicePublication {
            domain: PUBLICATION_DOMAIN.into(),
            publication: input,
            authority: binding.intent.authority,
            root_approval: approval.signed_action().clone(),
        },
        "authorizes-device-publication",
        "device-publication-v1".into(),
    )
}

#[hdk_extern]
pub fn verify_device_publication(
    input: qahal_types::VerifyDevicePublicationInput,
) -> ExternResult<qahal_types::VerifiedDevicePublication> {
    let (witness, envelope, proof): (_, _, DevicePublication) =
        record(input.witness.clone(), "authorizes-device-publication")?;
    if proof.domain != PUBLICATION_DOMAIN
        || envelope.signed_at != "device-publication-v1"
        || envelope.payload_json.as_bytes() != bytes(&proof)?
        || bytes(&proof.publication)? != bytes(&input.publication)?
    {
        return Err(refuse(
            "controller witness names another device, root or head",
        ));
    }
    let approval = proof.root_approval.action();
    if proof.root_approval.as_hash() != &input.publication.root_acceptance
        || hash_action(approval.clone())? != input.publication.root_acceptance
        || !verify_signature(
            approval.author().clone(),
            proof.root_approval.signature().clone(),
            approval,
        )?
        || approval.timestamp() > witness.action().timestamp()
        || !matches!(&approval.data, ActionData::CreateLink(link)
            if link.base_address == AnyLinkableHash::from(input.publication.content_root.clone())
                && link.target_address == AnyLinkableHash::from(input.publication.content_head.clone()))
    {
        return Err(refuse(
            "controller reconciliation does not bind native root approval",
        ));
    }
    let (entry, binding) = binding_record(input.publication.device.binding.clone())?;
    let (authority, checkpoint) = publication_authority(&binding)?;
    if proof.authority != binding.intent.authority
        || witness.action().author() != &authority.controllers[0]
    {
        return Err(refuse("publication witness controller differs"));
    }
    let count = publication_history_size(
        checkpoint.action().action_seq(),
        witness.action().action_seq(),
    )
    .ok_or_else(|| refuse("controller publication history exceeds bound — PENDING"))?;
    let activity = must_get_agent_activity(
        authority.controllers[0].clone(),
        ChainFilter::take(input.witness.clone(), count).include_cached_entries(),
    )?;
    if activity.len() != count as usize {
        return Err(refuse(
            "controller publication history incomplete — PENDING",
        ));
    }
    let mut activity = activity;
    activity.sort_by_key(|item| item.action.action().action_seq());
    if activity.first().map(|item| item.action.as_hash()) != Some(checkpoint.action_address())
        || activity.last().map(|item| item.action.as_hash()) != Some(&input.witness)
    {
        return Err(refuse("controller publication history checkpoint differs"));
    }
    for pair in activity.windows(2) {
        if !publication_predecessor(
            pair[0].action.action(),
            pair[1].action.action(),
            pair[0].action.as_hash(),
        ) {
            return Err(refuse("controller publication history has a gap or fork"));
        }
    }
    for item in activity {
        inspect_publication_action(
            &item.action,
            item.cached_entry,
            &binding,
            &entry,
            &authority,
        )?;
    }
    // The controller witness, not a caller timestamp, fixes this device exercise.
    let device = verify_historical_device_binding(qahal_types::VerifyHistoricalDeviceInput {
        device: input.publication.device,
        witnessed_at: witness.action().timestamp().as_micros(),
    })?;
    Ok(qahal_types::VerifiedDevicePublication {
        device,
        witnessed_at: witness.action().timestamp().as_micros(),
    })
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    fn chain_action(author: u8, seq: u32, timestamp: i64, previous: Option<ActionHash>) -> Action {
        Action {
            header: ActionHeader {
                author: AgentPubKey::from_raw_32(vec![author; 32]),
                timestamp: Timestamp::from_micros(timestamp),
                action_seq: seq,
                prev_action: previous,
            },
            data: ActionData::Create(CreateData {
                entry_type: EntryType::CapGrant,
                entry_hash: EntryHash::from_raw_32(vec![8; 32]),
            }),
        }
    }
    #[test]
    fn controller_history_requires_author_hash_sequence_and_native_order() {
        let hash = ActionHash::from_raw_32(vec![1; 32]);
        let prior = chain_action(1, 10, 100, None);
        let good = chain_action(1, 11, 101, Some(hash.clone()));
        assert!(publication_predecessor(&prior, &good, &hash));
        for next in [
            chain_action(2, 11, 101, Some(hash.clone())),
            chain_action(1, 12, 101, Some(hash.clone())),
            chain_action(1, 11, 100, Some(hash.clone())),
            chain_action(1, 11, 101, None),
            chain_action(1, 11, 101, Some(ActionHash::from_raw_32(vec![2; 32]))),
        ] {
            assert!(!publication_predecessor(&prior, &next, &hash));
        }
    }
    #[test]
    fn publication_history_never_truncates_to_permission() {
        assert_eq!(publication_history_size(9, 9), Some(1));
        assert_eq!(publication_history_size(9, 10), Some(2));
        assert_eq!(publication_history_size(9, 8), None);
        assert_eq!(publication_history_size(0, MAX_PUBLICATION_HISTORY), None);
        assert_eq!(
            publication_history_size(0, MAX_PUBLICATION_HISTORY - 1),
            Some(MAX_PUBLICATION_HISTORY)
        );
        assert_eq!(publication_history_size(0, u32::MAX), None);
    }
}
