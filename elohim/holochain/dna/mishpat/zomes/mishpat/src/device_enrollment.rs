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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerifyDeviceInput {
    pub binding: ActionHash,
    pub expected_device: AgentPubKey,
    pub expected_content_dna: DnaHash,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerifiedDevice {
    pub controllers: Vec<AgentPubKey>,
    pub human_id: String,
    pub human_action_hash: ActionHash,
    pub identity_root: ActionHash,
    pub device_key: AgentPubKey,
    pub authority_action_hash: ActionHash,
    pub binding_action_hash: ActionHash,
    pub network_dna: DnaHash,
    pub content_dna: DnaHash,
}
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
    if intent.domain != DOMAIN || intent.network_dna != dna_info()?.hash {
        return Err(refuse("device binding network/domain mismatch"));
    }
    let authority = current_authority(intent.authority.clone())?;
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
    Ok(authority)
}
/// Explicit ceremony signing only. Ordinary publication never calls this.
#[hdk_extern]
pub fn sign_device_enrollment(intent: DeviceIntent) -> ExternResult<Proof> {
    let authority = validate_intent(&intent)?;
    let me = agent_info()?.agent_initial_pubkey;
    if me != intent.device_key && !authority.controllers.contains(&me) {
        return Err(refuse("caller cannot authorize this device binding"));
    }
    Ok(Proof {
        signature: hdk::ed25519::sign_raw(me.clone(), bytes(&intent)?)?,
        agent: me,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    fn key(n: u8) -> AgentPubKey {
        AgentPubKey::from_raw_32(vec![n; 32])
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
