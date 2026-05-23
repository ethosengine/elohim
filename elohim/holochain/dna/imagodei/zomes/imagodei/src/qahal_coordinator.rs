//! Qahal coordinator: atomic multi-step orchestration for Collective + Collab flows.
//!
//! Per spec §2 + §5.1. Coordinator-only authority gates (link traversal happens here,
//! integrity-zome validators remain pure-data).

use hdk::prelude::*;
use imagodei_integrity::qahal::{
    Collective, Membership, MembershipRole, MemberKind,
    // CollabAgreement imported in Task 5 (create_collab_agreement)
};
use imagodei_integrity::{EntryTypes, LinkTypes, StringAnchor};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateCollectiveInput {
    pub charter: String,
    pub display_name: String,
    pub salt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateCollabAgreementInput {
    pub participants: Vec<String>,
    pub scope: String,
    pub share_allocation_json: String,
    pub commons_pool_tribute: f64,
    pub governance_terms_json: String,
    pub initial_tier: String,
    pub display_name_for_qahal: String,
    pub salt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttestCollabAgreementInput {
    pub agreement_action_hash: ActionHash,
    pub attesting_collective_cid: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawMembershipInput {
    pub membership_action_hash: ActionHash,
    pub collab_qahal_cid: String,
}

// =============================================================================
// Task 4: create_collective — atomic Collective + founder Steward Membership
// =============================================================================

/// Create a new Collective.
///
/// The founder's Steward Membership is created atomically in the same call to
/// resolve the chicken-and-egg of "who attests the first steward?". The founder
/// is bootstrapped via synthetic `sponsor_cid = "founder"` which the integrity
/// validator `validate_membership_pure` explicitly accepts.
///
/// Returns the `ActionHash` of the Collective entry. Callers use this hash for
/// subsequent reads (`get_collective_by_action`, `list_memberships_for_collective`)
/// and as the base for collab-agreement links in Tasks 5–6.
#[hdk_extern]
pub fn create_collective(input: CreateCollectiveInput) -> ExternResult<ActionHash> {
    let block_height = current_block_height()?;
    let founder_agent_pubkey = agent_info()?.agent_initial_pubkey;
    let founder_cid = encode_agent_cid(&founder_agent_pubkey);

    let collective = Collective {
        founder_agent_cid: founder_cid.clone(),
        charter: input.charter,
        display_name: input.display_name,
        created_at_block_height: block_height,
        salt: input.salt,
        anchor_agreement_cid: None,
    };

    let collective_hash = create_entry(&EntryTypes::Collective(collective))?;
    let collective_cid = action_hash_to_cid(&collective_hash);

    // Atomically create founder Steward Membership.
    // `sponsor_cid = Some("founder")` satisfies the integrity gate which
    // requires Steward-role memberships to carry a sponsor. The integrity
    // validator (validate_membership_pure) explicitly documents this bypass.
    let founder_membership = Membership {
        member_cid: founder_cid,
        member_kind: MemberKind::Person,
        collective_cid: collective_cid.clone(),
        role: MembershipRole::Steward,
        sponsor_cid: Some("founder".into()),
        joined_at_block_height: block_height,
        withdrawn_at_block_height: None,
    };
    let membership_hash = create_entry(&EntryTypes::Membership(founder_membership))?;

    // Anchor for discovery: StringAnchor("collective", <collective_cid>) -> Collective
    let charter_anchor = StringAnchor::new("collective", &collective_cid);
    let charter_anchor_hash = hash_entry(&EntryTypes::StringAnchor(charter_anchor))?;
    create_link(
        charter_anchor_hash,
        collective_hash.clone(),
        LinkTypes::CharterAnchor,
        (),
    )?;

    // Bidirectional discovery: Collective -> Membership
    create_link(
        collective_hash.clone(),
        membership_hash,
        LinkTypes::HasMembership,
        (),
    )?;

    Ok(collective_hash)
}

/// Read back a Collective record by its ActionHash.
///
/// Returns `None` if the entry has not yet been committed or is not
/// accessible in this network partition. Sweettests use this for the
/// "Collective record exists" assertion.
#[hdk_extern]
pub fn get_collective_by_action(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(action_hash, GetOptions::default())
}

/// List all Membership records linked from a Collective's ActionHash.
///
/// Traverses the `HasMembership` links from the given Collective ActionHash
/// and fetches each Membership entry. Skipped entries (DHT hole, network
/// partition) are silently omitted — callers should retry if the count is
/// unexpected.
#[hdk_extern]
pub fn list_memberships_for_collective(
    collective_hash: ActionHash,
) -> ExternResult<Vec<Record>> {
    let query = LinkQuery::try_new(collective_hash, LinkTypes::HasMembership)?;
    let links = get_links(query, GetStrategy::default())?;
    let mut out = Vec::new();
    for link in links {
        if let Some(membership_hash) = link.target.into_action_hash() {
            if let Some(record) = get(membership_hash, GetOptions::default())? {
                out.push(record);
            }
        }
    }
    Ok(out)
}

// =============================================================================
// Private helpers
// =============================================================================

/// Encode an AgentPubKey as the canonical `agent:<pubkey>` CID string used
/// throughout the imagodei coordinator (mirrors the `.to_string()` pattern
/// already used in `create_agent` and `commit_key_rotation`).
fn encode_agent_cid(pubkey: &AgentPubKey) -> String {
    format!("agent:{}", pubkey)
}

/// Encode an ActionHash as the canonical `collective:<hash>` CID string used
/// as the collective_cid field in Membership entries.
fn action_hash_to_cid(hash: &ActionHash) -> String {
    format!("collective:{}", hash)
}

/// Return the current time as a microsecond epoch value, used as a proxy for
/// block height until the real block-height primitive is available in HDK.
fn current_block_height() -> ExternResult<u64> {
    Ok(sys_time()?.as_micros() as u64)
}
