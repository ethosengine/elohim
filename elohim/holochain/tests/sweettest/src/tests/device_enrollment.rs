//! @dna-scope: imagodei,mishpat,lamad
//! Real coordinator acceptance, including malicious raw Commitment/link writes.
use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, SweetAgents},
    fixtures::network_seed,
};
use holochain::sweettest::{SweetCell, SweetConductor};
use holochain_keystore::AgentPubKeyExt;
use holochain_serialized_bytes::prelude::*;
use holochain_state::dht_store::{AppOutcome, SysOutcome};
use holochain_types::op::OpEntry;
use holochain_types::prelude::*;
use serde::{Deserialize, Serialize};

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approved_via: Vec<ApprovedVia>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ApprovedVia {
    pub agent: AgentPubKey,
    pub binding: ActionHash,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityDevice {
    pub device_key: AgentPubKey,
    pub binding: ActionHash,
    pub content_dna: DnaHash,
    pub approved_by: Vec<AgentPubKey>,
    pub joined_at: Timestamp,
    pub affirmed_by: Vec<AgentPubKey>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityDevices {
    pub identity_root: ActionHash,
    pub roots: Vec<AgentPubKey>,
    pub devices: Vec<IdentityDevice>,
    pub not_standing: u32,
    pub truncated: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerifyHistoricalDeviceInput {
    pub device: VerifyDeviceInput,
    pub witnessed_at: i64,
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
struct PublicationGrantPayload {
    grantor: AgentPubKey,
    delegate: AgentPubKey,
    scope: String,
    valid_until: Timestamp,
    root_action_hash: ActionHash,
    dna_hash: DnaHash,
    issuance_action_hash: Option<ActionHash>,
    device_binding: Option<ActionHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exercise: Option<PublicationExercise>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PublicationExercise {
    requester: AgentPubKey,
    executor: AgentPubKey,
    policy: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PublicationGrant {
    payload: PublicationGrantPayload,
    signature: Signature,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    acceptance: Option<PublicationAcceptance>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PublicationAcceptance {
    head_action_hash: ActionHash,
    witness_action_hash: ActionHash,
    accepted_at: Timestamp,
    signature: Signature,
    device_witness_action_hash: Option<ActionHash>,
}
#[derive(Serialize, Deserialize, Debug)]
struct PublicationPreflight {
    agent: AgentPubKey,
}
#[derive(Serialize, Deserialize, Debug)]
struct PublicationApproval {
    witness_action_hash: ActionHash,
    accepted_at: Timestamp,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifiedPublication {
    device: VerifiedDevice,
    witnessed_at: i64,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DeviceConsent {
    pub authority: ActionHash,
    pub identity_root: ActionHash,
    pub consent_cid: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedDeviceConsent {
    pub consent: DeviceConsent,
    pub proof: Proof,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<ActionHash>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceApproval {
    pub consent: DeviceConsent,
    pub enrollment: Option<DeviceIntent>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceApprovalProofs {
    pub consent: Proof,
    pub enrollment: Option<Proof>,
    #[serde(default)]
    pub via: Option<ActionHash>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsentStanding {
    pub identity_root: ActionHash,
    pub authority: Option<ActionHash>,
    pub controllers: Vec<AgentPubKey>,
    pub required: usize,
    pub network_dna: DnaHash,
    #[serde(default)]
    pub speaks_via: Option<ActionHash>,
    #[serde(default)]
    pub also_speaks_for: Vec<ActionHash>,
}

#[derive(Serialize, Deserialize, Debug, Clone, SerializedBytes)]
struct RawCommitment {
    action: String,
    payload_json: String,
    signed_at: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Receipt {
    action_hash: ActionHash,
    entry_hash: EntryHash,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Register {
    binding: ActionHash,
    expected_content_dna: DnaHash,
}

#[derive(Serialize, Deserialize, Debug)]
struct HumanOutput {
    action_hash: ActionHash,
    human: serde_json::Value,
}

// Signed malicious records enter the real DHT integration path. This fixture
// bypasses admission solely to test consumer verification; production WASM and
// integrity definitions remain unchanged. Each attacker extends its last honest
// chain head, then advances a cursor without making further honest writes.
#[derive(Default)]
struct Attacks {
    cursors: std::collections::HashMap<AgentPubKey, (ActionHash, u32, Timestamp)>,
    links: Vec<(ActionHash, CreateLinkData)>,
}
impl Attacks {
    async fn write(
        &mut self,
        c: &SweetConductor,
        cell: &SweetCell,
        data: ActionData,
        entry: Option<Entry>,
    ) -> ActionHash {
        let agent = cell.agent_pubkey().clone();
        if !self.cursors.contains_key(&agent) {
            let chain = c.get_agent_source_chain(&agent, cell.dna_hash()).await;
            let head = chain.chain_head_nonempty().unwrap();
            self.cursors
                .insert(agent.clone(), (head.action, head.seq, head.timestamp));
        }
        let cursor = self.cursors.get_mut(&agent).unwrap();
        let action = Action {
            header: ActionHeader {
                author: agent.clone(),
                prev_action: Some(cursor.0.clone()),
                action_seq: cursor.1 + 1,
                timestamp: std::cmp::max(
                    Timestamp::now(),
                    Timestamp::from_micros(cursor.2.as_micros() + 1),
                ),
            },
            data,
        };
        let hash = ActionHash::with_data_sync(&action);
        let signature = agent.sign(&c.keystore(), &action).await.unwrap();
        let signed = SignedAction::new(action.clone(), signature);
        *cursor = (
            hash.clone(),
            action.header.action_seq,
            action.header.timestamp,
        );
        let mut ops = vec![ChainOp::CreateRecord(
            signed.clone(),
            entry
                .clone()
                .map(OpEntry::Present)
                .unwrap_or(OpEntry::ActionOnly),
        )];
        match &action.data {
            ActionData::Create(_) => ops.push(ChainOp::CreateEntry(
                signed,
                OpEntry::Present(entry.unwrap()),
            )),
            ActionData::CreateLink(link) => {
                self.links.push((hash.clone(), link.clone()));
                ops.push(ChainOp::CreateLink(signed));
            }
            ActionData::DeleteLink(_) => ops.push(ChainOp::DeleteLink(signed)),
            _ => unreachable!(),
        }
        let store = c.get_dht_store(cell.dna_hash()).unwrap();
        for op in ops {
            let op = DhtOpHashed::from_content_sync(DhtOp::from(op));
            let op_hash = op.as_hash().clone();
            store.record_incoming_ops(vec![(op, false)]).await.unwrap();
            store
                .record_chain_op_sys_validation_outcomes(vec![(
                    op_hash.clone(),
                    SysOutcome::Accepted,
                )])
                .await
                .unwrap();
            store
                .record_app_validation_outcomes(vec![(op_hash, AppOutcome::Accepted)])
                .await
                .unwrap();
        }
        // The live integration worker shares this test DHT store. Retry only
        // its transient SQLite writer collision; malformed ops and all other
        // failures remain immediate test failures. No production lock changes.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            match store.integrate_ready_ops(Timestamp::now()).await {
                Ok(_) => break,
                Err(error)
                    if error.to_string().contains("database is locked")
                        && std::time::Instant::now() < deadline =>
                {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Err(error) => panic!("adversarial fixture integration failed: {error}"),
            }
        }
        hash
    }
    async fn commitment(
        &mut self,
        c: &SweetConductor,
        cell: &SweetCell,
        payload: RawCommitment,
    ) -> ActionHash {
        let entry = Entry::App(
            AppEntryBytes::try_from(SerializedBytes::try_from(payload).unwrap()).unwrap(),
        );
        self.write(
            c,
            cell,
            ActionData::Create(CreateData {
                entry_type: EntryType::App(AppEntryDef::new(
                    EntryDefIndex(8),
                    ZomeIndex(0),
                    EntryVisibility::Public,
                )),
                entry_hash: EntryHash::with_data_sync(&entry),
            }),
            Some(entry),
        )
        .await
    }
    async fn link(
        &mut self,
        c: &SweetConductor,
        cell: &SweetCell,
        anchor: EntryHash,
        target: ActionHash,
    ) {
        self.write(
            c,
            cell,
            ActionData::CreateLink(CreateLinkData {
                base_address: anchor.into(),
                target_address: target.into(),
                zome_index: ZomeIndex(0),
                link_type: LinkType(23),
                tag: LinkTag::new("active|1"),
            }),
            None,
        )
        .await;
    }
    async fn delete(
        &mut self,
        c: &SweetConductor,
        attacker: &SweetCell,
        controller: &SweetCell,
        anchor: EntryHash,
    ) -> usize {
        let chain = c
            .get_agent_source_chain(controller.agent_pubkey(), controller.dna_hash())
            .await;
        let mut links = self.links.clone();
        for record in chain.query(ChainQueryFilter::default()).await.unwrap() {
            if let ActionData::CreateLink(link) = &record.action().data {
                links.push((record.action_address().clone(), link.clone()));
            }
        }
        let mut deleted = 0;
        for (hash, link) in links {
            if link.base_address != AnyLinkableHash::from(anchor.clone()) {
                continue;
            }
            self.write(
                c,
                attacker,
                ActionData::DeleteLink(DeleteLinkData {
                    base_address: anchor.clone().into(),
                    link_add_address: hash,
                }),
                None,
            )
            .await;
            deleted += 1;
        }
        deleted
    }
}

async fn enroll(
    c: &SweetConductor,
    controller: &SweetCell,
    device: &SweetCell,
    authority: &Receipt,
    human: &ActionHash,
    content: &DnaHash,
    tick: i64,
) -> (Receipt, DeviceBinding) {
    let intent = DeviceIntent {
        domain: "elohim:device-enrollment:v1".into(),
        authority: authority.action_hash.clone(),
        identity_root: human.clone(),
        device_key: device.agent_pubkey().clone(),
        network_dna: device.dna_hash().clone(),
        content_dna: content.clone(),
        issued_at: Timestamp::from_micros(tick),
        supersedes: None,
    };
    let possession: Proof = c
        .call(
            &device.zome("mishpat"),
            "sign_device_enrollment",
            intent.clone(),
        )
        .await;
    let proof: Proof = c
        .call(
            &controller.zome("mishpat"),
            "sign_device_enrollment",
            intent.clone(),
        )
        .await;
    let binding = DeviceBinding {
        action: "binds-identity".into(),
        binding_kind: "device-v1".into(),
        intent,
        controllers: vec![proof],
        possession,
        approved_via: vec![],
    };
    let receipt = c
        .call(
            &device.zome("mishpat"),
            "enroll_identity_device",
            binding.clone(),
        )
        .await;
    (receipt, binding)
}
/// The intent a new `device` signs to join `human`'s identity.
fn intent_for(
    device: &SweetCell,
    authority: &Receipt,
    human: &ActionHash,
    content: &DnaHash,
    tick: i64,
) -> DeviceIntent {
    DeviceIntent {
        domain: "elohim:device-enrollment:v1".into(),
        authority: authority.action_hash.clone(),
        identity_root: human.clone(),
        device_key: device.agent_pubkey().clone(),
        network_dna: device.dna_hash().clone(),
        content_dna: content.clone(),
        issued_at: Timestamp::from_micros(tick),
        supersedes: None,
    }
}

/// A new `device` joins approved by `approvers`, each a cell that speaks for
/// the person, with the joining record it speaks through (None for a root
/// controller). Returns the enroll result.
async fn enroll_by(
    c: &SweetConductor,
    approvers: &[(&SweetCell, Option<ActionHash>)],
    device: &SweetCell,
    intent: DeviceIntent,
) -> Result<(Receipt, DeviceBinding), String> {
    let possession: Proof = c
        .call(
            &device.zome("mishpat"),
            "sign_device_enrollment",
            intent.clone(),
        )
        .await;
    let mut controllers = Vec::new();
    let mut approved_via = Vec::new();
    for (approver, via) in approvers {
        let proof: Proof = c
            .call_fallible(
                &approver.zome("mishpat"),
                "sign_device_enrollment",
                intent.clone(),
            )
            .await
            .map_err(|e| format!("{e:?}"))?;
        if let Some(via) = via {
            approved_via.push(ApprovedVia {
                agent: proof.agent.clone(),
                binding: via.clone(),
            });
        }
        controllers.push(proof);
    }
    let binding = DeviceBinding {
        action: "binds-identity".into(),
        binding_kind: "device-v1".into(),
        intent,
        controllers,
        possession,
        approved_via,
    };
    let receipt: Receipt = c
        .call_fallible(
            &device.zome("mishpat"),
            "enroll_identity_device",
            binding.clone(),
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok((receipt, binding))
}

fn query(receipt: &Receipt, device: &SweetCell, content: &DnaHash) -> VerifyDeviceInput {
    VerifyDeviceInput {
        binding: receipt.action_hash.clone(),
        expected_device: device.agent_pubkey().clone(),
        expected_content_dna: content.clone(),
    }
}

#[derive(Deserialize, Debug)]
struct ContentAction {
    action_hash: ActionHash,
}
#[derive(Deserialize, Debug)]
struct GovernanceOutput {
    cid: String,
}

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
struct ContentTypeAnchor {
    anchor_type: String,
    anchor_value: String,
}

/// Calling generic Content bypasses the friendly producer, so the effective
/// reader must check the signed action author, never forged author_id metadata.
async fn device_content_admin_is_denied(
    c: &SweetConductor,
    attacks: &mut Attacks,
    indexer: &SweetCell,
    device: &SweetCell,
    root: &SweetCell,
    root_key: &AgentPubKey,
    phase: &str,
    prove_legacy: bool,
) {
    for (kind, reader, subject) in [
        (
            "governance-action:key-revocation",
            "query_effective_revocation_for_key",
            root_key.to_string(),
        ),
        (
            "governance-action:identity-freeze",
            "query_effective_identity_freeze_for_human",
            "real-matthew-device-proof".to_string(),
        ),
    ] {
        let metadata = serde_json::json!({
            "author_id": root_key.to_string(), "initiated_by": "real-matthew-device-proof",
            "human_id": "real-matthew-device-proof", "revoked_key": root_key.to_string(),
            "threshold_reached": true, "effective_at": "2026-01-01T00:00:00Z",
            "is_active": true, "threshold": {"m":1,"n":1,"type":"single-cell"},
            "closes_at":"2099-01-01T00:00:00Z", "ballot_format":"approve-reject"
        });
        let proposal = serde_json::json!({
            "governance_kind":kind, "subject_human_id":"real-matthew-device-proof",
            "title":"Unauthorized device administration", "description":null,
            "reach":"private", "threshold":{"m":1,"n":1,"type":"single-cell"},
            "closes_at":"2099-01-01T00:00:00Z", "metadata":metadata, "supersedes_cid":null
        });
        let denied = c
            .call_fallible::<_, GovernanceOutput>(
                &device.zome("content_store"),
                "propose_recovery_governance_action",
                proposal.clone(),
            )
            .await
            .expect_err("enrollment does not grant direct legacy administration");
        assert!(format!("{denied:?}")
            .contains("enrolled devices cannot exercise legacy identity administration"));
        let injected: ContentAction = c
            .call(
                &device.zome("content_store"),
                "create_content",
                serde_json::json!({
                    "id":format!("device-admin-{phase}-{reader}"), "content_type":kind,
                    "title":"Spoofed root administration", "description":"adversarial fixture",
                    "content":"", "content_format":"markdown", "reach":"private",
                    "metadata_json":metadata.to_string(), "tags":[], "related_node_ids":[]
                }),
            )
            .await;
        assert_ne!(injected.action_hash, ActionHash::from_raw_32(vec![0; 32]));
        // Generic Content deliberately omits type indexing. A malicious
        // coordinator can still publish the existing public TypeToContent link.
        // Use the otherwise-unused second device's lamad chain, so these signed
        // attack links never fork a chain used for later honest Content writes.
        let _: Vec<ContentAction> = c
            .call(
                &indexer.zome("content_store"),
                "get_content_by_type",
                serde_json::json!({"content_type": kind, "limit": 1}),
            )
            .await;
        let anchor = Entry::App(
            AppEntryBytes::try_from(
                SerializedBytes::try_from(ContentTypeAnchor {
                    anchor_type: "content_type".into(),
                    anchor_value: kind.into(),
                })
                .unwrap(),
            )
            .unwrap(),
        );
        attacks
            .write(
                c,
                indexer,
                ActionData::CreateLink(CreateLinkData {
                    base_address: EntryHash::with_data_sync(&anchor).into(),
                    target_address: injected.action_hash.clone().into(),
                    zome_index: ZomeIndex(0),
                    link_type: LinkType(1),
                    tag: LinkTag::new(Vec::<u8>::new()),
                }),
                None,
            )
            .await;
        // Prove the receiving reader can see this exact malicious candidate;
        // a negative effective result must not merely mean delayed indexing.
        let visible_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let candidates: Vec<ContentAction> = c
                .call(
                    &root.zome("content_store"),
                    "get_content_by_type",
                    serde_json::json!({"content_type": kind, "limit": 4294967295u32}),
                )
                .await;
            if candidates
                .iter()
                .any(|candidate| candidate.action_hash == injected.action_hash)
            {
                break;
            }
            assert!(
                std::time::Instant::now() < visible_deadline,
                "forged candidate must be visible before testing authorization filtering"
            );
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        let effective: Option<ContentAction> = c
            .call(&root.zome("content_store"), reader, subject.clone())
            .await;
        assert!(
            effective.is_none(),
            "{phase}: forged metadata on a device-authored {kind} must not become effective"
        );
        if prove_legacy {
            let accepted: GovernanceOutput = c
                .call(
                    &root.zome("content_store"),
                    "propose_recovery_governance_action",
                    proposal,
                )
                .await;
            assert!(!accepted.cid.is_empty());
            let effective: Option<ContentAction> =
                c.call(&root.zome("content_store"), reader, subject).await;
            assert!(
                effective.is_some(),
                "original root's existing legacy path must remain available"
            );
        }
    }
}

/// The "join as it is" case: a node that is the sole controller of an identity
/// it began itself is bound as a device under another person's identity,
/// keeping its key. Records what the zomes do; asserts only what they must.
async fn a_node_with_its_own_identity_joins_as_it_is(
    c: &SweetConductor,
    cells: &[(SweetCell, SweetCell, SweetCell)],
    operator: &AgentPubKey,
    authority: &Receipt,
    human: &ActionHash,
    content: &DnaHash,
) {
    let node = &cells[3];
    let own_human: ActionHash = c
        .call(
            &node.0.zome("imagodei"),
            "create_human",
            serde_json::json!({
                "id": "prior-node-own-identity", "display_name": "Prior node", "bio": null,
                "affinities": [], "profile_reach": "private", "location": null
            }),
        )
        .await;
    let own_authority: Receipt = c
        .call(
            &node.1.zome("mishpat"),
            "bootstrap_device_identity",
            own_human.clone(),
        )
        .await;
    let own: Option<ConsentStanding> = c
        .call(&node.1.zome("mishpat"), "my_consent_standing", ())
        .await;
    let own = own.expect("the node stands on its own identity");
    assert_eq!(own.identity_root, own_human);
    assert_eq!(own.controllers, vec![node.1.agent_pubkey().clone()]);
    eprintln!("as-it-is: the node is the sole controller of its own identity");

    // Bind it, keeping its key, under the operator's identity.
    let (binding, _) = enroll(c, &cells[0].1, &node.1, authority, human, content, 125).await;
    eprintln!("as-it-is: enroll_identity_device accepted the binding");
    let verified: VerifiedDevice = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_device_binding",
            query(&binding, &node.1, content),
        )
        .await;
    assert_eq!(&verified.human_action_hash, human);
    assert_eq!(verified.controllers, vec![operator.clone()]);
    eprintln!("as-it-is: another agent verifies the node as the operator's device");

    // What the node now says about itself: its own identity is left as it was.
    let after: Option<ConsentStanding> = c
        .call(&node.1.zome("mishpat"), "my_consent_standing", ())
        .await;
    let after = after.expect("the node still has a Human");
    eprintln!(
        "as-it-is: my_consent_standing after joining names identity_root {} (own {}, operator {}), authority {:?} (own {})",
        after.identity_root, own_human, human, after.authority, own_authority.action_hash
    );
    let resolved: Option<HumanOutput> = c
        .call(
            &node.0.zome("imagodei"),
            "get_human_by_agent_key",
            node.0.agent_pubkey().clone(),
        )
        .await;
    eprintln!(
        "as-it-is: get_human_by_agent_key(node) resolves {:?}",
        resolved.map(|h| h.action_hash)
    );
    // Registering the binding is a separate imagodei step; see what it does
    // for a node that already has a Human of its own.
    let registered = c
        .call_fallible::<_, VerifiedDevice>(
            &node.0.zome("imagodei"),
            "register_device_identity",
            Register {
                binding: binding.action_hash.clone(),
                expected_content_dna: content.clone(),
            },
        )
        .await;
    eprintln!(
        "as-it-is: register_device_identity -> {}",
        match &registered {
            Ok(v) => format!("ok, human {}", v.human_action_hash),
            Err(e) => format!("refused: {e}"),
        }
    );
    let resolved: Option<HumanOutput> = c
        .call_fallible(
            &node.0.zome("imagodei"),
            "get_human_by_agent_key",
            node.0.agent_pubkey().clone(),
        )
        .await
        .unwrap_or(None);
    eprintln!(
        "as-it-is: after registering, get_human_by_agent_key(node) resolves {:?}",
        resolved.map(|h| h.action_hash)
    );
}

/// Every device a person has joined speaks for them, and any of them may
/// approve the next: the operator (root) approves B; B alone approves C while
/// the operator signs nothing; a fourth party verifies C from the network; the
/// root and B approve two new devices at once and both stand without the
/// authority moving; no device approves itself or counts twice; the person's
/// other devices affirm, and who approved and who affirmed is readable; a
/// revoked device cannot approve, and what it approved stops verifying now
/// while still verifying at an authenticated earlier moment.
async fn every_device_speaks_for_its_person(
    c: &SweetConductor,
    cells: &[(SweetCell, SweetCell, SweetCell)],
    operator: &AgentPubKey,
    authority: &Receipt,
    human: &ActionHash,
    content: &DnaHash,
) {
    let root = &cells[0].1;
    let (bee, cee, tee, dee, eve) = (&cells[4], &cells[5], &cells[6], &cells[7], &cells[8]);
    let bee_key = bee.1.agent_pubkey().clone();
    let register = |cell: &SweetCell, binding: ActionHash| {
        let cell = cell.clone();
        let content = content.clone();
        async move {
            let _: VerifiedDevice = c
                .call(
                    &cell.zome("imagodei"),
                    "register_device_identity",
                    Register {
                        binding,
                        expected_content_dna: content,
                    },
                )
                .await;
        }
    };

    // B joins, approved by the root.
    let (b, _) = enroll_by(
        c,
        &[(root, None)],
        &bee.1,
        intent_for(&bee.1, authority, human, content, 200),
    )
    .await
    .expect("the root approves B");
    register(&bee.0, b.action_hash.clone()).await;
    let standing: Option<ConsentStanding> = c
        .call(&bee.1.zome("mishpat"), "my_consent_standing", ())
        .await;
    let standing = standing.expect("B speaks for the person");
    assert_eq!(&standing.identity_root, human);
    assert_eq!(standing.authority.as_ref(), Some(&authority.action_hash));
    assert_eq!(standing.speaks_via.as_ref(), Some(&b.action_hash));
    assert!(standing.controllers.contains(&bee_key));
    assert_eq!(standing.required, 1);
    eprintln!("every-device: B joined and speaks for the person through its record");

    // C joins, approved by B alone; the root signs nothing.
    let (cc, c_binding) = enroll_by(
        c,
        &[(&bee.1, Some(b.action_hash.clone()))],
        &cee.1,
        intent_for(&cee.1, authority, human, content, 201),
    )
    .await
    .expect("B alone approves C");
    assert_eq!(c_binding.controllers.len(), 1);
    assert_eq!(c_binding.controllers[0].agent, bee_key);
    let verified: VerifiedDevice = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_device_binding",
            query(&cc, &cee.1, content),
        )
        .await;
    assert_eq!(&verified.identity_root, human);
    eprintln!("every-device: C joined with B's approval alone; a fourth party verifies it");

    // B's one-step approval names the record it speaks through, and a fourth
    // party can check B's agreement on a consent.
    let consent = DeviceConsent {
        authority: authority.action_hash.clone(),
        identity_root: human.clone(),
        consent_cid: "bafyreigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".into(),
    };
    let proofs: DeviceApprovalProofs = c
        .call(
            &bee.1.zome("mishpat"),
            "sign_device_approval",
            DeviceApproval {
                consent: consent.clone(),
                enrollment: Some(intent_for(&eve.1, authority, human, content, 199)),
            },
        )
        .await;
    assert_eq!(proofs.via.as_ref(), Some(&b.action_hash));
    let holds: bool = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_device_consent",
            SignedDeviceConsent {
                consent: consent.clone(),
                proof: proofs.consent.clone(),
                via: proofs.via.clone(),
            },
        )
        .await;
    assert!(holds);
    let holds: bool = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_device_consent",
            SignedDeviceConsent {
                consent,
                proof: proofs.consent,
                via: None,
            },
        )
        .await;
    assert!(
        !holds,
        "a device's agreement needs the record it speaks through"
    );

    // The root and B approve two new devices at once; neither holds up the
    // other, and the authority does not move.
    let (t, _) = enroll_by(
        c,
        &[(root, None)],
        &tee.1,
        intent_for(&tee.1, authority, human, content, 202),
    )
    .await
    .expect("the root approves T");
    let (d, _) = enroll_by(
        c,
        &[(&bee.1, Some(b.action_hash.clone()))],
        &dee.1,
        intent_for(&dee.1, authority, human, content, 203),
    )
    .await
    .expect("B approves D at the same time");
    for (receipt, cell) in [(&t, &tee.1), (&d, &dee.1)] {
        let _: VerifiedDevice = c
            .call(
                &cells[2].1.zome("mishpat"),
                "verify_device_binding",
                query(receipt, cell, content),
            )
            .await;
    }
    let after: Option<ConsentStanding> = c
        .call(&root.zome("mishpat"), "my_consent_standing", ())
        .await;
    assert_eq!(
        after.unwrap().authority.as_ref(),
        Some(&authority.action_hash)
    );
    eprintln!("every-device: two approvals at once both stand; the authority did not move");

    // No device approves itself, counts twice, or counts without showing the
    // record it speaks through; a device that does not speak cannot sign.
    let e_intent = intent_for(&eve.1, authority, human, content, 204);
    assert!(enroll_by(c, &[(&eve.1, None)], &eve.1, e_intent.clone())
        .await
        .is_err());
    let possession: Proof = c
        .call(
            &eve.1.zome("mishpat"),
            "sign_device_enrollment",
            e_intent.clone(),
        )
        .await;
    let bee_proof: Proof = c
        .call(
            &bee.1.zome("mishpat"),
            "sign_device_enrollment",
            e_intent.clone(),
        )
        .await;
    let via = ApprovedVia {
        agent: bee_key.clone(),
        binding: b.action_hash.clone(),
    };
    for (controllers, approved_via, what) in [
        (vec![possession.clone()], vec![], "itself"),
        (
            vec![bee_proof.clone(), bee_proof.clone()],
            vec![via.clone()],
            "twice",
        ),
        (vec![bee_proof.clone()], vec![], "without its record"),
    ] {
        let binding = DeviceBinding {
            action: "binds-identity".into(),
            binding_kind: "device-v1".into(),
            intent: e_intent.clone(),
            controllers,
            possession: possession.clone(),
            approved_via,
        };
        assert!(
            c.call_fallible::<_, Receipt>(
                &eve.1.zome("mishpat"),
                "enroll_identity_device",
                binding
            )
            .await
            .is_err(),
            "an approval {what} must not count"
        );
    }
    eprintln!("every-device: self-approval, a double count and an unshown record all refuse");

    // Who approved each device, and who has affirmed it since.
    let read = || async {
        let devices: IdentityDevices = c
            .call(
                &cells[2].1.zome("mishpat"),
                "identity_devices",
                human.clone(),
            )
            .await;
        devices
    };
    let devices = read().await;
    for key in [
        &bee_key,
        cee.1.agent_pubkey(),
        tee.1.agent_pubkey(),
        dee.1.agent_pubkey(),
    ] {
        assert!(
            devices.devices.iter().any(|d| &d.device_key == key),
            "{key} is listed"
        );
    }
    let of_c = |devices: &IdentityDevices| {
        devices
            .devices
            .iter()
            .find(|d| d.device_key == *cee.1.agent_pubkey())
            .unwrap()
            .clone()
    };
    assert_eq!(of_c(&devices).approved_by, vec![bee_key.clone()]);
    assert!(of_c(&devices).affirmed_by.is_empty());
    let _: Receipt = c
        .call(
            &root.zome("mishpat"),
            "affirm_identity_device",
            query(&cc, &cee.1, content),
        )
        .await;
    let _: Receipt = c
        .call(
            &bee.1.zome("mishpat"),
            "affirm_identity_device",
            query(&cc, &cee.1, content),
        )
        .await;
    let devices = read().await;
    assert_eq!(
        of_c(&devices).affirmed_by,
        vec![operator.clone()],
        "the root's affirmation counts; the approver's own adds nothing"
    );
    eprintln!(
        "every-device: identity_devices lists {} devices; C approved by B, affirmed by 1 other",
        devices.devices.len()
    );

    // Revocation: the root withdraws B.
    let before = Timestamp::now();
    let mut revocation = DeviceRevocation {
        action: "revokes-commitment".into(),
        binding_kind: "device-revocation-v1".into(),
        target: b.action_hash.clone(),
        authority: authority.action_hash.clone(),
        network_dna: root.dna_hash().clone(),
        signatures: vec![],
    };
    let proof: Proof = c
        .call(
            &root.zome("mishpat"),
            "sign_device_revocation",
            revocation.clone(),
        )
        .await;
    revocation.signatures.push(proof);
    let _: Receipt = c
        .call(&root.zome("mishpat"), "revoke_identity_device", revocation)
        .await;
    // B no longer speaks: it cannot approve.
    assert!(c
        .call_fallible::<_, Proof>(
            &bee.1.zome("mishpat"),
            "sign_device_enrollment",
            intent_for(&eve.1, authority, human, content, 205),
        )
        .await
        .is_err());
    // What B approved stops verifying now (rule 4) ...
    assert!(c
        .call_fallible::<_, VerifiedDevice>(
            &cells[2].1.zome("mishpat"),
            "verify_device_binding",
            query(&cc, &cee.1, content),
        )
        .await
        .is_err());
    // ... and verifies at an authenticated moment before the revocation, not after.
    let _: VerifiedDevice = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_historical_device_binding",
            VerifyHistoricalDeviceInput {
                device: query(&cc, &cee.1, content),
                witnessed_at: before.as_micros(),
            },
        )
        .await;
    assert!(c
        .call_fallible::<_, VerifiedDevice>(
            &cells[2].1.zome("mishpat"),
            "verify_historical_device_binding",
            VerifyHistoricalDeviceInput {
                device: query(&cc, &cee.1, content),
                witnessed_at: Timestamp::now().as_micros(),
            },
        )
        .await
        .is_err());
    // What the root approved still stands.
    let _: VerifiedDevice = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_device_binding",
            query(&t, &tee.1, content),
        )
        .await;
    eprintln!(
        "every-device: revoked B cannot approve; C verifies before the revocation and not now"
    );
}

/// What a node's own person stands on, and the one signing call an approval
/// makes. The base case: the person's node is their identity's only controller
/// and signs alone.
async fn consent_ceremony_signing(
    c: &SweetConductor,
    cells: &[(SweetCell, SweetCell, SweetCell)],
    operator: &AgentPubKey,
    authority: &Receipt,
    human: &ActionHash,
    content: &DnaHash,
) {
    let mishpat_dna = cells[0].1.dna_hash().clone();
    let standing: Option<ConsentStanding> = c
        .call(&cells[0].1.zome("mishpat"), "my_consent_standing", ())
        .await;
    let standing = standing.expect("the operator has a Human");
    assert_eq!(&standing.identity_root, human);
    assert_eq!(standing.authority.as_ref(), Some(&authority.action_hash));
    assert_eq!(&standing.controllers, &vec![operator.clone()]);
    assert_eq!(standing.required, 1);
    assert_eq!(standing.network_dna, mishpat_dna);

    // A person with a Human but no authority record yet is told so, and the
    // read creates nothing.
    let che: Option<ConsentStanding> = c
        .call(&cells[1].1.zome("mishpat"), "my_consent_standing", ())
        .await;
    assert!(
        che.as_ref().is_none_or(|s| s.authority.is_none()),
        "{che:?}"
    );
    let second: Option<ConsentStanding> = c
        .call(&cells[2].1.zome("mishpat"), "my_consent_standing", ())
        .await;
    assert!(second.is_none(), "{second:?}");

    let consent = DeviceConsent {
        authority: authority.action_hash.clone(),
        identity_root: human.clone(),
        consent_cid: "bafyreigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".into(),
    };
    let intent = DeviceIntent {
        domain: "elohim:device-enrollment:v1".into(),
        authority: authority.action_hash.clone(),
        identity_root: human.clone(),
        device_key: cells[1].1.agent_pubkey().clone(),
        network_dna: mishpat_dna.clone(),
        content_dna: content.clone(),
        issued_at: Timestamp::from_micros(123),
        supersedes: None,
    };
    let approval = DeviceApproval {
        consent: consent.clone(),
        enrollment: Some(intent.clone()),
    };
    let proofs: DeviceApprovalProofs = c
        .call(
            &cells[0].1.zome("mishpat"),
            "sign_device_approval",
            approval.clone(),
        )
        .await;
    assert_eq!(&proofs.consent.agent, operator);
    let signed = SignedDeviceConsent {
        consent: consent.clone(),
        proof: proofs.consent.clone(),
        via: None,
    };
    // Any holder can check the agreement; a different address does not verify.
    let holds: bool = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_device_consent",
            signed.clone(),
        )
        .await;
    assert!(holds);
    let mut moved = signed.clone();
    moved.consent.consent_cid = moved.consent.consent_cid.replace("bafy", "bafk");
    let holds: bool = c
        .call(&cells[2].1.zome("mishpat"), "verify_device_consent", moved)
        .await;
    assert!(!holds);
    // The enrollment proof is the same signature the enrollment extern makes.
    let alone: Proof = c
        .call(
            &cells[0].1.zome("mishpat"),
            "sign_device_enrollment",
            intent.clone(),
        )
        .await;
    assert_eq!(proofs.enrollment.unwrap().signature, alone.signature);

    // A device cannot approve itself, and one approval cannot speak for two
    // identities.
    assert!(c
        .call_fallible::<_, DeviceApprovalProofs>(
            &cells[1].1.zome("mishpat"),
            "sign_device_approval",
            approval.clone(),
        )
        .await
        .is_err());
    let mut split = approval;
    split.enrollment.as_mut().unwrap().identity_root = authority.action_hash.clone();
    assert!(c
        .call_fallible::<_, DeviceApprovalProofs>(
            &cells[0].1.zome("mishpat"),
            "sign_device_approval",
            split,
        )
        .await
        .is_err());
    eprintln!("identity proof: consent standing read and approval signed in one call");
}

#[test]
fn independently_keyed_devices_share_one_human_and_revocation_cannot_be_erased() -> Result<()> {
    // The multi-cell proof's future exceeds libtest's default 2 MiB stack.
    // Keep this bound local to the proof, including CI/nextest execution.
    const PROOF_STACK_BYTES: usize = 32 * 1024 * 1024;
    std::thread::Builder::new()
        .name("device-enrollment-proof".into())
        .stack_size(PROOF_STACK_BYTES)
        .spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(PROOF_STACK_BYTES)
                .enable_all()
                .build()?
                .block_on(device_enrollment_proof())
        })?
        .join()
        .expect("device enrollment proof thread panicked")
}

async fn device_enrollment_proof() -> Result<()> {
    let (mut c, operator) = single_agent_conductor().await?;
    let che = SweetAgents::one(c.keystore()).await;
    let second = SweetAgents::one(c.keystore()).await;
    // A node that begins an identity of its own before it joins the operator's.
    let prior = SweetAgents::one(c.keystore()).await;
    // The devices of the scenario where every device speaks for its person.
    let bee = SweetAgents::one(c.keystore()).await;
    let cee = SweetAgents::one(c.keystore()).await;
    let tee = SweetAgents::one(c.keystore()).await;
    let dee = SweetAgents::one(c.keystore()).await;
    let eve = SweetAgents::one(c.keystore()).await;
    assert_ne!(che, operator);
    assert_ne!(che, second);
    let seed = network_seed("device-enrollment");
    let imagodei = load_dna("imagodei", &seed, Some(operator.clone())).await?;
    let mishpat = load_dna("mishpat", &seed, Some(operator.clone())).await?;
    let lamad = load_dna("lamad", &seed, Some(operator.clone())).await?;
    let content = lamad.dna_hash().clone();
    let roles: Vec<(RoleName, DnaFile)> = vec![
        ("imagodei".into(), imagodei.clone()),
        ("mishpat".into(), mishpat.clone()),
        ("lamad".into(), lamad.clone()),
    ];
    let mut cells = vec![];
    for (name, agent) in [
        ("operator", operator.clone()),
        ("che", che.clone()),
        ("second", second.clone()),
        ("prior", prior.clone()),
        ("bee", bee.clone()),
        ("cee", cee.clone()),
        ("tee", tee.clone()),
        ("dee", dee.clone()),
        ("eve", eve.clone()),
    ] {
        let app = c.setup_app_for_agent(name, agent, &roles).await?;
        let identity = app
            .cells()
            .iter()
            .find(|cell| cell.dna_hash() == imagodei.dna_hash())
            .unwrap()
            .clone();
        let governance = app
            .cells()
            .iter()
            .find(|cell| cell.dna_hash() == mishpat.dna_hash())
            .unwrap()
            .clone();
        let content_cell = app
            .cells()
            .iter()
            .find(|cell| cell.dna_hash() == lamad.dna_hash())
            .unwrap()
            .clone();
        cells.push((identity, governance, content_cell));
    }
    let mut attacks = Attacks::default();
    let mut content_attacks = Attacks::default();
    eprintln!("identity proof: three independently keyed apps installed");
    let _legacy: ActionHash = c.call(&cells[1].0.zome("imagodei"), "create_human", serde_json::json!({"id":"prior-device-profile", "display_name":"Prior device", "bio":null,"affinities":[],"profile_reach":"public","location":null})).await;
    let human: ActionHash = c
        .call(
            &cells[0].0.zome("imagodei"),
            "create_human",
            serde_json::json!({
                "id":"real-matthew-device-proof", "display_name":"Matthew", "bio":null,
                "affinities":[], "profile_reach":"public", "location":null
            }),
        )
        .await;
    assert!(c
        .call_fallible::<_, Receipt>(
            &cells[1].1.zome("mishpat"),
            "bootstrap_device_identity",
            human.clone()
        )
        .await
        .is_err());
    let authority: Receipt = c
        .call(
            &cells[0].1.zome("mishpat"),
            "bootstrap_device_identity",
            human.clone(),
        )
        .await;
    eprintln!("identity proof: exact Human bootstrap accepted");
    consent_ceremony_signing(&c, &cells, &operator, &authority, &human, &content).await;
    a_node_with_its_own_identity_joins_as_it_is(
        &c, &cells, &operator, &authority, &human, &content,
    )
    .await;
    every_device_speaks_for_its_person(&c, &cells, &operator, &authority, &human, &content).await;
    let (che_binding, signed_binding) = enroll(
        &c,
        &cells[0].1,
        &cells[1].1,
        &authority,
        &human,
        &content,
        123,
    )
    .await;
    let (second_binding, _) = enroll(
        &c,
        &cells[0].1,
        &cells[2].1,
        &authority,
        &human,
        &content,
        124,
    )
    .await;
    for (i, binding) in [(1, &che_binding), (2, &second_binding)] {
        let answer: VerifiedDevice = c
            .call(
                &cells[i].1.zome("mishpat"),
                "verify_device_binding",
                query(binding, &cells[i].1, &content),
            )
            .await;
        assert_eq!(answer.human_action_hash, human);
        assert_eq!(answer.controllers, vec![operator.clone()]);
        let _: VerifiedDevice = c
            .call(
                &cells[i].0.zome("imagodei"),
                "register_device_identity",
                Register {
                    binding: binding.action_hash.clone(),
                    expected_content_dna: content.clone(),
                },
            )
            .await;
        let mine: Option<HumanOutput> = c
            .call(&cells[i].0.zome("imagodei"), "get_my_human", ())
            .await;
        assert_eq!(mine.unwrap().human["id"], "real-matthew-device-proof");
    }
    eprintln!("identity proof: both keys resolve operator, including prior self-profile");
    // Exact controller witness must survive later withdrawal, but cannot be
    // reused for a different head or created by the enrolled device itself.
    let publication_root: ContentAction = c.call(&cells[0].2.zome("content_store"),
        "create_content", serde_json::json!({
            "id":"device-publication-root", "content_type":"concept", "title":"Publication proof",
            "description":"isolated authorization regression", "content":"", "content_format":"markdown",
            "reach":"commons", "metadata_json":"{}", "tags":[], "related_node_ids":[],
        })).await;
    let requester = cells[2].2.agent_pubkey().clone();
    let secret = CapSecret::from([42; 64]);
    let broad_secret = CapSecret::from([43; 64]);
    let expires = Timestamp::from_micros(Timestamp::now().as_micros() + 300_000_000);
    let functions: std::collections::HashSet<GrantedFunction> = [
        "grant_head_delegation",
        "stage_delegated_head_acceptance",
        "accept_delegated_head",
        "get_accepted_delegated_head",
    ]
    .into_iter()
    .map(|f| (ZomeName::from("content_store"), FunctionName::from(f)))
    .collect();
    let mandate = serde_json::json!({
        "issuer":operator.to_string(), "requester":requester.to_string(),
        "dna":content.to_string(), "delegate":che.to_string(),
        "subjects":[{"id":"device-publication-root", "root":publication_root.action_hash.to_string()}],
        "operations":["grant_head_delegation", "stage_delegated_head_acceptance",
            "accept_delegated_head", "get_accepted_delegated_head"],
        "valid_until":expires.as_micros(), "binding":che_binding.action_hash.to_string(),
        "policy":"fct-native-regression", "exact_payload_json":null,
    });
    for (tag, capability) in [
        (
            format!("elohim:invocation-mandate:v1:{}", mandate),
            secret.clone(),
        ),
        ("signing_key".into(), broad_secret.clone()),
    ] {
        let cap_grant = ZomeCallCapGrant {
            tag,
            functions: GrantedFunctions::Listed(functions.clone()),
            access: CapAccess::Assigned {
                secret: capability,
                assignees: std::collections::BTreeSet::from([requester.clone()]),
            },
        };
        c.raw_handle()
            .grant_zome_call_capability(serde_json::from_value(serde_json::json!({
                "cell_id":cells[0].2.cell_id(), "cap_grant":cap_grant,
            }))?)
            .await?;
    }
    let grant_input = serde_json::json!({
        "delegate":che, "scope":"device-publication-root",
        "root_action_hash":publication_root.action_hash, "valid_until":expires,
        "device_binding":che_binding.action_hash,
    });
    assert!(
        c.call_from_fallible::<_, PublicationGrant>(
            &requester,
            Some(broad_secret),
            &cells[0].2.zome("content_store"),
            "grant_head_delegation",
            grant_input.clone()
        )
        .await
        .is_err(),
        "function-listed transport permission alone must not issue publication authority"
    );
    let mut expanded = grant_input.clone();
    expanded["scope"] = "another-course".into();
    assert!(
        c.call_from_fallible::<_, PublicationGrant>(
            &requester,
            Some(secret.clone()),
            &cells[0].2.zome("content_store"),
            "grant_head_delegation",
            expanded
        )
        .await
        .is_err(),
        "selected mandate must reject an expanded course scope"
    );
    let publication_grant: PublicationGrant = c
        .call_from(
            &requester,
            Some(secret.clone()),
            &cells[0].2.zome("content_store"),
            "grant_head_delegation",
            grant_input,
        )
        .await;
    let acceptance_input = serde_json::json!({"id":"device-publication-root",
        "head_action_hash":publication_root.action_hash, "delegation":publication_grant});
    let root_approval: PublicationApproval = c
        .call_from(
            &requester,
            Some(secret.clone()),
            &cells[0].2.zome("content_store"),
            "stage_delegated_head_acceptance",
            acceptance_input.clone(),
        )
        .await;
    assert!(
        c.call_from_fallible::<_, PublicationGrant>(
            &requester,
            Some(secret.clone()),
            &cells[0].2.zome("content_store"),
            "accept_delegated_head",
            acceptance_input
        )
        .await
        .is_err(),
        "provisional approval alone must not complete authorization"
    );
    let publication = serde_json::json!({
        "device": query(&che_binding, &cells[1].1, &content),
        "content_root": publication_root.action_hash,
        "content_head": publication_root.action_hash,
        "root_acceptance": root_approval.witness_action_hash,
    });
    assert!(c
        .call_fallible::<_, Receipt>(
            &cells[1].1.zome("mishpat"),
            "witness_device_publication",
            publication.clone()
        )
        .await
        .is_err());
    let publication_witness: Receipt = c
        .call(
            &cells[0].1.zome("mishpat"),
            "witness_device_publication",
            publication.clone(),
        )
        .await;
    let verified_publication: VerifiedPublication = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_device_publication",
            serde_json::json!({
                "publication": publication, "witness": publication_witness.action_hash,
            }),
        )
        .await;
    assert_eq!(
        verified_publication.device.human_id,
        "real-matthew-device-proof"
    );
    let accepted_publication: PublicationGrant = c.call_from(&requester, Some(secret.clone()),
        &cells[0].2.zome("content_store"), "accept_delegated_head",
        serde_json::json!({"id":"device-publication-root", "head_action_hash":publication_root.action_hash,
            "delegation":publication_grant, "device_witness_action_hash":publication_witness.action_hash})
    ).await;
    let accepted = accepted_publication.acceptance.as_ref().unwrap();
    assert_eq!(
        accepted.witness_action_hash,
        root_approval.witness_action_hash
    );
    assert_eq!(accepted.accepted_at, root_approval.accepted_at);
    assert_eq!(
        accepted.device_witness_action_hash,
        Some(publication_witness.action_hash.clone())
    );
    // Historical public acceptance still requires the private mandate's binding.
    let mut publisher_mandate = mandate.clone();
    publisher_mandate["issuer"] = che.to_string().into();
    publisher_mandate["operations"] = serde_json::json!(["preflight_head_publication"]);
    let preflight_input = serde_json::json!({"id":"device-publication-root",
        "expected_root":publication_root.action_hash, "accepted_head":publication_root.action_hash,
        "delegation":accepted_publication});
    for (device_binding, capability, should_accept) in [
        (
            che_binding.action_hash.clone(),
            CapSecret::from([44; 64]),
            true,
        ),
        (
            second_binding.action_hash.clone(),
            CapSecret::from([45; 64]),
            false,
        ),
    ] {
        publisher_mandate["binding"] = device_binding.to_string().into();
        let cap_grant = ZomeCallCapGrant {
            tag: format!("elohim:invocation-mandate:v1:{}", publisher_mandate),
            functions: GrantedFunctions::Listed(std::collections::HashSet::from([(
                ZomeName::from("content_store"),
                FunctionName::from("preflight_head_publication"),
            )])),
            access: CapAccess::Assigned {
                secret: capability.clone(),
                assignees: std::collections::BTreeSet::from([requester.clone()]),
            },
        };
        c.raw_handle()
            .grant_zome_call_capability(serde_json::from_value(serde_json::json!({
                "cell_id":cells[1].2.cell_id(), "cap_grant":cap_grant,
            }))?)
            .await?;
        let result = c
            .call_from_fallible::<_, PublicationPreflight>(
                &requester,
                Some(capability),
                &cells[1].2.zome("content_store"),
                "preflight_head_publication",
                preflight_input.clone(),
            )
            .await;
        if should_accept {
            assert_eq!(result?.agent, che);
        } else {
            assert!(
                result.is_err(),
                "historical acceptance must retain the selected credential binding"
            );
        }
    }
    let mut wrong_publication = publication.clone();
    wrong_publication["content_head"] =
        serde_json::to_value(ActionHash::from_raw_32(vec![63; 32]))?;
    assert!(c
        .call_fallible::<_, VerifiedPublication>(
            &cells[2].1.zome("mishpat"),
            "verify_device_publication",
            serde_json::json!({
                "publication": wrong_publication, "witness": publication_witness.action_hash,
            })
        )
        .await
        .is_err());

    let original_admin_allowed: bool = c
        .call(
            &cells[0].0.zome("imagodei"),
            "legacy_identity_admin_allowed",
            operator.clone(),
        )
        .await;
    assert!(
        original_admin_allowed,
        "enrollment must preserve original controller policy"
    );
    for target in [operator.clone(), cells[2].0.agent_pubkey().clone()] {
        let result = c
            .call_fallible::<_, serde_json::Value>(
                &cells[1].0.zome("imagodei"),
                "create_self_revocation",
                serde_json::json!({"revoked_key": target, "reason": "compromised"}),
            )
            .await;
        let error = result.expect_err("device enrollment must not grant identity administration");
        assert!(
            format!("{error:?}")
                .contains("enrolled device has no identity administration authority"),
            "must refuse at authority gate, before any missing lamad role: {error:?}"
        );
    }

    device_content_admin_is_denied(
        &c,
        &mut content_attacks,
        &cells[2].2,
        &cells[1].2,
        &cells[0].2,
        &operator,
        "enrolled",
        false,
    )
    .await;
    eprintln!("identity proof: direct and generic Content administration denied");
    // Unauthenticated lifecycle junk is not authorized pending evidence and
    // cannot disable another human's valid device or controller authority.
    for anchor in [che_binding.entry_hash.clone(), authority.entry_hash.clone()] {
        attacks
            .link(
                &c,
                &cells[1].1,
                anchor,
                ActionHash::from_raw_32(vec![91; 32]),
            )
            .await;
    }
    let unaffected: VerifiedDevice = c
        .call(
            &cells[1].1.zome("mishpat"),
            "verify_device_binding",
            query(&che_binding, &cells[1].1, &content),
        )
        .await;
    assert_eq!(unaffected.human_action_hash, human);
    let root_evidence: HumanRootEvidence = c
        .call(
            &cells[0].0.zome("imagodei"),
            "get_human_root_evidence",
            human.clone(),
        )
        .await;
    let forged_authority = Authority {
        action: "binds-identity".into(),
        binding_kind: "authority-v1".into(),
        chain_root: human.clone(),
        human_id: root_evidence.human_id.clone(),
        human_dna: root_evidence.dna_hash.clone(),
        network_dna: mishpat.dna_hash().clone(),
        head_key: operator.clone(),
        controllers: vec![operator.clone()],
        controller_policy: ControllerPolicy {
            kind: "self".into(),
            m: None,
            n: None,
        },
        authorization: "development-network".into(),
        previous_authority: None,
        signatures: vec![],
    };
    let forged_root = attacks
        .commitment(
            &c,
            &cells[1].1,
            RawCommitment {
                action: "binds-identity".into(),
                payload_json: serde_json::to_string(&forged_authority)?,
                signed_at: root_evidence.timestamp.as_micros().to_string(),
            },
        )
        .await;
    let mut forged_intent = signed_binding.intent.clone();
    forged_intent.authority = forged_root;
    assert!(
        c.call_fallible::<_, Proof>(
            &cells[0].1.zome("mishpat"),
            "sign_device_enrollment",
            forged_intent
        )
        .await
        .is_err(),
        "a copied canonical bootstrap payload is not authorization by its Human author"
    );
    // Possession alone, a forged controller signature, unknown reference and
    // wrong DNA must all fail without touching the Human or controller policy.
    let mut missing_possession = signed_binding.clone();
    missing_possession.possession = missing_possession.controllers[0].clone();
    assert!(c
        .call_fallible::<_, Receipt>(
            &cells[1].1.zome("mishpat"),
            "enroll_identity_device",
            missing_possession
        )
        .await
        .is_err());
    let mut wrong_network = signed_binding.intent.clone();
    wrong_network.network_dna = DnaHash::from_raw_32(vec![89; 32]);
    assert!(c
        .call_fallible::<_, Proof>(
            &cells[1].1.zome("mishpat"),
            "sign_device_enrollment",
            wrong_network
        )
        .await
        .is_err());
    let mut forged = signed_binding.clone();
    forged.controllers[0].signature = Signature([0; 64]);
    assert!(c
        .call_fallible::<_, Receipt>(
            &cells[1].1.zome("mishpat"),
            "enroll_identity_device",
            forged
        )
        .await
        .is_err());
    let mut unauthorized = signed_binding.clone();
    unauthorized.controllers = vec![unauthorized.possession.clone()];
    assert!(c
        .call_fallible::<_, Receipt>(
            &cells[1].1.zome("mishpat"),
            "enroll_identity_device",
            unauthorized
        )
        .await
        .is_err());
    let mut wrong = query(&che_binding, &cells[1].1, &content);
    wrong.expected_content_dna = DnaHash::from_raw_32(vec![78; 32]);
    assert!(c
        .call_fallible::<_, VerifiedDevice>(
            &cells[1].1.zome("mishpat"),
            "verify_device_binding",
            wrong
        )
        .await
        .is_err());
    let mut unknown = query(&che_binding, &cells[1].1, &content);
    unknown.binding = ActionHash::from_raw_32(vec![79; 32]);
    assert!(c
        .call_fallible::<_, VerifiedDevice>(
            &cells[1].1.zome("mishpat"),
            "verify_device_binding",
            unknown
        )
        .await
        .is_err());
    // Bypass the ceremony altogether: retain valid signed intent but change the
    // outer Commitment timestamp. This used to evade entry-keyed revocation.
    let fake: ActionHash = attacks
        .commitment(
            &c,
            &cells[1].1,
            RawCommitment {
                action: "binds-identity".into(),
                payload_json: serde_json::to_string(&signed_binding)?,
                signed_at: "99999".into(),
            },
        )
        .await;
    let mut fake_query = query(&che_binding, &cells[1].1, &content);
    fake_query.binding = fake;
    assert!(c
        .call_fallible::<_, VerifiedDevice>(
            &cells[1].1.zome("mishpat"),
            "verify_device_binding",
            fake_query
        )
        .await
        .is_err());
    eprintln!(
        "identity proof: context, possession, forgery and canonical envelope refusals passed"
    );
    let mut revocation = DeviceRevocation {
        action: "revokes-commitment".into(),
        binding_kind: "device-revocation-v1".into(),
        target: che_binding.action_hash.clone(),
        authority: authority.action_hash.clone(),
        network_dna: mishpat.dna_hash().clone(),
        signatures: vec![],
    };
    let proof: Proof = c
        .call(
            &cells[0].1.zome("mishpat"),
            "sign_device_revocation",
            revocation.clone(),
        )
        .await;
    revocation.signatures.push(proof);
    let _: Receipt = c
        .call(
            &cells[0].1.zome("mishpat"),
            "revoke_identity_device",
            revocation,
        )
        .await;
    assert!(c
        .call_fallible::<_, VerifiedDevice>(
            &cells[1].1.zome("mishpat"),
            "verify_device_binding",
            query(&che_binding, &cells[1].1, &content)
        )
        .await
        .is_err());
    assert!(
        c.call_fallible::<_, Option<HumanOutput>>(&cells[1].0.zome("imagodei"), "get_my_human", ())
            .await
            .is_err(),
        "revoked enrollment must not restore the device's old self-profile"
    );
    let revoked_admin = c
        .call_fallible::<_, serde_json::Value>(
            &cells[1].0.zome("imagodei"),
            "create_self_revocation",
            serde_json::json!({"revoked_key": operator, "reason": "compromised"}),
        )
        .await;
    let error = revoked_admin.expect_err("revoked device cannot administer via stale links");
    assert!(
        format!("{error:?}").contains("enrolled device has no identity administration authority")
    );
    device_content_admin_is_denied(
        &c,
        &mut content_attacks,
        &cells[2].2,
        &cells[1].2,
        &cells[0].2,
        &operator,
        "revoked",
        true,
    )
    .await;
    eprintln!("identity proof: revoked Content administration denied; legacy root retained");
    let old: Option<HumanOutput> = c
        .call(
            &cells[1].0.zome("imagodei"),
            "get_human_by_id",
            "prior-device-profile".to_string(),
        )
        .await;
    assert!(old.is_some(), "prior Human remains available as history");
    let deleted = attacks
        .delete(&c, &cells[1].1, &cells[0].1, che_binding.entry_hash.clone())
        .await;
    assert!(deleted > 0);
    assert!(c
        .call_fallible::<_, VerifiedDevice>(
            &cells[1].1.zome("mishpat"),
            "verify_device_binding",
            query(&che_binding, &cells[1].1, &content)
        )
        .await
        .is_err());
    let second_ok: VerifiedDevice = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_device_binding",
            query(&second_binding, &cells[2].1, &content),
        )
        .await;
    assert_eq!(second_ok.identity_root, human);
    let operator_human: Option<HumanOutput> = c
        .call(&cells[0].0.zome("imagodei"), "get_my_human", ())
        .await;
    assert_eq!(
        operator_human.unwrap().human["id"],
        "real-matthew-device-proof"
    );
    eprintln!(
        "identity proof: revocation survived link deletion; other participant and Human preserved"
    );
    let historical: VerifiedPublication = c
        .call(
            &cells[2].1.zome("mishpat"),
            "verify_device_publication",
            serde_json::json!({
                "publication": publication, "witness": publication_witness.action_hash,
            }),
        )
        .await;
    assert_eq!(historical.device.human_id, "real-matthew-device-proof");
    assert!(
        c.call_fallible::<_, Receipt>(
            &cells[0].1.zome("mishpat"),
            "witness_device_publication",
            publication
        )
        .await
        .is_err(),
        "withdrawal fences new controller exercise even after link deletion"
    );

    // A correctly authorized successor policy fences the old bootstrap entry,
    // including a new Create action containing the same deterministic root.
    let root_evidence: HumanRootEvidence = c
        .call(
            &cells[0].0.zome("imagodei"),
            "get_human_root_evidence",
            human.clone(),
        )
        .await;
    let mut next = Authority {
        action: "binds-identity".into(),
        binding_kind: "authority-v1".into(),
        chain_root: human.clone(),
        human_id: root_evidence.human_id,
        human_dna: root_evidence.dna_hash,
        network_dna: mishpat.dna_hash().clone(),
        head_key: operator.clone(),
        controllers: vec![operator.clone(), second.clone()],
        controller_policy: ControllerPolicy {
            kind: "recovery-quorum".into(),
            m: Some(2),
            n: Some(2),
        },
        authorization: "development-network".into(),
        previous_authority: Some(authority.action_hash.clone()),
        signatures: vec![],
    };
    let signature = c
        .keystore()
        .sign(operator.clone(), serde_json::to_vec(&next)?.into())
        .await?;
    next.signatures.push(Proof {
        agent: operator.clone(),
        signature,
    });
    let replayed_root: Receipt = c
        .call(
            &cells[0].1.zome("mishpat"),
            "bootstrap_device_identity",
            human.clone(),
        )
        .await;
    assert_eq!(replayed_root.entry_hash, authority.entry_hash);
    let successor: ActionHash = attacks
        .commitment(
            &c,
            &cells[0].1,
            RawCommitment {
                action: "binds-identity".into(),
                payload_json: serde_json::to_string(&next)?,
                signed_at: "authority-v1".into(),
            },
        )
        .await;
    attacks
        .link(
            &c,
            &cells[0].1,
            authority.entry_hash.clone(),
            successor.clone(),
        )
        .await;
    assert!(c
        .call_fallible::<_, VerifiedDevice>(
            &cells[2].1.zome("mishpat"),
            "verify_device_binding",
            query(&second_binding, &cells[2].1, &content)
        )
        .await
        .is_err());
    // With the person's policy at two (this successor: two of two), one
    // approval is not enough and two distinct ones are.
    let late = &cells[8].1;
    let mut two = intent_for(late, &authority, &human, &content, 300);
    two.authority = successor.clone();
    assert!(enroll_by(&c, &[(&cells[0].1, None)], late, two.clone())
        .await
        .is_err());
    let (late_binding, _) = enroll_by(&c, &[(&cells[0].1, None), (&cells[2].1, None)], late, two)
        .await
        .expect("two distinct approvals meet a policy of two");
    let _: VerifiedDevice = c
        .call(
            &cells[3].1.zome("mishpat"),
            "verify_device_binding",
            query(&late_binding, late, &content),
        )
        .await;
    eprintln!("identity proof: under a policy of two, one approval refused and two accepted");
    let mut replay_intent = signed_binding.intent.clone();
    replay_intent.authority = replayed_root.action_hash;
    assert!(c
        .call_fallible::<_, Proof>(
            &cells[0].1.zome("mishpat"),
            "sign_device_enrollment",
            replay_intent
        )
        .await
        .is_err());
    // Deleting the successor discovery link cannot resurrect the old policy.
    attacks
        .delete(&c, &cells[1].1, &cells[0].1, authority.entry_hash)
        .await;
    assert!(c
        .call_fallible::<_, VerifiedDevice>(
            &cells[2].1.zome("mishpat"),
            "verify_device_binding",
            query(&second_binding, &cells[2].1, &content)
        )
        .await
        .is_err());
    Ok(())
}
