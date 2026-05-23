//! Qahal coordinator: atomic multi-step orchestration for Collective + Collab flows.
//!
//! Per spec §2 + §5.1. Coordinator-only authority gates (link traversal happens here,
//! integrity-zome validators remain pure-data).

use hdk::prelude::*;
use imagodei_integrity::qahal::{
    CollabAgreement, Collective, MemberKind, Membership, MembershipRole,
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
pub fn list_memberships_for_collective(collective_hash: ActionHash) -> ExternResult<Vec<Record>> {
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
// Task 5: create_collab_agreement — pre-validated, pre-declared agreement
// =============================================================================

/// Create a CollabAgreement entry.
///
/// Pre-validates share-allocation JSON structure and tribute > 0 before commit.
/// The integrity validator checks pure-data fields (participant count, tribute
/// range, tier). This coordinator gate adds the structural JSON shape check and
/// verifies shares + tribute sum to 1.0.
///
/// Returns the ActionHash of the CollabAgreement entry. The Collab-Qahal is
/// NOT instantiated yet — instantiation is deferred until all participating
/// Collectives have counter-attested via `attest_collab_agreement`.
#[hdk_extern]
pub fn create_collab_agreement(input: CreateCollabAgreementInput) -> ExternResult<ActionHash> {
    validate_share_allocation_json(&input.share_allocation_json, input.commons_pool_tribute)?;

    let agent_cid = encode_agent_cid(&agent_info()?.agent_initial_pubkey);
    let agreement = CollabAgreement {
        authored_by_agent_cid: agent_cid,
        participants: input.participants.clone(),
        scope: input.scope,
        share_allocation_json: input.share_allocation_json,
        commons_pool_tribute: input.commons_pool_tribute,
        governance_terms_json: input.governance_terms_json,
        anchor_collective_cid: None,
        initial_tier: input.initial_tier,
        created_at_block_height: current_block_height()?,
        salt: input.salt,
    };
    create_entry(&EntryTypes::CollabAgreement(agreement))
}

/// Counter-attest a CollabAgreement on behalf of a participating Collective.
///
/// The caller must be a current Steward of `attesting_collective_cid` (checked
/// via `HasMembership` link traversal — OK in coordinator, not integrity zome).
///
/// Each attestation is recorded as an `AgreementOnCollab` link from the agreement
/// ActionHash to the attesting Collective's ActionHash, tagged with the Collective
/// CID bytes for cheap iteration.
///
/// When ALL participants have attested, atomically instantiates the Collab-Qahal:
/// creates a recursive Collective + one Collective-typed Steward Membership per
/// participant, bidirectionally linked via `HasMembership` and
/// `MembershipForAgreement`.
#[hdk_extern]
pub fn attest_collab_agreement(input: AttestCollabAgreementInput) -> ExternResult<()> {
    // 1. Verify caller is a current Steward of the attesting Collective.
    let caller_cid = encode_agent_cid(&agent_info()?.agent_initial_pubkey);
    require_caller_is_steward_of(&caller_cid, &input.attesting_collective_cid)?;

    // 2. Fetch and decode the CollabAgreement.
    let agreement_record = get(input.agreement_action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!("CollabAgreement not found"))?;
    let agreement: CollabAgreement = agreement_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!("{}", e))?
        .ok_or_else(|| wasm_error!("CollabAgreement decode failure"))?;

    // 3. Verify the attesting Collective is a declared participant.
    if !agreement
        .participants
        .contains(&input.attesting_collective_cid)
    {
        return Err(wasm_error!(
            "attesting Collective '{}' is not a participant",
            input.attesting_collective_cid
        ));
    }

    // 4. Record the attestation as a tagged AgreementOnCollab link.
    //    Tag = Collective CID bytes (UTF-8) for cheap set-membership checks below.
    let attesting_collective_hash =
        decode_collective_cid_to_action(&input.attesting_collective_cid)?;
    create_link(
        input.agreement_action_hash.clone(),
        attesting_collective_hash,
        LinkTypes::AgreementOnCollab,
        LinkTag::new(input.attesting_collective_cid.as_bytes()),
    )?;

    // 5. Check whether ALL participants have now attested.
    let attestation_links = {
        let q = LinkQuery::try_new(
            input.agreement_action_hash.clone(),
            LinkTypes::AgreementOnCollab,
        )?;
        get_links(q, GetStrategy::default())?
    };
    let attested_cids: std::collections::HashSet<String> = attestation_links
        .iter()
        .filter_map(|l| {
            // Skip the "INSTANTIATED" sentinel tag.
            if l.tag.0.as_slice() == b"INSTANTIATED" {
                None
            } else {
                std::str::from_utf8(l.tag.0.as_slice())
                    .ok()
                    .map(|s| s.to_string())
            }
        })
        .collect();

    if agreement
        .participants
        .iter()
        .all(|p| attested_cids.contains(p))
    {
        // All participants attested — instantiate the Collab-Qahal atomically.
        instantiate_collab_qahal(&input.agreement_action_hash, &agreement)?;
    }
    Ok(())
}

/// Atomic Collab-Qahal instantiation.
///
/// Creates the recursive Collective entry + one Collective-typed Steward
/// Membership for each participant, then links them bidirectionally.
/// The "INSTANTIATED" sentinel tag on an AgreementOnCollab link from the
/// agreement hash serves as the fast-path "is it done?" marker for
/// `get_collab_status`.
fn instantiate_collab_qahal(
    agreement_hash: &ActionHash,
    agreement: &CollabAgreement,
) -> ExternResult<()> {
    let block_height = current_block_height()?;

    let collab_qahal = Collective {
        founder_agent_cid: agreement.authored_by_agent_cid.clone(),
        charter: format!(
            "Collab-Qahal anchored on CollabAgreement {}",
            agreement_hash
        ),
        display_name: format!(
            "Collab: {}",
            agreement.scope.chars().take(64).collect::<String>()
        ),
        created_at_block_height: block_height,
        salt: agreement.salt.clone(),
        anchor_agreement_cid: Some(format!("agreement:{}", agreement_hash)),
    };
    let qahal_hash = create_entry(&EntryTypes::Collective(collab_qahal))?;
    let qahal_cid = action_hash_to_cid(&qahal_hash);

    // Sentinel link: agreement -> instantiated Qahal, tag = "INSTANTIATED".
    create_link(
        agreement_hash.clone(),
        qahal_hash.clone(),
        LinkTypes::AgreementOnCollab,
        LinkTag::new(b"INSTANTIATED"),
    )?;

    // Create a Collective-typed Steward Membership for each participating Collective.
    // sponsor_cid = "agreement:<hash>" — the Agreement IS the sponsorship act.
    let sponsor_cid = format!("agreement:{}", agreement_hash);
    for participant_cid in &agreement.participants {
        let membership = Membership {
            member_cid: participant_cid.clone(),
            member_kind: MemberKind::Collective,
            collective_cid: qahal_cid.clone(),
            role: MembershipRole::Steward,
            sponsor_cid: Some(sponsor_cid.clone()),
            joined_at_block_height: block_height,
            withdrawn_at_block_height: None,
        };
        let m_hash = create_entry(&EntryTypes::Membership(membership))?;
        // Collab-Qahal -> Membership (discovery)
        create_link(
            qahal_hash.clone(),
            m_hash.clone(),
            LinkTypes::HasMembership,
            (),
        )?;
        // Agreement -> Membership (provenance)
        create_link(
            agreement_hash.clone(),
            m_hash,
            LinkTypes::MembershipForAgreement,
            (),
        )?;
    }
    Ok(())
}

// =============================================================================
// Task 5: query helpers
// =============================================================================

/// Return "Instantiated" if all participants have attested and the Collab-Qahal
/// exists, or "PendingAttestations" otherwise.
///
/// The "INSTANTIATED" sentinel `AgreementOnCollab` link tag is the canonical
/// marker — set atomically by `instantiate_collab_qahal`.
#[hdk_extern]
pub fn get_collab_status(agreement_hash: ActionHash) -> ExternResult<String> {
    let q = LinkQuery::try_new(agreement_hash, LinkTypes::AgreementOnCollab)?;
    let links = get_links(q, GetStrategy::default())?;
    let instantiated = links.iter().any(|l| l.tag.0.as_slice() == b"INSTANTIATED");
    if instantiated {
        Ok("Instantiated".into())
    } else {
        Ok("PendingAttestations".into())
    }
}

/// Return the Collab-Qahal CID ("collective:<hash>") for a fully-attested
/// CollabAgreement.  Errors if not yet instantiated.
#[hdk_extern]
pub fn get_collab_qahal_cid_for_agreement(agreement_hash: ActionHash) -> ExternResult<String> {
    let q = LinkQuery::try_new(agreement_hash, LinkTypes::AgreementOnCollab)?;
    let links = get_links(q, GetStrategy::default())?;
    let sentinel = links
        .iter()
        .find(|l| l.tag.0.as_slice() == b"INSTANTIATED")
        .ok_or_else(|| wasm_error!("Collab not yet instantiated"))?;
    let qahal_hash = sentinel
        .target
        .clone()
        .into_action_hash()
        .ok_or_else(|| wasm_error!("sentinel link target is not an ActionHash"))?;
    Ok(action_hash_to_cid(&qahal_hash))
}

/// List Membership records for a Collective identified by its CID string.
///
/// Convenience wrapper over `list_memberships_for_collective` for callers who
/// hold the CID string (e.g., after `get_collab_qahal_cid_for_agreement`).
#[hdk_extern]
pub fn list_memberships_for_collective_cid(collective_cid: String) -> ExternResult<Vec<Record>> {
    let collective_hash = decode_collective_cid_to_action(&collective_cid)?;
    list_memberships_for_collective(collective_hash)
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

/// Verify that the given agent CID is an active Steward of the given Collective.
///
/// Traverses `HasMembership` links from the Collective's ActionHash and checks
/// for a non-withdrawn Steward Membership with matching `member_cid`.
/// This is intentionally coordinator-only (link traversal is not permitted in
/// the integrity zome).
fn require_caller_is_steward_of(agent_cid: &str, collective_cid: &str) -> ExternResult<()> {
    let collective_hash = decode_collective_cid_to_action(collective_cid)?;
    let membership_records = list_memberships_for_collective(collective_hash)?;
    for record in membership_records {
        if let Some(m) = record.entry().to_app_option::<Membership>().ok().flatten() {
            if m.member_cid == agent_cid
                && matches!(m.role, MembershipRole::Steward)
                && m.withdrawn_at_block_height.is_none()
            {
                return Ok(());
            }
        }
    }
    Err(wasm_error!(
        "caller is not a current Steward of {}",
        collective_cid
    ))
}

/// Decode a `collective:<ActionHash>` CID string back to its ActionHash.
///
/// The ActionHash was encoded via `action_hash_to_cid` which uses
/// `format!("collective:{}", hash)` where the ActionHash Display impl
/// produces the base64url-encoded holo-hash string (e.g. "uhCkkXXX...").
/// `ActionHash::try_from(&str)` reverses that encoding.
fn decode_collective_cid_to_action(cid: &str) -> ExternResult<ActionHash> {
    let raw = cid
        .strip_prefix("collective:")
        .ok_or_else(|| wasm_error!("collective CID must start with 'collective:'; got: {}", cid))?;
    ActionHash::try_from(raw)
        .map_err(|_| wasm_error!("invalid ActionHash in collective CID: {}", cid))
}

// =============================================================================
// Task 6: withdraw_membership_clean — clean-exit per spec §6.4
// =============================================================================

/// Withdraw a Membership via the clean-exit path (spec §6.4).
///
/// Sets `withdrawn_at_block_height` to the current block height. Future REA
/// flow allocation calculations at blocks >= withdrawn do not accrue to this
/// member (share-routing in Tasks 13/14 honours this boundary).
///
/// Authority rules:
/// - `Person` membership: caller must equal `member_cid`.
/// - `Collective` membership: caller must be a current Steward of the
///   withdrawing Collective.
/// - `ElohimAgent` membership: not supported in M1 (returns an error).
#[hdk_extern]
pub fn withdraw_membership_clean(input: WithdrawMembershipInput) -> ExternResult<()> {
    let existing = get(input.membership_action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!("Membership not found"))?;
    let m: Membership = existing
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!("{}", e))?
        .ok_or_else(|| wasm_error!("Membership decode failure"))?;

    // Authority check: caller must be the member (Person) or a current Steward
    // of the withdrawing Collective (Collective-typed memberships).
    let caller_cid = encode_agent_cid(&agent_info()?.agent_initial_pubkey);
    match m.member_kind {
        MemberKind::Person => {
            if m.member_cid != caller_cid {
                return Err(wasm_error!(
                    "only the member may withdraw their own Person Membership"
                ));
            }
        }
        MemberKind::Collective => {
            require_caller_is_steward_of(&caller_cid, &m.member_cid)?;
        }
        MemberKind::ElohimAgent => {
            // ElohimAgent membership withdrawal is governed differently; deferred.
            return Err(wasm_error!(
                "ElohimAgent Membership withdrawal not supported in M1"
            ));
        }
    }

    let block_height = current_block_height()?;
    let updated = Membership {
        withdrawn_at_block_height: Some(block_height),
        ..m
    };
    update_entry(
        input.membership_action_hash,
        &EntryTypes::Membership(updated),
    )?;
    Ok(())
}

/// Read the **latest** Membership record reachable from a creation ActionHash.
///
/// After `withdraw_membership_clean` runs an `update_entry`, the original
/// action still resolves via `get(originalHash)` to the pre-update entry
/// (HDK semantics: each action hash refers to a specific commit, and
/// `get` is action-addressed). To observe the updated state from the
/// original creation hash, we must traverse `RecordDetails.updates` and
/// resolve the latest Update action.
///
/// Used post-`withdraw_membership_clean` to assert that
/// `withdrawn_at_block_height` is now `Some(_)` — see sweettest
/// `qahal_collab_t0_test::withdraw_membership_clean_exit` (regression
/// repro: DNA #1268 panic at qahal_collab_t0_test.rs:470).
#[hdk_extern]
pub fn get_membership_by_action(action_hash: ActionHash) -> ExternResult<Record> {
    let details = get_details(action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!("Membership not found"))?;
    match details {
        Details::Record(record_details) => {
            // Updates are ordered by the source-chain action sequence; the
            // last entry is the most recent update reachable from this
            // original action. M1 Membership flows produce at most one
            // Update (clean withdrawal), so single-step follow is correct.
            // If multi-step updates become possible (e.g. reactivation),
            // this should iterate to the leaf of the update chain.
            if let Some(latest_update) = record_details.updates.last() {
                let updated_hash = latest_update.action_address().clone();
                get(updated_hash, GetOptions::default())?
                    .ok_or_else(|| wasm_error!("Membership Update record missing"))
            } else {
                Ok(record_details.record)
            }
        }
        Details::Entry(_) => Err(wasm_error!(
            "Expected Record details, got Entry-level details"
        )),
    }
}

/// Pre-validate the share-allocation JSON before committing a CollabAgreement.
///
/// Checks:
/// 1. Parses as valid JSON.
/// 2. `form == "declared"` (M1 only supports this form).
/// 3. `commons_pool_tribute` in JSON matches the `claimed_tribute` field.
/// 4. Tribute > 0 (structural — integrity also gates this on the entry field).
/// 5. `shares[*].share` sum + tribute == 1.0 (within 1e-6 tolerance).
fn validate_share_allocation_json(json: &str, claimed_tribute: f64) -> ExternResult<()> {
    let parsed: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| wasm_error!("share_allocation JSON parse error: {}", e))?;

    let form = parsed
        .get("form")
        .and_then(|v| v.as_str())
        .ok_or_else(|| wasm_error!("share_allocation.form missing or not a string"))?;
    if form != "declared" {
        return Err(wasm_error!(
            "M1 only supports share_allocation.form=\"declared\"; got \"{}\"",
            form
        ));
    }

    let tribute = parsed
        .get("commons_pool_tribute")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| {
            wasm_error!("share_allocation.commons_pool_tribute missing or not a number")
        })?;

    if (tribute - claimed_tribute).abs() > 1e-9 {
        return Err(wasm_error!(
            "share_allocation.commons_pool_tribute ({}) must match the commons_pool_tribute field ({})",
            tribute,
            claimed_tribute
        ));
    }
    if tribute <= 0.0 {
        return Err(wasm_error!("commons_pool_tribute must be > 0"));
    }

    let shares = parsed
        .get("shares")
        .and_then(|v| v.as_array())
        .ok_or_else(|| wasm_error!("share_allocation.shares missing or not an array"))?;

    let share_sum: f64 = shares
        .iter()
        .filter_map(|s| s.get("share").and_then(|v| v.as_f64()))
        .sum();

    if (share_sum + tribute - 1.0).abs() > 1e-6 {
        return Err(wasm_error!(
            "share_allocation shares ({}) + tribute ({}) must sum to 1.0",
            share_sum,
            tribute
        ));
    }
    Ok(())
}
