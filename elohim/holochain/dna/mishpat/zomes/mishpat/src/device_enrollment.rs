//! Additive device relationships, notarized as existing Mishpat Commitments.
//!
//! Every reader repeats authorization: generic Commitment integrity validation
//! does not confer identity authority. Lifecycle links are discovery, not proof.
//! No device enters the controller set, and no SQL row can authorize enrollment.
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
    pub controllers: Vec<Proof>,
    pub possession: Proof,
}
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
fn lifecycle(hash: EntryHash, controllers: &[AgentPubKey]) -> ExternResult<Vec<ActionHash>> {
    // Deleting a discoverability link cannot erase an authorized revocation or
    // policy successor. Generic link integrity does not protect deletion.
    let details = get_links_details(
        LinkQuery::try_new(hash, LinkTypes::CommitmentByState)?,
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
    let Some(identity_root) = my_human()? else {
        return Ok(None);
    };
    let network_dna = dna_info()?.hash;
    let human = human_root(identity_root.clone())?;
    let Some(bootstrap) = bootstrap_action(&human, network_dna.clone())? else {
        return Ok(Some(ConsentStanding {
            identity_root,
            authority: None,
            controllers: Vec::new(),
            required: 0,
            network_dna,
        }));
    };
    let current = current_successor(bootstrap)?;
    let authority = current_authority(current.clone())?;
    let required =
        threshold(&authority.controller_policy, &authority.controllers).map_err(|e| refuse(&e))?;
    Ok(Some(ConsentStanding {
        identity_root: authority.chain_root,
        authority: Some(current),
        controllers: authority.controllers,
        required,
        network_dna,
    }))
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
fn historical_binding_proofs(binding: &DeviceBinding) -> ExternResult<()> {
    let (authority, _) = authority_history(binding.intent.authority.clone(), 0)?;
    if binding.action != "binds-identity"
        || binding.binding_kind != "device-v1"
        || binding.intent.domain != DOMAIN
        || binding.intent.network_dna != dna_info()?.hash
        || binding.intent.identity_root != authority.chain_root
    {
        return Err(refuse("invalid historical device binding context"));
    }
    let message = bytes(&binding.intent)?;
    verify_quorum(&authority, &binding.controllers, message.clone())?;
    if binding.possession.agent != binding.intent.device_key
        || !verify_signature_raw(
            binding.possession.agent.clone(),
            binding.possession.signature.clone(),
            message,
        )?
    {
        return Err(refuse("historical device possession not proven"));
    }
    Ok(())
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
/// This cell's signature on an intent already checked against `authority`.
fn enrollment_proof(intent: &DeviceIntent, authority: &Authority) -> ExternResult<Proof> {
    let me = agent_info()?.agent_initial_pubkey;
    if me != intent.device_key && !authority.controllers.contains(&me) {
        return Err(refuse("caller cannot authorize this device binding"));
    }
    Ok(Proof {
        signature: hdk::ed25519::sign_raw(me.clone(), bytes(intent)?)?,
        agent: me,
    })
}
/// Explicit ceremony signing only. Ordinary publication never calls this.
#[hdk_extern]
pub fn sign_device_enrollment(intent: DeviceIntent) -> ExternResult<Proof> {
    crate::invocation::authorize("sign_device_enrollment", &intent)?;
    let authority = validate_intent(&intent)?;
    enrollment_proof(&intent, &authority)
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

/// Whether `signer` may put this identity's agreement on `consent`: the consent
/// must name the identity the authority belongs to, and the signer must be one
/// of its controllers. A device is never a controller, so a device cannot agree
/// to its own recognition.
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

/// This cell's agreement on `consent`, under an authority already resolved.
fn consent_proof(consent: &DeviceConsent, authority: &Authority) -> ExternResult<Proof> {
    let message = consent_message(consent).map_err(refuse)?;
    let me = agent_info()?.agent_initial_pubkey;
    consent_signer_stands(authority, consent, &me).map_err(refuse)?;
    Ok(Proof {
        signature: hdk::ed25519::sign_raw(me.clone(), message)?,
        agent: me,
    })
}

/// Explicit ceremony signing only: a controller agrees to one consent record.
#[hdk_extern]
pub fn sign_device_consent(consent: DeviceConsent) -> ExternResult<Proof> {
    crate::invocation::authorize("sign_device_consent", &consent)?;
    // A malformed address is refused before anything is read from the network.
    consent_message(&consent).map_err(refuse)?;
    let authority = current_authority(consent.authority.clone())?;
    consent_proof(&consent, &authority)
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
    let consent = consent_proof(&approval.consent, &authority)?;
    let enrollment = match &approval.enrollment {
        Some(intent) => {
            check_intent_identity(intent, &authority)?;
            Some(enrollment_proof(intent, &authority)?)
        }
        None => None,
    };
    Ok(DeviceApprovalProofs {
        consent,
        enrollment,
    })
}

/// Whether a controller of the named identity signed this consent. Any holder
/// of a consent can ask; the answer is recomputed here and trusts nothing the
/// caller says about who the controllers are.
#[hdk_extern]
pub fn verify_device_consent(signed: SignedDeviceConsent) -> ExternResult<bool> {
    let message = consent_message(&signed.consent).map_err(refuse)?;
    let authority = current_authority(signed.consent.authority.clone())?;
    if consent_signer_stands(&authority, &signed.consent, &signed.proof.agent).is_err() {
        return Ok(false);
    }
    verify_signature_raw(signed.proof.agent, signed.proof.signature, message)
}

fn verify_binding(binding: &DeviceBinding) -> ExternResult<Authority> {
    if binding.action != "binds-identity" || binding.binding_kind != "device-v1" {
        return Err(refuse("not an additive device binding"));
    }
    let authority = validate_intent(&binding.intent)?;
    let message = bytes(&binding.intent)?;
    verify_quorum(&authority, &binding.controllers, message.clone())?;
    if binding.possession.agent != binding.intent.device_key
        || !verify_signature_raw(
            binding.possession.agent.clone(),
            binding.possession.signature.clone(),
            message,
        )?
    {
        return Err(refuse("device key possession not proven"));
    }
    Ok(authority)
}
#[hdk_extern]
pub fn enroll_identity_device(binding: DeviceBinding) -> ExternResult<CommitmentOutput> {
    verify_binding(&binding)?;
    notarize(
        &binding,
        "binds-identity",
        binding.intent.issued_at.as_micros().to_string(),
    )
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
    Ok(out)
}
#[hdk_extern]
pub fn verify_device_binding(input: VerifyDeviceInput) -> ExternResult<VerifiedDevice> {
    let (entry, binding) = binding_record(input.binding.clone())?;
    if binding.intent.device_key != input.expected_device
        || binding.intent.content_dna != input.expected_content_dna
    {
        return Err(refuse(
            "device binding does not match actual device/content DNA",
        ));
    }
    let authority = verify_binding(&binding)?;
    for target in lifecycle(hash_entry(&entry)?, &authority.controllers)? {
        // A discovered lifecycle prerequisite that cannot be fetched is pending,
        // never evidence of absence. Invalid/forged evidence confers no standing.
        let (_, _, revocation): (_, _, DeviceRevocation) = record(target, "revokes-commitment")?;
        let (target_entry, _) = binding_record(revocation.target.clone())?;
        if target_entry == entry {
            verify_revocation(&revocation, &binding)?;
            return Err(refuse("device binding revoked"));
        }
    }
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
    let (binding_native, _, _): (_, _, DeviceBinding) =
        record(input.device.binding.clone(), "binds-identity")?;
    let (entry, binding) = binding_record(input.device.binding.clone())?;
    historical_binding_proofs(&binding)?;
    if binding_native.action().timestamp().as_micros() > input.witnessed_at
        || binding.intent.device_key != input.device.expected_device
        || binding.intent.content_dna != input.device.expected_content_dna
    {
        return Err(refuse(
            "historical device context or native ordering differs",
        ));
    }
    let authority = authority_at(binding.intent.authority.clone(), input.witnessed_at)?;
    for target in lifecycle(hash_entry(&entry)?, &authority.controllers)? {
        let (revocation_record, _, revocation): (_, _, DeviceRevocation) =
            record(target, "revokes-commitment")?;
        let (target_entry, _) = binding_record(revocation.target.clone())?;
        if target_entry == entry {
            verify_revocation(&revocation, &binding)?;
            if revocation_record.action().timestamp().as_micros() <= input.witnessed_at {
                return Err(refuse("device withdrawal preceded witnessed exercise"));
            }
        }
    }
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
    notarize(
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
    )
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
